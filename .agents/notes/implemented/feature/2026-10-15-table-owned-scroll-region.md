# Agent Note: 表格滚动面与展开行收归 Table 组件

Status: implemented

## Problem

用量监控的 7 列表格（数字列 nowrap 不可收缩）在窄窗口被右缘裁切，且 ink 滚动条静止时隐藏（[滚动条静止时自动隐藏](2026-10-14-autohide-ink-scrollbars.md)），滚动可供性完全丢失。根因是表格契约被拆成模块级并行实现：横向溢出由各模块自理——运行日志、用量、降智雷达各维护一个私有 `-table-wrap`（overflow、gutter、分页相邻选择器三处重复），网关最近请求表、扩展管理 9 处 `asb-ext-table-scroll`、重置面板日额度再各写一份；而其余十余处 `Table` 调用点（配额窗口、用量读数、导入、诊断等）没有任何滚动。备份历史更因「展开差异需要占满整行的第二行」整表绕开共享渲染器、自带一份裸 `<table>` 镜像标记，连带迫使分页边距选择器保留 `.asb-table +` 分支。

## Decision

- `Table` 组件标记自带包裹层 `.asb-table-scroll`（`tabIndex={0}`），是表格滚动面的唯一所有者：`overflow: auto` + `scrollbar-gutter: stable`（延续 DESIGN.md「滚动面预留稳定滚动条槽位」契约），键盘可聚焦、可平移（延续原扩展管理滚动壳的无条件 tabIndex 先例）。
- `Table` 契约补上可选行内展开行 `renderExpansion`：返回节点即在该行下方发出占满整行的 `.asb-table-expansion-row`，hover 保持中性（吸收原 `.asb-backup-diff-row` 规则）。备份历史迁移到共享 `Table` + `renderExpansion`，裸 `<table>` 镜像与 `Fragment`/`cx` 复制路径删除；分页边距只保留 `.asb-table-scroll + .asb-pagination` 一条路径。全库数据表格只剩 Table.tsx 一处 `<table>` 标记（DatePicker 日历网格是其原语自有语法，DESIGN.md §90，不属数据表格契约）。
- 删除全部模块级滚动壳及其规则：四个 `-table-wrap`/网关 div、扩展管理 9 处 `asb-ext-table-scroll`（CSS 规则整条删除）、重置面板 `asb-codex-reset-daily-data`。检测结果的 aria-live/aria-busy 播报改为无样式的纯语义 div（不承担滚动与布局）；日额度 18rem 高度裁剪迁到 `.asb-table-scroll:has(> .asb-codex-reset-daily-table)`，滚动本身归包裹层。用量的 `flex: 1 / min-height: 0` 与 sticky 表头分别迁到 `.asb-model-usage .asb-table-scroll` 与 `.asb-model-usage-table th`；网关类名移到表格元素上改写为复合选择器。
- 模块级 `min-width`（runtime-log 680px、gateway 56rem、扩展管理 600px、用量/雷达 max-content）原样保留在表格元素上：宽度约束是模块事实，滚动与展开行是组件契约，两者分层。
- DESIGN.md §8 `Table` 条款与 tables-backups.css 头注释同批更新，声明组件包裹层是唯一表格滚动面、展开行由组件标记发出。

## Related notes

- [滚动条静止时自动隐藏](2026-10-14-autohide-ink-scrollbars.md)：拥有滚动条显隐视觉；本篇包裹层的 stable gutter 是「显隐永不 reflow」的前提，无所有权重叠。
- [备份历史改随页面滚动](2026-10-14-backup-history-page-scroll.md)：页面级滚动的决定不变；其「模型用量等仍在自己滚动区内 sticky」的表述继续成立，只是滚动区从模块自建 wrap 变为组件包裹层，且该表的 DOM 由此回归共享契约。
- 当场审计：扩展管理与重置面板的滚动壳原先部分重叠（各带播报或高度语义），本篇已整体吸收——语义剥离到纯 aria div / `:has` 高度规则，壳与 overflow 规则全部删除，无残留双轨。

## Alternatives considered

- **全局 `.asb-table { min-width: max-content }`**——最强论据：所有表格彻底消灭列压扁，一处规则达成「左右滚动或自适应显示」。否：它会推翻 DESIGN.md 拥有的「长文本在列内换行」产品事实（诊断结论、扩展描述、备份路径等将停止换行、整表横滚）。滚动只应发生在「列已不可再收缩」时（数字列 nowrap 天然触发），文本列保留换行能力。
- **保留扩展/重置外层壳承载播报与高度语义**——最强论据：改动面最小，行为零风险。否（用户定调：底层无兼容、无过渡、无技术债）：它们与组件包裹层构成双滚动层与双 tab 停靠点，`overflow-x` 已成死规则；语义与滚动拆开后各归其位（纯 aria div、`:has` 高度规则），壳没有存在理由。
- **备份历史维持裸表格**——最强论据：展开行是共享渲染器不发出的结构，镜像标记是既成事实。否：唯一的 DOM 契约里不该有第二份表格标记；正确动作是把展开行纳入契约（`renderExpansion`），而不是为备份养永久旁路。
- **只改用量页**——改动最小。否（用户当场纠正）：这是全局表格契约的缺陷而非单页问题，十余处无滚动的调用点是同源问题。

## Consequences

收益：全库表格单一实现——滚动、稳定 gutter、键盘平移、行内展开行一处拥有；四处 `-table-wrap`、9 处扩展壳、1 处重置壳、备份裸表格及其 `.asb-table +` 分页分支全部消失，无兼容路径与死规则。代价：每张表常驻稳定滚动条槽（约 10px 墨线宽度的右缘预留，与用量、日志页原状一致）；不溢出的表格也多一个键盘停靠点；检测结果外层从可聚焦滚动面变为普通 div，键盘平移由内层包裹层承接；日志页加载骨架不再单独预留 gutter（骨架与表格同宽，出现时无位移）。

## Verification

按项目规则未运行编译、类型检查与自动化测试；静态核对：`rg "asb-ext-table-scroll|asb-codex-reset-daily-data|asb-backup-diff-row|*-table-wrap" src/` 零命中；全库 `<table>` 标记仅 Table.tsx 与 DatePicker.tsx（日历原语）两处；`.asb-table-scroll` 规则 5 处 = 1 个全局定义 + 4 个模块布局参与（用量高度、分页边距、趋势块边距、日额度高度），无 overflow 重复；`npm run verify-notes` 通过。宽表平移、备份差异展开、扩展检测播报、日额度高度裁剪等实际观感建议 dev 下抽查。
