# Agent Note: CI 门禁自供给钉死工具链（健康探针 + 校验下载）

Status: implemented

## Problem

本地演练远程 CI 门禁（`npm run ci`）时静默失败：门禁硬性要求 `.node-version`（22.23.2）而系统默认是 Node 24，此前靠手工把官方 zip 解压到 Windows Temp（`%LOCALAPPDATA%\Temp\asb-ci-node`）来满足；该副本随后损坏——npm 的 `lib/commands/` 六十七个文件只剩 `run-script.js`——`npm run ci` 以 exit 1 退出且**零输出**。根因链有三层：钉死工具链没有任何归属方（靠手工准备）；缓存放在 Temp 这类必然被清理/裁剪的位置；无完整性校验、无自愈路径，损坏后故障表现为静默而不是报错。

## Decision

`scripts/pinned-node.mjs` 是钉死工具链的唯一所有者；`scripts/ci.mjs` 的引导阶段保证门禁**永远运行在通过健康探针的钉死工具链上**：

1. 每次运行先探测仓库自有安装 `.tmp/node/node-v<版本>-<平台>-<架构>`。健康探针三层：`node --version` 与 `.node-version` 精确一致；自有 `npm-cli.js --version` 可加载；**安装树与校验过的压缩包清单逐一比对**（`tar -tf` 列出的每个文件都必须存在）——最后一层是关键，npm CLI 懒加载使得单纯烟测无法发现文件被裁剪的树（实测删掉两个命令模块后 `npm --version` 照常通过）。
2. 自有副本存在但不健康 → 打印 `[CI] Pinned toolchain ... is broken; re-provisioning` 后重建；PATH 上版本精确一致的 Node 仅在通过同一探针时使用（远程 runner 的 `setup-node` 走这条快路径，零下载）。
3. 无可用工具链时从 `NODE_DIST_MIRROR`（默认 `https://nodejs.org/dist`）下载压缩包与 `SHASUMS256.txt`，sha256 校验通过后用系统 `tar` 解压到 `.tmp/node/`，解压后再探针一次；archives 缓存保留压缩包，重供给只需重新拉校验清单。
4. 建立后 ci.mjs 以自有工具链 re-exec 自身（`ASB_CI_PINNED=1`，`npm_execpath` 改指自有 npm），notes、前端测试、前端构建的整条 npm 链因此都在钉死工具链上执行，不需要任何手工 PATH 操作。旧错误「CI requires Node X; received Y」（把版本问题推给用户）被删除，不留兼容分支。

同主题审计：与 [本地构建缓存保留边界](2026-09-24-local-build-cache-retention.md) 相邻——`.tmp/node/` 落在该脚本「保留其他 `.tmp` 内容」的边界内，不会被 prune 清掉；无其他笔记持有工具链供给决策。

## Alternatives considered

- 保留手工 Temp zip 的工作流：零代码，但 Temp 必然被清理或杀软裁剪，且已实际造成一次静默失败；否决。
- 要求开发者自装 nvm/volta 并手工对齐版本：契约落在一台台机器的本地状态上，不可审计、不可移植，团队每个新克隆都要重学一遍；否决。
- 无条件使用仓库自有副本、不保留健康 PATH 快路径：远程四个矩阵会各自多下载约 30MB，而 `setup-node` 已提供同版本工具链。快路径与自有副本共用同一探针、钉死同一版本，只是来源不同，不构成第二契约。

## Consequences

- `.tmp/node/` 每版本约 100MB（gitignored，prune 边界保留）；远程 CI 行为不变（快路径），本地任意 Node 启动 `npm run ci` 均可。
- 首次供给与损坏重建各需一次网络；健康状态下完全离线。镜像经 `NODE_DIST_MIRROR` 覆盖，默认官方源。
- 「工具链被裁剪」这一损坏类别在每次运行都被探针拦截并自动修复，门禁失去静默失败路径；所有供给失败（下载失败、校验不符、解压失败、探针不过）都以带原因的错误响亮终止。

## Testing

- 本机系统 Node 24 直接 `npm run ci`：命中「无自有副本 → PATH 版本不符 → 下载 + SHASUMS 校验 + 解压 → re-exec → 全门禁 PASS」完整链路。
- 复现本次事故并加固后验证：删除自有副本 npm 的两个命令模块（此时 `npm --version` 烟测仍通过，证明必须用清单对账）→ 重跑打印 `broken; re-provisioning` → 命中 archives 缓存重建、命令文件恢复全数 → 全门禁 PASS。
- 健康快路径：再次运行无下载、无告警，直接进入门禁步骤。
- Windows 解压的两个环境坑已固化在实现内并以真实运行验证：GNU tar（Git Bash PATH 首位）把盘符当远程主机且读不了 zip，因此解压固定使用相对路径 + Windows 自带 bsdtar（`System32\tar.exe`）。
- `npm run verify-notes` 通过。
