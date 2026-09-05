//! Metadata-only generation: never executes submitted SQL or writes generated code into the server.
use crate::{App, AppError, Input, Result, auth::Actor, data, db, ok};
use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};
async fn tables(app: &App) -> Result<Vec<Value>> {
    db::rows(&app.pool,"SELECT TABLE_NAME AS table_name,TABLE_COMMENT AS table_comment,CREATE_TIME AS create_time FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_TYPE='BASE TABLE' AND TABLE_NAME NOT LIKE '\\_%' ORDER BY TABLE_NAME",&[]).await
}
async fn table(app: &App, name: &str) -> Result<()> {
    if !db::identifiers(name) || !tables(app).await?.iter().any(|r| r["tableName"] == name) {
        return Err(AppError::bad("数据库表名无效"));
    }
    Ok(())
}
async fn columns(app: &App, name: &str) -> Result<Vec<Value>> {
    table(app, name).await?;
    let rows=db::rows(&app.pool,"SELECT COLUMN_NAME AS column_name,COLUMN_TYPE AS column_type,DATA_TYPE AS data_type,COLUMN_COMMENT AS column_comment,IS_NULLABLE AS nullable,COLUMN_KEY AS key_type,EXTRA AS extra FROM information_schema.COLUMNS WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=? ORDER BY ORDINAL_POSITION",&[json!(name)]).await?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, mut r)| {
            r["columnId"] = json!(i + 1);
            r["sort"] = json!(i + 1);
            r["javaField"] = json!(db::camel(&db::scalar(&r["columnName"])));
            r["javaType"] = json!(rust_type(&r));
            r["isPk"] = json!(if r["keyType"] == "PRI" { "1" } else { "0" });
            r["isIncrement"] = json!(if r["extra"] == "auto_increment" {
                "1"
            } else {
                "0"
            });
            for k in ["isInsert", "isEdit", "isList"] {
                r[k] = json!("1");
            }
            for k in ["isRequired", "isQuery"] {
                r[k] = json!("0");
            }
            r["queryType"] = json!("EQ");
            r["htmlType"] = json!("input");
            r
        })
        .collect())
}
fn rust_type(c: &Value) -> String {
    let base = match db::scalar(&c["dataType"]).as_str() {
        "bigint" | "int" | "smallint" | "tinyint" | "mediumint" => {
            if db::scalar(&c["columnType"]).contains("unsigned") {
                "u64"
            } else {
                "i64"
            }
        }
        "float" | "double" => "f64",
        "datetime" | "timestamp" => "chrono::NaiveDateTime",
        "date" => "chrono::NaiveDate",
        "blob" | "binary" | "varbinary" => "Vec<u8>",
        _ => "String",
    };
    if c["nullable"] == "YES" {
        format!("Option<{base}>")
    } else {
        base.into()
    }
}
async fn record(app: &App, id: &str) -> Result<Value> {
    let r = db::one(
        &app.pool,
        "SELECT * FROM gen_table WHERE table_id=?",
        &[json!(id)],
    )
    .await?;
    if r.is_null() {
        return Err(AppError::missing());
    }
    Ok(r)
}
fn class_valid(name: &str) -> bool {
    db::identifiers(name) && name.as_bytes()[0].is_ascii_uppercase()
}
async fn files(app: &App, r: &Value) -> Result<BTreeMap<String, String>> {
    let name = db::scalar(&r["tableName"]);
    let class = db::scalar(&r["className"]);
    if !class_valid(&class) {
        return Err(AppError::bad("类名必须以大写字母开头"));
    }
    let cols = columns(app, &name).await?;
    let pks = cols.iter().filter(|c| c["isPk"] == "1").collect::<Vec<_>>();
    if pks.len() != 1 {
        return Err(AppError::bad("首版生成器要求单列主键"));
    }
    let pk = db::scalar(&pks[0]["columnName"]);
    let safe = cols
        .iter()
        .filter(|c| {
            let s = db::scalar(&c["columnName"]).to_lowercase();
            ![
                "password",
                "secret",
                "token",
                "private_key",
                "setting_value",
            ]
            .iter()
            .any(|w| s.contains(w))
        })
        .collect::<Vec<_>>();
    if safe.is_empty() {
        return Err(AppError::bad("没有可生成的公开字段"));
    }
    let fieldnames = safe
        .iter()
        .map(|c| {
            let name = db::scalar(&c["columnName"]);
            if ["decimal", "json", "enum", "set"].contains(&db::scalar(&c["dataType"]).as_str()) {
                format!("CAST(`{name}` AS CHAR) AS `{name}`")
            } else {
                format!("`{name}`")
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let fields = safe
        .iter()
        .map(|c| {
            format!(
                "    pub r#{}: {},",
                db::scalar(&c["columnName"]),
                rust_type(c)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let model = format!(
        "// Generated from {name}; review column types and data access before integration.\n#[derive(Debug, serde::Serialize, sqlx::FromRow)]\n#[serde(rename_all = \"camelCase\")]\npub struct {class} {{\n{fields}\n}}\n"
    );
    let service = format!(
        "// Add authorization and data-scope conditions before exposing these queries.\nuse sqlx::MySqlPool;\nuse super::model::{class};\n\npub async fn list(pool: &MySqlPool, page: i64, size: i64) -> Result<Vec<{class}>, sqlx::Error> {{\n    let size = size.clamp(1, 100);\n    sqlx::query_as::<_, {class}>(\"SELECT {fieldnames} FROM `{name}` ORDER BY `{pk}` LIMIT ? OFFSET ?\")\n        .bind(size).bind((page.clamp(1, 100000) - 1) * size).fetch_all(pool).await\n}}\n"
    );
    let readme = format!(
        "# {class} 生成结果\n\n表：{name}。包含 SQLx 数据模型和分页查询示例；不是完整 CRUD，也不会自动注册路由或部署。\n\n接入前请核对 unsigned/decimal/enum 等列类型，补充写入验证、按钮权限和数据范围。密码、令牌、密钥及系统配置内容列已排除。列表列配置只展示实时元数据，首版不提供持久化列编辑。\n"
    );
    Ok(BTreeMap::from([
        ("model.rs".into(), model),
        ("service.rs".into(), service),
        ("README.md".into(), readme),
    ]))
}
pub async fn handle(app: &App, a: &Actor, i: &Input) -> Result<Response> {
    a.require("tool:gen:list")?;
    a.superuser()?;
    let suffix = i
        .path
        .strip_prefix("tool/gen")
        .unwrap_or("")
        .trim_matches('/');
    let v = match (i.method.as_str(), suffix) {
        ("GET", "db/list") => {
            let rows = tables(app)
                .await?
                .into_iter()
                .filter(|r| db::scalar(&r["tableName"]).contains(&i.s("tableName")))
                .collect::<Vec<_>>();
            let size = i.s("pageSize").parse::<usize>().unwrap_or(10).clamp(1, 100);
            let page = i
                .s("pageNum")
                .parse::<usize>()
                .unwrap_or(1)
                .clamp(1, 100000);
            json!({"code":200,"total":rows.len(),"rows":rows.into_iter().skip((page-1)*size).take(size).collect::<Vec<_>>()})
        }
        ("GET", "list") => {
            let rows = db::rows(
                &app.pool,
                "SELECT * FROM gen_table WHERE table_name LIKE ? ORDER BY table_id LIMIT 100",
                &[json!(format!("%{}%", i.s("tableName")))],
            )
            .await?;
            json!({"code":200,"total":rows.len(),"rows":rows})
        }
        ("POST", "importTable") => {
            let names = i.s("tables");
            let names = names.split(',').collect::<Vec<_>>();
            if names.len() > 20 {
                return Err(AppError::bad("一次最多导入 20 张表"));
            }
            for name in &names {
                table(app, name).await?;
            }
            let mut tx = app.pool.begin().await?;
            for name in names {
                let class = db::camel(&format!("_{name}"));
                db::query("INSERT IGNORE INTO gen_table (table_name,table_comment,class_name,business_name,function_name) VALUES (?,?,?,?,?)",&[json!(name),json!(name),json!(class),json!(name),json!(name)]).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            ok()
        }
        ("POST", "createTable") => {
            return Err(AppError::bad("请通过 migrations 创建表，再导入元数据"));
        }
        ("GET", p) if p.starts_with("preview/") => data(json!(
            files(app, &record(app, p.trim_start_matches("preview/")).await?).await?
        )),
        ("GET", p) if p.starts_with("synchDb/") => {
            let name = p.trim_start_matches("synchDb/");
            columns(app, name).await?;
            ok()
        }
        ("GET", "batchGenCode") | ("GET", _)
            if suffix.starts_with("genCode/") || suffix == "batchGenCode" =>
        {
            let names = if suffix == "batchGenCode" {
                i.s("tables")
            } else {
                suffix.trim_start_matches("genCode/").into()
            };
            let names = names.split(',').collect::<Vec<_>>();
            if names.len() > 20 {
                return Err(AppError::bad("每次最多 20 张表"));
            }
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            for name in names {
                table(app, name).await?;
                let r = db::one(
                    &app.pool,
                    "SELECT * FROM gen_table WHERE table_name=?",
                    &[json!(name)],
                )
                .await?;
                if r.is_null() {
                    return Err(AppError::missing());
                }
                for (path, text) in files(app, &r).await? {
                    zip.start_file(
                        format!("{name}/{path}"),
                        zip::write::SimpleFileOptions::default(),
                    )
                    .map_err(|_| AppError::bad("压缩失败"))?;
                    zip.write_all(text.as_bytes())
                        .map_err(|_| AppError::bad("压缩失败"))?;
                }
            }
            let bytes = zip
                .finish()
                .map_err(|_| AppError::bad("压缩失败"))?
                .into_inner();
            return Ok((
                [
                    (header::CONTENT_TYPE, "application/zip"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=ruoyi-rust-by-generated.zip",
                    ),
                ],
                bytes,
            )
                .into_response());
        }
        ("PUT", "") => {
            record(app, &i.s("tableId")).await?;
            if !class_valid(&i.s("className"))
                || !db::identifiers(&i.s("moduleName"))
                || !db::identifiers(&i.s("businessName"))
            {
                return Err(AppError::bad("类名、模块名、业务名无效"));
            }
            for k in ["tableComment", "functionName", "functionAuthor"] {
                if i.s(k).chars().count() > 100 {
                    return Err(AppError::bad("描述过长"));
                }
            }
            db::exec(&app.pool,"UPDATE gen_table SET class_name=?,module_name=?,business_name=?,table_comment=?,function_name=?,function_author=? WHERE table_id=?",&[i.value("className"),i.value("moduleName"),i.value("businessName"),i.value("tableComment"),i.value("functionName"),i.value("functionAuthor"),i.value("tableId")]).await?;
            ok()
        }
        ("DELETE", ids) => {
            let ids = db::ids(ids)?;
            db::exec(
                &app.pool,
                &format!(
                    "DELETE FROM gen_table WHERE table_id IN ({})",
                    db::placeholders(ids.len())
                ),
                &ids,
            )
            .await?;
            ok()
        }
        ("GET", id) if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) => {
            let r = record(app, id).await?;
            data(
                json!({"rows":columns(app,&db::scalar(&r["tableName"])).await?,"info":r,"tables":db::rows(&app.pool,"SELECT * FROM gen_table ORDER BY table_id LIMIT 100",&[]).await?}),
            )
        }
        _ => return Err(AppError::missing()),
    };
    Ok(Json(v).into_response())
}
