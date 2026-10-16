import { useLayoutEffect, useRef, type RefObject } from "react";
import { resizeTray, trayReady } from "../api/client";

/** The page reports content size; native configuration and work area own bounds. */
export function useTrayPanelLayout(
  panel: RefObject<HTMLDivElement | null>,
  list: RefObject<HTMLDivElement | null>,
  initialized: boolean,
  recoveryError: string | null,
  onError: (error: unknown) => void,
) {
  const readySent = useRef(false);
  useLayoutEffect(() => {
    let disposed = false;
    let revision = 0;
    let lastHeight = 0;
    const measure = () => {
      if (!panel.current || !list.current) return;
      const height = Math.ceil(panel.current.offsetHeight - list.current.clientHeight + list.current.scrollHeight);
      if (height <= 0 || height === lastHeight) return;
      lastHeight = height;
      const request = ++revision;
      void (async () => {
        try { await resizeTray(height); }
        catch (caught) { if (!disposed && request === revision) onError(caught); }
        finally {
          // Even a failed resize must reveal the recovery controls. Only the
          // latest layout can complete readiness; failed handshakes stay retryable.
          if (!disposed && request === revision && initialized && !readySent.current) {
            try { await trayReady(); readySent.current = true; }
            catch (caught) { if (!disposed) onError(caught); }
          }
        }
      })();
    };
    measure();
    const observer = new ResizeObserver(measure);
    if (panel.current) observer.observe(panel.current);
    if (list.current) observer.observe(list.current);
    if (list.current?.firstElementChild) observer.observe(list.current.firstElementChild);
    return () => { disposed = true; observer.disconnect(); };
  }, [panel, list, initialized, recoveryError, onError]);
}
