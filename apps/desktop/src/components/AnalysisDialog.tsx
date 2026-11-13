import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import {
  BarChart3,
  CalendarClock,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Coins,
  Compass,
  Download,
  Loader2,
  RefreshCw,
  TrendingUp,
  Users,
} from "lucide-react";
import { exportStats, pickExportFile } from "@/lib/api";
import {
  useManagedClub,
  useSimilarStatsPlayers,
  useSquadAnalysis,
  useStatsMoneyball,
  useValueFinder,
} from "@/lib/query";
import type {
  DistributionBand,
  SquadAnalysis,
  StatsMoneyballRequest,
  StatsMoneyballRow,
  ValueFinderRequest,
  ValueFinderRow,
} from "@/lib/types";
import { bandLabel, roleName } from "@/lib/labels";
import { formatNumber, type TranslationKey } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { caLabelClass, cn, initials, paTone } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { ScoutPanel } from "./ScoutPanel";
import { Badge, Button, Input, Modal, Select } from "./ui";

type AnalysisTab = "overview" | "scout" | "value" | "stats";

export function AnalysisDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.analysisOpen);
  const setOpen = useAppStore((state) => state.setAnalysisOpen);
  const saveId = useAppStore((state) => state.saveId);
  const dataMode = useAppStore((state) => state.dataMode);
  const summary = useAppStore((state) => state.summary);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const setInspectorOpen = useAppStore((state) => state.setInspectorOpen);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const [tab, setTab] = useState<AnalysisTab>(
    dataMode === "full" ? "overview" : "stats",
  );
  const managed = useManagedClub(saveId);
  const managedClubUid = managed.data?.club_uid ?? null;
  const squad = useSquadAnalysis(tab === "overview" ? managedClubUid : null);

  const openPlayer = (uid: number) => {
    setSelectedUid(uid);
    setViewMode("players");
    setInspectorOpen(true);
    setOpen(false);
  };

  useEffect(() => {
    const allowed: AnalysisTab[] = [
      "overview",
      ...(dataMode === "full" ? (["scout"] as AnalysisTab[]) : []),
      ...(dataMode !== "moneyball" ? (["value"] as AnalysisTab[]) : []),
      "stats",
    ];
    if (!allowed.includes(tab)) setTab("stats");
  }, [dataMode, tab]);

  if (!open) return null;

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
              if (tab === "overview") void squad.refetch();
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
          ["overview", t("analysis.tab.overview"), <Users size={12} key="overview" />],
          ...(dataMode === "full"
            ? [["scout", t("analysis.tab.scout"), <Compass size={12} key="scout" />]]
            : []),
          ...(dataMode !== "moneyball"
            ? [["value", t("analysis.tab.value"), <Coins size={12} key="value" />]]
            : []),
          ["stats", t("analysis.tab.stats"), <TrendingUp size={12} key="stats" />],
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
          (managed.isLoading ? (
            <LoadingBlock />
          ) : managedClubUid == null ? (
            <EmptyHint text={t("analysis.noManagedClub")} />
          ) : (
            <div className="space-y-4">
              <span className="text-[10px] text-[var(--muted)]">
                {dataMode === "full"
                  ? t("analysis.sortedByCa", { club: managed.data?.club_name ?? "" })
                  : t("analysis.squadListHint", { club: managed.data?.club_name ?? "" })}
              </span>
              {squad.isLoading ? (
                <LoadingBlock />
              ) : squad.data ? (
                <SquadPanel data={squad.data} onPlayer={openPlayer} />
              ) : (
                <EmptyHint text={t("analysis.noSquad")} />
              )}
            </div>
          ))}

        {tab === "scout" &&
          (managed.isLoading ? (
            <LoadingBlock />
          ) : managedClubUid != null ? (
            <ScoutPanel clubUid={managedClubUid} onPlayer={openPlayer} />
          ) : (
            <EmptyHint text={t("analysis.noManagedClub")} />
          ))}

        {tab === "value" && (
          <ValueFinderPanel saveId={saveId} onPlayer={openPlayer} />
        )}

        {tab === "stats" && (
          <StatsMoneyballPanel saveId={saveId} onPlayer={openPlayer} />
        )}
      </div>
    </Modal>
  );
}

type SquadPlayer = SquadAnalysis["players"][number];

type SquadGroup = {
  slot: number | null;
  /** Display order: 1 for the first team, then 2, 3 … for the other squads. */
  number: number | null;
  players: SquadPlayer[];
  minAge: number | null;
  maxAge: number | null;
};

type SquadTab = {
  key: string;
  group: SquadGroup;
};

function SquadPanel({
  data,
  onPlayer,
}: {
  data: SquadAnalysis;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale, formatValue, formatWage } = useI18n();
  const dataMode = useAppStore((state) => state.dataMode);
  const [sortBy, setSortBy] = useState<"ca" | "pa">("ca");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  // Open on the first team; All remains the first tab for the full list.
  const [squadTab, setSquadTab] = useState("slot-0");
  const tableScroll = useRef<HTMLDivElement>(null);
  const changeSort = (by: string) => {
    if (by !== "ca" && by !== "pa") return;
    if (sortBy === by) {
      setSortDir((current) => (current === "desc" ? "asc" : "desc"));
    } else {
      setSortBy(by);
      setSortDir("desc");
    }
  };
  // The backend returns the squad by CA; do not leak that order when ability
  // is hidden. Unknown PA values always sort last.
  const players = useMemo(() => {
    if (dataMode !== "full") {
      return [...data.players].sort((left, right) => (right.wage ?? 0) - (left.wage ?? 0));
    }
    const direction = sortDir === "desc" ? -1 : 1;
    return [...data.players].sort((left, right) => {
      if (sortBy === "ca") return (left.ca - right.ca) * direction;
      if (left.pa == null && right.pa == null) return 0;
      if (left.pa == null) return 1;
      if (right.pa == null) return -1;
      return (left.pa - right.pa) * direction;
    });
  }, [data.players, dataMode, sortBy, sortDir]);
  // The save stores teams in the club's own list order; slot 0 is the first
  // team, and later slots are its reserve, youth and affiliate sides. Players
  // without a resolved team stay in one trailing group.
  const groups = useMemo<SquadGroup[]>(() => {
    const bySlot = new Map<number | null, SquadGroup>();
    for (const player of players) {
      const slot = player.team_slot ?? null;
      let group = bySlot.get(slot);
      if (!group) {
        group = { slot, number: null, players: [], minAge: null, maxAge: null };
        bySlot.set(slot, group);
      }
      group.players.push(player);
      if (player.age != null) {
        group.minAge =
          group.minAge == null ? player.age : Math.min(group.minAge, player.age);
        group.maxAge =
          group.maxAge == null ? player.age : Math.max(group.maxAge, player.age);
      }
    }
    const ordered = [...bySlot.values()].sort((left, right) => {
      if (left.slot == null) return 1;
      if (right.slot == null) return -1;
      return left.slot - right.slot;
    });
    let squadNumber = 1;
    for (const group of ordered) {
      if (group.slot === 0) {
        group.number = 1;
      } else if (group.slot != null) {
        squadNumber += 1;
        group.number = squadNumber;
      }
    }
    return ordered;
  }, [players]);
  const tabs = useMemo<SquadTab[]>(() => {
    const ages = players
      .map((player) => player.age)
      .filter((age): age is number => age != null);
    const allGroup: SquadGroup = {
      slot: null,
      number: null,
      players,
      minAge: ages.length > 0 ? Math.min(...ages) : null,
      maxAge: ages.length > 0 ? Math.max(...ages) : null,
    };
    return [
      { key: "all", group: allGroup },
      ...groups.map((group) => ({
        key: group.slot == null ? "unassigned" : `slot-${group.slot}`,
        group,
      })),
    ];
  }, [groups, players]);
  const activeTab = tabs.find((tab) => tab.key === squadTab) ?? tabs[0];
  useEffect(() => {
    if (!tabs.some((tab) => tab.key === squadTab)) setSquadTab("all");
  }, [squadTab, tabs]);
  useEffect(() => {
    if (tableScroll.current) tableScroll.current.scrollTop = 0;
  }, [squadTab]);
  const teamBySlot = useMemo(
    () =>
      new Map(
        (data.teams ?? [])
          .filter((team) => team.slot != null)
          .map((team) => [team.slot as number, team] as const),
      ),
    [data.teams],
  );
  const groupLabel = (group: SquadGroup) => {
    if (group.slot == null) return t("analysis.squadGroup.unassigned");
    if (group.slot === 0) return t("analysis.squadGroup.firstTeam");
    const team = teamBySlot.get(group.slot);
    if (team?.label.kind === "youth") {
      return team.label.age == null
        ? t("analysis.squadGroup.youth")
        : `U${team.label.age}`;
    }
    if (team?.label.kind === "reserves") return t("analysis.squadGroup.reserves");
    return t("analysis.squadGroup.numbered", { n: group.number ?? 2 });
  };
  const groupSummary = (group: SquadGroup) => {
    if (group.minAge == null || group.maxAge == null) {
      return t("analysis.squadGroup.playerCount", { count: group.players.length });
    }
    if (group.minAge === group.maxAge) {
      return t("analysis.squadGroup.singleAge", {
        count: group.players.length,
        age: group.minAge,
      });
    }
    return t("analysis.squadGroup.ageRange", {
      count: group.players.length,
      min: group.minAge,
      max: group.maxAge,
    });
  };
  const groupTitle = (group: SquadGroup) => {
    const parts = [groupSummary(group)];
    const team = group.slot == null ? undefined : teamBySlot.get(group.slot);
    if (team) {
      if (team.competition_name != null) {
        parts.push(
          t("analysis.squadGroup.competitionFixtures", {
            name: team.competition_name,
            count: team.fixture_count,
          }),
        );
      } else if (team.competition_id != null && team.fixture_count > 0) {
        parts.push(t("analysis.squadGroup.unnamedCompetition"));
      } else if (group.slot !== 0) {
        parts.push(t("analysis.squadGroup.noCompetitiveFixtures"));
      }
      if (team.inferred) parts.push(t("analysis.squadGroup.inferredFromAge"));
    }
    return parts.join(" · ");
  };
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
        {dataMode === "full" && (
          <>
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
          </>
        )}
        <Stat
          icon={<Coins size={13} />}
          label={t("analysis.totalWage")}
          value={formatWage(data.total_wage)}
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
        <div className="border-b border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
          <div className="flex items-center justify-between">
            <SectionTitle>{t("analysis.clubSquad", { club: data.club_name })}</SectionTitle>
            <span className="text-[9px] text-[var(--muted)]">
              {t("analysis.loaneesCount", { count: data.loanees })}
            </span>
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-1">
            {tabs.map((tab) => {
              const active = tab.key === activeTab.key;
              return (
                <button
                  key={tab.key}
                  type="button"
                  data-testid="squad-tab"
                  aria-pressed={active}
                  title={groupTitle(tab.group)}
                  onClick={() => setSquadTab(tab.key)}
                  className={cn(
                    "focus-ring inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-medium transition-colors",
                    active
                      ? "border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                      : "border-[var(--line)] bg-[var(--surface)] text-[var(--muted)] hover:text-[var(--ink)]",
                  )}
                >
                  {tab.key === "all"
                    ? t("analysis.squadGroup.all")
                    : groupLabel(tab.group)}
                  <span
                    className={cn(
                      "rounded-full px-1 font-mono text-[8px]",
                      active ? "bg-[var(--accent)]/15" : "bg-[var(--surface-3)]",
                    )}
                  >
                    {tab.group.players.length}
                  </span>
                </button>
              );
            })}
            <span className="ml-auto font-mono text-[9px] text-[var(--muted)]">
              {groupSummary(activeTab.group)}
            </span>
          </div>
        </div>
        <div ref={tableScroll} className="thin-scroll max-h-[420px] overflow-auto">
          <table className="w-full text-left">
            <thead className="sticky top-0 z-10 bg-[var(--surface-2)] text-[9px] uppercase tracking-wider text-[var(--muted)]">
              <tr>
                <th className="px-3 py-1.5">{t("analysis.table.player")}</th>
                <th className="px-2 py-1.5 text-right">{t("analysis.table.age")}</th>
                <th className="px-2 py-1.5">{t("analysis.table.positions")}</th>
                {dataMode === "full" && (
                  <>
                    <SortHeader
                      label="CA"
                      by="ca"
                      active={sortBy}
                      dir={sortDir}
                      onClick={changeSort}
                    />
                    <SortHeader
                      label="PA"
                      by="pa"
                      active={sortBy}
                      dir={sortDir}
                      onClick={changeSort}
                    />
                  </>
                )}
                <th className="px-2 py-1.5 text-right">{t("analysis.table.wage")}</th>
                <th className="px-3 py-1.5 text-right">{t("analysis.table.contract")}</th>
              </tr>
            </thead>
            <tbody>
              {activeTab.group.players.map((player) => (
                <tr
                  key={player.uid}
                  data-testid="squad-row"
                  data-squad-slot={player.team_slot ?? "none"}
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
                  {dataMode === "full" && (
                    <>
                      <td className="px-2 py-1.5 text-right">
                        <span className={cn("inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold", caLabelClass)}>
                          {player.ca}
                        </span>
                      </td>
                      <td className="px-2 py-1.5 text-right">
                        <span className={cn("inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold", paTone(player.pa))}>
                          {player.pa ?? "?"}
                        </span>
                      </td>
                    </>
                  )}
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                    {player.wage != null ? formatWage(player.wage) : "—"}
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

type StatsView = {
  id: string;
  name: string;
  model: "core" | "advanced";
  window?: "overall" | "league" | "calendar";
  position: string;
  league: string;
  minCa: number;
  maxAge: number;
  minMinutes: number;
  maxFee: number | null;
  maxWage: number | null;
  contractBefore?: string;
  market?: string;
  minConfidence?: number | null;
  injuryFilter?: "all" | "fit" | "noHistory";
  includeUnknownFee: boolean;
};

const STATS_VIEWS_KEY = "fm27scout.statsViews";

function loadStatsViews(): StatsView[] {
  try {
    const raw = localStorage.getItem(STATS_VIEWS_KEY);
    return raw ? (JSON.parse(raw) as StatsView[]) : [];
  } catch {
    return [];
  }
}

function persistStatsViews(views: StatsView[]) {
  try {
    localStorage.setItem(STATS_VIEWS_KEY, JSON.stringify(views));
  } catch {
    // Local storage is optional in a webview; failure should not break analysis.
  }
}

function StatsMoneyballPanel({
  saveId,
  onPlayer,
}: {
  saveId: number | null;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale, formatValue } = useI18n();
  const dataMode = useAppStore((state) => state.dataMode);
  const [model, setModel] = useState<"core" | "advanced">("core");
  const [statsWindow, setStatsWindow] = useState<"overall" | "league" | "calendar">("overall");
  const [position, setPosition] = useState("");
  const [league, setLeague] = useState("");
  const [minCa, setMinCa] = useState(120);
  const [maxAge, setMaxAge] = useState(30);
  const [minMinutes, setMinMinutes] = useState(900);
  const [maxFee, setMaxFee] = useState<number | null>(null);
  const [maxWage, setMaxWage] = useState<number | null>(null);
  const [contractBefore, setContractBefore] = useState("");
  const [market, setMarket] = useState("all");
  const [minConfidence, setMinConfidence] = useState<number | null>(null);
  const [injuryFilter, setInjuryFilter] = useState<"all" | "fit" | "noHistory">("all");
  const [includeUnknownFee, setIncludeUnknownFee] = useState(false);
  const [page, setPage] = useState(0);
  const [sortBy, setSortBy] = useState("bargain");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [expandedUid, setExpandedUid] = useState<number | null>(null);
  const [savedViews, setSavedViews] = useState<StatsView[]>(loadStatsViews);
  const [exportFormat, setExportFormat] = useState<"csv" | "json">("csv");
  const [exporting, setExporting] = useState(false);
  const pushToast = useAppStore((state) => state.pushToast);

  useEffect(() => {
    setPage(0);
  }, [
    saveId,
    model,
    statsWindow,
    position,
    league,
    minCa,
    maxAge,
    minMinutes,
    maxFee,
    maxWage,
    contractBefore,
    market,
    minConfidence,
    injuryFilter,
    includeUnknownFee,
    sortBy,
    sortDir,
  ]);

  const request = useMemo<StatsMoneyballRequest | null>(
    () =>
      saveId == null
        ? null
        : {
            save_id: saveId,
            model,
            window: statsWindow,
            mode: dataMode,
            include_goalkeepers: position === "GK",
            position_group: position || null,
            league: league || null,
            ca_min: dataMode === "full" ? minCa : null,
            age_max: maxAge,
            min_minutes: minMinutes,
            max_fee: maxFee,
            max_wage: maxWage,
            contract_end_before: contractBefore || null,
            listings: market === "all" ? [] : [market],
            min_confidence: minConfidence,
            exclude_injured: injuryFilter !== "all",
            exclude_injury_history: injuryFilter === "noHistory",
            include_unknown_fee: includeUnknownFee,
            sort_by: sortBy,
            sort_dir: sortDir,
            page,
            page_size: VALUE_PAGE_SIZE,
          },
    [
      saveId,
      model,
      statsWindow,
      dataMode,
      position,
      league,
      minCa,
      maxAge,
      minMinutes,
      maxFee,
      maxWage,
      contractBefore,
      market,
      minConfidence,
      injuryFilter,
      includeUnknownFee,
      sortBy,
      sortDir,
      page,
    ],
  );
  useEffect(() => {
    if (dataMode !== "full" && sortBy === "ca") setSortBy("score");
  }, [dataMode, sortBy]);

  const query = useStatsMoneyball(request);
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
    setModel("core");
    setStatsWindow("overall");
    setPosition("");
    setLeague("");
    setMinCa(120);
    setMaxAge(30);
    setMinMinutes(900);
    setMaxFee(null);
    setMaxWage(null);
    setContractBefore("");
    setMarket("all");
    setMinConfidence(null);
    setInjuryFilter("all");
    setIncludeUnknownFee(false);
    setSortBy("bargain");
    setSortDir("desc");
    setPage(0);
  };

  const applyPreset = (preset: "win" | "resale" | "budget") => {
    setModel("core");
    setStatsWindow("overall");
    setPosition("");
    setLeague("");
    setMinMinutes(900);
    setContractBefore("");
    setMarket("all");
    setMinConfidence(null);
    setInjuryFilter("all");
    setIncludeUnknownFee(false);
    setSortBy("bargain");
    setSortDir("desc");
    setPage(0);
    if (preset === "win") {
      setMinCa(120);
      setMaxAge(30);
      setMaxFee(null);
      setMaxWage(null);
    } else if (preset === "resale") {
      setMinCa(110);
      setMaxAge(23);
      setMaxFee(null);
      setMaxWage(null);
    } else {
      setMinCa(110);
      setMaxAge(30);
      setMaxFee(5_000_000);
      setMaxWage(25_000);
    }
  };

  const exportNow = async () => {
    if (!request) return;
    const defaultName = `fm27-stats-${saveId}-${new Date().toISOString().slice(0, 10)}.${exportFormat}`;
    const path = await pickExportFile(defaultName, exportFormat);
    if (!path) return;
    setExporting(true);
    try {
      const report = await exportStats(request, exportFormat, path);
      pushToast({
        title: t("toast.exportDone"),
        description: t("toast.exportDoneDescription", {
          count: formatNumber(report.rows, locale),
          path: report.path,
        }),
        tone: "success",
      });
    } catch (error) {
      pushToast({ title: t("toast.exportFailed"), description: String(error), tone: "error" });
    } finally {
      setExporting(false);
    }
  };

  const saveStatsView = () => {
    const name = window.prompt(t("analysis.stats.saveViewPrompt"))?.trim();
    if (!name) return;
    const view: StatsView = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name,
      model,
      window: statsWindow,
      position,
      league,
      minCa,
      maxAge,
      minMinutes,
      maxFee,
      maxWage,
      contractBefore,
      market,
      minConfidence,
      injuryFilter,
      includeUnknownFee,
    };
    const next = [...savedViews, view].slice(-20);
    persistStatsViews(next);
    setSavedViews(next);
  };

  const applyStatsView = (id: string) => {
    const view = savedViews.find((item) => item.id === id);
    if (!view) return;
    setModel(view.model);
    setStatsWindow(view.window ?? "overall");
    setPosition(view.position);
    setLeague(view.league);
    setMinCa(view.minCa);
    setMaxAge(view.maxAge);
    setMinMinutes(view.minMinutes);
    setMaxFee(view.maxFee);
    setMaxWage(view.maxWage);
    setContractBefore(view.contractBefore ?? "");
    setMarket(view.market ?? "all");
    setMinConfidence(view.minConfidence ?? null);
    setInjuryFilter(view.injuryFilter ?? "all");
    setIncludeUnknownFee(view.includeUnknownFee);
    setSortBy("bargain");
    setSortDir("desc");
    setPage(0);
    setExpandedUid(null);
  };

  const flagLabel = (flag: string) => {
    const statsKey = `analysis.stats.flag.${flag}` as TranslationKey;
    const statsText = t(statsKey);
    return statsText === statsKey
      ? t(`analysis.value.flag.${flag}` as TranslationKey)
      : statsText;
  };

  const confidenceTone = (
    row: StatsMoneyballRow,
  ): "accent" | "default" | "danger" =>
    row.confidence >= 0.85 ? "accent" : row.confidence >= 0.65 ? "default" : "danger";

  return (
    <div className="space-y-3">
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
        <div className="flex flex-wrap items-end gap-2.5">
          <ValueFilter label={t("analysis.stats.model")} className="w-[180px]">
            <Select
              value={model}
              onChange={(event) => setModel(event.target.value as "core" | "advanced")}
              className="h-7 text-[10px]"
            >
              <option value="core">{t("analysis.stats.modelCore")}</option>
              <option value="advanced">{t("analysis.stats.modelAdvanced")}</option>
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.stats.window")} className="w-[150px]">
            <Select
              value={statsWindow}
              onChange={(event) =>
                setStatsWindow(event.target.value as "overall" | "league" | "calendar")
              }
              className="h-7 text-[10px]"
            >
              <option value="overall">{t("analysis.stats.windowOverall")}</option>
              <option value="league">{t("analysis.stats.windowLeague")}</option>
              <option value="calendar">{t("analysis.stats.windowCalendar")}</option>
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.stats.league")} className="w-[190px]">
            <Select
              value={league}
              onChange={(event) => setLeague(event.target.value)}
              className="h-7 text-[10px]"
            >
              <option value="">{t("analysis.stats.allLeagues")}</option>
              {leagues.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
          </ValueFilter>
          {dataMode === "full" && (
            <NumberField
              label={t("analysis.stats.minCa")}
              value={minCa}
              min={0}
              max={200}
              onChange={setMinCa}
            />
          )}
          <NumberField
            label={t("analysis.stats.maxAge")}
            value={maxAge}
            min={15}
            max={50}
            onChange={setMaxAge}
          />
          <DebouncedNumberField
            label={t("analysis.stats.minMinutes")}
            value={minMinutes}
            min={0}
            max={4000}
            onChange={setMinMinutes}
          />
          <label className="space-y-1">
            <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.stats.maxFee")}
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
              {t("analysis.stats.maxWage")}
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
          <label className="space-y-1">
            <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.stats.contractBefore")}
            </span>
            <Input
              type="date"
              value={contractBefore}
              onChange={(event) => setContractBefore(event.target.value)}
              className="h-7 w-[142px] text-[10px]"
            />
          </label>
          <ValueFilter label={t("analysis.stats.market")} className="w-[150px]">
            <Select
              value={market}
              onChange={(event) => setMarket(event.target.value)}
              className="h-7 text-[10px]"
            >
              <option value="all">{t("analysis.stats.marketAll")}</option>
              <option value="sale">{t("analysis.stats.marketSale")}</option>
              <option value="loan">{t("analysis.stats.marketLoan")}</option>
              <option value="free">{t("analysis.stats.marketFree")}</option>
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.stats.injury")} className="w-[150px]">
            <Select
              value={injuryFilter}
              onChange={(event) =>
                setInjuryFilter(event.target.value as "all" | "fit" | "noHistory")
              }
              className="h-7 text-[10px]"
            >
              <option value="all">{t("analysis.stats.injuryAll")}</option>
              <option value="fit">{t("analysis.stats.injuryFit")}</option>
              <option value="noHistory">{t("analysis.stats.injuryNoHistory")}</option>
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.stats.minConfidence")} className="w-[132px]">
            <Select
              value={minConfidence == null ? "" : String(minConfidence)}
              onChange={(event) =>
                setMinConfidence(event.target.value ? Number(event.target.value) : null)
              }
              className="h-7 text-[10px]"
            >
              <option value="">{t("analysis.stats.confidenceAny")}</option>
              <option value="0.6">≥ 60%</option>
              <option value="0.75">≥ 75%</option>
              <option value="0.85">≥ 85%</option>
            </Select>
          </ValueFilter>
          <label className="flex h-7 items-center gap-1.5 text-[10px] text-[var(--muted)]">
            <input
              type="checkbox"
              checked={includeUnknownFee}
              onChange={(event) => setIncludeUnknownFee(event.target.checked)}
              className="accent-[var(--accent)]"
            />
            {t("analysis.stats.includeUnknownFee")}
          </label>
          <div className="flex items-center gap-1">
            <span className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.stats.presets")}
            </span>
            {(["win", "resale", "budget"] as const).map((preset) => (
              <Button
                key={preset}
                variant="secondary"
                size="sm"
                className="h-7 px-2 text-[9px]"
                onClick={() => applyPreset(preset)}
              >
                {t(
                  preset === "win"
                    ? "analysis.stats.presetWinNow"
                    : preset === "resale"
                      ? "analysis.stats.presetResale"
                      : "analysis.stats.presetBudget",
                )}
              </Button>
            ))}
            <Button
              variant={sortBy === "score_low" ? "primary" : "secondary"}
              size="sm"
              className="h-7 px-2 text-[9px]"
              title={t("analysis.stats.conservativeHint")}
              onClick={() => {
                setSortBy(sortBy === "score_low" ? "score" : "score_low");
                setSortDir("desc");
                setPage(0);
              }}
            >
              {t("analysis.stats.conservative")}
            </Button>
          </div>
          {savedViews.length > 0 && (
            <ValueFilter label={t("analysis.stats.savedViews")} className="w-[150px]">
              <Select
                value=""
                onChange={(event) => applyStatsView(event.target.value)}
                className="h-7 text-[10px]"
              >
                <option value="">—</option>
                {savedViews.map((view) => (
                  <option key={view.id} value={view.id}>
                    {view.name}
                  </option>
                ))}
              </Select>
            </ValueFilter>
          )}
          <Button variant="secondary" size="sm" className="h-7" onClick={saveStatsView}>
            {t("analysis.stats.saveView")}
          </Button>
          <Button variant="ghost" size="sm" className="h-7" onClick={reset}>
            {t("analysis.stats.reset")}
          </Button>
          <Select
            value={exportFormat}
            onChange={(event) => setExportFormat(event.target.value as "csv" | "json")}
            className="h-7 w-[76px] text-[10px]"
            title={t("analysis.stats.export")}
          >
            <option value="csv">CSV</option>
            <option value="json">JSON</option>
          </Select>
          <Button
            variant="ghost"
            size="sm"
            className="h-7"
            onClick={() => void exportNow()}
            disabled={!request || exporting}
            title={t("analysis.stats.export")}
          >
            {exporting ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <Download size={11} />
            )}
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
        <div className="mt-2 flex flex-wrap items-center gap-1">
          <span className="mr-1 text-[9px] uppercase tracking-wider text-[var(--muted)]">
            {t("analysis.stats.position")}
          </span>
          {(["", ...VALUE_POSITIONS] as const).map((group) => (
            <button
              key={group || "all"}
              type="button"
              onClick={() => {
                setPosition(group);
                setExpandedUid(null);
              }}
              className={cn(
                "rounded border px-1.5 py-0.5 text-[9px] font-medium transition-colors",
                position === group
                  ? "border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                  : "border-[var(--line)] bg-[var(--surface)] text-[var(--muted)] hover:text-[var(--ink)]",
              )}
            >
              {group || t("analysis.stats.positionAll")}
            </button>
          ))}
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[9px] text-[var(--muted)]">
          <span>
            {data
              ? t("analysis.stats.summary", {
                  count: data.total,
                  shown: rows.length,
                  model: data.meta.model,
                })
              : t("analysis.stats.loading")}
          </span>
          <span>{t("analysis.stats.scoreHint")}</span>
          <span>{t("analysis.stats.priceHint")}</span>
          <span>{t("analysis.stats.leagueHint")}</span>
        </div>
      </div>

      {query.isLoading && !data ? (
        <div className="flex items-center justify-center gap-2 rounded-xl border border-dashed border-[var(--line)] py-12 text-[11px] text-[var(--muted)]">
          <Loader2 size={14} className="animate-spin" />
          {t("analysis.stats.loading")}
        </div>
      ) : query.isError ? (
        <div className="rounded-xl border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-4 text-[11px] text-[var(--danger)]">
          {t("analysis.loadError")}
        </div>
      ) : rows.length === 0 ? (
        <EmptyHint text={t("analysis.stats.empty")} />
      ) : (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <div className="thin-scroll max-h-[430px] overflow-auto">
            <table className="w-full min-w-[980px] text-left">
              <thead className="sticky top-0 z-10 bg-[var(--surface-2)] text-[9px] uppercase tracking-wider text-[var(--muted)]">
                <tr>
                  <th className="px-3 py-1.5">{t("analysis.stats.table.player")}</th>
                  <th className="px-2 py-1.5">{t("analysis.stats.table.position")}</th>
                  <SortHeader
                    label={t("analysis.stats.table.age")}
                    by="age"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  {dataMode === "full" && (
                    <SortHeader
                      label={t("analysis.stats.table.ca")}
                      by="ca"
                      active={sortBy}
                      dir={sortDir}
                      onClick={changeSort}
                    />
                  )}
                  <SortHeader
                    label={t("analysis.stats.table.minutes")}
                    by="minutes"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.stats.table.production")}
                    by="production"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.stats.table.cost")}
                    by="cost"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.stats.table.valueGap")}
                    by="value_gap"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.stats.table.bargain")}
                    by="bargain"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <th className="px-2 py-1.5 text-right">
                    {t("analysis.stats.table.confidence")}
                  </th>
                  <th className="px-3 py-1.5">{t("analysis.stats.table.flags")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <Fragment key={row.uid}>
                    <tr
                      className="cursor-pointer border-t border-[var(--line)]/60 text-[10px] hover:bg-[var(--surface-2)]"
                      onClick={() => onPlayer(row.uid)}
                    >
                    <td className="px-3 py-1.5">
                      <div className="flex items-center gap-1.5">
                        <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface-2)] text-[8px] font-bold text-[var(--muted)]">
                          {initials(row.name)}
                        </span>
                        <div className="min-w-0">
                          <div className="max-w-[180px] truncate font-medium text-[var(--ink)]">
                            {row.name ?? t("common.unnamed")}
                          </div>
                          <div className="max-w-[200px] truncate text-[9px] text-[var(--muted)]">
                            {row.club_name ?? t("common.freeAgent")}
                            {row.league_name ? ` · ${row.league_name}` : ""}
                          </div>
                        </div>
                        <button
                          type="button"
                          className="ml-auto flex h-5 w-5 shrink-0 items-center justify-center rounded border border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)] hover:text-[var(--ink)]"
                          title={
                            expandedUid === row.uid
                              ? t("analysis.stats.detail.collapse")
                              : t("analysis.stats.detail.expand")
                          }
                          onClick={(event) => {
                            event.stopPropagation();
                            setExpandedUid((current) =>
                              current === row.uid ? null : row.uid,
                            );
                          }}
                        >
                          {expandedUid === row.uid ? (
                            <ChevronDown size={11} />
                          ) : (
                            <ChevronRight size={11} />
                          )}
                        </button>
                      </div>
                    </td>
                    <td className="px-2 py-1.5 font-mono text-[10px] font-semibold text-[var(--accent-strong)]">
                      {row.position_group}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {row.age ?? "—"}
                    </td>
                    {dataMode === "full" && (
                      <td className="px-2 py-1.5 text-right">
                        <span className="inline-flex items-center gap-1">
                          <span
                            className={cn(
                              "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                              caLabelClass,
                            )}
                          >
                            {row.ca}
                          </span>
                          <span className="font-mono text-[9px] text-[var(--muted)]">/</span>
                          <span
                            className={cn(
                              "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                              paTone(row.pa),
                            )}
                          >
                            {row.pa ?? "?"}
                          </span>
                        </span>
                      </td>
                    )}
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatNumber(row.minutes, locale)}
                    </td>
                    <td
                      className="px-2 py-1.5 text-right font-mono text-[var(--muted)]"
                      title={row.drivers
                        .map((driver) => `${driver.label} ${Math.round(driver.percentile)}`)
                        .join(" · ")}
                    >
                      {row.production_tier != null ? Math.round(row.production_tier) : "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatValue(row.cost)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[9px]">
                      {row.value_gap_pct == null ? (
                        <span className="text-[var(--muted)]">—</span>
                      ) : (
                        <span
                          className={
                            row.value_gap_pct >= 0
                              ? "text-[var(--success)]"
                              : "text-[var(--danger)]"
                          }
                          title={t("analysis.stats.detail.valueGapHint")}
                        >
                          {row.value_gap_pct >= 0 ? "+" : ""}
                          {Math.round(row.value_gap_pct)}%
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <span
                        className={cn(
                          "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                          (row.bargain_score ?? 0) >= 75
                            ? "border-[var(--gold-line)] bg-[var(--gold-soft)] text-[var(--gold)]"
                            : (row.bargain_score ?? 0) >= 60
                              ? "border-[var(--success-line)] bg-[var(--success-soft)] text-[var(--success)]"
                              : (row.bargain_score ?? 0) >= 45
                                ? "border-[var(--blue-line)] bg-[var(--blue-soft)] text-[var(--blue)]"
                                : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)]",
                        )}
                      >
                        {row.bargain_score != null ? Math.round(row.bargain_score) : "—"}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 text-right">
                      <Badge tone={confidenceTone(row)} className="px-1.5 text-[8px]">
                        {Math.round(row.confidence * 100)}%
                      </Badge>
                    </td>
                    <td className="px-3 py-1.5">
                      <div className="flex max-w-[190px] flex-wrap gap-0.5">
                        {row.flags.slice(0, 3).map((flag) => (
                          <span
                            key={flag}
                            className="rounded border border-[var(--line)] bg-[var(--surface-2)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
                          >
                            {flagLabel(flag)}
                          </span>
                        ))}
                        {row.flags.length > 3 && (
                          <span
                            className="rounded border border-[var(--line)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
                            title={row.flags.map(flagLabel).join(", ")}
                          >
                            +{row.flags.length - 3}
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                    {expandedUid === row.uid && (
                      <tr className="border-t border-[var(--line)]/60 bg-[var(--surface-2)]/60">
                        <td colSpan={dataMode === "full" ? 11 : 10} className="px-3 py-2">
                          <StatsMoneyballDetail
                            row={row}
                            request={request}
                            onPlayer={onPlayer}
                          />
                        </td>
                      </tr>
                    )}
                  </Fragment>
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

function StatsMoneyballDetail({
  row,
  request,
  onPlayer,
}: {
  row: StatsMoneyballRow;
  request: StatsMoneyballRequest | null;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale, formatValue, formatWage } = useI18n();
  const flagLabel = (flag: string) => {
    const statsKey = `analysis.stats.flag.${flag}` as TranslationKey;
    const statsText = t(statsKey);
    return statsText === statsKey
      ? t(`analysis.value.flag.${flag}` as TranslationKey)
      : statsText;
  };
  return (
    <div className="space-y-3 text-[10px]">
      <div className="grid gap-3 lg:grid-cols-3">
      <div className="space-y-1">
        <div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          {t("analysis.stats.detail.title")}
        </div>
        <div className="text-[var(--ink)]">
          {t("analysis.stats.detail.sample")}: {formatNumber(row.minutes, locale)} min ·{" "}
          {row.starts} starts · {row.substitute_appearances} sub
        </div>
        <div className="text-[var(--muted)]">
          {t("analysis.stats.detail.advanced")}:{" "}
          {row.advanced_available
            ? t("analysis.stats.detail.available")
            : t("analysis.stats.detail.missing")}
        </div>
        <div className="font-mono text-[var(--muted)]">
          {t("analysis.stats.detail.production")}: {Math.round(row.production_tier ?? 0)} ·{" "}
          {t("analysis.stats.detail.costPercentile")}:{" "}
          {Math.round(row.cost_pct_tier ?? 0)}
        </div>
        <div className="font-mono text-[var(--muted)]">
          {t("analysis.stats.detail.range")}:{" "}
          {Math.round(row.production_low ?? 0)}–{Math.round(row.production_high ?? 0)}
        </div>
        <div className="text-[var(--muted)]">
          {t("analysis.stats.detail.peers", { count: row.peer_count })}
        </div>
        <div className="flex flex-wrap gap-1 pt-1">
          {row.flags.map((flag) => (
            <span
              key={flag}
              className="rounded border border-[var(--line)] bg-[var(--surface)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
            >
              {flagLabel(flag)}
            </span>
          ))}
        </div>
      </div>
      <div>
        <div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          {t("analysis.stats.detail.drivers")}
        </div>
        <div className="mt-1 space-y-1">
          {row.drivers.length === 0 ? (
            <span className="text-[var(--muted)]">—</span>
          ) : (
            row.drivers.map((driver) => (
              <div key={driver.key} className="flex items-center justify-between gap-3">
                <span className="text-[var(--muted)]">{driver.label}</span>
                <span className="font-mono text-[var(--ink)]">
                  {Math.round(driver.percentile)}
                </span>
              </div>
            ))
          )}
        </div>
      </div>
      <div>
        <div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          {t("analysis.stats.detail.cost")}
        </div>
        <div className="mt-1 space-y-1 font-mono text-[var(--muted)]">
          <div className="flex justify-between gap-3">
            <span>{t("analysis.stats.detail.fairPrice")}</span>
            <span className="text-[var(--ink)]">
              {row.fair_price == null ? "—" : formatValue(Math.round(row.fair_price))}
            </span>
          </div>
          <div className="flex justify-between gap-3">
            <span>{t("analysis.stats.detail.valueGap")}</span>
            <span
              className={
                row.value_gap_pct == null
                  ? "text-[var(--muted)]"
                  : row.value_gap_pct >= 0
                    ? "text-[var(--success)]"
                    : "text-[var(--danger)]"
              }
            >
              {row.value_gap_pct == null
                ? "—"
                : `${row.value_gap_pct >= 0 ? "+" : ""}${Math.round(row.value_gap_pct)}%`}
            </span>
          </div>
          <div className="flex justify-between gap-3 border-t border-[var(--line)] pt-1">
            <span>{t("analysis.stats.detail.fee")}</span>
            <span className="text-[var(--ink)]">
              {row.fee == null ? t("analysis.value.unknownFee") : formatValue(row.fee)}
            </span>
          </div>
          <div className="flex justify-between gap-3">
            <span>{t("analysis.stats.detail.wage")}</span>
            <span className="text-[var(--ink)]">{formatWage(row.wage)}</span>
          </div>
          <div className="flex justify-between gap-3 border-t border-[var(--line)] pt-1">
            <span>{t("analysis.stats.detail.total")}</span>
            <span className="text-[var(--ink)]">{formatValue(row.cost)}</span>
          </div>
        </div>
        <div className="mt-2 text-[9px] text-[var(--muted)]">
          {t("analysis.stats.priceHint")}
        </div>
        </div>
      </div>
      <SimilarStatsList row={row} request={request} onPlayer={onPlayer} />
    </div>
  );
}

function SimilarStatsList({
  row,
  request,
  onPlayer,
}: {
  row: StatsMoneyballRow;
  request: StatsMoneyballRequest | null;
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  const similar = useSimilarStatsPlayers(request, row.uid);
  const hits = similar.data ?? [];
  if (similar.isLoading) {
    return (
      <div className="flex items-center gap-1 text-[9px] text-[var(--muted)]">
        <Loader2 size={10} className="animate-spin" />
        {t("analysis.stats.detail.similarLoading")}
      </div>
    );
  }
  if (hits.length === 0) return null;
  return (
    <div className="border-t border-[var(--line)] pt-2">
      <div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
        {t("analysis.stats.detail.similar")}
      </div>
      <div className="mt-1 grid gap-1 sm:grid-cols-2 lg:grid-cols-3">
        {hits.map((hit) => (
          <button
            key={hit.uid}
            type="button"
            onClick={() => onPlayer(hit.uid)}
            title={`${t("analysis.stats.detail.distance")} ${hit.distance.toFixed(1)}`}
            className="flex items-center justify-between gap-2 rounded border border-[var(--line)] bg-[var(--surface)] px-2 py-1 text-left text-[9px] hover:border-[var(--accent-line)] hover:bg-[var(--accent-soft)]"
          >
            <span className="min-w-0 truncate text-[var(--ink)]">
              {hit.name ?? t("common.unnamed")}
              <span className="text-[var(--muted)]">
                {" "}
                · {hit.club_name ?? t("common.freeAgent")}
              </span>
            </span>
            <span className="shrink-0 font-mono text-[9px] text-[var(--muted)]">
              {hit.position_group} · {hit.age ?? "—"} · Q{Math.round(hit.quality ?? 0)}
              {hit.value_gap_pct != null && (
                <span
                  className={
                    hit.value_gap_pct >= 0 ? "text-[var(--success)]" : "text-[var(--danger)]"
                  }
                >
                  {" "}
                  {hit.value_gap_pct >= 0 ? "+" : ""}
                  {Math.round(hit.value_gap_pct)}%
                </span>
              )}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

const VALUE_PAGE_SIZE = 40;
const VALUE_POSITIONS = ["GK", "DC", "FB", "DM", "MC", "WM", "AMC", "ST"] as const;

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
  const { t, locale, formatValue, formatWage } = useI18n();
  const dataMode = useAppStore((state) => state.dataMode);
  const [position, setPosition] = useState("");
  const [league, setLeague] = useState("");
  const [minCa, setMinCa] = useState(120);
  const [minRole, setMinRole] = useState(12);
  const [maxAge, setMaxAge] = useState(30);
  const [maxFee, setMaxFee] = useState<number | null>(null);
  const [maxWage, setMaxWage] = useState<number | null>(null);
  const [market, setMarket] = useState("all");
  const [contractBefore, setContractBefore] = useState("");
  const [minConfidence, setMinConfidence] = useState<number | null>(null);
  const [injuryFilter, setInjuryFilter] = useState<"all" | "fit" | "noHistory">("all");
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
    contractBefore,
    minConfidence,
    injuryFilter,
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
            mode: dataMode,
            position_group: position || null,
            league: league || null,
            age_max: maxAge,
            ca_min: dataMode === "full" ? minCa : null,
            role_min: minRole,
            max_fee: maxFee,
            max_wage: maxWage,
            contract_end_before: contractBefore || null,
            listings: market === "all" ? [] : [market],
            min_confidence: minConfidence,
            exclude_injured: injuryFilter !== "all",
            exclude_injury_history: injuryFilter === "noHistory",
            include_unknown_fee: includeUnknownFee,
            sort_by: sortBy,
            sort_dir: sortDir,
            page,
            page_size: VALUE_PAGE_SIZE,
          },
    [
      saveId,
      dataMode,
      position,
      league,
      minCa,
      minRole,
      maxAge,
      maxFee,
      maxWage,
      market,
      contractBefore,
      minConfidence,
      injuryFilter,
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
    setContractBefore("");
    setMinConfidence(null);
    setInjuryFilter("all");
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
          {dataMode === "full" && (
            <NumberField
              label={t("analysis.value.minCa")}
              value={minCa}
              min={0}
              max={200}
              onChange={setMinCa}
            />
          )}
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
          <label className="space-y-1">
            <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
              {t("analysis.value.contractBefore")}
            </span>
            <Input
              type="date"
              value={contractBefore}
              onChange={(event) => setContractBefore(event.target.value)}
              className="h-7 w-[142px] text-[10px]"
            />
          </label>
          <ValueFilter label={t("analysis.value.injury")} className="w-[150px]">
            <Select
              value={injuryFilter}
              onChange={(event) =>
                setInjuryFilter(event.target.value as "all" | "fit" | "noHistory")
              }
              className="h-7 text-[10px]"
            >
              <option value="all">{t("analysis.value.injuryAll")}</option>
              <option value="fit">{t("analysis.value.injuryFit")}</option>
              <option value="noHistory">{t("analysis.value.injuryNoHistory")}</option>
            </Select>
          </ValueFilter>
          <ValueFilter label={t("analysis.value.minConfidence")} className="w-[132px]">
            <Select
              value={minConfidence == null ? "" : String(minConfidence)}
              onChange={(event) =>
                setMinConfidence(event.target.value ? Number(event.target.value) : null)
              }
              className="h-7 text-[10px]"
            >
              <option value="">{t("analysis.value.confidenceAny")}</option>
              <option value="0.7">≥ 70%</option>
              <option value="0.85">≥ 85%</option>
              <option value="0.9">≥ 90%</option>
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
                  <SortHeader
                    label={t("analysis.value.table.age")}
                    by="age"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  {dataMode === "full" && (
                    <SortHeader
                      label={t("analysis.value.table.ca")}
                      by="ca"
                      active={sortBy}
                      dir={sortDir}
                      onClick={changeSort}
                    />
                  )}
                  <SortHeader
                    label={t("analysis.value.table.role")}
                    by="role"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.value.table.fee")}
                    by="fee"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.value.table.wage")}
                    by="wage"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
                    label={t("analysis.value.table.cost")}
                    by="cost"
                    active={sortBy}
                    dir={sortDir}
                    onClick={changeSort}
                  />
                  <SortHeader
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
                    {dataMode === "full" && (
                      <td className="px-2 py-1.5 text-right">
                        <span className="inline-flex items-center gap-1">
                          <span
                            className={cn(
                              "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                              caLabelClass,
                            )}
                          >
                            {row.ca}
                          </span>
                          <span className="font-mono text-[9px] text-[var(--muted)]">/</span>
                          <span
                            className={cn(
                              "inline-flex rounded border px-1.5 py-0.5 font-mono text-[9px] font-bold",
                              paTone(row.pa),
                            )}
                          >
                            {row.pa ?? "?"}
                          </span>
                        </span>
                      </td>
                    )}
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {row.role_score != null ? `${row.role_score}/20` : "—"}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {feeLabel(row)}
                    </td>
                    <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                      {formatWage(row.wage)}
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

function SortHeader({
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

function DebouncedNumberField(props: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(props.value);

  useEffect(() => {
    setDraft(props.value);
  }, [props.value]);

  useEffect(() => {
    if (draft === props.value) return;
    const timer = window.setTimeout(() => props.onChange(draft), 350);
    return () => window.clearTimeout(timer);
  }, [draft, props.value, props.onChange]);

  return <NumberField {...props} value={draft} onChange={setDraft} />;
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
