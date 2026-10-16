import { useCallback, useRef, useState } from "react";
import { repairApplicationData, type ApplicationDataRepairReport } from "../api/data-repair";
import type { CommandError } from "../api/shared";
import { useI18n } from "../i18n";
import { commandErrorText } from "../i18n/errors";
import { toast, toastMessage } from "../components/use-toast";

function RepairDetails({ report }: { report: ApplicationDataRepairReport }) {
  const { t } = useI18n();
  return <>
    {report.backupPaths.map((path) => <div key={path}>{t("operations.repairData.backup", { path })}</div>)}
    {report.issues.map((issue, index) => <div key={index}>{commandErrorText(issue, t)}</div>)}
  </>;
}

/** One global repair transaction with explicit partial results and a revision
 * for mounted settings consumers to reload after application-owned writes. */
export function useApplicationDataRepair({ busy, setBusy, clearError, onError, invalidateCandidates, refresh, onProvidersRepaired }: {
  busy: boolean;
  setBusy: (busy: boolean) => void;
  clearError: () => void;
  onError: (error: CommandError) => void;
  invalidateCandidates: () => void;
  refresh: () => Promise<void>;
  onProvidersRepaired: () => void;
}) {
  const [revision, setRevision] = useState(0);
  const locked = useRef(false);
  const run = useCallback(async () => {
    if (busy || locked.current) return;
    locked.current = true;
    invalidateCandidates(); setBusy(true); clearError();
    try {
      const report = await repairApplicationData();
      if (report.repairedFiles.some((path) => path.startsWith("configuration/"))) onProvidersRepaired();
      setRevision((value) => value + 1);
      await refresh();
      if (report.issues.length) onError(report.issues[0]);
      toast({ kind: report.issues.length ? "warning" : "success", durationMs: 10000,
        title: toastMessage(report.issues.length ? "operations.repairData.partial" : "operations.repairData.done",
          { count: report.repairedFiles.length }), description: <RepairDetails report={report} /> });
    } catch (caught) { onError(caught as CommandError); }
    finally { locked.current = false; setBusy(false); }
  }, [busy, setBusy, clearError, onError, invalidateCandidates, refresh, onProvidersRepaired]);
  return { revision, run };
}
