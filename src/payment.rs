//! Epay protocol adapter. The order ledger is authoritative; browser returns never settle orders.
use crate::{App, AppError, Input, Result, auth::Actor, data, db, extra};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use md5::{Digest, Md5};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    pkcs1v15::{Signature, SigningKey, VerifyingKey},
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    signature::{RandomizedSigner, SignatureEncoding, Verifier},
    traits::PublicKeyParts,
};
use serde_json::{Value, json};
use sha2::Sha256;
use std::{collections::BTreeMap, time::Duration};
use subtle::ConstantTimeEq;
type Params = BTreeMap<String, String>;

fn canonical(p: &Params) -> String {
    p.iter()
        .filter(|(k, v)| !["sign", "sign_type"].contains(&k.as_str()) && !v.trim().is_empty())
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}
fn private_key(s: &str) -> Result<RsaPrivateKey> {
    let key = RsaPrivateKey::from_pkcs8_pem(s)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(s))
        .map_err(|_| AppError::bad("商户 RSA 私钥格式无效，请填写 PEM"))?;
    if key.n().bits() < 2048 {
        return Err(AppError::bad("RSA 密钥至少 2048 位"));
    }
    Ok(key)
}
fn public_key(s: &str) -> Result<RsaPublicKey> {
    let key = RsaPublicKey::from_public_key_pem(s)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(s))
        .map_err(|_| AppError::bad("平台 RSA 公钥格式无效，请填写 PEM"))?;
    if key.n().bits() < 2048 {
        return Err(AppError::bad("RSA 密钥至少 2048 位"));
    }
    Ok(key)
}
fn sign(p: &mut Params, cfg: &Value) -> Result<()> {
    let base = canonical(p);
    let (signature, kind) = if cfg["epayVersion"] == "v2" {
        let key = SigningKey::<Sha256>::new(private_key(&db::scalar(&cfg["merchantPrivateKey"]))?);
        (
            STANDARD.encode(
                key.sign_with_rng(&mut rand::rngs::OsRng, base.as_bytes())
                    .to_vec(),
            ),
            "RSA",
        )
    } else {
        (
            hex::encode(Md5::digest(format!(
                "{base}{}",
                db::scalar(&cfg["merchantKey"])
            ))),
            "MD5",
        )
    };
    p.insert("sign".into(), signature);
    p.insert("sign_type".into(), kind.into());
    Ok(())
}
fn verify(p: &Params, cfg: &Value) -> Result<()> {
    let sig = p.get("sign").ok_or_else(|| AppError::bad("缺少支付签名"))?;
    if cfg["epayVersion"] == "v2" {
        if p.get("sign_type").map(String::as_str) != Some("RSA") {
            return Err(AppError::bad("支付签名类型无效"));
        }
        let timestamp = p
            .get("timestamp")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| AppError::bad("缺少支付时间戳"))?;
        if Utc::now().timestamp().abs_diff(timestamp) > 300 {
            return Err(AppError::bad("支付时间戳过期"));
        }
        let bytes = STANDARD
            .decode(sig)
            .map_err(|_| AppError::bad("支付签名无效"))?;
        let signature =
            Signature::try_from(bytes.as_slice()).map_err(|_| AppError::bad("支付签名无效"))?;
        VerifyingKey::<Sha256>::new(public_key(&db::scalar(&cfg["platformPublicKey"]))?)
            .verify(canonical(p).as_bytes(), &signature)
            .map_err(|_| AppError::bad("支付签名校验失败"))?;
    } else {
        if p.get("sign_type").map(String::as_str) != Some("MD5") {
            return Err(AppError::bad("支付签名类型无效"));
        }
        let expected = hex::encode(Md5::digest(format!(
            "{}{}",
            canonical(p),
            db::scalar(&cfg["merchantKey"])
        )));
        if !bool::from(expected.as_bytes().ct_eq(sig.as_bytes())) {
            return Err(AppError::bad("支付签名校验失败"));
        }
    }
    Ok(())
}
fn url(s: &str) -> Result<reqwest::Url> {
    let u = reqwest::Url::parse(s).map_err(|_| AppError::bad("支付 URL 格式无效"))?;
    let test_local = crate::env("RUOYI_TEST_MODE", "false") == "true"
        && [Some("127.0.0.1"), Some("localhost"), Some("[::1]")].contains(&u.host_str());
    if (u.scheme() != "https" && !(test_local && u.scheme() == "http"))
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.fragment().is_some()
    {
        return Err(AppError::bad(
            "支付 URL 必须使用 HTTPS 且不能包含凭据或片段",
        ));
    }
    Ok(u)
}
pub fn validate(cfg: &Value) -> Result<()> {
    if cfg["provider"] != "epay"
        || !["v1", "v2"].contains(&db::scalar(&cfg["epayVersion"]).as_str())
        || db::scalar(&cfg["merchantId"]).is_empty()
    {
        return Err(AppError::bad("易支付商户或版本配置无效"));
    }
    let gateway = url(&db::scalar(&cfg["gatewayUrl"]))?;
    if gateway.query().is_some() {
        return Err(AppError::bad("支付网关地址不能含查询参数"));
    }
    for field in ["notifyUrl", "returnUrl"] {
        if !db::scalar(&cfg[field]).is_empty() {
            url(&db::scalar(&cfg[field]))?;
        }
    }
    if cfg["epayVersion"] == "v2" {
        private_key(&db::scalar(&cfg["merchantPrivateKey"]))?;
        public_key(&db::scalar(&cfg["platformPublicKey"]))?;
    } else if db::scalar(&cfg["merchantKey"]).is_empty() {
        return Err(AppError::bad("缺少商户密钥"));
    }
    let types = cfg["enabledPayTypes"]
        .as_array()
        .ok_or_else(|| AppError::bad("请选择支付方式"))?;
    if types.is_empty()
        || types
            .iter()
            .any(|v| !["alipay", "wxpay", "qqpay"].contains(&db::scalar(v).as_str()))
    {
        return Err(AppError::bad("支付方式无效"));
    }
    Ok(())
}
/// Parse decimal money exactly; no floating point settlement.
fn cents(s: &str) -> Result<i64> {
    let parts = s.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts[0].is_empty()
        || !parts[0].bytes().all(|b| b.is_ascii_digit())
        || (parts.len() == 2
            && (parts[1].is_empty()
                || parts[1].len() > 2
                || !parts[1].bytes().all(|b| b.is_ascii_digit())))
    {
        return Err(AppError::bad("金额需为最多两位小数的正数"));
    }
    let whole = parts[0]
        .parse::<i64>()
        .map_err(|_| AppError::bad("金额超限"))?;
    let frac = if parts.len() == 2 {
        format!("{:0<2}", parts[1]).parse::<i64>().unwrap()
    } else {
        0
    };
    let n = whole
        .checked_mul(100)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(|| AppError::bad("金额超限"))?;
    if !(1..=100_000_000).contains(&n) {
        return Err(AppError::bad("金额范围 0.01–1000000.00"));
    }
    Ok(n)
}
async fn gateway(cfg: &Value, endpoint: &str, mut params: Params, signed: bool) -> Result<Value> {
    if signed {
        sign(&mut params, cfg)?;
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AppError::bad("支付客户端初始化失败"))?;
    let endpoint = format!(
        "{}{endpoint}",
        db::scalar(&cfg["gatewayUrl"]).trim_end_matches('/')
    );
    let mut response = client
        .post(url(&endpoint)?)
        .form(&params)
        .send()
        .await
        .map_err(|_| AppError::bad("支付网关连接失败或超时，请通过原订单号核对状态"))?;
    if !response.status().is_success() {
        return Err(AppError::bad("支付网关返回 HTTP 错误"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AppError::bad("读取支付响应失败"))?
    {
        if bytes.len() + chunk.len() > 128 * 1024 {
            return Err(AppError::bad("支付网关响应过大"));
        }
        bytes.extend(chunk);
    }
    let response: Value =
        serde_json::from_slice(&bytes).map_err(|_| AppError::bad("支付网关响应格式无效"))?;
    if db::scalar(&response["code"]) != if cfg["epayVersion"] == "v2" { "0" } else { "1" } {
        return Err(AppError::bad(
            "支付网关拒绝请求，请在商户后台核对配置及订单",
        ));
    }
    if cfg["epayVersion"] == "v2" {
        let params = response
            .as_object()
            .ok_or_else(|| AppError::bad("支付响应无效"))?
            .iter()
            .map(|(k, v)| (k.clone(), db::scalar(v)))
            .collect();
        verify(&params, cfg)?;
    }
    Ok(response)
}
async fn config(app: &App) -> Result<Value> {
    let cfg = extra::settings(app, "payment", false).await?;
    if cfg["enabled"] != true {
        return Err(AppError::bad("请先保存并启用支付配置"));
    }
    validate(&cfg)?;
    Ok(cfg)
}
async fn create(app: &App, a: &Actor, i: &Input, test: bool) -> Result<Value> {
    let cfg = config(app).await?;
    let number = if test {
        format!("T{}", crate::auth::random_hex(16))
    } else {
        i.s("outTradeNo")
    };
    if number.is_empty()
        || number.len() > 64
        || !number
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(AppError::bad(
            "请提供唯一的商户订单号（字母、数字、短横线或下划线）",
        ));
    }
    let amount = if test { 1 } else { cents(&i.s("money"))? };
    let kind = if test {
        db::scalar(&cfg["enabledPayTypes"][0])
    } else {
        i.s("payType")
    };
    let subject = if test {
        "支付功能测试".into()
    } else {
        i.s("subject")
    };
    if subject.is_empty()
        || subject.chars().count() > 120
        || !cfg["enabledPayTypes"]
            .as_array()
            .unwrap()
            .contains(&json!(kind))
    {
        return Err(AppError::bad("订单标题或支付方式无效"));
    }
    let existing = db::one(
        &app.pool,
        "SELECT * FROM sys_payment_order WHERE out_trade_no=?",
        &[json!(number)],
    )
    .await?;
    if !existing.is_null() {
        if db::num(&existing["amountCents"]) != amount
            || existing["payType"] != kind
            || existing["subject"] != subject
            || db::num(&existing["createBy"]) != a.id()
        {
            return Err(AppError::bad("订单号已使用且参数不一致"));
        }
        return Ok(existing);
    }
    let notify = if db::scalar(&cfg["notifyUrl"]).is_empty() {
        format!("{}/api/system-config/payment/notify", app.url)
    } else {
        db::scalar(&cfg["notifyUrl"])
    };
    let ret = if db::scalar(&cfg["returnUrl"]).is_empty() {
        format!("{}/api/system-config/payment/return", app.url)
    } else {
        db::scalar(&cfg["returnUrl"])
    };
    url(&notify)?;
    url(&ret)?;
    db::exec(&app.pool,"INSERT INTO sys_payment_order (out_trade_no,merchant_id,version,pay_type,amount_cents,subject,create_by) VALUES (?,?,?,?,?,?,?)",&[json!(number),cfg["merchantId"].clone(),cfg["epayVersion"].clone(),json!(kind),json!(amount),json!(subject),json!(a.id())]).await?;
    let mut params = Params::from([
        ("pid".into(), db::scalar(&cfg["merchantId"])),
        ("type".into(), kind),
        ("out_trade_no".into(), number.clone()),
        ("name".into(), subject),
        (
            "money".into(),
            format!("{}.{:02}", amount / 100, amount % 100),
        ),
        ("notify_url".into(), notify),
        ("return_url".into(), ret),
        ("clientip".into(), i.ip.clone()),
        ("device".into(), "pc".into()),
    ]);
    let v2 = cfg["epayVersion"] == "v2";
    if v2 {
        params.insert("timestamp".into(), Utc::now().timestamp().to_string());
        params.insert("method".into(), "web".into());
    }
    // Keep pending on transport failure: gateway may have accepted it. Never blindly resubmit.
    let response = gateway(
        &cfg,
        if v2 { "/api/pay/create" } else { "/mapi.php" },
        params,
        true,
    )
    .await?;
    let trade = db::scalar(&response["trade_no"]);
    if trade.is_empty() || trade.len() > 100 {
        return Err(AppError::bad("支付平台订单号无效"));
    }
    let pay = ["pay_info", "payurl", "qrcode", "urlscheme"]
        .iter()
        .map(|k| db::scalar(&response[k]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let changed=db::query("UPDATE sys_payment_order SET trade_no=?,pay_info=? WHERE out_trade_no=? AND (trade_no IS NULL OR trade_no=?)",&[json!(trade),json!(pay),json!(number),json!(trade)]).execute(&app.pool).await?.rows_affected();
    if changed == 0 {
        return Err(AppError::bad("平台订单号与回调不一致"));
    }
    db::one(
        &app.pool,
        "SELECT * FROM sys_payment_order WHERE out_trade_no=?",
        &[json!(number)],
    )
    .await
}
async fn settle(app: &App, cfg: &Value, p: &Params) -> Result<()> {
    let get = |k: &str| p.get(k).cloned().unwrap_or_default();
    let number = get("out_trade_no");
    let trade = get("trade_no");
    if trade.is_empty()
        || trade.len() > 100
        || get("trade_status") != "TRADE_SUCCESS"
        || get("pid") != db::scalar(&cfg["merchantId"])
    {
        return Err(AppError::bad("支付通知商户或状态无效"));
    }
    let amount = cents(&get("money"))?;
    let mut tx = app.pool.begin().await?;
    let row = db::query(
        "SELECT * FROM sys_payment_order WHERE out_trade_no=? FOR UPDATE",
        &[json!(number)],
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::missing)?;
    let order = db::row(row)?;
    if order["merchantId"] != cfg["merchantId"]
        || order["version"] != cfg["epayVersion"]
        || order["payType"] != get("type")
        || db::num(&order["amountCents"]) != amount
        || (!order["tradeNo"].is_null() && order["tradeNo"] != trade)
    {
        return Err(AppError::bad("支付通知与订单不一致"));
    }
    db::query("UPDATE sys_payment_order SET status='paid',trade_no=?,paid_time=COALESCE(paid_time,NOW()) WHERE out_trade_no=? AND status='pending'",&[json!(trade),json!(number)]).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn public(app: &App, i: &Input) -> Result<Option<Response>> {
    if i.path == "system-config/payment/return" && i.method == axum::http::Method::GET {
        return Ok(Some(
            "支付结果请以订单状态为准；此页面不会变更订单。".into_response(),
        ));
    }
    if i.path != "system-config/payment/notify" || !["GET", "POST"].contains(&i.method.as_str()) {
        return Ok(None);
    }
    let cfg = extra::settings(app, "payment", false).await?;
    let mut p: Params = i
        .params
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(body) = i.body.as_object() {
        for (k, v) in body {
            if p.contains_key(k) {
                return Err(AppError::bad("重复的支付通知参数"));
            }
            p.insert(k.clone(), db::scalar(v));
        }
    }
    verify(&p, &cfg)?;
    settle(app, &cfg, &p).await?;
    Ok(Some("success".into_response()))
}
pub async fn test(app: &App, a: &Actor, i: &Input) -> Result<Value> {
    a.require("system:setting:pay:test")?;
    a.superuser()?;
    let order = create(app, a, i, true).await?;
    Ok(data(
        json!({"success":true,"action":"create","version":order["version"],"outTradeNo":order["outTradeNo"],"tradeNo":order["tradeNo"],"payType":order["payType"],"payInfo":order["payInfo"],"steps":[{"name":"创建 0.01 元测试订单","status":"success","message":"订单已创建，尚未付款；付款结果以已验签通知为准","elapsedMs":0}]}),
    ))
}
async fn reconcile(app: &App, number: &str) -> Result<Value> {
    let cfg = config(app).await?;
    let order = db::one(
        &app.pool,
        "SELECT * FROM sys_payment_order WHERE out_trade_no=?",
        &[json!(number)],
    )
    .await?;
    if order.is_null() {
        return Err(AppError::missing());
    }
    if order["merchantId"] != cfg["merchantId"] || order["version"] != cfg["epayVersion"] {
        return Err(AppError::bad("订单商户与当前配置不同，需恢复对应配置查询"));
    }
    let v2 = cfg["epayVersion"] == "v2";
    let mut params = Params::from([
        ("pid".into(), db::scalar(&cfg["merchantId"])),
        ("out_trade_no".into(), number.to_owned()),
    ]);
    if v2 {
        params.insert("timestamp".into(), Utc::now().timestamp().to_string());
    } else {
        params.insert("act".into(), "order".into());
        params.insert("key".into(), db::scalar(&cfg["merchantKey"]));
    }
    let response = gateway(
        &cfg,
        if v2 { "/api/pay/query" } else { "/api.php" },
        params,
        v2,
    )
    .await?;
    if response["status"] == 1
        || response["status"] == "1"
        || response["trade_status"] == "TRADE_SUCCESS"
    {
        let mut p: Params = response
            .as_object()
            .ok_or_else(|| AppError::bad("支付查询响应无效"))?
            .iter()
            .map(|(k, v)| (k.clone(), db::scalar(v)))
            .collect();
        p.insert("trade_status".into(), "TRADE_SUCCESS".into());
        // V1 query responses omit pid; the authenticated request fixes the merchant.
        p.entry("pid".into())
            .or_insert_with(|| db::scalar(&cfg["merchantId"]));
        if p.get("out_trade_no").map(String::as_str) != Some(number) {
            return Err(AppError::bad("支付查询返回了不同订单"));
        }
        settle(app, &cfg, &p).await?;
    }
    db::one(
        &app.pool,
        "SELECT * FROM sys_payment_order WHERE out_trade_no=?",
        &[json!(number)],
    )
    .await
}

pub async fn handle(app: &App, a: &Actor, i: &Input) -> Result<Response> {
    a.superuser()?;
    let v = match (i.method.as_str(), i.path.as_str()) {
        ("POST", p) if p.starts_with("payment/orders/") && p.ends_with("/query") => {
            a.require("system:setting:pay:test")?;
            data(
                reconcile(
                    app,
                    p.trim_start_matches("payment/orders/")
                        .trim_end_matches("/query"),
                )
                .await?,
            )
        }
        ("POST", "payment/orders") => {
            a.require("system:setting:pay:test")?;
            data(create(app, a, i, false).await?)
        }
        ("GET", "payment/orders") => {
            a.require("system:setting:query")?;
            let rows = db::rows(
                &app.pool,
                "SELECT * FROM sys_payment_order ORDER BY id DESC LIMIT 100",
                &[],
            )
            .await?;
            json!({"code":200,"rows":rows,"total":rows.len()})
        }
        ("GET", p) if p.starts_with("payment/orders/") => {
            a.require("system:setting:query")?;
            let row = db::one(
                &app.pool,
                "SELECT * FROM sys_payment_order WHERE out_trade_no=?",
                &[json!(p.trim_start_matches("payment/orders/"))],
            )
            .await?;
            if row.is_null() {
                return Err(AppError::missing());
            }
            data(row)
        }
        _ => return Err(AppError::missing()),
    };
    Ok(Json(v).into_response())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_exact() {
        assert_eq!(cents("12.30").unwrap(), 1230);
        assert_eq!(cents("0.01").unwrap(), 1);
        for s in [
            "NaN",
            "1.001",
            "-1",
            "0",
            "1e2",
            "99999999999999999999",
            "1.",
        ] {
            assert!(cents(s).is_err());
        }
    }
    #[test]
    fn md5_sign_verify() {
        let cfg = json!({"epayVersion":"v1","merchantKey":"test-key"});
        let mut p = Params::from([("money".into(), "1.00".into()), ("pid".into(), "1".into())]);
        sign(&mut p, &cfg).unwrap();
        verify(&p, &cfg).unwrap();
        p.insert("money".into(), "2.00".into());
        assert!(verify(&p, &cfg).is_err());
    }
    #[test]
    fn canonical_order() {
        assert_eq!(
            canonical(&Params::from([
                ("b".into(), "2".into()),
                ("a".into(), "1".into()),
                ("sign".into(), "x".into()),
                ("blank".into(), "".into())
            ])),
            "a=1&b=2"
        );
    }
}
