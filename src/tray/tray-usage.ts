import type { TrayUsage, UsageReading } from "../api/client";
import { formatTrayReading, officialQuotaReadings, usagePrimary, usageTone } from "../lib/usage-format.ts";
import { timeLabel } from "../lib/time.ts";
import { tr } from "../i18n/current.ts";
import type { MessageKey } from "../i18n/messages";

export interface TrayReading {
  name: string | null;
  value: string;
  tone: "danger" | undefined;
  summary: string;
}

interface TrayUsageAlert {
  title: string;
  detail: string;
}

export interface TrayUsageContent {
  readings: TrayReading[];
  alert: TrayUsageAlert | null;
  summary: string;
}

function lastReadingLine(at: string | null | undefined): string | null {
  if (!at || !Number.isFinite(new Date(at).getTime())) return null;
  return tr("tray.usage.lastReading", { time: timeLabel(at) });
}

function usageState(usage: TrayUsage | null): { readings: UsageReading[]; alert: TrayUsageAlert | null } {
  if (!usage) return { readings: [], alert: null };
  if (usage.kind === "script") {
    const readings = usage.reading.summary?.readings ?? [];
    if (!usage.reading.error) return { readings, alert: null };
    return { readings, alert: {
      title: tr("tray.state.queryFailed"),
      detail: [tr("tray.state.queryFailed"), usage.reading.error,
        lastReadingLine(usage.reading.summary?.at)].filter(Boolean).join("\n"),
    } };
  }
  const reading = usage.reading;
  const readings = officialQuotaReadings(reading);
  const statusKey: MessageKey | null = reading.status === "signInRequired" ? "tray.state.signInRequired"
    : reading.status === "reauthenticationRequired" ? "tray.state.reauthRequired"
    : reading.status === "unavailable" ? "tray.state.queryFailed" : null;
  const stateKey: MessageKey | null = statusKey ?? (reading.stale && readings.length > 0 ? "tray.state.stale" : null);
  return { readings, alert: stateKey ? {
    title: tr(stateKey),
    detail: [tr(stateKey), lastReadingLine(reading.at)].filter(Boolean).join("\n"),
  } : null };
}

function formatReading(reading: UsageReading): TrayReading {
  const plan = reading.planName?.trim() || null;
  const valueHasUnit = reading.availability.kind !== "unavailable" && usagePrimary(reading) !== null;
  const name = valueHasUnit && plan?.toUpperCase() === reading.unit?.trim().toUpperCase() ? null : plan;
  const value = formatTrayReading(reading);
  return { name, value, tone: usageTone(reading), summary: [name, value].filter(Boolean).join(" · ") };
}

/** One presentation projection feeds the row, accessible text and width ruler. */
export function trayUsageContent(usage: TrayUsage | null): TrayUsageContent {
  const state = usageState(usage);
  const readings = state.readings.map(formatReading);
  return { readings, alert: state.alert,
    summary: [...readings.map((reading) => reading.summary), state.alert?.title].filter(Boolean).join("，") };
}
