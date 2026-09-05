//! SQL identifiers come only from the bundled schema; all values use bind parameters.
use crate::{AppError, Result};
use serde_json::{Value, json};
use sqlx::{Column, MySql, MySqlPool, Row, TypeInfo, ValueRef, mysql::MySqlRow};

pub fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}
pub fn num(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| scalar(v).parse().unwrap_or(0))
}
pub fn snake(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
pub fn camel(s: &str) -> String {
    let mut upper = false;
    let mut out = String::new();
    for c in s.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}
pub fn public(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("password");
        o.remove("tokenHash");
        o.remove("deleteTime");
    }
    v
}
pub fn row(r: MySqlRow) -> Result<Value> {
    let mut out = serde_json::Map::new();
    for c in r.columns() {
        let n = c.name();
        let raw = r.try_get_raw(n)?;
        let v = if raw.is_null() {
            Value::Null
        } else {
            match c.type_info().name() {
                "BIGINT" | "INT" | "SMALLINT" | "TINYINT" | "MEDIUMINT" => {
                    json!(r.try_get::<i64, _>(n)?)
                }
                "BIGINT UNSIGNED" | "INT UNSIGNED" | "SMALLINT UNSIGNED" | "TINYINT UNSIGNED" => {
                    json!(r.try_get::<u64, _>(n)?)
                }
                "DATETIME" | "TIMESTAMP" => json!(
                    r.try_get::<chrono::NaiveDateTime, _>(n)?
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                ),
                "FLOAT" | "DOUBLE" => json!(r.try_get::<f64, _>(n)?),
                _ => json!(r.try_get::<String, _>(n)?),
            }
        };
        out.insert(camel(n), v);
    }
    Ok(Value::Object(out))
}
pub fn query<'a>(
    sql: &'a str,
    args: &[Value],
) -> sqlx::query::Query<'a, MySql, sqlx::mysql::MySqlArguments> {
    let mut q = sqlx::query(sql);
    for a in args {
        q = match a {
            Value::Null => q.bind(None::<String>),
            Value::Bool(b) => q.bind(*b as i64),
            Value::Number(n) => q.bind(n.as_i64().unwrap_or(0)),
            _ => q.bind(scalar(a)),
        };
    }
    q
}
pub async fn rows(pool: &MySqlPool, sql: &str, args: &[Value]) -> Result<Vec<Value>> {
    query(sql, args)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(row)
        .collect()
}
pub async fn one(pool: &MySqlPool, sql: &str, args: &[Value]) -> Result<Value> {
    Ok(rows(pool, sql, args)
        .await?
        .into_iter()
        .next()
        .unwrap_or(Value::Null))
}
pub async fn exec(pool: &MySqlPool, sql: &str, args: &[Value]) -> Result<u64> {
    Ok(query(sql, args).execute(pool).await?.last_insert_id())
}
pub fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}
pub fn identifiers(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !s.as_bytes()[0].is_ascii_digit()
}
pub fn ids(s: &str) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for x in s.split(',') {
        let n = x.parse::<i64>().map_err(|_| AppError::bad("ID 格式错误"))?;
        if n <= 0 {
            return Err(AppError::bad("ID 格式错误"));
        }
        let v = json!(n);
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if out.len() > 100 {
        return Err(AppError::bad("每次最多 100 条"));
    }
    Ok(out)
}

pub async fn initialize(pool: &MySqlPool, password: String) -> Result<()> {
    crate::auth::validate_password(&password)?;
    let lock = sqlx::query_scalar::<_, i64>("SELECT GET_LOCK('ruoyi_rust_by_init',10)")
        .fetch_one(pool)
        .await?;
    if lock != 1 {
        return Err(AppError::bad("初始化正在运行"));
    }
    // Migrations are tracked by SQLx, and seed data is committed as one transaction.
    sqlx::migrate!().run(pool).await.map_err(|e| {
        tracing::error!("migration failed: {e}");
        AppError::bad("数据库迁移失败，请检查服务端日志")
    })?;
    let mut tx = pool.begin().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sys_user")
        .fetch_one(&mut *tx)
        .await?;
    if count > 0 {
        return Err(AppError::bad("数据库已初始化，拒绝覆盖用户"));
    }
    let mut seed: Value =
        serde_json::from_str(include_str!("../database/seed.json")).expect("bundled seed");
    let hash = crate::auth::hash_password(password).await?;
    seed["sys_user"][0]["password"] = json!(hash);
    for (table, rows) in seed.as_object().unwrap() {
        for r in rows.as_array().unwrap() {
            let o = r.as_object().unwrap();
            let cols: Vec<_> = o.keys().map(|k| format!("`{k}`")).collect();
            let vals: Vec<_> = o.values().cloned().collect();
            query(
                &format!(
                    "INSERT INTO `{table}` ({}) VALUES ({})",
                    cols.join(","),
                    placeholders(vals.len())
                ),
                &vals,
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
