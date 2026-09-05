# v0.1.0：RuoYi-Rust BY 首个预发布版本

> **历史 v0.1.0 标签与下载包说明。** 当前 main 源码已是 v0.2.0，支付、SMTP、定时任务、Redis 监控和 XLSX 导入已实现。下面的未实现项与测试数字仅描述旧包；当前情况见 [VERSIONS.md](VERSIONS.md) 和 [INTEGRATIONS.md](INTEGRATIONS.md)。

保留 RuoYi-Go BY 系列的 Vue 3 / TypeScript 管理界面，使用 Rust / Axum + MySQL 实现后端。

实现登录和验证码、可撤销会话、用户/角色/菜单/部门/岗位/字典/参数/通知管理、五种用户数据范围、日志、在线会话、配置加密、图片上传、CSV 导出及 Rust 模型/分页查询模板生成。

验证：7 项 Rust 单元测试、10 组真实 MySQL HTTP 集成测试、Clippy、前端构建及桌面/手机浏览器检查。附带 Linux x86_64 musl 二进制与前端静态资源，可直接按文档安装。

在 v0.1.0 发布时：支付网关、SMTP 发送、Redis/定时任务、Excel 导入和完整 CRUD 生成尚未实现。生产前请阅读 `docs/FEATURES.md`、`SECURITY.md` 和 `docs/VALIDATION.md`，其中明确记录未修复的上游 RSA 与前端构建依赖审计问题。生产前端依赖审计为零报告；CI 模板尚未激活。

安装步骤见 README 与 `docs/DEPLOYMENT.md`。管理员密码由安装者初始化时自选，没有通用默认密码。下载后使用 `SHA256SUMS` 校验压缩包。
