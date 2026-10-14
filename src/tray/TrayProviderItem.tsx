import { Check, RefreshCw } from "lucide-react";
import type { TraySnapshot } from "../api/client";
import { useI18n } from "../i18n";
import { formatTrayReading } from "../lib/usage-format";
import { TrayUsage, trayUsageContent } from "./TrayUsage";
import { cx } from "../utils/cx";

type TrayProvider = TraySnapshot["providers"][number];

interface Props {
  provider: TrayProvider;
  busy: boolean;
  switchingId: string | null;
  refreshingId: string | null;
  onSwitch: () => void;
  onRefreshUsage: () => void;
}

/** The row is a container owning the silhouette; the switch action spans
 * every column as one native button and the usage warning carries a second
 * native refresh button. Both stay real buttons because a button cannot be
 * nested; tray.css owns every visual property. */
export function TrayProviderItem({ provider, busy, switchingId, refreshingId, onSwitch, onRefreshUsage }: Props) {
  const { t } = useI18n();
  const { readings, alert } = trayUsageContent(provider.usage);
  const balance = [
    ...readings.map((reading) => `${reading.planName ?? ""} ${formatTrayReading(reading)}`),
    alert?.title,
  ].filter(Boolean).join("，");
  const action = provider.active
    ? t("tray.provider.active", { name: provider.name })
    : t("tray.provider.switchTo", { name: provider.name });

  return (
    <div className={cx("tray-provider", provider.active && "is-active", switchingId === provider.id && "is-switching")}>
      <button
        type="button"
        className="tray-provider-main"
        disabled={busy || provider.active}
        aria-label={balance ? `${action}，${balance}` : action}
        title={[provider.name, balance].filter(Boolean).join("\n")}
        onClick={onSwitch}
      >
        <span className="tray-provider-check">{provider.active && <Check size={18} aria-hidden="true" />}</span>
        <span className="tray-provider-name" title={provider.name}>{provider.name}</span>
        <TrayUsage usage={provider.usage} />
        <span className="tray-provider-switch">
          {provider.active ? null : switchingId === provider.id ? t("tray.provider.switching") : t("tray.provider.switch")}
        </span>
      </button>
      {alert && (
        <button
          type="button"
          className="tray-provider-refresh"
          disabled={busy}
          aria-label={t("tray.usage.refresh")}
          title={t("tray.usage.refresh")}
          onClick={onRefreshUsage}
        >
          <RefreshCw size={14} className={cx(refreshingId === provider.id && "is-busy")} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}
