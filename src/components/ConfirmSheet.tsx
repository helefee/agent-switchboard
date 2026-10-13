import type { ReactNode } from "react";

import { useI18n } from "../i18n";
import { Button } from "./Button";
import { AppDialog } from "./AppDialog";

interface Props {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  busy?: boolean;
  confirmDisabled?: boolean;
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Confirmation sheet for irreversible actions. Escape cancels; focus starts
 * on the cancel button so a stray Enter cannot confirm by accident.
 */
export function ConfirmSheet({
  title,
  children,
  confirmLabel,
  cancelLabel,
  busy = false,
  confirmDisabled = false,
  destructive = false,
  onConfirm,
  onCancel,
}: Props) {
  const { t } = useI18n();
  return (
    <AppDialog title={title} busy={busy} narrow onClose={onCancel} footer={
      <>
          <Button autoFocus variant="secondary" disabled={busy} onClick={onCancel}>
            {cancelLabel ?? t("confirm.cancel")}
          </Button>
          <Button
            variant={destructive ? "danger" : "primary"}
            disabled={busy || confirmDisabled}
            onClick={onConfirm}
          >
            {busy ? t("shell.busy") : confirmLabel}
          </Button>
      </>
    }>{children}</AppDialog>
  );
}
