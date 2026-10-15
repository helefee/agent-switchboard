# Agent Note: Time 组件 date 模式承载日历日期，存储归档截止日接入全局时间组件

Status: implemented

## Problem

存储与缓存的归档确认弹窗把 UTC 截止日期当原始字符串插值：`storage.archiveReview` 的 `{cutoff}` 占位符直接输出后端回传的 `YYYY-MM-DD`，未经本地化、无 `<time>` 语义，违反 `DESIGN.md`「`Time`：界面时间语义和本地格式化」的唯一所有权（「不创建平行的时间组件」）。直接换用既有 `timeLabel` 也不行：`new Date("2026-06-16")` 被解析为 UTC 午夜再转本地时区，会虚构出「08:00」这类不存在的时刻，且在 UTC 负偏移时区把日历日期显示成前一天——截止日是 UTC 日期边界，不是本地时刻。

## Decision

`Time` 组件新增 `date` 模式，成为「日历日期（非时刻）」显示的唯一入口；`lib/time.ts` 新增 `dateLabel` 作为其唯一格式化器，维持「Time 是格式化器唯一消费者」契约：

- `dateLabel` 对 `YYYY-MM-DD` 逐字呈现、不做时区换算：zh 走既有 `format.date.full`（`2026年06月16日`），en 走 `timeZone: "UTC"` 锚定的 `Intl.DateTimeFormat`（`Jun 16, 2026`），与 `timeLabel` 的双语言结构一致。
- `date` 模式不参与相对计时（同 absolute 跳过 30 秒 tick），无 `title` 悬浮（值本身就是日期，悬浮补充时刻只会误导）；仍输出语义化 `<time dateTime>`。
- 归档确认弹窗改为 `<Time iso={plan.cutoff} mode="date" />` 内联渲染。因 zh/en 两语中 `{count}` 与 `{cutoff}` 语序不同（zh 日期在前、en 日期在后），`storage.archiveReview` 拆为 `storage.archiveReview.head` / `storage.archiveReview.tail` 两段，原占位符两侧空格逐字保留在段内；旧键 `storage.archiveReview` 删除，无双读。

范围边界：本次只迁移存储归档确认这一处日期边界展示。日期选择控件由全局 `DatePicker` 原语拥有（见 [全局日期选择原语](2026-10-15-global-date-picker-primitive.md)），不在本契约内；其余日期边界展示（如雷达按日分组）日后应走 `date` 模式，不再插原始字符串。

同主题笔记审计：`implemented/feature/2026-10-14-tray-usage-alert-refresh.md` 是 `timeLabel` 的消费事实（不受影响）；`implemented/feature/2026-10-14-installer-ring-progress.md` 是进度语义（无关）；无旧笔记持有时间显示契约。

## Alternatives considered

- 不拆消息，把 `dateLabel(iso)` 结果作为字符串塞进原 `{cutoff}` 占位符：最强理由是模板零改动、翻译按整句审校。否决：`lib/time.ts` 契约写明 Time 组件是其格式化器的唯一消费者，绕过组件会产生第二个消费点，且丢失语义化 `<time>` 元素与后续统一调整显示的收口点。
- 复用 `timeLabel` 显示完整本地时刻：最强理由是零新增代码。否决：UTC 日期边界不是本地时刻，负偏移时区日期后移一天并虚构时刻，语义错误优先级高于改动最小化。
- 截止日期选择改为会话库同款「预设档位下拉」（近 7 天/近 30 天/自定义）：最强理由是全应用时间筛选交互一致。否决：归档是单一 UTC 截止边界，不是时间范围筛选，档位会引入多余语义；本次诉求是显示层接入全局时间组件，不是改选择交互。

## Consequences

- 收益：确认弹窗截止日期随界面语言本地化并带 `<time>` 语义；负偏移时区不再显示错位日期。
- `date` 模式确立后，任何「日历日期」显示都有唯一路径；新增日期展示若绕过 `Time` 插原始字符串，即为违反契约。
- 代价：确认文案拆为两段消息，翻译审校需按段拼读；两段之间内嵌 React 节点，段内空格是文案的一部分，整理文案时不可丢弃。

## Testing

- `npm run verify-notes` 通过（笔记结构门禁）。
- 按工作区规则未运行 typecheck/test（用户未要求）；以逐文件人工核对：`dateLabel` 对 `2026-06-16` 在 zh 输出 `2026年06月16日`、en 输出 `Jun 16, 2026`（UTC 锚定）；拆分后 head+日期+tail 与原模板逐字符一致（zh「将归档 」+日期+「 之前的 {count} 条…」、en「Archive {count} … before 」+日期+「. A complete…」）；`storage.archiveReview` 旧键全库零残留。
