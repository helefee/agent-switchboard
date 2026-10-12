import type { ConfigValue, SettingSpec, SettingValue } from "../api/client";
import { useI18n } from "../i18n";
import { catalogText } from "../i18n/errors";
import { Button } from "./Button";
import { Input } from "./Input";
import { RadioOption } from "./RadioOption";

interface Props {
  spec: SettingSpec;
  value: SettingValue;
  busy: boolean;
  onChange: (value: SettingValue) => void;
}

function explicit(value: ConfigValue): SettingValue { return { mode: "explicit", value }; }

export function SettingInput({ spec, value, busy, onChange }: Props) {
  const { t } = useI18n();
  const label = catalogText(spec.label, t);
  if (spec.control === "notifications") {
    const events = value.mode === "explicit" && Array.isArray(value.value) ? value.value : null;
    const update = (items: string[]) => onChange(explicit(items));
    return <div className="asb-setting-input">
      <div className="asb-segments" role="radiogroup" aria-label={label}>
        <RadioOption name={`${spec.key}-setting`} checked={value.mode === "automatic"} disabled={busy}
          label={t("clientConfig.settings.automatic")} onChange={() => onChange({ mode: "automatic" })} />
        {[true, false].map((enabled) => <RadioOption key={String(enabled)} name={`${spec.key}-setting`}
          checked={value.mode === "explicit" && value.value === enabled} disabled={busy}
          label={t(enabled ? "clientConfig.settings.on" : "clientConfig.settings.off")}
          onChange={() => onChange(explicit(enabled))} />)}
        <RadioOption name={`${spec.key}-setting`} checked={events !== null} disabled={busy}
          label={t("clientConfig.settings.eventList")} onChange={() => update([])} />
      </div>
      {events !== null && <div className="asb-setting-input">
        {events.map((event, index) => <div className="asb-setting-input-row" key={index}>
          <Input code value={event} disabled={busy} aria-label={t("clientConfig.settings.eventName", { index: index + 1 })}
            aria-invalid={!event.trim()} onChange={(change) => update(events.map((item, i) => i === index ? change.target.value : item))} />
          <Button variant="secondary" disabled={busy} aria-label={t("clientConfig.settings.removeEvent", { index: index + 1 })}
            onClick={() => update(events.filter((_, i) => i !== index))}>{t("clientConfig.settings.remove")}</Button>
        </div>)}
        <Button variant="secondary" disabled={busy} onClick={() => update([...events, ""])}>{t("clientConfig.settings.addEvent")}</Button>
      </div>}
    </div>;
  }
  if (spec.control !== "text" && spec.control !== "integer" && spec.control !== "stringInteger") return null;
  const numeric = spec.control !== "text";
  const bounds = "min" in spec ? spec : undefined;
  const raw = value.mode === "automatic" ? "" : String(value.value);
  return <div className="asb-setting-input">
    <Input code={numeric} aria-label={label} disabled={busy} value={raw}
      type={numeric ? "number" : "text"} min={bounds?.min} max={bounds?.max}
      step={numeric ? 1 : undefined} placeholder={t("clientConfig.settings.automatic")}
      onChange={(event) => {
        const text = event.target.value;
        onChange(text === "" ? { mode: "automatic" } : explicit(spec.control === "integer" ? Number(text) : text));
      }} />
    {bounds && <span className="asb-field-help">{t("clientConfig.settings.integerRange", { min: bounds.min, max: bounds.max })}</span>}
  </div>;
}
