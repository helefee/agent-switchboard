import { useLayoutEffect, useRef, useState } from "react";
import { TriangleAlert } from "lucide-react";
import type { TrayUsage as Usage, UsageReading } from "../api/client";
import { formatTrayReading, officialQuotaReadings, usageTone } from "../lib/usage-format";
import { timeLabel } from "../lib/time";
import { tr } from "../i18n/current";
import type { MessageKey } from "../i18n/messages";

/** One warning state of a provider's usage. `title` names the state in the
 * row summary; `detail` is the hover content: state label, backend error
 * text and the last successful read, one per line. */
export interface TrayUsageAlert {
  title: string;
  detail: string;
}

interface TrayUsageState {
  readings: UsageReading[];
  alert: TrayUsageAlert | null;
}

function lastReadingLine(at: string | null | undefined): string | null {
  if (!at || !Number.isFinite(new Date(at).getTime())) return null;
  return tr("tray.usage.lastReading", { time: timeLabel(at) });
}

export function trayUsageContent(usage: Usage | null): TrayUsageState {
  if (!usage) return { readings: [], alert: null };
  if (usage.kind === "script") {
    const readings = usage.reading.summary?.readings ?? [];
    if (!usage.reading.error) return { readings, alert: null };
    return { readings, alert: {
      title: tr("tray.state.queryFailed"),
      detail: [
        tr("tray.state.queryFailed"),
        usage.reading.error,
        lastReadingLine(usage.reading.summary?.at),
      ].filter(Boolean).join("\n"),
    } };
  }
  const reading = usage.reading;
  const readings = officialQuotaReadings(reading, true);
  const statusKey: MessageKey | null = reading.status === "signInRequired" ? "tray.state.signInRequired"
    : reading.status === "reauthenticationRequired" ? "tray.state.reauthRequired"
    : reading.status === "unavailable" ? "tray.state.queryFailed" : null;
  const staleKey: MessageKey | null = !statusKey && reading.stale && readings.length > 0
    ? "tray.state.stale" : null;
  const stateKey = statusKey ?? staleKey;
  if (!stateKey) return { readings, alert: null };
  return { readings, alert: { title: tr(stateKey),
    detail: [tr(stateKey), lastReadingLine(reading.at)].filter(Boolean).join("\n") } };
}

function Reading({ reading }: { reading: UsageReading }) {
  return <span className="tray-usage-reading" data-tone={usageTone(reading)}>
    {reading.planName && <span className="tray-usage-name">{reading.planName}</span>}
    <span className="tray-usage-value">{formatTrayReading(reading)}</span>
  </span>;
}

export function TrayUsage({ usage }: { usage: Usage | null }) {
  const { readings, alert } = trayUsageContent(usage);
  const host = useRef<HTMLSpanElement>(null);
  const measure = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(2);
  const total = readings.length;
  const signature = JSON.stringify({ readings, alert });
  useLayoutEffect(() => {
    const element = host.current;
    const ruler = measure.current;
    if (!element || !ruler || !element.parentElement) return;
    const resize = () => {
      const row = element.parentElement!;
      const available = row.clientWidth * 0.60;
      const children = Array.from(ruler.children) as HTMLElement[];
      const fixed = children.filter((child) => child.dataset.measure === "fixed")
        .reduce((sum, child) => sum + child.offsetWidth + 8, 0);
      const values = children.filter((child) => child.dataset.measure === "reading");
      let width = fixed;
      let count = 0;
      for (const child of values) {
        const more = count + 1 < total ? 40 : 0;
        if (width + child.offsetWidth + 8 + more > available) break;
        width += child.offsetWidth + 8;
        count += 1;
      }
      setVisible(count);
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(element.parentElement);
    observer.observe(ruler);
    return () => observer.disconnect();
  }, [signature, total]);
  if (readings.length === 0 && !alert) return null;
  return <span ref={host} className="tray-provider-balance">
    {readings.slice(0, visible).map((reading, index) => <Reading key={index} reading={reading} />)}
    {readings.length > visible && <span className="tray-usage-more" title={readings.slice(visible)
      .map((reading) => `${reading.planName ?? ""} ${formatTrayReading(reading)}`.trim()).join("\n")}>
      +{readings.length - visible}</span>}
    {alert && <span className="tray-usage-alert" role="img" aria-label={alert.title} title={alert.detail}>
      <TriangleAlert size={14} aria-hidden="true" /></span>}
    <span ref={measure} className="tray-usage-measure" aria-hidden="true">
      {readings.slice(0, 2).map((reading, index) => <span key={index} data-measure="reading"><Reading reading={reading} /></span>)}
      {alert && <span data-measure="fixed"><TriangleAlert size={14} aria-hidden="true" /></span>}
    </span>
  </span>;
}
