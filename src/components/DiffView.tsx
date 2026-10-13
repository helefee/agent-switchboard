import type { KeyChange } from "../api/client";
import { useI18n } from "../i18n";
import { Table, type TableColumn } from "./Table";

interface Props {
  changes: KeyChange[];
  /** Accessible name for the change list; names the surface it serves. */
  label: string;
}

/**
 * The single owner of change-row diff rendering (DESIGN.md §8): one change
 * per row, with aligned key, before and after columns. Every diff surface
 * consumes this component; Table owns its markup and base treatment.
 * Column labels and removal text keep changes legible without color alone.
 */
export function DiffView({ changes, label }: Props) {
  const { t } = useI18n();
  if (changes.length === 0) return null;
  const columns: Array<TableColumn<KeyChange>> = [
    { key: "key", header: t("providers.diff.column.key"), cellClassName: "asb-diff-key", render: (change) => change.key },
    {
      key: "before", header: t("providers.diff.column.before"), cellClassName: "asb-diff-value",
      render: (change) => change.before === null
        ? <span className="asb-diff-none">{t("providers.diff.none")}</span>
        : <span className="asb-diff-old">{change.before}</span>,
    },
    {
      key: "after", header: t("providers.diff.column.after"), cellClassName: "asb-diff-value",
      render: (change) => change.after === null
        ? <span className="asb-diff-remove">{t("providers.diff.removed")}</span>
        : <span className="asb-diff-new">{change.after}</span>,
    },
  ];
  return (
    <Table columns={columns} rows={changes} rowKey={(change) => change.key}
      ariaLabel={label} className="asb-diff" />
  );
}
