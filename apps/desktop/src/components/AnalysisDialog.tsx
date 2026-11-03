import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowRight,
  BarChart3,
  CalendarClock,
  ChevronLeft,
  ChevronRight,
  Coins,
  Loader2,
  RefreshCw,
  Shield,
  Sparkles,
  TrendingUp,
  Users,
} from "lucide-react";
import { queryPlayers } from "@/lib/api";
import {
  useAnalysisOverview,
  useFilterOptions,
  useSquadAnalysis,
  useValueFinder,
} from "@/lib/query";
import type {
  AnalysisOverview,
  DistributionBand,
  SquadAnalysis,
  ValueFinderRequest,
  ValueFinderRow,
} from "@/lib/types";
import { bandLabel, roleName } from "@/lib/labels";
import { formatNumber, type TranslationKey } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caTone, cn, formatValue, formatWage, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Badge, Button, Input, Modal, Select } from "./ui";
import { MultiSelect } from "./MultiSelect";

type AnalysisTab = "overview" | "squad" | "wonderkids" | "value";

export function AnalysisDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.analysisOpen);
  const setOpen = useAppStore((state) => state.setAnalysisOpen);
  const saveId = useAppStore((state) => state.saveId);
  const summary = useAppStore((state) => state.summary);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const patchFilters = useAppStore((state) => state.patchFilters);
  const setInspectorOpen = useAppStore((state) => state.setInspectorOpen);
  const [tab, setTab] = useState<AnalysisTab>("overview");
  const [clubUid, setClubUid] = useState<number | null>(null);
  const clubChosen = useRef(false);
  const overview = useAnalysisOverview(open);
  const options = useFilterOptions();
  const squad = useSquadAnalysis(tab === "squad" ? clubUid : null);
  const [wonderkidMaxAge, setWonderkidMaxAge] = useState(21);
  const [wonderkidMinPa, setWonderkidMinPa] = useState(140);
  const [wonderkidMinGap, setWonderkidMinGap] = useState(10);
  const wonderkids = useQuery({
    queryKey: ["wonderkids", saveId, wonderkidMaxAge, wonderkidMinPa, wonderkidMinGap],
    queryFn: () =>
      queryPlayers({
        save_id: saveId!,
        age_max: wonderkidMaxAge,
        pa_min: wonderkidMinPa,
        pa_ca_gap_min: wonderkidMinGap,
        sort_by: "pa",
        sort_dir: "desc",
        page: 0,
        page_size: 50,
        include_hidden: false,
      }),
    enabled: open && tab === "wonderkids" && saveId != null,
    staleTime: 30_000,
  });

  useEffect(() => {
    if (clubChosen.current || clubUid != null) return;
    const first = overview.data?.top_clubs[0]?.club_uid ?? options.data?.clubs[0]?.id ?? null;
    if (first != null) {
      clubChosen.current = true;
      setClubUid(first);
    }
  }, [clubUid, options.data, overview.data]);

  const chooseClub = (uid: number | null) => {
    clubChosen.current = true;
    setClubUid(uid);
  };

  const clubName = useMemo(
    () => options.data?.clubs.find((club) => club.id === clubUid)?.name,
    [clubUid, options.data],
  );

  if (!open) return null;

  const applyWonderkidFilters = () => {
    patchFilters({
      ageMax: wonderkidMaxAge,
      paMin: wonderkidMinPa,
      paCaGapMin: wonderkidMinGap,
      ageMin: null,
      caMin: null,
      caMax: null,
      paMax: null,
    });
    setSelectedUid(null);
    setOpen(false);
  };

  return (
    <Modal
      open={open}
      onClose={() => setOpen(false)}
      title={
        <span className="inline-flex items-center gap-2">
          <BarChart3 size={16} className="text-[var(--accent)]" />
          {t("analysis.title")}
        </span>
      }
      description={`${summary?.save_name ?? t("analysis.currentSave")} · ${
        summary?.game_date ?? t("common.unknownDate")
      }`}
      width="max-w-[1080px]"
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
            {t("analysis.close")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              void overview.refetch();
              if (tab === "squad") void squad.refetch();
              if (tab === "wonderkids") void wonderkids.refetch();
            }}
          >
            <RefreshCw size={13} />
            {t("analysis.refresh")}
          </Button>
        </>
      }
    >
      <div className="flex gap-1 border-b border-[var(--line)] px-4 pt-3">
        {[
          ["overview", t("analysis.tab.overview"), <BarChart3 size={12} key="overview" />],
          ["squad", t("analysis.tab.squad"), <Users size={12} key="squad" />],
          ["wonderkids", t("analysis.tab.wonderkids"), <Sparkles size={12} key="wonderkids" />],
          ["value", t("analysis.tab.value"), <Coins size={12} key="value" />],
        ].map(([value, label, icon]) => (
          <button
            key={value as string}
            type="button"
            onClick={() => setTab(value as AnalysisTab)}
            className={cn(
              "focus-ring flex items-center gap-1.5 rounded-t-lg border-b-2 px-3 py-2 text-[11px] font-medium transition-colors",
              tab === value
                ? "border-[var(--accent)] bg-[var(--surface-2)] text-[var(--accent-strong)]"
                : "border-transparent text-[var(--muted)] hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
            )}
          >
            {icon}
            {label as string}
          </button>
        ))}
      </div>

      <div className="p-4">
        {tab === "overview" &&
          (overview.isLoading ? (
            <LoadingBlock />
          ) : overview.data ? (
            <OverviewPanel
              data={overview.data}
              onClub={(uid) => {
                chooseClub(uid);
                setTab("squad");
              }}
            />
          ) : (
            <div className="text-xs text-[var(--danger)]">{t("analysis.loadError")}</div>
          ))}

        {tab === "squad" && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="w-[280px]">
                <MultiSelect
                  options={options.data?.clubs ?? []}
                  selected={clubUid == null ? [] : [clubUid]}
                  onChange={(ids) => chooseClub(ids[0] ?? null)}
                  placeholder={t("analysis.chooseClub")}
                  maxLabel={1}
                  single
                />
              </div>
              <span className="text-[10px] text-[var(--muted)]">
                {clubName
                  ? t("analysis.sortedByCa", { club: clubName })
                  : t("analysis.chooseClubHint")}
              </span>
            </div>
            {squad.isLoading ? (
              <LoadingBlock />
            ) : squad.data ? (
              <SquadPanel
                data={squad.data}
                onPlayer={(uid) => {
                  setSelectedUid(uid);
                  setInspectorOpen(true);
                  setOpen(false);
                }}
              />
            ) : clubUid == null ? (
              <EmptyHint text={t("analysis.chooseFirst")} />
            ) : (
              <EmptyHint text={t("analysis.noSquad")} />
            )}
          </div>
        )}

        {tab === "wonderkids" && (
          <WonderkidPanel
            maxAge={wonderkidMaxAge}
            minPa={wonderkidMinPa}
            minGap={wonderkidMinGap}
            onMaxAge={setWonderkidMaxAge}
            onMinPa={setWonderkidMinPa}
            onMinGap={setWonderkidMinGap}
            loading={wonderkids.isLoading}
            rows={wonderkids.data?.rows ?? []}
            total={wonderkids.data?.total ?? 0}
            onApply={applyWonderkidFilters}
            onPlayer={(uid) => {
              setSelectedUid(uid);
              setInspectorOpen(true);
              setOpen(false);
            }}
          />
        )}

        {tab === "value" && (
          <ValueFinderPanel
            saveId={saveId}
            onPlayer={(uid) => {
              setSelectedUid(uid);
              setInspectorOpen(true);
              setOpen(false);
            }}
          />
        )}
      </div>
    </Modal>
  );
}

function OverviewPanel({
  data,
  onClub,
}: {
  data: AnalysisOverview;
  onClub: (clubUid: number) => void;
}) {
  const { t, locale } = useI18n();
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-3 gap-2 lg:grid-cols-6">
        <Stat
          icon={<Users size={13} />}
          label={t("analysis.players")}
          value={formatNumber(data.total_players, locale)}
        />
        <Stat
          icon={<Shield size={13} />}
          label={t("analysis.clubs")}
          value={formatNumber(data.total_clubs, locale)}
        />
        <Stat
          icon={<CalendarClock size={13} />}
          label={t("analysis.averageAge")}
          value={data.average_age.toFixed(1)}
        />
        <Stat
          icon={<TrendingUp size={13} />}
          label={t("analysis.medianCa")}
          value={String(data.median_ca)}
        />
        <Stat
          icon={<Coins size={13} />}
          label={t("analysis.totalValue")}
          value={formatValue(data.total_value)}
        />
        <Stat
          icon={<CalendarClock size={13} />}
          label={t("analysis.expiringWithinYear")}
          value={formatNumber(data.expiring_contracts, locale)}
        />
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <SectionTitle>{t("analysis.ageStructure")}</SectionTitle>
          <BandBars bands={data.age_bands} tone="accent" />
        </div>
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <SectionTitle>{t("analysis.caDistribution")}</SectionTitle>
          <BandBars bands={data.ca_bands} tone="gold" />
        </div>
      </div>

      <div className="grid gap-3 lg:grid-cols-[1.6fr_1fr]">
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <div className="mb-2 flex items-center justify-between">
            <SectionTitle>{t("analysis.clubStrength")}</SectionTitle>
            <Badge tone="outline">{t("analysis.averageCa")}</Badge>
          </div>
          <div className="thin-scroll max-h-[300px] overflow-auto">
            <table className="w-full text-left">
              <thead className="sticky top-0 bg-[var(--surface-2)]">
                <tr className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
                  <th className="py-1 pr-2">#</th>
                  <th className="py-1">{t("analysis.table.club")}</th>
                  <th className="py-1 text-right">{t("analysis.table.players")}</th>
                  <th className="py-1 text-right">{t("analysis.averageCa")}</th>
                  <th className="py-1 text-right">{t("analysis.table.maxCa")}</th>
                  <th className="py-1 text-right">{t("analysis.averageAge")}</th>
                  <th className="py-1 text-right">{t("analysis.table.value")}</th>
                </tr>
              </thead>
              <tbody>
                {data.top_clubs.map((club, index) => (
                  <tr
                    key={club.club_uid}
                    className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface)]"
                    onClick={() => onClub(club.club_uid)}
                  >
                    <td className="py-1.5 pr-2 font-mono text-[var(--muted)]">{index + 1}</td>
                    <td className="max-w-[190px] truncate py-1.5 font-medium text-[var(--ink)]">
                      {club.club_name}
                    </td>
                    <td className="py-1.5 text-right font-mono text-[var(--muted)]">{club.players}</td>
                    <td className="py-1.5 text-right font-mono font-semibold text-[var(--accent-strong)]">
                      {club.avg_ca}
                    </td>
                    <td className="py-1.5 text-right font-mono text-[var(--muted)]">{club.max_ca}</td>
                    <td className="py-1.5 text-right font-mono text-[var(--muted)]">{club.avg_age}</td>
                    <td className="py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatValue(club.total_value)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <div className="space-y-3">
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <SectionTitle>{t("analysis.contractYears")}</SectionTitle>
            <BandBars bands={data.contract_years} tone="blue" compact />
          </div>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <SectionTitle>{t("analysis.status")}</SectionTitle>
            <div className="mt-2 grid grid-cols-3 gap-2 text-center">
              <MiniStat
                label={t("analysis.freeAgents")}
                value={formatNumber(data.free_agents, locale)}
              />
              <MiniStat
                label={t("analysis.loanees")}
                value={formatNumber(data.loanees, locale)}
              />
              <MiniStat
                label={t("analysis.expiringWithinYear")}
                value={formatNumber(data.expiring_contracts, locale)}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function SquadPanel({
  data,
  onPlayer,
}: {
  data: SquadAnalysis;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale } = useI18n();
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-2 lg:grid-cols-6">
        <Stat
          icon={<Users size={13} />}
          label={t("analysis.squadSize")}
          value={String(data.player_count)}
        />
        <Stat
          icon={<CalendarClock size={13} />}
          label={t("analysis.averageAge")}
          value={data.average_age.toFixed(1)}
        />
        <Stat
          icon={<TrendingUp size={13} />}
          label={t("analysis.averageCa")}
          value={data.average_ca.toFixed(1)}
        />
        <Stat
          icon={<TrendingUp size={13} />}
          label={t("analysis.averagePa")}
          value={data.average_pa.toFixed(1)}
        />
        <Stat
          icon={<Coins size={13} />}
          label={t("analysis.totalWage")}
          value={formatWage(data.total_wage, locale)}
        />
        <Stat
          icon={<Coins size={13} />}
          label={t("analysis.totalValue")}
          value={formatValue(data.total_value)}
        />
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <SectionTitle>{t("analysis.positionDepth")}</SectionTitle>
          <BandBars bands={data.position_groups} tone="accent" compact />
        </div>
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <SectionTitle>{t("analysis.ageStructure")}</SectionTitle>
          <BandBars bands={data.age_bands} tone="gold" compact />
        </div>
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <SectionTitle>{t("analysis.contractExpiry")}</SectionTitle>
          <BandBars bands={data.contract_years} tone="blue" compact />
        </div>
      </div>

      <div className="overflow-hidden rounded-xl border border-[var(--line)]">
        <div className="flex items-center justify-between border-b border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
          <SectionTitle>{t("analysis.clubSquad", { club: data.club_name })}</SectionTitle>
          <span className="text-[9px] text-[var(--muted)]">
            {t("analysis.loaneesCount", { count: data.loanees })}
          </span>
        </div>
        <div className="thin-scroll max-h-[340px] overflow-auto">
          <table className="w-full text-left">
            <thead className="sticky top-0 z-10 bg-[var(--surface-2)] text-[9px] uppercase tracking-wider text-[var(--muted)]">
              <tr>
                <th className="px-3 py-1.5">{t("analysis.table.player")}</th>
                <th className="px-2 py-1.5 text-right">{t("analysis.table.age")}</th>
                <th className="px-2 py-1.5">{t("analysis.table.positions")}</th>
                <th className="px-2 py-1.5 text-right">CA</th>
                <th className="px-2 py-1.5 text-right">PA</th>
                <th className="px-2 py-1.5 text-right">{t("analysis.table.wage")}</th>
                <th className="px-3 py-1.5 text-right">{t("analysis.table.contract")}</th>
              </tr>
            </thead>
            <tbody>
              {data.players.map((player) => (
                <tr
                  key={player.uid}
                  className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface-2)]"
                  onClick={() => onPlayer(player.uid)}
                >
                  <td className="px-3 py-1.5">
                    <div className="flex items-center gap-1.5">
                      <span className="flex h-5 w-5 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[8px] font-bold text-[var(--muted)]">
                        {initials(player.name)}
                      </span>
                      <span className="max-w-[160px] truncate font-medium text-[var(--ink)]">
                        {player.name ?? t("common.unnamed")}
                      </span>
                    </div>
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">{player.age ?? "—"}</td>
                  <td className="px-2 py-1.5 font-mono text-[var(--accent-strong)]">
                    {player.positions_text ?? "—"}
                  </td>
                  <td className="px-2 py-1.5 text-right">
                    <span className={cn("inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold", caTone(player.ca))}>
                      {player.ca}
                    </span>
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">{player.pa ?? "?"}</td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                    {player.wage != null ? formatWage(player.wage, locale) : "—"}
                  </td>
                  <td className="px-3 py-1.5 text-right font-mono text-[var(--muted)]">
                    {player.contract_end ?? "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

function WonderkidPanel({
  maxAge,
  minPa,
  minGap,
  onMaxAge,
  onMinPa,
  onMinGap,
  loading,
  rows,
  total,
  onApply,
  onPlayer,
}: {
  maxAge: number;
  minPa: number;
  minGap: number;
  onMaxAge: (value: number) => void;
  onMinPa: (value: number) => void;
  onMinGap: (value: number) => void;
  loading: boolean;
  rows: import("@/lib/types").PlayerRow[];
  total: number;
  onApply: () => void;
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-end gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
        <NumberField
          label={t("analysis.wonderkidAge")}
          value={maxAge}
          min={15}
          max={30}
          onChange={onMaxAge}
        />
        <NumberField
          label={t("analysis.wonderkidPa")}
          value={minPa}
          min={80}
          max={200}
          onChange={onMinPa}
        />
        <NumberField
          label={t("analysis.wonderkidGap")}
          value={minGap}
          min={0}
          max={80}
          onChange={onMinGap}
        />
        <div className="ml-auto flex items-center gap-2">
          <span className="text-[10px] text-[var(--muted)]">
            {t("analysis.matches", { count: total })}
          </span>
          <Button variant="primary" size="sm" onClick={onApply}>
            {t("analysis.applyFilters")}
            <ArrowRight size={12} />
          </Button>
        </div>
      </div>
      {loading ? (
        <LoadingBlock />
      ) : rows.length === 0 ? (
        <EmptyHint text={t("analysis.noWonderkids")} />
      ) : (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <table className="w-full text-left">
            <thead className="bg-[var(--surface-2)] text-[9px] uppercase tracking-wider text-[var(--muted)]">
              <tr>
                <th className="px-3 py-1.5">{t("analysis.table.player")}</th>
                <th className="px-2 py-1.5">{t("analysis.table.club")}</th>
                <th className="px-2 py-1.5 text-right">{t("analysis.table.age")}</th>
                <th className="px-2 py-1.5">{t("analysis.table.positions")}</th>
                <th className="px-2 py-1.5 text-right">CA</th>
                <th className="px-2 py-1.5 text-right">PA</th>
                <th className="px-2 py-1.5 text-right">{t("filter.paCaGap")}</th>
                <th className="px-3 py-1.5 text-right">{t("analysis.table.value")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((player) => (
                <tr
                  key={player.uid}
                  className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface-2)]"
                  onClick={() => onPlayer(player.uid)}
                >
                  <td className="px-3 py-1.5 font-medium text-[var(--ink)]">{player.name}</td>
                  <td className="max-w-[160px] truncate px-2 py-1.5 text-[var(--muted)]">
                    {player.club_name ?? t("common.freeAgent")}
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">{player.age ?? "—"}</td>
                  <td className="px-2 py-1.5 font-mono text-[var(--accent-strong)]">{player.positions_text}</td>
                  <td className="px-2 py-1.5 text-right font-mono">{player.ca}</td>
                  <td className="px-2 py-1.5 text-right font-mono font-semibold text-[var(--gold)]">{player.pa ?? "?"}</td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                    {player.pa != null ? player.pa - player.ca : "—"}
                  </td>
                  <td className="px-3 py-1.5 text-right font-mono text-[var(--muted)]">{formatValue(player.transfer_value)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

const VALUE_PAGE_SIZE = 40;
const VALUE_POSITIONS = ["GK", "DC", "FB", "MID", "AM", "ST"] as const;

function optionalNumber(value: string): number | null {
  if (!value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.max(0, Math.round(parsed)) : null;
}

function ValueFinderPanel({
  saveId,
  onPlayer,
}: {
  saveId: number | null;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale } = useI18n();
  const [position, setPosition] = useState("");
  const [league, setLeague] = useState("");
  const [minCa, setMinCa] = useState(120);
  const [minRole, setMinRole] = useState(12);
  const [maxAge, setMaxAge] = useState(30);
  const [maxFee, setMaxFee] = useState<number | null>(null);
  const [maxWage, setMaxWage] = useState<number | null>(null);
  const [market, setMarket] = useState("all");
  const [includeUnknownFee, setIncludeUnknownFee] = useState(false);
  const [page, setPage] = useState(0);
  const [sortBy, setSortBy] = useState("score");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");

  useEffect(() => {
    setPage(0);
  }, [
    saveId,
    position,
    league,
    minCa,
    minRole,
    maxAge,
    maxFee,
    maxWage,
    market,
    includeUnknownFee,
    sortBy,
    sortDir,
  ]);

  const request = useMemo<ValueFinderRequest | null>(
    () =>
      saveId == null
        ? null
        : {
            save_id: saveId,
            position_group: position || null,
            league: league || null,
            age_max: maxAge,
            ca_min: minCa,
            role_min: minRole,
            max_fee: maxFee,
            max_wage: maxWage,
            listings: market === "all" ? [] : [market],
            include_unknown_fee: includeUnknownFee,
            sort_by: sortBy,
            sort_dir: sortDir,
            page,
            page_size: VALUE_PAGE_SIZE,
          },
    [
      saveId,
      position,
      league,
      minCa,
      minRole,
      maxAge,
      maxFee,
      maxWage,
      market,
      includeUnknownFee,
      sortBy,
      sortDir,
      page,
    ],
  );
  const query = useValueFinder(request);
  const data = query.data;
  const rows = data?.rows ?? [];
  const pages = Math.max(1, Math.ceil((data?.total ?? 0) / VALUE_PAGE_SIZE));
  const leagues = data?.meta.leagues ?? [];

  const changeSort = (by: string) => {
    if (sortBy === by) {
      setSortDir((current) => (current === "desc" ? "asc" : "desc"));
    } else {
      setSortBy(by);
      setSortDir("desc");
    }
  };

  const reset = () => {
    setPosition("");
    setLeague("");
    setMinCa(120);
    setMinRole(12);
    setMaxAge(30);
    setMaxFee(null);
    setMaxWage(null);
    setMarket("all");
    setIncludeUnknownFee(false);
    setSortBy("score");
    setSortDir("desc");
    setPage(0);
  };

  const feeLabel = (row: ValueFinderRow) => {
    if (row.fee == null) return t("analysis.value.unknownFee");
    if (row.fee === 0) return t("analysis.value.free");
    return formatValue(row.fee);
  };

  const confidenceTone = (
    row: ValueFinderRow,
  ): "accent" | "default" | "danger" =>
    row.confidence >= 0.9 ? "accent" : row.confidence >= 0.7 ? "default" : "danger";

  return (
    <div className="space-y-3">
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
        <div className="flex flex-wrap items-end gap-2.5">
          <ValueFilter label={t("analysis.value.position")} className="w-[118px]">
            <Select
              value={position}
              onChange={(event) => setPosition(event.target.value)}
              className="h-7 text-[10px]"
            >
              <option value="">{t("analysis.value.allPositions")}</option>
              {VALUE_POSITIONS.map((group) => (
                <option key={group} value={group}>
                  {group}
                </option>
              ))}
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.value.league")} className="w-[190px]">
            <Select
              value={league}
              onChange={(event) => setLeague(event.target.value)}
              className="h-7 text-[10px]"
            >
              <option value="">{t("analysis.value.allLeagues")}</option>
              {leagues.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
          </ValueFilter>
          <NumberField
            label={t("analysis.value.minCa")}
            value={minCa}
            min={0}
            max={200}
            onChange={setMinCa}
          />
          <NumberField
            label={t("analysis.value.minRole")}
            value={minRole}
            min={1}
            max={20}
            onChange={setMinRole}
          />
          <NumberField
            label={t("analysis.value.maxAge")}
            value={maxAge}
            min={15}
            max={50}
            onChange={setMaxAge}
          />
          <label className="space-y-1">
            <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.value.maxFee")}
            </span>
            <Input
              type="number"
              min={0}
              value={maxFee ?? ""}
              placeholder="—"
              onChange={(event) => setMaxFee(optionalNumber(event.target.value))}
              className="h-7 w-[96px] font-mono text-[10px]"
            />
          </label>
          <label className="space-y-1">
            <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.value.maxWage")}
            </span>
            <Input
              type="number"
              min={0}
              value={maxWage ?? ""}
              placeholder="—"
              onChange={(event) => setMaxWage(optionalNumber(event.target.value))}
              className="h-7 w-[96px] font-mono text-[10px]"
            />
          </label>
          <ValueFilter label={t("analysis.value.market")} className="w-[150px]">
            <Select
              value={market}
              onChange={(event) => setMarket(event.target.value)}
              className="h-7 text-[10px]"
            >
              <option value="all">{t("analysis.value.marketAll")}</option>
              <option value="sale">{t("analysis.value.marketSale")}</option>
              <option value="loan">{t("analysis.value.marketLoan")}</option>
              <option value="free">{t("analysis.value.marketFree")}</option>
            </Select>
          </ValueFilter>
          <label className="flex h-7 items-center gap-1.5 text-[10px] text-[var(--muted)]">
            <input
              type="checkbox"
              checked={includeUnknownFee}
              onChange={(event) => setIncludeUnknownFee(event.target.checked)}
              className="accent-[var(--accent)]"
            />
            {t("analysis.value.includeUnknownFee")}
          </label>
          <Button variant="ghost" size="sm" className="h-7" onClick={reset}>
            {t("analysis.value.reset")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7"
            onClick={() => void query.refetch()}
            title={t("analysis.refresh")}
          >
            <RefreshCw size={11} />
          </Button>
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[9px] text-[var(--muted)]">
          <span>
            {data
              ? t("analysis.value.summary", {
                  count: data.total,
                  shown: rows.length,
                  years: data.meta.cost_horizon_years,
                })
              : t("analysis.value.loading")}
          </span>
          <span>{t("analysis.value.scoreHint")}</span>
        </div>
      </div>

      {query.isLoading && !data ? (
        <div className="flex items-center justify-center gap-2 rounded-xl border border-dashed border-[var(--line)] py-12 text-[11px] text-[var(--muted)]">
          <Loader2 size={14} className="animate-spin" />
          {t("analysis.value.loading")}
        </div>
      ) : query.isError ? (
        <div className="rounded-xl border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-4 text-[11px] text-[var(--danger)]">
          {t("analysis.loadError")}
        </div>
      ) : rows.length === 0 ? (
        <EmptyHint text={t("analysis.value.empty")} />
      ) : (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <div className="thin-scroll max-h-[430px] overflow-auto">
            <table className="w-full min-w-[1040px] text-left">
              <thead className="sticky top-0 z-10 bg-[var(--surface-2)] text-[9px] uppercase tracking-wider text-[var(--muted)]">
                <tr>
                  <th className="px-3 py-1.5">{t("analysis.value.table.player")}</th>
                  <th className="px-2 py-1.5">{t("analysis.value.table.position")}</th>
                  <ValueSortHeader
                    label={t("analysis.value.table.age")}
                    by="age"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.ca")}
                    by="ca"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.role")}
                    by="role"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.fee")}
                    by="fee"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.wage")}
                    by="wage"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.cost")}
                    by="cost"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <ValueSortHeader
                    label={t("analysis.value.table.score")}
                    by="score"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <th className="px-2 py-1.5 text-right">{t("analysis.value.table.confidence")}</th>
                  <th className="px-3 py-1.5">{t("analysis.value.table.flags")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr
                    key={row.uid}
                    className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface-2)]"
                    onClick={() => onPlayer(row.uid)}
                  >
                    <td className="px-3 py-1.5">
                      <div className="flex items-center gap-1.5">
                        <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[8px] font-bold text-[var(--muted)]">
                          {initials(row.name)}
                        </span>
                        <div className="min-w-0">
                          <div className="max-w-[170px] truncate font-medium text-[var(--ink)]">
                            {row.name ?? t("common.unnamed")}
                          </div>
                          <div className="max-w-[190px] truncate text-[9px] text-[var(--muted)]">
                            {row.club_name ?? t("common.freeAgent")}
                            {row.league_name ? ` · ${row.league_name}` : ""}
                          </div>
                        </div>
                      </div>
                    </td>
                    <td className="px-2 py-1.5">
                      <div className="font-mono text-[10px] font-semibold text-[var(--accent-strong)]">
                        {row.role_position ?? row.position_group}
                      </div>
                      <div className="max-w-[120px] truncate text-[9px] text-[var(--muted)]">
                        {roleName(row.role_id, row.role_name, locale) ?? "—"}
                      </div>
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {row.age ?? "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <span
                        className={cn(
                          "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                          caTone(row.ca),
                        )}
                      >
                        {row.ca}
                      </span>
                      <span className="ml-1 font-mono text-[9px] text-[var(--muted)]">
                        / {row.pa ?? "?"}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {row.role_score != null ? `${row.role_score}/20` : "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {feeLabel(row)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatWage(row.wage, locale)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatValue(row.cost)}
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <span
                        className={cn(
                          "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                          row.score >= 75
                            ? "border-[var(--gold-line)] bg-[var(--gold-soft)] text-[var(--gold)]"
                            : row.score >= 60
                              ? "border-[var(--success-line)] bg-[var(--success-soft)] text-[var(--success)]"
                              : row.score >= 45
                                ? "border-[var(--blue-line)] bg-[var(--blue-soft)] text-[var(--blue)]"
                                : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)]",
                        )}
                      >
                        {Math.round(row.score)}
                      </span>
                      {row.surplus != null && (
                        <div className="mt-0.5 font-mono text-[8px] text-[var(--muted)]">
                          {row.surplus >= 0 ? "+" : ""}
                          {Math.round(row.surplus * 100)}
                        </div>
                      )}
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <Badge tone={confidenceTone(row)} className="px-1.5 text-[8px]">
                        {Math.round(row.confidence * 100)}%
                      </Badge>
                    </td>
                    <td className="px-3 py-1.5">
                      <div className="flex max-w-[180px] flex-wrap gap-0.5">
                        {row.flags.slice(0, 2).map((flag) => (
                          <span
                            key={flag}
                            className="rounded border border-[var(--line)] bg-[var(--surface-2)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
                          >
                            {t(`analysis.value.flag.${flag}` as TranslationKey)}
                          </span>
                        ))}
                        {row.flags.length > 2 && (
                          <span
                            className="rounded border border-[var(--line)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
                            title={row.flags
                              .map((flag) => t(`analysis.value.flag.${flag}` as TranslationKey))
                              .join(", ")}
                          >
                            +{row.flags.length - 2}
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="flex items-center justify-between border-t border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
            <span className="text-[9px] text-[var(--muted)]">
              {t("analysis.value.page", { page: page + 1, pages })}
            </span>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-[10px]"
                disabled={page <= 0}
                onClick={() => setPage((current) => Math.max(0, current - 1))}
              >
                <ChevronLeft size={11} />
                {t("analysis.value.prev")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-[10px]"
                disabled={page + 1 >= pages}
                onClick={() => setPage((current) => Math.min(pages - 1, current + 1))}
              >
                {t("analysis.value.next")}
                <ChevronRight size={11} />
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function ValueFilter({
  label,
  className,
  children,
}: {
  label: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <label className={cn("space-y-1", className)}>
      <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
        {label}
      </span>
      {children}
    </label>
  );
}

function ValueSortHeader({
  label,
  by,
  active,
  dir,
  onClick,
}: {
  label: string;
  by: string;
  active: string;
  dir: "asc" | "desc";
  onClick: (by: string) => void;
}) {
  return (
    <th className="px-2 py-1.5 text-right">
      <button
        type="button"
        onClick={() => onClick(by)}
        className={cn(
          "inline-flex items-center gap-0.5 font-medium uppercase tracking-wider transition-colors hover:text-[var(--ink)]",
          active === by ? "text-[var(--accent-strong)]" : "text-[var(--muted)]",
        )}
      >
        {label}
        {active === by && <span className="text-[8px]">{dir === "desc" ? "↓" : "↑"}</span>}
      </button>
    </th>
  );
}

function Stat({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
      <div className="flex items-center gap-1 text-[9px] uppercase tracking-wider text-[var(--muted)]">
        {icon}
        {label}
      </div>
      <div className="mt-0.5 font-mono text-[14px] font-semibold text-[var(--ink)]">{value}</div>
    </div>
  );
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-[var(--line)] bg-[var(--surface)] px-2 py-1.5">
      <div className="text-[9px] text-[var(--muted)]">{label}</div>
      <div className="font-mono text-[12px] font-semibold text-[var(--ink)]">{value}</div>
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
      {children}
    </div>
  );
}

function BandBars({
  bands,
  tone,
  compact = false,
}: {
  bands: DistributionBand[];
  tone: "accent" | "gold" | "blue";
  compact?: boolean;
}) {
  const { t, locale } = useI18n();
  const max = Math.max(1, ...bands.map((band) => band.count));
  const color = tone === "gold" ? "bg-[var(--gold)]" : tone === "blue" ? "bg-[var(--blue)]" : "bg-[var(--accent)]";
  return (
    <div
      className={cn(
        "mt-2 grid grid-cols-[auto_1fr_auto] items-center gap-x-2",
        compact ? "gap-y-1" : "gap-y-1.5",
      )}
    >
      {bands.length === 0 && (
        <div className="col-span-3 text-[10px] text-[var(--muted)]">{t("analysis.noData")}</div>
      )}
      {bands.map((band) => (
        <Fragment key={band.label}>
          <span className="whitespace-nowrap text-right font-mono text-[9px] text-[var(--muted)]">
            {bandLabel(band.label, locale)}
          </span>
          <div className={cn("overflow-hidden rounded-full bg-[var(--surface-3)]", compact ? "h-1" : "h-1.5")}>
            <div className={cn("h-full rounded-full", color)} style={{ width: `${(band.count / max) * 100}%` }} />
          </div>
          <span className="text-right font-mono text-[9px] text-[var(--muted)]">{band.count}</span>
        </Fragment>
      ))}
    </div>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="space-y-1">
      <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="h-7 w-20 rounded-[6px] border border-[var(--line)] bg-[var(--surface)] px-2 font-mono text-[11px]"
      />
    </label>
  );
}

function LoadingBlock() {
  const { t } = useI18n();
  return (
    <div className="flex items-center justify-center gap-2 rounded-xl border border-dashed border-[var(--line)] py-12 text-[11px] text-[var(--muted)]">
      <Loader2 size={14} className="animate-spin" />
      {t("analysis.loading")}
    </div>
  );
}

function EmptyHint({ text }: { text: string }) {
  return (
    <div className="rounded-xl border border-dashed border-[var(--line)] py-10 text-center text-[11px] text-[var(--muted)]">
      {text}
    </div>
  );
}
