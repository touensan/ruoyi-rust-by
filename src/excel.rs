//! Atomic XLSX user import; role assignment is intentionally absent from the workbook schema.
use crate::{
    App, AppError, Result,
    auth::{self, Actor},
    data, db, system,
};
use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use calamine::{Reader, Xlsx};
use rust_xlsxwriter::Workbook;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read},
};
const HEADERS: [&str; 8] = [
    "登录名称",
    "用户昵称",
    "部门编号",
    "邮箱",
    "手机号码",
    "性别",
    "状态",
    "初始密码",
];
const LIMIT: usize = 500;
fn location(s: &str) -> bool {
    s.split(':').all(|p| {
        let p = p.replace('$', "");
        let split = p.find(|c: char| c.is_ascii_digit()).unwrap_or(0);
        let (col, row) = p.split_at(split);
        let n = col.bytes().try_fold(0usize, |a, b| {
            if b.is_ascii_uppercase() {
                a.checked_mul(26)?.checked_add((b - b'A' + 1) as usize)
            } else {
                None
            }
        });
        n.is_some_and(|n| (1..=8).contains(&n))
            && row
                .parse::<usize>()
                .is_ok_and(|n| (1..=LIMIT + 1).contains(&n))
    })
}
fn parse(bytes: Vec<u8>) -> Result<Vec<Vec<String>>> {
    // Inspect archive expansion and worksheet coordinates before the workbook parser allocates ranges.
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes))
        .map_err(|_| AppError::bad("请上传有效的 xlsx 工作簿；不支持旧版 xls"))?;
    if archive.len() > 100 {
        return Err(AppError::bad("工作簿内部文件过多"));
    }
    let mut total = 0u64;
    let mut names = HashSet::new();
    for n in 0..archive.len() {
        let mut f = archive
            .by_index(n)
            .map_err(|_| AppError::bad("工作簿损坏"))?;
        total += f.size();
        if total > 20 * 1024 * 1024
            || f.size() > 4 * 1024 * 1024
            || !names.insert(f.name().to_owned())
        {
            return Err(AppError::bad("工作簿解压后过大或含重复文件"));
        }
        if f.name().starts_with("xl/worksheets/") && f.name().ends_with(".xml") {
            let mut xml = Vec::new();
            f.by_ref()
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut xml)
                .map_err(|_| AppError::bad("工作簿损坏"))?;
            if xml.len() > 4 * 1024 * 1024 {
                return Err(AppError::bad("工作表过大"));
            }
            let mut reader = quick_xml::Reader::from_reader(xml.as_slice());
            let mut count = 0;
            loop {
                match reader
                    .read_event()
                    .map_err(|_| AppError::bad("工作表 XML 无效"))?
                {
                    quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                        let local = e.local_name();
                        let tag = local.as_ref();
                        if tag == b"f" {
                            return Err(AppError::bad("请使用纯值工作表，不支持公式单元格"));
                        }
                        if tag == b"row" {
                            count += 1;
                            if count > LIMIT + 1 {
                                return Err(AppError::bad("最多导入 500 行"));
                            }
                        }
                        for attr in e.attributes() {
                            let a = attr.map_err(|_| AppError::bad("工作表属性无效"))?;
                            let value = std::str::from_utf8(&a.value)
                                .map_err(|_| AppError::bad("工作表编码无效"))?;
                            if ((tag == b"dimension" && a.key.as_ref() == b"ref")
                                || (tag == b"c" && a.key.as_ref() == b"r"))
                                && !location(value)
                            {
                                return Err(AppError::bad("工作表范围超过 500 行或 8 列"));
                            }
                            if tag == b"row"
                                && a.key.as_ref() == b"r"
                                && !value
                                    .parse::<usize>()
                                    .is_ok_and(|n| (1..=LIMIT + 1).contains(&n))
                            {
                                return Err(AppError::bad("工作表行号超限"));
                            }
                        }
                    }
                    quick_xml::events::Event::Eof => break,
                    _ => (),
                }
            }
        }
    }
    let mut workbook: Xlsx<_> =
        Xlsx::new(Cursor::new(bytes)).map_err(|_| AppError::bad("工作簿格式无效"))?;
    let name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::bad("工作簿没有工作表"))?;
    let range = workbook
        .worksheet_range(&name)
        .map_err(|_| AppError::bad("无法读取工作表"))?;
    if range.height() > LIMIT + 1 || range.width() != 8 {
        return Err(AppError::bad("请使用下载的 8 列模板，最多 500 行数据"));
    }
    let rows = range
        .rows()
        .map(|r| {
            r.iter()
                .map(|v| v.to_string().trim().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if rows
        .first()
        .is_none_or(|r| r.iter().map(String::as_str).collect::<Vec<_>>() != HEADERS)
    {
        return Err(AppError::bad("表头不匹配，请下载最新用户模板"));
    }
    let out = rows
        .into_iter()
        .skip(1)
        .filter(|r| r.iter().any(|s| !s.is_empty()))
        .collect::<Vec<_>>();
    if out.is_empty() {
        return Err(AppError::bad("工作簿没有用户数据"));
    }
    Ok(out)
}
pub fn template(a: &Actor) -> Result<Response> {
    a.require("system:user:import")?;
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    for (col, name) in HEADERS.iter().enumerate() {
        sheet
            .write_string(0, col as u16, *name)
            .map_err(|_| AppError::bad("生成模板失败"))?;
        sheet
            .set_column_width(col as u16, 22)
            .map_err(|_| AppError::bad("生成模板失败"))?;
    }
    let bytes = workbook
        .save_to_buffer()
        .map_err(|_| AppError::bad("生成模板失败"))?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=user_template.xlsx",
            ),
        ],
        bytes,
    )
        .into_response())
}
pub async fn import(
    State(app): State<App>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let a = auth::actor(&app, &headers).await?;
    a.require("system:user:import")?;
    let _permit = app
        .imports
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::bad("已有导入任务正在处理，请稍后再试"))?;
    let update = match params.get("updateSupport").map(String::as_str) {
        None | Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => return Err(AppError::bad("更新选项无效")),
    };
    let field = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad("上传表单无效"))?
        .ok_or_else(|| AppError::bad("请选择工作簿"))?;
    if field.name() != Some("file")
        || !field
            .file_name()
            .unwrap_or("")
            .to_ascii_lowercase()
            .ends_with(".xlsx")
    {
        return Err(AppError::bad("请上传 .xlsx 文件"));
    }
    let bytes = field
        .bytes()
        .await
        .map_err(|_| AppError::bad("工作簿超过 5 MB"))?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::bad("工作簿超过 5 MB"));
    }
    let rows = tokio::task::spawn_blocking(move || parse(bytes.to_vec()))
        .await
        .map_err(|_| AppError::bad("工作簿处理失败"))??;
    let mut seen = HashSet::new();
    let mut prepared = Vec::new();
    for (n, r) in rows.into_iter().enumerate() {
        let invalid = || {
            AppError::bad(&format!(
                "第 {} 行字段无效：检查账号、昵称、部门、邮箱、手机号、性别和状态",
                n + 2
            ))
        };
        if r.len() != 8
            || r[0].is_empty()
            || r[0].len() > 30
            || !r[0]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
            || r[1].is_empty()
            || r[1].chars().count() > 30
            || !seen.insert(r[0].clone())
            || !r[2].parse::<i64>().is_ok_and(|n| n > 0)
            || r[3].len() > 50
            || (!r[3].is_empty() && r[3].parse::<lettre::Address>().is_err())
            || r[4].len() > 11
            || !r[4].bytes().all(|b| b.is_ascii_digit())
            || !["0", "1", "2"].contains(&r[5].as_str())
            || !["0", "1"].contains(&r[6].as_str())
        {
            return Err(invalid());
        }
        let hash = if r[7].is_empty() {
            None
        } else {
            Some(auth::hash_password(r[7].clone()).await?)
        };
        prepared.push((r, hash));
    }
    let (scope, args) = system::scope(&app, &a).await?;
    let mut tx = app.pool.begin().await?;
    let mut created = 0;
    let mut updated = 0;
    for (r, hash) in prepared {
        let dept = r[2].parse::<i64>().unwrap();
        if !a.admin() && dept != db::num(&a.user["deptId"]) {
            return Err(AppError::forbidden());
        }
        if db::query(
            "SELECT dept_id FROM sys_dept WHERE dept_id=? AND delete_time IS NULL AND status='0'",
            &[json!(dept)],
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_none()
        {
            return Err(AppError::bad("导入包含不存在或停用的部门；未写入任何用户"));
        }
        let existing = db::query(
            "SELECT user_id,delete_time FROM sys_user WHERE user_name=? FOR UPDATE",
            &[json!(r[0])],
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(db::row)
        .transpose()?;
        if let Some(user) = existing {
            a.require("system:user:edit")?;
            if !update || !user["deleteTime"].is_null() {
                return Err(AppError::bad("账号已存在或已删除；未写入任何用户"));
            }
            if db::num(&user["userId"]) == 1 {
                return Err(AppError::forbidden());
            }
            let mut scoped = args.clone();
            scoped.push(user["userId"].clone());
            if db::query(
                &format!("SELECT user_id FROM sys_user WHERE ({scope}) AND user_id=?"),
                &scoped,
            )
            .fetch_optional(&mut *tx)
            .await?
            .is_none()
            {
                return Err(AppError::forbidden());
            }
            // Imported initial passwords never overwrite existing account passwords.
            db::query("UPDATE sys_user SET nick_name=?,dept_id=?,email=?,phonenumber=?,sex=?,status=?,update_by=?,update_time=NOW() WHERE user_id=?",&[json!(r[1]),json!(dept),json!(r[3]),json!(r[4]),json!(r[5]),json!(r[6]),a.user["userName"].clone(),user["userId"].clone()]).execute(&mut *tx).await?;
            if r[6] == "1" {
                db::query(
                    "DELETE FROM sys_access_token WHERE user_id=?",
                    &[user["userId"].clone()],
                )
                .execute(&mut *tx)
                .await?;
            }
            updated += 1;
        } else {
            a.require("system:user:add")?;
            let hash = hash.ok_or_else(|| {
                AppError::bad(
                    "新用户必须填写初始密码（至少 12 字符，最多 72 字节）；未写入任何用户",
                )
            })?;
            db::query("INSERT INTO sys_user(user_name,nick_name,dept_id,email,phonenumber,sex,status,password,create_by) VALUES(?,?,?,?,?,?,?,?,?)",&[json!(r[0]),json!(r[1]),json!(dept),json!(r[3]),json!(r[4]),json!(r[5]),json!(r[6]),json!(hash),a.user["userName"].clone()]).execute(&mut *tx).await?;
            created += 1;
        }
    }
    db::query("INSERT INTO sys_oper_log(title,request_method,oper_name,oper_url,status) VALUES('Excel 用户导入','POST',?,'/api/system/user/importData','0')",&[a.user["userName"].clone()]).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut result = data(json!({"created":created,"updated":updated}));
    result["msg"] = json!(format!(
        "导入成功：新增 {created} 人，更新 {updated} 人。已有用户密码和角色保持不变。"
    ));
    Ok(Json(result))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds() {
        assert!(location("A1:H501"));
        assert!(!location("A1:H9999999"));
        assert!(!location("XFD1"));
        assert!(!location("A0"));
    }
    #[test]
    fn invalid_file() {
        assert!(parse(b"not a workbook".to_vec()).is_err());
    }
}
