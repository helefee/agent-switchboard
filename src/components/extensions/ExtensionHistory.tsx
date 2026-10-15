import type { ExtensionListItem, ExtensionOperationRecord } from "../../api/client";
import { useState } from "react";
import { useI18n } from "../../i18n";
import { Button } from "../Button";
import { Table } from "../Table";
import { Time } from "../Time";
import { OPERATION_LABELS, outcomeText, targetLabel } from "./labels";
import { UpdateIcon } from "../icons";
import { Pagination } from "../Pagination";

interface Props {
  records: ExtensionOperationRecord[];
  items: ExtensionListItem[];
  busy: boolean;
  projectNames: ReadonlyMap<string, string>;
  onRestore: (operationId: string) => void;
}

type Resource = ExtensionOperationRecord["resources"][number];
interface OutcomeRow { resource: Resource; target: Resource["targets"][number] | null }

function HistoryRecord({ record, props }: { record: ExtensionOperationRecord; props: Props }) {
  const { t } = useI18n();
  const nameOf = (definitionId: string) => props.items.find((item) => item.id === definitionId)?.name ?? definitionId;
  const operation = record.resources.find((resource) => resource.operation === "install" || resource.operation === "remove")
    ?.operation ?? record.resources[0]?.operation ?? "update";
  const rows = record.resources.flatMap<OutcomeRow>((resource) => resource.targets.length
    ? resource.targets.map((target) => ({ resource, target })) : [{ resource, target: null }]);
  return <article className="asb-ext-history-item">
    <div className="asb-ext-history-head">
      <span className="asb-pill-status">{t(OPERATION_LABELS[operation])}</span>
      <span>{record.resources.map((resource) => nameOf(resource.definitionId)).join(t("extensions.join.comma"))}</span>
      <Time iso={record.createdAt} />
      <Button variant="secondary" disabled={props.busy} onClick={() => props.onRestore(record.id)}>
        {t("extensions.history.restore")}
      </Button>
    </div>
    {rows.length > 0 && <Table rows={rows} rowKey={(row, index) => `${row.resource.definitionId}:${index}`}
      ariaLabel={t("extensions.history.resultsAria", { time: record.createdAt })}
      className="asb-ext-management-table" columns={[
        { key: "extension", header: t("extensions.table.extension"), render: (row) => nameOf(row.resource.definitionId) },
        { key: "operation", header: t("extensions.table.actions"), render: (row) => t(OPERATION_LABELS[row.resource.operation]) },
        { key: "target", header: t("extensions.table.target"), render: (row) => row.target
          ? targetLabel(row.target.target, props.projectNames) : "—" },
        { key: "outcome", header: t("extensions.table.result"), render: (row) => row.target ? (
          <div className="asb-ext-table-cell-stack">
            <span>{outcomeText(row.target.outcome)}</span>
            {row.target.warnings?.map((warning) => <span key={warning} className="asb-warn-text">{warning}</span>)}
          </div>
        ) : "—" },
      ]} />}
    {record.rollback && <p className="asb-scope-note">
      {t("extensions.history.rollback", { restored: record.rollback.restored.length, failed: record.rollback.failed.length })}
    </p>}
  </article>;
}

/** Each batch keeps one restore action; its resource/target outcomes are table rows. */
export function ExtensionHistory(props: Props) {
  const { t } = useI18n();
  const [page, setPage] = useState(1);
  const pageSize = 20;
  const currentPage = Math.min(page, Math.max(1, Math.ceil(props.records.length / pageSize)));
  if (props.records.length === 0) return <div className="asb-empty-state">
    <span className="asb-empty-state-icon" aria-hidden="true"><UpdateIcon /></span>
    <h3 className="asb-section-title">{t("extensions.history.empty")}</h3>
  </div>;
  return <section className="asb-ext-history" aria-label={t("extensions.history.title")}>
    {props.records.slice((currentPage - 1) * pageSize, currentPage * pageSize)
      .map((record) => <HistoryRecord key={record.id} record={record} props={props} />)}
    <Pagination total={props.records.length} page={currentPage} pageSize={pageSize}
      onPageChange={setPage} label={t("extensions.history.title")} />
  </section>;
}
