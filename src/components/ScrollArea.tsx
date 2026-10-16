import { useId, useImperativeHandle, useRef, type HTMLAttributes, type Ref, type UIEventHandler } from "react";
import { BackToTop } from "./BackToTop";

interface Props extends Omit<HTMLAttributes<HTMLElement>, "onScroll"> {
  as?: "div" | "main" | "section";
  /** Reading-position owners reference the actual scrolling element. */
  viewportRef?: Ref<HTMLDivElement>;
  onScroll?: UIEventHandler<HTMLDivElement>;
}

/** Layout belongs to the fixed frame; overflow and keyboard scrolling belong
 * to its viewport. The sibling action never travels with the content. */
export function ScrollArea({ as: Tag = "div", className, viewportRef, onScroll,
  tabIndex = -1, children, ...props }: Props) {
  const viewport = useRef<HTMLDivElement>(null);
  const id = useId();
  useImperativeHandle(viewportRef, () => viewport.current!, []);
  return (
    <Tag {...props} className={["asb-scroll-area", className].filter(Boolean).join(" ")}>
      <div id={id} className="asb-scroll-viewport" ref={viewport} tabIndex={tabIndex}
        aria-label={props["aria-label"]} onScroll={onScroll}>
        {children}
      </div>
      <BackToTop viewport={viewport} controls={id} />
    </Tag>
  );
}
