import { useState } from "react";
import { useI18n } from "../i18n";
import { ConfirmSheet } from "./ConfirmSheet";

/** The editor supplies dirtiness; this hook owns only a pending navigation action. */
export function useUnsavedChanges(dirty: boolean, busy = false) {
  const { t } = useI18n();
  const [pending, setPending] = useState<{ run: () => void } | null>(null);
  const request = (run: () => void) => {
    if (busy) return;
    if (dirty) setPending({ run });
    else run();
  };
  const confirmation = pending && <ConfirmSheet title={t("common.unsavedTitle")} busy={busy}
    confirmLabel={t("common.discardAndContinue")} cancelLabel={t("common.keepEditing")} destructive
    onCancel={() => setPending(null)} onConfirm={() => { setPending(null); pending.run(); }}>
    <p>{t("common.unsavedBody")}</p>
  </ConfirmSheet>;
  return { request, confirmation };
}
