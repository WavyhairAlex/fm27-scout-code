import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  ChevronLeft,
  ChevronRight,
  Loader2,
  SearchX,
  Shield,
} from "lucide-react";
import { useClubPage } from "@/lib/query";
import { facilityRatingLabel, nationLabel } from "@/lib/labels";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caTone, cn, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Button, EmptyState } from "./ui";

export function ClubTable() {
  const { t, locale, formatMoney, formatValue } = useI18n();
  const page = useClubPage();
  const selectedClubUid = useAppStore((state) => state.selectedClubUid);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const sort = useAppStore((state) => state.clubSort);
  const setSort = useAppStore((state) => state.setClubSort);
  const currentPage = useAppStore((state) => state.clubPage);
  const setPage = useAppStore((state) => state.setClubPage);
  const rows = page.data?.rows ?? [];
  const total = page.data?.total ?? 0;
  const pageSize = page.data?.page_size ?? 100;
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const primary = sort[0];

  const header = (
    id: string,
    label: string,
    align: "left" | "right" = "left",
    extra?: string,
  ) => {
    const active = primary?.by === id;
    const Icon = active ? (primary.dir === "desc" ? ArrowDown : ArrowUp) : ArrowUpDown;
    return (
      <th
        className={cn(
          "sticky top-0 z-10 whitespace-nowrap bg-[var(--surface-2)] px-2 py-2 text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]",
          align === "right" ? "text-right" : "text-left",
          extra,
        )}
      >
        <button
          type="button"
          className="inline-flex items-center gap-1 hover:text-[var(--ink)]"
          onClick={() => setSort(id)}
        >
          {align === "right" && <Icon size={10} />}
          {label}
          {align === "left" && <Icon size={10} />}
        </button>
      </th>
    );
  };

  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col bg-[var(--canvas)]">
      <div className="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-[var(--line)] px-3">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-semibold text-[var(--ink)]">
            {page.isFetching ? (
              <span className="inline-flex items-center gap-1.5 text-[var(--muted)]">
                <Loader2 size={11} className="animate-spin" />
                {t("clubs.loading")}
              </span>
            ) : (
              `${formatNumber(total, locale)} ${t("clubs.title")}`
            )}
          </span>
          {page.isPlaceholderData && (
            <span className="text-[10px] text-[var(--muted)]">{t("table.updating")}</span>
          )}
        </div>
      </div>

      <div className="thin-scroll min-h-0 min-w-0 flex-1 overflow-auto">
        {page.isLoading ? (
          <ClubTableSkeleton />
        ) : total === 0 ? (
          <EmptyState
            icon={<SearchX size={20} />}
            title={t("clubs.empty")}
            description={t("clubs.searchPlaceholder")}
          />
        ) : (
          <table className="w-full min-w-[860px] border-collapse 2xl:min-w-[1280px]">
            <thead>
              <tr>
                {header("name", t("clubs.table.club"))}
                {header("nation", t("clubs.table.nation"))}
                {header(
                  "league",
                  t("clubs.table.league"),
                  "left",
                  "hidden xl:table-cell",
                )}
                {header("reputation", t("clubs.table.reputation"), "right")}
                {header(
                  "last_position",
                  t("clubs.table.lastPosition"),
                  "right",
                  "hidden 2xl:table-cell",
                )}
                {header("player_count", t("clubs.table.players"), "right")}
                {header("average_ca", t("clubs.table.avgCa"), "right")}
                {header(
                  "total_value",
                  t("clubs.table.value"),
                  "right",
                  "hidden 2xl:table-cell",
                )}
                {header("balance", t("clubs.table.balance"), "right")}
                {header("transfer_budget", t("clubs.table.transferBudget"), "right")}
                {header("wage_budget", t("clubs.table.wageBudget"), "right")}
                {header(
                  "corporate_facilities",
                  t("clubs.table.facilities"),
                  "right",
                  "hidden 2xl:table-cell",
                )}
                {header(
                  "stadium_capacity",
                  t("clubs.table.stadium"),
                  "right",
                  "hidden 2xl:table-cell",
                )}
              </tr>
            </thead>
            <tbody>
              {rows.map((club) => {
                const selected = selectedClubUid === club.uid;
                return (
                  <tr
                    key={club.uid}
                    data-selected={selected}
                    tabIndex={0}
                    aria-selected={selected}
                    onClick={() => setSelectedClubUid(club.uid)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedClubUid(club.uid);
                      }
                    }}
                    className="table-row focus-ring cursor-pointer border-b border-[var(--line)]/70 text-[11px]"
                  >
                    <td className="px-2 py-1.5">
                      <div className="flex items-center gap-2">
                        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-[var(--line)] bg-[var(--surface-2)] text-[9px] font-bold text-[var(--muted)]">
                          {initials(club.name)}
                        </div>
                        <div className="min-w-0">
                          <div className="max-w-[220px] truncate font-semibold text-[var(--ink)]">
                            {club.name}
                          </div>
                          <div className="flex items-center gap-1.5 text-[9px] text-[var(--muted)]">
                            <span className="font-mono">#{club.uid}</span>
                            {club.finance_month && <span>· {club.finance_month}</span>}
                          </div>
                        </div>
                      </div>
                    </td>
                    <td className="max-w-[130px] truncate px-2 py-1.5 text-[var(--muted)]">
                      {nationLabel(club.nation_name, locale) ?? `#${club.nation_id}`}
                    </td>
                    <td className="hidden max-w-[180px] truncate px-2 py-1.5 text-[10px] text-[var(--muted)] xl:table-cell">
                      {club.league_name ?? "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {club.reputation ?? "—"}
                    </td>
                    <td className="hidden px-2 py-1.5 text-right font-mono text-[var(--muted)] 2xl:table-cell">
                      {club.last_league_position ?? "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {club.player_count}
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <span
                        className={cn(
                          "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                          club.average_ca != null
                            ? caTone(Math.round(club.average_ca))
                            : "border-[var(--line)] text-[var(--muted)]",
                        )}
                      >
                        {club.average_ca != null ? club.average_ca.toFixed(1) : "—"}
                      </span>
                    </td>
                    <td className="hidden px-2 py-1.5 text-right font-mono text-[var(--muted)] 2xl:table-cell">
                      {formatValue(club.total_value ?? undefined)}
                    </td>
                    <td
                      className={cn(
                        "px-2 py-1.5 text-right font-mono",
                        (club.balance ?? 0) < 0 ? "text-[var(--danger)]" : "text-[var(--ink)]",
                      )}
                    >
                      {formatMoney(club.balance)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatMoney(club.transfer_budget_remaining)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatMoney(club.wage_budget_weekly)}
                    </td>
                    <td className="hidden px-2 py-1.5 text-right text-[10px] text-[var(--muted)] 2xl:table-cell">
                      {facilityRatingLabel(club.corporate_facilities, locale) ?? "—"}
                    </td>
                    <td className="hidden px-2 py-1.5 text-right font-mono text-[var(--muted)] 2xl:table-cell">
                      {club.stadium_capacity != null
                        ? formatNumber(club.stadium_capacity, locale)
                        : "—"}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div className="flex h-9 shrink-0 items-center justify-between border-t border-[var(--line)] bg-[var(--surface)] px-3">
        <div className="flex items-center gap-2 text-[10px] text-[var(--muted)]">
          <Shield size={11} />
          <span>
            {formatNumber(total, locale)} {t("clubs.title")}
          </span>
          <span className="text-[var(--line-strong)]">·</span>
          <span>{t("clubs.table.page", { page: currentPage + 1, pages })}</span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            disabled={currentPage <= 0}
            onClick={() => setPage(currentPage - 1)}
            aria-label={t("clubs.table.prev")}
          >
            <ChevronLeft size={13} />
          </Button>
          <span className="min-w-16 text-center font-mono text-[10px] text-[var(--muted)]">
            {currentPage + 1} / {pages}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            disabled={currentPage + 1 >= pages}
            onClick={() => setPage(currentPage + 1)}
            aria-label={t("clubs.table.next")}
          >
            <ChevronRight size={13} />
          </Button>
        </div>
      </div>
    </section>
  );
}

function ClubTableSkeleton() {
  return (
    <div className="space-y-1 p-3">
      {Array.from({ length: 14 }).map((_, index) => (
        <div
          key={index}
          className="flex h-9 items-center gap-3 rounded-[6px] bg-[var(--surface-2)] px-3"
          style={{ opacity: 1 - index * 0.045 }}
        >
          <div className="h-6 w-6 rounded-lg bg-[var(--surface-3)]" />
          <div className="h-2.5 w-32 rounded bg-[var(--surface-3)]" />
          <div className="h-2.5 w-24 rounded bg-[var(--surface-3)]" />
          <div className="ml-auto h-2.5 w-10 rounded bg-[var(--surface-3)]" />
        </div>
      ))}
    </div>
  );
}
