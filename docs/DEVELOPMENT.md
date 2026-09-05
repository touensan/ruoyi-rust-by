# 开发与逻辑原理

## 请求链路

Axum 路由先区分公开入口（验证码、登录、退出、站点公开配置）与鉴权入口。受保护请求把 Bearer 值计算为 SHA-256，匹配 MySQL 中未过期的令牌并检查用户是否启用。每次请求读取有效角色和菜单，因此停用角色、修改权限后无需等待前端刷新即可在服务端生效。

实体管理只接受内置 `database/schema.json` 描述的 11 类系统实体；不接受客户端提供表名或列名作为 SQL。字段值绑定参数，输出转换为 camelCase，密码/令牌摘要/软删除时间不对外返回。

用户数据范围形成 SQL WHERE 约束，先于分页执行，并复用于详情、修改、删除、导出。多角色授权取并集，无授权范围默认仅本人。角色/菜单/部门修改及角色分配还必须通过超级管理员检查，防止拥有某个普通 CRUD 权限的用户自我提权。

角色/用户关联修改在事务内提交；重设密码同时撤销该用户所有会话。部门层级更新检查祖先链，并更新子部门 ancestors。MySQL 表使用软删除时保留唯一键，已删除的用户名/角色标识不能再次使用。

## 验证码、密码与配置

验证码是 PNG 光栅图，响应不包含答案；随机答案仅存在服务端内存中，每次尝试都消费验证码。登录同时按实际对端 IP 和账号限速。bcrypt 放到阻塞线程池处理，并限制并发验证数。密码的 72 字节限制在服务端按 UTF-8 字节检查。

APP_KEY 是 32 字节随机密钥的 hex 表示。配置密钥使用随机 nonce 的 AES-256-GCM，关联数据绑定到配置组及字段名，防止密文跨字段替换。读取配置返回 `********`，保存相同掩码表示保留已有值。`frontendHeadCode` 被清空，站点配置不能注入脚本。

上传只解码支持的光栅图片格式并重新编码为 PNG，文件名随机生成；不直接把用户原始文件发布到静态目录。上传与日志容量规划由部署方负责；过期会话可由 session.cleanup 内置定时任务清理。

## 代码生成

从 information_schema 读取当前数据库的表和列元数据，表名先验证标识符再核对数据库。只下载模型和查询模板，不接受任意 SQL，不将代码写入运行中的服务器，不自动暴露路由。后续完整 CRUD 生成需要配套字段验证、写操作权限和数据范围模板。

## 验证命令

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cd frontend && npm ci && npm run build:prod
```

HTTP 集成测试需要专用空测试库。启动单独服务时配置 `RUOYI_TEST_MODE=true`、`CAPTCHA_ENABLED=false`、独立端口和对应 `DATABASE_URL`，先运行 `init`。

```bash
TEST_BASE_URL=http://127.0.0.1:18889 \
TEST_ADMIN_PASSWORD='你为测试库设置的密码' \
python3 tests/api_test.py
```

测试会创建、修改和删除专用测试数据。运行器先检查服务返回的 `testMode` 标志，不满足时拒绝任何写入。不要在生产实例设置这个标志，也不要把测试服务暴露到公网。验证码启用状态还需用浏览器验证实际登录。

## 版本与构建

后端 Rust 工具链由 `rust-toolchain.toml` 固定，Cargo.lock 与 npm 锁文件必须提交。资源较小的生产主机应在独立构建环境编译，再核对产物 SHA-256。Linux 发布目标为 x86_64-unknown-linux-musl；自行构建该目标时需要 `rustup target add x86_64-unknown-linux-musl` 及 musl C 链接器。

CI 模板位于 `docs/ci/github-actions.yml`。本次 GitHub 凭证不具有 workflow scope，因此模板未放入活动 workflows 目录；需要仓库维护者使用具备权限的凭证启用。

实际验证结果记录在 `docs/VALIDATION.md`，不以本文件中的计划命令代替成功证据。

## v0.2.0 集成模块

新增模块和增量迁移见 [INTEGRATIONS.md](INTEGRATIONS.md)。`tests/integrations_test.py` 在原十组 API 回归上增加支付 V1/V2、SMTP、Redis、调度和 XLSX 事务/数据范围用例；仅在显式测试模式的隔离 MySQL/Redis 上运行，网关与 SMTP 为进程内回环协议接收器。

## 2026-09-05｜统一后台 RBAC（实现与验证完成）

统一架构契约见 [ARCHITECTURE.md](ARCHITECTURE.md)。`admin` 单向继承启用的 `common` 功能授权，服务端菜单、路由、接口与前端权限规则一致；前端全权限判断来自服务端 `*:*:*`，角色全部撤销时同步清空旧权限。

- Rust 1.98.1：14 项单元测试、Clippy（`-D warnings`）、fmt 及 musl release 构建通过。18 组真实 MySQL 5.7.43/Redis/API 回归通过，包含新增的角色继承、共同个人中心、动态路由和停用后即时拒绝场景。
- Node 22.22.3：`npm ci`、7 项 `npm run test:rbac` 和生产构建通过。
- Playwright 使用真实测试 API 和两类合成账号，验证共用登录、管理员访问普通功能及管理功能、普通用户仅见普通菜单且管理接口返回 403；1440×1000 与 390×844 页面通过，未观察到 pageerror 或异常 API 请求。截图只用于本次合成测试验收。
- 最终受测 Rust musl 二进制 SHA-256：`3f02a77b255c1c1a59a1d342dde13410fc396e3340f41eb2993342ba513ffa58`，取回后版本检查通过。代码仍为 0.2.0，本次没有创建新 Release/标签或替换此前发布包。
- 数据库结构、显式角色分配和数据范围未因该实现自动迁移；只是功能权限计算时加入有效 `common` 角色。依赖锁文件未变更，既有依赖风险记录继续适用。

本次调整公开框架源码与开发约定，不涉及其他业务站点的生产部署。Go/PHP 前端通过权限函数回归和构建；两类账号的真实浏览器场景在同系列 Rust 测试实例执行，不把它描述为三个后端均运行了浏览器验收。
