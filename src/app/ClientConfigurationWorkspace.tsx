import { ClientSettingsPanel } from "../components/ClientSettingsPanel";
import { CodexSubagentSettingsPanel } from "../components/CodexSubagentSettingsPanel";
import { GlobalPromptManager } from "../components/GlobalPromptManager";
import { useI18n } from "../i18n";
import type { SwitchboardModel } from "./useSwitchboardModel";
import type { AppKind } from "../api/client";

export function ClientConfigurationWorkspace({ model }: { model: SwitchboardModel }) {
  return <>{(["codex", "claude"] as const).map((app) => (
    <div key={app} hidden={model.appFilter !== app}>
      <ClientConfigurationPanel model={model} app={app} />
    </div>
  ))}</>;
}

function ClientConfigurationPanel({ model, app }: { model: SwitchboardModel; app: AppKind }) {
  const { t } = useI18n();
  const {
    clientSettings: settings,
    codexSubagentSettings,
    promptDocuments: prompts,
    snapshot,
    busy,
  } = model;
  const promptDocument = prompts.documents[app];
  return (
    <section className="asb-panel" aria-label={t("clientConfig.title")}>
      <ClientSettingsPanel
        app={app}
        onSelectApp={model.providers.selectApp}
        editorState={settings.states[app]}
        busy={busy}
        configStatus={snapshot.statuses?.find((status) => status.app === app)}
        hasUnsavedConfigurationDraft={settings.states[app].phase === "dirty" ||
          (app === "codex" && codexSubagentSettings.editorState.phase === "dirty")}
        onValueChange={settings.changeValue}
        onApplied={() => {
          settings.reloadFromStored(app);
          if (app === "codex") codexSubagentSettings.reloadFromFile();
          model.providerSwitch.invalidateCandidates();
          void snapshot.refresh();
        }} onRetryLoad={settings.retryLoad}
        onReview={settings.reviewConfiguration}
        onClaudeExtraChange={settings.changeClaudeExtra}
        globalInstructions={
          <GlobalPromptManager document={promptDocument}
            draft={prompts.drafts[app] ?? promptDocument?.content ?? ""}
            dirty={prompts.isPromptDirty(app)} busy={busy}
            onChange={(content) => prompts.setPromptDraft(app, content)}
            onSave={() => void prompts.savePromptDocument(app)}
            onDiscard={() => prompts.discardPromptDraft(app)}
            onReload={() => prompts.reloadPromptDocument(app)} />
        }
        subagentDraft={app === "codex" ? codexSubagentSettings.editorState.draft : undefined}
        subagentSettings={app === "codex" ? (
          <CodexSubagentSettingsPanel
            editorState={codexSubagentSettings.editorState}
            busy={busy}
            onChange={codexSubagentSettings.changeValue}
            onRetryLoad={codexSubagentSettings.retryLoad}
          />
        ) : undefined}
      />
    </section>
  );
}
