import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  BriefcaseBusiness,
  CalendarDays,
  Check,
  ChevronRight,
  CircleDot,
  ClipboardList,
  Coins,
  Copy,
  GitCompareArrows,
  HeartPulse,
  Maximize2,
  Pencil,
  Ruler,
  Shield,
  ShieldAlert,
  Sparkles,
  TrendingUp,
  UserRound,
  X,
} from "lucide-react";
import { useGrowthHistory, usePlayerDetail, usePlayerRoles } from "@/lib/query";
import { copyTextToClipboard } from "@/lib/api";
import type { Attributes, Player, Positions, RoleScore, SnapshotPoint } from "@/lib/types";
import { ATTRIBUTE_GROUPS, POSITION_LABELS } from "@/lib/types";
import {
  attributeLabel,
  clauseKindLabel,
  clauseValueLabel,
  nationLabel,
  personalityLabel,
  roleGroup,
  roleName,
  severityLabel,
  traitLabel,
} from "@/lib/labels";
import { useI18n } from "@/lib/useI18n";
import {
  attributeTone,
  caTone,
  cn,
  formatDate,
  formatValue,
  formatWage,
  initials,
} from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { AttributeRadar } from "./AttributeRadar";
import { Badge, Button, EmptyState, Switch } from "./ui";

type Tab = "attributes" | "roles" | "positions" | "growth" | "hidden" | "contract";

export function Inspector() {
  const { t, locale } = useI18n();
  const selectedUid = useAppStore((state) => state.selectedUid);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const setEditOpen = useAppStore((state) => state.setEditOpen);
  const pendingEdits = useAppStore((state) => state.pendingEdits);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const pushToast = useAppStore((state) => state.pushToast);
  const gameDate = useAppStore((state) => state.summary?.game_date ?? null);
  const displayMode = useAppStore((state) => state.displayMode);
  const setDisplayMode = useAppStore((state) => state.setDisplayMode);
  const compareUids = useAppStore((state) => state.compareUids);
  const toggleCompare = useAppStore((state) => state.toggleCompare);
  const { data: player, isLoading } = usePlayerDetail(selectedUid);
  const { data: roleScores } = usePlayerRoles(selectedUid);
  const { data: growth } = useGrowthHistory(selectedUid);
  const [tab, setTab] = useState<Tab>("attributes");
  const [nameCopied, setNameCopied] = useState(false);
  const tabBarRef = useRef<HTMLDivElement>(null);
  const [tabBarWidth, setTabBarWidth] = useState(0);

  useEffect(() => {
    if (!hiddenEnabled && tab === "hidden") setTab("attributes");
  }, [hiddenEnabled, tab]);

  useLayoutEffect(() => {
    const element = tabBarRef.current;
    if (!element) return;
    const styles = window.getComputedStyle(element);
    const gap = Number.parseFloat(styles.columnGap) || 0;
    const padding =
      (Number.parseFloat(styles.paddingLeft) || 0) +
      (Number.parseFloat(styles.paddingRight) || 0);
    const buttons = Array.from(element.children) as HTMLElement[];
    const width = Math.ceil(
      buttons.reduce((sum, button) => sum + button.getBoundingClientRect().width, 0) +
        gap * Math.max(0, buttons.length - 1) +
        padding,
    );
    setTabBarWidth((current) => (current === width ? current : width));
  }, [hiddenEnabled, locale, selectedUid, isLoading]);

  useEffect(() => {
    if (!nameCopied) return;
    const timer = window.setTimeout(() => setNameCopied(false), 1600);
    return () => window.clearTimeout(timer);
  }, [nameCopied]);

  if (selectedUid == null) {
    return (
      <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]">
        <EmptyState
          icon={<UserRound size={20} />}
          title={t("inspector.emptyTitle")}
          description={t("inspector.emptyDescription")}
        />
      </aside>
    );
  }

  if (isLoading || !player) {
    return (
      <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)] p-4">
        <div className="h-24 animate-pulse rounded-xl bg-[var(--surface-2)]" />
        <div className="mt-3 h-8 w-2/3 animate-pulse rounded bg-[var(--surface-2)]" />
        <div className="mt-6 space-y-2">
          {Array.from({ length: 8 }).map((_, index) => (
            <div key={index} className="h-7 animate-pulse rounded bg-[var(--surface-2)]" />
          ))}
        </div>
      </aside>
    );
  }

  const attributes = displayMode === "display" ? player.attributes : player.raw_attributes;
  const maxValue = displayMode === "display" ? 20 : 100;
  const hiddenTab: Array<[Tab, string]> = hiddenEnabled
    ? [["hidden", t("inspector.tab.hidden")]]
    : [];
  const tabs: Array<[Tab, string]> = [
    ["attributes", t("inspector.tab.attributes")],
    ["roles", t("inspector.tab.roles")],
    ["positions", t("inspector.tab.positions")],
    ["growth", t("inspector.tab.growth")],
    ...hiddenTab,
    ["contract", t("inspector.tab.contract")],
  ];

  const copyName = async () => {
    const name = player.name;
    if (!name) return;
    try {
      await copyTextToClipboard(name);
      setNameCopied(true);
      pushToast({ title: t("toast.nameCopied"), description: name, tone: "success" });
    } catch (error) {
      pushToast({ title: t("toast.copyFailed"), description: String(error), tone: "error" });
    }
  };

  return (
    <aside
      className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]"
      style={tabBarWidth > 0 ? { minWidth: tabBarWidth } : undefined}
    >
      <div className="flex items-center justify-between border-b border-[var(--line)] px-3 py-2">
        <div className="flex items-center gap-2 text-[11px] font-semibold text-[var(--muted)]">
          <ClipboardList size={13} className="text-[var(--accent)]" />
          {t("inspector.profile")}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5 text-[10px]"
            onClick={() => setEditOpen(true)}
          >
            <Pencil size={11} />
            {t("inspector.edit")}
          </Button>
          <Button
            variant={compareUids.includes(player.uid) ? "gold" : "ghost"}
            size="sm"
            className="h-6 px-1.5 text-[10px]"
            onClick={() => toggleCompare(player.uid)}
          >
            {compareUids.includes(player.uid) ? <Check size={11} /> : <GitCompareArrows size={11} />}
            {t("inspector.compare")}
          </Button>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => setSelectedUid(null)} aria-label={t("inspector.closeProfile")}>
            <X size={14} />
          </Button>
        </div>
      </div>

      <div className="thin-scroll min-h-0 flex-1 overflow-y-auto">
        <div className="border-b border-[var(--line)] px-4 pb-4 pt-3">
          <div className="flex items-start gap-3">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-[var(--line)] bg-[var(--surface-2)] text-sm font-bold text-[var(--muted)]">
              {initials(player.name)}
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="flex min-w-0 items-start gap-1">
                    <h2 className="min-w-0 text-[17px] font-semibold leading-tight tracking-tight text-[var(--ink)]">
                      {player.name ?? t("common.unnamedPlayer")}
                    </h2>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="mt-px h-6 w-6 shrink-0"
                      onClick={() => void copyName()}
                      disabled={!player.name}
                      aria-label={t("inspector.copyName")}
                      title={t("inspector.copyName")}
                    >
                      {nameCopied ? (
                        <Check size={12} className="text-[var(--success)]" />
                      ) : (
                        <Copy size={12} />
                      )}
                    </Button>
                  </div>
                  {player.club_uid != null ? (
                    <button
                      type="button"
                      className="mt-0.5 block max-w-full truncate text-left text-[11px] text-[var(--muted)] hover:text-[var(--accent-strong)] hover:underline"
                      onClick={() => {
                        setSelectedClubUid(player.club_uid);
                        setViewMode("clubs");
                      }}
                    >
                      {player.club_name ?? t("common.freeAgent")}
                    </button>
                  ) : (
                    <div className="mt-0.5 truncate text-[11px] text-[var(--muted)]">
                      {player.club_name ?? t("common.freeAgent")}
                    </div>
                  )}
                  {player.league_name && (
                    <div className="mt-0.5 truncate text-[10px] text-[var(--muted)]">
                      {player.league_name}
                    </div>
                  )}
                  <div className="mt-0.5 flex items-center gap-1.5 text-[11px] text-[var(--muted)]">
                    <span>{t("common.yearsOld", { age: player.age ?? "?" })}</span>
                    <span className="text-[var(--line-strong)]">·</span>
                    <span>
                      {nationLabel(player.nation_name, locale) ??
                        (player.nation_id != null
                          ? t("common.nationNumber", { id: player.nation_id })
                          : t("common.unknownNation"))}
                    </span>
                    {player.gender === 1 && (
                      <Badge tone="blue">{t("inspector.womenBadge")}</Badge>
                    )}
                    {pendingEdits[player.uid] && (
                      <Badge tone="gold">{t("pending.badge")}</Badge>
                    )}
                  </div>
                </div>
                <div className="flex shrink-0 flex-col items-end gap-1">
                  <div className="flex gap-1">
                    <span className={cn("flex h-8 min-w-10 flex-col items-center justify-center rounded-lg border px-1.5", caTone(player.ca))}>
                      <span className="text-[8px] font-bold uppercase opacity-60">CA</span>
                      <span className="font-mono text-[12px] font-bold leading-none">{player.ca}</span>
                    </span>
                    <span className="flex h-8 min-w-10 flex-col items-center justify-center rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-1.5 text-[var(--muted)]">
                      <span className="text-[8px] font-bold uppercase opacity-60">PA</span>
                      <span className="font-mono text-[12px] font-bold leading-none">
                        {player.pa ?? (player.pa_range_code != null ? "?" : "—")}
                      </span>
                    </span>
                  </div>
                  {player.generated && (
                    <Badge tone="gold" className="px-1.5 py-0 text-[9px]">
                      {t("inspector.generatedBadge")}
                    </Badge>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className="mt-3 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-[10px] text-[var(--muted)]">
            <span className="inline-flex items-center gap-1">
              <Shield size={11} />
              {player.natural_positions.join(" / ") || "—"}
            </span>
            <span className="inline-flex items-center gap-1">
              <Ruler size={11} />
              {player.height_cm ? `${player.height_cm} cm` : "—"}
            </span>
            <span className="inline-flex items-center gap-1">
              <CalendarDays size={11} />
              {t("inspector.contractUntil", { date: player.contract?.end ?? "—" })}
              {player.transfer_listed && (
                <Badge
                  tone="gold"
                  title={t("inspector.transferListed")}
                  className="px-1.5 py-0 text-[9px]"
                >
                  {t("inspector.transferListedShort")}
                </Badge>
              )}
              {player.loan_listed && (
                <Badge
                  tone="accent"
                  title={t("inspector.loanListed")}
                  className="px-1.5 py-0 text-[9px]"
                >
                  {t("inspector.loanListedShort")}
                </Badge>
              )}
            </span>
          </div>

          <FootCard left={player.left_foot} right={player.right_foot} />

          <div className="mt-3 grid grid-cols-3 gap-1.5">
            <MiniStat
              icon={<Coins size={11} />}
              label={t("inspector.value")}
              value={formatValue(player.transfer_value)}
            />
            <MiniStat
              icon={<BriefcaseBusiness size={11} />}
              label={t("inspector.wage")}
              value={formatWage(player.contract?.wage, locale)}
            />
            <MiniStat
              icon={<HeartPulse size={11} />}
              label={t("inspector.condition")}
              value={`${Math.round(player.raw_condition / 100)}% / ${Math.round(player.raw_match_sharpness / 100)}%`}
            />
          </div>

          {player.on_loan && (
            <div className="mt-2 flex items-center gap-1.5 rounded-[7px] border border-[var(--blue-line)] bg-[var(--blue-soft)] px-2 py-1.5 text-[10px] text-[var(--blue)]">
              <span className="font-semibold">{t("inspector.onLoan")}</span>
              <span>
                {t("inspector.fromClub", {
                  club: player.loan_parent_club_name ?? t("common.unknownClub"),
                })}
              </span>
              <span className="ml-auto font-mono">{player.loan_end ?? ""}</span>
            </div>
          )}
          {player.injuries.some((injury) => injury.kind === "current") && (
            <div className="mt-2 flex items-center gap-1.5 rounded-[7px] border border-[var(--danger-line)] bg-[var(--danger-soft)]/60 px-2 py-1.5 text-[10px] text-[var(--danger)]">
              <HeartPulse size={11} />
              <span className="font-semibold">{t("inspector.injured")}</span>
              <span className="ml-auto font-mono">
                {player.injuries.find((injury) => injury.kind === "current")?.expected_return ?? ""}
              </span>
            </div>
          )}
          {player.suspensions.length > 0 && (
            <div className="mt-2 flex items-center gap-1.5 rounded-[7px] border border-[var(--gold-line)] bg-[var(--gold-soft)] px-2 py-1.5 text-[10px] text-[var(--gold)]">
              <ShieldAlert size={11} />
              <span className="font-semibold">
                {t(
                  player.suspensions.length === 1
                    ? "inspector.suspensionOne"
                    : "inspector.suspensions",
                  { count: player.suspensions.length },
                )}
              </span>
              <span className="ml-auto font-mono">
                {t("inspector.latest", {
                  date:
                    [...player.suspensions].sort((left, right) =>
                      (right.issued_date ?? "").localeCompare(left.issued_date ?? ""),
                    )[0]?.issued_date ?? "",
                })}
              </span>
            </div>
          )}

          <div className="mt-3 flex items-center justify-between rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5">
            <span className="text-[10px] text-[var(--muted)]">{t("inspector.displayMode")}</span>
            <Switch
              checked={displayMode === "raw"}
              onChange={(raw) => setDisplayMode(raw ? "raw" : "display")}
              label={t("inspector.rawValues")}
            />
          </div>
        </div>

        {player.traits.length > 0 && (
          <div className="border-b border-[var(--line)] px-4 py-3">
            <div className="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
              <Sparkles size={11} />
              {t("inspector.traits")}
            </div>
            <div className="flex flex-wrap gap-1">
              {player.traits.map((trait) => (
                <Badge key={trait.bit} tone="outline">
                  {traitLabel(trait.bit, trait.name, locale)}
                </Badge>
              ))}
            </div>
          </div>
        )}

        <div
          ref={tabBarRef}
          className="sticky top-0 z-10 flex gap-0.5 overflow-hidden border-b border-[var(--line)] bg-[var(--surface)] px-2 py-1.5"
        >
          {tabs.map(([key, label]) => (
            <button
              key={key}
              type="button"
              onClick={() => setTab(key)}
              className={cn(
                "focus-ring shrink-0 whitespace-nowrap rounded-[6px] px-2 py-1 text-[10px] font-medium transition-colors",
                tab === key
                  ? "bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                  : "text-[var(--muted)] hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
              )}
            >
              {label}
            </button>
          ))}
        </div>

        {tab === "attributes" && (
          <AttributesPanel player={player} attributes={attributes} max={maxValue} />
        )}
        {tab === "roles" && (
          <RolesPanel roles={roleScores ?? []} bestId={player.best_role?.id ?? null} />
        )}
        {tab === "growth" && (
          <GrowthPanel
            key={selectedUid}
            points={growth ?? []}
            gameDate={gameDate}
          />
        )}
        {tab === "hidden" && (
          <HiddenPanel player={player} attributes={attributes} max={maxValue} />
        )}
        {tab === "positions" && <PositionsPanel positions={player.positions} />}
        {tab === "contract" && <ContractPanel player={player} />}
      </div>
    </aside>
  );
}

function MiniStat({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5">
      <div className="flex items-center gap-1 text-[9px] text-[var(--muted)]">
        {icon}
        {label}
      </div>
      <div className="mt-0.5 truncate font-mono text-[11px] font-semibold text-[var(--ink)]">{value}</div>
    </div>
  );
}

function FootCard({ left, right }: { left: number; right: number }) {
  const { t } = useI18n();
  const difference = left - right;
  const balanced = Math.abs(difference) <= 2;
  const leftFooted = !balanced && left > right;
  const summary = balanced
    ? t("inspector.footBoth")
    : leftFooted
      ? t("inspector.footLeftFooted")
      : t("inspector.footRightFooted");
  return (
    <div
      className="mt-3 rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] px-2.5 py-2"
      title={t("inspector.footHint")}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="inline-flex items-center gap-1 text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          <CircleDot size={10} />
          {t("inspector.foot")}
        </span>
        <span className="rounded-[4px] bg-[var(--accent-soft)] px-1.5 py-0.5 text-[9px] font-semibold text-[var(--accent-strong)]">
          {summary}
        </span>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[10px] text-[var(--muted)]">
        {balanced ? (
          <span>{t("inspector.footBalancedDetail", { left, right })}</span>
        ) : (
          <>
            <span>{t("inspector.footPreferred")}</span>
            <span className="font-semibold text-[var(--ink)]">
              {leftFooted ? t("inspector.footLeft") : t("inspector.footRight")}{" "}
              <span className="font-mono">{Math.max(left, right)}</span>
            </span>
            <span className="text-[var(--line-strong)]">·</span>
            <span>{t("inspector.footWeak")}</span>
            <span className="font-medium text-[var(--ink)]">
              {leftFooted ? t("inspector.footRight") : t("inspector.footLeft")}{" "}
              <span className="font-mono">{Math.min(left, right)}</span>
            </span>
          </>
        )}
      </div>
    </div>
  );
}

function AttributesPanel({
  player,
  attributes,
  max,
}: {
  player: Player;
  attributes: Attributes;
  max: number;
}) {
  const { t, locale } = useI18n();
  const isGoalkeeper = player.natural_positions.includes("GK") || player.positions.gk >= 15;
  const groups = isGoalkeeper
    ? [...ATTRIBUTE_GROUPS.technical.slice(0, 2), ...ATTRIBUTE_GROUPS.goalkeeping, ...ATTRIBUTE_GROUPS.technical.slice(2)]
    : ATTRIBUTE_GROUPS.technical;
  const radarKeys = isGoalkeeper
    ? ["handling", "reflexes", "aerial_reach", "command_of_area", "kicking", "one_on_ones"]
    : ["passing", "dribbling", "finishing", "tackling", "vision", "pace"];
  return (
    <div className="space-y-4 px-4 py-3">
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)]">
        <div className="border-b border-[var(--line)] px-3 py-1.5 text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          {t("inspector.radar")}
        </div>
        <AttributeRadar values={attributes} keys={radarKeys} max={max} />
      </div>
      {groups.map((group) => (
        <div key={group.labelKey}>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t(`attribute.group.${group.labelKey}`)}
          </div>
          <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
            {group.keys.map((key) => (
              <AttributeRow
                key={key}
                label={attributeLabel(key, locale)}
                value={attributes[key] ?? 0}
                max={max}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function AttributeRow({ label, value, max }: { label: string; value: number; max: number }) {
  const tone = attributeTone(max === 20 ? value : Math.round(value / 5));
  return (
    <div className="flex items-center gap-1.5">
      <span className="w-[68px] shrink-0 truncate text-[10px] text-[var(--muted)]">{label}</span>
      <div className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--surface-3)]">
        <div
          className={cn("h-full rounded-full", tone.split(" ")[0])}
          style={{ width: `${Math.max(4, Math.min(100, (value / max) * 100))}%` }}
        />
      </div>
      <span className="w-6 text-right font-mono text-[10px] font-semibold text-[var(--ink)]">{value}</span>
    </div>
  );
}

function HiddenPanel({
  player,
  attributes,
  max,
}: {
  player: Player;
  attributes: Attributes;
  max: number;
}) {
  const { t, locale } = useI18n();
  const hidden = attributes;
  return (
    <div className="space-y-4 px-4 py-3">
      <div>
        <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
          {t("inspector.hiddenAttributes")}
        </div>
        <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
          {ATTRIBUTE_GROUPS.hidden[0].keys.map((key) => (
            <AttributeRow
              key={key}
              label={attributeLabel(key, locale)}
              value={hidden[key] ?? 0}
              max={max}
            />
          ))}
        </div>
      </div>
      {player.personality && (
        <div>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("inspector.personality")}
          </div>
          <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
            {Object.entries(player.personality).map(([key, value]) => (
              <AttributeRow
                key={key}
                label={personalityLabel(key, locale)}
                value={value}
                max={20}
              />
            ))}
          </div>
        </div>
      )}
      <div className="rounded-lg border border-[var(--danger-line)] bg-[var(--danger-soft)]/50 p-2 text-[10px] leading-relaxed text-[var(--danger)]">
        {t("inspector.hiddenWarning")}
      </div>
    </div>
  );
}

function RolesPanel({ roles, bestId }: { roles: RoleScore[]; bestId: string | null }) {
  const { t, locale } = useI18n();
  if (roles.length === 0) {
    return (
      <div className="px-4 py-6 text-center text-[11px] text-[var(--muted)]">
        {t("inspector.rolesLoading")}
      </div>
    );
  }
  const groups = [...new Set(roles.map((role) => role.group))];
  return (
    <div className="space-y-4 px-4 py-3">
      <p className="text-[10px] leading-relaxed text-[var(--muted)]">
        {t("inspector.rolesDescription")}
      </p>
      {groups.map((group) => (
        <div key={group}>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {roleGroup(group, locale)}
          </div>
          <div className="space-y-1.5">
            {roles
              .filter((role) => role.group === group)
              .sort((left, right) => right.score - left.score)
              .map((role) => {
                const best = role.id === bestId;
                return (
                  <div
                    key={role.id}
                    className={cn(
                      "rounded-[7px] border px-2 py-1.5",
                      best
                        ? "border-[var(--accent-line)] bg-[var(--accent-soft)]"
                        : "border-[var(--line)] bg-[var(--surface-2)]",
                    )}
                  >
                    <div className="flex items-center justify-between gap-2 text-[10px]">
                      <span className={cn("truncate", best ? "font-semibold text-[var(--accent-strong)]" : "text-[var(--ink)]")}>
                        {roleName(role.id, role.name, locale)}
                      </span>
                      <span className="shrink-0 font-mono text-[9px] text-[var(--muted)]">
                        {role.position} {role.position_rating}
                      </span>
                    </div>
                    <div className="mt-1 flex items-center gap-1.5">
                      <div className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--surface-3)]">
                        <div
                          className={cn("h-full rounded-full", attributeTone(role.score).split(" ")[0])}
                          style={{ width: `${Math.max(4, (role.score / 20) * 100)}%` }}
                        />
                      </div>
                      <span className="w-6 text-right font-mono text-[10px] font-semibold text-[var(--ink)]">
                        {role.score}
                      </span>
                    </div>
                  </div>
                );
              })}
          </div>
        </div>
      ))}
    </div>
  );
}

function GrowthPanel({ points, gameDate }: { points: SnapshotPoint[]; gameDate: string | null }) {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);
  if (points.length < 2) {
    return (
      <div className="flex flex-col items-center gap-2 px-6 py-10 text-center">
        <TrendingUp size={20} className="text-[var(--muted)]" />
        <div className="text-xs font-semibold text-[var(--ink)]">{t("inspector.noGrowth")}</div>
        <p className="max-w-[250px] text-[11px] leading-relaxed text-[var(--muted)]">
          {t("inspector.noGrowthDescription")}
          {gameDate ? t("inspector.currentSnapshot", { date: gameDate }) : ""}
        </p>
      </div>
    );
  }
  const first = points[0];
  const last = points[points.length - 1];
  const caDelta = (last.ca ?? 0) - (first.ca ?? 0);
  return (
    <div className="space-y-3 px-4 py-3">
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2">
        <div className="mb-1 flex items-center justify-between px-1 text-[9px] text-[var(--muted)]">
          <span>{t("inspector.growthTitle")}</span>
          <span className={caDelta >= 0 ? "text-[var(--success)]" : "text-[var(--danger)]"}>
            CA {caDelta >= 0 ? "+" : ""}
            {caDelta}
          </span>
        </div>
        <button
          type="button"
          onClick={() => setExpanded(true)}
          className="focus-ring group relative block w-full cursor-zoom-in rounded-md"
          aria-label={t("inspector.growthExpand")}
          title={t("inspector.growthExpand")}
        >
          <GrowthChart points={points} />
          <span className="pointer-events-none absolute inset-0 flex items-center justify-center rounded-md bg-black/0 opacity-0 transition-opacity group-hover:bg-black/5 group-hover:opacity-100 dark:group-hover:bg-black/20">
            <span className="flex h-7 w-7 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface)]/95 text-[var(--ink)] shadow-sm">
              <Maximize2 size={13} />
            </span>
          </span>
        </button>
        <div className="flex items-center gap-3 px-1 text-[9px] text-[var(--muted)]">
          <span className="inline-flex items-center gap-1"><span className="h-0.5 w-3 bg-[var(--success)]" /> CA</span>
          <span className="inline-flex items-center gap-1"><span className="h-0.5 w-3 bg-[var(--gold)]" /> PA</span>
          <span className="ml-auto">{first.game_date} → {last.game_date}</span>
        </div>
      </div>
      <div className="space-y-1">
        { [...points].reverse().map((point) => (
          <div key={point.game_date} className="flex items-center justify-between rounded-[6px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5 text-[10px]">
            <div className="min-w-0">
              <div className="font-mono text-[var(--ink)]">{point.game_date}</div>
              <div className="truncate text-[9px] text-[var(--muted)]">
                {point.club_name ?? t("common.freeAgent")}
              </div>
            </div>
            <div className="flex items-center gap-3 font-mono">
              <span className="text-[var(--success)]">CA {point.ca ?? "—"}</span>
              <span className="text-[var(--gold)]">PA {point.pa ?? "—"}</span>
              <span className="text-[var(--muted)]">{formatValue(point.transfer_value)}</span>
            </div>
          </div>
        ))}
      </div>
      <p className="text-[9px] leading-relaxed text-[var(--muted)]">
        {t("inspector.growthNote")}
      </p>
      {expanded && <GrowthChartDialog points={points} onClose={() => setExpanded(false)} />}
    </div>
  );
}

const GROWTH_CHART_MAX = 200;

function growthPath(
  points: SnapshotPoint[],
  key: "ca" | "pa",
  x: (index: number) => number,
  y: (value: number) => number,
) {
  return points
    .map((point, index) => {
      const value = point[key];
      return value == null ? null : `${index === 0 ? "M" : "L"}${x(index).toFixed(1)},${y(value).toFixed(1)}`;
    })
    .filter(Boolean)
    .join(" ");
}

function GrowthChart({ points, expanded = false }: { points: SnapshotPoint[]; expanded?: boolean }) {
  const { t } = useI18n();
  const width = expanded ? 900 : 320;
  const height = expanded ? 360 : 132;
  const padLeft = expanded ? 44 : 8;
  const padRight = expanded ? 20 : 8;
  const padTop = expanded ? 14 : 8;
  const padBottom = expanded ? 32 : 8;
  const plotWidth = width - padLeft - padRight;
  const plotHeight = height - padTop - padBottom;
  const x = (index: number) => padLeft + (index / Math.max(1, points.length - 1)) * plotWidth;
  const knownValues = points
    .flatMap((point) => [point.ca, point.pa])
    .filter((value): value is number => value != null);
  const initialCa = points.find((point) => point.ca != null)?.ca ?? null;
  const currentPa = [...points].reverse().find((point) => point.pa != null)?.pa ?? null;
  const zoomed = expanded && knownValues.length > 0;
  const domainMin = zoomed
    ? Math.min(initialCa ?? Math.min(...knownValues), ...knownValues)
    : 0;
  const domainMax = zoomed
    ? Math.max(currentPa ?? Math.max(...knownValues), ...knownValues, domainMin + 1)
    : GROWTH_CHART_MAX;
  const y = (value: number) =>
    padTop + plotHeight - ((value - domainMin) / (domainMax - domainMin)) * plotHeight;
  const gridLines = zoomed
    ? Array.from({ length: 5 }, (_, index) => domainMin + ((domainMax - domainMin) * index) / 4)
    : [100, 140, 180];
  const labelStep = Math.max(1, Math.ceil((points.length - 1) / 6));
  const dateLabels = expanded
    ? points
        .map((_, index) => index)
        .filter((index) => index === 0 || index === points.length - 1 || index % labelStep === 0)
    : [];

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="w-full"
      role="img"
      aria-label={t("inspector.growthAria")}
    >
      {gridLines.map((line) => (
        <g key={line}>
          <line
            x1={padLeft}
            x2={width - padRight}
            y1={y(line)}
            y2={y(line)}
            stroke="var(--line)"
            strokeWidth={expanded ? 0.75 : 0.5}
            strokeDasharray="3 3"
          />
          <text
            x={expanded ? padLeft - 7 : width - padRight}
            y={expanded ? y(line) : y(line) - 2}
            textAnchor="end"
            dominantBaseline={expanded ? "middle" : undefined}
            fontSize={expanded ? 10 : 7}
            fill="var(--muted)"
          >
            {Math.round(line)}
          </text>
        </g>
      ))}
      <path
        d={growthPath(points, "pa", x, y)}
        fill="none"
        stroke="var(--gold)"
        strokeWidth={expanded ? 2.5 : 1.5}
        strokeDasharray="3 2"
      />
      <path
        d={growthPath(points, "ca", x, y)}
        fill="none"
        stroke="var(--success)"
        strokeWidth={expanded ? 3 : 2}
      />
      {points.map((point, index) =>
        point.ca == null ? null : (
          <g key={point.game_date}>
            <circle cx={x(index)} cy={y(point.ca)} r={expanded ? 3.5 : 2} fill="var(--success)" />
            {expanded && <title>{`${point.game_date} · CA ${point.ca}`}</title>}
          </g>
        ),
      )}
      {dateLabels.map((index) => (
        <text
          key={`date-${points[index].game_date}`}
          x={x(index)}
          y={height - 10}
          textAnchor={index === 0 ? "start" : index === points.length - 1 ? "end" : "middle"}
          fontSize="9"
          fill="var(--muted)"
        >
          {points[index].game_date}
        </text>
      ))}
    </svg>
  );
}

function GrowthChartDialog({ points, onClose }: { points: SnapshotPoint[]; onClose: () => void }) {
  const { t } = useI18n();
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopImmediatePropagation();
      onClose();
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [onClose]);

  const first = points[0];
  const last = points[points.length - 1];
  const caDelta = (last.ca ?? 0) - (first.ca ?? 0);
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label={t("common.close")}
        className="absolute inset-0 cursor-default bg-black/35 backdrop-blur-[2px] dark:bg-black/55"
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("inspector.growthAria")}
        className="relative flex max-h-[88vh] w-full max-w-[1040px] flex-col overflow-hidden rounded-2xl border border-[var(--line)] bg-[var(--surface)] shadow-[var(--shadow-pop)]"
      >
        <div className="flex items-start justify-between gap-4 border-b border-[var(--line)] px-5 py-4">
          <div>
            <div className="text-[15px] font-semibold tracking-tight text-[var(--ink)]">
              {t("inspector.growthTitle")}
            </div>
            <div className="mt-0.5 text-xs text-[var(--muted)]">
              {first.game_date} → {last.game_date} ·{" "}
              {t("inspector.growthSnapshots", { count: points.length })}
            </div>
          </div>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label={t("common.close")}>
            <X size={16} />
          </Button>
        </div>
        <div className="thin-scroll min-h-0 flex-1 overflow-auto p-5">
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <div className="mb-2 flex items-center gap-3 px-1 text-[10px] text-[var(--muted)]">
              <span className="inline-flex items-center gap-1.5">
                <span className="h-0.5 w-4 bg-[var(--success)]" /> CA
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-0.5 w-4 bg-[var(--gold)]" /> PA
              </span>
              <span
                className={cn(
                  "ml-auto font-mono text-[11px] font-semibold",
                  caDelta >= 0 ? "text-[var(--success)]" : "text-[var(--danger)]",
                )}
              >
                CA {caDelta >= 0 ? "+" : ""}
                {caDelta}
              </span>
            </div>
            <GrowthChart points={points} expanded />
          </div>
          <p className="mt-3 px-1 text-[10px] leading-relaxed text-[var(--muted)]">
            {t("inspector.growthNote")}
          </p>
        </div>
      </div>
    </div>,
    document.body,
  );
}

function PositionsPanel({ positions }: { positions: Positions }) {
  const { t } = useI18n();
  return (
    <div className="px-4 py-3">
      <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
        {t("inspector.positionRatings")}
      </div>
      <div className="grid grid-cols-3 gap-1.5">
        {POSITION_ROWS.flat().map((position, index) =>
          position ? (
            <div
              key={position}
              className={cn(
                "flex flex-col items-center justify-center rounded-lg border py-2",
                (positions[position as keyof Positions] ?? 0) >= 18
                  ? "border-[var(--accent-line)] bg-[var(--accent-soft)]"
                  : "border-[var(--line)] bg-[var(--surface-2)]",
              )}
            >
              <span className="font-mono text-[9px] font-bold text-[var(--muted)]">
                {POSITION_LABELS[position]}
              </span>
              <span
                className={cn(
                  "font-mono text-[14px] font-bold",
                  (positions[position as keyof Positions] ?? 0) >= 18
                    ? "text-[var(--accent-strong)]"
                    : "text-[var(--muted)]",
                )}
              >
                {positions[position as keyof Positions] ?? 1}
              </span>
            </div>
          ) : (
            <div key={`blank-${index}`} />
          ),
        )}
      </div>
      <div className="mt-2 flex items-center gap-2 text-[10px] text-[var(--muted)]">
        <span className="inline-block h-2 w-2 rounded-full bg-[var(--accent)]" />{" "}
        {t("inspector.natural")}
        <span className="ml-2 inline-block h-2 w-2 rounded-full bg-[var(--line-strong)]" />{" "}
        {t("inspector.accomplished")}
      </div>
    </div>
  );
}

function ContractPanel({ player }: { player: Player }) {
  const { t, locale } = useI18n();
  const contract = player.contract;
  return (
    <div className="space-y-3 px-4 py-3">
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[10px] uppercase tracking-wider text-[var(--muted)]">
              {t("inspector.currentContract")}
            </div>
            <div className="mt-0.5 text-sm font-semibold text-[var(--ink)]">
              {contract?.club_name ?? t("inspector.noContract")}
            </div>
          </div>
          <div className="flex flex-col items-end gap-1">
            {contract?.on_loan && <Badge tone="blue">{t("inspector.loanBadge")}</Badge>}
            {contract?.transfer_listed && (
              <Badge tone="gold" title={t("inspector.transferListed")}>
                {t("inspector.transferListedShort")}
              </Badge>
            )}
            {contract?.loan_listed && (
              <Badge tone="accent" title={t("inspector.loanListed")}>
                {t("inspector.loanListedShort")}
              </Badge>
            )}
          </div>
        </div>
        <div className="mt-3 grid grid-cols-2 gap-2">
          <MiniStat
            icon={<Coins size={11} />}
            label={t("inspector.wage")}
            value={formatWage(contract?.wage, locale)}
          />
          <MiniStat
            icon={<CalendarDays size={11} />}
            label={t("inspector.expiry")}
            value={formatDate(contract?.end)}
          />
        </div>
        <div className="mt-2 grid grid-cols-2 gap-2 text-[10px]">
          <div className="rounded-[6px] bg-[var(--surface)] px-2 py-1.5">
            <div className="text-[var(--muted)]">{t("inspector.start")}</div>
            <div className="font-mono text-[var(--ink)]">{formatDate(contract?.start)}</div>
          </div>
          <div className="rounded-[6px] bg-[var(--surface)] px-2 py-1.5">
            <div className="text-[var(--muted)]">{t("inspector.joinedClub")}</div>
            <div className="font-mono text-[var(--ink)]">{formatDate(player.club_join_date)}</div>
          </div>
        </div>
      </div>

      {player.injuries.length > 0 && (
        <div>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("inspector.injuryHistory")}
          </div>
          <div className="space-y-1">
            {[...player.injuries]
              .sort((left, right) =>
                (right.date ?? right.expected_return ?? "").localeCompare(
                  left.date ?? left.expected_return ?? "",
                ),
              )
              .slice(0, 6)
              .map((injury, index) => (
                <div
                  key={`${injury.date ?? injury.expected_return}-${index}`}
                  className="flex items-center justify-between rounded-[6px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5 text-[10px]"
                >
                  <div className="min-w-0">
                    <div className="font-mono text-[var(--ink)]">
                      {injury.kind === "current"
                        ? t("inspector.expectedReturn", {
                            date: injury.expected_return ?? "—",
                          })
                        : (injury.date ?? t("inspector.unknownDate"))}
                    </div>
                    <div className="truncate text-[9px] text-[var(--muted)]">
                      {injury.club_name ?? t("common.unknownClub")}
                      {injury.cause != null
                        ? ` · ${injury.cause === 1 ? t("inspector.matchCause") : t("inspector.trainingCause")}`
                        : ""}
                    </div>
                  </div>
                  <Badge tone={injury.severity != null && injury.severity >= 2 ? "danger" : "outline"}>
                    {injury.severity != null
                      ? (severityLabel(injury.severity, locale) ??
                        t("inspector.severityFallback", { level: injury.severity }))
                      : (injury.type_name ?? t("inspector.injury"))}
                  </Badge>
                </div>
              ))}
          </div>
        </div>
      )}

      {player.suspensions.length > 0 && (
        <div>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("inspector.suspensionsTitle")}
          </div>
          <div className="space-y-1">
            {[...player.suspensions]
              .sort((left, right) =>
                (right.issued_date ?? "").localeCompare(left.issued_date ?? ""),
              )
              .map((suspension, index) => (
                <div
                  key={`${suspension.issued_date}-${index}`}
                  className="flex items-center justify-between rounded-[6px] border border-[var(--gold-line)] bg-[var(--gold-soft)] px-2 py-1.5 text-[10px]"
                >
                  <div>
                    <div className="font-medium text-[var(--gold)]">
                      {suspension.scope === "nation"
                        ? t("inspector.nationScope", {
                            nation:
                              nationLabel(suspension.nation_name, locale) ??
                              t("common.nationNumber", { id: suspension.nation_id ?? "?" }),
                          })
                        : suspension.scope === "competition"
                          ? t("inspector.competitionScope", {
                              id: suspension.competition_id ?? "?",
                            })
                          : t("inspector.scopeUnknown")}
                    </div>
                    <div className="text-[9px] opacity-70">
                      {t("inspector.issued", { date: suspension.issued_date ?? "—" })}
                    </div>
                  </div>
                  <span className="font-mono text-[9px] opacity-70">#{suspension.scope_code}</span>
                </div>
              ))}
          </div>
        </div>
      )}

      {contract?.on_loan && (
        <div className="rounded-xl border border-[var(--blue-line)] bg-[var(--blue-soft)] p-3 text-[11px] text-[var(--blue)]">
          <div className="font-semibold">{t("inspector.loanInfo")}</div>
          <div className="mt-1">
            {t("inspector.parentClub", { club: contract.loan_parent_club_name ?? "—" })}
          </div>
          <div>
            {t("inspector.loanPeriod", {
              start: formatDate(contract.loan_start),
              end: formatDate(contract.loan_end),
            })}
          </div>
        </div>
      )}

      {contract && contract.clauses.length > 0 && (
        <div>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("inspector.clauses")}
          </div>
          <div className="space-y-1">
            {contract.clauses.map((clause, index) => (
              <div
                key={index}
                className="flex items-center justify-between rounded-[6px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5 text-[10px]"
              >
                <span className="min-w-0 pr-2 text-[var(--muted)]">
                  {clauseKindLabel(clause.kind, locale) ??
                    t("inspector.clause", { kind: clause.kind })}
                </span>
                <span className="shrink-0 text-right font-mono text-[var(--ink)]">
                  {clauseValueLabel(clause, locale)}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {contract && contract.chain.length > 1 && (
        <div>
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("inspector.chain")}
          </div>
          <div className="space-y-1">
            {contract.chain.map((entry, index) => (
              <div key={index} className="flex items-center gap-2 rounded-[6px] border border-[var(--line)] px-2 py-1.5">
                <ChevronRight size={11} className="text-[var(--muted)]" />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[10px] font-medium text-[var(--ink)]">
                    {entry.club_name ?? t("common.unknownClub")}
                  </div>
                  <div className="text-[9px] text-[var(--muted)]">
                    {entry.start ?? "—"} → {entry.end ?? "—"}
                  </div>
                </div>
                {entry.has_terms && (
                  <Badge tone="outline">{t("inspector.contractBadge")}</Badge>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

const POSITION_ROWS: (string | null)[][] = [
  [null, "stc", null],
  ["aml", "amc", "amr"],
  ["ml", "mc", "mr"],
  ["wbl", "dm", "wbr"],
  ["dl", "dc", "dr"],
  [null, "gk", null],
];
