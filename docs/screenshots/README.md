# 软件截图与演示数据

本目录的 8 张截图于 2026 年 10 月 13 日从当前源码前端截取，供中英文 README 和官网共用。截图展示中文界面；它们不代表某个已发布安装包的验收结果。

## 数据边界

- 使用独立无头浏览器和空白浏览器上下文，直接渲染项目的 React 前端。
- 在页面加载前接管 Tauri 调用，只返回演示档案、配置预览、余额、用量、检测历史和网关状态；不启动真实配置后端。
- 浏览器只允许访问截图专用的本机前端地址，阻断外部请求；不调用模型服务，不读取或写入个人配置、凭据、真实会话或业务数据。
- 供应商仅使用 OpenAI、Claude 演示档案；地址使用保留的 `.example` 域名，配置与备份路径使用 `C:/Demo/`，密钥使用明确的演示占位值。
- 余额、Token、检测通过数、耗时和网关请求均为模拟值，不是价格、配额、性能或可用性的实测声明。

## 模型名称

截图采用截取时官方文档列出的模型标识，模型名本身不是虚构名称：

- Codex：`gpt-6.1-sol`、`gpt-6-astra`、`gpt-6-luna`。来源：[OpenAI 官方模型文档](https://learn.chatgpt.com/docs/models)。
- Claude：`claude-sonnet-5-5`；演示档案中的其他角色使用 `claude-opus-5-5`、`claude-haiku-5-5` 和 `claude-fable-5-1`。来源：[Claude 官方模型概览](https://platform.claude.com/docs/en/models/overview)。

这些标识仅用于说明界面，不表示演示供应商实际提供对应模型。

## 图片归属

| 图片 | 展示内容 |
| --- | --- |
| `providers.png` | 当前连接、OpenAI 档案与模拟余额 |
| `switch-preview.png` | 模型及运行参数的切换差异、候选文件与备份位置 |
| `client-configuration.png` | 子 agent 运行设置与终端偏好 |
| `usage.png` | 模型消耗构成、七日趋势与明细 |
| `radar.png` | 五次模拟检测的结果与消耗 |
| `radar-history.png` | 模拟检测历史及筛选入口 |
| `gateway.png` | 模拟网关连接拓扑、协议与请求状态 |
| `subagent-route.png` | 跨档案的默认子 agent 模型选择 |

官网使用相同图片的副本，位于 `website/public/media/screenshots/`；图片说明和尺寸由 `website/src/content/site-content.ts` 拥有。更新截图时同步替换两处图片，并按实际画面更新两份 README 和官网说明。
