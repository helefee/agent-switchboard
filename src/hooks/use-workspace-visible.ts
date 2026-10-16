import { useSyncExternalStore } from "react";

const listeners = new Set<() => void>();
const notify = () => listeners.forEach((listener) => listener());

function subscribe(listener: () => void) {
  if (listeners.size === 0) document.addEventListener("visibilitychange", notify);
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) document.removeEventListener("visibilitychange", notify);
  };
}
const snapshot = () => document.visibilityState === "visible";

export function useWorkspaceVisible(active: boolean): boolean {
  const visible = useSyncExternalStore(subscribe, snapshot, () => true);
  return active && visible;
}
