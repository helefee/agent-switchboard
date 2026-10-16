import { invoke } from "./client";

/** Egress policy for application-owned HTTP clients: none (always direct),
 * system (follow the environment) or manual (an explicit proxy URL). */
export type OutboundProxyMode = "none" | "system" | "manual";

export interface OutboundProxySettings {
  version: number;
  mode: OutboundProxyMode;
  /** Absent outside manual mode; the backend omits the key entirely. */
  url?: string | null;
  revision: string;
}

export function getOutboundProxySettings(): Promise<OutboundProxySettings> {
  return invoke<OutboundProxySettings>("get_outbound_proxy_settings", {});
}

/** Saves with optimistic concurrency; a stale revision is rejected. */
export function setOutboundProxySettings(
  settings: OutboundProxySettings,
  expectedRevision: string,
): Promise<OutboundProxySettings> {
  return invoke<OutboundProxySettings>("set_outbound_proxy_settings", { settings, expectedRevision });
}

export interface OutboundProxyRepairReport {
  settings: OutboundProxySettings;
  backupPath: string | null;
}

/** Rechecks the file and preserves invalid bytes before restoring defaults. */
export function repairOutboundProxySettings(): Promise<OutboundProxyRepairReport> {
  return invoke<OutboundProxyRepairReport>("repair_outbound_proxy_settings", {});
}
