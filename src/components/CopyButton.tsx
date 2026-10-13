import { useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n";
import { Button } from "./Button";
import { CheckIcon, RestoreIcon } from "./icons";

/** One visible clipboard result for code, device codes and generated answers. */
export function CopyButton({ content }: { content: string }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<"idle" | "copying" | "copied" | "failed">("idle");
  const locked = useRef(false);
  const revision = useRef(0);
  useEffect(() => { revision.current += 1; setStatus("idle"); }, [content]);
  useEffect(() => () => { revision.current += 1; }, []);
  const copy = async () => {
    if (locked.current) return;
    locked.current = true;
    const current = revision.current;
    setStatus("copying");
    try {
      await navigator.clipboard.writeText(content);
      if (current === revision.current) setStatus("copied");
    } catch {
      if (current === revision.current) setStatus("failed");
    } finally { locked.current = false; }
  };
  return <>
    <Button variant="secondary" disabled={status === "copying"} onClick={() => void copy()}>
      {status === "copied" ? <CheckIcon /> : <RestoreIcon />}
      {t(status === "copied" ? "common.copied" : "common.copy")}
    </Button>
    {status === "failed" && <span className="asb-field-error" role="alert">{t("common.copyFailed")}</span>}
  </>;
}
