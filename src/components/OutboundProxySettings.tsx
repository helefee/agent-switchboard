import { useCallback, useEffect, useRef, useState } from "react";
import { getOutboundProxySettings, repairOutboundProxySettings, setOutboundProxySettings,
  type OutboundProxyMode, type OutboundProxySettings as Settings } from "../api/outbound-proxy";
import { useI18n } from "../i18n";
import { useMessageState } from "../i18n/use-message-state";
import { Button } from "./Button";
import { AppSettingRow } from "./AppPreferenceControls";
import { RadioOption } from "./RadioOption";

const MODES: readonly OutboundProxyMode[] = ["none", "system", "manual"];

/** The application's one egress policy. Self-contained: owns its load and
 * save transaction, separate from the native AppSettings save. */
export function OutboundProxySettings({ active, busy }: { active: boolean; busy: boolean }) {
  const { t } = useI18n();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [mode, setMode] = useState<OutboundProxyMode>("none");
  const [url, setUrl] = useState("");
  const [saved, setSaved] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useMessageState();
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);

  const refresh = useCallback(async () => {
    try {
      const next = await getOutboundProxySettings();
      if (!mounted.current) return;
      setSettings(next);
      setMode(next.mode);
      setUrl(next.url ?? "");
    } catch (caught) { if (mounted.current) setError(caught); }
  }, [setError]);
  useEffect(() => { if (active) void refresh(); }, [active, refresh]);

  const save = async () => {
    if (!settings || working) return;
    setWorking(true); setError(null); setSaved(false);
    const next: Settings = {
      version: settings.version,
      mode,
      url: mode === "manual" ? url.trim() : null,
      revision: settings.revision,
    };
    try {
      const savedSettings = await setOutboundProxySettings(next, settings.revision);
      if (!mounted.current) return;
      setSettings(savedSettings);
      setUrl(savedSettings.url ?? "");
      setSaved(true);
    } catch (caught) { if (mounted.current) setError(caught); }
    finally { if (mounted.current) setWorking(false); }
  };

  /** A failed read means the file is from another version or corrupt; the
   * reset returns the card to the default policy instead of staying stuck. */
  const repair = async () => {
    if (working) return;
    setWorking(true); setError(null); setSaved(false);
    try {
      const reset = await repairOutboundProxySettings();
      if (!mounted.current) return;
      setSettings(reset);
      setMode(reset.mode);
      setUrl(reset.url ?? "");
    } catch (caught) { if (mounted.current) setError(caught); }
    finally { if (mounted.current) setWorking(false); }
  };

  const disabled = busy || working || settings === null;
  const dirty = settings !== null
    && (settings.mode !== mode || (mode === "manual" && settings.url !== url.trim()));
  return <section className="asb-app-settings-group" aria-labelledby="egress-settings">
    <h3 id="egress-settings" className="asb-section-title">{t("settings.egress.title")}</h3>
    <p className="asb-field-help">{t("settings.egress.description")}</p>
    <AppSettingRow label={t("settings.egress.mode")}>
      <div className="asb-segments" role="radiogroup" aria-label={t("settings.egress.mode")}>
        {MODES.map((value) => (
          <RadioOption key={value} name="egress-mode" label={t(`settings.egress.mode.${value}`)}
            checked={mode === value} disabled={disabled}
            onChange={() => { setMode(value); setSaved(false); }} />
        ))}
      </div>
    </AppSettingRow>
    {mode === "manual" && (
      <AppSettingRow label={t("settings.egress.url")} detail={t("settings.egress.urlHelp")}>
        <input className="asb-input" type="text" value={url} disabled={disabled}
          spellCheck={false} onChange={(event) => { setUrl(event.target.value); setSaved(false); }} />
      </AppSettingRow>
    )}
    {mode === "system" && <p className="asb-field-help">{t("settings.egress.systemNote")}</p>}
    {error && <p className="asb-field-error" role="alert">{error}</p>}
    {settings === null && error ? (
      <div className="asb-session-actions">
        <Button variant="secondary" disabled={busy || working} onClick={() => void repair()}>
          {t("settings.egress.repair")}
        </Button>
      </div>
    ) : (
      <div className="asb-session-actions">
        <Button variant="secondary" disabled={disabled || !dirty} onClick={() => void save()}>
          {t("settings.egress.save")}
        </Button>
        {saved && <span className="asb-ok-text">{t("settings.egress.saved")}</span>}
      </div>
    )}
  </section>;
}
