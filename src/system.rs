use crate::{
    App, AppError, Input, Result,
    auth::{self, Actor},
    data, db, ok,
};
use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub struct Entity {
    pub path: &'static str,
    pub table: &'static str,
    pub pk: &'static str,
    pub perm: &'static str,
}
pub const ENTITIES: [Entity; 11] = [
    Entity {
        path: "system/dict/type",
        table: "sys_dict_type",
        pk: "dict_id",
        perm: "system:dict",
    },
    Entity {
        path: "system/dict/data",
        table: "sys_dict_data",
        pk: "dict_code",
        perm: "system:dict",
    },
    Entity {
        path: "system/user",
        table: "sys_user",
        pk: "user_id",
        perm: "system:user",
    },
    Entity {
        path: "system/role",
        table: "sys_role",
        pk: "role_id",
        perm: "system:role",
    },
    Entity {
        path: "system/menu",
        table: "sys_menu",
        pk: "menu_id",
        perm: "system:menu",
    },
    Entity {
        path: "system/dept",
        table: "sys_dept",
        pk: "dept_id",
        perm: "system:dept",
    },
    Entity {
        path: "system/post",
        table: "sys_post",
        pk: "post_id",
        perm: "system:post",
    },
    Entity {
        path: "system/config",
        table: "sys_config",
        pk: "config_id",
        perm: "system:config",
    },
    Entity {
        path: "system/notice",
        table: "sys_notice",
        pk: "notice_id",
        perm: "system:notice",
    },
    Entity {
        path: "monitor/operlog",
        table: "sys_oper_log",
        pk: "oper_id",
        perm: "monitor:operlog",
    },
    Entity {
        path: "monitor/logininfor",
        table: "sys_logininfor",
        pk: "info_id",
        perm: "monitor:logininfor",
    },
];
impl Entity {
    fn protected(&self) -> bool {
        ["sys_role", "sys_menu", "sys_dept"].contains(&self.table)
    }
    fn log(&self) -> bool {
        self.path.starts_with("monitor/")
    }
}
pub fn columns(app: &App, e: Entity) -> &Vec<Value> {
    app.schema[e.table]["columns"].as_array().unwrap()
}
fn has(app: &App, e: Entity, c: &str) -> bool {
    columns(app, e).iter().any(|v| v["name"] == c)
}

pub async fn scope(app: &App, a: &Actor) -> Result<(String, Vec<Value>)> {
    if a.admin() || a.roles.iter().any(|r| db::scalar(&r["dataScope"]) == "1") {
        return Ok(("1=1".into(), vec![]));
    }
    let mut depts = Vec::new();
    for r in &a.roles {
        match db::scalar(&r["dataScope"]).as_str() {
            "2" => {
                for d in db::rows(
                    &app.pool,
                    "SELECT dept_id FROM sys_role_dept WHERE role_id=?",
                    &[r["roleId"].clone()],
                )
                .await?
                {
                    depts.push(d["deptId"].clone());
                }
            }
            "3" | "4" => {
                depts.push(a.user["deptId"].clone());
                if db::scalar(&r["dataScope"]) == "4" {
                    for d in db::rows(&app.pool,"SELECT dept_id FROM sys_dept WHERE FIND_IN_SET(?,ancestors)>0 AND delete_time IS NULL",&[a.user["deptId"].clone()]).await?{depts.push(d["deptId"].clone());}
                }
            }
            _ => {}
        }
    }
    let mut args = vec![json!(a.id())];
    let sql = if depts.is_empty() {
        "user_id=?".into()
    } else {
        let s = format!(
            "(user_id=? OR dept_id IN ({}))",
            db::placeholders(depts.len())
        );
        args.extend(depts);
        s
    };
    Ok((sql, args))
}
async fn base(app: &App, a: &Actor, e: Entity) -> Result<(String, Vec<Value>)> {
    let mut cond = if has(app, e, "delete_time") {
        "delete_time IS NULL".into()
    } else {
        "1=1".into()
    };
    let mut args = vec![];
    if e.table == "sys_user" {
        let (s, v) = scope(app, a).await?;
        cond = format!("{cond} AND ({s})");
        args = v;
    }
    Ok((cond, args))
}
pub async fn user_access(app: &App, a: &Actor, id: Value) -> Result<Value> {
    let (cond, mut args) = scope(app, a).await?;
    args.push(id);
    let row = db::one(
        &app.pool,
        &format!("SELECT * FROM sys_user WHERE delete_time IS NULL AND ({cond}) AND user_id=?"),
        &args,
    )
    .await?;
    if row.is_null() {
        return Err(AppError::missing());
    }
    Ok(row)
}
pub async fn output(app: &App, e: Entity, row: &Value) -> Result<Value> {
    let mut r = db::public(row);
    if r.is_null() {
        return Ok(r);
    }
    if e.table == "sys_user" {
        r["dept"] = db::one(
            &app.pool,
            "SELECT * FROM sys_dept WHERE dept_id=? AND delete_time IS NULL",
            &[row["deptId"].clone()],
        )
        .await?;
    }
    if e.table == "sys_role" {
        r["admin"] = json!(db::num(&r["roleId"]) == 1);
        for k in ["menuCheckStrictly", "deptCheckStrictly"] {
            r[k] = json!(db::num(&r[k]) != 0);
        }
    }
    Ok(r)
}
pub async fn handle(app: &App, a: &Actor, i: &Input) -> Result<Response> {
    let e = ENTITIES
        .iter()
        .find(|e| i.path == e.path || i.path.starts_with(&format!("{}/", e.path)))
        .copied()
        .ok_or_else(AppError::missing)?;
    let suffix = i.path.strip_prefix(e.path).unwrap().trim_matches('/');
    let v = match (i.method.as_str(), suffix) {
        ("GET", "list") => listing(app, a, i, e).await?,
        ("GET", id) if id.is_empty() || id.bytes().all(|b| b.is_ascii_digit()) => {
            detail(app, a, e, id).await?
        }
        ("POST", "") | ("PUT", "") | ("PUT", "changeStatus") | ("PUT", "dataScope") => {
            save(app, a, i, e).await?
        }
        ("DELETE", "refreshCache") => {
            a.require(&format!("{}:remove", e.perm))?;
            ok()
        }
        ("DELETE", "clean") if e.log() => {
            a.require(&format!("{}:remove", e.perm))?;
            db::exec(&app.pool, &format!("DELETE FROM {}", e.table), &[]).await?;
            ok()
        }
        ("DELETE", ids) => remove(app, a, e, ids).await?,
        ("POST", "export") => {
            a.require(&format!("{}:export", e.perm))?;
            let (cond, args) = filtered(app, a, i, e).await?;
            let rows = db::rows(
                &app.pool,
                &format!(
                    "SELECT * FROM {} WHERE {cond} ORDER BY {} LIMIT 10001",
                    e.table, e.pk
                ),
                &args,
            )
            .await?;
            if rows.len() > 10000 {
                return Err(AppError::bad("导出最多 10000 条，请缩小筛选范围"));
            }
            let mut csv = String::from("\u{feff}");
            let fields: Vec<_> = columns(app, e)
                .iter()
                .map(|c| db::camel(&db::scalar(&c["name"])))
                .filter(|k| !["password", "tokenHash", "deleteTime"].contains(&k.as_str()))
                .collect();
            csv.push_str(&fields.join(","));
            csv.push_str("\r\n");
            for row in rows {
                csv.push_str(
                    &fields
                        .iter()
                        .map(|k| csv_cell(&db::scalar(&row[k])))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                csv.push_str("\r\n");
            }
            return Ok((
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=export.csv",
                    ),
                ],
                csv,
            )
                .into_response());
        }
        _ => return Err(AppError::missing()),
    };
    Ok(Json(v).into_response())
}
fn csv_cell(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}
async fn filtered(app: &App, a: &Actor, i: &Input, e: Entity) -> Result<(String, Vec<Value>)> {
    let (mut cond, mut args) = base(app, a, e).await?;
    let mut entries = i.params.clone();
    if let Some(o) = i.body.as_object() {
        for (k, v) in o {
            if v.is_string() || v.is_number() {
                entries.insert(k.clone(), db::scalar(v));
            }
        }
    }
    for (k, v) in entries {
        let c = db::snake(&k);
        if v.is_empty()
            || !has(app, e, &c)
            || ["password", "token_hash", "delete_time"].contains(&c.as_str())
        {
            continue;
        }
        if c.ends_with("_id") || ["status", "sex", "dict_type", "notice_type"].contains(&c.as_str())
        {
            cond.push_str(&format!(" AND `{c}`=?"));
            args.push(json!(v));
        } else {
            cond.push_str(&format!(" AND `{c}` LIKE ?"));
            args.push(json!(format!("%{v}%")));
        }
    }
    let time = if has(app, e, "create_time") {
        Some("create_time")
    } else if e.table == "sys_logininfor" {
        Some("login_time")
    } else if e.table == "sys_oper_log" {
        Some("oper_time")
    } else {
        None
    };
    if let Some(time) = time {
        for (k, op, tail) in [
            ("params[beginTime]", ">=", " 00:00:00"),
            ("params[endTime]", "<=", " 23:59:59"),
        ] {
            if let Some(v) = i.params.get(k) {
                if chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                    return Err(AppError::bad("日期格式错误"));
                }
                cond.push_str(&format!(" AND {time}{op}?"));
                args.push(json!(format!("{v}{tail}")));
            }
        }
    }
    Ok((cond, args))
}
async fn listing(app: &App, a: &Actor, i: &Input, e: Entity) -> Result<Value> {
    a.require(&format!("{}:list", e.perm))?;
    let (cond, mut args) = filtered(app, a, i, e).await?;
    let count = db::one(
        &app.pool,
        &format!("SELECT COUNT(*) AS total FROM {} WHERE {cond}", e.table),
        &args,
    )
    .await?;
    let tree = ["sys_menu", "sys_dept"].contains(&e.table);
    let size = if tree {
        5000
    } else {
        i.s("pageSize").parse::<i64>().unwrap_or(10).clamp(1, 100)
    };
    let page = i.s("pageNum").parse::<i64>().unwrap_or(1).clamp(1, 100000);
    let sort = if has(app, e, "order_num") {
        "order_num"
    } else {
        e.pk
    };
    args.extend([json!(size), json!(if tree { 0 } else { (page - 1) * size })]);
    let rows = db::rows(
        &app.pool,
        &format!(
            "SELECT * FROM {} WHERE {cond} ORDER BY {sort} LIMIT ? OFFSET ?",
            e.table
        ),
        &args,
    )
    .await?;
    let mut out = vec![];
    for r in rows {
        out.push(output(app, e, &r).await?);
    }
    Ok(if tree {
        data(json!(out))
    } else {
        json!({"code":200,"rows":out,"total":count["total"]})
    })
}
async fn detail(app: &App, a: &Actor, e: Entity, id: &str) -> Result<Value> {
    a.require(&format!("{}:query", e.perm))?;
    let row = if id.is_empty() {
        Value::Null
    } else {
        let (cond, mut args) = base(app, a, e).await?;
        args.push(json!(id));
        let r = db::one(
            &app.pool,
            &format!("SELECT * FROM {} WHERE {cond} AND {}=?", e.table, e.pk),
            &args,
        )
        .await?;
        if r.is_null() {
            return Err(AppError::missing());
        }
        r
    };
    let mut v = data(output(app, e, &row).await?);
    if e.table == "sys_user" {
        let mut roles = vec![];
        for r in db::rows(
            &app.pool,
            "SELECT * FROM sys_role WHERE status='0' AND delete_time IS NULL",
            &[],
        )
        .await?
        {
            roles.push(output(app, ENTITIES[3], &r).await?);
        }
        v["roles"] = json!(roles);
        v["posts"] = json!(
            db::rows(
                &app.pool,
                "SELECT * FROM sys_post WHERE status='0' AND delete_time IS NULL",
                &[]
            )
            .await?
        );
        v["roleIds"] = json!(
            db::rows(
                &app.pool,
                "SELECT role_id FROM sys_user_role WHERE user_id=?",
                &[json!(id)]
            )
            .await?
            .iter()
            .map(|r| r["roleId"].clone())
            .collect::<Vec<_>>()
        );
        v["postIds"] = json!(
            db::rows(
                &app.pool,
                "SELECT post_id FROM sys_user_post WHERE user_id=?",
                &[json!(id)]
            )
            .await?
            .iter()
            .map(|r| r["postId"].clone())
            .collect::<Vec<_>>()
        );
    }
    Ok(v)
}
async fn save(app: &App, a: &Actor, i: &Input, e: Entity) -> Result<Value> {
    if e.log() {
        return Err(AppError::missing());
    }
    let update = i.method == axum::http::Method::PUT;
    a.require(&format!(
        "{}:{}",
        e.perm,
        if update { "edit" } else { "add" }
    ))?;
    if e.protected() || i.body.get("roleIds").is_some() {
        a.superuser()?;
    }
    let id = i.value(&db::camel(e.pk));
    if update && db::num(&id) <= 0 {
        return Err(AppError::bad("缺少 ID"));
    }
    if update && ["sys_user", "sys_role"].contains(&e.table) && db::num(&id) == 1 {
        return Err(AppError::bad("超级管理员请通过个人资料维护"));
    }
    if update {
        let (cond, mut args) = base(app, a, e).await?;
        args.push(id.clone());
        if db::one(
            &app.pool,
            &format!(
                "SELECT {} FROM {} WHERE {cond} AND {}=?",
                e.pk, e.table, e.pk
            ),
            &args,
        )
        .await?
        .is_null()
        {
            return Err(AppError::missing());
        }
    }
    let object = i
        .body
        .as_object()
        .ok_or_else(|| AppError::bad("请求必须是 JSON 对象"))?;
    let mut fields = Vec::new();
    let mut vals = Vec::new();
    for c in columns(app, e) {
        let name = db::scalar(&c["name"]);
        let key = db::camel(&name);
        if [
            e.pk,
            "password",
            "create_by",
            "create_time",
            "update_by",
            "update_time",
            "delete_time",
            "user_type",
            "login_ip",
            "login_date",
            "avatar",
            "ancestors",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        if let Some(v) = object.get(&key) {
            if v.is_null() {
                if c["nullable"] == true {
                    fields.push(name);
                    vals.push(Value::Null);
                }
                continue;
            }
            if !(v.is_string() || v.is_number() || v.is_boolean()) {
                return Err(AppError::bad("字段类型错误"));
            }
            let text = db::scalar(v);
            let ty = db::scalar(&c["type"]);
            if ["BIGINT", "INT", "TINYINT", "SMALLINT"].contains(&ty.as_str()) {
                if !(v.is_boolean() || text.parse::<i64>().is_ok()) {
                    return Err(AppError::bad("数字字段格式错误"));
                }
            } else if !ty.contains("TEXT")
                && c["size"].as_u64().unwrap_or(0) > 0
                && text.chars().count() > c["size"].as_u64().unwrap() as usize
            {
                return Err(AppError::bad("字段超过允许长度"));
            }
            if name == "status" && !["0", "1"].contains(&text.as_str()) {
                return Err(AppError::bad("状态无效"));
            }
            if name == "data_scope" && !["1", "2", "3", "4", "5"].contains(&text.as_str()) {
                return Err(AppError::bad("数据范围无效"));
            }
            if name == "menu_type" && !["M", "C", "F"].contains(&text.as_str()) {
                return Err(AppError::bad("菜单类型无效"));
            }
            if [
                "user_name",
                "role_key",
                "dict_type",
                "post_code",
                "config_key",
            ]
            .contains(&name.as_str())
                && (text.is_empty()
                    || !text
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-.:".contains(&c)))
            {
                return Err(AppError::bad(
                    "标识仅允许字母、数字、下划线、点、冒号和短横线",
                ));
            }
            fields.push(name);
            vals.push(v.clone());
        } else if !update && c["nullable"] == false && c["default"].is_null() && c["auto"] == false
        {
            return Err(AppError::bad(&format!("缺少字段 {key}")));
        }
    }
    if e.table == "sys_user" {
        if update && !i.s("password").is_empty() {
            a.require("system:user:resetPwd")?;
        }
        if !update || !i.s("password").is_empty() {
            fields.push("password".into());
            vals.push(json!(auth::hash_password(i.s("password")).await?));
        }
        if !a.admin() {
            if i.body.get("deptId").is_some()
                && db::num(&i.value("deptId")) != db::num(&a.user["deptId"])
            {
                return Err(AppError::forbidden());
            }
            if !update && !fields.contains(&"dept_id".into()) {
                fields.push("dept_id".into());
                vals.push(a.user["deptId"].clone());
            }
        }
        if fields.contains(&"dept_id".into()) {
            let dept = &vals[fields.iter().position(|s| s == "dept_id").unwrap()];
            if db::one(
                &app.pool,
                "SELECT dept_id FROM sys_dept WHERE dept_id=? AND delete_time IS NULL",
                std::slice::from_ref(dept),
            )
            .await?
            .is_null()
            {
                return Err(AppError::bad("部门不存在"));
            }
        }
    }
    let rels: Vec<(&str, &str, &str, &str)> = match e.table {
        "sys_user" => vec![
            ("roleIds", "sys_user_role", "sys_role", "role_id"),
            ("postIds", "sys_user_post", "sys_post", "post_id"),
        ],
        "sys_role" => vec![
            ("menuIds", "sys_role_menu", "sys_menu", "menu_id"),
            ("deptIds", "sys_role_dept", "sys_dept", "dept_id"),
        ],
        _ => vec![],
    };
    for (field, _, target, pk) in &rels {
        if let Some(v) = i.body.get(*field) {
            let arr = v
                .as_array()
                .ok_or_else(|| AppError::bad("关联 ID 必须是数组"))?;
            if arr.len() > 1000 {
                return Err(AppError::bad("关联项过多"));
            }
            for id in arr {
                if db::num(id) <= 0
                    || db::one(
                        &app.pool,
                        &format!("SELECT {pk} FROM {target} WHERE {pk}=? AND delete_time IS NULL"),
                        std::slice::from_ref(id),
                    )
                    .await?
                    .is_null()
                {
                    return Err(AppError::bad("关联记录不存在"));
                }
            }
        }
    }
    let mut tx = app.pool.begin().await?;
    if ["sys_menu", "sys_dept"].contains(&e.table) && fields.contains(&"parent_id".into()) {
        let parent = db::num(&vals[fields.iter().position(|s| s == "parent_id").unwrap()]);
        let all = db::query(
            &format!(
                "SELECT * FROM {} WHERE delete_time IS NULL FOR UPDATE",
                e.table
            ),
            &[],
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(db::row)
        .collect::<Result<Vec<_>>>()?;
        let mut cursor = parent;
        let mut seen = vec![db::num(&id)];
        let mut chain = vec![];
        while cursor != 0 {
            if seen.contains(&cursor) || seen.len() > 32 {
                return Err(AppError::bad("上级形成循环或层级超过 32"));
            }
            seen.push(cursor);
            chain.insert(0, cursor);
            let r = all
                .iter()
                .find(|r| db::num(&r[&db::camel(e.pk)]) == cursor)
                .ok_or_else(|| AppError::bad("上级不存在"))?;
            cursor = db::num(&r["parentId"]);
        }
        if e.table == "sys_dept" {
            chain.insert(0, 0);
            fields.push("ancestors".into());
            vals.push(json!(
                chain
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
    if fields.is_empty() {
        return Err(AppError::bad("没有可修改字段"));
    }
    let saved_id = if update {
        let mut sets = fields
            .iter()
            .map(|f| format!("`{f}`=?"))
            .collect::<Vec<_>>();
        if has(app, e, "update_time") {
            sets.push("update_time=NOW()".into());
        }
        if has(app, e, "update_by") {
            sets.push("update_by=?".into());
            vals.push(a.user["userName"].clone());
        }
        vals.push(id.clone());
        db::query(
            &format!("UPDATE {} SET {} WHERE {}=?", e.table, sets.join(","), e.pk),
            &vals,
        )
        .execute(&mut *tx)
        .await?;
        db::num(&id)
    } else {
        if has(app, e, "create_by") {
            fields.push("create_by".into());
            vals.push(a.user["userName"].clone());
        }
        db::query(
            &format!(
                "INSERT INTO {} ({}) VALUES ({})",
                e.table,
                fields
                    .iter()
                    .map(|f| format!("`{f}`"))
                    .collect::<Vec<_>>()
                    .join(","),
                db::placeholders(vals.len())
            ),
            &vals,
        )
        .execute(&mut *tx)
        .await?
        .last_insert_id() as i64
    };
    for (field, relation, _, pk) in rels {
        if let Some(arr) = i.body.get(field).and_then(Value::as_array) {
            db::query(
                &format!("DELETE FROM {relation} WHERE {}=?", e.pk),
                &[json!(saved_id)],
            )
            .execute(&mut *tx)
            .await?;
            let mut seen = vec![];
            for rid in arr {
                if seen.contains(&db::num(rid)) {
                    continue;
                }
                seen.push(db::num(rid));
                db::query(
                    &format!("INSERT INTO {relation} ({},{pk}) VALUES (?,?)", e.pk),
                    &[json!(saved_id), rid.clone()],
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    if e.table == "sys_user" && (fields.contains(&"password".into()) || i.s("status") == "1") {
        db::query(
            "DELETE FROM sys_access_token WHERE user_id=?",
            &[json!(saved_id)],
        )
        .execute(&mut *tx)
        .await?;
    }
    if e.table == "sys_dept" {
        let all = db::query(
            "SELECT dept_id,parent_id FROM sys_dept WHERE delete_time IS NULL",
            &[],
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(db::row)
        .collect::<Result<Vec<_>>>()?;
        for d in &all {
            let mut chain = vec![];
            let mut cur = db::num(&d["parentId"]);
            while cur != 0 {
                if chain.len() > 32 || chain.contains(&cur) {
                    return Err(AppError::bad("部门层级无效"));
                }
                chain.insert(0, cur);
                cur = all
                    .iter()
                    .find(|d| db::num(&d["deptId"]) == cur)
                    .map(|d| db::num(&d["parentId"]))
                    .unwrap_or(0);
            }
            chain.insert(0, 0);
            db::query(
                "UPDATE sys_dept SET ancestors=? WHERE dept_id=?",
                &[
                    json!(
                        chain
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    d["deptId"].clone(),
                ],
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(data(json!(saved_id)))
}
async fn remove(app: &App, a: &Actor, e: Entity, input: &str) -> Result<Value> {
    a.require(&format!("{}:remove", e.perm))?;
    if e.protected() {
        a.superuser()?;
    }
    let ids = db::ids(input)?;
    if ["sys_user", "sys_role"].contains(&e.table) && ids.iter().any(|v| db::num(v) == 1) {
        return Err(AppError::forbidden());
    }
    let marks = db::placeholders(ids.len());
    let (cond, mut args) = base(app, a, e).await?;
    args.extend(ids.clone());
    let found = db::rows(
        &app.pool,
        &format!(
            "SELECT {} FROM {} WHERE {cond} AND {} IN ({marks})",
            e.pk, e.table, e.pk
        ),
        &args,
    )
    .await?;
    if found.len() != ids.len() {
        return Err(AppError::missing());
    }
    if ["sys_dept", "sys_menu"].contains(&e.table)
        && !db::one(
            &app.pool,
            &format!(
                "SELECT {} FROM {} WHERE parent_id IN ({marks}) AND delete_time IS NULL LIMIT 1",
                e.pk, e.table
            ),
            &ids,
        )
        .await?
        .is_null()
    {
        return Err(AppError::bad("请先删除子级"));
    }
    if e.table=="sys_dept"&&!db::one(&app.pool,&format!("SELECT user_id FROM sys_user WHERE dept_id IN ({marks}) AND delete_time IS NULL LIMIT 1"),&ids).await?.is_null(){return Err(AppError::bad("部门仍有用户"));}
    if e.table == "sys_dict_type" {
        for row in db::rows(
            &app.pool,
            &format!("SELECT dict_type FROM sys_dict_type WHERE dict_id IN ({marks})"),
            &ids,
        )
        .await?
        {
            if !db::one(
                &app.pool,
                "SELECT dict_code FROM sys_dict_data WHERE dict_type=? LIMIT 1",
                &[row["dictType"].clone()],
            )
            .await?
            .is_null()
            {
                return Err(AppError::bad("请先删除字典数据"));
            }
        }
    }
    let mut tx = app.pool.begin().await?;
    let rels = match e.table {
        "sys_user" => vec![
            "sys_user_role",
            "sys_user_post",
            "sys_access_token",
            "sys_notice_read",
        ],
        "sys_role" => vec!["sys_user_role", "sys_role_menu", "sys_role_dept"],
        "sys_menu" => vec!["sys_role_menu"],
        "sys_dept" => vec!["sys_role_dept"],
        "sys_post" => vec!["sys_user_post"],
        "sys_notice" => vec!["sys_notice_read"],
        _ => vec![],
    };
    for t in rels {
        db::query(
            &format!("DELETE FROM {t} WHERE {} IN ({marks})", e.pk),
            &ids,
        )
        .execute(&mut *tx)
        .await?;
    }
    let sql = if has(app, e, "delete_time") {
        format!(
            "UPDATE {} SET delete_time=NOW() WHERE {} IN ({marks})",
            e.table, e.pk
        )
    } else {
        format!("DELETE FROM {} WHERE {} IN ({marks})", e.table, e.pk)
    };
    db::query(&sql, &ids).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_formula_escape() {
        assert_eq!(csv_cell("=1+2"), "\"'=1+2\"");
        assert_eq!(csv_cell("a\"b"), "\"a\"\"b\"");
    }
    #[test]
    fn ids_are_numeric() {
        assert!(db::ids("1,2 OR 1=1").is_err());
        assert_eq!(db::ids("1,1,2").unwrap().len(), 2);
    }
}
