# 源码、标签与下载包状态

更新日期：2026-09-08。功能描述按所属版本解读，历史发布说明不代表当前源码。

**v0.2.0 提供 Linux x86_64 musl 运行包**，包含后端二进制、Vue 前端、部署模板和文档，实现支付、SMTP、定时任务、Redis 监控、XLSX 用户导入和统一后台 RBAC。

| 来源 | 实际含义 |
| --- | --- |
| [v0.2.0 标签及运行包](https://github.com/touensan/ruoyi-rust-by/releases/tag/v0.2.0) | 五项集成和统一后台 RBAC；源码提交及后端摘要见包内 BUILD_INFO.json，压缩包摘要见 SHA256SUMS |
| main | 继续开发的分支；下载包固定对应发布标签，不随 main 自动变化 |
| [v0.1.0 历史初版](https://github.com/touensan/ruoyi-rust-by/releases/tag/v0.1.0) | 不含支付网关、SMTP 发送、Redis/定时任务、XLSX 导入和后续 RBAC 继承修正；对应提交 1d0e056225b11fdb3e45a18da71141b28efb80f0 |

## 获取当前功能

从 [v0.2.0 Release](https://github.com/touensan/ruoyi-rust-by/releases/tag/v0.2.0) 下载 `ruoyi-rust-by-v0.2.0-linux-x86_64.tar.gz` 和 `SHA256SUMS`，执行 `sha256sum -c SHA256SUMS` 后解包。运行时需要 MySQL，可选 Redis；不需要安装 Rust 或 Node.js。

初次安装与旧版升级见 [部署说明](DEPLOYMENT.md) 和 [集成说明](INTEGRATIONS.md)。已有部署先备份数据库、配置、上传和旧产物，再执行 `./ruoyi-rust-by migrate`，不要重新初始化已有业务库。

GitHub 自动生成的 Source code ZIP/TAR 是标签源码，不能替代已编译运行包。旧 v0.1.0 标签和附件保持原样；其内部文档属于历史快照。

## 维护约定

新增、移植或撤销功能时，同步 README、FEATURES、开发记忆与本页。发布新包时记录源码提交和校验和，确认后端、前端与该提交一致；后端源码未变时可复用已经验证并核对摘要的产物，必须记录来源。不得把“未真实供应商联调”写成“未实现”，也不得把旧版本局限写成整个语言框架的当前局限。
