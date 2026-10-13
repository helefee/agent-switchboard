import { useState, type ReactNode } from "react";
import { useI18n } from "../i18n";
import { Button } from "./Button";
import { ModuleHeader } from "./WorkspaceHeader";
import { ConfirmSheet } from "./ConfirmSheet";
import { CopyButton } from "./CopyButton";

export function ConfirmGatewayRecoveryDiscard({ onConfirm, busy }: { onConfirm: () => Promise<void>; busy: boolean }) {
  const { t } = useI18n();
  const [confirming, setConfirming] = useState(false);
  if (!confirming) {
    return (
      <div>
        <Button variant="secondary" disabled={busy} onClick={() => setConfirming(true)}>
          {t("gateway.discardRecordButton")}
        </Button>
      </div>
    );
  }
  return (
    <ConfirmSheet title={t("gateway.discardRecordButton")} busy={busy} destructive
      confirmLabel={t("gateway.discardConfirmButton")} onCancel={() => setConfirming(false)}
      onConfirm={() => void onConfirm().finally(() => setConfirming(false))}>
      <span className="asb-gateway-guidance-copy">
        {t("gateway.discardConfirmCopy")}
      </span>
    </ConfirmSheet>
  );
}

export function CopyGatewayAddressButton({ value }: { value: string }) {
  return <CopyButton content={value} />;
}

/** Status and recovery precede topology, charts and recent requests. */
export function GatewayPanel({ children, controls }: { children: ReactNode; controls?: ReactNode }) {
  const { t } = useI18n();
  return (
    <section className="asb-panel asb-gateway-panel" aria-label={t("gateway.panelAriaLabel")}>
      <ModuleHeader title={t("gateway.title")} primaryActions={controls} />
      <div className="asb-gateway-stack">{children}</div>
    </section>
  );
}
