# Agent Switchboard 网站

独立静态介绍站：Vite + React + TypeScript，与桌面应用工程完全分离。
内容契约见 `src/content/site-content.ts`；视觉令牌唯一所有者是 `src/styles/tokens.css`。

软件截图于 2026 年 10 月 13 日从当前源码前端使用隔离模拟数据重新截取。`public/media/screenshots/` 与根目录 `docs/screenshots/` 共用同一组图片；供应商使用 OpenAI、Claude 演示档案，模型名使用当前官方名称。余额、用量、检测与网关请求均为模拟记录。来源、隔离边界和图片清单见[截图数据说明](../docs/screenshots/README.md)。

## 本地开发

```bash
cd website
npm ci
npm run dev        # http://127.0.0.1:1421 之前请以命令输出为准
npm run build      # tsc --noEmit && vite build，输出 dist/
npm run preview    # 预览生产构建
```

类型检查、构建与自动化测试仅在当前请求明确要求时运行。

## Vercel 部署（Git 集成，无需令牌与脚本）

1. 在 vercel.com 导入 GitHub 仓库 `<GitHub 用户名>/agent-switchboard`。
2. 项目设置：
   - **Root Directory**：`website`
   - **Framework Preset**：Vite（或 Other，构建命令相同）
   - **Build Command**：`npm run build`
   - **Output Directory**：`dist`
   - **生产分支**：`master`
3. 保存后，推送 `master` 即自动更新正式站点；其他分支获得 Preview 部署。
4. 下载入口唯一指向 GitHub Releases 最新页，不维护版本号与安装包文件名；
   无需因发版改动站点代码。

链接契约：

以下示例中的 `<GitHub 用户名>` 是占位符，使用时替换为实际仓库所有者。

- 源码：`https://github.com/<GitHub 用户名>/agent-switchboard`
- 下载：`https://github.com/<GitHub 用户名>/agent-switchboard/releases/latest`
