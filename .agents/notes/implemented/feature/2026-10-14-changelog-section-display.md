# Agent Note: 应用内更新日志的 Markdown 渲染与当前版本段落

Status: implemented

## Problem

更新模块只在发现新版本时显示新版本的更新说明，用户看不到当前运行版本包含哪些变更；且说明中的行内 Markdown（代码、链接等）不曾渲染。仓库根的 CHANGELOG.md 已自声明为"GitHub Release 与应用内更新说明的唯一来源"，但应用内没有读取它的通路。

## Decision

Markdown 本身就是格式契约，项目不拥有也不解析它。CHANGELOG.md 经 `tauri.conf.json` 资源映射随应用打包；后端 `changelog.rs` 只拥有版本段落边界：`changelog_section(version)` 定位 `## [version]` 段、截取到下一个 `## [` 为止，返回段落原文（不含段标题行），文件或段落缺失返回 `null`。渲染归前端共享查看器 `MarkdownContent`（react-markdown + remark-gfm，会话记录与更新日志唯一共用的渲染管线）：原生 HTML 一律不渲染（库默认，无注入面），http(s) 链接经 opener 打开系统浏览器，非 http 文本不渲染为链接。更新说明与新版本 release body 都是原文进查看器；元素间距由共享 `.asb-markdown` 作用域类拥有（自 session-message.css 提升），`.asb-update-notes` 与 `.asb-session-message-body` 只保留各自的布局。当前版本说明在 `appVersion` 就绪时加载一次，结果为空或读取失败即不渲染披露。设置行（`.asb-app-setting-row`）的垂直对齐契约为统一的顶端对齐：动作按钮钉在文案首行旁，任何行内增长的正文（如整篇日志）都不会把按钮拖进正文中间，不设按行特例。

## Related decisions

- [标题栏信息](../feature/2026-10-12-titlebar-information.md)拥有全局反馈位置，与本篇更新模块内容无冲突。
- 壳层笔记（`2026-10-14-unified-native-glass-shell.md`）拥有侧边栏更新入口的位置与状态，本篇只拥有设置页更新模块的日志展示。

## Alternatives considered

- 自定义结构契约（初版方案：后端解析 `### `/`- ` 返回 `{title, items}`，前端纯渲染结构）：前端无法渲染任意 Markdown，行内语法只能按契约逐个加点；为了"完整 Markdown"再养一套自定义格式，等于重复 markdown 库的职责。
- 后端 pulldown-cmark 渲染成 HTML 下发：多一条 HTML 消毒链路与注入面，Rust 侧重复前端已有的渲染能力。
- 前端直接请求 GitHub releases API 按标签取正文：每次打开设置页都要联网，离线不可用，且与 updater manifest 形成重复通路。
- 两个消费面各写一份 `Markdown remarkPlugins` 接线和链接拦截（中间态）：链接打开行为成为两处契约；已提取为 `MarkdownContent` 单一所有者，会话记录同步迁移。
- 把 CHANGELOG.md 编译进二进制常量：多一条构建生成链路；资源文件随包分发已足够。

## Consequences

- 收益：应用代码零 Markdown 解析——格式契约归 Markdown 规范与 remark 库，应用只拥有"取段落"和"渲染"两端；会话记录与更新日志共用同一查看器、同一链接行为、同一元素样式；当前版本与新版本共享同一渲染路径。安装包只多几 KB 文本资源，读取是本地文件访问。
- 代价：react-markdown 进主包（此前仅会话页使用）；`## [version]` 边界仍是 `changelog.rs` 解析的契约，改段标题格式必须同步；链接行为沿用会话记录的既有语义（`openUrl` 失败静默）；开发模式下资源目录可能不含该文件，披露不出现（生产行为不受影响）。
- 披露默认收起，不增加更新节的默认高度；details 原生键盘行为无需额外代码。

## Verification

按项目要求不运行编译与自动化测试。静态核对：`version_section` 针对当前 CHANGELOG.md 首段与相邻段边界手推通过，`## [0.3.3]` 的右括号保证不误匹配 `0.3.30` 类版本；`tauri.conf.json` 资源映射（JSON 解析通过）、lib.rs 模块声明、命令注册、client.ts 契约与渲染链经源码交叉检查；旧结构契约（`ChangelogSection`、`parse_release_notes`、`asb-update-notes-section`）与 `asb-session-message-body` 元素样式的残留引用检查为零，react-markdown 库引用仅剩共享查看器一处。会话记录迁移后的视觉等价性依据是选择器集合逐条平移，未做像素比对。
