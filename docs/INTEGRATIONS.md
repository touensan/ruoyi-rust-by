# 支付、邮件、任务、Redis 与 Excel

本页描述 v0.2.0 的接口与实际边界。所有管理接口以 `/api` 为前缀，需要 Bearer 会话和服务端权限。支付通知是公开接口，依靠商户签名及订单核对认证。

## 从 v0.1.0 升级

先备份数据库、二进制、静态文件与 `.env`，保留 APP_KEY。停止本应用进程后，使用新二进制执行 `ruoyi-rust-by migrate`，再替换 `public/admin` 并启动。不要重新运行 `init`，不要删除原库。

迁移 `0002_integrations.sql` 新增支付订单、定时任务、任务日志表，以及菜单 2100–2109；已有账号、角色和业务表保持原样。若部署方已使用这些菜单 ID，应在升级前解决冲突。MySQL DDL 不保证整体回滚，迁移异常时应检查迁移记录和备份，不要盲目重复执行或修改旧迁移校验和。

新增可选环境项：

```dotenv
REDIS_URL=redis://127.0.0.1:6379/0
REDIS_CACHE_PREFIX=ruoyi:cache:
SCHEDULER_ENABLED=true
```

Redis 不配置时，监控接口明确提示未配置，其他功能可使用。支持 `rediss://` TLS 连接；凭据放在受保护的 `.env`，不要写入参数管理或日志。数据库连接统一使用 UTC，任务与日志时间均按 UTC 显示。

## 易支付网关

系统配置中支持 V1（MD5）和 V2（RSA-SHA256 / PKCS#1 v1.5）。V2 使用至少 2048 位 PEM 商户私钥、平台公钥；支持 PKCS#8/PKCS#1 私钥及 SPKI/PKCS#1 公钥。密钥继续使用 AES-GCM 加密存储、读取掩码。网关和回调地址必须是 HTTPS；仅显式测试模式允许回环 HTTP。

保存并启用配置后，“创建 0.01 元测试订单”会二次确认并真实下单，不会自动付款。“最近支付订单”可以刷新本地状态或向网关查询。创建操作使用已保存配置，测试前先保存。

| 方法与接口 | 用途 |
| --- | --- |
| POST `/system/setting/payment/test` | 创建 0.01 元测试订单 |
| POST `/payment/orders` | 提交 `outTradeNo`、`money`（十进制字符串）、`payType`、`subject` 创建订单 |
| GET `/payment/orders` | 最近 100 条订单 |
| GET `/payment/orders/{outTradeNo}` | 查询本地订单 |
| POST `/payment/orders/{outTradeNo}/query` | 调用网关查询并核对已支付状态 |
| GET/POST `/system-config/payment/notify` | 验签通知，成功返回纯文本 `success` |
| GET `/system-config/payment/return` | 用户返回提示；不修改订单 |

下单/网关查询需超级管理员及 `system:setting:pay:test`，本地查询需超级管理员及 `system:setting:query`。这是框架管理接口，业务下单接入时应自行增加服务端定价和业务实体权限，不能直接给客户开放任意金额下单。

金额以整数分保存，范围 0.01–1000000.00 元。商户订单号唯一；同号同参数请求返回已有订单，不重复向网关下单；不同参数拒绝。请求超时可能代表网关已受理，所以保留 pending 订单，通过原订单号查询，不能盲目重试为新订单。

通知核对签名类型、商户、版本、订单号、平台流水号、支付方式和金额；V2 同时核对五分钟内时间戳，成功网关响应必须验签。数据库行锁和唯一流水号保证重复通知幂等。前台返回页不作为到账依据。

当前包含下单、查询与收款状态账本；没有退款、代付、钱包余额、自动发货或财务总账。已有 pending 订单期间应保留其商户和密钥配置，轮换配置前完成核对。各易支付部署的协议扩展不同，上线仍需对应商户沙箱/小额联调。本次测试使用本地协议模拟网关，未使用真实商户或实际扣款。

协议参考：同系列 [RuoYi-Go BY 系统设置服务](https://github.com/touensan/ruoyi-go-by/blob/main/app/service/system_setting_service.go) 与 [易支付 V2 文档](https://pay.newapi.link/doc/index.html)。V1 使用 `/mapi.php`、`/api.php`，V2 使用 `/api/pay/create`、`/api/pay/query`。

## SMTP 发送

保存并启用邮箱配置后，`POST /system/setting/mail/test` 接收 `to`、`subject`、`body`；缺省使用已保存测试收件人及测试文案。需要超级管理员及 `system:setting:mail:test`。

支持隐式 TLS（ssl）、强制 STARTTLS（starttls，兼容旧值 tls）和明确选择的明文（none），支持 SMTP 认证及 UTF-8 邮件。TLS 校验证书；STARTTLS 不可用会失败，不自动降级。主机/邮箱/主题校验、连接和整体投递超时、敏感错误脱敏均在服务端执行。

成功表示 SMTP 服务器接受了邮件，不保证最终入箱。超时后先核对服务器投递记录，避免重复发送。没有邮件队列、重试、附件和营销群发。测试采用回环 SMTP 接收器，验证认证、邮件内容及 STARTTLS 拒绝降级；真实公网证书/供应商限额和外部送达需部署方核对。

## 定时任务

菜单“系统监控 → 定时任务”，支持新增/修改/暂停/恢复/删除、执行一次、日志查询/清空/CSV 导出。服务内置每秒调度器，`SCHEDULER_ENABLED=false` 可关闭自动调度。数据库保存下次执行时间，MySQL 行锁串行化同一任务的手动和定时执行，适用于共享数据库的多进程。

Cron 使用 6 或 7 段：秒 分 时 日 月 周 [年]；UTC 时区。支持通配、列表、范围和步长，`?` 作为日期字段通配，不支持 Quartz 的 L/W/# 扩展。示例 `0 */5 * * * ?` 每五分钟一次。任务默认暂停，用户确认后启用。

| 内置目标 | 操作 |
| --- | --- |
| `system.heartbeat` | 记录服务运行检查 |
| `session.cleanup` | 删除已过期的登录会话 |
| `cache.ping` | 对已配置 Redis 执行 PING |

不执行任意 Shell、SQL、URL 或动态代码。写操作仅超级管理员且校验 `monitor:job:*` 权限。并发固定禁止；错过执行策略支持“执行一次”（2）和“放弃执行”（3，超过约两秒调度容差时跳过），不逐条回放停机期间所有计划。无未来执行时间的任务拒绝创建，到期后自动暂停。

执行日志保存开始/结束时间、结果、说明和脱敏错误。内置数据库操作与日志在同一事务内提交；Redis PING 是无副作用检查。扩展为有外部副作用的任务前，必须额外设计幂等和失败恢复，不能将当前调度器称为通用 exactly-once 执行引擎。

## Redis 缓存监控

菜单包含 Redis INFO/命令统计、当前数据库 DBSIZE，以及应用缓存列表、字符串预览、TTL、类型和清理。INFO/DBSIZE 为当前 Redis 实例/选中数据库指标，不是应用前缀的占用量。

列表使用 SCAN，最多 200 批、10000 个键；超限明确失败，不使用阻塞 KEYS。读写键必须以 `REDIS_CACHE_PREFIX` 开头；“清理应用缓存”只删除该前缀，绝不执行 FLUSHDB/FLUSHALL。字符串预览最多 64 KiB；其他类型仅显示类型和 TTL。列表有扫描时一致性限制；并发新建的键可能不在本次清理中，超时后需刷新核对进度。

监控需 `monitor:cache:list`，列表/读取需 `monitor:cache:query`，清理额外需要 `monitor:cache:remove`；所有 Redis 管理操作目前仅超级管理员。应使用独立应用前缀和受限 Redis ACL，勿将会话令牌、密钥或其他服务数据放进可预览缓存前缀。该模块是监控管理功能，没有把登录验证码和限流改成分布式 Redis 状态。

## Excel 用户导入

用户管理“导入”可下载 `.xlsx` 模板，上传首个工作表。最多 500 行、5 MB；不支持旧 `.xls`、公式单元格或额外列。预检查解压大小、文件数和工作表坐标，单实例同时只处理一个导入。

模板固定为：登录名称、用户昵称、部门编号、邮箱、手机号码、性别、状态、初始密码。账号和手机号建议设为文本，避免 Excel 自动改变前导零；性别 0/1/2，状态 0 正常/1 停用。

新账号必须填写符合现有规则的初始密码（至少 12 字符、最多 72 字节），不会默认分配角色。勾选更新已存在账号时，只更新个人资料、部门和状态，不覆盖密码或角色。超级管理员账号必须在个人中心维护，不允许导入覆盖。

接口：`POST /system/user/importTemplate` 下载模板；`POST /system/user/importData?updateSupport=0|1`，multipart 字段 `file`。均需 `system:user:import`；新增额外需 `system:user:add`，更新额外需 `system:user:edit`。逐行应用用户数据范围；非管理员只能写本部门。重复账号、无效部门、越权或其他失败都会整批回滚；停用账号同时撤销会话，成功后记录导入审计，不记录表格中的密码。
