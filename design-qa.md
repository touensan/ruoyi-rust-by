# 前端一致性与浏览器验收

- source visual truth path: `docs/screenshots/reference-users-desktop.png`、`reference-users-mobile.png`。来源为 ruoyi-php-by 公开前端（其基线为 ruoyi-go-by `fbaf4bf`），连接同一份隔离测试数据。
- implementation screenshot path: `docs/screenshots/users-desktop.png`、`users-mobile.png`、`settings-desktop.png`。
- viewport: 桌面 1440×1000 CSS px，手机 390×844 CSS px。
- source / implementation pixel dimensions: 分别与对应视口完全一致，deviceScaleFactor=1；均为浏览器原始截图，无缩放补偿。
- state: 管理员登录，用户管理列表加载完成；数据来自专用测试库。桌面保留原布局；小屏幕默认折叠组织树。
- focused region: `docs/screenshots/reference-users-table.png` 与 `users-table.png`，均为 988×383，检查表格文字、行高、按钮、状态开关和列对齐。

## 检查结果

| 项目 | 结果 |
| --- | --- |
| 字体与层级 | 沿用原字体栈与字号、字重，桌面标题/筛选/表格层级一致 |
| 间距与布局 | 桌面 200px 主侧栏、220px 组织树、筛选和表格布局一致；手机修复组织树占据主要视口的问题 |
| 色彩与状态 | 沿用原深色侧栏、蓝色主色、边框、禁用和启用状态，不另造主题 |
| 图像与图标 | 沿用原 logo、头像及图标资源，没有用手绘图形替换；图像在原始比例下清晰 |
| 文案 | 项目名和技术栈改为 Rust；未实现功能标明状态；导入按钮禁用；配置图标上传提示与 PNG 支持一致 |
| 交互 | 已检查登录、动态菜单、用户/角色/部门/字典/参数/系统配置/生成器/服务监控页面，以及退出后的 /admin/login 跳转 |
| 浏览器存储 | 不保存密码 Cookie；请求防重复提交不再写入 sessionStorage；实际检查 sessionObj 不存在 |
| 控制台 | 最终对照截图验收无未预期 JavaScript 或控制台错误 |

## 对照历史与修复

- [P2，已修复] 手机原组织树为 220px，挤压筛选项和用户列表。旧图 `docs/screenshots/users-mobile-before.png`；修复为手机初始折叠至 20px、选择部门后收起，并让筛选项自适应宽度。新图 `users-mobile.png`，390px 视口的 document.scrollWidth=390。表格保留内部横向滚动。
- [P2，已修复] 异步切页后直接截图会捕获加载遮罩或旧视图淡出状态。最终截图等待目标表格行、网络请求结束和过渡完成，旧加载中截图不作为验收证据。
- [P2，已修复] 子目录下退出地址与失效会话处理需一致。前端使用 BASE_URL + login，后端 logout 幂等，避免失效令牌阻断退出。
- 项目名称、Rust 技术说明、功能状态提示与小屏幕折叠属于本项目的预期差异。

未发现尚待修复的 P0/P1/P2；本次验证不代表所有上游未启用视图或所有业务弹窗已完成全面移动端审计。

补充验证：`docs/screenshots/login-captcha-desktop.png`（1440×1000）与 `login-captcha-mobile.png`（390×844）验证开启验证码的真实登录。失效令牌正确返回登录页并保留 redirect 参数；手机登录框适配视口，密码 Cookie 不存在。

final result: passed
