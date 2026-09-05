# v0.1.0 验证记录

日期：2026-09-05。隔离构建环境使用 Rust 1.98.1、Node.js 22.22.3；真实数据库为独立 MySQL 5.7.43 实例。MySQL 8.0 为兼容目标，本次未单独运行 8.0 实例。

| 验证 | 结果 |
| --- | --- |
| cargo fmt --check | 通过 |
| cargo check --locked | 通过 |
| cargo clippy --locked --all-targets -- -D warnings | 通过 |
| cargo test --locked | 7 项通过 |
| tests/api_test.py | 最终 musl 发布二进制上 10 组通过，耗时约 6 秒 |
| 重复初始化 | 正确拒绝覆盖已有用户 |
| 配置密文检查 | MySQL 中的邮件密码为 enc:v1 密文，不含测试明文；接口返回掩码 |
| npm ci | 通过完整性检查 |
| npm run build:prod | 通过 |
| npm audit --omit=dev | 0 项报告 |
| 完整 npm audit（包含构建工具） | 7 high、2 moderate；具体范围见下文与 SECURITY.md |
| OSV / Cargo.lock | 1 项已知上游问题：rsa 0.9.10 / RUSTSEC-2023-0071，无已发布补丁 |
| Linux 产物 | x86_64 musl 静态 PIE，无 ELF INTERP / NEEDED；在构建机和 Ubuntu 20.04 本机执行版本检查成功 |
| 浏览器 | 桌面 1440×1000、手机 390×844；原前端与 Rust 前端对照，最终截图无未预期控制台错误 |

HTTP 测试覆盖鉴权与敏感字段输出、五种数据范围、跨范围读写/导出拒绝、角色提权拒绝、字典与参数 CRUD、部门循环检查、关联事务、公告已读、审计记录、配置加密/掩码、未实现接口明确报错、生成器预览与 ZIP、图片上传成功及 SVG 拒绝、密码变更/停用后的会话撤销。

前端完整审计仍涉及 braces、decode-uri-component、image-size、micromatch、postcss、source-map-resolve、svg-baker、vite、vite-plugin-svg-icons。这些属于开发/构建依赖链，不随二进制发布包作为 Node 服务运行；需要后续升级维护，不能据生产依赖零报告声称整个依赖树无风险。

后端二进制 SHA-256：

```text
21e472f91132ff7701f7393f8be29c9a43bc59cc8b3b916c21a2145db3015a06
```

发布压缩包的校验以 Release 附件 `SHA256SUMS` 为准。源码中没有上传真实凭据、生产数据库或业务站点配置。CI 模板尚未激活，本文件记录的是实际手工执行结果。

截图和详细对照见仓库根目录 `design-qa.md`。

开启 CAPTCHA_ENABLED 后，浏览器人工识读 PNG 并成功登录；替换为无效令牌后正确跳转 `/admin/login?redirect=/index`。手机登录截图 390×844 无横向溢出、无密码 Cookie，未出现未预期页面错误。对应截图为 `docs/screenshots/login-captcha-desktop.png` 与 `login-captcha-mobile.png`。
