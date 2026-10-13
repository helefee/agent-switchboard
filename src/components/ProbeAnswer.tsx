import { useMemo, useState } from "react";
import type { CodexProbeGradingVersion } from "../api/client";
import { useI18n } from "../i18n";
import { createProbeHtmlPreview } from "../lib/probe-html";
import { extractProbeSvg } from "../lib/probe-svg";
import { AppDialog } from "./AppDialog";
import { Button } from "./Button";
import { CodePreview } from "./CodePreview";
import { CopyButton } from "./CopyButton";

function VisualAnswer({ source, html, seq, onError }: {
  source: string; html: boolean; seq: number; onError: () => void;
}) {
  const { t } = useI18n();
  return html ? <iframe className="asb-probe-html" srcDoc={source} sandbox="allow-scripts"
    referrerPolicy="no-referrer" loading="lazy" title={t("codex.probe.html.title", { seq })} />
    : <img className="asb-probe-svg" src={source} alt={t("codex.probe.svg.alt", { seq })} onError={onError} />;
}

export function ProbeAnswer({ answer, gradingVersion, seq }: {
  answer: string | null;
  gradingVersion: CodexProbeGradingVersion;
  seq: number;
}) {
  const { t } = useI18n();
  const source = useMemo(() => {
    if (!answer || gradingVersion === "1") return null;
    if (gradingVersion === "html-visual-v1") return createProbeHtmlPreview(answer);
    const svg = extractProbeSvg(answer);
    return svg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` : null;
  }, [answer, gradingVersion]);
  const [failedSource, setFailedSource] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);
  if (!answer) return <>—</>;
  if (gradingVersion === "1") return <>{answer}</>;
  const html = gradingVersion === "html-visual-v1";
  return (
    <div className={`asb-probe-answer${html ? " asb-probe-answer--html" : ""}`}>
      <div className="asb-panel-actions">
        <Button variant="secondary" onClick={() => setExpanded(true)}>{t("common.expandPreview")}</Button>
        <CopyButton content={answer} />
      </div>
      {source && (html || failedSource !== source) ? (
        !expanded && <VisualAnswer source={source} html={html} seq={seq} onError={() => setFailedSource(source)} />
      ) : (
        <p className="asb-warn-text" role="status">
          {t(html ? "codex.probe.html.invalid" : source ? "codex.probe.svg.loadError" : "codex.probe.svg.invalid")}
        </p>
      )}
      <details className="asb-probe-answer-source">
        <summary>{t("codex.probe.visual.source")}</summary>
        <pre>{answer}</pre>
      </details>
      {expanded && <AppDialog wide title={t(html ? "codex.probe.html.title" : "codex.probe.svg.alt", { seq })}
        busy={false} onClose={() => setExpanded(false)}>
        <div className="asb-probe-answer-viewer">
          {source && (html || failedSource !== source) &&
            <VisualAnswer source={source} html={html} seq={seq} onError={() => setFailedSource(source)} />}
          <CodePreview target={t("codex.probe.visual.source")} content={answer} />
        </div>
      </AppDialog>}
    </div>
  );
}
