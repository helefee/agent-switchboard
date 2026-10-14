# Agent Note: 归档笔记封印要求 LF 检出（.gitattributes 持有行尾契约）

Status: implemented

## Problem

远程 CI 的 Windows 任务在「共用源码门禁」失败：`verify-archived-agent-notes` 报三个归档笔记 seal mismatch，而同一提交在 macOS、Linux 与本机（工作树全为 LF）全部通过。根因：封印哈希（`archive-agent-note.ts` 与 `verify-archived-agent-notes.ts` 的 `sealOf`）对**原始字节**计算，封印语义是"字节冻结"；GitHub Windows runner 检出时按 `autocrlf` 把笔记写成 CRLF，字节必然与封印不符。本地无法发现，因为本机工作树恰好是 LF；在全新克隆（CRLF 检出）中完整复现。

## Decision

行尾契约由检出层持有：新增 `.agents/notes/.gitattributes`，对该目录下全部 `.md` 与 `manifest.json` 声明 `text eol=lf`，任何平台检出都得到与封印一致的 LF 字节。封印哈希保持原始字节语义不变——冻结即字节冻结，`verify-archived` 的 append-only 不变量不受影响；不在校验脚本里做行尾归一化去迁就 CRLF（那会改变封印含义并触发对 HEAD 基线的 append-only 冲突）。

## Alternatives considered

- `sealOf` 改为行尾归一化哈希：最强理由是对任何检出形态都稳健。否决：封印含义从"字节冻结"弱化为"规范化内容冻结"，且需要重写全部既有 seal，直接触发 `verify-archived` 对 HEAD 基线的 append-only 检查，要么加豁免要么重立基线，都是为绕过自己引入的特例。
- 只在 runner 上设置 `autocrlf=false`：修的是这一台机器的环境而不是仓库契约，任何新克隆（Windows 默认 `autocrlf=true`）都会再次踩中；否决。

## Consequences

- 任何平台、任何克隆方式检出的 `.agents/notes/` 都是 LF 字节，封印校验与平台无关。
- 已有工作树若曾以 CRLF 检出笔记，需要重新检出（`rm` 后 `git checkout -- .agents/notes`）才能对齐；仓库内现有工作树本为 LF，无需处理。
- 手工把 CRLF 副本拷入归档目录仍会被 seal 校验拒绝——这是冻结语义的正确行为，不是缺陷。

## Testing

- 在全新克隆（CRLF 检出）中复现了远程失败（三个 seal mismatch，notes 步骤 exit 1）。
- 加入 `.gitattributes` 后重新克隆：检出为 LF，`npm run ci` 完整门禁（notes → 前端测试 → Rust 测试 → 前端构建）在 Windows 上通过。
- `npm run verify-notes` 通过。
