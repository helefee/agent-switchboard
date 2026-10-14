# Agent Note: 备份历史改随页面滚动

Status: implemented

## Problem

「备份恢复 → 本地备份」页签持有工作区高度：tables-backups.css 用一条 `:has()` 接管规则链把 `.asb-main` 锁成 `overflow: hidden`，沿 page-stack → settings-content → tabpanel → panel → backup-local 拉出 flex 高度链，让历史表格在自己容器内滚动、表头 sticky。这条链是全库判定最脆的滚动契约——分区常驻挂载、靠祖先 `hidden` 隐藏的挂载结构已经迫使它修过一次有效可见性缺陷（[被取代的旧篇](../../archived/bug-fix/2026-10-13-settings-backups-scroll-visibility.md)，其 `:not([hidden], [hidden] *)` 判定随规则链一起消亡）；而诊断日志、用量等设置分区都是页面级滚动，备份历史是设置页里唯一的表内滚动特例。用户本次改判：历史表格不再独占高度，随设置页面一起滚动。

## Decision

本地备份页签回归流式页面布局，不再接管工作区高度：tables-backups.css 的整条接管链（`.asb-main` 的 overflow 锁、page-stack/settings-content/tabpanel/panel 的高度链、`.asb-backup-local` 的 flex 拉伸）全部删除；`.asb-backups` 不再是滚动容器，flex/min-height/overflow 与 sticky 表头一并移除，工具栏与危险行的 `flex: none` 随 flex 父级消亡。页面滚动归 `.asb-main` 的默认 `overflow-y: auto`。TSX 结构不变：`.asb-backups` 保留为包裹表格与分页的普通块级容器。备份表因此退出「受限滚动页」名单，DESIGN.md 的铬件契约与 workspace-main.css 注释同步去除「备份表」。

## Alternatives considered

- **保留表内滚动，仅让表头在页面滚动下继续 sticky**（top 对齐标题栏下缘）：最强的理由是长历史滚动时列名驻留可见。否——诊断日志、用量等分区都是页面滚动且无常驻表头，备份表要维持 sticky 就得保留整条高度接管链与有效可见性判定，为一个模块养着全库最脆的滚动契约；且分页控件会被高度链推出可视区，而历史每页 20 条本就有限。
- **保留接管高度链、只删 sticky 表头**：改动最小的折中。否——接管的本质是「表格内滚动」，不删高度链滚动模型没变，等于保留缺陷源只摘症状。

## Consequences

收益：删去约 40 行判定脆弱的 `:has()` 高度链，「本地备份页签是否接管高度」这条契约连同它的可见性判定整体消失，设置页所有分区统一为页面级滚动；长历史、展开的差异行、确认弹层都随页面自然生长，不再依赖隐藏祖先判定。代价：滚动越过首屏后表头与分页控件不再驻留，需滚回顶部翻页——历史按 20 条分页，可接受。云端备份子页签行为不变（本就页面滚动）。模型用量等仍在自己滚动区内 sticky 的表格不受影响，那类模块本就各自持有滚动区。

## Verification

按项目规则未运行编译、类型检查与自动化测试；改动以静态检查核对：CSS 无 `.asb-backups` / `.asb-backup-local` 高度与 overflow 残留、无孤儿 `flex: none`，TSX 未动。`npm run verify-notes` 通过。实际滚动观感（长列表、展开差异行后的页面滚动）建议在 dev 下抽查。
