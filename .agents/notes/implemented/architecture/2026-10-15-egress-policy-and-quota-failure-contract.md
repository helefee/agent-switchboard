# Agent Note: 出口策略可操作契约与官方额度失败原因贯通

Status: implemented

## Problem

官方额度读取 `https://chatgpt.com/backend-api/wham/usage`。在该网络环境下直连被 DNS 污染黑洞（解析到假 IP，TCP 秒连但永远无响应），唯一可行出口是本机代理。出站代理模块自称 egress 契约唯一 owner，但 `save` 是 `#[expect(dead_code)]` 死代码：没有命令、没有 UI，用户无法把应用指向代理；默认 `None` 模式强制 `no_proxy()` 直连，应用在架构上就不可能读到额度。同时 `fetch()` 用 `Err(_)` 把已分类的传输错误整类丢弃，`CodexOfficialQuota` 契约没有失败原因字段，前端只能显示「请重试」；`quota_from_http_response` 把服务器应答异常（429/5xx）和 2xx 响应形状漂移混进同一个 Unavailable，与网络故障不可区分。传输分类枚举（原 `probe::FailureKind`）是 `pub(super)` 内部类型，五类中文文案硬编码在 `failure_message`，游离于 i18n 之外。托管账号路径甚至把传输失败伪装成 HTTP 503 再解析。

## Decision

出口策略仍是单一契约，owner 仍是 `outbound_proxy` 与 `state/outbound-proxy.json`；本批补全其缺失的写路径。`get_outbound_proxy_settings` / `set_outbound_proxy_settings` / `repair_outbound_proxy_settings` 三个命令暴露读取、保存（revision 乐观并发）与一键重置，保存与重置分别记录 `OutboundProxySaved` / `OutboundProxyRepaired` 运行日志动作；`save` 的 dead_code 标记由命令调用方移除，无任何调用方的 `mask_url` 直接删除；校验错误从裸 `String` 改为类型化枚举，经 `CommandError::localized` 携带 `errors.outboundProxy.*` message key。设置 UI 在应用设置分区新增自含式出口代理卡片（模式三选 + 代理地址 + 保存），读取失败时卡片提供重置入口。重置删除设置文件并回到缺省直连策略，覆盖另一版本写入的旧格式与损坏文件，使版本升级后出口配置不会卡死。默认模式维持直连不变：静默继承环境代理意味着供应商凭据流量可能被环境代理截收，直连是诚实的默认，缺陷在缺失的写路径而非默认值。`outbound-proxy.json` 保持独立文件不并入 `AppSettings`：手动代理 URL 可含凭据，桌面设置是更宽的暴露面，独立文件与独立 revision 是既有正确边界。System 模式的已知边界保持现状：代理客户端在系统内开关时 revision 不变，缓存客户端不热感知，需在应用内再保存或重启生效；写进卡片说明，不加侦听机制。

传输分类上移为跨边界契约：`asb-core::contracts` 新增 `CodexTransportClass`（dns/timeout/connect/tls/other），`probe::FailureKind` 删除并由它取代；`http_request`/`http_get` 及两个 `_with_options` 变体的错误类型从裸 `String` 改为 `TransportFailure { class }`（实现 `Display` 与 `From<TransportFailure> for String`， incidental 调用方的 `?` 传播与既有用户可见文案不变）。`ProbeResult.error: Option<String>` 改为 `error_class: Option<CodexTransportClass>`，探测结果文案归 i18n。`failure_message` 后端文案删除，额度与探测两个表面的文本 owner 统一为 i18n（`errors.transport.*`）。

`CodexOfficialQuota` 新增 `failure: Option<CodexQuotaReadFailure>`，不变量为：发起过网络请求且失败 ⇒ 必有值。`CodexQuotaReadFailure` 是平铺结构（`kind` + `transportClass` + `httpStatus`），`kind ∈ transport/httpStatus/invalidResponse`，两个载荷字段按 kind 互斥由构造函数保证；`Available`/`SignInRequired` 必无 failure（前者成功、后者未尝试）。`quota_from_http_response` 三分：非 2xx 且非 401/403 → `httpStatus`，401/403 → `ReauthenticationRequired` + `httpStatus`，2xx 但无有效窗口 → `invalidResponse`。`fetch` 的传输错误 → `transport(class)`，不再丢弃。托管账号路径删除伪装 503 的做法，直接携带真实传输分类。`retain_last_success` 语义不变：窗口与 `at` 保留自上次成功，`status`/`failure`/`stale` 取自本次。TS 镜像（`src/api/usage.ts`、`src/api/providers.ts`）同批更新；`clientConfig.quota.errorUnavailable` 键删除，不可用渲染改按 failure 分类取 `clientConfig.quota.failure.*` 文案。

明确排除：额度读取不加超时重试（probe 的重试为延迟分级服务，后台调度器自会重跑）；不做 PAC、不分域路由；不改默认模式；`codex_auth::quota` 托管路径自身的 warning 通道保持原契约，仅修复其失败分类来源。

## Alternatives considered

- 默认改为跟随系统代理：本机实测系统代理关闭（`ProxyEnable=0`）时 System 模式仍直连，修不了本次故障；且让凭据流量默认跟随环境代理是安全负债。直连默认 + 可操作的手动模式才是完整契约。
- 把出口代理并入 `AppSettings` 获得单一设置文件与现成 UI：省一个卡片组件，但 `AppSettings` 是桌面行为契约且暴露面更宽，代理 URL 凭据不应进入该面；独立 owner 文件是既有边界，收编它反而制造一个更大的异常面。
- 保留 `http_get` 现签名、另加 `http_get_classified` 给额度用：diff 最小，但同一传输出现两条错误路径，分类 owner 永远分裂，属于持续性技术债，拒绝。
- 失败原因走独立诊断命令而非契约字段：UI 要轮询两个来源拼装状态，`retain_last_success` 的保留窗口与原因会在两处失配；字段随读数走是唯一不失真的位置。
- 给额度读取加超时重试：与 probe 的重试动机不同，后台调度与手动刷新都会再次发起，重试只加倍最坏持锁时间。

## Related decisions

[查询结果与额度状态分离](../bug-fix/2026-10-13-usage-availability-contract.md)确立官方订阅额度是独立契约、不进供应商脚本链；本决策只扩展该契约的失败维度，百分比展示与 availability 归一不受影响。[供应商诊断的档案解析与修复写入边界](../bug-fix/2026-10-13-provider-diagnostics-contract.md)拥有诊断表格 UI；`ProbeResult` 的 error 字段改分类枚举只影响单次探测结果呈现，表格结构不变。其余命中「额度/出口」关键词的笔记（托盘告警、文案密度）主题无关。

## Consequences

不可达出口环境可以在应用内修复：手动模式填 `http://127.0.0.1:7890`（HTTP CONNECT 与 socks5h 都由代理解析域名，绕开 DNS 污染；纯 socks5 仍本地解析，污染环境下依旧失败——失败分类现在能区分并指出）。任何一次额度失败都显示具体原因（域名解析失败/连接超时/连接被拒绝/TLS 握手失败/网络请求失败/HTTP 状态/响应无效），不再只有「请重试」。设置文件在版本升级后无法解析时，卡片的一键重置删除文件回到缺省直连并刷新进程快照，出口配置不再卡死；重置入口只在读取失败后出现，有效设置到不了这条路径。本次契约的其余落盘数据不受升级影响：usage-history 与重置基线都存自有扁平结构、不序列化 `CodexOfficialQuota`，`failure` 字段只存在于进程内与命令边界。代价：`http_request` 错误类型变更波及 9 个调用方（多数经 `From<String>` 零改动）；探测结果前端消费方从字符串改为分类渲染；i18n 新增三组键、删除一个键；`outbound-proxy.json` 可能含代理凭据，它只落本机状态目录（云备份仅覆盖供应商档案库，已核实不波及）。System 模式的热感知边界如上，不新增机制。

## Verification

传输分类构造、quota_from_http_response 三分与 retain_last_success 的 failure 传递以源码内联测试覆盖；前端静态复核确认 OfficialQuotaSummary、ProbeFeedback 只消费分类键、`errorUnavailable` 与 `ProbeResult.error` 全局无残留引用。按项目规则未运行编译、类型检查与自动化测试；`npm run verify-notes` 校验笔记结构。真实网络读取与代理写入未验收，需在桌面运行中确认：停代理刷新额度显示具体原因，配置手动代理后恢复 Available；手工写入坏 JSON 的 `outbound-proxy.json` 后卡片读取报错并可一键重置回直连。
