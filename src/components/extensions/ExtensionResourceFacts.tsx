import type { ExtensionListItem, SecretValueView as CredentialView } from "../../api/client";
import { useI18n } from "../../i18n";
import { clientName } from "../../lib/client-name";
import { FactList, type FactListRow } from "../FactList";
import { Table } from "../Table";
import { TRANSPORT_LABELS } from "./labels";

function SecretValue({ value }: { value: CredentialView }) {
  const { t } = useI18n();
  if (value.mode === "envRef")
    return (
      <span>
        {t("extensions.facts.envRef")} <span className="asb-code">{value.name}</span>
      </span>
    );
  return value.mode === "stored" ? (
    <span className="asb-pill-status">{t("extensions.facts.credentialSet")}</span>
  ) : (
    <span className="asb-code">••••••••</span>
  );
}

function SecretMap({ title, values }: { title: string; values: Record<string, CredentialView> }) {
  const { t } = useI18n();
  if (Object.keys(values).length === 0) return null;
  return (
    <div className="asb-ext-section">
      <h4 className="asb-group-title">{title}</h4>
      <Table rows={Object.entries(values)} rowKey={([key]) => key} ariaLabel={title} columns={[
        { key: "name", header: t("extensions.table.name"), render: ([key]) => <span className="asb-code">{key}</span> },
        { key: "value", header: t("extensions.table.value"), render: ([, value]) => <SecretValue value={value} /> },
      ]} />
    </div>
  );
}

function SkillFacts({ item }: { item: Extract<ExtensionListItem, { kind: "skill" }> }) {
  const { t } = useI18n();
  const rows: FactListRow[] = [
    { label: t("extensions.facts.skillName"), value: <span className="asb-code">{item.manifest.name}</span> },
  ];
  if (item.manifest.description) rows.push({ label: t("extensions.facts.description"), value: item.manifest.description });
  if (item.manifest.license) rows.push({ label: t("extensions.facts.license"), value: item.manifest.license });
  if (item.manifest.allowedTools?.length)
    rows.push({ label: t("extensions.facts.allowedTools"), value: item.manifest.allowedTools.join(t("extensions.join.comma")) });
  if (item.manifest.unparsedKeys?.length)
    rows.push({ label: t("extensions.facts.unparsedKeys"), value: item.manifest.unparsedKeys.join(t("extensions.join.comma")) });
  rows.push({ label: t("extensions.facts.digest"), value: <span className="asb-code">{item.contentDigest.slice(0, 12)}</span> });
  if (item.source)
    rows.push({
      label: t("extensions.facts.source"),
      value: item.source.resolvedCommit
        ? t("extensions.facts.sourceCommit", { commit: item.source.resolvedCommit.slice(0, 12) })
        : t("extensions.facts.sourceRecorded"),
    });
  if (item.hostScoped)
    rows.push({ label: t("extensions.facts.hostScoped"), value: t("extensions.facts.hostOnly", { client: clientName(item.hostScoped) }) });
  const dependencyLabels = {
    bound: t("extensions.deps.bound"),
    pendingConfiguration: t("extensions.deps.pending"),
    targetUnsupported: t("extensions.deps.unsupported"),
  };
  return (
    <>
      <FactList rows={rows} />
      {item.compatibility.length > 0 && (
        <div className="asb-ext-section">
          <h4 className="asb-group-title">{t("extensions.facts.compatibility")}</h4>
          <ul className="asb-ext-secret-list">
            {item.compatibility.map((note) => (
              <li key={note.code}>{note.message}</li>
            ))}
          </ul>
        </div>
      )}
      <div className="asb-ext-section">
        <h4 className="asb-group-title">{t("extensions.facts.dependencies")}</h4>
        {item.dependencyStates.length === 0 ? (
          <p className="asb-empty">{t("extensions.facts.noDependencies")}</p>
        ) : (
          <Table rows={item.dependencyStates} rowKey={(dependency) => dependency.name}
            ariaLabel={t("extensions.facts.dependencies")} columns={[
              { key: "name", header: t("extensions.table.name"), render: (dependency) => dependency.name },
              { key: "status", header: t("extensions.table.status"), render: (dependency) => dependencyLabels[dependency.state] },
            ]} />
        )}
      </div>
    </>
  );
}

export function ExtensionResourceFacts({ item }: { item: ExtensionListItem }) {
  const { t } = useI18n();
  if (item.kind === "skill") return <SkillFacts item={item} />;
  const rows: FactListRow[] = [{ label: t("extensions.facts.transport"), value: t(TRANSPORT_LABELS[item.transport]) }];
  if (item.transport === "stdio")
    rows.push(
      { label: t("extensions.facts.command"), value: <span className="asb-code">{item.command}</span> },
      {
        label: t("extensions.facts.args"),
        value: item.argumentCount > 0
          ? t("extensions.facts.argsCount", { count: item.argumentCount })
          : t("extensions.facts.none"),
      },
    );
  else rows.push({ label: t("extensions.facts.url"), value: <span className="asb-code">{item.url}</span> });
  return (
    <>
      <FactList rows={rows} />
      {item.transport === "stdio" ? (
        <SecretMap title={t("extensions.facts.envVars")} values={item.env} />
      ) : (
        <SecretMap title={t("extensions.facts.headers")} values={item.headers} />
      )}
      {item.transport === "http" && item.bearer && (
        <div className="asb-ext-section">
          <h4 className="asb-group-title">{t("extensions.facts.bearer")}</h4>
          <SecretValue value={item.bearer} />
        </div>
      )}
    </>
  );
}
