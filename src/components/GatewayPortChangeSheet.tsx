import { errorText, localizedMessageText } from "../i18n/errors";
import type { LocalizedMessage } from "../api/client";
import {
  type AppKind,
  type GatewayPortChangePlan,
} from "../api/client";
import { useI18n } from "../i18n";
import { Button } from "./Button";
import { Input } from "./Input";
import { AppDialog } from "./AppDialog";
import { MIN_PORT, MAX_PORT, useGatewayPortChange } from "./useGatewayPortChange";

const APP_LABELS: Record<AppKind, string> = { codex: "Codex", claude: "Claude Code" };

export interface PortSheetState {
  stage: "input" | "preview" | "done";
  port?: string;
  plan?: GatewayPortChangePlan;
  resultToPort?: number;
  warnings?: LocalizedMessage[];
  error?: unknown;
}

interface Props {
  state: PortSheetState;
  configuredPort: number;
  onState: (state: PortSheetState | null) => void;
  onRefreshed: () => void;
}

/** Input, preview, and confirmation for one backend-held port reservation. */
export function GatewayPortChangeSheet({ state, configuredPort, onState, onRefreshed }: Props) {
  const { t } = useI18n();
  const flow = useGatewayPortChange(state, configuredPort, onState, onRefreshed);
  return (
    <AppDialog title={t("gateway.sheetTitle")} busy={flow.busy} narrow onClose={flow.cancel}
      footer={<PortActions stage={state.stage} flow={flow} />}>
          {state.stage === "input" && (
            <PortInput state={state} configuredPort={configuredPort} onState={onState} flow={flow} />
          )}
          {state.stage === "preview" && state.plan && <Preview plan={state.plan} />}
          {state.stage === "done" && (
            <>
              <p className="asb-scope-note" role="status">
                {t("gateway.doneCopy", { port: state.resultToPort! })}
              </p>
              {state.warnings?.map((warning, index) => (
                <p key={`${index}:${warning.key}`} className="asb-warn-text" role="status">{localizedMessageText(warning, t)}</p>
              ))}
            </>
          )}
          {state.error != null && (
            <p className="asb-scope-note asb-fail-text" role="alert">
              {errorText(state.error, t)}
            </p>
          )}
    </AppDialog>
  );
}

type PortFlow = ReturnType<typeof useGatewayPortChange>;

function PortInput({ state, configuredPort, onState, flow }: Omit<Props, "onRefreshed"> & { flow: PortFlow }) {
  const { t } = useI18n();
  return <>
    <label className="asb-field is-narrow">
      <span>{t("gateway.newPortLabel", { port: configuredPort })}</span>
      <Input type="number" min={MIN_PORT} max={MAX_PORT} step={1} value={state.port ?? ""}
        autoFocus disabled={flow.busy}
        onChange={(event) => onState({ ...state, port: event.target.value, error: null })}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            event.preventDefault(); void flow.prepare();
          }
        }} />
    </label>
    <p className="asb-scope-note">{t("gateway.portRangeNote", { min: MIN_PORT, max: MAX_PORT })}</p>
  </>;
}

function PortActions({ stage, flow }: { stage: PortSheetState["stage"]; flow: PortFlow }) {
  const { t } = useI18n();
  const { busy, prepare, commit, cancel } = flow;
  return <>
          {stage === "input" && (
            <>
              <Button variant="secondary" disabled={busy} onClick={cancel}>{t("gateway.cancel")}</Button>
              <Button variant="primary" disabled={busy} onClick={() => void prepare()}>
                {busy ? t("gateway.validating") : t("gateway.nextPreview")}
              </Button>
            </>
          )}
          {stage === "preview" && (
            <>
              <Button autoFocus variant="secondary" disabled={busy} onClick={cancel}>{t("gateway.cancel")}</Button>
              <Button variant="primary" disabled={busy} onClick={() => void commit()}>
                {busy ? t("gateway.applying") : t("gateway.confirmApply")}
              </Button>
            </>
          )}
          {stage === "done" && <Button autoFocus variant="secondary" onClick={cancel}>{t("gateway.close")}</Button>}
  </>;
}

function Preview({ plan }: { plan: GatewayPortChangePlan }) {
  const { t } = useI18n();
  return (
    <ul className="asb-dialog-details">
      <li>{t("gateway.previewPort", { from: plan.fromPort, to: plan.toPort })}</li>
      {plan.clients.length === 0 ? (
        <li>{t("gateway.previewNoClients")}</li>
      ) : plan.clients.map((client) => (
        <li key={`${client.app}:${client.profileId}`}>
          {t("gateway.previewClientAddress", { name: client.profileName, app: APP_LABELS[client.app] })}<br />
          <span className="asb-num">{client.currentBaseUrl}</span><br />
          → <span className="asb-num">{client.newBaseUrl}</span>
        </li>
      ))}
      <li>{t("gateway.previewUnchanged")}</li>
    </ul>
  );
}
