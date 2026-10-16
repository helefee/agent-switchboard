import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { useI18n } from "../i18n";
import { Button } from "./Button";
import { BackToTopIcon } from "./icons";
import { Tooltip } from "./Tooltip";

/** The fixed action belongs to one native scroll viewport, never the window. */
export function BackToTop({ viewport, controls }: {
  viewport: RefObject<HTMLDivElement | null>;
  controls: string;
}) {
  const { t } = useI18n();
  const button = useRef<HTMLButtonElement>(null);
  const [visible, setVisible] = useState(false);
  const sync = useCallback(() => {
    const node = viewport.current;
    if (!node) return;
    const overflow = getComputedStyle(node).overflowY;
    const next = node.clientHeight > 0 && node.scrollTop > 0
      && node.scrollHeight > node.clientHeight && (overflow === "auto" || overflow === "scroll");
    if (!next && node.clientHeight > 0 && document.activeElement === button.current) {
      node.focus({ preventScroll: true });
    }
    setVisible(next);
  }, [viewport]);
  // Route visibility and overflow can change without resizing the viewport.
  useLayoutEffect(sync);
  useLayoutEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const resize = new ResizeObserver(sync);
    resize.observe(node);
    node.addEventListener("scroll", sync, { passive: true });
    return () => {
      resize.disconnect();
      node.removeEventListener("scroll", sync);
    };
  }, [viewport, sync]);
  if (!visible) return null;
  return (
    <div className="asb-back-to-top" aria-live="off">
      <Tooltip label={t("common.backToTop")}>
        <Button ref={button} variant="icon" aria-label={t("common.backToTop")} aria-controls={controls}
          onClick={() => {
            const node = viewport.current;
            if (!node) return;
            const duration = Number.parseFloat(getComputedStyle(node).getPropertyValue("--asb-motion-fast"));
            node.focus({ preventScroll: true });
            node.scrollTo({ top: 0, left: node.scrollLeft, behavior: duration === 0 ? "instant" : "smooth" });
          }}>
          <BackToTopIcon />
        </Button>
      </Tooltip>
    </div>
  );
}
