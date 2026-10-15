# Agent Note: 全局键值事实行原语 FactList 统一

Status: implemented

## Problem

「标签在左、数值在右、逐行堆叠」的键值事实行有八个消费点、三套并行实现：`.asb-fact-row`（`panels.css`，flex + `--asb-fact-label-width: 6em` 定宽标签列，值用 label 字阶）、`.asb-session-facts`（`sessions.css`，`auto 1fr` 两列网格，值用 body 字阶）、`.asb-provider-resolved-endpoint`（`provider-editor.css`，同型网格 + 值等宽字阶 + 窄屏收拢单列）。三套 CSS 各自声明同一视觉契约——`panels.css` 的注释自称拥有该模式，另两套并未归一；`--asb-fact-label-width` 只服务其中一套。组件层同样三分：`ExtensionResourceFacts` 的本地 `Facts`、`ConfigStatusPanel` 的本地 `StatusField`，其余手写 `<dl>`。跨行对齐行为因此不一致：定宽列与自适应列给出的值起点不同。

## Decision

`src/components/FactList.tsx` 是软件内全部键值事实行的唯一原语，`styles/base/panels.css` 的 `.asb-fact-row` 是其语法的唯一拥有者：

- DOM 契约：`<dl class="asb-fact-row"><div><dt/><dd/></div>…</dl>`；API 为 `rows: { label, value }[]` 加 `inline?: boolean`，无其他配置面。
- 视觉契约：dl 是单一 `auto minmax(0, 1fr)` 两列网格、基线对齐、行距 space-1 列距 space-2；包装 div 为 `display: contents`，dt/dd 直接成为网格项——标签列取全表最宽标签，所有值自同一左缘逐行向下。dt 弱化色 + label 字阶，dd 正文 body 字阶 + `overflow-wrap: anywhere`。
- `inline` 变体（网关摘要横轨）：dl 退回可换行横轨、包装 div 退回 flex 行，标签取自然宽度——共享标签列只在值逐行向下时才有收益。
- 行内值的专属排布（`.asb-code` 等宽、`.asb-num` 数字、`.asb-warn-text` 警示色、`.asb-fact-path` 路径控件、会话恢复命令的 `.asb-session-resume-fact` 行内 flex）由行值自身携带，原语不拥有值内部布局。
- 供应商编辑器的解析端点字段保留模块级间距规则 `.asb-provider-endpoint .asb-fact-row { margin-top }`——全局原语之上的业务布局，归 `provider-editor.css`。

已完整接入（无兼容无过渡），八个消费点：设置-存储与缓存、会话详情事实区、供应商编辑器解析端点、设置-诊断-配置状态、设置-诊断-运行时概览、扩展资源事实、网关中心端口横轨、供应商导入发现卡。同步删除：`.asb-session-facts` 全块（sessions.css）、`.asb-provider-resolved-endpoint` 全块及其窄屏收拢行（provider-editor.css）、`--asb-fact-label-width`（tokens.css，唯一消费者消失）、本地 `Facts` 与 `StatusField` 辅助组件。`DESIGN.md` 组件清单新增 `FactList` 条目。

同主题笔记审计：全库检索无任何笔记持有事实行契约（panels.css 注释是唯一的口头声明），无可归档或互链的旧篇；[DatePicker](2026-10-15-global-date-picker-primitive.md) 与折叠原语笔记不触及键值行布局。

## Alternatives considered

- 保留 flex + 定宽标签列：最强理由是列宽与最长标签解耦——某行标签特别长时不会把整列值推远，且各行的标签-值间距恒定。否决：本产品的标签全是短 i18n 短语（现存最长「其中：配置恢复备份」），6em 定宽要么留白要么截断；auto 列与用户已见、已认可的存储/会话两处完全一致，且单一网格天然给出跨行对齐，不需要 token 参与。
- children API（`<FactList><FactRow label value/></FactList>`）：最强理由是条件行与带 key 行表达更自然，不必用数组 spread。否决：八个消费点里多数从数据数组出发，rows 数组最短；条件行用条件 spread 表达同样直接；数组 API 的暴露面更小。
- 把「标签在上、数值在下」的请求诊断 dl（`ProviderRequestPanel`）与 Codex 重置记录卡也并入：最强理由是进一步消灭手写 `<dl>`。否决：那是刻意不同的模式——URL、Request ID 等长等宽值竖排更可读，与横排事实行不是同一契约，强行归一会推翻既有的可读性决策；`DESIGN.md` 的禁止项针对的是平行组件，不是形态不同的独立布局。
- 只统一组件层、保留三套 CSS：最强理由是零视觉变化、改动面最小。否决：三套并行实现本身就是技术债，组件归一而样式三头所有只是把债藏进样式层，违背本次目标。

## Consequences

- 收益：八处获得同一标签列对齐与同一字阶；新增事实行只有一个入口；`--asb-fact-label-width` 与两套平行类消失；诊断、导入、扩展面板的值列对齐到标签列右缘、值升为 body 字阶，长路径与数值更可读。
- 代价：`display: contents` 意味着包装 div 不参与渲染盒——若未来要做行级装饰（底色、分隔线），必须挂在 dt/dd 或重建行容器，不能挂包装；诊断、导入、扩展三处有可见的视觉变化（对齐与字阶），属于统一的预期结果而非回归。
- 后续任何键值事实行必须经 `FactList`；不得再手写 `<dl>` 事实结构或新增平行键值行布局类。修改行语法只改 `panels.css` 的 `.asb-fact-row` 块。

## Testing

- `npm run verify-notes` 通过（笔记结构门禁）。
- 按工作区规则未运行 typecheck/test（用户未要求）；人工核对：全库 `asb-session-facts`、`asb-provider-resolved-endpoint`、`asb-fact-label-width`、组件内 `<dl className="asb-fact-row">` 与旧辅助组件名零残留；八处消费点逐条对照原渲染——条件行、警示色 span、等宽值、`asb-num` 数字、`FactPath`、恢复命令的行内 flex（类移至值内 span）全部保留，仅承载结构变更。
