# Agent Note: 滚动条静止时自动隐藏

Status: implemented

## Problem

ink-line 滚动条（root.css 全局所有者块）此前常驻可见：每个滚动面右侧固定一条 10px 墨线滑块，即使用户没有在滚动。多面板桌面工作区里同时存在多条常驻滑块，视觉噪音与 0.3.3 毛玻璃外壳追求的安静 chrome 相抵触。目标：滑块静止时隐藏，滚动进行中或指针落在滑块上时出现，且不引起布局晃动。

## Decision

- `src/styles/base/root.css` 仍是滚动条视觉的唯一所有者：滑块默认 `background-color: transparent`，在任一状态恢复墨色——滚动面带 `data-scrolling`、滑块 `:hover`、滑块 `:active`（同特异性靠声明顺序分层：基础 → 滚动中 → 悬停 → 按压）。各面的 `scrollbar-gutter: stable` 不变，显隐永不 reflow。
- `data-scrolling` 由 `useScrollActivityMarker`（src/app/global-effects.ts，App.tsx 挂载）唯一拥有：window 级 capture + passive 的 `scroll` 监听，`event.target` 即滚动面（document 滚动映射到 `scrollingElement`），写入 `data-scrolling` 并在 900ms（`SCROLL_IDLE_MS`）无滚动后移除。`scroll` 事件不冒泡但 capture 阶段可达所有滚动面，一个监听覆盖全部界面，无需逐组件接线。
- Chromium 不支持 `::-webkit-scrollbar-*` 的 transition，显隐为瞬时切换，不引入动画。

## Related notes

当场审计（rg scrollbar/scroll，排除 archived）：命中 [备份历史页内滚动](2026-10-14-backup-history-page-scroll.md) 与 [毛玻璃外壳](../architecture/2026-10-14-unified-native-glass-shell.md)，分别拥有备份列表的滚动位置与窗口 chrome 材质，均不拥有滚动条视觉契约，无重叠。

## Alternatives considered

- **WebView2/Chromium 原生覆盖滚动条（`--enable-features=OverlayScrollbar`，经 additionalBrowserArgs）** — 最强论据：零 CSS 零 JS，系统级自动隐藏行为最地道。被否：会把 DESIGN.md 拥有的 ink-line 契约替换成系统默认外观；feature flag 依赖 WebView2 版本，additionalBrowserArgs 作用于所有窗口，行为不可控。
- **纯 CSS hover 显隐（`*:hover::-webkit-scrollbar-thumb`）** — 最强论据：零 JS，指针进入滚动面即显示。被否：指针悬停在内容上阅读时滑块也常驻显示，与「不操作时隐藏」不符；纯键盘滚动（指针不在面上）时滑块永不出现。
- **逐组件 scroll 接线（每个滚动面各挂 hook）** — 最强论据：状态最局部、无全局监听。被否：滚动面遍布工作区、预览、表格、弹窗、文本域，逐个接线重复且易漏新面；window capture 监听一处拥有。

## Consequences

- **收益**：静止界面不再有常驻滑块噪音；gutter 稳定预留，显隐零布局抖动；键盘或程序化滚动（无指针参与）同样触发显示；拖动中滑块经 `:hover`/`:active` 保持可见，拖完指针停在滑块上也不消失。
- **代价与已知上限**：每个 scroll 事件做一次属性写入与定时器维护（passive 监听，开销可忽略）；900ms 宽限期是拍板值，过短会闪烁、过长近似常驻；未来若引入 iframe 内滚动面，window capture 收不到其内部滚动，需另行接线。
- 显隐无过渡动画是平台限制；若必须淡入淡出，需重访「webkit 伪元素」这一选型本身（如迁往 `scrollbar-color`）。

## Verification

- `rg -n "data-scrolling" src/` 仅命中 global-effects.ts（写入方）与 root.css（消费方），两处即契约全体。
- 运行应用人工确认：静止时滑块不可见；滚轮滚动立即出现，停手约 0.9s 后消失；按住滑块拖动全程可见；滚动面布局无位移。
