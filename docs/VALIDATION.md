# v0.2.0 验证记录（2026-09-05）

本次使用 Rust 1.98.1、Node.js 22.22.3、独立 MySQL 5.7.43 和 Redis 8.2.3。测试服务与协议模拟器仅监听回环地址，没有接触生产数据库、真实商户或外部收件人。

| 验证 | 结果 |
| --- | --- |
| cargo fmt --check / cargo check | 通过 |
| cargo clippy --locked --all-targets -- -D warnings | 通过 |
| cargo test --locked | 14 项通过 |
| tests/integrations_test.py | 最终 musl 0.2.0 二进制上 17 组通过，13.885 秒 |
| v0.1.0 → v0.2.0 迁移 | 独立旧版库执行增量迁移通过；全部原用户行摘要一致，新增 10 个菜单 |
| npm ci / npm run build:prod | Node 22 隔离安装与最终构建通过 |
| npm audit --omit=dev（npmjs 官方源） | 0 项报告 |
| 完整 npm audit | 7 high / 2 moderate，仍为已有开发工具链问题 |
| OSV / Cargo.lock | 检查 341 项依赖；剩余 rsa 0.9.10 / RUSTSEC-2023-0071 上游问题 |
| Linux 二进制 | x86_64 musl 静态 PIE，无 INTERP；远端与本机 SHA-256 一致，本机运行版本检查通过 |
| 浏览器 | 1440×1000 与 390×844，任务新增/执行/日志、订单列表与下单确认、缓存、Excel 模板下载通过；未观察到页面错误或失败 API |

新增集成测试覆盖 V1 MD5 和 V2 RSA 请求及通知、网关返回验签、金额不符拒绝、重复下单/回调、查询到账、SMTP 认证与真实协议投递、STARTTLS 拒绝降级、Redis INFO/TTL/前缀清理、定时执行/暂停/手动执行与日志、日志和导出筛选、XLSX 模板/新增/更新/整批回滚、超级管理员保护及拥有导入权限的普通用户数据范围。原有十组鉴权、系统 CRUD、导出、配置、生成器等回归也包含在上述 17 组中。

依赖审计促使 Excel 读取升级为 calamine 0.36.1、quick-xml 0.41.0，消除了初选旧 XML 版本的 RUSTSEC-2026-0194 / 0195。RSA 公告仍保留并在 SECURITY.md 说明；不能称为依赖零风险。SMTP 生产证书链/外部送达、真实支付供应商差异、MySQL 8.0 和大规模集群压力未在本次验证。

后端二进制 SHA-256：

```text
d4fe7229d6d83756809ddbc8656fedf3a93bdfd85f37cdf37cd4554e0e65ecdc
```

前端截图与复核见根目录 design-qa.md。CI 模板仍未启用；以上为实际手工执行记录，不是 GitHub Actions 结果。

---

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

## 2026-09-05｜统一后台 RBAC 增量验证

- Rust 1.98.1：14 项单元测试、Clippy（`-D warnings`）、fmt 及 musl release 构建通过。18 组真实 MySQL 5.7.43/Redis/API 回归通过，包含新增的角色继承、共同个人中心、动态路由和停用后即时拒绝场景。
- Node 22.22.3：`npm ci`、7 项 `npm run test:rbac` 和生产构建通过。
- Playwright 使用真实测试 API 和两类合成账号，验证共用登录、管理员访问普通功能及管理功能、普通用户仅见普通菜单且管理接口返回 403；1440×1000 与 390×844 页面通过，未观察到 pageerror 或异常 API 请求。截图只用于本次合成测试验收。
- 最终受测 Rust musl 二进制 SHA-256：`3f02a77b255c1c1a59a1d342dde13410fc396e3340f41eb2993342ba513ffa58`，取回后版本检查通过。代码仍为 0.2.0，本次没有创建新 Release/标签或替换此前发布包。
- 数据库结构、显式角色分配和数据范围未因该实现自动迁移；只是功能权限计算时加入有效 `common` 角色。依赖锁文件未变更，既有依赖风险记录继续适用。
