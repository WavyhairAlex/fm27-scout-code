import { type ReactNode, useEffect, useMemo, useState } from "react";
import { Compass, RefreshCw } from "lucide-react";
import { useI18n } from "@/lib/useI18n";
import { useScoutAdvice, useScoutOverview } from "@/lib/query";
import { roleName } from "@/lib/labels";
import { formatNumber, type TranslationKey } from "@/lib/i18n";
import { convertToBase } from "@/lib/currency";
import {
  POSITION_LABELS,
  type ScoutDeparture,
  type ScoutDepartureAction,
  type ScoutFitting,
  type ScoutLine,
  type ScoutOverview,
  type ScoutPurpose,
  type ScoutRequest,
  type ScoutSuggestion,
} from "@/lib/types";
import { useAppStore } from "@/store/app";
import { caLabelClass, cn, formatDate, paTone } from "@/lib/utils";
import { Button, Input, Select } from "./ui";

type ScoutView = "advice" | "depth" | "departures";

const VIEWS: ScoutView[] = ["advice", "depth", "departures"];

/** Positions on the pitch grid: [key, column, row], attack at the top. */
const PITCH: [string, number, number][] = [
  ["stc", 2, 1],
  ["aml", 1, 2],
  ["amc", 2, 2],
  ["amr", 3, 2],
  ["ml", 1, 3],
  ["mc", 2, 3],
  ["mr", 3, 3],
  ["wbl", 1, 4],
  ["dm", 2, 4],
  ["wbr", 3, 4],
  ["dl", 1, 5],
  ["dc", 2, 5],
  ["dr", 3, 5],
  ["gk", 2, 6],
];

const LINE_OF_POSITION: Record<string, string> = {
  gk: "gk",
  dc: "dc",
  dr: "rightBack",
  wbr: "rightBack",
  dl: "leftBack",
  wbl: "leftBack",
  dm: "dm",
  mc: "mc",
  amr: "rightWing",
  mr: "rightWing",
  aml: "leftWing",
  ml: "leftWing",
  amc: "amc",
  stc: "st",
};

const PURPOSE_KEYS: ScoutPurpose[] = ["firstTeam", "rotation", "future", "any"];

const DEPARTURE_ACTIONS: ScoutDepartureAction[] = ["sell", "loan", "release"];

const STATUS_DOT: Record<string, string> = {
  missing: "bg-[var(--danger)]",
  thin: "bg-[var(--gold)]",
  fine: "bg-[var(--success)]",
  crowded: "bg-[var(--accent)]",
};

function optionalNumber(value: string): number | null {
  if (value.trim() === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? Math.max(0, Math.round(number)) : null;
}

function defaultPosition(overview: ScoutOverview): string {
  const needy =
    overview.lines.find((line) => line.status === "missing" || line.status === "thin") ??
    overview.lines.find((line) => line.weak) ??
    overview.lines.find((line) => line.ageing);
  const wanted = needy?.positions[0] ?? "stc";
  return overview.positions.includes(wanted) ? wanted : overview.positions[0] ?? "stc";
}

function lineOf(overview: ScoutOverview, position: string): ScoutLine | undefined {
  const key = LINE_OF_POSITION[position];
  return overview.lines.find((line) => line.key === key);
}

function fittingTitle(fitting: ScoutFitting, locale: "en" | "zh-CN"): string {
  return `${roleName(fitting.role_id, fitting.role_name, locale) ?? fitting.role_id} · ${fitting.role_score.toFixed(1)}`;
}

export function ScoutPanel({
  clubUid,
  onPlayer,
}: {
  clubUid: number;
  onPlayer: (uid: number) => void;
}) {
  const { t, locale, formatValue, formatMoney, formatWage } = useI18n();
  const saveId = useAppStore((state) => state.saveId);
  const currency = useAppStore((state) => state.currency);
  const overview = useScoutOverview(clubUid);
  const [view, setView] = useState<ScoutView>("advice");
  const [position, setPosition] = useState("");
  const [roleId, setRoleId] = useState("");
  const [purpose, setPurpose] = useState<ScoutPurpose>("firstTeam");
  const [minAge, setMinAge] = useState<number | null>(null);
  const [maxAge, setMaxAge] = useState<number | null>(null);
  const [maxFee, setMaxFee] = useState<number | null>(null);
  const [minProbability, setMinProbability] = useState<number | null>(null);
  const [noFreeAgents, setNoFreeAgents] = useState(false);
  const [adviceSort, setAdviceSort] = useState<{
    by: "rank" | "fit" | "probability" | "fee";
    dir: "asc" | "desc";
  }>({ by: "rank", dir: "asc" });

  useEffect(() => {
    if (!overview.data) return;
    setPosition((current) =>
      current && overview.data!.positions.includes(current)
        ? current
        : defaultPosition(overview.data!),
    );
  }, [overview.data]);

  useEffect(() => {
    setRoleId((current) => {
      if (!current || !overview.data) return current;
      const role = overview.data.roles.find((candidate) => candidate.id === current);
      return role?.positions.includes(position) ? current : "";
    });
  }, [position, overview.data]);

  const request = useMemo<ScoutRequest | null>(() => {
    if (saveId == null || !position) return null;
    return {
      save_id: saveId,
      club_uid: clubUid,
      position,
      role_id: roleId || null,
      purpose,
      min_age: minAge,
      max_age: maxAge,
      max_fee: maxFee == null ? null : Math.round(convertToBase(maxFee, currency)),
      min_probability: minProbability,
      no_free_agents: noFreeAgents,
      how_many: 50,
    };
  }, [
    saveId,
    clubUid,
    position,
    roleId,
    purpose,
    minAge,
    maxAge,
    maxFee,
    currency,
    minProbability,
    noFreeAgents,
  ]);
  const advice = useScoutAdvice(request);
  const sortedSuggestions = useMemo(() => {
    const rows = (advice.data?.suggestions ?? []).map((suggestion, index) => ({
      suggestion,
      rank: index + 1,
    }));
    if (adviceSort.by === "rank") return rows;
    const value = (row: (typeof rows)[number]) => {
      switch (adviceSort.by) {
        case "fit":
          return row.suggestion.fit;
        case "probability":
          return row.suggestion.probability;
        case "fee":
          return row.suggestion.fee;
      }
    };
    const direction = adviceSort.dir === "asc" ? 1 : -1;
    return rows.sort((left, right) => {
      const first = value(left);
      const second = value(right);
      if (first == null || second == null) {
        return first === second ? left.rank - right.rank : first == null ? 1 : -1;
      }
      return (Number(first) - Number(second)) * direction || left.rank - right.rank;
    });
  }, [advice.data, adviceSort]);

  if (overview.isLoading) {
    return <div className="py-10 text-center text-xs text-[var(--muted)]">{t("scout.reading")}</div>;
  }
  if (overview.isError || !overview.data) {
    return (
      <div className="space-y-3 py-8 text-center text-xs text-[var(--danger)]">
        <div>{t("scout.loadError")}</div>
        <Button size="sm" variant="secondary" onClick={() => void overview.refetch()}>
          <RefreshCw size={13} />
          {t("scout.refresh")}
        </Button>
      </div>
    );
  }

  const data = overview.data;
  const line = lineOf(data, position);
  const roles = data.roles.filter((role) => role.positions.includes(position));
  const gameDate = advice.data?.game_date ?? null;
  const choosePosition = (key: string) => {
    setPosition(key);
    setRoleId((current) => {
      if (!current) return current;
      const role = data.roles.find((candidate) => candidate.id === current);
      return role?.positions.includes(key) ? current : "";
    });
  };
  const toggleSort = (by: "rank" | "fit" | "probability" | "fee") => {
    setAdviceSort((current) =>
      current.by === by
        ? { by, dir: current.dir === "asc" ? "desc" : "asc" }
        : { by, dir: by === "fee" || by === "rank" ? "asc" : "desc" },
    );
  };
  const sortHeader = (
    by: "rank" | "fit" | "probability" | "fee",
    label: string,
    align: "left" | "right" = "left",
  ) => (
    <button
      type="button"
      className={cn(
        "inline-flex items-center gap-0.5 hover:text-[var(--ink)]",
        align === "right" && "w-full justify-end",
      )}
      onClick={() => toggleSort(by)}
      title={t("scout.sortBy", { column: label })}
    >
      {label}
      <span className="text-[7px]">
        {adviceSort.by === by ? (adviceSort.dir === "asc" ? "▲" : "▼") : ""}
      </span>
    </button>
  );

  const reasonText = (code: string, value: number | null): string => {
    const key = `scout.reason.${code}` as TranslationKey;
    const money = ["releaseClause", "overBudget", "stretch", "wageHigh"].includes(code);
    const shown = value == null ? null : money ? formatMoney(value) : formatNumber(value, locale);
    return t(key, shown == null ? undefined : { value: shown });
  };
  const departureReasonText = (code: string, value: number | null): string => {
    const key = `scout.departureReason.${code}` as TranslationKey;
    const money = code === "highWage";
    const shown = value == null ? null : money ? formatMoney(value) : formatNumber(value, locale);
    return t(key, shown == null ? undefined : { value: shown });
  };

  return (
    <div className="space-y-4" data-testid="scout-panel">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-semibold text-[var(--ink)]">
          <Compass size={16} className="text-[var(--accent)]" />
          {data.club_name}
        </div>
        <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-[10px]">
          <Figure label={t("scout.header.transferBudget")} value={formatMoney(data.transfer_budget)} />
          <Figure
            label={data.wage_room == null ? t("scout.header.wageBudget") : t("scout.header.wageRoom")}
            value={formatWage(data.wage_room ?? data.wage_budget)}
            tone={data.wage_room != null && data.wage_room < 0}
          />
          <Figure label={t("scout.header.squad")} value={t("scout.header.players", { count: data.squad_size })} />
          <Figure
            label={t("scout.header.level")}
            value={formatNumber(data.level, locale)}
          />
        </div>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            void overview.refetch();
            if (request) void advice.refetch();
          }}
          disabled={overview.isFetching}
        >
          <RefreshCw size={13} className={overview.isFetching ? "animate-spin" : undefined} />
          {t("scout.refresh")}
        </Button>
      </div>

      <div className="flex gap-1 border-b border-[var(--line)]">
        {VIEWS.map((id) => (
          <button
            key={id}
            type="button"
            data-testid={`scout-view-${id}`}
            onClick={() => setView(id)}
            className={cn(
              "focus-ring flex items-center gap-1.5 rounded-t-lg border-b-2 px-3 py-2 text-[11px] font-medium transition-colors",
              view === id
                ? "border-[var(--accent)] bg-[var(--surface-2)] text-[var(--accent-strong)]"
                : "border-transparent text-[var(--muted)] hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
            )}
          >
            {t(`scout.view.${id}` as TranslationKey)}
            {id === "departures" && data.departures.length > 0 && (
              <span className="rounded-full bg-[var(--surface-3)] px-1.5 py-0.5 text-[9px]">
                {data.departures.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {view === "advice" && (
        <div className="grid gap-4 lg:grid-cols-[250px_minmax(0,1fr)]">
          <aside className="space-y-4">
            <section className="space-y-2">
              <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                {t("scout.position")}
              </div>
              <div
                className="grid grid-cols-3 gap-1 rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-1.5"
                role="group"
                aria-label={t("scout.position")}
              >
                {PITCH.filter(([key]) => data.positions.includes(key)).map(([key, column, row]) => {
                  const pitchLine = lineOf(data, key);
                  return (
                    <button
                      key={key}
                      type="button"
                      onClick={() => choosePosition(key)}
                      aria-pressed={position === key}
                      title={
                        pitchLine
                          ? t("scout.positionTip", {
                              line: t(`scout.line.${pitchLine.key}` as TranslationKey),
                              count: pitchLine.players.length,
                              target: pitchLine.target,
                              status: t(`scout.status.${pitchLine.status}` as TranslationKey),
                            })
                          : POSITION_LABELS[key]
                      }
                      style={{ gridColumn: column, gridRow: row }}
                      className={cn(
                        "focus-ring relative flex h-8 items-center justify-center rounded-md border text-[10px] font-semibold transition-colors",
                        position === key
                          ? "border-[var(--accent)] bg-[var(--accent)] text-white"
                          : "border-[var(--line)] bg-[var(--surface)] text-[var(--ink)] hover:border-[var(--accent-line)]",
                      )}
                    >
                      {POSITION_LABELS[key]}
                      {pitchLine && (
                        <span
                          className={cn(
                            "absolute right-1 top-1 h-1.5 w-1.5 rounded-full",
                            STATUS_DOT[pitchLine.status],
                          )}
                        />
                      )}
                    </button>
                  );
                })}
              </div>
            </section>

            <section className="space-y-2">
              <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                {t("scout.purpose")}
              </div>
              <div className="grid grid-cols-2 gap-1" role="group" aria-label={t("scout.purpose")}>
                {PURPOSE_KEYS.map((value) => (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setPurpose(value)}
                    aria-pressed={purpose === value}
                    title={t(`scout.purposeTip.${value}` as TranslationKey)}
                    className={cn(
                      "focus-ring rounded-md border px-2 py-1.5 text-[10px] font-medium transition-colors",
                      purpose === value
                        ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                        : "border-[var(--line)] bg-[var(--surface)] text-[var(--muted)] hover:border-[var(--line-strong)]",
                    )}
                  >
                    {t(`scout.purpose.${value}` as TranslationKey)}
                  </button>
                ))}
              </div>
            </section>

            <label className="block space-y-1">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                {t("scout.role")}
              </span>
              <Select value={roleId} onChange={(event) => setRoleId(event.target.value)}>
                <option value="">{t("scout.anyRole")}</option>
                {roles.map((role) => (
                  <option key={role.id} value={role.id}>
                    {roleName(role.id, role.name, locale) ?? role.id}
                  </option>
                ))}
              </Select>
            </label>

            <details className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
              <summary className="cursor-pointer text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                {t("scout.filters")}
              </summary>
              <div className="mt-2 grid grid-cols-2 gap-2">
                <FilterField label={t("scout.minAge")}>
                  <Input
                    type="number"
                    min={15}
                    max={60}
                    value={minAge ?? ""}
                    onChange={(event) => setMinAge(optionalNumber(event.target.value))}
                  />
                </FilterField>
                <FilterField label={t("scout.maxAge")}>
                  <Input
                    type="number"
                    min={15}
                    max={60}
                    value={maxAge ?? ""}
                    onChange={(event) => setMaxAge(optionalNumber(event.target.value))}
                  />
                </FilterField>
                <FilterField label={t("scout.maxFee", { currency })}>
                  <Input
                    type="number"
                    min={0}
                    value={maxFee ?? ""}
                    placeholder="—"
                    onChange={(event) => setMaxFee(optionalNumber(event.target.value))}
                  />
                </FilterField>
                <FilterField label={t("scout.minProbability")}>
                  <Select
                    value={minProbability ?? ""}
                    onChange={(event) =>
                      setMinProbability(
                        event.target.value === "" ? null : Number(event.target.value),
                      )
                    }
                  >
                    <option value="">{t("scout.anyProbability")}</option>
                    {[30, 50, 70].map((value) => (
                      <option key={value} value={value}>
                        {value}%+
                      </option>
                    ))}
                  </Select>
                </FilterField>
                <label className="col-span-2 flex items-center gap-2 text-[10px] text-[var(--muted)]">
                  <input
                    type="checkbox"
                    checked={noFreeAgents}
                    onChange={(event) => setNoFreeAgents(event.target.checked)}
                  />
                  {t("scout.noFreeAgents")}
                </label>
              </div>
            </details>
          </aside>

          <div className="min-w-0 space-y-3">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <div className="text-xs font-semibold text-[var(--ink)]">
                {t("scout.adviceFor", {
                  position: POSITION_LABELS[position] ?? position,
                  purpose: t(`scout.purpose.${purpose}` as TranslationKey),
                })}
              </div>
              <div className="text-[10px] text-[var(--muted)]">
                {advice.data
                  ? t("scout.summary", {
                      examined: formatNumber(advice.data.examined, locale),
                      candidates: formatNumber(advice.data.candidates, locale),
                      eligible: formatNumber(advice.data.eligible, locale),
                      shown: advice.data.suggestions.length,
                    })
                  : advice.isLoading
                    ? t("scout.searching")
                    : ""}
              </div>
            </div>

            <details className="text-[9px] text-[var(--muted)]">
              <summary className="cursor-pointer select-none">{t("scout.how.label")}</summary>
              <p className="mt-1 max-w-[720px] leading-relaxed">{t("scout.how.note")}</p>
            </details>

            <OwnSquad
              squad={advice.data?.squad ?? []}
              line={line}
              locale={locale}
              onPlayer={onPlayer}
            />

            {advice.isError ? (
              <div className="rounded-lg border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-2 text-[10px] text-[var(--danger)]">
                {t("scout.adviceError")}
              </div>
            ) : null}

            {advice.data && advice.data.suggestions.length === 0 ? (
              <div className="rounded-lg border border-dashed border-[var(--line)] px-3 py-8 text-center text-[10px] text-[var(--muted)]">
                {t("scout.empty")}
              </div>
            ) : null}

            {advice.data && advice.data.suggestions.length > 0 && (
              <div className="overflow-x-auto rounded-lg border border-[var(--line)]">
                <table className="w-full table-fixed border-collapse text-[10px]">
                  <thead className="bg-[var(--surface-2)] text-[var(--muted)]">
                    <tr>
                      <th className="w-8 px-2 py-1.5 text-right font-medium">
                        {sortHeader("rank", "#", "right")}
                      </th>
                      <th className="px-2 py-1.5 text-left font-medium">{t("scout.column.player")}</th>
                      <th className="w-9 px-1 py-1.5 text-right font-medium">{t("scout.column.age")}</th>
                      <th className="w-9 px-1 py-1.5 text-right font-medium">CA</th>
                      <th className="w-9 px-1 py-1.5 text-right font-medium">PA</th>
                      <th className="w-24 px-2 py-1.5 text-left font-medium">
                        {sortHeader("fit", t("scout.column.fit"))}
                      </th>
                      <th
                        className="w-12 px-1 py-1.5 text-right font-medium"
                        title={t("scout.probabilityInfo")}
                      >
                        {sortHeader("probability", t("scout.column.probability"), "right")}
                      </th>
                      <th className="w-20 px-2 py-1.5 text-right font-medium">
                        {sortHeader("fee", t("scout.column.fee"), "right")}
                      </th>
                      <th className="w-[150px] px-2 py-1.5 text-left font-medium">{t("scout.column.reasons")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sortedSuggestions.map(({ suggestion, rank }) => (
                      <SuggestionRow
                        key={suggestion.player.uid}
                        rank={rank}
                        suggestion={suggestion}
                        locale={locale}
                        reasonText={reasonText}
                        formatValue={formatValue}
                        formatWage={formatWage}
                        onPlayer={onPlayer}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {gameDate && (
              <div className="text-[9px] text-[var(--muted)]">
                {t("scout.estimates", { date: gameDate })}
              </div>
            )}
          </div>
        </div>
      )}

      {view === "depth" && (
        <DepthTable
          overview={data}
          locale={locale}
          onAdvice={(key) => {
            setPosition(key);
            setRoleId("");
            setView("advice");
          }}
          onPlayer={onPlayer}
        />
      )}

      {view === "departures" && (
        <DepartureTable
          departures={data.departures}
          locale={locale}
          departureReasonText={departureReasonText}
          formatValue={formatValue}
          formatWage={formatWage}
          onPlayer={onPlayer}
        />
      )}
    </div>
  );
}

function Figure({
  label,
  value,
  tone = false,
}: {
  label: string;
  value: string;
  tone?: boolean;
}) {
  return (
    <span className="flex flex-col">
      <span className="uppercase tracking-wider text-[var(--muted)]">{label}</span>
      <span className={cn("font-mono text-[11px]", tone ? "text-[var(--danger)]" : "text-[var(--ink)]")}>
        {value}
      </span>
    </span>
  );
}

function FilterField({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="block text-[9px] text-[var(--muted)]">{label}</span>
      {children}
    </label>
  );
}

function FitBar({ fit, title }: { fit: number; title?: string }) {
  const tone =
    fit >= 70 ? "bg-[var(--success)]" : fit >= 50 ? "bg-[var(--gold)]" : "bg-[var(--danger)]";
  return (
    <div className="flex items-center gap-1.5" title={title}>
      <span className="h-1.5 w-16 overflow-hidden rounded-full bg-[var(--line)]">
        <span className={cn("block h-full rounded-full", tone)} style={{ width: `${fit}%` }} />
      </span>
      <span className="w-6 text-right font-mono text-[10px] text-[var(--ink)]">{fit}</span>
    </div>
  );
}

function OwnSquad({
  squad,
  line,
  locale,
  onPlayer,
}: {
  squad: ScoutFitting[];
  line: ScoutLine | undefined;
  locale: "en" | "zh-CN";
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  return (
    <section className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10px] font-semibold text-[var(--ink)]">{t("scout.own")}</span>
        {line && (
          <span className="flex items-center gap-1.5 text-[9px] text-[var(--muted)]">
            <span className={cn("h-1.5 w-1.5 rounded-full", STATUS_DOT[line.status])} />
            {t(`scout.line.${line.key}` as TranslationKey)} {line.players.length}/{line.target}
          </span>
        )}
      </div>
      {squad.length === 0 ? (
        <div className="mt-1 text-[10px] text-[var(--muted)]">{t("scout.ownEmpty")}</div>
      ) : (
        <ul className="mt-1.5 space-y-1">
          {squad.map((fitting) => (
            <li key={fitting.player.uid} className="flex items-center gap-2 text-[10px]">
              <button
                type="button"
                className="name-link min-w-0 flex-1 truncate text-left text-[var(--ink)] hover:text-[var(--accent)]"
                onClick={() => onPlayer(fitting.player.uid)}
              >
                {fitting.player.name ?? t("common.unnamedPlayer")}
              </button>
              <span className="inline-flex items-center gap-1 font-mono">
                <span className={cn("rounded border px-1 py-0.5 text-[9px] font-bold", caLabelClass)}>
                  {fitting.player.ca}
                </span>
                <span className="text-[9px] text-[var(--muted)]">/</span>
                <span className={cn("rounded border px-1 py-0.5 text-[9px] font-bold", paTone(fitting.player.pa))}>
                  {fitting.player.pa ?? "—"}
                </span>
              </span>
              <FitBar fit={fitting.fit} title={fittingTitle(fitting, locale)} />
              <span className="w-20 truncate text-[9px] text-[var(--muted)]">
                {roleName(fitting.role_id, fitting.role_name, locale)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function SuggestionRow({
  rank,
  suggestion,
  locale,
  reasonText,
  formatValue,
  formatWage,
  onPlayer,
}: {
  rank: number;
  suggestion: ScoutSuggestion;
  locale: "en" | "zh-CN";
  reasonText: (code: string, value: number | null) => string;
  formatValue: (value: number | null | undefined) => string;
  formatWage: (value: number | null | undefined) => string;
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  const shown = suggestion.reasons.slice(0, 2);
  const more = suggestion.reasons.length - shown.length;
  const probabilityTone =
    suggestion.probability >= 70
      ? "text-[var(--success)]"
      : suggestion.probability >= 50
        ? "text-[var(--gold)]"
        : "text-[var(--danger)]";
  const reasonsTitle = suggestion.reasons
    .map((reason) => `${reasonText(reason.code, reason.value)} (${reason.points > 0 ? "+" : ""}${reason.points})`)
    .join("\n");
  return (
    <tr className="border-t border-[var(--line)] hover:bg-[var(--surface-2)]">
      <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">{rank}</td>
      <td className="max-w-[180px] px-2 py-1.5">
        <button
          type="button"
          onClick={() => onPlayer(suggestion.player.uid)}
          className="name-link block max-w-full truncate text-left font-medium text-[var(--ink)] hover:text-[var(--accent)]"
        >
          {suggestion.player.name ?? t("common.unnamedPlayer")}
        </button>
        <span className="block truncate text-[9px] text-[var(--muted)]">
          {suggestion.owner_club
            ? `${suggestion.owner_club} · ${t("scout.loanOwner", { club: suggestion.player.club_name ?? "—" })}`
            : suggestion.player.club_name ?? t("common.freeAgent")}
          {suggestion.player.transfer_listed && (
            <span className="ml-1 rounded bg-[var(--gold-soft)] px-1 text-[8px] text-[var(--gold)]">
              {t("scout.listed")}
            </span>
          )}
        </span>
      </td>
      <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
        {suggestion.player.age ?? "—"}
      </td>
      <td className="px-2 py-1.5 text-right font-mono text-[var(--ink)]">{suggestion.player.ca}</td>
      <td className="px-2 py-1.5 text-right">
        <span className={cn("inline-flex rounded border px-1 py-0.5 font-mono text-[9px] font-bold", paTone(suggestion.player.pa))}>
          {suggestion.player.pa ?? "—"}
        </span>
      </td>
      <td className="px-2 py-1.5">
        <FitBar fit={suggestion.fit} title={fittingTitle(suggestion, locale)} />
        <span className="mt-0.5 block max-w-[110px] truncate text-[8px] text-[var(--muted)]">
          {roleName(suggestion.role_id, suggestion.role_name, locale)}
        </span>
      </td>
      <td className={cn("px-2 py-1.5 text-right font-mono font-semibold", probabilityTone)} title={reasonsTitle}>
        {suggestion.probability}%
      </td>
      <td className="px-2 py-1.5 text-right">
        <span className="block font-mono text-[var(--ink)]">
          {suggestion.fee == null
            ? t("scout.unknownFee")
            : suggestion.fee === 0
              ? t("scout.freeTransfer")
              : formatValue(suggestion.fee)}
        </span>
        <span className="block font-mono text-[8px] text-[var(--muted)]">
          {suggestion.wage == null ? "—" : formatWage(suggestion.wage)}
        </span>
      </td>
      <td className="max-w-[150px] px-2 py-1.5" title={reasonsTitle}>
        <div className="flex flex-wrap gap-1">
          {shown.map((reason) => (
            <span
              key={reason.code}
              className={cn(
                "rounded px-1 py-0.5 text-[8px]",
                reason.points > 0
                  ? "bg-[var(--success-soft)] text-[var(--success)]"
                  : reason.points < 0
                    ? "bg-[var(--danger-soft)] text-[var(--danger)]"
                    : "bg-[var(--surface-3)] text-[var(--muted)]",
              )}
            >
              {reasonText(reason.code, reason.value)}
            </span>
          ))}
          {more > 0 && (
            <span className="rounded bg-[var(--surface-3)] px-1 py-0.5 text-[8px] text-[var(--muted)]">
              +{more}
            </span>
          )}
        </div>
      </td>
    </tr>
  );
}

function DepthTable({
  overview,
  locale,
  onAdvice,
  onPlayer,
}: {
  overview: ScoutOverview;
  locale: "en" | "zh-CN";
  onAdvice: (position: string) => void;
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  const flagged = overview.lines.filter(
    (line) => line.status !== "fine" || line.weak || line.ageing,
  );
  return (
    <div className="space-y-3">
      <div className="text-[10px] text-[var(--muted)]">
        {t("scout.depth.summary", {
          size: overview.squad_size,
          age: overview.average_age == null ? "—" : formatNumber(overview.average_age, locale),
          level: formatNumber(overview.level, locale),
        })}
      </div>
      <div className="text-[10px]">
        {flagged.length === 0 ? (
          <span className="text-[var(--success)]">{t("scout.depth.allFine")}</span>
        ) : (
          <span className="text-[var(--muted)]">
            <span className="font-semibold text-[var(--ink)]">{t("scout.depth.needsLook")}</span>{" "}
            {flagged
              .map((line) => t(`scout.line.${line.key}` as TranslationKey))
              .join(", ")}
          </span>
        )}
      </div>
      <div className="overflow-x-auto rounded-lg border border-[var(--line)]">
        <table className="w-full border-collapse text-[10px]">
          <thead className="bg-[var(--surface-2)] text-[var(--muted)]">
            <tr>
              <th className="px-2 py-1.5 text-left font-medium">{t("scout.depth.line")}</th>
              <th className="px-2 py-1.5 text-left font-medium">{t("scout.depth.status")}</th>
              <th className="px-2 py-1.5 text-right font-medium">{t("scout.depth.count")}</th>
              <th className="px-2 py-1.5 text-left font-medium">{t("scout.depth.players")}</th>
              <th className="px-2 py-1.5 text-left font-medium">{t("scout.depth.cover")}</th>
              <th className="px-2 py-1.5 text-right font-medium">{t("scout.depth.age")}</th>
              <th className="w-24 px-2 py-1.5" />
            </tr>
          </thead>
          <tbody>
            {overview.lines.map((line) => {
              const ages = line.players
                .map((player) => player.player.age)
                .filter((age): age is number => age != null);
              const average =
                ages.length === 0
                  ? null
                  : ages.reduce((sum, age) => sum + age, 0) / ages.length;
              return (
                <tr key={line.key} className="border-t border-[var(--line)] align-top">
                  <td className="px-2 py-2 font-medium text-[var(--ink)]">
                    {t(`scout.line.${line.key}` as TranslationKey)}
                  </td>
                  <td className="px-2 py-2">
                    <span className="inline-flex items-center gap-1.5 text-[var(--muted)]">
                      <span className={cn("h-1.5 w-1.5 rounded-full", STATUS_DOT[line.status])} />
                      {t(`scout.status.${line.status}` as TranslationKey)}
                    </span>
                    <div className="mt-0.5 flex flex-wrap gap-1">
                      {line.weak && <Tag>{t("scout.depth.weak")}</Tag>}
                      {line.ageing && <Tag>{t("scout.depth.ageing")}</Tag>}
                    </div>
                  </td>
                  <td className="px-2 py-2 text-right font-mono">
                    <span className="text-[var(--ink)]">{line.players.length}</span>
                    <span className="text-[var(--muted)]"> / {line.target}</span>
                  </td>
                  <td className="max-w-[260px] px-2 py-2">
                    {line.players.length === 0 ? (
                      <span className="text-[var(--muted)]">—</span>
                    ) : (
                      <span className="text-[var(--ink)]">
                        {line.players.slice(0, 4).map((player) => (
                          <button
                            key={player.player.uid}
                            type="button"
                            className="name-link mr-1 hover:text-[var(--accent)]"
                            onClick={() => onPlayer(player.player.uid)}
                          >
                            {player.player.name ?? "—"}
                            <span className="ml-0.5 font-mono text-[9px] text-[var(--muted)]">
                              {player.player.ca}
                            </span>
                          </button>
                        ))}
                        {line.players.length > 4 && (
                          <span className="text-[var(--muted)]">+{line.players.length - 4}</span>
                        )}
                      </span>
                    )}
                  </td>
                  <td className="max-w-[180px] px-2 py-2 text-[var(--muted)]">
                    {line.cover.length === 0
                      ? "—"
                      : line.cover
                          .slice(0, 2)
                          .map((player) => player.player.name ?? "—")
                          .join(", ") +
                        (line.cover.length > 2 ? ` +${line.cover.length - 2}` : "")}
                  </td>
                  <td className="px-2 py-2 text-right font-mono text-[var(--muted)]">
                    {average == null ? "—" : average.toFixed(1)}
                  </td>
                  <td className="px-2 py-2 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      className="whitespace-nowrap"
                      onClick={() => onAdvice(line.positions[0])}
                    >
                      {t("scout.depth.advice")}
                    </Button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Tag({ children }: { children: ReactNode }) {
  return (
    <span className="rounded bg-[var(--gold-soft)] px-1 py-0.5 text-[8px] text-[var(--gold)]">
      {children}
    </span>
  );
}

function DepartureTable({
  departures,
  locale,
  departureReasonText,
  formatValue,
  formatWage,
  onPlayer,
}: {
  departures: ScoutDeparture[];
  locale: "en" | "zh-CN";
  departureReasonText: (code: string, value: number | null) => string;
  formatValue: (value: number | null | undefined) => string;
  formatWage: (value: number | null | undefined) => string;
  onPlayer: (uid: number) => void;
}) {
  const { t } = useI18n();
  const [filter, setFilter] = useState<"all" | ScoutDepartureAction>("all");
  const shown =
    filter === "all"
      ? departures
      : departures.filter((departure) => departure.action === filter);
  const count = (value: "all" | ScoutDepartureAction) =>
    value === "all"
      ? departures.length
      : departures.filter((departure) => departure.action === value).length;
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-[10px] text-[var(--muted)]">{t("scout.departures.intro")}</div>
        <div className="flex gap-1">
          {(["all", ...DEPARTURE_ACTIONS] as const).map((value) => (
            <button
              key={value}
              type="button"
              onClick={() => setFilter(value)}
              aria-pressed={filter === value}
              className={cn(
                "focus-ring rounded-md border px-2 py-1 text-[9px] font-medium",
                filter === value
                  ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                  : "border-[var(--line)] bg-[var(--surface)] text-[var(--muted)]",
              )}
            >
              {t(`scout.departures.filter.${value}` as TranslationKey)} ({count(value)})
            </button>
          ))}
        </div>
      </div>
      {shown.length === 0 ? (
        <div className="rounded-lg border border-dashed border-[var(--line)] px-3 py-8 text-center text-[10px] text-[var(--muted)]">
          {t("scout.departures.empty")}
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-[var(--line)]">
          <table className="w-full border-collapse text-[10px]">
            <thead className="bg-[var(--surface-2)] text-[var(--muted)]">
              <tr>
                <th className="w-8 px-2 py-1.5 text-right font-medium">#</th>
                <th className="px-2 py-1.5 text-left font-medium">{t("scout.column.player")}</th>
                <th className="px-2 py-1.5 text-right font-medium">{t("scout.column.age")}</th>
                <th className="px-2 py-1.5 text-right font-medium">CA</th>
                <th className="px-2 py-1.5 text-right font-medium">PA</th>
                <th className="px-2 py-1.5 text-right font-medium">{t("scout.departures.value")}</th>
                <th className="px-2 py-1.5 text-right font-medium">{t("scout.departures.wage")}</th>
                <th className="px-2 py-1.5 text-left font-medium">{t("scout.departures.contract")}</th>
                <th className="px-2 py-1.5 text-left font-medium">{t("scout.departures.action")}</th>
                <th className="px-2 py-1.5 text-left font-medium">{t("scout.column.reasons")}</th>
              </tr>
            </thead>
            <tbody>
              {shown.map((departure, index) => (
                <tr key={departure.player.uid} className="border-t border-[var(--line)] hover:bg-[var(--surface-2)]">
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">{index + 1}</td>
                  <td className="px-2 py-1.5">
                    <button
                      type="button"
                      className="name-link block max-w-[200px] truncate text-left font-medium text-[var(--ink)] hover:text-[var(--accent)]"
                      onClick={() => onPlayer(departure.player.uid)}
                    >
                      {departure.player.name ?? t("common.unnamedPlayer")}
                    </button>
                    <span className="block text-[9px] text-[var(--muted)]">
                      {t(`scout.line.${departure.line}` as TranslationKey)} · #{departure.rank}
                    </span>
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                    {departure.player.age ?? "—"}
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono">{departure.player.ca}</td>
                  <td className="px-2 py-1.5 text-right">
                    <span className={cn("inline-flex rounded border px-1 py-0.5 font-mono text-[9px] font-bold", paTone(departure.player.pa))}>
                      {departure.player.pa ?? "—"}
                    </span>
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono">
                    {departure.not_for_sale
                      ? t("scout.notForSale")
                      : departure.value == null
                        ? "—"
                        : formatValue(departure.value)}
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono text-[var(--muted)]">
                    {departure.wage == null ? "—" : formatWage(departure.wage)}
                  </td>
                  <td className="px-2 py-1.5 text-[var(--muted)]">
                    {departure.player.contract_end
                      ? formatDate(departure.player.contract_end)
                      : "—"}
                  </td>
                  <td className="px-2 py-1.5">
                    <span
                      className={cn(
                        "rounded px-1.5 py-0.5 text-[9px] font-medium",
                        departure.action === "sell"
                          ? "bg-[var(--danger-soft)] text-[var(--danger)]"
                          : departure.action === "loan"
                            ? "bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                            : "bg-[var(--surface-3)] text-[var(--muted)]",
                      )}
                    >
                      {t(`scout.departures.filter.${departure.action}` as TranslationKey)}
                    </span>
                  </td>
                  <td className="px-2 py-1.5">
                    <div className="flex max-w-[240px] flex-wrap gap-1">
                      {departure.reasons.map((reason) => (
                        <span
                          key={reason.code}
                          className="rounded bg-[var(--surface-3)] px-1 py-0.5 text-[8px] text-[var(--muted)]"
                        >
                          {departureReasonText(reason.code, reason.value)}
                        </span>
                      ))}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
