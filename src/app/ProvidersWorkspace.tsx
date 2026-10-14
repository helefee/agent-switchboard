import { useCallback, useEffect, useState } from "react";
import type { AppKind } from "../api/client";
import { CodexProvidersPage } from "../pages/CodexProvidersPage";
import { ProviderImportPage } from "../pages/ProviderImportPage";
import { ProvidersPage } from "../pages/ProvidersPage";
import type { ClientRouteDirection } from "../components/ProviderWorkspaceShell";
import type { SwitchboardModel } from "./useSwitchboardModel";

/** Client-tab order; the direction contract is defined against this single
 * list, so reordering tabs only means reordering here. */
const CLIENT_ROUTE_ORDER: readonly AppKind[] = ["codex", "claude"];

function routeDirectionBetween(previous: AppKind, current: AppKind): ClientRouteDirection {
  if (previous === current) return "none";
  return CLIENT_ROUTE_ORDER.indexOf(current) > CLIENT_ROUTE_ORDER.indexOf(previous) ? "forward" : "back";
}

export function ProvidersWorkspace({ model, active }: { model: SwitchboardModel; active: boolean }) {
  const importing = model.providerView.kind === "import";
  const { snapshot, appFilter, activeProfileId, providers, providerSwitch, appSettingsState, busy, operations } = model;
  const { discoveryState, ccImport, sqlImport } = model;
  /** One-shot entry direction for a client-tab switch: set only by the
   * picker on this page, consumed when the entry animation has played. The
   * replaced workspace stays mounted for the same one exit cycle (leaving),
   * so both shells cross-slide as one track with no blank frame. Every
   * other mount (editor close, usage view, page return, cross-client
   * switches elsewhere) sees "none" and stays still. */
  const [routeDirection, setRouteDirection] = useState<ClientRouteDirection>("none");
  const [leavingApp, setLeavingApp] = useState<AppKind | null>(null);
  const changeApp = (app: AppKind) => {
    if (app === appFilter) return;
    setRouteDirection(routeDirectionBetween(appFilter, app));
    setLeavingApp(appFilter);
    providers.selectApp(app);
  };
  const settleRoute = useCallback(() => {
    setRouteDirection("none");
    setLeavingApp(null);
  }, []);
  useEffect(() => {
    if (leavingApp === null) return;
    // Janitor only: the exit normally ends through the shell's animationend;
    // the timer covers a switch that unmounts the leaving shell mid-flight.
    const timer = window.setTimeout(() => setLeavingApp(null), 400);
    return () => window.clearTimeout(timer);
  }, [leavingApp]);
  const editorApp = providers.editorSession?.app ?? appFilter;
  const userConfigRoute = snapshot.statuses?.find((status) => status.app === editorApp)?.route ?? null;
  const codexEditorSession = providers.editorSession?.app === "codex" ? providers.editorSession : null;
  const claudeEditorSession = providers.editorSession?.app === "claude" ? providers.editorSession : null;
  useEffect(() => {
    if (!active || importing || appFilter !== "claude" || claudeEditorSession) providerSwitch.clearCandidates();
  }, [active, importing, appFilter, claudeEditorSession, providerSwitch.clearCandidates]);
  return (
    <>
      {importing && <div className="asb-editor-route" hidden={!active}>
        <ProviderImportPage appFilter={appFilter} discovery={discoveryState.discovery} busy={busy}
          ccScan={ccImport.ccScan} ccSelected={ccImport.ccSelected} ccResult={ccImport.ccResult}
          ccDirectory={ccImport.ccDirectory}
          sqlScan={sqlImport.sqlScan} sqlSelected={sqlImport.sqlSelected} sqlResult={sqlImport.sqlResult}
          onBack={() => model.setProviderView({ kind: "list" })}
          onScanLocal={() => void discoveryState.runDiscovery()} onImportLocal={discoveryState.runImport}
          onScanCc={() => void ccImport.runCcScan()} onImportCc={ccImport.runCcImport}
          onSelectCc={(key, checked) => ccImport.setCcSelected((current) => ({ ...current, [key]: checked }))}
          onCcDirectory={ccImport.changeCcDirectory}
          onApplySql={(path) => void sqlImport.runSqlApply(path)} onImportSql={sqlImport.runSqlImport}
          onSelectSql={(key, checked) => sqlImport.setSqlSelected((current) => ({ ...current, [key]: checked }))}
          onError={model.reportError} />
      </div>}
      <CodexProvidersPage
        active={active && !importing && appFilter === "codex"} routeDirection={routeDirection}
        onRouteAnimationEnd={settleRoute} leaving={leavingApp === "codex"}
        onSelectApp={changeApp} records={snapshot.codexRecords}
        officialRecord={snapshot.codexOfficialRecords[0] ?? null}
        activeProfileId={activeProfileId("codex")}
        busy={busy} onBusy={model.setBusy} onError={model.reportError} onRefresh={async () => { await snapshot.refresh(); }}
        onImport={() => { providerSwitch.clearCandidates(); model.setProviderView({ kind: "import" }); }}
        onDelete={(record) => providers.setDeletePending({ kind: "codexThirdParty", record })}
        onDeleteOfficial={(record) => providers.setDeletePending({ kind: "generic", profile: record.profile })}
        editorSession={codexEditorSession} onNew={providers.newEditor} onEdit={providers.openCodexEditor}
        onEditOfficial={providers.openCodexOfficialEditor}
        onCloseEditor={providers.closeEditor}
        onSave={providers.saveCodexProfile} onSaveOfficial={providers.saveCodexOfficialProfile}
        onSwitchAccessMode={providers.switchCodexAccessMode}
        onSwitchClient={providers.newEditorFor}
        onSaveOfficialQuotaInterval={providers.saveOfficialQuotaInterval}
        expandedUsageIds={appSettingsState.appSettings?.expandedUsageIds ?? []}
        onToggleUsage={(profileId) => appSettingsState.toggleUsageExpanded(profileId)}
        onSaveUsageQuery={providers.saveCodexProfileUsageQuery}
        loginBlocker={snapshot.loginBlocker}
        statuses={snapshot.statuses} relayProfiles={snapshot.relayProfiles} locks={snapshot.locks}
        userConfigModel={userConfigRoute?.model ?? null} userConfigWarnings={userConfigRoute?.scopeWarnings ?? []} />
    <ProvidersPage active={active && !importing && appFilter === "claude"} routeDirection={routeDirection}
      onRouteAnimationEnd={settleRoute} leaving={leavingApp === "claude"}
      onSelectApp={changeApp} view={model.providerView} onViewChange={model.setProviderView}
      profiles={snapshot.profiles}
      relayProfiles={snapshot.relayProfiles}
      activeProfileId={activeProfileId("claude")} statuses={snapshot.statuses} locks={snapshot.locks}
      userConfigModel={userConfigRoute?.model ?? null} userConfigWarnings={userConfigRoute?.scopeWarnings ?? []}
      editorSession={claudeEditorSession}
      busy={busy} expandedUsageIds={appSettingsState.appSettings?.expandedUsageIds ?? []}
      activationCandidate={providerSwitch.activationCandidate}
      onNew={providers.newEditor}
      onImport={() => { providerSwitch.clearCandidates(); model.setProviderView({ kind: "import" }); }}
      onCloseEditor={providers.closeEditor} onSave={providers.saveProfile}
      onSwitchClient={providers.newEditorFor}
      onSaveUsageQuery={async (profile, query) => {
        const saved = await providers.saveProfileUsageQuery(profile, query);
        if (saved) {
          providers.selectApp(profile.app);
          providerSwitch.setTargetProfile(profile.id);
        }
        return saved;
      }}
      onReorder={providers.dragReorderClaudeProfiles}
      onToggleUsage={(profile) => appSettingsState.toggleUsageExpanded(profile.id)}
      onActivate={providerSwitch.requestActivation}
      onConfirmSwitch={() => void operations.runSwitch()}
      onCancelActivation={providerSwitch.clearCandidates}
      onEdit={providers.openEditor}
      onDelete={(profile) => providers.setDeletePending({ kind: "generic", profile })}
      onRefresh={async () => { await snapshot.refresh(); }} />
    </>
  );
}
