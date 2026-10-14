# Agent Note: Anthropic 服务根地址拒绝版本前缀

Status: implemented

## Problem

本地 CI 的供应商请求测试暴露出 `provider.example/v1` 形态的 Anthropic 基址被静默接受，请求路径拼出 `/v1/v1/messages` 双重版本段。endpoint.rs 的服务根契约（模块头与 `anthropic_paths_match_the_client_service_root_contract` 测试）只拒绝以完整请求端点结尾的基址，没拒绝以 `/v1` 结尾的基址；用户把 OpenAI 风格地址填进 Anthropic 协议供应商时，只能在请求失败时才发现。

## Decision

`parse_base` 是供应商 URL 语义的唯一所有者，`validate_base_url`、模型发现与请求构造都经它。Anthropic 协议下，基址路径的末段恰为 `v1`（含尾斜杠形式）时明确拒绝，报「服务地址应填写服务根地址，不要包含 /v1 版本前缀」；不猜测、不自动剥除。非 `v1` 的末尾段（如 `/v1beta`）视为显式代理前缀，照常保留。OpenAI 系协议不受影响——其基址本来就携带服务商要求的 `/v1` 前缀。

收紧 `parse_base` 一处即同时覆盖保存校验（`contracts/codex/validate.rs`、`validate/profile.rs`）、CC Switch 与本地配置导入、Claude 原生路径、供应商诊断和请求准备；各层不另设规则。完整 URL 模式（`is_full_url`）走 `parse_full_url`，不受影响。契约测试 `anthropic_version_prefixed_bases_are_rejected_instead_of_doubled` 固化拒绝与 `/v1beta` 放行两条边界；被旧规则漏过的共享测试夹具按协议拆分基址（Codex 用 `/v1` 前缀，Claude 用服务根）。

## Related decisions

- [供应商诊断契约笔记](2026-10-13-provider-diagnostics-contract.md) 拥有诊断消费端点解析的方式，本次只收紧其消费的底层规则，消费结构不变。

## Alternatives considered

- 自动剥除末尾 `/v1` 再拼接：用户零操作，但属于猜测——与 endpoint.rs 明示的「不猜测版本」立场冲突，且真实以 `/v1` 结尾的代理前缀会被误改。
- 在各消费层（保存、导入、诊断）分别校验：看似层层设防，实际把同一规则复制成多份，正是单一所有者要消除的形态；故只在 `parse_base` 收紧。

## Consequences

- 收益：错误形状的 Anthropic 地址在保存/导入/诊断/请求任一入口都被同一规则提前拒绝，并给出可操作的中文原因；不再产生双重版本路径的静默坏请求。
- 代价：已保存了 `/v1` 结尾 Anthropic 基址的旧档案在下次校验时会报错，需要用户改为服务根地址——这是有意的 fail-closed，不提供兼容读取。修复路径后续开了唯一例外（备份 + 报告下的自动剥除/丢弃），见 [修复流程解锁版本前缀死锁](2026-10-14-repair-unlocks-anthropic-version-prefix-deadlock.md)。
- 验证：`cargo test --locked --workspace` 583 项全过（含新增契约测试与按协议拆分的夹具）。

## Related decisions

- [供应商诊断契约笔记](2026-10-13-provider-diagnostics-contract.md) 拥有诊断消费端点解析的方式，本次只收紧其消费的底层规则，消费结构不变。
- [修复流程解锁版本前缀死锁](2026-10-14-repair-unlocks-anthropic-version-prefix-deadlock.md) 拥有 repair 路径对这类基址的自动改写规则；保存/导入/诊断/请求仍按本篇拒绝。
