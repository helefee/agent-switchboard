import type { DeclarativeUsageQuery } from "../../api/client";
import { useI18n } from "../../i18n";
import { Input } from "../Input";
import { Textarea } from "../Textarea";

const SCRIPT_TEMPLATE = `({
  request: ({ baseUrl, apiKey }) => ({
    url: \`${"${baseUrl}"}/user/balance\`,
    method: "GET",
    headers: { Authorization: \`Bearer ${"${apiKey}"}\` },
  }),
  extract: ({ body }) => ({ remaining: body.balance, used: null, total: null, unit: "USD" }),
})`;

const PATH_FIELDS = [
  { key: "remainingPath", placeholder: "data/balance" },
  { key: "usedPath", placeholder: "data/used" },
  { key: "totalPath", placeholder: "data/total" },
  { key: "unit", placeholder: "USD" },
] as const;

export function DeclarativeFields({ value, disabled, onChange }: { value: DeclarativeUsageQuery;
  disabled: boolean; onChange: (fields: Partial<DeclarativeUsageQuery>) => void }) {
  const { t } = useI18n();
  return <><h3 className="asb-section-title">{t("usage.query.configTitle")}</h3>
    <div className="asb-editor-section-fields"><label className="asb-field">
      <span>{t("usage.query.urlLabel")}</span><Input aria-label={t("usage.query.urlAria")}
        value={value.url} disabled={disabled} placeholder="{{baseUrl}}/user/balance"
        onChange={(event) => onChange({ url: event.target.value })} /></label>
      <div className="asb-usage-paths">{PATH_FIELDS.map(({ key, placeholder }) => <label className="asb-field" key={key}>
        <span>{t(`usage.query.${key}`)}</span><Input code={key !== "unit"} aria-label={t(`usage.query.${key}Aria`)}
          value={value[key] ?? ""} disabled={disabled} placeholder={placeholder}
          onChange={(event) => onChange({ [key]: event.target.value.trim() || null })} /></label>)}</div>
      <p className="asb-scope-note">{t("usage.query.declarativeNote", { baseUrl: "{{baseUrl}}", apiKey: "{{apiKey}}" })}</p>
      <p className="asb-scope-note">{t("usage.query.fieldsNote")}</p>
    </div></>;
}

export function ScriptFields({ source, disabled, onChange }: { source: string; disabled: boolean; onChange: (source: string) => void }) {
  const { t } = useI18n();
  return <><h3 className="asb-section-title">{t("usage.query.configTitle")}</h3>
    <div className="asb-editor-section-fields"><label className="asb-field"><span>{t("usage.query.scriptLabel")}</span>
      <Textarea code rows={16} aria-label={t("usage.query.scriptLabel")} value={source} disabled={disabled}
        placeholder={SCRIPT_TEMPLATE} spellCheck={false} onChange={(event) => onChange(event.target.value)} /></label>
      <div className="asb-usage-script-contract"><span>{t("usage.query.input")}</span><code>{"request({ baseUrl, apiKey })"}</code>
        <span>{t("usage.query.output")}</span><code>{"extract({ body, status })"}</code></div>
      <p className="asb-scope-note">{t("usage.query.scriptNote")}</p></div></>;
}

export function RefreshInterval({ text, disabled, onChange, onCommit }: { text: string; disabled: boolean;
  onChange: (text: string) => void; onCommit: () => void }) {
  const { t } = useI18n();
  return <section className="asb-editor-section" aria-label={t("usage.query.autoRefresh")}>
    <h3 className="asb-section-title">{t("usage.query.autoRefresh")}</h3><div className="asb-editor-section-fields">
      <label className="asb-field is-narrow"><span>{t("usage.query.intervalLabel")}</span>
        <Input type="number" min={0} max={1440} step={1} aria-label={t("usage.query.intervalAria")}
          value={text} disabled={disabled} onChange={(event) => onChange(event.target.value)} onBlur={onCommit}
          onKeyDown={(event) => { if (event.key === "Enter") onCommit(); }} /></label></div></section>;
}
