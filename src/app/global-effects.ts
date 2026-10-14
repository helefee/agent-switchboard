import { useEffect } from "react";
import { toggleDevtools } from "../api/client";
import { isBrowserDevelopment } from "../lib/runtime";

/** Dev-build debug affordance: F12 toggles the WebView inspector. Absent
 * from production bundles, where the WebView inspector is disabled. */
export function useDevtoolsShortcut(): void {
  useEffect(() => {
    if (!import.meta.env.DEV || isBrowserDevelopment) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "F12") {
        event.preventDefault();
        void toggleDevtools().catch(() => {});
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}

const SCROLL_IDLE_MS = 900;

/** Marks each scroll surface with `data-scrolling` while its content is in
 * motion, so the ink-line scrollbar (root.css) stays hidden at rest and
 * reappears while scrolling. `scroll` events do not bubble, but the capture
 * phase reaches every surface from this single window listener. */
export function useScrollActivityMarker(): void {
  useEffect(() => {
    const idleTimers = new Map<Element, number>();
    const onScroll = (event: Event) => {
      // Document-level scrolls report `document`; the root element owns the bar.
      const surface = event.target === document ? document.scrollingElement : event.target;
      if (!(surface instanceof Element)) return;
      surface.setAttribute("data-scrolling", "");
      const pending = idleTimers.get(surface);
      if (pending !== undefined) window.clearTimeout(pending);
      idleTimers.set(surface, window.setTimeout(() => {
        idleTimers.delete(surface);
        surface.removeAttribute("data-scrolling");
      }, SCROLL_IDLE_MS));
    };
    window.addEventListener("scroll", onScroll, { capture: true, passive: true });
    return () => {
      window.removeEventListener("scroll", onScroll, { capture: true });
      for (const timeout of idleTimers.values()) window.clearTimeout(timeout);
      idleTimers.clear();
    };
  }, []);
}

/** Tracks input modality for keyboard-only action affordances. */
export function useKeyboardFocusMarker(): void {
  useEffect(() => {
    const root = document.documentElement;
    const markKeyboard = (event: KeyboardEvent) => {
      if (event.key === "Tab" || event.key === " " || event.key.startsWith("Arrow")) {
        root.dataset.focusSource = "key";
      }
    };
    const markPointer = () => {
      delete root.dataset.focusSource;
    };
    window.addEventListener("keydown", markKeyboard);
    window.addEventListener("pointerdown", markPointer, true);
    return () => {
      window.removeEventListener("keydown", markKeyboard);
      window.removeEventListener("pointerdown", markPointer, true);
    };
  }, []);
}
