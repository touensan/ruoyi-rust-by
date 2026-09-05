# 源码、标签与下载包状态

核对日期：2026-09-05。功能描述按所属版本解读，历史发布说明不代表当前源码。

当前 `main` 的源码版本为 **v0.2.0**，已实现支付、SMTP、定时任务、Redis 监控、XLSX 用户导入和统一后台 RBAC。**GitHub 现有 Release 与运行下载包仍为历史 v0.1.0，尚未发布 v0.2.0 下载包。**

| 来源 | 实际含义 |
| --- | --- |
| `main`（源码 0.2.0） | 五项集成和统一后台 RBAC 已实现 |
| `v0.1.0` 标签及 Linux 运行包 | 历史初版；不含支付网关、SMTP 发送、Redis/定时任务、XLSX 导入和后续 RBAC 继承修正 |

## 获取当前功能

使用 [main 源码](https://github.com/touensan/ruoyi-rust-by/tree/main)，记录 `git rev-parse HEAD` 的完整提交号，并按 [README](../README.md) 构建/安装。已有部署升级前先备份并阅读迁移说明。

[Releases](https://github.com/touensan/ruoyi-rust-by/releases) 中的附件和 GitHub 自动生成的 Source code ZIP/TAR 均对应其标签，不能因发布页位于列表首位、标记 Latest 或版本号较大，就当成 main 的最新代码。

本次更正文档和发布说明，不移动旧标签、不替换旧包、不改变其校验和。旧包内的文档也属于该版本的历史快照；当前状态请以本页为准。

## 已核对的标签与附件

| 标签 | 对应源码提交 | 附件 |
| --- | --- | --- |
| [v0.1.0](https://github.com/touensan/ruoyi-rust-by/releases/tag/v0.1.0) | [`1d0e056`](https://github.com/touensan/ruoyi-rust-by/tree/1d0e056225b11fdb3e45a18da71141b28efb80f0) | ruoyi-rust-by-v0.1.0-linux-x86_64.tar.gz, SHA256SUMS |

## 维护约定

新增、移植或撤销功能时，同步 README、FEATURES、开发记忆与本页。发布新包时必须记录源码提交和校验和，并确认二进制、前端与该提交一致。未发布新包时明确标注“源码已实现，旧下载包不包含”，不能把“未真实供应商联调”写成“未实现”，也不能把旧版本局限写成整个语言框架的当前局限。
