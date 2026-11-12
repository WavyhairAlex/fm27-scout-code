import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Building2,
  ChevronRight,
  MapPin,
  Pencil,
  Shield,
  X,
} from "lucide-react";
import { useClubDetail, useSquadAnalysis } from "@/lib/query";
import { facilityRatingLabel, nationLabel } from "@/lib/labels";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caLabelClass, cn, formatDate, initials, paTone } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { SquadNeedsSection } from "./SquadNeeds";
import { Badge, Button, EmptyState } from "./ui";
import { EditClubDialog } from "./EditClubDialog";

type ClubTab = "overview" | "squad" | "finances" | "stadium";

export function ClubInspector() {
  const { t, locale, formatMoney, formatValue, formatWage } = useI18n();
  const selectedClubUid = useAppStore((state) => state.selectedClubUid);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const setClubEditOpen = useAppStore((state) => state.setClubEditOpen);
  const { data: detail, isLoading } = useClubDetail(selectedClubUid);
  const [tab, setTab] = useState<ClubTab>("overview");
  const tabBarRef = useRef<HTMLDivElement>(null);
  const [tabBarWidth, setTabBarWidth] = useState(0);

  useEffect(() => {
    setTab("overview");
  }, [selectedClubUid]);

  useLayoutEffect(() => {
    const element = tabBarRef.current;
    if (!element) return;
    const measure = () => {
      const width = Math.ceil(element.scrollWidth);
      setTabBarWidth((current) => (current === width ? current : width));
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [selectedClubUid, locale, detail]);

  if (selectedClubUid == null) {
    return (
      <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]">
        <EmptyState
          icon={<Shield size={20} />}
          title={t("clubs.noSelection")}
          description={t("clubs.noSelectionHint")}
        />
      </aside>
    );
  }

  if (isLoading || !detail) {
    return (
      <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)] p-4">
        <div className="h-20 animate-pulse rounded-xl bg-[var(--surface-2)]" />
        <div className="mt-3 h-6 w-2/3 animate-pulse rounded bg-[var(--surface-2)]" />
        <div className="mt-6 space-y-2">
          {Array.from({ length: 7 }).map((_, index) => (
            <div key={index} className="h-8 animate-pulse rounded bg-[var(--surface-2)]" />
          ))}
        </div>
      </aside>
    );
  }

  const club = detail.club;
  const latestFinance = detail.finance_months.at(-1) ?? null;

  return (
    <aside
      className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]"
      style={tabBarWidth > 0 ? { minWidth: tabBarWidth } : undefined}
    >
      <div className="flex items-center justify-between border-b border-[var(--line)] px-3 py-2">
        <div className="flex items-center gap-2 text-[11px] font-semibold text-[var(--muted)]">
          <Building2 size={13} className="text-[var(--accent)]" />
          {t("clubs.inspector.profile")}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5 text-[10px]"
            onClick={() => setClubEditOpen(true)}
          >
            <Pencil size={11} />
            {t("clubs.inspector.edit")}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => setSelectedClubUid(null)}
            aria-label={t("common.close")}
          >
            <X size={14} />
          </Button>
        </div>
      </div>

      <div className="border-b border-[var(--line)] px-4 pb-3 pt-3">
        <div className="flex items-start gap-3">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-[var(--line)] bg-[var(--surface-2)] text-sm font-bold text-[var(--muted)]">
            {initials(club.name)}
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-[17px] font-semibold leading-tight tracking-tight text-[var(--ink)]">
              {club.name}
            </h2>
            <div className="mt-0.5 truncate text-[11px] text-[var(--muted)]">
              {nationLabel(club.nation_name, locale) ?? `#${club.nation_id}`}
              {club.city_id != null && <span> · {t("clubs.inspector.city")} {club.city_id}</span>}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-1.5">
              {club.league_name && <Badge tone="outline">{club.league_name}</Badge>}
              {club.reputation != null && (
                <Badge tone="outline">
                  {t("clubs.table.reputation")} {club.reputation}
                </Badge>
              )}
              {club.last_league_position != null && (
                <Badge tone="outline">
                  {t("clubs.table.lastPosition")} {club.last_league_position}
                </Badge>
              )}
              {club.gender === 1 && <Badge tone="blue">{t("clubs.inspector.women")}</Badge>}
            </div>
          </div>
        </div>
        <div className="mt-3 grid grid-cols-3 gap-1.5">
          <MiniStat
            label={t("clubs.table.balance")}
            value={formatMoney(latestFinance?.balance ?? null)}
            tone={(latestFinance?.balance ?? 0) < 0 ? "danger" : "default"}
          />
          <MiniStat
            label={t("clubs.table.transferBudget")}
            value={formatMoney(latestFinance?.transfer_budget_remaining ?? null)}
          />
          <MiniStat
            label={t("clubs.table.wageBudget")}
            value={formatMoney(latestFinance?.wage_budget_weekly ?? null)}
          />
        </div>
      </div>

      <div
        ref={tabBarRef}
        className="flex gap-1 overflow-hidden border-b border-[var(--line)] px-2 pt-1"
      >
        {(
          [
            ["overview", t("clubs.inspector.overview")],
            ["squad", t("clubs.inspector.squad")],
            ["finances", t("clubs.inspector.finances")],
            ["stadium", t("clubs.inspector.stadium")],
          ] as [ClubTab, string][]
        ).map(([value, label]) => (
          <button
            key={value}
            type="button"
            onClick={() => setTab(value)}
            className={cn(
              "focus-ring shrink-0 whitespace-nowrap rounded-t-lg border-b-2 px-2.5 py-2 text-[10px] font-medium transition-colors",
              tab === value
                ? "border-[var(--accent)] bg-[var(--surface-2)] text-[var(--accent-strong)]"
                : "border-transparent text-[var(--muted)] hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
            )}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="thin-scroll min-h-0 flex-1 overflow-y-auto p-3">
        {tab === "overview" && (
          <OverviewTab detail={detail} />
        )}
        {tab === "squad" && <SquadTab clubUid={club.uid} />}
        {tab === "finances" && <FinancesTab detail={detail} />}
        {tab === "stadium" && <StadiumTab detail={detail} />}
      </div>

      <EditClubDialog />
    </aside>
  );
}

function OverviewTab({ detail }: { detail: import("@/lib/types").ClubDetail }) {
  const { t, locale } = useI18n();
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const club = detail.club;
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-1.5">
        <Info label={t("clubs.inspector.uid")} value={String(club.uid)} />
        <Info label="Unique ID" value={club.unique_id != null ? String(club.unique_id) : "—"} />
        <Info
          label={t("clubs.inspector.faNation")}
          value={nationLabel(club.fa_nation_name, locale) ?? `#${club.fa_nation_id}`}
        />
        <Info
          label={t("clubs.inspector.gender")}
          value={
            club.gender === 1
              ? t("clubs.inspector.women")
              : club.gender === 0
                ? t("clubs.inspector.men")
                : "—"
          }
        />
        <Info
          label={t("clubs.inspector.teams")}
          value={formatNumber(club.teams.length, locale)}
        />
        <Info
          label={t("clubs.facilities.corporate")}
          value={facilityRatingLabel(club.corporate_facilities, locale) ?? "—"}
        />
      </div>
      {club.parent_club_uid != null && (
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5">
          <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
            {t("clubs.inspector.parent")}
          </div>
          <button
            type="button"
            className="mt-0.5 inline-flex items-center gap-1 text-[11px] font-semibold text-[var(--accent-strong)] hover:underline"
            onClick={() => setSelectedClubUid(club.parent_club_uid)}
          >
            {club.parent_club_name ?? `#${club.parent_club_uid}`}
            <ChevronRight size={11} />
          </button>
        </div>
      )}
      {detail.child_clubs.length > 0 && (
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5">
          <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
            {t("clubs.inspector.children")}
          </div>
          <div className="mt-1 space-y-1">
            {detail.child_clubs.map((child) => (
              <button
                key={child.uid}
                type="button"
                className="flex w-full items-center justify-between gap-2 rounded px-1 py-0.5 text-left text-[10px] hover:bg-[var(--surface)]"
                onClick={() => setSelectedClubUid(child.uid)}
              >
                <span className="truncate font-medium text-[var(--ink)]">{child.name}</span>
                <ChevronRight size={11} className="shrink-0 text-[var(--muted)]" />
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

type SquadSortField = "ca" | "pa";

function SquadTab({ clubUid }: { clubUid: number }) {
  const { t, locale, formatMoney, formatValue, formatWage } = useI18n();
  const squad = useSquadAnalysis(clubUid);
  const dataMode = useAppStore((state) => state.dataMode);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const setInspectorOpen = useAppStore((state) => state.setInspectorOpen);
  const [sort, setSort] = useState<{ by: SquadSortField; dir: "asc" | "desc" }>({
    by: "ca",
    dir: "desc",
  });

  const players = useMemo(() => {
    if (!squad.data) return [];
    if (dataMode !== "full") {
      // Sorting by a hidden ability score would still leak the ranking.
      return [...squad.data.players].sort(
        (left, right) => (right.wage ?? 0) - (left.wage ?? 0),
      );
    }
    return [...squad.data.players].sort((left, right) => {
      const leftValue = sort.by === "ca" ? left.ca : left.pa;
      const rightValue = sort.by === "ca" ? right.ca : right.pa;
      if (leftValue == null && rightValue == null) return 0;
      if (leftValue == null) return 1;
      if (rightValue == null) return -1;
      const delta = leftValue - rightValue;
      return sort.dir === "desc" ? -delta : delta;
    });
  }, [dataMode, squad.data, sort]);

  const toggleSort = (by: SquadSortField) => {
    setSort((current) =>
      current.by === by
        ? { by, dir: current.dir === "desc" ? "asc" : "desc" }
        : { by, dir: "desc" },
    );
  };

  if (squad.isLoading) {
    return <div className="text-[11px] text-[var(--muted)]">{t("clubs.loading")}</div>;
  }
  if (!squad.data || squad.data.players.length === 0) {
    return <div className="text-[11px] text-[var(--muted)]">{t("clubs.empty")}</div>;
  }
  const data = squad.data;
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-1.5">
        <MiniStat label={t("clubs.table.players")} value={String(data.player_count)} />
        {dataMode === "full" && (
          <>
            <MiniStat label={t("clubs.table.avgCa")} value={data.average_ca.toFixed(1)} />
            <MiniStat label={t("analysis.averagePa")} value={data.average_pa.toFixed(1)} />
          </>
        )}
        <MiniStat label={t("clubs.table.value")} value={formatValue(data.total_value)} />
        <MiniStat label={t("clubs.inspector.payroll")} value={formatMoney(data.total_wage)} />
        <MiniStat label={t("analysis.averageAge")} value={data.average_age.toFixed(1)} />
      </div>
      <SquadNeedsSection
        clubUid={clubUid}
        onPlayer={(uid) => {
          setSelectedUid(uid);
          setViewMode("players");
          setInspectorOpen(true);
        }}
      />
      <div className="overflow-hidden rounded-xl border border-[var(--line)]">
        <table className="w-full text-left">
          <thead className="bg-[var(--surface-2)] text-[8px] uppercase tracking-wider text-[var(--muted)]">
            <tr>
              <th className="px-2 py-1.5">{t("table.player")}</th>
              {dataMode === "full" && (
                <>
                  <SortHeader
                    label="CA"
                    active={sort.by === "ca"}
                    dir={sort.dir}
                    onClick={() => toggleSort("ca")}
                  />
                  <SortHeader
                    label="PA"
                    active={sort.by === "pa"}
                    dir={sort.dir}
                    onClick={() => toggleSort("pa")}
                  />
                </>
              )}
              <th className="px-2 py-1.5 text-right">{t("analysis.table.wage")}</th>
            </tr>
          </thead>
          <tbody>
            {players.map((player) => (
              <tr
                key={player.uid}
                className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface-2)]"
                onClick={() => {
                  setSelectedUid(player.uid);
                  setViewMode("players");
                  setInspectorOpen(true);
                }}
              >
                <td className="px-2 py-1.5">
                  <div className="flex items-center gap-1.5">
                    <span className="flex h-5 w-5 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[8px] font-bold text-[var(--muted)]">
                      {initials(player.name)}
                    </span>
                    <div className="min-w-0">
                      <div className="max-w-[150px] truncate font-medium text-[var(--ink)]">
                        {player.name ?? t("common.unnamedPlayer")}
                      </div>
                      <div className="text-[8px] text-[var(--muted)]">
                        {player.age ?? "?"} · {player.positions_text ?? "—"}
                      </div>
                    </div>
                  </div>
                </td>
                {dataMode === "full" && (
                  <>
                    <td className="px-1 py-1.5 text-right">
                      <span className={cn("rounded border px-1 font-mono text-[8px] font-bold", caLabelClass)}>
                        {player.ca}
                      </span>
                    </td>
                    <td className="px-1 py-1.5 text-right">
                      {player.pa != null ? (
                        <span className={cn("rounded border px-1 font-mono text-[8px] font-bold", paTone(player.pa))}>
                          {player.pa}
                        </span>
                      ) : (
                        <span className="font-mono text-[8px] text-[var(--muted)]">—</span>
                      )}
                    </td>
                  </>
                )}
                <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                  {player.wage != null ? formatWage(player.wage) : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function SortHeader({
  label,
  active,
  dir,
  onClick,
}: {
  label: string;
  active: boolean;
  dir: "asc" | "desc";
  onClick: () => void;
}) {
  const Icon = active ? (dir === "desc" ? ArrowDown : ArrowUp) : ArrowUpDown;
  return (
    <th className="px-1 py-1.5 text-right">
      <button
        type="button"
        className={cn(
          "inline-flex items-center gap-0.5 hover:text-[var(--ink)]",
          active && "text-[var(--ink)]",
        )}
        onClick={onClick}
      >
        {label}
        <Icon size={9} />
      </button>
    </th>
  );
}

function FinancesTab({ detail }: { detail: import("@/lib/types").ClubDetail }) {
  const { t, locale, formatMoney } = useI18n();
  const months = detail.finance_months;
  if (months.length === 0) {
    return (
      <div className="rounded-xl border border-dashed border-[var(--line)] p-4 text-center">
        <div className="text-[11px] font-medium text-[var(--ink)]">
          {t("clubs.inspector.noFinance")}
        </div>
        <div className="mt-1 text-[10px] leading-relaxed text-[var(--muted)]">
          {t("clubs.inspector.noFinanceHint")}
        </div>
      </div>
    );
  }
  const latest = months.at(-1)!;
  const history = months.slice(-12).reverse();
  const maxBalance = Math.max(1, ...history.map((month) => Math.abs(month.balance)));
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-1.5">
        <Info label={t("clubs.inspector.financeMonth")} value={latest.month} />
        <Info
          label={t("clubs.inspector.balance")}
          value={formatMoney(latest.balance)}
          tone={latest.balance < 0 ? "danger" : "default"}
        />
        <Info
          label={t("clubs.inspector.transferBudget")}
          value={formatMoney(latest.transfer_budget_remaining)}
        />
        <Info
          label={t("clubs.inspector.transferAllocated")}
          value={formatMoney(latest.transfer_budget_allocated)}
        />
        <Info
          label={t("clubs.inspector.wageBudget")}
          value={formatMoney(latest.wage_budget_weekly)}
        />
        <Info
          label={t("clubs.inspector.payroll")}
          value={formatMoney(latest.wage_payroll_weekly)}
        />
        <Info label={t("clubs.inspector.income")} value={formatMoney(latest.total_income)} />
        <Info
          label={t("clubs.inspector.expenditure")}
          value={formatMoney(latest.total_expenditure)}
        />
      </div>

      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5">
        <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
          {t("clubs.inspector.budgetVsPayroll")}
        </div>
        <div className="mt-2 flex items-center gap-2">
          <div className="h-2 flex-1 overflow-hidden rounded-full bg-[var(--surface-3)]">
            <div
              className={cn(
                "h-full rounded-full",
                latest.wage_payroll_weekly > latest.wage_budget_weekly
                  ? "bg-[var(--danger)]"
                  : "bg-[var(--accent)]",
              )}
              style={{
                width: `${Math.min(
                  100,
                  latest.wage_budget_weekly > 0
                    ? (latest.wage_payroll_weekly / latest.wage_budget_weekly) * 100
                    : 0,
                )}%`,
              }}
            />
          </div>
          <span className="font-mono text-[9px] text-[var(--muted)]">
            {formatNumber(latest.wage_payroll_weekly, locale)} /{" "}
            {formatNumber(latest.wage_budget_weekly, locale)}
          </span>
        </div>
      </div>

      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5">
        <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
          {t("clubs.inspector.history")}
        </div>
        <div className="mt-2 space-y-1.5">
          {history.map((month) => (
            <div key={month.month} className="flex items-center gap-2">
              <span className="w-[72px] shrink-0 font-mono text-[9px] text-[var(--muted)]">
                {month.month}
              </span>
              <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--surface-3)]">
                <div
                  className={cn(
                    "h-full rounded-full",
                    month.balance < 0 ? "bg-[var(--danger)]" : "bg-[var(--accent)]",
                  )}
                  style={{ width: `${(Math.abs(month.balance) / maxBalance) * 100}%` }}
                />
              </div>
              <span className="w-14 text-right font-mono text-[9px] text-[var(--muted)]">
                {formatMoney(month.balance)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function StadiumTab({ detail }: { detail: import("@/lib/types").ClubDetail }) {
  const { t, locale } = useI18n();
  const club = detail.club;
  const facilities: { key: string; label: string; value: number | null | undefined }[] = [
    {
      key: "corporate",
      label: t("clubs.facilities.corporate"),
      value: club.corporate_facilities,
    },
    {
      key: "training",
      label: t("clubs.facilities.training"),
      value: club.training_facilities,
    },
    {
      key: "youth",
      label: t("clubs.facilities.youth"),
      value: club.youth_facilities,
    },
    {
      key: "junior",
      label: t("clubs.facilities.junior"),
      value: club.junior_coaching,
    },
    {
      key: "recruitment",
      label: t("clubs.facilities.recruitment"),
      value: club.youth_recruitment,
    },
    {
      key: "youthLevel",
      label: t("clubs.facilities.youthLevel"),
      value: club.youth_level,
    },
  ];
  return (
    <div className="space-y-3">
      <div className="overflow-hidden rounded-xl border border-[var(--line)] bg-[var(--surface-2)]">
        <div className="flex items-center gap-1.5 border-b border-[var(--line)] px-2.5 py-2 text-[9px] uppercase tracking-wider text-[var(--muted)]">
          <Building2 size={11} />
          {t("clubs.inspector.facilities")}
        </div>
        <div className="divide-y divide-[var(--line)]/60">
          {facilities.map((facility) => {
            const isYouthLevel = facility.key === "youthLevel";
            const label = isYouthLevel
              ? facility.value != null
                ? t("clubs.facilities.youthLevelValue", { level: facility.value })
                : null
              : facilityRatingLabel(facility.value, locale);
            return (
              <div
                key={facility.key}
                className="flex items-center justify-between gap-2 px-2.5 py-1.5"
              >
                <span className="text-[10px] text-[var(--muted)]">{facility.label}</span>
                {label ? (
                  <span className="text-right">
                    <span className="text-[10px] font-semibold text-[var(--ink)]">{label}</span>
                    {!isYouthLevel && (
                      <span className="ml-1 font-mono text-[8px] text-[var(--muted)]">
                        {facility.value}
                      </span>
                    )}
                  </span>
                ) : (
                  <span className="font-mono text-[10px] text-[var(--muted)]">—</span>
                )}
              </div>
            );
          })}
        </div>
      </div>
      {facilities.some((facility) => facility.value == null) && (
        <div className="text-[9px] leading-relaxed text-[var(--muted)]">
          {t("clubs.inspector.facilitiesAvailableHint")}
        </div>
      )}
      {detail.stadiums.length === 0 ? (
        <div className="rounded-xl border border-dashed border-[var(--line)] p-4 text-center text-[11px] text-[var(--muted)]">
          {t("clubs.inspector.noStadium")}
        </div>
      ) : (
        detail.stadiums.map((stadium) => (
          <div
            key={stadium.uid}
            className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5"
          >
            <div className="flex items-start gap-2">
              <MapPin size={14} className="mt-0.5 text-[var(--accent)]" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[12px] font-semibold text-[var(--ink)]">
                  {stadium.name ?? t("clubs.inspector.unnamedStadium")}
                </div>
                <div className="text-[9px] text-[var(--muted)]">
                  {t("clubs.inspector.owner")}: {stadium.owner_club_name ?? "—"}
                </div>
              </div>
            </div>
            <div className="mt-2 grid grid-cols-2 gap-1.5">
              <Info
                label={t("clubs.inspector.capacity")}
                value={formatNumber(stadium.all_seater_capacity, locale)}
              />
              <Info
                label={t("clubs.inspector.expansion")}
                value={formatNumber(stadium.expansion_capacity, locale)}
              />
              <Info
                label={t("clubs.inspector.pitch")}
                value={`${stadium.pitch_length_dm / 10} × ${stadium.pitch_width_dm / 10} m`}
              />
              <Info label={t("clubs.inspector.built")} value={formatDate(stadium.built_date)} />
              <Info label={t("clubs.inspector.rebuilt")} value={formatDate(stadium.rebuilt_date)} />
            </div>
          </div>
        ))
      )}
    </div>
  );
}

function Info({
  label,
  value,
  tone = "default",
}: {
  label: string;
  value: string;
  tone?: "default" | "danger";
}) {
  return (
    <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5">
      <div className="text-[8px] uppercase tracking-wider text-[var(--muted)]">{label}</div>
      <div
        className={cn(
          "mt-0.5 truncate font-mono text-[10px] font-semibold",
          tone === "danger" ? "text-[var(--danger)]" : "text-[var(--ink)]",
        )}
      >
        {value}
      </div>
    </div>
  );
}

function MiniStat({
  label,
  value,
  tone = "default",
}: {
  label: string;
  value: string;
  tone?: "default" | "danger";
}) {
  return (
    <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5">
      <div className="truncate text-[8px] uppercase tracking-wider text-[var(--muted)]">
        {label}
      </div>
      <div
        className={cn(
          "mt-0.5 truncate font-mono text-[11px] font-semibold",
          tone === "danger" ? "text-[var(--danger)]" : "text-[var(--ink)]",
        )}
      >
        {value}
      </div>
    </div>
  );
}
