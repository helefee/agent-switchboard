import { useEffect, useState } from "react";
import type { GatewayStatus, GatewayStatusKind } from "../api/client";
import { useI18n, type MessageKey } from "../i18n";
import { Button } from "./Button";
import { GatewayDeviceArt } from "./GatewayDeviceArt";
import { CopyGatewayAddressButton } from "./GatewayPagePrimitives";

const STATUS_KEYS: Record<GatewayStatusKind, MessageKey> = {
  standby: "gateway.status.standby",
  running: "gateway.status.running",
  portConflict: "gateway.status.portConflict",
  bindRejected: "gateway.status.bindRejected",
  needsRepair: "gateway.status.needsRepair",
  recoveryBlocked: "gateway.status.recoveryBlocked",
};

interface Props {
  status: GatewayStatus;
  failed: boolean;
  refreshedAt: number | null;
  retrying: boolean;
  onRetry: () => void;
  onChangePort: () => void;
  rows: number;
}

export function GatewayHub({ status, failed, refreshedAt, retrying, onRetry, onChangePort, rows }: Props) {
  const { t } = useI18n();
  return (
    <section data-gateway-node="hub" className={`asb-gateway-hub${failed ? " is-failure" : ""}`} style={{ gridRow: `1 / span ${rows}` }}>
      <div className="asb-gateway-hub-identity">
        <GatewayDeviceArt kind="hub" />
        <div className="asb-gateway-hub-copy">
          <p className="asb-gateway-kicker">{t("gateway.title")}</p>
          <p className="asb-gateway-hub-state">
            <span className={`asb-gateway-lamp${failed ? " is-failure" : status.status === "running" ? " is-running" : ""}`} aria-hidden="true" />
            {t(STATUS_KEYS[status.status])}
          </p>
          <GatewayUpdatedAgo at={refreshedAt} />
        </div>
      </div>
      <div className="asb-gateway-hub-address">
        <p className="asb-gateway-kicker">{t("gateway.loopbackKicker")}</p>
        <div className={`asb-gateway-plate${status.baseUrl ? "" : " is-unavailable"}`}>
          <span className="asb-gateway-plate-address asb-code">{status.baseUrl ?? t("gateway.addressNotListening")}</span>
          {status.baseUrl && <CopyGatewayAddressButton value={status.baseUrl} />}
        </div>
      </div>
      {status.failure && (
        <div className="asb-gateway-hub-failure" role="alert">
          {status.status === "portConflict" && <p className="asb-gateway-hub-message">{t("gateway.portConflictHelp")}</p>}
          {status.status === "bindRejected" && <p className="asb-gateway-hub-message">{t("gateway.bindRejectedHelp")}</p>}
          <p className="asb-gateway-hub-detail">{t("gateway.failureDetail", { reason: status.failure.message })}</p>
        </div>
      )}
      <dl className="asb-fact-row">
        <div><dt>{t("gateway.configuredPort")}</dt><dd className="asb-num">{status.configuredPort}</dd></div>
        <div><dt>{t("gateway.actualPort")}</dt><dd className="asb-num">{status.listeningPort ?? t("gateway.portNotListening")}</dd></div>
      </dl>
      <div className="asb-gateway-hub-actions">
        {status.failure && (
          <Button variant="secondary" disabled={retrying} onClick={onRetry}>{t("gateway.retryListen")}</Button>
        )}
        <Button variant="secondary" disabled={status.status === "recoveryBlocked"} onClick={onChangePort}>{t("gateway.changePort")}</Button>
      </div>
    </section>
  );
}

/** Only the timestamp rerenders each second; the topology follows snapshots. */
function GatewayUpdatedAgo({ at }: { at: number | null }) {
  const { t } = useI18n();
  const [, setTick] = useState(0);
  useEffect(() => {
    if (at === null) return;
    const timer = window.setInterval(() => setTick((value) => value + 1), 1000);
    return () => window.clearInterval(timer);
  }, [at]);
  if (at === null) return null;
  const seconds = Math.max(0, Math.round((Date.now() - at) / 1000));
  const text = seconds < 5
    ? t("gateway.updatedJustNow")
    : seconds < 60
      ? t("gateway.updatedSecondsAgo", { seconds })
      : t("gateway.updatedMinutesAgo", { minutes: Math.floor(seconds / 60) });
  return <p className="asb-gateway-hub-updated">{text}</p>;
}
