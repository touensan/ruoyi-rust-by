use crate::{App, AppError, Input, Result, auth::Actor, data, db, ok};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use cron::Schedule;
use serde_json::{Value, json};
use std::{str::FromStr, time::Duration};
const TARGETS: [&str; 3] = ["system.heartbeat", "session.cleanup", "cache.ping"];
fn schedule(s: &str) -> Result<Schedule> {
    if s.len() > 100 || !(6..=7).contains(&s.split_whitespace().count()) {
        return Err(AppError::bad("Cron 使用 UTC 时区，需 6 或 7 段（包含秒）"));
    }
    Schedule::from_str(s).map_err(|_| AppError::bad("Cron 格式无效；不支持 L/W/# 扩展"))
}
fn next(s: &str, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
    schedule(s)?
        .after(&after)
        .next()
        .ok_or_else(|| AppError::bad("Cron 没有后续执行时间"))
}
fn datetime(t: DateTime<Utc>) -> Value {
    json!(t.format("%Y-%m-%d %H:%M:%S").to_string())
}
fn validate(i: &Input) -> Result<()> {
    if i.s("jobName").is_empty()
        || i.s("jobName").chars().count() > 64
        || !["DEFAULT", "SYSTEM"].contains(&i.s("jobGroup").as_str())
        || !TARGETS.contains(&i.s("invokeTarget").as_str())
        || !["0", "1"].contains(&i.s("status").as_str())
        || i.s("concurrent") != "1"
        || !["2", "3"].contains(&i.s("misfirePolicy").as_str())
        || i.s("remark").chars().count() > 500
    {
        return Err(AppError::bad(
            "任务字段无效；仅内置任务、禁止并发、错过后执行一次或跳过",
        ));
    }
    next(&i.s("cronExpression"), Utc::now())?;
    Ok(())
}
async fn execute(app: &App, id: Value, manual: bool) -> Result<()> {
    // Row lock serializes manual and scheduled executions across application replicas.
    let mut tx = app.pool.begin().await?;
    let job = db::query(
        "SELECT * FROM sys_job WHERE job_id=? FOR UPDATE",
        std::slice::from_ref(&id),
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::missing)?;
    let job = db::row(job)?;
    let now = Utc::now();
    if !manual {
        if job["status"] != "0" {
            return Ok(());
        }
        let due =
            NaiveDateTime::parse_from_str(&db::scalar(&job["nextRunTime"]), "%Y-%m-%d %H:%M:%S")
                .map_err(|_| AppError::bad("任务下次执行时间无效"))?
                .and_utc();
        if due > now {
            return Ok(());
        }
        let future = next(&db::scalar(&job["cronExpression"]), now)
            .ok()
            .map(datetime);
        db::query(
            "UPDATE sys_job SET next_run_time=?,status=IF(? IS NULL,'1',status) WHERE job_id=?",
            &[
                future.clone().unwrap_or(Value::Null),
                future.unwrap_or(Value::Null),
                id.clone(),
            ],
        )
        .execute(&mut *tx)
        .await?;
        if job["misfirePolicy"] == "3" && (now - due).num_seconds() > 2 {
            tx.commit().await?;
            return Ok(());
        }
    }
    let started = datetime(now);
    let result: Result<String> = match db::scalar(&job["invokeTarget"]).as_str() {
        "system.heartbeat" => Ok("服务运行正常".into()),
        "session.cleanup" => db::query("DELETE FROM sys_access_token WHERE expires_at<=NOW()", &[])
            .execute(&mut *tx)
            .await
            .map(|r| format!("已清理 {} 条过期会话", r.rows_affected()))
            .map_err(AppError::from),
        "cache.ping" => crate::cache::ping().await.map(|_| "Redis PING 成功".into()),
        _ => Err(AppError::bad("任务不在内置白名单中")),
    };
    let (status, message, error) = match &result {
        Ok(s) => ("0", s.clone(), String::new()),
        Err(e) => ("1", "任务执行失败".into(), e.1.clone()),
    };
    db::query("INSERT INTO sys_job_log(job_id,job_name,job_group,invoke_target,job_message,status,exception_info,start_time,stop_time) VALUES(?,?,?,?,?,?,?,?,?)",&[id,job["jobName"].clone(),job["jobGroup"].clone(),job["invokeTarget"].clone(),json!(message),json!(status),json!(error),started,datetime(Utc::now())]).execute(&mut *tx).await?;
    tx.commit().await?;
    result.map(|_| ())
}
pub fn start(app: App) {
    if crate::env("SCHEDULER_ENABLED", "true") == "false" {
        return;
    }
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            match db::rows(&app.pool,"SELECT job_id FROM sys_job WHERE status='0' AND next_run_time<=NOW() ORDER BY next_run_time LIMIT 20",&[]).await{Ok(rows)=>for row in rows{if let Err(e)=execute(&app,row["jobId"].clone(),false).await{tracing::warn!(message=%e.1,"scheduled job failed");}},Err(_)=>tracing::warn!("scheduler could not read jobs; run database migrations before serve")}
        }
    });
}
fn filters(i: &Input, logs: bool) -> Result<(String, Vec<Value>)> {
    let mut cond = "1=1".to_owned();
    let mut args = Vec::new();
    for (key, col) in [
        ("jobName", "job_name"),
        ("jobGroup", "job_group"),
        ("status", "status"),
    ] {
        if !i.s(key).is_empty() {
            cond.push_str(&format!(
                " AND {col} {} ?",
                if key == "jobName" { "LIKE" } else { "=" }
            ));
            args.push(json!(if key == "jobName" {
                format!("%{}%", i.s(key))
            } else {
                i.s(key)
            }));
        }
    }
    if logs && db::num(&i.value("jobId")) > 0 {
        cond.push_str(" AND job_id=?");
        args.push(i.value("jobId"));
    }
    if logs {
        for (key, op, suffix) in [
            ("beginTime", ">=", "00:00:00"),
            ("endTime", "<=", "23:59:59"),
        ] {
            let value = i.body["params"][key]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| i.s(&format!("params[{key}]")));
            if !value.is_empty() {
                chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                    .map_err(|_| AppError::bad("日志日期格式无效"))?;
                cond.push_str(&format!(" AND create_time {op} ?"));
                args.push(json!(format!("{value} {suffix}")));
            }
        }
    }
    Ok((cond, args))
}

pub async fn handle(app: &App, a: &Actor, i: &Input) -> Result<Response> {
    let logs = i.path.starts_with("monitor/jobLog");
    let root = if logs {
        "monitor/jobLog"
    } else {
        "monitor/job"
    };
    let tail = i.path.strip_prefix(root).unwrap_or("").trim_matches('/');
    let table = if logs { "sys_job_log" } else { "sys_job" };
    let select = if logs { "*,stop_time AS end_time" } else { "*" };
    let pk = if logs { "job_log_id" } else { "job_id" };
    if i.method == axum::http::Method::POST && tail == "export" {
        a.require("monitor:job:export")?;
        let (cond, args) = filters(i, logs)?;
        let rows = db::rows(
            &app.pool,
            &format!("SELECT * FROM {table} WHERE {cond} ORDER BY {pk} DESC LIMIT 10000"),
            &args,
        )
        .await?;
        let fields = if logs {
            vec![
                "jobLogId",
                "jobName",
                "jobGroup",
                "invokeTarget",
                "status",
                "jobMessage",
                "exceptionInfo",
                "startTime",
                "stopTime",
            ]
        } else {
            vec![
                "jobId",
                "jobName",
                "jobGroup",
                "invokeTarget",
                "cronExpression",
                "status",
                "nextRunTime",
            ]
        };
        let mut csv = format!("\u{feff}{}\r\n", fields.join(","));
        for row in rows {
            let values = fields
                .iter()
                .map(|key| {
                    let mut value = db::scalar(&row[key]);
                    if value.starts_with(['=', '+', '-', '@', '\t', '\r', '\n']) {
                        value.insert(0, '\'');
                    }
                    format!("\"{}\"", value.replace('"', "\"\""))
                })
                .collect::<Vec<_>>();
            csv.push_str(&values.join(","));
            csv.push_str("\r\n");
        }
        return Ok((
            [
                (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=jobs.csv",
                ),
            ],
            csv,
        )
            .into_response());
    }
    let v = match (i.method.as_str(), tail) {
        ("GET", "list") => {
            a.require("monitor:job:list")?;
            let page = i.s("pageNum").parse::<i64>().unwrap_or(1).clamp(1, 1000000);
            let size = i.s("pageSize").parse::<i64>().unwrap_or(10).clamp(1, 100);
            let (cond, args) = filters(i, logs)?;
            let total = db::one(
                &app.pool,
                &format!("SELECT COUNT(*) AS total FROM {table} WHERE {cond}"),
                &args,
            )
            .await?;
            let rows = db::rows(
                &app.pool,
                &format!(
                    "SELECT {select} FROM {table} WHERE {cond} ORDER BY {pk} DESC LIMIT {size} OFFSET {}",
                    (page - 1) * size
                ),
                &args,
            )
            .await?;
            json!({"code":200,"rows":rows,"total":total["total"]})
        }
        ("POST", "") | ("PUT", "") if !logs => {
            a.require(if i.method == axum::http::Method::POST {
                "monitor:job:add"
            } else {
                "monitor:job:edit"
            })?;
            a.superuser()?;
            validate(i)?;
            let mut args = vec![
                i.value("jobName"),
                i.value("jobGroup"),
                i.value("invokeTarget"),
                i.value("cronExpression"),
                i.value("misfirePolicy"),
                i.value("concurrent"),
                i.value("status"),
                json!(i.s("remark")),
                datetime(next(&i.s("cronExpression"), Utc::now())?),
            ];
            if i.method == axum::http::Method::POST {
                args.push(a.user["userName"].clone());
                data(json!(db::exec(&app.pool,"INSERT INTO sys_job(job_name,job_group,invoke_target,cron_expression,misfire_policy,concurrent,status,remark,next_run_time,create_by) VALUES(?,?,?,?,?,?,?,?,?,?)",&args).await?))
            } else {
                args.push(i.value("jobId"));
                let r=db::query("UPDATE sys_job SET job_name=?,job_group=?,invoke_target=?,cron_expression=?,misfire_policy=?,concurrent=?,status=?,remark=?,next_run_time=?,update_time=NOW() WHERE job_id=?",&args).execute(&app.pool).await?;
                if r.rows_affected() == 0 {
                    return Err(AppError::missing());
                }
                ok()
            }
        }
        ("PUT", "changeStatus") if !logs => {
            a.require("monitor:job:changeStatus")?;
            a.superuser()?;
            if !["0", "1"].contains(&i.s("status").as_str()) {
                return Err(AppError::bad("状态无效"));
            }
            let mut tx = app.pool.begin().await?;
            let row = db::query(
                "SELECT cron_expression FROM sys_job WHERE job_id=? FOR UPDATE",
                &[i.value("jobId")],
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(AppError::missing)?;
            let job = db::row(row)?;
            db::query(
                "UPDATE sys_job SET status=?,next_run_time=?,update_time=NOW() WHERE job_id=?",
                &[
                    i.value("status"),
                    datetime(next(&db::scalar(&job["cronExpression"]), Utc::now())?),
                    i.value("jobId"),
                ],
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            ok()
        }
        ("PUT", "run") if !logs => {
            a.require("monitor:job:changeStatus")?;
            a.superuser()?;
            execute(app, i.value("jobId"), true).await?;
            ok()
        }
        ("GET", id) if !id.is_empty() => {
            a.require("monitor:job:query")?;
            db::ids(id)?;
            let mut row = db::one(
                &app.pool,
                &format!("SELECT {select} FROM {table} WHERE {pk}=?"),
                &[json!(id)],
            )
            .await?;
            if row.is_null() {
                return Err(AppError::missing());
            }
            if !logs {
                row["nextValidTime"] = row["nextRunTime"].clone();
            }
            data(row)
        }
        ("DELETE", "clean") if logs => {
            a.require("monitor:job:remove")?;
            a.superuser()?;
            db::exec(&app.pool, "DELETE FROM sys_job_log", &[]).await?;
            ok()
        }
        ("DELETE", ids) => {
            a.require("monitor:job:remove")?;
            a.superuser()?;
            let ids = db::ids(ids)?;
            db::exec(
                &app.pool,
                &format!(
                    "DELETE FROM {table} WHERE {pk} IN ({})",
                    db::placeholders(ids.len())
                ),
                &ids,
            )
            .await?;
            ok()
        }
        _ => return Err(AppError::missing()),
    };
    Ok(Json(v).into_response())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cron_validation() {
        let now = DateTime::parse_from_rfc3339("2026-09-05T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            next("0 */5 * * * ?", now).unwrap().to_rfc3339(),
            "2026-09-05T12:05:00+00:00"
        );
        assert!(schedule("not cron").is_err());
        assert!(next("0 0 0 1 1 * 2020", now).is_err());
    }
}
