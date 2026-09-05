# v0.2.0 五项集成的界面与浏览器验收

日期：2026-09-05。沿用已有若依界面，使用 Product Design 工作流检查现有视觉样式与新增操作。v0.1.0 原验收保留在 `docs/DESIGN_QA_V0_1.md`。

- source visual truth path：`docs/screenshots/settings-desktop.png`（v0.1.0 原界面）、`docs/screenshots/reference-users-desktop.png` / `users-mobile.png`；缓存复核前图 `docs/screenshots/features/cache-mobile-before.png`。
- implementation screenshot path：`docs/screenshots/features/settings-desktop.png`、`payment-desktop.png`、`job-log-desktop.png`、`import-mobile.png`、`cache-mobile.png`。
- viewport：桌面 1440×1000、手机 390×844 CSS px；截图像素与 CSS 尺寸相同，deviceScaleFactor=1，无密度缩放。应用内滚动容器截图代表当时可见视口，不声称涵盖所有滚动内容。
- state：隔离测试管理员登录；桌面配置/表格加载完成；手机侧栏关闭、弹窗过渡完成。数据均为测试数据；站点图标、描述和版权值与原图有测试数据差异，不属于布局变更。
- full-view comparison evidence：同一比较输入中打开旧/新系统配置全图；同一比较输入中打开缓存移动端修复前/后图。未将分别查看图片称为并排对照。
- focused region：直接检查原尺寸配置标签/输入框、手机导入弹窗的说明与操作区、缓存图表标签与图例、任务日志表格；这些字在原尺寸输入中清晰，无需额外裁剪。

## 五项视觉检查

| 表面 | 结果 |
| --- | --- |
| 字体与层级 | 原字体栈、字重、表格/表单字号保持；手机弹窗说明正常换行 |
| 间距与布局 | 桌面沿用原侧栏和双列表单；导入弹窗约 94vw；手机四条验收路由 document.scrollWidth 均为 390 |
| 色彩与状态 | 保留深色侧栏、蓝色按钮、成功/待确认状态色；没有新增不一致主题 |
| 图像与图标 | 沿用原 Logo、头像和 Element 图标，比例清晰，无替代手绘资产 |
| 文案与操作 | 删除支付/邮件“未接通”旧提示；下单明确说明 0.01 元及确认；XLSX 限额、更新规则和 UTC 调度有清晰说明 |

## 发现、修复和复核历史

- [P2，已修复] 系统配置顶部仍称支付/邮件未接通。改为说明使用已保存配置，重新截图和旧版界面对照确认；表单布局保持一致。
- [P2，已修复] 原缓存图表在 390px 下外置长标签被裁切。小屏改用可翻页的紧凑图例，桌面保留原标注；`cache-mobile-before.png` 与 `cache-mobile.png` 在同一输入中对照，裁切问题已消除。
- 导入/任务弹窗宽度与任务表单分列已适配小屏。数据库字段与图表单位核对时修正任务结束时间映射、Redis 内存数值换算及空数据库 0 显示。
- 初次手机截屏赶上弹窗动画，另一次截到展开的侧栏；这些属于状态不一致，已使用真实关闭侧栏操作并等待过渡后重新捕获，不作为产品缺陷或最终证据。
- 浏览器脚本的同名输入框、按钮文案定位及 SSH 预览隧道问题已修正；不计为产品修复。

## 交互与范围

已实际操作登录、刷新订单、取消下单确认、新增暂停任务、执行一次、查看任务日志、Redis 两个页面、打开导入和下载 XLSX 模板。浏览器未观察到 pageerror、console error 或失败 API。真实工作簿写入、支付签名/到账和 SMTP 投递由 17 组协议/API 回归验证，不把打开页面当作真实商户付款或外部邮件送达。

没有待修复的 P0/P1/P2；没有扩展审计所有上游未启用页面。Redis 信息表格保留容器内横向滚动，应用缓存列表在手机上纵向排列。

final result: passed
