import { useCallback, useEffect, useRef, useState } from "react";
import { cancelMcpCheck, checkMcpConnection, getMcpCheck } from "../../api/client";
import type { CommandError, ExtensionTarget } from "../../api/client";
import { useMessageState } from "../../i18n/use-message-state";

interface CheckSubject { definitionId: string; definitionRevision: number; target: ExtensionTarget }
interface CheckRun extends CheckSubject { checkId: string | null; cancelRequested: boolean }
type CheckPhase = "starting" | "running" | "cancelling";

async function driveCheck(run: CheckRun, mounted: () => boolean, running: () => void) {
  const started = await checkMcpConnection(run.definitionId, run.definitionRevision, run.target, true);
  run.checkId = started.checkId;
  if (run.cancelRequested || !mounted()) await cancelMcpCheck(run.checkId);
  if (!mounted()) return;
  running();
  while (mounted()) {
    const status = await getMcpCheck(run.checkId);
    if (status.state === "completed") return;
    await new Promise<void>((resolve) => window.setTimeout(resolve, 300));
  }
}

/** The workspace owns one active probe; closing its dialog cancels it while
 * the owner keeps polling through the saved terminal result and refresh. */
export function useMcpConnectionCheck(
  managementDefinitionId: string | null,
  refresh: () => Promise<unknown>,
  onError: (error: CommandError) => void,
) {
  const mounted = useRef(true);
  const active = useRef<CheckRun | null>(null);
  const latest = useRef({ refresh, onError });
  latest.current = { refresh, onError };
  const [subject, setSubject] = useState<CheckSubject | null>(null);
  const [phase, setPhase] = useState<CheckPhase | null>(null);
  const [failure, setFailure] = useMessageState();
  const cancel = useCallback(async () => {
    const run = active.current;
    if (!run) return;
    run.cancelRequested = true;
    if (mounted.current) setPhase("cancelling");
    if (run.checkId) {
      try { await cancelMcpCheck(run.checkId); }
      catch (error) {
        if (mounted.current) setFailure(error);
        else latest.current.onError(error as CommandError);
      }
    }
  }, [setFailure]);
  const start = useCallback(async (next: CheckSubject) => {
    if (active.current) return;
    const run: CheckRun = { ...next, checkId: null, cancelRequested: false };
    active.current = run;
    setSubject(next);
    setFailure(null);
    setPhase("starting");
    try {
      await driveCheck(run, () => mounted.current, () => setPhase(run.cancelRequested ? "cancelling" : "running"));
    } catch (error) {
      await cancel();
      if (mounted.current) setFailure(error);
      else latest.current.onError(error as CommandError);
    } finally {
      try { if (mounted.current) await latest.current.refresh(); }
      finally {
        active.current = null;
        if (mounted.current) setPhase(null);
      }
    }
  }, [cancel, setFailure]);
  useEffect(() => {
    if (active.current && active.current.definitionId !== managementDefinitionId) void cancel();
  }, [managementDefinitionId, cancel]);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; void cancel(); };
  }, [cancel]);
  return { subject, phase, failure, start, cancel };
}

export type McpConnectionCheck = ReturnType<typeof useMcpConnectionCheck>;
