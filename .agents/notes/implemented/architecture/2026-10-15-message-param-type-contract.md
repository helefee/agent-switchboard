# Agent Note: 消息模板占位符与参数的编译期契约

Status: implemented

## Problem

消息模板的 `{name}` 占位符与调用点参数之间没有任何一层拥有契约：`MessageParams` 是无类型 `Record<string, string|number>`，`formatMessage` 对匹配不到的占位符静默保留原文。「官方额度读取失败」修复中模板写 `{detail}`、调用点传 `detailKey`，错配一路通过编译直到用户屏幕上显示字面 `{detail}` 才暴露；中英两个语言模板的占位符不一致同样无人检查。这类缺陷在 384 个带参调用点上每个都可能复发。

## Decision

模板字面量成为参数契约的唯一 owner，占位符集合由类型系统从模板推导，编译期强制：

- `Placeholders<S>` 从模板字符串字面量递归提取占位符名；`MessageParamsOf<K>` 把每个占位符映射为值类型——名字以 `Key` 结尾的占位符要求 `MessageKey` 值（递归目录解析约定），其余要求 `string | number`。
- `t`（useI18n）、`tr`（模块级）、`uiMessage` 统一改为 `<K extends MessageKey>(key: K, ...args: TemplateArgs<K>)`：模板无占位符时禁止传参，有占位符时参数对象缺少、错名任何必需键都是编译错误。
- `messages.ts` 增加中英占位符一致性哨兵：任一键的两个语言模板占位符集合不一致，编译在该声明处失败并点名键名。
- 唯一允许的动态边界是 `commandErrorText`：后端 `CommandError` 的 `messageKey`/`params` 是 Rust 侧运行时数据，无法静态对齐模板，因此该函数不再经过 `t`，直接以 `formatMessage` + 模块级语言解析渲染，并对 `*Key` 参数做递归目录解析。此边界是后端消息契约与前端目录的唯一定位点，不允许其他调用点绕过类型检查。

## Alternatives considered

- 保留宽松签名、补一个运行时开发模式校验（占位符与参数名比对告警）：能把错误提前到控制台，但仍依赖跑起来才暴露，且校验逻辑与 `formatMessage` 形成第二份插值语义，拒绝。
- 给每个键手写参数接口：最精确但 800+ 键的接口与模板必然漂移，模板推导让模板本身保持唯一事实，拒绝。
- 把 `*Key` 递归约定改为模板内嵌目录键字符串：插值语义会更简单，但后端 `CommandError` 的多语言复用依赖该约定，推翻它是更大的重写且与本次缺陷无关，不纳入。

## Consequences

编译器接管后，占位符错名、缺参、给 `*Key` 占位符传非目录键值、多传参数（对象字面量多余属性）、中英模板占位符漂移全部在 `tsc` 阶段失败；本次落地即借此清出所有存量错配点。代价：类型推导为每个键解析模板字面量，`tsc` 略慢；动态键名调用点（如 `` t(`doctor.${check.status}`) ``）按键联合分布检查，联合中任一键需要参数而调用未传时会失败——这类点必须拆明或确认全部无参。后端 Rust 侧的参数名与模板的对齐仍无法跨语言静态检查，`commandErrorText` 是它们唯一的、集中的汇入点。

Node 测试直接以裸 ESM 加载源码，无打包器兜底：凡可被 `tests/` 触达的模块（i18n、lib 及其传递闭包）相对导入必须带显式 `.ts` 扩展名，`import type` 因加载期擦除不受此限。该约定由源码门禁的前端测试步强制——`usage-format.ts` 引入 `i18n/errors` 时漏写扩展名，门禁即以 `ERR_MODULE_NOT_FOUND` 拒绝。

## Verification

`npm run typecheck` 通过。首轮检查在 384 个带参调用点中暴露 4 处问题，逐一修复并点名：

- `storage.archiveReview.head/tail`：中英占位符不对称——`{count}` 在英文位于 head、中文位于 tail，哨兵与调用点同时报错；重排为两种语言都由 head 携带 `{count}`，tail 调用删除参数。中文语序由「将归档 〈日期〉 之前的 N 条历史记录」变为「将归档 N 条 〈日期〉 之前的历史记录」，语义不变。
- `usage.query.declarativeNote`：占位符 `{apiKey}` 以 `Key` 结尾，触发目录键值类型要求，而它承载的是展示示例文本——正是命名约定与显示值的真实冲突；模板与调用点改名为 `{apiKeyExample}`。
- `ToastText`：toast 队列天然擦除键的参数类型，定为动态边界；`toastMessage` 生产者改为按模板检查，渲染处保留一处显式豁免。
- 其余 380+ 带参调用点与全部动态键调用点直接通过。

`npm run verify-notes` 校验笔记结构。按项目规则未运行其余编译与测试；插值运行时行为由既有界面文案验收（失败原因、倒计时、相对时间等带参文案）。
