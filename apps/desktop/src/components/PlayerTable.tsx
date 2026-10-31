import { useMemo, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
  type VisibilityState,
} from "@tanstack/react-table";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  ChevronLeft,
  ChevronRight,
  Columns3,
  EyeOff,
  Loader2,
  SearchX,
  Users,
} from "lucide-react";
import { usePlayerPage } from "@/lib/query";
import { type PlayerRow } from "@/lib/types";
import { attributeLabel, nationLabel, roleName } from "@/lib/labels";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caTone, cn, formatValue, formatWage, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Button, EmptyState } from "./ui";

const TECHNICAL_COLUMNS = ["passing", "dribbling", "finishing", "first_touch", "technique", "crossing"];
const MENTAL_COLUMNS = ["vision", "decisions", "anticipation", "teamwork", "work_rate", "determination"];
const PHYSICAL_COLUMNS = ["pace", "acceleration", "stamina", "strength", "jumping_reach", "agility"];
const GOALKEEPING_COLUMNS = ["handling", "aerial_reach", "command_of_area", "communication", "kicking", "reflexes"];
const ATTRIBUTE_TABLE_KEYS = [...TECHNICAL_COLUMNS, ...MENTAL_COLUMNS, ...PHYSICAL_COLUMNS, ...GOALKEEPING_COLUMNS];
const OVERVIEW_VISIBILITY: VisibilityState = Object.fromEntries(
  ATTRIBUTE_TABLE_KEYS.map((key) => [`attr_${key}`, false]),
);

export function PlayerTable() {
  const { t, locale } = useI18n();
  const page = usePlayerPage();
  const sort = useAppStore((state) => state.sort);
  const setSort = useAppStore((state) => state.setSort);
  const selectedUid = useAppStore((state) => state.selectedUid);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const compareUids = useAppStore((state) => state.compareUids);
  const toggleCompare = useAppStore((state) => state.toggleCompare);
  const currentPage = useAppStore((state) => state.page);
  const setPage = useAppStore((state) => state.setPage);
  const pageSize = useAppStore((state) => state.pageSize);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({
    height: false,
    foot: false,
    ...OVERVIEW_VISIBILITY,
  });
  const [columnsOpen, setColumnsOpen] = useState(false);

  const columns = useMemo<ColumnDef<PlayerRow>[]>(() => {
    const sortable = (id: string, label: string, align = "left") => ({
      id,
      accessorFn: (row: PlayerRow) => (row as unknown as Record<string, unknown>)[id],
      header: label,
      cell: () => null,
      meta: { align },
    });
    const baseColumns: ColumnDef<PlayerRow>[] = [
      {
        id: "compare",
        header: "",
        size: 30,
        cell: ({ row }) => (
          <button
            type="button"
            aria-label={t("table.addCompare")}
            onClick={(event) => {
              event.stopPropagation();
              toggleCompare(row.original.uid);
            }}
            className={cn(
              "flex h-4 w-4 items-center justify-center rounded-[4px] border text-[9px] font-bold transition-colors",
              compareUids.includes(row.original.uid)
                ? "border-[var(--gold)] bg-[var(--gold)] text-[var(--gold-contrast)]"
                : "border-[var(--line-strong)] bg-[var(--surface)] text-transparent hover:border-[var(--gold)]",
            )}
          >
            ✓
          </button>
        ),
      },
      {
        id: "name",
        accessorFn: (row) => row.name,
        header: t("table.player"),
        size: 224,
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[9px] font-bold text-[var(--muted)]">
              {initials(row.original.name)}
            </div>
            <div className="min-w-0">
              <div className="truncate text-[12px] font-semibold leading-tight text-[var(--ink)]">
                {row.original.name ?? t("common.unnamedPlayer")}
              </div>
              <div className="mt-0.5 flex items-center gap-1.5 text-[10px] leading-tight text-[var(--muted)]">
                <span>{t("common.yearsOld", { age: row.original.age ?? "?" })}</span>
                <span className="text-[var(--line-strong)]">·</span>
                <span className="truncate">
                  {nationLabel(row.original.nation_name, locale) ??
                    (row.original.nation_id != null
                      ? t("common.nationNumber", { id: row.original.nation_id })
                      : t("common.unknownNation"))}
                </span>
                {row.original.gender === 1 && (
                  <span className="rounded bg-[var(--blue-soft)] px-1 text-[8px] font-semibold text-[var(--blue)]">
                    {t("table.female")}
                  </span>
                )}
              </div>
            </div>
          </div>
        ),
      },
      {
        id: "club",
        accessorFn: (row) => row.club_name,
        header: t("table.club"),
        size: 154,
        cell: ({ row }) => (
          <div className="min-w-0">
            <div className="truncate text-[11px] font-medium text-[var(--ink)]">
              {row.original.club_name ?? t("common.freeAgent")}
            </div>
            {row.original.on_loan && (
              <span className="mt-0.5 inline-flex items-center rounded-full bg-[var(--blue-soft)] px-1.5 py-px text-[9px] font-semibold text-[var(--blue)]">
                {t("table.onLoan")}
              </span>
            )}
          </div>
        ),
      },
      {
        id: "status",
        accessorFn: (row) =>
          row.injury_status === "injured"
            ? "injured"
            : row.suspension_count > 0
              ? "suspended"
              : row.on_loan
                ? "loan"
                : row.club_uid == null
                  ? "free"
                  : "available",
        header: t("table.status"),
        size: 104,
        cell: ({ row }) => <StatusBadges row={row.original} />,
      },
      {
        id: "positions",
        accessorFn: (row) => row.positions_text,
        header: t("table.positions"),
        size: 78,
        cell: ({ row }) => (
          <span className="font-mono text-[10px] font-semibold text-[var(--accent-strong)]">
            {row.original.positions_text || "—"}
          </span>
        ),
      },
      {
        id: "role",
        accessorFn: (row) => row.best_role_score,
        header: t("table.bestRole"),
        size: 116,
        cell: ({ row }) => (
          <div className="min-w-0">
            <div className="truncate text-[10px] font-medium text-[var(--ink)]">
              {roleName(
                row.original.best_role_id,
                row.original.best_role_name,
                locale,
              ) ?? "—"}
            </div>
            <div className="text-[9px] text-[var(--muted)]">
              {row.original.best_role_score != null
                ? `${row.original.best_role_score} / 20 · ${row.original.best_role_position ?? ""}`
                : "—"}
            </div>
          </div>
        ),
      },
      {
        id: "ca",
        accessorFn: (row) => row.ca,
        header: "CA",
        size: 42,
        cell: ({ row }) => (
          <span
            className={cn(
              "inline-flex h-5 min-w-8 items-center justify-center rounded border px-1.5 font-mono text-[10px] font-bold",
              caTone(row.original.ca),
            )}
          >
            {row.original.ca}
          </span>
        ),
      },
      {
        id: "pa",
        accessorFn: (row) => row.pa,
        header: "PA",
        size: 42,
        cell: ({ row }) => (
          <span className="font-mono text-[11px] font-semibold text-[var(--muted)]">
            {row.original.pa != null ? row.original.pa : row.original.pa_range_code != null ? "?" : "—"}
          </span>
        ),
      },
      {
        id: "value",
        accessorFn: (row) => row.transfer_value,
        header: t("table.value"),
        size: 76,
        cell: ({ row }) => (
          <span className="whitespace-nowrap font-mono text-[11px] tabular-nums text-[var(--ink)]">
            {formatValue(row.original.transfer_value)}
          </span>
        ),
      },
      {
        id: "wage",
        accessorFn: (row) => row.wage,
        header: t("table.wage"),
        size: 82,
        cell: ({ row }) => (
          <span className="whitespace-nowrap font-mono text-[10px] tabular-nums text-[var(--muted)]">
            {row.original.wage != null ? formatWage(row.original.wage, locale) : "—"}
          </span>
        ),
      },
      {
        id: "contract",
        accessorFn: (row) => row.contract_end,
        header: t("table.contractEnd"),
        size: 80,
        cell: ({ row }) => (
          <span
            className={cn(
              "whitespace-nowrap font-mono text-[10px] tabular-nums",
              row.original.contract_end && row.original.contract_end < "2027-07-01"
                ? "text-[var(--gold)]"
                : "text-[var(--muted)]",
            )}
          >
            {row.original.contract_end ?? "—"}
          </span>
        ),
      },
      {
        id: "height",
        accessorFn: (row) => row.height_cm,
        header: t("table.height"),
        size: 56,
        cell: ({ row }) => (
          <span className="font-mono text-[10px] text-[var(--muted)]">
            {row.original.height_cm ? `${row.original.height_cm} cm` : "—"}
          </span>
        ),
      },
      {
        id: "foot",
        accessorFn: (row) => row.left_foot,
        header: t("table.foot"),
        size: 88,
        cell: ({ row }) => (
          <span className="font-mono text-[10px] text-[var(--muted)]">
            {row.original.left_foot != null && row.original.right_foot != null
              ? `L: ${row.original.left_foot} / R: ${row.original.right_foot}`
              : "—"}
          </span>
        ),
      },
      ...ATTRIBUTE_TABLE_KEYS.map<ColumnDef<PlayerRow>>((key) => ({
        id: `attr_${key}`,
        accessorFn: (row) => row.attributes[key],
        header: attributeLabel(key, locale),
        size: 62,
        cell: ({ row }) => (
          <span className="font-mono text-[10px] font-semibold text-[var(--ink)]">
            {row.original.attributes[key] ?? "—"}
          </span>
        ),
      })),
      {
        id: "hidden",
        header: t("table.hidden"),
        size: 52,
        cell: ({ row }) =>
          hiddenEnabled && row.original.personality ? (
            <span className="font-mono text-[10px] font-semibold text-[var(--gold)]">
              {t("table.professionalismShort")} {row.original.personality.professionalism}
            </span>
          ) : (
            <span className="text-[10px] text-[var(--line-strong)]">—</span>
          ),
      },
    ];
    return baseColumns.map((column) =>
      column.id && ["name", "club", "positions", "ca", "pa", "value", "wage", "contract", "height", "foot"].includes(column.id)
        ? { ...column, meta: { ...(column.meta ?? {}), sortable: true } }
        : column,
    );
  }, [compareUids, hiddenEnabled, locale, t, toggleCompare]);

  const table = useReactTable({
    data: page.data?.rows ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    enableColumnResizing: true,
    columnResizeMode: "onChange",
    defaultColumn: { minSize: 44, maxSize: 520 },
    state: { columnVisibility },
    onColumnVisibilityChange: setColumnVisibility,
  });

  const total = page.data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));
  const sortableIds: Record<string, string> = {
    name: "name",
    club: "club",
    positions: "positions",
    role: "role",
    status: "injury_status",
    ca: "ca",
    pa: "pa",
    value: "value",
    wage: "wage",
    contract: "contract_end",
    height: "height",
    foot: "left_foot",
  };

  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col bg-[var(--canvas)]">
      <div className="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-[var(--line)] px-3">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-semibold text-[var(--ink)]">
            {page.isFetching ? (
              <span className="inline-flex items-center gap-1.5 text-[var(--muted)]">
                <Loader2 size={11} className="animate-spin" />
                {t("table.loading")}
              </span>
            ) : (
              t("table.players", { count: formatNumber(total, locale) })
            )}
          </span>
          {page.isPlaceholderData && (
            <span className="text-[10px] text-[var(--muted)]">{t("table.updating")}</span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <div className="relative">
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-1.5 text-[10px]"
              onClick={() => setColumnsOpen((value) => !value)}
            >
              <Columns3 size={12} />
              {t("table.columns")}
            </Button>
            {columnsOpen && (
              <div className="absolute right-0 top-7 z-30 w-44 rounded-[9px] border border-[var(--line)] bg-[var(--surface)] p-1.5 shadow-[var(--shadow-pop)]">
                <div className="mb-1.5 grid grid-cols-2 gap-1 border-b border-[var(--line)] pb-1.5">
                  {[
                    ["overview", t("table.overview"), null],
                    ["technical", t("table.technical"), TECHNICAL_COLUMNS],
                    ["mental", t("table.mental"), MENTAL_COLUMNS],
                    ["physical", t("table.physical"), PHYSICAL_COLUMNS],
                    ["goalkeeping", t("table.goalkeeping"), GOALKEEPING_COLUMNS],
                  ].map(([key, label, keys]) => (
                    <button
                      key={key as string}
                      type="button"
                      onClick={() => {
                        const visibility: VisibilityState = {};
                        for (const column of table.getAllLeafColumns()) {
                          visibility[column.id] = !column.id.startsWith("attr_");
                        }
                        visibility.height = false;
                        visibility.foot = false;
                        if (keys) {
                          for (const key of keys as string[]) visibility[`attr_${key}`] = true;
                        }
                        setColumnVisibility(visibility);
                        setColumnsOpen(false);
                      }}
                      className="rounded-[5px] border border-[var(--line)] px-1.5 py-1 text-[10px] text-[var(--muted)] hover:border-[var(--accent-line)] hover:bg-[var(--accent-soft)] hover:text-[var(--accent-strong)]"
                    >
                      {label as string}
                    </button>
                  ))}
                </div>
                <div className="thin-scroll max-h-72 overflow-auto">
                {table
                  .getAllLeafColumns()
                  .filter((column) => column.id !== "compare" && column.id !== "name")
                  .map((column) => (
                    <button
                      key={column.id}
                      type="button"
                      onClick={() => column.toggleVisibility()}
                      className="flex w-full items-center justify-between rounded-[5px] px-2 py-1.5 text-left text-[11px] hover:bg-[var(--surface-2)]"
                    >
                      <span>{String(column.columnDef.header)}</span>
                      <span className={column.getIsVisible() ? "text-[var(--accent)]" : "text-[var(--line-strong)]"}>
                        {column.getIsVisible() ? t("table.shown") : <EyeOff size={11} />}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
          <div className="h-4 w-px bg-[var(--line)]" />
          <span className="text-[10px] text-[var(--muted)]">
            {compareUids.length > 0
              ? t("table.compareSelected", { count: compareUids.length })
              : t("table.compareHint")}
          </span>
        </div>
      </div>

      <div className="thin-scroll min-h-0 flex-1 overflow-auto">
        {page.isLoading ? (
          <TableSkeleton />
        ) : total === 0 ? (
          <EmptyState
            icon={<SearchX size={20} />}
            title={t("table.emptyTitle")}
            description={t("table.emptyDescription")}
          />
        ) : (
          <table className="w-full border-separate border-spacing-0 text-left" style={{ tableLayout: "fixed" }}>
            <thead className="sticky top-0 z-20 bg-[var(--surface)]/95 backdrop-blur">
              <tr>
                {table.getHeaderGroups()[0].headers.map((header) => {
                  const sortKey = sortableIds[header.id];
                  const sortIndex = sortKey ? sort.findIndex((rule) => rule.by === sortKey) : -1;
                  const activeRule = sortIndex >= 0 ? sort[sortIndex] : null;
                  return (
                    <th
                      key={header.id}
                      style={{ width: header.getSize() }}
                      className={cn(
                        "border-b border-[var(--line)] px-2 py-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]",
                        header.id === "name" && "sticky left-0 z-30 bg-[var(--surface)]",
                        header.id === "compare" && "sticky left-0 z-30 bg-[var(--surface)]",
                        sortKey && "cursor-pointer select-none hover:text-[var(--ink)]",
                        "relative",
                      )}
                      onClick={(event) => sortKey && setSort(sortKey, event.shiftKey)}
                      title={sortKey ? t("table.sortHint") : undefined}
                    >
                      <span className="inline-flex items-center gap-1">
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {sortKey &&
                          (activeRule ? (
                            <>
                              {activeRule.dir === "asc" ? (
                                <ArrowUp size={10} className="text-[var(--accent)]" />
                              ) : (
                                <ArrowDown size={10} className="text-[var(--accent)]" />
                              )}
                              {sort.length > 1 && (
                                <span className="rounded bg-[var(--accent-soft)] px-1 font-mono text-[8px] text-[var(--accent-strong)]">
                                  {sortIndex + 1}
                                </span>
                              )}
                            </>
                          ) : (
                            <ArrowUpDown size={10} className="opacity-30" />
                          ))}
                      </span>
                      {header.column.getCanResize() && (
                        <span
                          role="separator"
                          aria-orientation="vertical"
                          onMouseDown={header.getResizeHandler()}
                          onTouchStart={header.getResizeHandler()}
                          onClick={(event) => event.stopPropagation()}
                          className="absolute right-0 top-0 h-full w-1 cursor-col-resize select-none hover:bg-[var(--accent-line)]"
                        />
                      )}
                    </th>
                  );
                })}
              </tr>
            </thead>
            <tbody>
              {table.getRowModel().rows.map((row) => {
                const isSelected = row.original.uid === selectedUid;
                const isCompare = compareUids.includes(row.original.uid);
                return (
                  <tr
                    key={row.id}
                    data-selected={isSelected}
                    data-compare={isCompare}
                    tabIndex={0}
                    aria-selected={isSelected}
                    className="table-row group focus-ring cursor-pointer border-b border-[var(--line)]/70"
                    onClick={() => setSelectedUid(row.original.uid)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedUid(row.original.uid);
                      }
                    }}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <td
                        key={cell.id}
                        style={{ width: cell.column.getSize() }}
                        className={cn(
                          "h-[38px] border-b border-[var(--line)]/60 px-2 align-middle",
                          cell.column.id === "name" &&
                            "sticky left-0 z-10 bg-[var(--surface)] group-hover:bg-[var(--surface-2)]",
                          cell.column.id === "compare" &&
                            "sticky left-0 z-10 bg-[var(--surface)]",
                          (cell.column.id === "name" || cell.column.id === "compare") &&
                            isSelected &&
                            "bg-[var(--accent-soft)]",
                        )}
                      >
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div className="flex h-9 shrink-0 items-center justify-between border-t border-[var(--line)] bg-[var(--surface)] px-3">
        <div className="flex items-center gap-2 text-[10px] text-[var(--muted)]">
          <Users size={11} />
          <span>{t("table.serverPagination", { size: pageSize })}</span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            disabled={currentPage <= 0}
            onClick={() => setPage(currentPage - 1)}
            aria-label={t("table.prevPage")}
          >
            <ChevronLeft size={13} />
          </Button>
          <span className="min-w-16 text-center font-mono text-[10px] text-[var(--muted)]">
            {currentPage + 1} / {totalPages}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            disabled={currentPage + 1 >= totalPages}
            onClick={() => setPage(currentPage + 1)}
            aria-label={t("table.nextPage")}
          >
            <ChevronRight size={13} />
          </Button>
        </div>
      </div>
    </section>
  );
}

function StatusBadges({ row }: { row: PlayerRow }) {
  const { t } = useI18n();
  const badges: { label: string; tone: string; title?: string }[] = [];
  if (row.transfer_listed) {
    badges.push({
      label: t("table.transferListedShort"),
      tone: "bg-[var(--gold-soft)] text-[var(--gold)]",
      title: t("table.transferListed"),
    });
  }
  if (row.loan_listed) {
    badges.push({
      label: t("table.loanListedShort"),
      tone: "bg-[var(--accent-soft)] text-[var(--accent-strong)]",
      title: t("table.loanListed"),
    });
  }
  if (row.injury_status === "injured") {
    badges.push({
      label: t("table.injuredShort"),
      tone: "bg-[var(--danger-soft)] text-[var(--danger)]",
      title: row.injury_until
        ? t("table.injuredUntil", { date: row.injury_until })
        : t("table.injured"),
    });
  } else if (row.recent_injury) {
    badges.push({
      label: t("table.injuredShort"),
      tone: "bg-[var(--surface-3)] text-[var(--muted)]",
      title: t("table.recentInjury", { date: row.recent_injury }),
    });
  }
  if (row.suspension_count > 0) {
    badges.push({
      label: t("table.suspendedShort"),
      tone: "bg-[var(--gold-soft)] text-[var(--gold)]",
      title: t(
        row.suspension_count === 1 ? "table.suspensionOne" : "table.suspensions",
        { count: row.suspension_count },
      ),
    });
  }
  if (row.on_loan) {
    badges.push({
      label: t("table.loanShort"),
      tone: "bg-[var(--blue-soft)] text-[var(--blue)]",
      title: t("table.loanActive"),
    });
  }
  if (badges.length === 0) {
    return <span className="text-[10px] text-[var(--line-strong)]">—</span>;
  }
  return (
    <div className="flex items-center gap-0.5">
      {badges.map((badge) => (
        <span
          key={badge.label}
          title={badge.title}
          className={cn(
            "inline-flex h-4 min-w-4 items-center justify-center rounded px-1 text-[9px] font-bold",
            badge.tone,
          )}
        >
          {badge.label}
        </span>
      ))}
    </div>
  );
}

function TableSkeleton() {
  return (
    <div className="space-y-1 p-3">
      {Array.from({ length: 14 }).map((_, index) => (
        <div
          key={index}
          className="flex h-9 items-center gap-3 rounded-[6px] bg-[var(--surface-2)] px-3"
          style={{ opacity: 1 - index * 0.045 }}
        >
          <div className="h-6 w-6 rounded-full bg-[var(--surface-3)]" />
          <div className="h-2.5 w-32 rounded bg-[var(--surface-3)]" />
          <div className="h-2.5 w-24 rounded bg-[var(--surface-3)]" />
          <div className="ml-auto h-2.5 w-10 rounded bg-[var(--surface-3)]" />
        </div>
      ))}
    </div>
  );
}
