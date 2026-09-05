//! Redis monitoring is restricted to the explicitly configured application cache namespace.
use crate::{App, AppError, Input, Result, auth::Actor, data, ok};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use redis::aio::MultiplexedConnection;
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};
fn error(_: redis::RedisError) -> AppError {
    AppError::bad("Redis 操作失败，请检查连接配置和 ACL 权限")
}
fn prefix() -> Result<String> {
    let s = crate::env("REDIS_CACHE_PREFIX", "ruoyi:cache:");
    if s.len() < 3
        || s.len() > 100
        || !s.ends_with(':')
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:-".contains(&b))
    {
        return Err(AppError::bad(
            "REDIS_CACHE_PREFIX 需为独立应用前缀并以冒号结束",
        ));
    }
    Ok(s)
}
async fn connect() -> Result<MultiplexedConnection> {
    let url = crate::env("REDIS_URL", "");
    if url.is_empty() {
        return Err(AppError::bad("尚未配置 REDIS_URL"));
    }
    let client = redis::Client::open(url).map_err(error)?;
    tokio::time::timeout(
        Duration::from_secs(3),
        client.get_multiplexed_async_connection(),
    )
    .await
    .map_err(|_| AppError::bad("Redis 连接超时"))?
    .map_err(error)
}
pub async fn ping() -> Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut c = connect().await?;
        let pong: String = redis::cmd("PING")
            .query_async(&mut c)
            .await
            .map_err(error)?;
        if pong != "PONG" {
            return Err(AppError::bad("Redis PING 响应无效"));
        }
        Ok(())
    })
    .await
    .map_err(|_| AppError::bad("Redis PING 超时"))?
}
fn key(s: &str, prefix: &str) -> Result<String> {
    let s = percent_encoding::percent_decode_str(s)
        .decode_utf8()
        .map_err(|_| AppError::bad("缓存键编码无效"))?
        .into_owned();
    if !s.starts_with(prefix) || s.len() > 512 || s.chars().any(char::is_control) {
        return Err(AppError::forbidden());
    }
    Ok(s)
}
async fn keys(c: &mut MultiplexedConnection, prefix: &str) -> Result<Vec<String>> {
    let mut cursor = 0u64;
    let mut keys = BTreeSet::new();
    for _ in 0..200 {
        let (next, batch): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(format!("{prefix}*"))
            .arg("COUNT")
            .arg(100)
            .query_async(c)
            .await
            .map_err(error)?;
        for k in batch {
            if k.starts_with(prefix) {
                keys.insert(k);
            }
        }
        if keys.len() > 10000 {
            return Err(AppError::bad(
                "应用缓存超过 10000 个键，请使用运维工具分批管理",
            ));
        }
        cursor = next;
        if cursor == 0 {
            return Ok(keys.into_iter().collect());
        }
    }
    Err(AppError::bad(
        "Redis 扫描达到本次上限，请使用运维工具分批管理",
    ))
}
fn info(s: &str) -> HashMap<String, String> {
    s.lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.to_owned(), v.trim().to_owned()))
        .collect()
}
async fn inner(a: &Actor, i: &Input) -> Result<Value> {
    a.require(if i.path == "monitor/cache" {
        "monitor:cache:list"
    } else {
        "monitor:cache:query"
    })?;
    // Values and cache deletion are reserved for trusted administrators.
    a.superuser()?;
    let namespace = prefix()?;
    let mut c = connect().await?;
    let path = i
        .path
        .strip_prefix("monitor/cache")
        .unwrap_or("")
        .trim_matches('/');
    match (i.method.as_str(), path) {
        ("GET", "") => {
            let raw: String = redis::cmd("INFO")
                .query_async(&mut c)
                .await
                .map_err(error)?;
            let all = info(&raw);
            let mut safe = HashMap::new();
            for name in [
                "redis_version",
                "redis_mode",
                "tcp_port",
                "used_memory",
                "used_cpu_user_children",
                "os",
                "arch_bits",
                "uptime_in_days",
                "connected_clients",
                "used_memory_human",
                "used_memory_peak_human",
                "maxmemory_human",
                "used_cpu_sys",
                "used_cpu_user",
                "instantaneous_ops_per_sec",
                "total_commands_processed",
                "instantaneous_input_kbps",
                "instantaneous_output_kbps",
                "aof_enabled",
                "rdb_last_bgsave_status",
            ] {
                if let Some(v) = all.get(name) {
                    safe.insert(name.to_owned(), v.clone());
                }
            }
            let count: i64 = redis::cmd("DBSIZE")
                .query_async(&mut c)
                .await
                .map_err(error)?;
            let stats: String = redis::cmd("INFO")
                .arg("commandstats")
                .query_async(&mut c)
                .await
                .map_err(error)?;
            let commands = info(&stats)
                .into_iter()
                .filter_map(|(k, v)| {
                    let name = k.strip_prefix("cmdstat_")?.to_owned();
                    let calls = v
                        .split(',')
                        .find_map(|s| s.strip_prefix("calls="))
                        .unwrap_or("0")
                        .parse::<u64>()
                        .unwrap_or(0);
                    Some(json!({"name":name,"value":calls}))
                })
                .collect::<Vec<_>>();
            Ok(data(
                json!({"info":safe,"dbSize":count,"commandStats":commands,"cachePrefix":namespace}),
            ))
        }
        ("GET", "getNames") => Ok(data(
            json!([{"cacheName":namespace,"remark":"当前应用缓存；不会清理其他前缀"}]),
        )),
        ("GET", p) if p.starts_with("getKeys/") => {
            let name = key(p.trim_start_matches("getKeys/"), &namespace)?;
            if name != namespace {
                return Err(AppError::forbidden());
            }
            Ok(data(json!(keys(&mut c, &namespace).await?)))
        }
        ("GET", p) if p.starts_with("getValue/") => {
            let (name, k) = p
                .trim_start_matches("getValue/")
                .split_once('/')
                .ok_or_else(|| AppError::bad("缺少缓存键"))?;
            if key(name, &namespace)? != namespace {
                return Err(AppError::forbidden());
            }
            let k = key(k, &namespace)?;
            let ty: String = redis::cmd("TYPE")
                .arg(&k)
                .query_async(&mut c)
                .await
                .map_err(error)?;
            let ttl: i64 = redis::cmd("TTL")
                .arg(&k)
                .query_async(&mut c)
                .await
                .map_err(error)?;
            let content = if ty == "string" {
                let bytes: Vec<u8> = redis::cmd("GETRANGE")
                    .arg(&k)
                    .arg(0)
                    .arg(65535)
                    .query_async(&mut c)
                    .await
                    .map_err(error)?;
                String::from_utf8_lossy(&bytes).into_owned()
            } else {
                format!("类型：{ty}；仅字符串缓存支持内容预览")
            };
            Ok(data(
                json!({"cacheName":namespace,"cacheKey":k,"cacheValue":content,"type":ty,"ttl":ttl,"previewLimit":65536}),
            ))
        }
        ("DELETE", p) => {
            a.require("monitor:cache:remove")?;
            let list = if p == "clearCacheAll" {
                keys(&mut c, &namespace).await?
            } else if let Some(name) = p.strip_prefix("clearCacheName/") {
                if key(name, &namespace)? != namespace {
                    return Err(AppError::forbidden());
                }
                keys(&mut c, &namespace).await?
            } else if let Some(k) = p.strip_prefix("clearCacheKey/") {
                vec![key(k, &namespace)?]
            } else {
                return Err(AppError::missing());
            };
            for batch in list.chunks(100) {
                let _: i64 = redis::cmd("DEL")
                    .arg(batch)
                    .query_async(&mut c)
                    .await
                    .map_err(error)?;
            }
            Ok(ok())
        }
        _ => Err(AppError::missing()),
    }
}
pub async fn handle(_: &App, a: &Actor, i: &Input) -> Result<Response> {
    let value = tokio::time::timeout(Duration::from_secs(10), inner(a, i))
        .await
        .map_err(|_| AppError::bad("Redis 操作超时；若为清理操作，请刷新确认进度"))??;
    Ok(Json(value).into_response())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespace_boundary() {
        assert!(key("other:secret", "ruoyi:cache:").is_err());
        assert!(key("ruoyi:cacheevil:x", "ruoyi:cache:").is_err());
        assert_eq!(
            key("ruoyi%3Acache%3Aa%2Fb", "ruoyi:cache:").unwrap(),
            "ruoyi:cache:a/b"
        );
    }
}
