# Agent Note: 供应商切换页客户端页签方向性滑入

Status: implemented

## Problem

供应商切换页的 Codex / Claude 客户端页签（`ClientPicker`）切换时是瞬时硬切：`CodexProvidersPage` / `ProvidersPage` 在 inactive 时返回 `null`，整块 `.asb-provider-workspace` DOM 销毁重建。只做入场动画时，旧画面瞬间消失、新画面从透明淡入，中间出现空帧——表现为闪烁；用户要求的是左右横移式的连续过渡。

## Decision

两端读作一条横向轨道，进入与退场同步互补：

- **方向**：`ProvidersWorkspace` 的 `changeApp` 拦截本页 ClientPicker 点选，按页签序 `CLIENT_ROUTE_ORDER`（codex 左、claude 右）算出 `"forward" | "back"` 存入 state，经 `routeDirection` prop 传入 `ProviderWorkspaceShell`；其它跨端路径（编辑器内 `newEditorFor`、用量保存后的 `selectApp`、客户端配置页选择器）直接调 `providers.selectApp`，不产生方向。
- **入场**：进入端 DOM 由切换重建，挂载即播入场动画——forward 自右、back 自左，位移 `--asb-space-3`（16px）加透明度 0→1，时长 `--asb-motion-fast`、统一缓动、`both` 填充。
- **退场**：被替换端不再立即返回 `null`——`leavingApp` state（切换时记下旧端）经 `leaving` prop 让页面在 `active || leaving` 时保持列表挂载一个退场周期；shell 加 `asb-route-out-left/right` 类（向自身页签方位、同距同向滑出淡出，`forwards` 填充），同时绝对定位（`inset: 0 0 auto`）叠于进入端之上、`pointer-events: none`、`aria-hidden`。定位上下文由 `.asb-page-stack:has(.asb-route-out-*)` 仅在退场存在时提供，不波及其它路由。两端同距同向 → 视觉上是同一条轨道整体平移，无空帧。
- **结算**：shell 根节点 `animationend`（过滤子元素冒泡，仅 `target === currentTarget`）回调 `onRouteAnimationEnd`，工作区把方向与退场端一并清空；一个 400ms 兜底计时器只防动画中途卸载导致的事件丢失。reduced motion 时长归零，入场退场同帧完成，无闪烁。

## Alternatives considered

- 只保留入场动画（本特性的前一版）：实现最小，但旧画面瞬消到新画面淡入之间存在空帧，正是用户报告的闪烁，已推翻。
- 两端常驻、`hidden` 切换 + transform 过渡：能做真滑动，但 inactive 页面必须保持挂载，两客户端的轮询卡常驻 DOM，违背「inactive 只卸 DOM 不卸组件、仅可见列表挂轮询卡」的既有语义，放弃。
- 切换瞬间对旧工作区做 DOM 快照克隆叠层退场：不要求页面改渲染条件，但克隆是冻结帧（无交互无语义）、克隆时机要抢在 React 提交之前，管理层复杂；保持真实 DOM 挂载一个周期行为等价且更简单，放弃。
- 渲染期派生方向（ref 对比）/ `data-app` 交 CSS 选向：方向驻留或重播等副作用此前已逐一推翻，见 git 历史。

## Consequences

- 收益：切换为无空帧的连续横移；方向与退场都是显式类型化契约，一次性消耗杜绝任何重播；动画仍只锚定本页页签点选。
- 代价：退场周期（约 180ms，兜底 400ms）内两端 DOM 并存——退场端不响应指针且对辅助技术隐藏，数据层面 inactive 效果照常（如清空切换候选），不可见；容器高度以进入端为准，退场端超高部分在淡出中短暂溢出。
- 边界：180ms 内反向再切换属于退化竞速——方向与退场端被覆盖，退场类换入场类产生一次跳切，可接受；动画期间 `.asb-provider-workspace` 持有 transform，内部 `position: fixed` 后代的包含块被临时改写，当前 shell 内无 fixed 后代，日后加浮层需留意。
