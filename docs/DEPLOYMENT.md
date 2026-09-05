# 部署

## 编译与运行的区别

Rust 需要用 Cargo 编译后端；Vue 使用 Vite 打包前端。最终服务器运行 `ruoyi-rust-by` 二进制，Nginx 转发 HTTP 请求。运行服务时不需要 Rust 编译器、Node.js、PHP-FPM 或 Composer。

发布包的 Linux x86_64 版本使用 musl 静态链接。开发者也可自行在目标平台执行 `cargo build --locked --release`。发布二进制不包含环境凭证、上传文件或数据库内容；迁移和净化种子嵌入二进制。

## 初次安装

1. 创建专用数据库，例如 `ruoyi_rust_by`，字符集 `utf8mb4`；为它分配专用账号。`init` 需要建表权限，运行账号仅需业务表读写权限。
2. 将二进制、`public/admin` 和 `.env` 放入独立应用目录；`uploads` 必须由进程用户可写。
3. `.env` 配置 `DATABASE_URL`、`APP_KEY`、`APP_URL=https://你的域名`、`BIND_ADDR=127.0.0.1:8000`。用 `./ruoyi-rust-by keygen` 生成 APP_KEY。
4. 设置自选初始密码，运行 `./ruoyi-rust-by init`。有用户数据时会拒绝重复初始化。删除 `.env` 中的初始密码字段，权限设为 `600`。
5. 参考 `deploy/ruoyi-rust-by.service` 启动专用 systemd 服务，再参考 `deploy/nginx.conf` 设置 HTTPS 反向代理。
6. 登录 `/admin/` 验证接口与数据库持久化。宝塔部署同样使用反向代理，不选择 PHP 运行环境。

应用从工作目录读取 `.env`、`public/admin`、`uploads`。密钥必须持久保存；更换 APP_KEY 前需要迁移已加密设置。HTTPS 证书按已有服务器的证书管理流程配置。

## 网络与会话

默认只监听回环地址，不启用跨域。生产中浏览器与 API 使用同一 HTTPS 域名。令牌保存在前端 Cookie 中供 Axios 添加 Authorization，属于上游 RuoYi 的客户端 Bearer 模式，请避免不受信任脚本。

服务依据实际 TCP 对端限速，不直接信任 X-Forwarded-For。位于 Nginx 后时 IP 限速会按代理地址汇总；目前适合单实例内部管理后台。并发公网部署前，应接入共享限速存储及可信代理解析。验证码每 120 秒过期，失败尝试同样消耗该验证码；单实例最多保存 10000 个验证码或限速键。

数据库远程连接应使用受保护网络和校验服务器证书的 TLS 配置。请勿用数据库 root 账号作为应用运行账号。

## 升级与回退

- 升级前备份数据库、`.env`、上传文件及当前二进制/前端产物。
- 在独立目录解包并核对 SHA256SUMS，按迁移内容安排停机或维护窗口。
- 使用 `./ruoyi-rust-by migrate` 执行 SQLx 版本迁移；普通启动不会自动修改数据库结构。
- 验证成功后切换服务工作目录或发布软链接，再重启本项目进程。
- 回退前核对数据库迁移兼容性；不要未经核对直接把旧版本接到新结构数据库。

本仓库不包含任何真实服务器配置、绑定域名、凭据或业务数据。
