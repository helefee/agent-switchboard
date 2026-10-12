import { useMessageState } from "../i18n/use-message-state";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "../components/Button";
import { ClientFilter, type ClientFilterValue } from "../components/ClientFilter";
import { GatewayTopology } from "../components/GatewayTopology";
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
  const { status, error, retrying, refreshedAt, refresh, retryBind, discardRecovery } = useGatewayRuntime(active);
  const profileNames = useMemo(() => {
    const names = new Map<string, string>();
    for (const profile of profiles) names.set(profile.id, profile.name);
    return names;
  }, [profiles]);

  if (error) return <GatewayUnavailable error={error} onRetry={refresh} />;
  if (!status) return <GatewayLoading />;

  const failed = FAILURE_STATES.has(status.status);
  return (
    <GatewayPanel controls={<ClientFilter value={focus} onChange={setFocus} label={t("gateway.focusAriaLabel")} showLogos />}>
      <GatewayTopology
        status={status}
        failed={failed}
        refreshedAt={refreshedAt}
        retrying={retrying}
        onRetry={retryBind}
        onChangePort={() => setPortSheet({ stage: "input" })}
        onDiscardRecovery={discardRecovery}
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
  const refresh = useCallback(async () => {
    try {
      setStatus(await getGatewayStatus());
      setRefreshedAt(Date.now());
      setError(null);
    } catch (cause) {
      setError(cause);
    }
  }, []);

  useEffect(() => {
    if (!active) return;
    void refresh();
    const timer = window.setInterval(() => void refresh(), POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [active, refresh]);

  const retryBind = useCallback(async () => {
    setRetrying(true);
    try {
      setStatus(await retryGatewayBind());
      setRefreshedAt(Date.now());
      setError(null);
    } catch (cause) {
      setError(cause);
    } finally {
      setRetrying(false);
    }
  }, []);

  const discardRecovery = useCallback(async () => {
    try {
      setStatus(await discardGatewayPortChange(true));
      setRefreshedAt(Date.now());
      setError(null);
    } catch (cause) {
      setError(cause);
    }
  }, []);

  return { status, error, retrying, refreshedAt, refresh, retryBind, discardRecovery };
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
