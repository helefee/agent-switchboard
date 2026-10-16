import { useLayoutEffect, useRef, type UIEvent } from "react";

export function useReadingScroll(position: { current: number }) {
  const container = useRef<HTMLDivElement>(null);
  const restoring = useRef(false);
  useLayoutEffect(() => {
    const node = container.current;
    if (!node || position.current === 0) return;
    const surface = node.parentElement;
    if (!surface) return;
    const saved = position.current;
    restoring.current = true;
    const stop = () => {
      restoring.current = false;
      observer.disconnect();
      position.current = node.scrollTop;
    };
    const restore = () => {
      if (!restoring.current || node.querySelector('[aria-busy="true"]')) return;
      node.scrollTop = saved;
      stop();
    };
    const observer = new MutationObserver(restore);
    observer.observe(node, { childList: true, subtree: true, attributes: true, attributeFilter: ["aria-busy"] });
    const gestures = ["wheel", "pointerdown", "touchstart", "keydown"] as const;
    // The fixed back-to-top action is a sibling of the viewport in ScrollArea.
    for (const gesture of gestures) surface.addEventListener(gesture, stop, { passive: true });
    restore();
    return () => {
      observer.disconnect(); restoring.current = false;
      for (const gesture of gestures) surface.removeEventListener(gesture, stop);
    };
  }, [position]);
  const onScroll = (event: UIEvent<HTMLDivElement>) => {
    if (!restoring.current) position.current = event.currentTarget.scrollTop;
  };
  return { container, onScroll };
}
