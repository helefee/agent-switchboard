# Agent Note: 上游协议标签本地化，网关页清除纯英文文案

Status: implemented

## Problem

设置页「本机网关」在中文界面下仍有成片纯英文：上游协议名（OpenAI Responses、Chat Completions、Anthropic Messages、Gemini Native）来自 `src/lib/protocol.ts` 的静态常量 `PROTOCOL_LABELS`，绕过 i18n 目录，不随界面语言变化；出现在拓扑图连线协议芯片、供应商节点协议行、上游协议分布图例与最近请求路径列，供应商请求面板和编辑器网关警告也引用同一常量。图表带延迟行的 `p50`/`p95` 是另一处硬编码英文。

## Decision

协议显示标签的文案事实迁入 i18n，路由点保持在 `protocol.ts`：

- 新域 `src/i18n/messages/protocols.ts`（前缀 `protocol.`）拥有协议名文案，中文主导、括注英文协议名：「OpenAI 响应（Responses）」「对话补全（Chat Completions）」「Anthropic 消息（Messages）」「Gemini 原生（Native）」；en-US 保持原英文名。括注让标签仍能与供应商控制台 / API 文档对照。
- `PROTOCOL_LABELS` 常量删除，替换为 `protocolLabel(t, protocol)`：内部 `Record<UpstreamProtocol, MessageKey>` 映射，调用方在渲染时用当前语言解析。直接消费者为 `GatewayTopology`（客户端原生协议与路由协议）、`GatewayTelemetry`（协议分布与请求路径）、`ProviderRequestPanel` 与 `useProviderConnection`，无兼容读取路径。
- 延迟行标签走 `gateway.latencyMedian` / `gateway.latencyP95`（一半请求 / 95% 的请求；en: Half of requests / 95% of requests），说明对应比例的已完成请求用时不超过该值，统计仍是中位数与 95 分位，不把分位数改叫平均值。
- 保留原文的技术记号：Codex、Claude Code 品牌名，HTTP 状态码、回环地址、端口、路由修订号——它们是技术值不是描述。
- DESIGN.md「本机网关」条目同步协议名本地化事实与括注规则。

## Alternatives considered

- **纯中文协议名（去掉英文括注）**：界面最纯粹；但协议名要与供应商文档里的端点名对照（OpenAI 的 Chat Completions 端点等），完全脱离英文名会提高配置对照成本。否。
- **只在网关页做本地映射、不动全局常量**：改动最小、只影响被点名的页面；但同一协议名会网关页中文、供应商编辑器英文，且诞生第二份协议名映射，违反单一 owner。否。
- **结构化双语组件（中文主标 + 弱化英文小字 span）**：视觉层次最好；但协议分布图例是字符串拼接、编辑器网关警告经 `t()` 参数插值，组件化要维护两套取值形态。否。

拓扑呈现、客户端聚焦与本页易读文案由[2.5D 网关拓扑](2026-10-12-gateway-spatial-topology.md)拥有，复用此处的协议本地化入口。

## Consequences

- 收益：中文界面下网关页不再出现成片纯英文描述；协议名跨页面一致并随语言切换即时更新；协议文案进入键与双语成对的 i18n 目录，新增语言时不再遗漏。
- 代价：双语协议名比原英文占用更多空间；当前拓扑将协议附着节点并允许换行，图例与请求路径同样保留 `overflow-wrap`，不通过缩写隐藏完整协议名。
- 验证：检索确认 `PROTOCOL_LABELS` 无残留、协议直接消费者均使用 `protocolLabel`；网关节点协议与延迟标签在隔离静态预览中可完整读取。按项目验证约定未运行编译、类型检查或自动化测试，真实应用事件与轮询不由静态预览证明。
