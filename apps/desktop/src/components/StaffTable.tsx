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
  BriefcaseBusiness,
  ChevronLeft,
  ChevronRight,
  Columns3,
  EyeOff,
  Loader2,
  SearchX,
} from "lucide-react";
import { useStaffPage } from "@/lib/query";
import {
  STAFF_ATTRIBUTE_GROUPS,
  STAFF_ROLE_GREEN_THRESHOLD,
  STAFF_ROLE_KEYS,
  type StaffRow,
} from "@/lib/types";
import {
  nationLabel,
  staffAttributeLabel,
  staffJobLabel,
  staffRoleLabel,
} from "@/lib/labels";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caLabelClass, cn, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Button, EmptyState } from "./ui";

const STAFF_ATTRIBUTE_COLUMNS = STAFF_ATTRIBUTE_GROUPS.flatMap((group) => group.keys);
const OVERVIEW_VISIBILITY: VisibilityState = Object.fromEntries([
  ...STAFF_ATTRIBUTE_COLUMNS.map((key) => [`attr_${key}`, false]),
  ...STAFF_ROLE_KEYS.map((key) => [`role_${key}`, false]),
]);

export function StaffTable() {
  const { t, locale, formatWage } = useI18n();
  const page = useStaffPage();
  const sort = useAppStore((state) => state.staffSort);
  const setSort = useAppStore((state) => state.setStaffSort);
  const selectedUid = useAppStore((state) => state.selectedStaffUid);
  const setSelectedUid = useAppStore((state) => state.setSelectedStaffUid);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const currentPage = useAppStore((state) => state.staffPage);
  const setPage = useAppStore((state) => state.setStaffPage);
  const pageSize = useAppStore((state) => state.staffPageSize);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>(OVERVIEW_VISIBILITY);
  const [columnsOpen, setColumnsOpen] = useState(false);

  const columns = useMemo<ColumnDef<StaffRow>[]>(() => {
    const baseColumns: ColumnDef<StaffRow>[] = [
      {
        id: "name",
        accessorFn: (row) => row.name,
        header: t("staff.table.person"),
        size: 220,
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[9px] font-bold text-[var(--muted)]">
              {initials(row.original.name)}
            </div>
            <div className="min-w-0 flex-1">
              <div className="truncate text-[12px] font-semibold leading-tight text-[var(--ink)]">
                {row.original.name ?? t("common.unnamedStaff")}
              </div>
              <div className="mt-0.5 flex min-w-0 items-center gap-1.5 whitespace-nowrap text-[10px] leading-tight text-[var(--muted)]">
                <span className="shrink-0">
                  {t("common.yearsOld", { age: row.original.age ?? "?" })}
                </span>
                <span className="shrink-0 text-[var(--line-strong)]">·</span>
                <span className="min-w-0 truncate">
                  {nationLabel(row.original.nation_name, locale) ??
                    (row.original.nation_id != null
                      ? t("common.nationNumber", { id: row.original.nation_id })
                      : t("common.unknownNation"))}
                </span>
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
        cell: ({ row }) =>
          row.original.club_uid != null ? (
            <button
              type="button"
              className="block w-full min-w-0 max-w-full overflow-hidden text-left"
              title={row.original.club_name ?? t("common.unknownClub")}
              onClick={(event) => {
                event.stopPropagation();
                setSelectedClubUid(row.original.club_uid);
                setViewMode("clubs");
              }}
            >
              <div className="truncate text-[11px] font-medium text-[var(--ink)] hover:text-[var(--accent-strong)] hover:underline">
                {row.original.club_name ?? t("common.unknownClub")}
              </div>
            </button>
          ) : (
            <span className="text-[10px] text-[var(--line-strong)]">—</span>
          ),
      },
      {
        id: "job",
        accessorFn: (row) => row.job_code,
        header: t("staff.table.job"),
        size: 130,
        cell: ({ row }) => (
          <div className="min-w-0">
            <div className="truncate text-[10px] font-medium text-[var(--ink)]">
              {staffJobLabel(row.original.job_code, row.original.job_name, locale) ??
                t("common.unknownJob")}
            </div>
            {!row.original.has_contract && (
              <div className="text-[9px] text-[var(--muted)]">{t("staff.table.noContract")}</div>
            )}
          </div>
        ),
      },
      {
        id: "role",
        accessorFn: (row) => row.best_role_score,
        header: t("staff.table.bestRole"),
        size: 132,
        cell: ({ row }) => {
          const key =
            row.original.best_role_index != null
              ? STAFF_ROLE_KEYS[row.original.best_role_index]
              : undefined;
          const label = staffRoleLabel(key, row.original.best_role_name, locale);
          const score = row.original.best_role_score;
          return (
            <div className="min-w-0">
              <div className="truncate text-[10px] font-medium text-[var(--ink)]">
                {label ?? "—"}
              </div>
              <div
                className={cn(
                  "font-mono text-[9px]",
                  score != null && score >= 15
                    ? "text-[var(--gold)]"
                    : score != null && score >= STAFF_ROLE_GREEN_THRESHOLD
                      ? "text-[var(--accent-strong)]"
                      : "text-[var(--muted)]",
                )}
              >
                {score != null ? `${score} / 20` : "—"}
              </div>
            </div>
          );
        },
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
              caLabelClass,
            )}
          >
            {row.original.ca ?? "—"}
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
            {row.original.pa ?? "—"}
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
            {row.original.wage != null ? formatWage(row.original.wage) : "—"}
          </span>
        ),
      },
      {
        id: "contract",
        accessorFn: (row) => row.contract_end,
        header: t("table.contractEnd"),
        size: 80,
        cell: ({ row }) => (
          <span className="whitespace-nowrap font-mono text-[10px] tabular-nums text-[var(--muted)]">
            {row.original.contract_end ?? "—"}
          </span>
        ),
      },
      ...STAFF_ATTRIBUTE_COLUMNS.map<ColumnDef<StaffRow>>((key) => ({
        id: `attr_${key}`,
        accessorFn: (row) => row.attributes?.[key],
        header: staffAttributeLabel(key, locale),
        size: 72,
        cell: ({ row }) => (
          <span className="font-mono text-[10px] font-semibold text-[var(--ink)]">
            {row.original.attributes?.[key] ?? "—"}
          </span>
        ),
      })),
      ...STAFF_ROLE_KEYS.map<ColumnDef<StaffRow>>((key, index) => ({
        id: `role_${key}`,
        accessorFn: (row) => row.roles?.[index],
        header: staffRoleLabel(key, null, locale) ?? key,
        size: 82,
        cell: ({ row }) => {
          const score = row.original.roles?.[index];
          return (
            <span
              className={cn(
                "font-mono text-[10px] font-semibold",
                score != null && score >= STAFF_ROLE_GREEN_THRESHOLD
                  ? "text-[var(--accent-strong)]"
                  : "text-[var(--muted)]",
              )}
            >
              {score ?? "—"}
            </span>
          );
        },
      })),
    ];
    return baseColumns.map((column) =>
      column.id &&
      ["name", "club", "job", "role", "ca", "pa", "wage", "contract"].includes(column.id)
        ? { ...column, meta: { ...(column.meta ?? {}), sortable: true } }
        : column,
    );
  }, [formatWage, locale, setSelectedClubUid, setViewMode, t]);

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
    job: "job",
    role: "role",
    ca: "ca",
    pa: "pa",
    wage: "wage",
    contract: "contract_end",
  };

  const presets: [string, string, readonly string[] | "roles" | null][] = [
    ["overview", t("table.overview"), null],
    [
      "profile",
      t("staff.table.profile"),
      STAFF_ATTRIBUTE_GROUPS[0].keys,
    ],
    [
      "judging",
      t("staff.table.judging"),
      STAFF_ATTRIBUTE_GROUPS[1].keys,
    ],
    [
      "coaching",
      t("staff.table.coaching"),
      STAFF_ATTRIBUTE_GROUPS[2].keys,
    ],
    [
      "support",
      t("staff.table.support"),
      STAFF_ATTRIBUTE_GROUPS[3].keys,
    ],
    ["roles", t("staff.table.roleScores"), "roles"],
  ];

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
              t("staff.table.title", { count: formatNumber(total, locale) })
            )}
          </span>
          {page.isPlaceholderData && (
            <span className="text-[10px] text-[var(--muted)]">{t("table.updating")}</span>
          )}
        </div>
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
            <div className="absolute right-0 top-7 z-30 w-56 rounded-[9px] border border-[var(--line)] bg-[var(--surface)] p-1.5 shadow-[var(--shadow-pop)]">
              <div className="mb-1.5 grid grid-cols-2 gap-1 border-b border-[var(--line)] pb-1.5">
                {presets.map(([key, label, keys]) => (
                  <button
                    key={key}
                    type="button"
                    onClick={() => {
                      const visibility: VisibilityState = {};
                      for (const column of table.getAllLeafColumns()) {
                        visibility[column.id] = !column.id.startsWith("attr_") && !column.id.startsWith("role_");
                      }
                      if (keys === "roles") {
                        for (const roleKey of STAFF_ROLE_KEYS) visibility[`role_${roleKey}`] = true;
                      } else if (keys) {
                        for (const attributeKey of keys) visibility[`attr_${attributeKey}`] = true;
                      }
                      setColumnVisibility(visibility);
                      setColumnsOpen(false);
                    }}
                    className="rounded-[5px] border border-[var(--line)] px-1.5 py-1 text-[10px] text-[var(--muted)] hover:border-[var(--accent-line)] hover:bg-[var(--accent-soft)] hover:text-[var(--accent-strong)]"
                  >
                    {label}
                  </button>
                ))}
              </div>
              <div className="thin-scroll max-h-72 overflow-auto">
                {table
                  .getAllLeafColumns()
                  .filter((column) => column.id !== "name")
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
      </div>

      <div className="thin-scroll min-h-0 flex-1 overflow-auto">
        {page.isLoading ? (
          <TableSkeleton />
        ) : total === 0 ? (
          <EmptyState
            icon={<SearchX size={20} />}
            title={t("staff.table.emptyTitle")}
            description={t("staff.table.emptyDescription")}
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
                        "relative border-b border-[var(--line)] px-2 py-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]",
                        header.id === "name" && "sticky left-0 z-30 bg-[var(--surface)]",
                        sortKey && "cursor-pointer select-none hover:text-[var(--ink)]",
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
                return (
                  <tr
                    key={row.id}
                    data-selected={isSelected}
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
                          cell.column.id === "name" && isSelected && "bg-[var(--accent-soft)]",
                          (cell.column.id === "name" || cell.column.id === "club") &&
                            "overflow-hidden",
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
          <BriefcaseBusiness size={11} />
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
