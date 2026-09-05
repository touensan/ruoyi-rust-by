use crate::{App, AppError, Input, Result, auth::Actor, data, db, extra};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub fn validate(cfg: &Value) -> Result<()> {
    let host = db::scalar(&cfg["host"]);
    if host.is_empty() || host.len() > 253 || host.contains(['/', '\r', '\n', '@']) {
        return Err(AppError::bad("SMTP 主机名无效"));
    }
    if !(1..=65535).contains(&db::num(&cfg["port"]))
        || !["ssl", "starttls", "tls", "none"].contains(&db::scalar(&cfg["encryption"]).as_str())
    {
        return Err(AppError::bad("SMTP 端口或加密方式无效"));
    }
    db::scalar(&cfg["fromEmail"])
        .parse::<lettre::Address>()
        .map_err(|_| AppError::bad("发件人邮箱无效"))?;
    Ok(())
}

pub async fn send(app: &App, to: &str, subject: &str, body: &str) -> Result<Value> {
    let cfg = extra::settings(app, "mail", false).await?;
    if cfg["enabled"] != true {
        return Err(AppError::bad("请先保存并启用 SMTP 配置"));
    }
    validate(&cfg)?;
    if subject.is_empty()
        || subject.len() > 200
        || subject.contains(['\r', '\n'])
        || body.len() > 100_000
    {
        return Err(AppError::bad("邮件主题或正文无效"));
    }
    let recipient = to
        .parse::<lettre::Address>()
        .map_err(|_| AppError::bad("收件人邮箱无效"))?;
    let msg = Message::builder()
        .from(Mailbox::new(
            Some(db::scalar(&cfg["fromName"])),
            db::scalar(&cfg["fromEmail"])
                .parse()
                .map_err(|_| AppError::bad("发件人邮箱无效"))?,
        ))
        .to(Mailbox::new(None, recipient))
        .subject(subject)
        .body(body.to_owned())
        .map_err(|_| AppError::bad("邮件格式无效"))?;
    let host = db::scalar(&cfg["host"]);
    let mut builder = match db::scalar(&cfg["encryption"]).as_str() {
        "ssl" => AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|_| AppError::bad("SMTP TLS 配置无效"))?,
        "starttls" | "tls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|_| AppError::bad("SMTP STARTTLS 配置无效"))?,
        _ => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host),
    }
    .port(db::num(&cfg["port"]) as u16)
    .timeout(Some(Duration::from_secs(10)));
    if !db::scalar(&cfg["username"]).is_empty() {
        builder = builder.credentials(Credentials::new(
            db::scalar(&cfg["username"]),
            db::scalar(&cfg["password"]),
        ));
    }
    let transport = builder.build();
    let start = Instant::now();
    tokio::time::timeout(Duration::from_secs(25), transport.send(msg))
        .await
        .map_err(|_| AppError::bad("SMTP 发送超时，请检查服务端投递状态后再重试"))?
        .map_err(|_| AppError::bad("SMTP 拒绝投递或连接失败，请检查服务器、证书和认证配置"))?;
    Ok(
        json!({"success":true,"message":"SMTP 服务器已接受邮件，最终送达请查看收件箱","server":format!("{}:{}", host, db::num(&cfg["port"])),"elapsedMs":start.elapsed().as_millis()}),
    )
}

pub async fn test(app: &App, a: &Actor, i: &Input) -> Result<Value> {
    a.require("system:setting:mail:test")?;
    a.superuser()?;
    let cfg = extra::settings(app, "mail", false).await?;
    let to = if i.s("to").is_empty() {
        db::scalar(&cfg["testRecipient"])
    } else {
        i.s("to")
    };
    let subject = if i.s("subject").is_empty() {
        "RuoYi-Rust BY 测试邮件".into()
    } else {
        i.s("subject")
    };
    let body = if i.s("body").is_empty() {
        "SMTP 发送测试。".into()
    } else {
        i.s("body")
    };
    Ok(data(send(app, &to, &subject, &body).await?))
}
