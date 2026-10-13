import type { ProviderDiagnosticCheck, ProviderDiagnosticsReport } from "../../api/provider-diagnostics";
import { useI18n } from "../../i18n";
import { localizedMessageText } from "../../i18n/errors";
import { Table, type TableColumn } from "../Table";

export function ProviderDiagnosticReport({ report }: { report: ProviderDiagnosticsReport }) {
  const { t } = useI18n();
  const columns: Array<TableColumn<ProviderDiagnosticCheck>> = [
    { key: "category", header: t("doctor.column.check"), render: (check) => t(`doctor.${check.category}`) },
    {
      key: "status", header: t("doctor.column.status"),
      render: (check) => <span className="asb-doctor-status" data-status={check.status}>{t(`doctor.${check.status}`)}</span>,
    },
    { key: "message", header: t("doctor.column.conclusion"), render: (check) => localizedMessageText(check.message, t) },
    {
      key: "suggestion", header: t("doctor.column.suggestion"),
      render: (check) => check.suggestion ? localizedMessageText(check.suggestion, t) : "—",
    },
  ];
  return <div className="asb-provider-diagnostic-report">
    <p className="asb-scope-note">{t("doctor.localNote")}</p>
    <div className="asb-doctor-checks" role="region" aria-label={t("doctor.local")} tabIndex={0}>
      <Table columns={columns} rows={report.checks} rowKey={(check, index) => `${check.category}-${index}`}
        ariaLabel={t("doctor.local")} className="asb-doctor-checks-table" />
    </div>
  </div>;
}
