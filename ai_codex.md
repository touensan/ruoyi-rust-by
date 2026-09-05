# RuoYi-Rust BY 长期本地记忆

> 新对话开始请先阅读本文件、AGENTS.md、docs/DEVELOPMENT.md 和 docs/FEATURES.md。继续开发时，及时把重要记忆、实现原理、开发里程碑、真实验证结果、已知问题更新进来。不要依赖跨对话自动记忆，不要把计划或工具启动写成已完成。

## 项目目标

用户要求：主要参考 `touensan/ruoyi-go-by`，同时参考 `chelunfu/qiluo_admin` 与 `y_project/RuoYi-Vue`，前端几乎保持不变，后端使用 Rust，数据库继续 MySQL；建立 `touensan/ruoyi-rust-by` GitHub 公开仓库，按同系列 Go/PHP 项目开源。

这是独立开源框架，不包含任何现有业务站点的私有代码、凭据、域名、数据库或运维配置。公开发布本框架不等于替换业务生产站点。

## 架构决策

- Axum 0.8、Tokio、SQLx 0.8.6/MySQL；Rust 工具链固定 1.98.1，实际依赖以 Cargo.lock 为准。
- Vue 3 + TypeScript + Element Plus + Vite；通过同系列 PHP 公开仓库复用已整理的 Go 原生前端和数据库定义。上游版本与许可证见 THIRD_PARTY_NOTICES.md。
- `/api` 是服务端接口，`/admin/` 是管理界面；Rust 二进制同时可提供前端静态资源和上传图片，Nginx 负责正式 HTTPS 入口。
- 只保存 Bearer 令牌摘要；有效角色/权限逐请求查询；密码变更和停用用户撤销会话。
- 登录验证码为一次性 PNG，限速/验证码属于单实例内存状态。多副本部署需要共享状态，不能直接复制单实例方案。
- 通用 CRUD 的表和字段来自内置受控清单，SQL 值全部绑定。数据范围必须应用在用户列表、详情、修改、删除、导出。角色、菜单、部门和角色分配仅超级管理员操作。
- AES-GCM 配置密文绑定组和字段，APP_KEY 必须持久保存。配置接口输出掩码；前端 head 脚本不执行。
- 生成器只输出 Rust 模型和分页示例，不自动执行 SQL 或部署代码。支付/邮件仅保存配置，不模拟联通成功。

## 2026-09-05 开发里程碑

1. 核对 Go/Qiluo 源码、提交与 MIT 许可；Gitee 原始地址访问受到 429/超时限制，前端使用固定 Go 仓库中的可获取版本。
2. 创建独立 Rust 项目、SQLx 迁移、净化种子与前端接口适配；实现系统管理核心模块。
3. 加入权限与数据范围、密钥加密、图片重新编码、CSV、通知/在线/日志和模板下载。
4. 首轮 Rust 单元测试 7 项通过；MySQL 5.7 + HTTP 集成测试扩展到 10 组，五种数据范围已验证。
5. 前端更新 Axios、js-cookie、ECharts 及 glob；生产依赖审计零报告。Rust 间接依赖 rsa 的上游未修复问题已写入 SECURITY.md。
6. 对照原前端做浏览器验证；手机组织树默认折叠、筛选区宽度适配，桌面布局保持。最终验证结果以 docs/VALIDATION.md 为准。公开仓库为 https://github.com/touensan/ruoyi-rust-by，初始版本 v0.1.0（预发布）。

## 实际踩坑与维护提醒

- musl-gcc 的部分 specs 会给静态 PIE 注入 `/lib/ld-musl-x86_64.so.1`。build.rs 对 musl 目标加入 `--no-dynamic-linker`；必须用 file/readelf 和实际启动确认产物，而非只看 Cargo 成功。
- Axum 的 `nest_service("/admin", ...)` 已注册对应路径，不要再重复 `.route("/admin", ...)`，否则启动时路由冲突。
- SPA 静态入口使用 ServeDir 的 fallback，避免 `not_found_service` 把正常管理页面变成 HTTP 404。
- `logout` 必须在令牌失效时仍返回成功，前端跳转要使用 BASE_URL + login，避免子目录部署下的重新登录循环。
- 登录界面的“记住账号”只保存账号，不使用固定前端 RSA 密钥保存密码。请求防重复提交只使用短时内存，并清除旧 sessionObj；不能把请求体写入 sessionStorage，因为登录和配置请求可能包含密钥。
- 变更项目名称时只改明确的包名和文案字段；不得在 lock 文件完整文本中替换语言缩写，可能误改完整性摘要。必须重新执行 npm ci 验证锁文件。
- 浏览器路由变化不等于视图已就绪；等待目标表格行、加载遮罩消失和过渡完成再截图。
- 不在生产数据库执行测试。HTTP 测试器要求目标明确返回 testMode=true；测试服务不得公网开放。
- GitHub 当前凭证不具备 workflow scope，CI 保存在 docs/ci/github-actions.yml 模板中，尚未自动运行。

## 后续方向

补齐 Redis/任务、Excel 导入、完整 CRUD 模板、邮件/支付真正接入与更完整监控；这些不是 v0.1.0 已实现能力。继续跟踪 RustSec 依赖修复，并为新增业务实体定义独立的数据权限。
