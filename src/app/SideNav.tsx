import type { Icon as NavIcon } from "@phosphor-icons/react";
import { useI18n } from "../i18n";
import { PAGES, pageLabelKey, type Page } from "./navigation";
import { Button } from "../components/Button";
import { Tooltip } from "../components/Tooltip";
import {
  ClientConfigurationIcon,
  ExtensionsIcon,
  ProviderSwitchIcon,
  SessionsIcon,
  SettingsIcon,
  UsageNavigationIcon,
} from "../components/icons";
import appIcon from "../assets/app-icon.svg";

/** Icon per top-level page; the mapping sits beside the rail, its single
 * consumer, while the page list itself stays owned by navigation.ts. */
const PAGE_ICONS: Record<Page, NavIcon> = {
  providers: ProviderSwitchIcon,
  clientConfiguration: ClientConfigurationIcon,
  extensions: ExtensionsIcon,
  sessions: SessionsIcon,
  usage: UsageNavigationIcon,
  settings: SettingsIcon,
};

/** Workspaces in the scrollable group; settings is the pinned footer entry. */
const WORKSPACE_PAGES = PAGES.filter((page) => page !== "settings");

interface SideNavProps {
  page: Page;
  onPageChange: (page: Page) => void;
}

/** Left navigation rail, icon-only (Codex desktop style): the brand mark tops
 * the drag region, the workspace group scrolls, the settings entry stays
 * pinned above the bottom edge. Names live in each button's accessible name
 * and surface as right-side tooltips on hover and keyboard focus. */
export function SideNav({ page, onPageChange }: SideNavProps) {
  const { t } = useI18n();
  return (
    <aside className="asb-sidebar">
      <div className="asb-sidebar-top" data-tauri-drag-region>
        <img className="asb-sidebar-icon" src={appIcon} alt="" data-tauri-drag-region />
      </div>
      <nav className="asb-sidenav" aria-label={t("shell.nav.aria")}>
        <ul className="asb-sidenav-group">
          {WORKSPACE_PAGES.map((item) => (
            <li key={item}>
              <SideNavItem page={item} active={page === item}
                onSelect={() => onPageChange(item)} />
            </li>
          ))}
        </ul>
        <ul className="asb-sidenav-footer">
          <li>
            <SideNavItem page="settings" active={page === "settings"}
              onSelect={() => onPageChange("settings")} />
          </li>
        </ul>
      </nav>
    </aside>
  );
}

function SideNavItem({ page, active, onSelect }: {
  page: Page;
  active: boolean;
  onSelect: () => void;
}) {
  const { t } = useI18n();
  const label = t(pageLabelKey(page));
  const Icon = PAGE_ICONS[page];
  return (
    <Tooltip label={label} side="right">
      <Button
        variant="unstyled"
        className="asb-sidenav-item"
        aria-label={label}
        aria-current={active ? "page" : undefined}
        onClick={onSelect}
      >
        <Icon size={24} weight={active ? "fill" : "regular"} aria-hidden="true" focusable="false" />
      </Button>
    </Tooltip>
  );
}
