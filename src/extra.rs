use crate::{
    App, AppError, Input, Result,
    auth::{self, Actor},
    data, db, ok, system,
};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use axum::{
    Json,
    extract::{Multipart, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::RngCore;
use serde_json::{Value, json};

pub fn defaults(group: &str) -> Value {
    match group {
        "site" => {
            json!({"title":"RuoYi-Rust BY","logo":"","favicon":"","description":"Rust / Axum 后台管理系统","keywords":"","frontendHeadCode":"","siteUrl":"","icpNo":"","publicSecurityNo":"","customerServiceEmail":"","copyright":"RuoYi-Rust BY","defaultLanguage":"zh-CN","enableSeo":true})
        }
        "payment" => {
            json!({"enabled":false,"provider":"epay","epayVersion":"v1","gatewayUrl":"","merchantId":"","merchantKey":"","merchantPrivateKey":"","platformPublicKey":"","enabledPayTypes":[],"notifyUrl":"","returnUrl":""})
        }
        "mail" => {
            json!({"enabled":false,"provider":"smtp","host":"","port":465,"username":"","password":"","fromEmail":"","fromName":"RuoYi-Rust BY","encryption":"ssl","testRecipient":""})
        }
        _ => Value::Null,
    }
}
const SECRETS: [&str; 3] = ["password", "merchantKey", "merchantPrivateKey"];
fn encrypt(key: &[u8; 32], scope: &str, s: &str) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let mut nonce = [0; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let bytes = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: s.as_bytes(),
                aad: scope.as_bytes(),
            },
        )
        .map_err(|_| AppError::bad("加密失败"))?;
    let mut out = nonce.to_vec();
    out.extend(bytes);
    Ok(format!("enc:v1:{}", STANDARD.encode(out)))
}
fn decrypt(key: &[u8; 32], scope: &str, s: &str) -> Result<String> {
    let data = STANDARD
        .decode(
            s.strip_prefix("enc:v1:")
                .ok_or_else(|| AppError::bad("密钥配置格式无效"))?,
        )
        .map_err(|_| AppError::bad("密钥配置格式无效"))?;
    if data.len() < 28 {
        return Err(AppError::bad("密钥配置格式无效"));
    }
    let out = Aes256Gcm::new_from_slice(key)
        .unwrap()
        .decrypt(
            Nonce::from_slice(&data[..12]),
            Payload {
                msg: &data[12..],
                aad: scope.as_bytes(),
            },
        )
        .map_err(|_| AppError::bad("解密失败，请检查 APP_KEY"))?;
    String::from_utf8(out).map_err(|_| AppError::bad("密钥配置无效"))
}
pub async fn settings(app: &App, group: &str, redact: bool) -> Result<Value> {
    let mut out = defaults(group);
    if out.is_null() {
        return Err(AppError::missing());
    }
    let row = db::one(
        &app.pool,
        "SELECT setting_value FROM sys_system_setting WHERE setting_key=?",
        &[json!(format!("by.{group}"))],
    )
    .await?;
    if let Some(s) = row["settingValue"].as_str() {
        let saved: Value =
            serde_json::from_str(s).map_err(|_| AppError::bad("配置数据格式无效"))?;
        for (k, v) in out.as_object_mut().unwrap() {
            if let Some(value) = saved.get(k) {
                *v = value.clone();
            }
        }
    }
    for key in SECRETS {
        if !db::scalar(&out[key]).is_empty() {
            out[key] = json!(if redact {
                "********".into()
            } else {
                decrypt(&app.key, &format!("{group}.{key}"), &db::scalar(&out[key]))?
            });
        }
    }
    if group == "site" {
        out["frontendHeadCode"] = json!("");
    }
    Ok(out)
}
async fn save_settings(app: &App, a: &Actor, i: &Input, group: &str) -> Result<Value> {
    a.require("system:setting:edit")?;
    a.superuser()?;
    let mut cfg = settings(app, group, false).await?;
    let default = defaults(group);
    for (k, t) in default.as_object().unwrap() {
        if let Some(v) = i.body.get(k) {
            if (t.is_boolean() && !v.is_boolean())
                || (t.is_string() && !v.is_string())
                || (t.is_array() && !v.is_array())
                || (t.is_number() && !v.is_number())
            {
                return Err(AppError::bad("配置字段类型错误"));
            }
            if db::scalar(v).len() > 16000 {
                return Err(AppError::bad("配置字段过长"));
            }
            if SECRETS.contains(&k.as_str()) && v == "********" {
                continue;
            }
            cfg[k] = v.clone();
        }
    }
    if group == "site" {
        if db::scalar(&cfg["title"]).is_empty() || db::scalar(&cfg["title"]).chars().count() > 100 {
            return Err(AppError::bad("站点名称需为 1–100 字符"));
        }
        cfg["frontendHeadCode"] = json!("");
        for key in ["logo", "favicon", "siteUrl"] {
            let s = db::scalar(&cfg[key]);
            if !s.is_empty()
                && !s.starts_with("https://")
                && !s.starts_with("http://")
                && !(key != "siteUrl" && s.starts_with("/uploads/"))
            {
                return Err(AppError::bad("站点链接必须为 HTTP(S) URL 或上传图片路径"));
            }
        }
    }
    if (group == "payment" || group == "mail") && cfg["enabled"] == true {
        return Err(AppError::bad(
            "首版支持保存配置；支付网关和 SMTP 尚未实现，暂不能启用",
        ));
    }
    if group == "mail"
        && (!(1..=65535).contains(&db::num(&cfg["port"]))
            || !["ssl", "tls", "none"].contains(&db::scalar(&cfg["encryption"]).as_str()))
    {
        return Err(AppError::bad("邮件端口或加密方式无效"));
    }
    for key in SECRETS {
        let s = db::scalar(&cfg[key]);
        if !s.is_empty() {
            cfg[key] = json!(encrypt(&app.key, &format!("{group}.{key}"), &s)?);
        }
    }
    db::exec(&app.pool,"INSERT INTO sys_system_setting (setting_key,setting_group,setting_value,remark,create_by) VALUES (?,?,?,'',?) ON DUPLICATE KEY UPDATE setting_value=VALUES(setting_value),update_time=NOW(),update_by=VALUES(create_by)",&[json!(format!("by.{group}")),json!(group),json!(cfg.to_string()),a.user["userName"].clone()]).await?;
    Ok(ok())
}

pub async fn upload(
    State(app): State<App>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let a = auth::actor(&app, &headers).await?;
    let field = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad("上传表单错误"))?
        .ok_or_else(|| AppError::bad("请选择图片"))?;
    let avatar = field.name() == Some("avatarfile");
    if !avatar {
        a.superuser()?;
    } // User-avatar uploads are self-service; general uploads are administrative.
    let bytes = field
        .bytes()
        .await
        .map_err(|_| AppError::bad("图片超过 5 MB"))?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::bad("图片超过 5 MB"));
    }
    let output = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| AppError::bad("仅支持 PNG、JPEG、WebP 图片"))?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(80 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .map_err(|_| AppError::bad("图片格式或尺寸无效，最大 4096×4096"))?;
        let mut out = std::io::Cursor::new(Vec::new());
        decoded
            .write_to(&mut out, image::ImageFormat::Png)
            .map_err(|_| AppError::bad("图片处理失败"))?;
        Ok(out.into_inner())
    })
    .await
    .map_err(|_| AppError::bad("图片处理失败"))??;
    let name = format!("{}.png", auth::random_hex(16));
    tokio::fs::write(format!("uploads/{name}"), output)
        .await
        .map_err(|_| AppError::bad("图片保存失败"))?;
    let path = format!("/uploads/{name}");
    if avatar {
        db::exec(
            &app.pool,
            "UPDATE sys_user SET avatar=?,update_time=NOW() WHERE user_id=?",
            &[json!(path), json!(a.id())],
        )
        .await?;
    }
    Ok(Json(
        json!({"code":200,"msg":"上传成功","fileName":format!("{}{path}",app.url),"url":format!("{}{path}",app.url),"imgUrl":path,"newFileName":name,"originalFilename":name}),
    ))
}
async fn profile(app: &App, a: &Actor, i: &Input) -> Result<Value> {
    match i.method.as_str() {
        "GET" => {
            let roles = a
                .roles
                .iter()
                .map(|r| db::scalar(&r["roleName"]))
                .collect::<Vec<_>>()
                .join(",");
            let posts=db::rows(&app.pool,"SELECT p.post_name FROM sys_post p JOIN sys_user_post up ON up.post_id=p.post_id WHERE up.user_id=? AND p.delete_time IS NULL",&[json!(a.id())]).await?;
            Ok(
                json!({"code":200,"data":system::output(app,system::ENTITIES[2],&a.user).await?,"roleGroup":roles,"postGroup":posts.iter().map(|r|db::scalar(&r["postName"])).collect::<Vec<_>>().join(",")}),
            )
        }
        "PUT" => {
            let nick = i.s("nickName");
            if nick.is_empty()
                || nick.chars().count() > 30
                || i.s("email").len() > 50
                || i.s("phonenumber").len() > 11
                || !["0", "1", "2"].contains(&i.s("sex").as_str())
            {
                return Err(AppError::bad("个人资料格式错误"));
            }
            db::exec(&app.pool,"UPDATE sys_user SET nick_name=?,email=?,phonenumber=?,sex=?,update_time=NOW() WHERE user_id=?",&[json!(nick),i.value("email"),i.value("phonenumber"),i.value("sex"),json!(a.id())]).await?;
            Ok(ok())
        }
        _ => Err(AppError::missing()),
    }
}
async fn reset_password(app: &App, a: &Actor, i: &Input, own: bool) -> Result<Value> {
    let id = if own {
        json!(a.id())
    } else {
        i.value("userId")
    };
    if !own {
        a.require("system:user:resetPwd")?;
        if db::num(&id) == 1 {
            return Err(AppError::forbidden());
        }
        system::user_access(app, a, id.clone()).await?;
    }
    if own && !auth::verify(app, i.s("oldPassword"), db::scalar(&a.user["password"])).await? {
        return Err(AppError::bad("旧密码不正确"));
    }
    let hash = auth::hash_password(i.s(if own { "newPassword" } else { "password" })).await?;
    let mut tx = app.pool.begin().await?;
    db::query(
        "UPDATE sys_user SET password=?,update_time=NOW() WHERE user_id=?",
        &[json!(hash), id.clone()],
    )
    .execute(&mut *tx)
    .await?;
    db::query("DELETE FROM sys_access_token WHERE user_id=?", &[id])
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ok())
}
async fn select_tree(app: &App, a: &Actor, menu: bool, role: Option<&str>) -> Result<Value> {
    if menu {
        a.require("system:role:query")?;
    } else if !a.permissions.contains("system:dept:list") {
        a.require("system:user:list")?;
    }
    let (table, pk, label) = if menu {
        ("sys_menu", "menu_id", "menu_name")
    } else {
        ("sys_dept", "dept_id", "dept_name")
    };
    let rows=db::rows(&app.pool,&format!("SELECT {pk} AS id,parent_id,{label} AS label FROM {table} WHERE delete_time IS NULL ORDER BY order_num"),&[]).await?;
    let tree = auth::tree(&rows, "id", 0, 0);
    if let Some(id) = role {
        let rel = if menu {
            "sys_role_menu"
        } else {
            "sys_role_dept"
        };
        let checked = db::rows(
            &app.pool,
            &format!("SELECT {pk} AS id FROM {rel} WHERE role_id=?"),
            &[json!(id)],
        )
        .await?;
        let mut v = json!({"code":200,"checkedKeys":checked.iter().map(|r|r["id"].clone()).collect::<Vec<_>>()});
        v[if menu { "menus" } else { "depts" }] = json!(tree);
        Ok(v)
    } else {
        Ok(data(json!(tree)))
    }
}

pub async fn handle(app: &App, a: &Actor, i: &Input) -> Result<Option<Response>> {
    let p = i.path.as_str();
    let m = i.method.as_str();
    let v = match (m, p) {
        ("GET" | "PUT", "system/user/profile") => profile(app, a, i).await?,
        ("PUT", "system/user/profile/updatePwd") => reset_password(app, a, i, true).await?,
        ("PUT", "system/user/resetPwd") => reset_password(app, a, i, false).await?,
        ("GET", "system/user/deptTree") => select_tree(app, a, false, None).await?,
        ("GET", "system/menu/treeselect") => select_tree(app, a, true, None).await?,
        ("GET", p) if p.starts_with("system/menu/roleMenuTreeselect/") => {
            select_tree(app, a, true, p.rsplit('/').next()).await?
        }
        ("GET", p) if p.starts_with("system/role/deptTree/") => {
            select_tree(app, a, false, p.rsplit('/').next()).await?
        }
        ("GET", p) if p.starts_with("system/dept/list/exclude/") => {
            a.require("system:dept:list")?;
            let id = p.rsplit('/').next().unwrap();
            data(json!(db::rows(&app.pool,"SELECT * FROM sys_dept WHERE delete_time IS NULL AND dept_id<>? AND FIND_IN_SET(?,ancestors)=0",&[json!(id),json!(id)]).await?))
        }
        ("GET", "system/dict/type/optionselect") => data(json!(
            db::rows(
                &app.pool,
                "SELECT * FROM sys_dict_type WHERE status='0' ORDER BY dict_id",
                &[]
            )
            .await?
        )),
        ("GET", p) if p.starts_with("system/dict/data/type/") => data(json!(
            db::rows(
                &app.pool,
                "SELECT * FROM sys_dict_data WHERE dict_type=? AND status='0' ORDER BY dict_sort",
                &[json!(p.strip_prefix("system/dict/data/type/").unwrap())]
            )
            .await?
        )),
        ("GET", p) if p.starts_with("system/config/configKey/") => {
            let key = p.strip_prefix("system/config/configKey/").unwrap();
            if key == "sys.account.registerUser" {
                json!({"code":200,"msg":"false"})
            } else if key == "sys.account.captchaEnabled" {
                json!({"code":200,"msg":app.captcha.to_string()})
            } else {
                let row = db::one(
                    &app.pool,
                    "SELECT config_value FROM sys_config WHERE config_key=?",
                    &[json!(key)],
                )
                .await?;
                json!({"code":200,"msg":db::scalar(&row["configValue"])})
            }
        }
        ("GET", "system/setting") => {
            a.require("system:setting:query")?;
            data(
                json!({"site":settings(app,"site",true).await?,"payment":settings(app,"payment",true).await?,"mail":settings(app,"mail",true).await?,"generated":{"notifyUrl":format!("{}/api/system-config/payment/notify",app.url),"returnUrl":format!("{}/api/system-config/payment/return",app.url)}}),
            )
        }
        ("PUT", p) if p.starts_with("system/setting/") => {
            save_settings(app, a, i, p.strip_prefix("system/setting/").unwrap()).await?
        }
        ("POST", "system/setting/payment/test" | "system/setting/mail/test") => {
            return Err(AppError::bad("此版本暂未实现支付网关和 SMTP 联通测试"));
        }
        ("GET", "monitor/online/list") => {
            a.require("monitor:online:list")?;
            let rows=db::rows(&app.pool,"SELECT t.id AS token_id,u.user_name,d.dept_name,t.ip AS ipaddr,t.browser,t.created_at AS login_time FROM sys_access_token t JOIN sys_user u ON u.user_id=t.user_id LEFT JOIN sys_dept d ON u.dept_id=d.dept_id WHERE t.expires_at>NOW() AND u.status='0' AND u.delete_time IS NULL ORDER BY t.id DESC LIMIT 100",&[]).await?;
            json!({"code":200,"total":rows.len(),"rows":rows})
        }
        ("DELETE", p) if p.starts_with("monitor/online/") => {
            a.require("monitor:online:forceLogout")?;
            a.superuser()?;
            db::exec(
                &app.pool,
                "DELETE FROM sys_access_token WHERE id=?",
                &[json!(p.rsplit('/').next().unwrap())],
            )
            .await?;
            ok()
        }
        ("GET", p) if p.starts_with("monitor/logininfor/unlock/") => {
            a.require("monitor:logininfor:unlock")?;
            a.superuser()?;
            auth::clear_login(app, p.rsplit('/').next().unwrap());
            ok()
        }
        ("GET", "monitor/server") => {
            a.require("monitor:server:list")?;
            data(server(app))
        }
        ("GET", "system/notice/listTop") => {
            let rows=db::rows(&app.pool,"SELECT n.*,IF(r.user_id IS NULL,0,1) AS is_read FROM sys_notice n LEFT JOIN sys_notice_read r ON n.notice_id=r.notice_id AND r.user_id=? WHERE n.status='0' ORDER BY n.notice_id DESC LIMIT 10",&[json!(a.id())]).await?;
            let unread=db::one(&app.pool,"SELECT COUNT(*) AS total FROM sys_notice n WHERE n.status='0' AND NOT EXISTS (SELECT 1 FROM sys_notice_read r WHERE r.notice_id=n.notice_id AND r.user_id=?)",&[json!(a.id())]).await?;
            json!({"code":200,"data":rows,"total":rows.len(),"unreadCount":unread["total"]})
        }
        ("POST", "system/notice/markRead" | "system/notice/markReadAll") => {
            let extra = if p.ends_with("markReadAll") {
                String::new()
            } else {
                " AND notice_id=?".into()
            };
            let mut args = vec![json!(a.id())];
            if !extra.is_empty() {
                args.push(i.value("noticeId"));
            }
            db::exec(&app.pool,&format!("INSERT IGNORE INTO sys_notice_read (notice_id,user_id) SELECT notice_id,? FROM sys_notice WHERE status='0'{extra}"),&args).await?;
            ok()
        }
        ("GET", "system/notice/readUsers/list") => {
            a.require("system:notice:query")?;
            let rows=db::rows(&app.pool,"SELECT u.user_id,u.user_name,u.nick_name,r.read_time FROM sys_notice_read r JOIN sys_user u ON u.user_id=r.user_id WHERE r.notice_id=? LIMIT 100",&[i.value("noticeId")]).await?;
            json!({"code":200,"rows":rows,"total":rows.len()})
        }
        ("PUT", "system/menu/updateSort") => {
            a.require("system:menu:edit")?;
            a.superuser()?;
            let list = i
                .body
                .as_array()
                .or_else(|| i.body.get("menus").and_then(Value::as_array))
                .ok_or_else(|| AppError::bad("请提交菜单排序数组"))?;
            if list.len() > 1000 {
                return Err(AppError::bad("排序项过多"));
            }
            let mut tx = app.pool.begin().await?;
            for row in list {
                db::query(
                    "UPDATE sys_menu SET order_num=?,update_time=NOW() WHERE menu_id=?",
                    &[row["orderNum"].clone(), row["menuId"].clone()],
                )
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            ok()
        }
        ("GET", p) if p.starts_with("system/user/authRole/") => {
            a.require("system:user:query")?;
            let id = json!(p.rsplit('/').next().unwrap());
            let user = system::user_access(app, a, id.clone()).await?;
            let mut roles=db::rows(&app.pool,"SELECT r.*,IF(ur.user_id IS NULL,0,1) AS flag FROM sys_role r LEFT JOIN sys_user_role ur ON ur.role_id=r.role_id AND ur.user_id=? WHERE r.status='0' AND r.delete_time IS NULL",&[id]).await?;
            for r in &mut roles {
                r["flag"] = json!(db::num(&r["flag"]) == 1);
                r["admin"] = json!(db::num(&r["roleId"]) == 1);
            }
            json!({"code":200,"user":db::public(&user),"roles":roles})
        }
        ("PUT", "system/user/authRole") => {
            a.superuser()?;
            let id = i.value("userId");
            system::user_access(app, a, id.clone()).await?;
            if db::num(&id) == 1 {
                return Err(AppError::forbidden());
            }
            let ids = if i.s("roleIds").is_empty() {
                vec![]
            } else {
                db::ids(&i.s("roleIds"))?
            };
            let mut tx = app.pool.begin().await?;
            for rid in &ids {
                if db::query(
                    "SELECT role_id FROM sys_role WHERE role_id=? AND delete_time IS NULL",
                    std::slice::from_ref(rid),
                )
                .fetch_optional(&mut *tx)
                .await?
                .is_none()
                {
                    return Err(AppError::bad("角色不存在"));
                }
            }
            db::query(
                "DELETE FROM sys_user_role WHERE user_id=?",
                std::slice::from_ref(&id),
            )
            .execute(&mut *tx)
            .await?;
            for rid in ids {
                db::query(
                    "INSERT INTO sys_user_role (user_id,role_id) VALUES (?,?)",
                    &[id.clone(), rid],
                )
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            ok()
        }
        ("GET", p) if p.starts_with("system/role/authUser/") => {
            a.require("system:role:list")?;
            a.superuser()?;
            let allocated = p.ends_with("allocatedList");
            let role = i.value("roleId");
            let rows=db::rows(&app.pool,&format!("SELECT u.user_id,u.user_name,u.nick_name,u.email,u.phonenumber,u.status FROM sys_user u WHERE u.delete_time IS NULL AND {}EXISTS (SELECT 1 FROM sys_user_role ur WHERE ur.user_id=u.user_id AND ur.role_id=?) AND u.user_name LIKE ? LIMIT 100",if allocated{""}else{"NOT "}),&[role,json!(format!("%{}%",i.s("userName")))]).await?;
            json!({"code":200,"rows":rows,"total":rows.len()})
        }
        ("PUT", p) if p.starts_with("system/role/authUser/") => {
            a.superuser()?;
            let role = i.value("roleId");
            if db::num(&role) == 1 {
                return Err(AppError::forbidden());
            }
            if db::one(
                &app.pool,
                "SELECT role_id FROM sys_role WHERE role_id=? AND delete_time IS NULL",
                std::slice::from_ref(&role),
            )
            .await?
            .is_null()
            {
                return Err(AppError::missing());
            }
            let users = db::ids(&if i.s("userIds").is_empty() {
                i.s("userId")
            } else {
                i.s("userIds")
            })?;
            let add = p.ends_with("selectAll");
            if !add && !p.ends_with("cancel") && !p.ends_with("cancelAll") {
                return Err(AppError::missing());
            }
            let mut tx = app.pool.begin().await?;
            for id in users {
                if db::num(&id) == 1 {
                    return Err(AppError::forbidden());
                }
                if db::query(
                    "SELECT user_id FROM sys_user WHERE user_id=? AND delete_time IS NULL",
                    std::slice::from_ref(&id),
                )
                .fetch_optional(&mut *tx)
                .await?
                .is_none()
                {
                    return Err(AppError::missing());
                }
                db::query(
                    if add {
                        "INSERT IGNORE INTO sys_user_role (user_id,role_id) VALUES (?,?)"
                    } else {
                        "DELETE FROM sys_user_role WHERE user_id=? AND role_id=?"
                    },
                    &[id, role.clone()],
                )
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            ok()
        }
        ("POST", "system/user/importTemplate" | "system/user/importData") => {
            return Err(AppError::bad(
                "首版暂不支持 Excel 用户导入，请使用新增用户或 CSV 导出",
            ));
        }
        _ => return Ok(None),
    };
    Ok(Some(Json(v).into_response()))
}
fn server(app: &App) -> Value {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory() as f64;
    let used = sys.used_memory() as f64;
    let gb = 1024f64.powi(3);
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(1);
    json!({"cpu":{"cpuNum":cpus,"used":null,"sys":null,"free":null},"mem":{"total":(total/gb*100.).round()/100.,"used":(used/gb*100.).round()/100.,"free":((total-used)/gb*100.).round()/100.,"usage":(used/total*10000.).round()/100.},"sys":{"computerName":sysinfo::System::host_name().unwrap_or_default(),"osName":sysinfo::System::name().unwrap_or_default(),"osArch":std::env::consts::ARCH,"computerIp":"","userDir":""},"jvm":{"name":"Rust / Axum","version":env!("CARGO_PKG_VERSION"),"total":null,"used":null,"free":null,"usage":null,"runTime":format!("{} 秒",app.started.elapsed().as_secs()),"startTime":null,"home":"","inputArgs":""},"sysFiles":[]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_authentication() {
        let key = [7u8; 32];
        let s = encrypt(&key, "mail.password", "secret").unwrap();
        assert_eq!(decrypt(&key, "mail.password", &s).unwrap(), "secret");
        assert!(decrypt(&key, "payment.password", &s).is_err());
        assert!(decrypt(&[8; 32], "mail.password", &s).is_err());
    }
}
