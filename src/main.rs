mod auth;
mod cache;
mod db;
mod excel;
mod extra;
mod generator;
mod jobs;
mod mail;
mod payment;
mod system;
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{any, get, post},
};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

pub type Result<T> = std::result::Result<T, AppError>;
#[derive(Debug)]
pub struct AppError(pub StatusCode, pub String);
impl AppError {
    pub fn bad(s: &str) -> Self {
        Self(StatusCode::UNPROCESSABLE_ENTITY, s.into())
    }
    pub fn forbidden() -> Self {
        Self(StatusCode::FORBIDDEN, "没有操作或数据权限".into())
    }
    pub fn missing() -> Self {
        Self(StatusCode::NOT_FOUND, "记录或接口不存在".into())
    }
}
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref d) = e
            && d.is_unique_violation()
        {
            return Self::bad("名称或标识已存在");
        }
        tracing::error!(error=%e,"database operation failed");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "数据库操作失败".into())
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"code":self.0.as_u16(),"msg":self.1}))).into_response()
    }
}
#[derive(Clone)]
pub struct App {
    pool: MySqlPool,
    key: [u8; 32],
    url: String,
    captcha: bool,
    token_minutes: i64,
    schema: Arc<Value>,
    gates: Arc<Mutex<auth::Gates>>,
    passwords: Arc<tokio::sync::Semaphore>,
    imports: Arc<tokio::sync::Semaphore>,
    started: Instant,
}
fn env(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.into())
}
pub fn ok() -> Value {
    json!({"code":200,"msg":"操作成功"})
}
pub fn data(v: Value) -> Value {
    json!({"code":200,"msg":"操作成功","data":v})
}
pub struct Input {
    method: Method,
    path: String,
    params: HashMap<String, String>,
    body: Value,
    headers: HeaderMap,
    ip: String,
}
impl Input {
    pub fn s(&self, k: &str) -> String {
        self.body
            .get(k)
            .map(db::scalar)
            .or_else(|| self.params.get(k).cloned())
            .unwrap_or_default()
    }
    pub fn value(&self, k: &str) -> Value {
        self.body
            .get(k)
            .cloned()
            .unwrap_or_else(|| json!(self.params.get(k).cloned().unwrap_or_default()))
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}", e.1);
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    if cmd == "--version" {
        println!("ruoyi-rust-by {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if cmd == "keygen" {
        println!("{}", auth::random_hex(32));
        return Ok(());
    }
    if !["serve", "init", "migrate"].contains(&cmd.as_str()) {
        return Err(AppError::bad(
            "用法: ruoyi-rust-by [serve|init|migrate|keygen|--version]",
        ));
    }
    let url = std::env::var("DATABASE_URL").map_err(|_| AppError::bad("请配置 DATABASE_URL"))?;
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET time_zone='+00:00'").execute(conn).await?;
                Ok(())
            })
        })
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await?;
    if cmd == "init" {
        db::initialize(&pool, env("ADMIN_INITIAL_PASSWORD", "")).await?;
        println!("初始化完成；请从 .env 删除 ADMIN_INITIAL_PASSWORD");
        return Ok(());
    }
    if cmd == "migrate" {
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|_| AppError::bad("数据库迁移失败"))?;
        println!("数据库迁移完成");
        return Ok(());
    }
    let key: [u8; 32] = hex::decode(env("APP_KEY", ""))
        .map_err(|_| AppError::bad("APP_KEY 必须是 64 位随机十六进制字符"))?
        .try_into()
        .map_err(|_| AppError::bad("请运行 keygen 生成 APP_KEY"))?;
    let app = App {
        pool,
        key,
        url: env("APP_URL", "http://127.0.0.1:8000")
            .trim_end_matches('/')
            .into(),
        captcha: env("CAPTCHA_ENABLED", "true") != "false",
        token_minutes: env("TOKEN_MINUTES", "120")
            .parse::<i64>()
            .unwrap_or(120)
            .clamp(5, 10080),
        schema: Arc::new(serde_json::from_str(include_str!("../database/schema.json")).unwrap()),
        gates: Arc::new(Mutex::new(auth::Gates::default())),
        passwords: Arc::new(tokio::sync::Semaphore::new(4)),
        imports: Arc::new(tokio::sync::Semaphore::new(1)),
        started: Instant::now(),
    };
    tokio::fs::create_dir_all("uploads")
        .await
        .map_err(|_| AppError::bad("不能创建 uploads 目录"))?;
    jobs::start(app.clone());
    let api = Router::new()
        .route("/common/upload", post(extra::upload))
        .route("/system/user/importData", post(excel::import))
        .route("/system/user/profile/avatar", post(extra::upload))
        .fallback(any(dispatch))
        .with_state(app.clone())
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        ));
    let router = Router::new()
        .nest("/api", api)
        .route("/", get(|| async { Redirect::temporary("/admin/") }))
        .nest_service(
            "/admin",
            ServeDir::new("public/admin")
                .precompressed_gzip()
                .fallback(ServeFile::new("public/admin/index.html")),
        )
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        ));
    let address: SocketAddr = env("BIND_ADDR", "127.0.0.1:8000")
        .parse()
        .map_err(|_| AppError::bad("BIND_ADDR 无效"))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|_| AppError::bad("无法绑定监听地址"))?;
    tracing::info!(%address,"ruoyi-rust-by started");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .map_err(|_| AppError::bad("HTTP 服务退出"))?;
    Ok(())
}
async fn dispatch(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
) -> Result<Response> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path().trim_matches('/').to_string();
    let params = serde_urlencoded::from_str(parts.uri.query().unwrap_or(""))
        .map_err(|_| AppError::bad("查询参数格式错误"))?;
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| AppError::bad("请求体超过 1 MB"))?;
    let value = if bytes.is_empty() {
        json!({})
    } else if parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .starts_with("application/x-www-form-urlencoded")
    {
        let v: HashMap<String, String> =
            serde_urlencoded::from_bytes(&bytes).map_err(|_| AppError::bad("表单格式错误"))?;
        json!(v)
    } else {
        serde_json::from_slice(&bytes).map_err(|_| AppError::bad("JSON 格式错误"))?
    };
    let input = Input {
        method: parts.method,
        path,
        params,
        body: value,
        headers: parts.headers,
        ip: peer.ip().to_string(),
    };
    if let Some(v) = auth::public_api(&app, &input).await? {
        return Ok(Json(v).into_response());
    }
    if input.path == "site/config" && input.method == Method::GET {
        return Ok(Json(data(extra::settings(&app, "site", true).await?)).into_response());
    }
    if let Some(response) = payment::public(&app, &input).await? {
        return Ok(response);
    }
    let actor = auth::actor(&app, &input.headers).await?;
    let start = Instant::now();
    let response = if let Some(v) = auth::private_api(&app, &actor, &input).await? {
        Ok(Json(v).into_response())
    } else if input.path.starts_with("monitor/job") {
        jobs::handle(&app, &actor, &input).await
    } else if input.path.starts_with("monitor/cache") {
        cache::handle(&app, &actor, &input).await
    } else if input.path.starts_with("payment/") {
        payment::handle(&app, &actor, &input).await
    } else if input.path == "system/user/importTemplate" {
        excel::template(&actor)
    } else if input.path.starts_with("tool/gen") {
        generator::handle(&app, &actor, &input).await
    } else if let Some(v) = extra::handle(&app, &actor, &input).await? {
        Ok(v)
    } else {
        system::handle(&app, &actor, &input).await
    };
    if input.method != Method::GET {
        let status = if response.is_ok() { "0" } else { "1" };
        let _=db::exec(&app.pool,"INSERT INTO sys_oper_log (title,request_method,oper_name,oper_url,oper_ip,status,cost_time) VALUES (?,?,?,?,?,?,?)",&[json!(input.path.chars().take(50).collect::<String>()),json!(input.method.as_str()),actor.user["userName"].clone(),json!(format!("/api/{}",input.path)),json!(input.ip),json!(status),json!(start.elapsed().as_millis() as i64)]).await;
    }
    response
}
