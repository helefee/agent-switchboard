import type { MessageEntry } from "../types";

/**
 * Application frame: navigation, global information, shared controls and
 * generic actions. Page/section keys derive their prefix from stable ids;
 * display text lives only here.
 */
export const messages = {
  "nav.page.providers": ["供应商切换", "Providers"],
  "nav.page.clientConfiguration": ["客户端配置", "Client Config"],
  "nav.page.extensions": ["扩展", "Extensions"],
  "nav.page.sessions": ["会话记录", "Sessions"],
  "nav.page.usage": ["用量监控", "Usage"],
  "nav.page.settings": ["设置", "Settings"],

  "nav.section.application": ["偏好设置", "Preferences"],
  "nav.section.clientManagement": ["客户端工具", "Client Tools"],
  "nav.section.gateway": ["本机网关", "Local Gateway"],
  "nav.section.backups": ["备份恢复", "Backup & Restore"],
  "nav.section.diagnostics": ["诊断", "Diagnostics"],
  "nav.section.about": ["关于", "About"],

  "nav.section.consumption": ["消耗统计", "Consumption"],
  "nav.section.quota": ["额度与重置", "Quota & Resets"],
  "nav.section.radar": ["降智雷达", "Degradation Radar"],

  "nav.section.configuration": ["配置与环境", "Config & Environment"],
  "nav.section.logs": ["运行日志", "Runtime Logs"],

  "nav.section.skill": ["Skills", "Skills"],
  "nav.section.mcp": ["MCP", "MCP"],

  "shell.devBadge": ["浏览器开发 · 本机后端", "Browser dev · local backend"],
  "shell.nav.aria": ["主导航", "Primary navigation"],
  "shell.busy": ["处理中", "Working"],
  "shell.information.aria": ["全局信息", "Global information"],
  "shell.information.show": ["查看全局信息（{count} 条）", "View global information ({count})"],
  "shell.information.close": ["收起全局信息", "Close global information"],
  "shell.banner.repairStore": ["一键智能修复", "Repair provider data"],
  "shell.banner.settingsUnavailable": ["应用设置不可用：{detail}", "App settings unavailable: {detail}"],
  "shell.banner.repair": ["一键修复", "Quick repair"],

  "shell.workspaceRestoring.aria": ["正在恢复工作区", "Restoring workspace"],
  "shell.startupPageReading": ["正在读取启动页面", "Reading startup page"],
  "shell.sessions.aria": ["会话管理", "Session management"],

  "window.minimize": ["最小化", "Minimize"],
  "window.maximize": ["最大化", "Maximize"],
  "window.restore": ["还原", "Restore"],
  "window.close": ["关闭", "Close"],
  "window.closeDialog": ["关闭窗口", "Close window"],

  "pin.pin": ["置顶窗口", "Keep window on top"],
  "pin.unpin": ["取消置顶", "Disable always on top"],

  "update.available": ["发现新版本 {version}", "New version available {version}"],
  "update.button": ["更新", "Update"],

  "toast.dismiss": ["关闭通知", "Dismiss notification"],
  "common.operationFailed": ["操作失败", "Operation failed"],

  "confirm.cancel": ["取消", "Cancel"],

  "pagination.aria": ["分页", "Pagination"],
  "pagination.prev": ["上一页", "Previous page"],
  "pagination.next": ["下一页", "Next page"],
  "pagination.status": ["第 {page} / {total} 页", "Page {page} of {total}"],

  "recovery.aria": ["界面恢复", "Interface recovery"],
  "recovery.title": ["界面未能加载", "The interface failed to load"],
  "recovery.body": ["可关闭窗口后从系统托盘重新打开应用。", "Close the window and reopen the app from the system tray."],
} as const satisfies Record<string, MessageEntry>;
