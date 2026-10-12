import { useState } from "react";
import type { SettingChoiceOption } from "../../api/client";
import { useI18n } from "../../i18n";
import { catalogText } from "../../i18n/errors";
import { Button } from "../Button";
import { Input } from "../Input";
import { Select } from "../Select";

type Fragment = Record<string, unknown>;
interface Props {
  fragment: Fragment;
  options: SettingChoiceOption[];
  busy: boolean;
  onChange: (fragment: Fragment) => void;
}

function object(value: unknown): Fragment {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Fragment : {};
}

/** Change only the effort leaf: other per-model preferences keep their owner. */
function withEffort(fragment: Fragment, model: string, effort: string): Fragment {
  const models = { ...object(fragment.modelSettings) };
  const preferences = { ...object(models[model]) };
  if (effort === "automatic") delete preferences.effortLevel;
  else preferences.effortLevel = effort;
  if (Object.keys(preferences).length) models[model] = preferences;
  else delete models[model];
  const next = { ...fragment };
  if (Object.keys(models).length) next.modelSettings = models;
  else delete next.modelSettings;
  return next;
}

export function withoutModelEffort(fragment: Fragment): Fragment {
  return Object.keys(object(fragment.modelSettings))
    .reduce((next, model) => withEffort(next, model, "automatic"), fragment);
}

export function ClaudeModelEffortEditor({ fragment, options, busy, onChange }: Props) {
  const { t } = useI18n();
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState<string | null>(null);
  const choices = options.map((option) => ({ value: option.value, label: catalogText(option.label, t) }));
  const entries = Object.entries(object(fragment.modelSettings))
    .filter(([, value]) => typeof object(value).effortLevel === "string");
  const validModel = model.length > 0 && model.trim() === model && !/[\u0000-\u001f\u007f]/.test(model);
  return <section className="asb-toggle-group" aria-label={t("clientConfig.modelEffort.title")}>
    <div className="asb-toggle-group-head"><h3 className="asb-section-title">{t("clientConfig.modelEffort.title")}</h3></div>
    <p className="asb-field-help">{t("clientConfig.modelEffort.help")}</p>
    {entries.map(([name, preferences]) => <div className="asb-setting-input-row" key={name}>
      <code className="asb-model-effort-name">{name}</code>
      <Select value={String(object(preferences).effortLevel)} disabled={busy}
        options={[{ value: "automatic", label: t("clientConfig.settings.automatic") }, ...choices]}
        ariaLabel={t("clientConfig.modelEffort.forModel", { model: name })}
        onChange={(value) => onChange(withEffort(fragment, name, value))} />
    </div>)}
    <div className="asb-setting-input-row">
      <Input code value={model} disabled={busy} aria-label={t("clientConfig.modelEffort.model")}
        placeholder={t("clientConfig.modelEffort.model")} onChange={(event) => setModel(event.target.value)} />
      <Select value={effort} disabled={busy} options={choices}
        placeholder={t("clientConfig.modelEffort.choose")} ariaLabel={t("clientConfig.modelEffort.choose")}
        onChange={setEffort} />
      <Button variant="secondary" disabled={busy || !validModel || !effort} onClick={() => {
        if (!validModel || !effort) return;
        onChange(withEffort(fragment, model, effort));
        setModel(""); setEffort(null);
      }}>{t("clientConfig.modelEffort.add")}</Button>
    </div>
  </section>;
}
