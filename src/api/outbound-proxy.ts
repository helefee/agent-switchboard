import { invoke } from "./client";

/** Egress policy for every gateway-side HTTP client: none (always direct),
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

/** One-click reset for a settings file that no longer parses (another
 * version's format or corruption); returns the fresh default. */
export function repairOutboundProxySettings(): Promise<OutboundProxySettings> {
  return invoke<OutboundProxySettings>("repair_outbound_proxy_settings", {});
}
