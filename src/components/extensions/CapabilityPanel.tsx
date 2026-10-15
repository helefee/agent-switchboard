import { useState } from "react";
import type { CapabilityEntry, ClientCapabilityReport } from "../../api/client";
import { useI18n } from "../../i18n";
import { catalogText } from "../../i18n/errors";
import { clientName } from "../../lib/client-name";
import { CollapsibleSection } from "../Collapsible";
import { Table } from "../Table";

interface Props { reports: ClientCapabilityReport[] }
interface CapabilityRow { client: ClientCapabilityReport["client"]; entry: CapabilityEntry | null }

/** Capability rows share one table so resource support can be compared across clients. */
export function CapabilityPanel({ reports }: Props) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const rows = reports.flatMap<CapabilityRow>((report) => report.entries.length
    ? report.entries.map((entry) => ({ client: report.client, entry }))
    : [{ client: report.client, entry: null }]);
  return (
    <CollapsibleSection className="asb-ext-capabilities" open={open} onOpenChange={setOpen}
      label={<span className="asb-section-title">{t("extensions.capabilities.title")}</span>}
      value={t("extensions.capabilities.count", { count: reports.length })}>
      {rows.length === 0 ? <p className="asb-empty">{t("extensions.capabilities.empty")}</p> : (
        <Table rows={rows} rowKey={(row) => `${row.client}:${row.entry?.code ?? "empty"}`}
          ariaLabel={t("extensions.capabilities.title")} className="asb-ext-management-table" columns={[
            { key: "client", header: t("extensions.table.client"), render: (row) => clientName(row.client) },
            { key: "resource", header: t("extensions.table.resource"), render: ({ entry }) => entry
              ? catalogText(entry.resource, t) : t("extensions.capabilities.empty") },
            { key: "status", header: t("extensions.table.status"), render: ({ entry }) => !entry ? "—" : (
              <span className={entry.verification === "verified" && entry.supported ? "asb-ok-text" : "asb-warn-text"}>
                {entry.verification === "verified" && entry.supported
                  ? t("extensions.capabilities.verified", { version: entry.clientVersion })
                  : t("extensions.capabilities.unverified")}
              </span>
            ) },
            { key: "condition", header: t("extensions.table.condition"), render: ({ entry }) => entry?.verification === "open"
              ? catalogText(entry.condition, t) : "—" },
          ]} />
      )}
    </CollapsibleSection>
  );
}
