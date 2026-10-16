import { uiMessage } from "../i18n/errors";
import { useMessageState } from "../i18n/use-message-state";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getUsageHistory,
  type UsageHistoryRequest,
  type UsageHistorySeries,
} from "../api/client";

function requestKey(request: UsageHistoryRequest): string {
  return request.kind === "provider" ? `provider:${request.profileId}` : "official";
}

/** Owns a rendered panel's read-only history request. A completed live query
 * calls `refresh` to show its newly persisted point without any second
 * provider request. Fetches only while `enabled`. */
export function useUsageHistory(request: UsageHistoryRequest, enabled: boolean) {
  const key = requestKey(request);
  const currentRequest = useMemo<UsageHistoryRequest>(
    () => (request.kind === "provider" ? { kind: "provider", profileId: request.profileId } : { kind: "official" }),
    [key],
  );
  const [snapshot, setSnapshot] = useState<{ key: string; series: UsageHistorySeries[] } | null>(null);
  const [readKey, setReadKey] = useState(key);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useMessageState();
  const version = useRef(0);
  const visible = useRef(enabled);
  visible.current = enabled;

  const refresh = useCallback(async () => {
    if (!visible.current) return;
    const current = ++version.current;
    setReadKey(key);
    setLoading(true);
    setError(null);
    try {
      const next = await getUsageHistory(currentRequest);
      if (!Array.isArray(next)) throw uiMessage("usage.history.invalidResponse");
      if (version.current === current) setSnapshot({ key, series: next });
    } catch (reason) {
      if (version.current === current) setError(reason);
    } finally {
      if (version.current === current) setLoading(false);
    }
  }, [currentRequest, key]);

  useEffect(() => {
    if (!enabled) { setLoading(false); return undefined; }
    setError(null);
    void refresh();
    return () => {
      version.current += 1;
    };
  }, [enabled, refresh]);

  return { series: snapshot?.key === key ? snapshot.series : [],
    loading: readKey === key ? loading : enabled, error: readKey === key ? error : null, refresh };
}
