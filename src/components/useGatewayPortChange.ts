import { useEffect, useRef, useState } from "react";
import { cancelGatewayPortChange, commitGatewayPortChange, prepareGatewayPortChange } from "../api/client";
import { errorText, uiMessage } from "../i18n/errors";
import { tr } from "../i18n/current";
import { toast, toastMessage } from "./use-toast";
import type { PortSheetState } from "./GatewayPortChangeSheet";

export const MIN_PORT = 1024;
export const MAX_PORT = 65535;

function release(preparationId: string) {
  void cancelGatewayPortChange(preparationId).catch((error) => toast({ kind: "error",
    title: toastMessage("gateway.releaseFailed"), description: errorText(error, tr) }));
}

/** A preparation is released on exit; commit consumes it even on failure. */
export function useGatewayPortChange(state: PortSheetState, configuredPort: number,
  onState: (state: PortSheetState | null) => void, onRefreshed: () => void) {
  const [busy, setBusy] = useState(false);
  const locked = useRef(false);
  const alive = useRef(true);
  const reservation = useRef(state.plan?.preparationId ?? null);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (reservation.current) release(reservation.current);
      reservation.current = null;
    };
  }, []);
  const prepare = async () => {
    if (locked.current) return;
    const port = Number(state.port ?? "");
    if (!Number.isInteger(port) || port < MIN_PORT || port > MAX_PORT) {
      onState({ ...state, error: uiMessage("gateway.errorPortRange", { min: MIN_PORT, max: MAX_PORT }) });
      return;
    }
    if (port === configuredPort) {
      onState({ ...state, error: uiMessage("gateway.errorSamePort") });
      return;
    }
    locked.current = true;
    setBusy(true);
    try {
      const plan = await prepareGatewayPortChange(port);
      if (!alive.current) { release(plan.preparationId); return; }
      reservation.current = plan.preparationId;
      onState({ stage: "preview", port: state.port, plan });
    } catch (error) {
      if (alive.current) onState({ ...state, error });
    } finally {
      locked.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const commit = async () => {
    if (locked.current || !state.plan) return;
    locked.current = true;
    setBusy(true);
    reservation.current = null;
    try {
      const result = await commitGatewayPortChange(state.plan.preparationId, true);
      onRefreshed();
      if (alive.current) onState({ stage: "done", resultToPort: result.toPort, warnings: result.warnings });
    } catch (error) {
      if (alive.current) onState({ stage: "input", port: state.port, error });
    } finally {
      locked.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const cancel = () => {
    if (locked.current) return;
    onState(null);
  };
  return { busy, prepare, commit, cancel };
}
