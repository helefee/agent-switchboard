import type { ReactNode } from "react";
import type { CommandError } from "../api/client";
import { useI18n } from "../i18n";
import type { Page } from "./navigation";
import { SideNav } from "./SideNav";
import { PinTopButton } from "../components/PinTopButton";
import { UpdateButton } from "../components/UpdateButton";
import { GlobalInformation } from "../components/GlobalInformation";
import { WindowControls } from "../components/WindowControls";
import { isBrowserDevelopment } from "../lib/runtime";

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

/** Application frame: left navigation sidebar (brand and primary navigation)
 * beside one workspace column whose slim titlebar owns the
 * drag region and the window actions. Page geometry never changes when
 * banners appear or disappear. */
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
  return (
    <>
      <div className="asb-shell">
        <SideNav page={page} onPageChange={onPageChange} />
        <div className="asb-body">
          <header className="asb-titlebar" data-tauri-drag-region>
            <span className="asb-titlebar-name" data-tauri-drag-region>Agent Switchboard</span>
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
      </div>
    </>
  );
}
