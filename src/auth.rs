use crate::{App, AppError, Input, Result, data, db, ok};
use axum::http::{HeaderMap, StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use font8x8::UnicodeFonts;
use rand::{Rng, RngCore};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Gates {
    rates: HashMap<String, (Instant, u32)>,
    captchas: HashMap<String, (Instant, String)>,
}
impl Gates {
    fn rate(&mut self, key: String, limit: u32) -> Result<()> {
        let now = Instant::now();
        self.rates
            .retain(|_, (t, _)| now.duration_since(*t) < Duration::from_secs(600));
        if self.rates.len() >= 10000 && !self.rates.contains_key(&key) {
            return Err(AppError(StatusCode::TOO_MANY_REQUESTS, "请求过多".into()));
        }
        let v = self.rates.entry(key).or_insert((now, 0));
        v.1 += 1;
        if v.1 > limit {
            return Err(AppError(
                StatusCode::TOO_MANY_REQUESTS,
                "尝试过多，请 10 分钟后重试".into(),
            ));
        }
        Ok(())
    }
    fn consume(&mut self, id: &str, answer: &str) -> Result<()> {
        let item = self.captchas.remove(id);
        if let Some((time, value)) = item
            && time.elapsed() < Duration::from_secs(120)
            && value.eq_ignore_ascii_case(answer)
        {
            return Ok(());
        }
        Err(AppError::bad("验证码错误或已过期"))
    }
}
pub fn random_hex(n: usize) -> String {
    let mut b = vec![0; n];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}
pub fn token_hash(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}
pub fn bearer(h: &HeaderMap) -> String {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap_or("")
        .into()
}
pub fn validate_password(p: &str) -> Result<()> {
    if p.chars().count() < 12 || p.len() > 72 {
        return Err(AppError::bad("密码至少 12 个字符，最多 72 字节"));
    }
    Ok(())
}
pub async fn hash_password(p: String) -> Result<String> {
    validate_password(&p)?;
    tokio::task::spawn_blocking(move || bcrypt::hash(p, 12))
        .await
        .map_err(|_| AppError::bad("密码处理失败"))?
        .map_err(|_| AppError::bad("密码处理失败"))
}
pub async fn verify(app: &App, p: String, hash: String) -> Result<bool> {
    if p.len() > 72 {
        return Ok(false);
    }
    let permit = app
        .passwords
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::bad("服务关闭中"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        bcrypt::verify(p, &hash).unwrap_or(false)
    })
    .await
    .map_err(|_| AppError::bad("密码处理失败"))
}

pub struct Actor {
    pub user: Value,
    pub roles: Vec<Value>,
    pub role_keys: Vec<String>,
    pub menus: Vec<Value>,
    pub permissions: HashSet<String>,
}
impl Actor {
    pub fn id(&self) -> i64 {
        db::num(&self.user["userId"])
    }
    pub fn admin(&self) -> bool {
        self.id() == 1
    }
    pub fn require(&self, p: &str) -> Result<()> {
        if self.admin() || self.permissions.contains(p) {
            Ok(())
        } else {
            Err(AppError::forbidden())
        }
    }
    pub fn superuser(&self) -> Result<()> {
        if self.admin() {
            Ok(())
        } else {
            Err(AppError::forbidden())
        }
    }
}
pub async fn actor(app: &App, h: &HeaderMap) -> Result<Actor> {
    let token = bearer(h);
    if token.is_empty() {
        return Err(AppError(StatusCode::UNAUTHORIZED, "请先登录".into()));
    }
    let user=db::one(&app.pool,"SELECT u.* FROM sys_user u JOIN sys_access_token t ON t.user_id=u.user_id WHERE t.token_hash=? AND t.expires_at>NOW() AND u.status='0' AND u.delete_time IS NULL",&[json!(token_hash(&token))]).await?;
    if user.is_null() {
        return Err(AppError(
            StatusCode::UNAUTHORIZED,
            "登录已过期，请重新登录".into(),
        ));
    }
    let roles=db::rows(&app.pool,"SELECT r.* FROM sys_role r JOIN sys_user_role ur ON r.role_id=ur.role_id WHERE ur.user_id=? AND r.status='0' AND r.delete_time IS NULL",&[user["userId"].clone()]).await?;
    // Inherited roles affect functions, not explicit assignments or data scope.
    let root = db::num(&user["userId"]) == 1;
    let mut effective = roles.clone();
    if root || roles.iter().any(|r| r["roleKey"] == "admin") {
        let common = db::rows(
            &app.pool,
            "SELECT * FROM sys_role WHERE role_key='common' AND status='0' AND delete_time IS NULL",
            &[],
        )
        .await?;
        for role in common {
            if !effective.iter().any(|r| r["roleId"] == role["roleId"]) {
                effective.push(role);
            }
        }
    }
    let mut role_keys: Vec<String> = effective
        .iter()
        .map(|r| db::scalar(&r["roleKey"]))
        .collect();
    if root && !role_keys.iter().any(|key| key == "admin") {
        role_keys.push("admin".into());
    }
    let role_ids: Vec<Value> = effective.iter().map(|r| r["roleId"].clone()).collect();
    let menus = if root {
        db::rows(&app.pool,"SELECT * FROM sys_menu WHERE status='0' AND delete_time IS NULL ORDER BY order_num,menu_id",&[]).await?
    } else if role_ids.is_empty() {
        Vec::new()
    } else {
        db::rows(&app.pool, &format!("SELECT DISTINCT m.* FROM sys_menu m JOIN sys_role_menu rm ON rm.menu_id=m.menu_id WHERE rm.role_id IN ({}) AND m.status='0' AND m.delete_time IS NULL ORDER BY m.order_num,m.menu_id", db::placeholders(role_ids.len())), &role_ids).await?
    };
    let permissions = menus
        .iter()
        .flat_map(|m| {
            db::scalar(&m["perms"])
                .split(',')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|p| !p.is_empty())
        .collect();
    Ok(Actor {
        user,
        roles,
        role_keys,
        menus,
        permissions,
    })
}
pub async fn public_api(app: &App, i: &Input) -> Result<Option<Value>> {
    match (i.method.as_str(), i.path.as_str()) {
        ("GET", "health") => {
            db::one(&app.pool, "SELECT 1 AS alive", &[]).await?;
            Ok(Some(data(
                json!({"status":"ok","version":env!("CARGO_PKG_VERSION"),"testMode":crate::env("RUOYI_TEST_MODE","false")=="true"}),
            )))
        }
        ("GET", "captchaImage") => {
            if !app.captcha {
                return Ok(Some(json!({"code":200,"captchaEnabled":false})));
            }
            let mut gates = app.gates.lock().unwrap();
            gates.rate(format!("captcha:{}", i.ip), 120)?;
            gates
                .captchas
                .retain(|_, (t, _)| t.elapsed() < Duration::from_secs(120));
            if gates.captchas.len() >= 10000 {
                return Err(AppError(
                    StatusCode::TOO_MANY_REQUESTS,
                    "验证码请求过多".into(),
                ));
            }
            let uuid = random_hex(16);
            let chars = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
            let mut rng = rand::thread_rng();
            let answer: String = (0..5)
                .map(|_| chars[rng.gen_range(0..chars.len())] as char)
                .collect();
            let mut im = image::RgbImage::from_pixel(160, 48, image::Rgb([243, 247, 253]));
            for _ in 0..250 {
                im.put_pixel(
                    rng.gen_range(0..160),
                    rng.gen_range(0..48),
                    image::Rgb([150, 177, 203]),
                );
            }
            for (idx, ch) in answer.chars().enumerate() {
                if let Some(glyph) = font8x8::BASIC_FONTS.get(ch) {
                    let top = rng.gen_range(8..17);
                    for (y, bits) in glyph.iter().enumerate() {
                        for x in 0..8 {
                            if bits & (1 << x) != 0 {
                                for dy in 0..3 {
                                    for dx in 0..3 {
                                        im.put_pixel(
                                            7 + idx as u32 * 30 + x * 3 + dx,
                                            top + y as u32 * 3 + dy,
                                            image::Rgb([30, 66, 103]),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let mut bytes = Cursor::new(Vec::new());
            im.write_to(&mut bytes, image::ImageFormat::Png)
                .map_err(|_| AppError::bad("图片生成失败"))?;
            gates
                .captchas
                .insert(uuid.clone(), (Instant::now(), answer));
            Ok(Some(
                json!({"code":200,"captchaEnabled":true,"uuid":uuid,"img":STANDARD.encode(bytes.into_inner())}),
            ))
        }
        ("POST", "login") => {
            let name = i.s("username");
            if name.is_empty() || name.len() > 30 || i.s("password").len() > 72 {
                return Err(AppError::bad("用户名或密码格式错误"));
            }
            {
                let mut gates = app.gates.lock().unwrap();
                gates.rate(format!("login-ip:{}", i.ip), 100)?;
                gates.rate(format!("login-user:{}", name.to_lowercase()), 10)?;
                if app.captcha {
                    gates.consume(&i.s("uuid"), &i.s("code"))?;
                }
            }
            let user = db::one(
                &app.pool,
                "SELECT * FROM sys_user WHERE user_name=? AND status='0' AND delete_time IS NULL",
                &[json!(name)],
            )
            .await?;
            // A fixed valid bcrypt hash gives unknown users the same expensive verification path.
            let hash = if user.is_null() {
                "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxmGsPPoYuFKRH9yJb3OWcxab2u".into()
            } else {
                db::scalar(&user["password"])
            };
            let valid = verify(app, i.s("password"), hash).await? && !user.is_null();
            db::exec(&app.pool,"INSERT INTO sys_logininfor (user_name,ipaddr,status,msg,browser) VALUES (?,?,?,?,?)",&[json!(name),json!(i.ip),json!(if valid{"0"}else{"1"}),json!(if valid{"登录成功"}else{"用户名或密码错误"}),json!(i.headers.get("user-agent").and_then(|v|v.to_str().ok()).unwrap_or("").chars().take(50).collect::<String>())]).await?;
            if !valid {
                return Err(AppError::bad("用户名或密码错误"));
            }
            app.gates
                .lock()
                .unwrap()
                .rates
                .remove(&format!("login-user:{}", name.to_lowercase()));
            let token = random_hex(32);
            let mut tx = app.pool.begin().await?;
            db::query("DELETE FROM sys_access_token WHERE expires_at<=NOW()", &[])
                .execute(&mut *tx)
                .await?;
            db::query("INSERT INTO sys_access_token (user_id,token_hash,ip,browser,expires_at) VALUES (?,?,?,?,DATE_ADD(NOW(),INTERVAL ? MINUTE))",&[user["userId"].clone(),json!(token_hash(&token)),json!(i.ip),json!(i.headers.get("user-agent").and_then(|v|v.to_str().ok()).unwrap_or("").chars().take(255).collect::<String>()),json!(app.token_minutes)]).execute(&mut *tx).await?;
            db::query(
                "UPDATE sys_user SET login_ip=?,login_date=NOW() WHERE user_id=?",
                &[json!(i.ip), user["userId"].clone()],
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(Some(json!({"code":200,"msg":"登录成功","token":token})))
        }
        ("POST", "logout") => {
            db::exec(
                &app.pool,
                "DELETE FROM sys_access_token WHERE token_hash=?",
                &[json!(token_hash(&bearer(&i.headers)))],
            )
            .await?;
            Ok(Some(ok()))
        }
        ("POST", "register") => Err(AppError(
            StatusCode::FORBIDDEN,
            "首版由管理员创建用户，未开放公开注册".into(),
        )),
        _ => Ok(None),
    }
}
pub fn tree(rows: &[Value], id: &str, parent: i64, depth: usize) -> Vec<Value> {
    if depth > 32 {
        return Vec::new();
    }
    rows.iter()
        .filter(|r| db::num(&r["parentId"]) == parent)
        .map(|r| {
            let mut r = r.clone();
            let children = tree(rows, id, db::num(&r[id]), depth + 1);
            if !children.is_empty() {
                r["children"] = json!(children);
            }
            r
        })
        .collect()
}
pub fn routers(rows: &[Value], parent: i64, depth: usize) -> Vec<Value> {
    if depth > 32 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for m in rows {
        if db::num(&m["parentId"]) != parent || m["menuType"] == "F" {
            continue;
        }
        let path = db::scalar(&m["path"]);
        let name = db::scalar(&m["routeName"]);
        let component = db::scalar(&m["component"]);
        let mut r = json!({"name":if name.is_empty(){db::camel(&format!("_{}",path))}else{name},"path":if parent==0{format!("/{}",path.trim_start_matches('/'))}else{path.clone()},"component":if component.is_empty(){if parent==0{"Layout"}else{"ParentView"}.into()}else{component},"hidden":m["visible"]=="1","meta":{"title":m["menuName"],"icon":m["icon"],"noCache":db::num(&m["isCache"])==1}});
        let children = routers(rows, db::num(&m["menuId"]), depth + 1);
        if !children.is_empty() {
            r["children"] = json!(children);
            r["alwaysShow"] = json!(true);
            r["redirect"] = json!("noRedirect");
        } else if parent == 0 && m["menuType"] == "C" {
            let mut child = r.clone();
            child["path"] = json!(path);
            r["path"] = json!("/");
            r["component"] = json!("Layout");
            r["name"] = json!("");
            r["children"] = json!([child]);
        }
        out.push(r);
    }
    out
}
pub async fn private_api(app: &App, a: &Actor, i: &Input) -> Result<Option<Value>> {
    match (i.method.as_str(), i.path.as_str()) {
        ("GET", "getInfo") => {
            let mut user = db::public(&a.user);
            user["admin"] = json!(a.admin());
            user["roles"] = json!(a.roles);
            user["dept"] = db::one(
                &app.pool,
                "SELECT * FROM sys_dept WHERE dept_id=?",
                &[user["deptId"].clone()],
            )
            .await?;
            Ok(Some(
                json!({"code":200,"user":user,"roles":a.role_keys,"permissions":if a.admin(){vec!["*:*:*".to_string()]}else{a.permissions.iter().cloned().collect()}}),
            ))
        }
        ("GET", "getRouters") => Ok(Some(data(json!(routers(&a.menus, 0, 0))))),
        ("POST", "unlockscreen") => {
            app.gates
                .lock()
                .unwrap()
                .rate(format!("unlock:{}", a.id()), 10)?;
            if !verify(app, i.s("password"), db::scalar(&a.user["password"])).await? {
                return Err(AppError::bad("密码错误"));
            }
            Ok(Some(ok()))
        }
        _ => Ok(None),
    }
}
pub fn clear_login(app: &App, name: &str) {
    app.gates
        .lock()
        .unwrap()
        .rates
        .remove(&format!("login-user:{}", name.to_lowercase()));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_time_captcha() {
        let mut g = Gates::default();
        g.captchas
            .insert("id".into(), (Instant::now(), "AB234".into()));
        assert!(g.consume("id", "wrong").is_err());
        assert!(g.consume("id", "AB234").is_err());
        g.captchas.insert(
            "id".into(),
            (Instant::now() - Duration::from_secs(121), "AB234".into()),
        );
        assert!(g.consume("id", "AB234").is_err());
    }
    #[test]
    fn password_byte_limit() {
        assert!(validate_password("123456789012").is_ok());
        assert!(validate_password(&"中".repeat(25)).is_err());
    }
    #[test]
    fn rate_limit() {
        let mut g = Gates::default();
        for _ in 0..10 {
            g.rate("key".into(), 10).unwrap();
        }
        assert!(g.rate("key".into(), 10).is_err());
    }
    #[test]
    fn token_is_hashed() {
        let t = random_hex(32);
        assert_eq!(t.len(), 64);
        assert_ne!(token_hash(&t), t);
    }
}
