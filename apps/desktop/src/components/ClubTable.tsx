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
import { corporateFacilitiesLabel, nationLabel } from "@/lib/labels";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caTone, cn, formatMoney, formatValue, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Button, EmptyState } from "./ui";

export function ClubTable() {
  const { t, locale } = useI18n();
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

  if (page.isLoading && rows.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center gap-2 text-[11px] text-[var(--muted)]">
        <Loader2 size={14} className="animate-spin" />
        {t("clubs.loading")}
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <EmptyState
          icon={<SearchX size={20} />}
          title={t("clubs.empty")}
          description={t("clubs.searchPlaceholder")}
        />
      </div>
    );
  }

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
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="thin-scroll min-h-0 min-w-0 flex-1 overflow-auto">
        <table className="w-full min-w-[760px] border-collapse 2xl:min-w-[1180px]">
          <thead>
            <tr>
              {header("name", t("clubs.table.club"))}
              {header("nation", t("clubs.table.nation"))}
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
            {rows.map((club) => (
              <tr
                key={club.uid}
                onClick={() => setSelectedClubUid(club.uid)}
                className={cn(
                  "cursor-pointer border-t border-[var(--line)]/60 text-[11px] transition-colors hover:bg-[var(--surface-2)]",
                  selectedClubUid === club.uid && "bg-[var(--accent-soft)]",
                )}
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
                  {corporateFacilitiesLabel(club.corporate_facilities, locale) ?? "—"}
                </td>
                <td className="hidden px-2 py-1.5 text-right font-mono text-[var(--muted)] 2xl:table-cell">
                  {club.stadium_capacity != null
                    ? formatNumber(club.stadium_capacity, locale)
                    : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex items-center justify-between border-t border-[var(--line)] bg-[var(--surface)] px-3 py-1.5 text-[10px] text-[var(--muted)]">
        <span className="inline-flex items-center gap-1.5">
          <Shield size={12} />
          {formatNumber(total, locale)} {t("clubs.title")}
          <span className="text-[var(--line-strong)]">·</span>
          {t("clubs.table.page", { page: currentPage + 1, pages })}
        </span>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5"
            disabled={currentPage <= 0}
            onClick={() => setPage(currentPage - 1)}
          >
            <ChevronLeft size={12} />
            {t("clubs.table.prev")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5"
            disabled={currentPage + 1 >= pages}
            onClick={() => setPage(currentPage + 1)}
          >
            {t("clubs.table.next")}
            <ChevronRight size={12} />
          </Button>
        </div>
      </div>
    </div>
  );
}
