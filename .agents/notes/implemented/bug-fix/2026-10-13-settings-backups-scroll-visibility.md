# Agent Note: 设置页滚动被备份高度接管锁死

Status: implemented

## Problem

设置页除「备份恢复」外的所有分区都无法滚动，超出视口的内容被直接裁掉。根因在 tables-backups.css 的「本地备份页签持有工作区高度」规则链：判定条件写的是 `.asb-backup-local:not([hidden])`，但该元素的 `hidden` 属性归属本地/云端子页签，设置分区与页级的隐藏分别在外层 tabpanel 和页级 wrapper 上。`:not([hidden])` 只检查元素自身属性，`:has()` 只看 DOM 不看有效可见性，于是「设置分区常驻挂载、靠祖先 `hidden` 隐藏」（[桌面工作区](../architecture/2026-10-13-desktop-workspace-uiux.md) 的保留挂载契约）落地后，只要进过设置页且本地子页签激活，`.asb-main` 就在所有分区被锁成 `overflow: hidden`。同文件 118–138 行的高度链规则带有同样的跨层判定，属于同一处隐患。

## Decision

备份高度接管规则链统一以有效可见性判定生效：`.asb-backup-local:not([hidden], [hidden] *)`——自身或任一祖先带 `hidden` 即不接管。`.asb-main` 的接管规则同时删除冗余的 `> .asb-page-stack:not([hidden])` 前置：一个未被任何祖先隐藏的后代已蕴含页级 wrapper 未隐藏，结构前置是第二份事实来源。判定语义从此与挂载结构解耦：DOM 深度、哪一层持有 `hidden` 再变化，都不影响「本地备份页签实际显示时才接管工作区高度」这条契约。

其余带 `:not([hidden])` 的结构性 `:has` 规则（forms.css 的 editor-route、extension-library.css 的 ext、sessions.css 的 sessions）被测元素自持 `hidden`，语义精确，本次审计后不改。挂载契约本身（保留挂载、按 `active` 停止副作用）归桌面工作区笔记所有，维持不变。

## Alternatives considered

- **把分区可见性前传进 `.asb-backup-local` 自身的 `hidden`**（组件已接收 `active` prop）：只改 TSX 不改 CSS，但同一事实获得两个来源——祖先 `hidden` 与自身 `hidden` 各写一遍，未来任何新的嵌套隐藏层都得记得前传，只是把结构性假设换了一层。否。
- **给全部兄弟 `:has` 规则统一套有效可见性模式**：一致性更好，但那些规则被测元素本就自持 `hidden`，改动无行为收益，属于与本次缺陷无关的样式扰动。否，仅留审计结论。

## Consequences

设置页「偏好设置 / 客户端工具 / 本机网关 / 诊断 / 关于」恢复页面级滚动；「备份恢复 + 本地备份」仍独占工作区高度、历史表格内部滚动；「加密云端备份」子页签恢复页面滚动。判定引入 `:not()` 复杂选择器（Chromium 88+），支持面窄于规则链已在使用的 `:has()`（Chromium 105+），无新增运行环境要求。代价是选择器比单一 `:not([hidden])` 长，语义集中在 tables-backups.css 一处，无第二份副本。

## Verification

`npm run dev:frontend` + Playwright 注入 `__TAURI_INTERNALS__` 垫片渲染真实组件（设置加载失败错误态，挂载结构真实），900×420 视口实测：偏好设置 / 客户端工具 / 诊断的 `.asb-main` 为 `overflow-y: auto` 且 `scrollTop` 实际位移 81 / 200 / 198；备份恢复 + 本地备份为 `hidden`（接管生效）；+ 加密云端备份为 `auto` 且可滚；离开设置页为 `auto`。stash 前后对照：无修复时偏好设置 `overflow-y: hidden`、位移 0 而内容溢出，bug 复现。`npm run verify-notes` 通过。按项目规则未运行编译、类型检查与自动化测试。
