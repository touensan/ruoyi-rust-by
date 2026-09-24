# 单机平滑更新

当前 `main` 提供双端口发布流程；v0.2.0 及以前下载包不包含。应用升级期间新旧二进制并行，候选在回环备用端口通过 `/internal/ready` 的数据库与版本检查后，Nginx 平滑 reload。旧 worker 完成连接后旧进程才可退出。共享 `uploads` 与 `/admin/assets/` 中保留的历史 hash 文件避免上传或旧页面资源丢失。工具不会自动删旧制品。

首次接入时把现有旧进程视为 legacy，监督核对其端口、上传目录、后台任务与登录流程；旧版无私有就绪接口。原 Nginx server block 内 include `nginx_include`，移除冲突的 `location /`、`/api/` 和 `/admin/assets/` 等旧代理/静态规则，保留原域名、TLS、日志和其他规则；宝塔也用现有虚拟主机。`probe_origin` 须指向命中该虚拟主机的本机 HTTP 监听，HTTPS 站点可另开仅回环 HTTP 监听供探测。`adopt` 写入指向旧端口的片段并 reload，不重启旧进程。私有配置以 `deploy/smooth-release.example.json` 为起点，`env_file` 保留现有 `DATABASE_URL`、`APP_KEY`、`APP_URL` 等，文件权限 0600；`shared_uploads` 先创建并交服务用户写入。候选用 `RUOYI_PORT` 覆盖监听端口，始终只绑定回环。

```sh
python3 deploy/smooth-release.py --config /private/release.json adopt legacy /old/runtime
python3 deploy/smooth-release.py --config /private/release.json prepare release-yyyymmdd /build/ruoyi-rust-by.tar.gz ARCHIVE_SHA256 0.2.0
python3 deploy/smooth-release.py --config /private/release.json start release-yyyymmdd
python3 deploy/smooth-release.py --config /private/release.json switch release-yyyymmdd
python3 deploy/smooth-release.py --config /private/release.json status
python3 deploy/smooth-release.py --config /private/release.json retire previous-managed-release
```

从隔离构建节点经 SSH/SCP 传运行包，确认归档 SHA256；`prepare` 再校验包内逐文件摘要、Linux x86_64 二进制、路径安全与上传目录为空。`switch` 使用恢复日志，核对 Nginx 实际端口与页面/API 探测，失败自动恢复旧路由；进程中断后下次命令会先恢复旧路由。`rollback` 返回仍在运行的上个版本；已停止的托管版本先 `start`。`retire` 发现旧 Nginx worker 仍在时会拒绝停进程。首次 legacy 进程还需人工确认调度任务排空并优雅停止，工具故意不自动退休它。systemd 用 SIGTERM，Rust 现在同时处理 SIGTERM 和 Ctrl-C 并优雅关闭 HTTP。先确认双进程内存余量，再准备候选。

`serve` 不自动运行 SQLx 迁移。数据库升级须先审查，使用兼容旧版的增量迁移；破坏性变更不能直接使用此流程。调度任务的数据库行锁会串行化相同任务的双实例执行；跨实例会话令牌存在 MySQL。**当前验证码和限速仍保存在各实例内存。** 旧版页面取到验证码后若跨切流提交到新版，可能提示验证码失效；首次更新不能宣称所有登录零影响。要消除该边界，需先把验证码与限速迁入共同存储，并在过渡版本与旧版共存后再切流。旧浏览器仍会调用旧 API，需保证新后端兼容；Nginx 不重放支付/写请求。

本次仅修改开源仓库，未启用任何业务站点。正式使用前应在隔离环境演练长请求、上传、登录、任务、旧静态文件、切流失败回退和后台服务存活。
