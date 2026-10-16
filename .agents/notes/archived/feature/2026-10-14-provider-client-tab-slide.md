# Agent Note: 供应商切换页客户端页签方向性滑入

Status: implemented
Archived: 2026-10-16

## Problem

供应商切换页的 Codex / Claude 客户端页签（`ClientPicker`）切换时是瞬时硬切：`CodexProvidersPage` / `ProvidersPage` 在 inactive 时返回 `null`，整块 `.asb-provider-workspace` DOM 销毁重建。只做入场动画时，旧画面瞬间消失、新画面从透明淡入，中间出现空帧——表现为闪烁；且整页（含与另一端完全一致的页头和概览卡）一起滑动既多余又晃眼。用户要求：概览卡不动，仅下方档案列表做左右横移。

## Decision

**动画作用域只有 shell 内的 `.asb-provider-list`（档案列表）**——页头与概览连接卡在两端数据、布局完全一致，瞬时更替不可见也不该动。两端列表读作一条横向轨道，进入与退场同步互补：

- **方向**：`ProvidersWorkspace` 的 `changeApp` 拦截本页 ClientPicker 点选，按页签序 `CLIENT_ROUTE_ORDER`（codex 左、claude 右）算出 `"forward" | "back"` 存入 state，经 `routeDirection` prop 传入 `ProviderWorkspaceShell`；其它跨端路径（编辑器内 `newEditorFor`、用量保存后的 `selectApp`、客户端配置页选择器）直接调 `providers.selectApp`，不产生方向。
- **入场**：进入端列表 DOM 由切换重建，挂载即播入场动画——forward 自右、back 自左，位移 `--asb-space-3`（16px）加透明度 0→1，时长 `--asb-motion-fast`、统一缓动、`both` 填充；shell 的头部与概览卡瞬时呈现。
- **退场**：被替换端不再立即返回 `null`——`leavingApp` state（切换时记下旧端）经 `leaving` prop 让页面在 `active || leaving` 时保持列表挂载一个退场周期；shell 加 `asb-route-out-left/right` 类：壳体绝对定位（`inset: 0 0 auto`）叠于进入端之上、`pointer-events: none`、`aria-hidden`，其头部与概览卡子元素 `visibility: hidden`（visibility 保留布局使两端列表 y 对齐，内容由下方进入端的同款接替），仅列表以 `asb-client-route-out-*` 向自身页签方位、同距同向滑出淡出（`forwards` 填充）。定位上下文由 `.asb-page-stack:has(.asb-route-out-*)` 仅在退场存在时提供。两端列表同距同向 → 视觉上是同一条轨道整体平移，无空帧。
- **结算**：动画挂在列表子元素上，shell 根节点 `animationend` 按 `animationName` 前缀（`asb-client-route-`）过滤后回调 `onRouteAnimationEnd`，工作区把方向与退场端一并清空；一个 400ms 兜底计时器只防动画中途卸载导致的事件丢失。reduced motion 时长归零，入场退场同帧完成，无闪烁。

共享关键帧由 `src/styles/base/client-route-motion.css` 唯一拥有，[客户端配置页切换](2026-10-16-client-configuration-tab-slide.md)也消费这一视觉规则；两个工作区分别拥有各自的方向与退场生命周期。

## Alternatives considered

- 只保留入场动画（本特性的前一版）：实现最小，但旧画面瞬消到新画面淡入之间存在空帧，正是用户报告的闪烁，已推翻。
- 整个 shell（含头部与概览卡）一起交叉滑动（本特性的上一版）：无闪烁了，但两端完全一致的头部与概览卡跟着滑，视觉上"内容没变的部分也在动"，用户明确要求概览卡不动，已推翻。
- 两端常驻、`hidden` 切换 + transform 过渡：能做真滑动，但 inactive 页面必须保持挂载，两客户端的轮询卡常驻 DOM，违背「inactive 只卸 DOM 不卸组件、仅可见列表挂轮询卡」的既有语义，放弃。
- 切换瞬间对旧工作区做 DOM 快照克隆叠层退场：不要求页面改渲染条件，但克隆是冻结帧（无交互无语义）、克隆时机要抢在 React 提交之前，管理层复杂；保持真实 DOM 挂载一个周期行为等价且更简单，放弃。

## Consequences

- 收益：切换为无空帧的连续横移，且只有数据真正变化的列表在动；头部与概览卡瞬时更替不可见；方向与退场都是显式类型化契约，一次性消耗杜绝任何重播；动画仍只锚定本页页签点选。
- 代价：退场周期（约 180ms，兜底 400ms）内两端 DOM 并存——退场端不响应指针且对辅助技术隐藏，数据层面 inactive 效果照常（如清空切换候选），不可见；两端概览卡高度一致是列表 y 对齐的前提，若未来两端卡片高度解耦，需改为按列表实际位置定位退场层。
- 边界：180ms 内反向再切换属于退化竞速——方向与退场端被覆盖，退场类换入场类产生一次跳切，可接受；结算按 `animationName` 前缀过滤，若新增 `asb-client-route-` 前缀的无关动画会误触结算，命名前缀为客户端切换专用；动画期间列表持有 transform，内部 `position: fixed` 后代的包含块被临时改写，列表内当前无 fixed 后代。
