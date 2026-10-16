import { useEffect, useRef, useState } from "react";
import { hideTray, switchTrayProvider, type TraySnapshot } from "../api/client";
import { queryCodexOfficialQuota, queryProfileUsage } from "../api/usage";
import { useMessageState } from "../i18n/use-message-state";

type PendingAction = { kind: "switch" | "refresh"; providerId: string } | { kind: "panel" };

/** All tray commands share one serial action and structured error owner. */
export function useTrayActions(refresh: () => Promise<void>) {
  const [error, setError] = useMessageState();
  const [pending, setPending] = useState<PendingAction | null>(null);
  const actionInFlight = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void hideTray().catch((caught) => { if (mounted.current) setError(caught); });
      }
    };
    document.addEventListener("keydown", onKey);
    return () => { mounted.current = false; document.removeEventListener("keydown", onKey); };
  }, [setError]);
  const run = async (mark: PendingAction, action: () => Promise<void>) => {
    if (actionInFlight.current) return;
    actionInFlight.current = true;
    setPending(mark);
    setError(null);
    try { await action(); }
    catch (caught) { if (mounted.current) setError(caught); }
    finally {
      actionInFlight.current = false;
      if (mounted.current) setPending(null);
    }
  };
  const switchProvider = (providerId: string) => {
    void run({ kind: "switch", providerId }, async () => {
      await switchTrayProvider(providerId);
      await refresh();
    });
  };
  const refreshUsage = (provider: TraySnapshot["providers"][number]) => {
    void run({ kind: "refresh", providerId: provider.id }, async () => {
      if (provider.usage?.kind === "official") await queryCodexOfficialQuota(provider.id);
      else await queryProfileUsage(provider.app, provider.id);
      await refresh();
    });
  };
  return { error, setError, pending, run, switchProvider, refreshUsage };
}
