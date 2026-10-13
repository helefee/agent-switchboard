import { useState, type ReactNode } from "react";
import type { CommandError } from "../api/client";
import { useI18n } from "../i18n";
import type { Page } from "./navigation";
import { SideNav } from "./SideNav";
import { PinTopButton } from "../components/PinTopButton";
import { UpdateButton } from "../components/UpdateButton";
import { GlobalInformation } from "../components/GlobalInformation";
import { WindowControls } from "../components/WindowControls";
import { isBrowserDevelopment } from "../lib/runtime";
import appIcon from "../assets/app-icon.svg";

interface AppShellProps {
  page: Page;
  onPageChange: (page: Page) => void;
  /** Persistent decision errors, displayed alongside transient global feedback. */
  error: CommandError | null;
  busy: boolean;
  /** Why application settings could not load; global information keeps the repair
   * reachable from every page while settings-backed actions silently wait. */
  settingsError: string | null;
  /** Replaces an unreadable settings file with validated defaults. */
  onRepairSettings: () => void;
  /** Offered only for an unsupported profile store. */
  onRepairStore: () => void;
  /** Always-on-top toggle state; null until settings load. */
  pin: { active: boolean; onToggle: () => void } | null;
  /** Update indicator; present only while a newer release is known. */
  update: { latestVersion: string; onOpen: () => void } | null;
  children: ReactNode;
}

/** Application frame: one full-window canvas beneath a frosted-glass chrome —
 * the left navigation rail and the slim titlebar, which owns the drag region
 * and the window actions. Page content scrolls under the chrome and is
 * blurred by it. Page geometry never changes when banners appear or
 * disappear. */
export function AppShell({
  page,
  onPageChange,
  error,
  busy,
  settingsError,
  onRepairSettings,
  onRepairStore,
  pin,
  update,
  children,
}: AppShellProps) {
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState(false);
  return (
    <div className={`asb-shell${collapsed ? " is-collapsed" : ""}`}>
      <SideNav page={page} onPageChange={onPageChange} collapsed={collapsed}
        onToggle={() => setCollapsed((value) => !value)} />
      <header className="asb-titlebar" data-tauri-drag-region>
        <span className="asb-titlebar-brand" data-tauri-drag-region>
          <img className="asb-titlebar-icon" src={appIcon} alt="" data-tauri-drag-region />
          <span className="asb-titlebar-name" data-tauri-drag-region>Agent Switchboard</span>
        </span>
        <GlobalInformation error={error} settingsError={settingsError} busy={busy}
          onRepairStore={onRepairStore} onRepairSettings={onRepairSettings} />
        <div className="asb-titlebar-actions">
          {isBrowserDevelopment ? <span className="asb-web-development-badge">{t("shell.devBadge")}</span> : null}
          {update ? (
            <UpdateButton latestVersion={update.latestVersion} onOpen={update.onOpen} />
          ) : null}
          {pin ? (
            <PinTopButton active={pin.active} disabled={busy} onToggle={pin.onToggle} />
          ) : null}
          {!isBrowserDevelopment && <WindowControls />}
        </div>
      </header>
      <div className="asb-workspace">
        {children}
      </div>
    </div>
  );
}
