# Agent Note: 移除 externallyModified 匹配状态契约

Status: implemented
Archived: 2026-10-16

## Problem

`MatchStatus::ExternallyModified`（「配置已被外部修改，与上次切换不一致」）在「有切换记录、内容哈希与记录不符、且无档案命中」时产出。产品没有任何行为依赖这一区分：切换执行器的预览后外部修改拦截（`errors.switch.externalChange`）走独立的预览快照比对，与匹配状态无关；保存前过期检查（`validate.save.staleFile`）亦然。界面上，客户端设置页对该状态早已返回 `null` 不显示，供应商页连接卡横幅被用户要求移除——后端独自报告一个无人消费的状态，属于死契约面。

## Decision

从整个栈一次性移除 `externallyModified` 匹配状态，不留兼容、过渡或回退路径：

- **契约所有者** `crates/asb-core/src/contracts/plan.rs`：删除 `ExternallyModified { at }` 变体；`MatchStatus` 收窄为五个变体。`Unmanaged` 语义放宽为「无任何当前档案匹配内容，无论此前是否由本应用切换」，文档注释同步。
- **唯一产出点** `src-tauri/src/commands/status/report.rs` 的 `classify_match_status`：哈希不匹配且无档案命中的分支归入 `Unmanaged`。
- **前端类型** `src/api/status.ts` 的 `MatchStatus` 联合类型删除该成员；`ConfigStatusPanel` 诊断行、`ClientSettingsPanel` 状态行、`DualRelay` 连接卡三分支删除；tray 测试夹具改用 `Unmanaged`。
- **文案收紧** `clientConfig.match.unmanaged` 由「从未由本应用切换，也不匹配任何档案」改为「当前配置未匹配已存档案」，与新语义及连接卡提示措辞一致；`clientConfig.match.externalBefore/After` 两条删除。

`MatchStatus` 是运行时从文件内容推导的值、不落盘，无持久化兼容问题。

## Alternatives considered

- **仅在界面隐藏、后端照旧报告**（本次改动的前一版方案）：最强理由是保留可诊断性——状态带时间戳，能区分「手改配置」与「从未切换」。否决：应用内没有任何消费方依据该状态改变行为，诊断价值无法兑现；保留一个前端联合类型必须穷举、Rust 契约必须维护的变体，只增加维护面。
- **整体移除连接卡 notes 区**：可一并去掉 `profileChanged` / `unmanaged` 等其余提示，但超出请求范围；其余状态仍承担「尚未应用 / 未匹配档案」的即时反馈职责。

## Consequences

「内容与上次切换记录不符」此后在应用内的可观察表现与「未匹配任何档案」一致；切换与保存的真实写入防护不受影响（独立机制保留）。代价是诊断页失去该状态的专门展示与时间戳——这一信息仅存在于切换历史记录中。运行时防护不变：预览后外部修改仍会被执行器拦截。验证为全库 `rg` 零残留与 `npm run verify-notes`；按项目规则未运行 `cargo check` / `npm run typecheck`，编译层面为人工检查。
