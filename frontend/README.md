# RuoYi-Rust BY 统一后台前端

普通用户与管理员使用同一套前端、登录和页面，按 RBAC 动态授权。当前功能与下载版本见 [项目说明](../README.md) 和 [版本状态](../docs/VERSIONS.md)。

来自 RuoYi-Go BY 的 Vue 3 / TypeScript / Element Plus 界面。项目名称、API 适配和功能边界以根目录 README 为准。

开发：`npm ci` 后运行 `npm run dev`，开发接口 `/dev-api` 代理到 Rust 服务的 `http://127.0.0.1:8000/api`。

生产：`npm run build:prod`，产物在 `dist`，部署路径 `/admin/`、API 前缀 `/api`。提交并维护 package-lock.json；公开的 .env.development / .env.production 仅包含非敏感前端参数。
