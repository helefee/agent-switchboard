import { useMessageState } from "../i18n/use-message-state";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../components/Button";
import { ClientFilter, type ClientFilterValue } from "../components/ClientFilter";
import { GatewayGuidance, GatewayTopology } from "../components/GatewayTopology";
import { GatewayHub } from "../components/GatewayHub";
import { GatewayPortChangeSheet, type PortSheetState } from "../components/GatewayPortChangeSheet";
import { GatewayTelemetry } from "../components/GatewayTelemetry";
import { GatewayPanel } from "../components/GatewayPagePrimitives";
import { GatewayIcon } from "../components/icons";
import { useI18n } from "../i18n";
import {
  discardGatewayPortChange,
  getGatewayStatus,
  retryGatewayBind,
  type GatewayStatus,
  type GatewayStatusKind,
  type ProviderProfile,
} from "../api/client";

const POLL_INTERVAL_MS = 5000;
const FAILURE_STATES: ReadonlySet<GatewayStatusKind> = new Set([
  "portConflict",
  "bindRejected",
  "needsRepair",
  "recoveryBlocked",
]);

interface Props {
  /** False while another page is shown; polling runs only when visible. */
  active: boolean;
  profiles: ProviderProfile[];
}

/** Runtime snapshots and transient focus share one page owner. */
export function GatewayPage({ active, profiles }: Props) {
  const { t } = useI18n();
  const [focus, setFocus] = useState<ClientFilterValue>("all");
  const [portSheet, setPortSheet] = useState<PortSheetState | null>(null);
  // The sheet is portaled over the whole window; leaving the gateway context
  // (settings section or workspace) must close it so it never floats over an
  // unrelated page. The flow hook releases any held preparation on unmount.
  useEffect(() => {
    if (!active) setPortSheet(null);
  }, [active]);
  const { status, error, retrying, refreshedAt, refresh, retryBind, discardRecovery } = useGatewayRuntime(active);
  const profileNames = useMemo(() => {
    const names = new Map<string, string>();
    for (const profile of profiles) names.set(profile.id, profile.name);
    return names;
  }, [profiles]);

  if (error && !status) return <GatewayUnavailable error={error} onRetry={refresh} />;
  if (!status) return <GatewayLoading />;

  const failed = FAILURE_STATES.has(status.status);
  return (
    <GatewayPanel controls={<ClientFilter value={focus} onChange={setFocus} label={t("gateway.focusAriaLabel")} showLogos />}>
      {error && <p className="asb-fail-text" role="alert">{error}</p>}
      <GatewayHub status={status} failed={failed} refreshedAt={refreshedAt} retrying={retrying} active={active}
        onRetry={retryBind} onChangePort={() => setPortSheet({ stage: "input" })} />
      <GatewayGuidance status={status} busy={retrying} onDiscardRecovery={discardRecovery} />
      <GatewayTopology
        status={status}
        failed={failed}
        profileNames={profileNames}
        focus={focus}
        onFocus={(app) => setFocus((current) => current === app ? "all" : app)}
      />
      <GatewayTelemetry metrics={status.metrics} profileNames={profileNames} focus={focus} />
      {portSheet && (
        <GatewayPortChangeSheet
          state={portSheet}
          configuredPort={status.configuredPort}
          onState={setPortSheet}
          onRefreshed={() => void refresh()}
        />
      )}
    </GatewayPanel>
  );
}

function useGatewayRuntime(active: boolean) {
  const [status, setStatus] = useState<GatewayStatus | null>(null);
  const [refreshedAt, setRefreshedAt] = useState<number | null>(null);
  const [error, setError] = useMessageState();
  const [retrying, setRetrying] = useState(false);
  const mutation = useRef(false);
  const generation = useRef(0);
  const refresh = useCallback(async () => {
    if (mutation.current) return;
    const request = ++generation.current;
    try {
      const snapshot = await getGatewayStatus();
      if (request !== generation.current) return;
      setStatus(snapshot);
      setRefreshedAt(Date.now());
      setError(null);
    } catch (cause) {
      if (request === generation.current) setError(cause);
    }
  }, []);

  useEffect(() => {
    if (!active) return;
    void refresh();
    const timer = window.setInterval(() => void refresh(), POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [active, refresh]);

  const mutate = useCallback(async (action: () => Promise<GatewayStatus>) => {
    if (mutation.current) return;
    mutation.current = true;
    generation.current += 1;
    setRetrying(true);
    try {
      setStatus(await action());
      setRefreshedAt(Date.now());
      setError(null);
    } catch (cause) {
      setError(cause);
    } finally {
      mutation.current = false;
      setRetrying(false);
    }
  }, []);

  return { status, error, retrying, refreshedAt, refresh,
    retryBind: () => mutate(retryGatewayBind), discardRecovery: () => mutate(() => discardGatewayPortChange(true)) };
}

function GatewayUnavailable({ error, onRetry }: { error: string; onRetry: () => void }) {
  const { t } = useI18n();
  return (
    <GatewayPanel>
      <div className="asb-gateway-unavailable" role="alert">
        <GatewayIcon />
        <div>
          <p className="asb-gateway-unavailable-title">{t("gateway.unavailableTitle")}</p>
          <p className="asb-gateway-error">{error}</p>
        </div>
        <Button variant="secondary" onClick={() => void onRetry()}>{t("gateway.retry")}</Button>
      </div>
    </GatewayPanel>
  );
}

/* Loading reserves the new page's footprint (map, strip, recent requests). */
function GatewayLoading() {
  const { t } = useI18n();
  return (
    <GatewayPanel>
      <div className="asb-gateway-loading" role="status" aria-label={t("gateway.loadingAriaLabel")}>
        <span className="asb-skeleton" aria-hidden="true" />
        <span className="asb-skeleton" aria-hidden="true" />
        <span className="asb-skeleton" aria-hidden="true" />
      </div>
    </GatewayPanel>
  );
}
