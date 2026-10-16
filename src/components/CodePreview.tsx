import { useId, useState, type ReactNode } from "react";
import { useI18n } from "../i18n";
import { CopyButton } from "./CopyButton";
import { Button } from "./Button";
import { ScrollArea } from "./ScrollArea";

/**
 * The single owner of code-preview file rendering (DESIGN.md §8): the
 * numbered file view with light token coloring. Change-row diffs are owned
 * by DiffView; every preview surface consumes these components instead of
 * rebuilding the markup.
 */

/** Boolean and numeric literals get their own color; keys are muted so the
    values the overlay writes stay the visual anchor. */
function renderValue(text: string): ReactNode {
  const trimmed = text.trim();
  if (trimmed === "true" || trimmed === "false") {
    return <span className="asb-tok-bool">{text}</span>;
  }
  if (/^-?\d/.test(trimmed)) {
    return <span className="asb-tok-num">{text}</span>;
  }
  return <span>{text}</span>;
}

/** Per-line token coloring; TOML (`key = value`, `[table]`) and JSON
    (`"key": value`, braces) line shapes are both recognized. */
function renderLine(line: string): ReactNode {
  const trimmed = line.trim();
  if (!trimmed) {
    return <span>{line}</span>;
  }
  if (/^\[.*\]$/.test(trimmed) || /^[{}]+$/.test(trimmed)) {
    return <span className="asb-tok-section">{line}</span>;
  }
  const json = line.match(/^(\s*"(?:[^"\\]|\\.)*"\s*:)(.*)$/);
  if (json) {
    return (
      <>
        <span className="asb-tok-key">{json[1]}</span>
        {renderValue(json[2])}
      </>
    );
  }
  const eq = line.indexOf("=");
  if (eq > 0) {
    return (
      <>
        <span className="asb-tok-key">{line.slice(0, eq + 1)}</span>
        {renderValue(line.slice(eq + 1))}
      </>
    );
  }
  return <span>{line}</span>;
}

interface CodePreviewProps {
  target: string;
  content: string;
}

/** Pretty-printed candidate configuration file: line numbers, light token
    coloring, near-solid code backdrop (DESIGN.md §6 — config text stays
    high-contrast without blur). */
export function CodePreview({ target, content }: CodePreviewProps) {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);
  const id = useId();
  const lines = content.replace(/\n$/, "").split("\n");
  return (
    <div className="asb-filepreview" aria-label={t("providers.code.previewAria", { target })}>
      <div className="asb-filepreview-head">
        <span className="asb-code">{target}</span>
        <div className="asb-panel-actions">
          <span className="asb-filepreview-meta">{t("providers.code.lines", { count: lines.length })}</span>
          <CopyButton content={content} />
          {lines.length > 12 && <Button variant="secondary" aria-expanded={expanded} aria-controls={id}
            onClick={() => setExpanded((value) => !value)}>
            {t(expanded ? "common.collapsePreview" : "common.expandPreview")}
          </Button>}
        </div>
      </div>
      <ScrollArea id={id} tabIndex={0} aria-label={target} className={`asb-filepreview-body${expanded ? " is-expanded" : ""}`}>
        <pre className="asb-filepreview-content">
          {lines.map((line, index) => (
            <span className="asb-filepreview-line" key={index}>
              <span className="asb-filepreview-num" aria-hidden="true">
                {index + 1}
              </span>
              <span className="asb-filepreview-text">{renderLine(line)}</span>
            </span>
          ))}
        </pre>
      </ScrollArea>
    </div>
  );
}
