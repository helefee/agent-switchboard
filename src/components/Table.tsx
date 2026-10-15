import { Fragment, type ReactNode } from "react";
import { cx } from "@/utils/cx";

export interface TableColumn<T> {
  /** Stable column identity; also the header/cell React key. */
  key: string;
  header: string;
  /** Renders one row's cell content; cell semantics stay caller-owned. */
  render: (row: T) => ReactNode;
  cellClassName?: string;
  align?: "start" | "end";
}

interface Props<T> {
  columns: Array<TableColumn<T>>;
  rows: T[];
  /** Receives the row index too: some tables hold identical entries. */
  rowKey: (row: T, index: number) => string;
  /** Accessible name of the whole table. */
  ariaLabel: string;
  /** Module-owned width or scroll hints appended to the table element. */
  className?: string;
  /** Optional whole-row activation (pointer); keyboard users keep the
   * in-cell buttons. */
  onRowClick?: (row: T) => void;
  /** When it returns a node for a row, a full-width expansion row is
   * emitted directly under that row (open diff views, in-row details);
   * null emits nothing. The expansion row's hover stays neutral. */
  renderExpansion?: (row: T) => ReactNode | null;
}

/**
 * The single data-table renderer: one markup and style contract for every
 * tabular module. The component owns the table's scroll region: the wrap
 * pans wide tables horizontally with the workspace-wide stable gutter and a
 * keyboard-focusable surface, so no module rolls its own overflow wrapper.
 * Headers are in normal flow; a module may opt into sticky headers for its
 * own table in its own stylesheet. Visual values come from
 * styles/base/tables-backups.css via the .asb-table rules.
 */
export function Table<T>({ columns, rows, rowKey, ariaLabel, className, onRowClick, renderExpansion }: Props<T>) {
  return (
    <div className="asb-table-scroll" tabIndex={0}>
      <table className={cx("asb-table", className)} aria-label={ariaLabel}>
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column.key} scope="col" className={cx("asb-table-cell", column.align === "end" && "is-numeric")}>
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => {
            const expansion = renderExpansion?.(row) ?? null;
            return (
              <Fragment key={rowKey(row, index)}>
                <tr
                  className={onRowClick ? "asb-table-row-click" : undefined}
                  onClick={onRowClick ? () => onRowClick(row) : undefined}
                >
                  {columns.map((column) => (
                    <td key={column.key} className={cx("asb-table-cell", column.cellClassName, column.align === "end" && "is-numeric")}>
                      {column.render(row)}
                    </td>
                  ))}
                </tr>
                {expansion !== null && (
                  <tr className="asb-table-expansion-row">
                    <td colSpan={columns.length}>{expansion}</td>
                  </tr>
                )}
              </Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
