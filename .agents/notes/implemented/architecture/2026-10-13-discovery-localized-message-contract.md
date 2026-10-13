# Agent Note: 发现/导入边界的 LocalizedMessage 契约

Status: implemented

## Problem

本机配置导入面板（设置 → 导入/导出 → 本机配置）渲染的后端字符串全部是裸中文：`ClaudeImportProposal.basis` / `CodexImportProposal.basis`（来源描述）、`DiscoveredState::Ok.warnings`（就绪警告）、`ReadError.message`、以及 `codex.rs` 导入失败后经 `discover()` 压入警告的约 20 条错误。这些字符串以 `String` 跨过 Tauri 边界，英文界面用户看到的仍是中文。同一模块的适配器、切换执行器与项目场景（`adapter/*/preview.rs`、`asb-switch/src/executor/*`、`codex_project_plans.rs`）早已采用 `LocalizedMessage { key, params, text }` 契约，discovery 是唯一漏网的显示字符串生产者。另外 `CodexImportProposal.warnings`（模型目录逐条警告、缺密钥提示）从未被渲染器读取，是死线面。

## Decision

discovery 边界统一交给 `LocalizedMessage`：凡是**应用自著**的显示句都携带渲染器翻译坐标（key+params），`text` 保留中文兜底；**外部诊断**（`ParseError.message` 为解析器自身输出）按 CommandError 既有哲学保持原文，不伪装成可翻译键。

- `crates/asb-core/src/discovery/report.rs`：`DiscoveredState::Ok.warnings: Vec<LocalizedMessage>`、`ReadError.message: LocalizedMessage`（`ParseError.message` 维持 `String`）；`CodexImportProposal.basis` / `ClaudeImportProposal.basis` 改为 `LocalizedMessage`；**删除 `CodexImportProposal.warnings` 字段**（零渲染方，连带删除 codex.rs 的 catalog/auth 警告产出与 `warnings` 形参）；`CodexImportProposal` 的 `Eq` derive 随 `LocalizedMessage`（仅 `PartialEq`）一并移除。
- `inspect.rs` / `import.rs` / `codex.rs`：全部警告、basis、错误改 `LocalizedMessage::new(key, params, text)`，键命名 `importDiscovery.discovery.*`（22 条）与 `importDiscovery.basis.*`（5 条）；`discover()` 与 `proposal()` 的注入读函数错误类型随之改为 `LocalizedMessage`。
- `src-tauri/src/commands/error.rs`：`CommandError.message_key` 由 `Option<&'static str>` 放宽为 `Option<String>`（键可能来自 core 的运行时字符串），新增 `from_localized(code, LocalizedMessage)` 构造器（text/params 照常 scrub）；`commands/discovery.rs` 与 `asb-discover` CLI 的读失败改走 `importDiscovery.discovery.readFailed`。
- 前端 `src/api/discovery.ts` 类型同步；`LocalConfigImport` 的 basis / warnings / 读错误统一经 `localizedMessageText(message, t)` 渲染（未知键回落 `text`，消息永不为空）。
- i18n：`importDiscovery.ts` 新增 27 键，zh 与 Rust 兜底文本逐字一致。

**无过渡**：发现缓存（`DiscoveryReport` 落盘）序列化格式随契约一次性变更，旧缓存反序列化失败即被上层当作「尚无扫描」忽略——`useDiscovery` 对缓存不可读的静默回落本就存在，无需兼容分支。

## Alternatives considered

- **在命令层逐条 `CommandError::keyed` 翻译**：最强理由是改动只在 src-tauri。否决：错误在 core 内部就已分叉为「警告」与「失败」两条渲染路径，命令层无法还原 key；且 warnings 走的是数据面不是错误面，命令层根本拦不到。
- **同时把 CcSwitch / SQL 导入面的 `warnings`/`reason` 裸中文一并迁移**：最强理由是同一缺陷类。否决：那是约 40 条串加 failover 测试面的另一个边界，与本决策不属于同一契约单元；混入会扩大回归面且无法在本轮验证。作为已知缺口记录于此，不静默。

## Consequences

本机配置导入面板在任意界面语言下文案一致；同一句警告在「就绪检查」与「导入失败」两条路径下共享同一键。代价：`CodexImportProposal.warnings` 携带的信息（如「模型目录未包含主模型，已补入」）彻底消失——它们此前就没有任何渲染方，属于删除死线面而非损失；缺密钥事实仍由保存时校验兜底。发现缓存在升级后首轮失效一次（等价于未扫描）。验证：自写解析器交叉核对 27 个 Rust 键与 i18n 模板的存在性与中文逐字一致（problems: NONE）；`grep` 确认 discovery 模块无残留裸 `Err("…")` / `warnings` 引用；按项目规则未运行 `cargo check` / `typecheck`，编译一致性靠逐文件人工核对（发现并修复了一处批量替换静默失败导致的签名/函数体类型不一致）。
