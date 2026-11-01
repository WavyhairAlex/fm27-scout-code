import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  ClipboardPlus,
  RotateCcw,
  Search,
  Sparkles,
} from "lucide-react";
import { estimatePlayerCa, getTraitOptions } from "@/lib/api";
import { attributeLabel, personalityLabel, traitLabel } from "@/lib/labels";
import { usePlayerEditDetail } from "@/lib/query";
import {
  ATTRIBUTE_GROUPS,
  CA_EXEMPT_ATTRIBUTE_KEYS,
  PERSONALITY_KEYS,
  POSITION_LABELS,
  type AttributeEdit,
  type CaEstimate,
  type Player,
  type PlayerEditRequest,
} from "@/lib/types";
import { cn, displayAttributeValue } from "@/lib/utils";
import { useI18n } from "@/lib/useI18n";
import { useAppStore, type PendingChange } from "@/store/app";
import { Button, Checkbox, Input, Modal } from "./ui";

type Tab = "ability" | "attributes" | "hidden" | "traits";

export function EditPlayerDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.editOpen);
  const setOpen = useAppStore((state) => state.setEditOpen);
  const selectedUid = useAppStore((state) => state.selectedUid);
  const pendingEdits = useAppStore((state) => state.pendingEdits);
  const stageEdit = useAppStore((state) => state.stageEdit);
  const removePendingEdit = useAppStore((state) => state.removePendingEdit);
  const pushToast = useAppStore((state) => state.pushToast);
  const { data: player } = usePlayerEditDetail(open ? selectedUid : null);
  const { data: traitOptions = [] } = useQuery({
    queryKey: ["trait-options"],
    queryFn: getTraitOptions,
    enabled: open,
    staleTime: Infinity,
  });

  const [tab, setTab] = useState<Tab>("ability");
  const [ca, setCa] = useState<number | null>(null);
  const [pa, setPa] = useState<number | null>(null);
  const [attributes, setAttributes] = useState<Record<string, number>>({});
  const [personality, setPersonality] = useState<Record<string, number>>({});
  const [traits, setTraits] = useState<Set<number>>(new Set());
  const [traitSearch, setTraitSearch] = useState("");
  const [estimate, setEstimate] = useState<CaEstimate | null>(null);
  const [estimatePosition, setEstimatePosition] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !player) return;
    const staged = pendingEdits[player.uid];
    const nextAttributes = { ...player.raw_attributes };
    for (const edit of staged?.request.attributes ?? []) {
      nextAttributes[edit.key] = edit.value;
    }
    const nextPersonality: Record<string, number> = player.personality
      ? { ...player.personality }
      : {};
    for (const edit of staged?.request.personality ?? []) {
      nextPersonality[edit.key] = edit.value;
    }
    const nextTraits = new Set(player.traits.map((trait) => trait.bit));
    for (const bit of staged?.request.traits ?? []) nextTraits.add(bit);
    for (const bit of staged?.request.untraits ?? []) nextTraits.delete(bit);

    setCa(staged?.request.ca ?? player.ca);
    setPa(staged ? (staged.request.pa ?? player.pa ?? null) : (player.pa ?? null));
    setAttributes(nextAttributes);
    setPersonality(nextPersonality);
    setTraits(nextTraits);
    setTraitSearch("");
    setEstimatePosition(
      player.natural_positions.length > 1
        ? "AUTO"
        : (player.natural_positions[0] ?? player.best_role?.position ?? "AUTO"),
    );
    setEstimate(null);
    setTab("ability");
    setError(null);
  }, [open, player?.uid]);

  const positionOptions = useMemo(() => {
    const values = [
      ...(player?.natural_positions ?? []),
      ...(player?.accomplished_positions ?? []),
    ];
    return ["AUTO", ...(values.length > 0 ? [...new Set(values)] : ["STC"])];
  }, [player]);

  useEffect(() => {
    if (!open || !player || !estimatePosition) {
      setEstimate(null);
      return;
    }
    const handle = window.setTimeout(() => {
      const displayAttributes: Record<string, number> = {};
      for (const [key, value] of Object.entries(attributes)) {
        displayAttributes[key] = displayAttributeValue(value);
      }
      const positions =
        estimatePosition === "AUTO"
          ? player.natural_positions.length > 0
            ? player.natural_positions
            : [player.best_role?.position ?? "STC"]
          : [estimatePosition];
      void estimatePlayerCa({
        positions,
        attributes: displayAttributes,
        weaker_foot: Math.min(player.left_foot, player.right_foot),
      })
        .then(setEstimate)
        .catch(() => setEstimate(null));
    }, 120);
    return () => window.clearTimeout(handle);
  }, [
    attributes,
    estimatePosition,
    open,
    player?.left_foot,
    player?.right_foot,
    player?.uid,
  ]);

  const originalTraits = useMemo(
    () => new Set(player?.traits.map((trait) => trait.bit) ?? []),
    [player],
  );

  const isGoalkeeper =
    player?.natural_positions.includes("GK") || (player?.positions.gk ?? 0) >= 15;

  const attributeGroups = useMemo(() => {
    const groups: { key: string; label: string; note: string; keys: string[] }[] = [
      {
        key: "ca-exempt",
        label: t("edit.group.caExempt"),
        note: t("edit.note.caExempt"),
        keys: ["determination", "natural_fitness", "flair", "aggression"],
      },
    ];
    for (const group of ATTRIBUTE_GROUPS.technical) {
      const keys =
        group.labelKey === "mental"
          ? group.keys.filter(
              (key) => !["determination", "flair", "aggression"].includes(key),
            )
          : group.labelKey === "physical"
            ? group.keys.filter((key) => key !== "natural_fitness")
            : group.keys;
      groups.push({
        key: group.labelKey,
        label: t(`attribute.group.${group.labelKey}`),
        note: t("edit.group.caWeighted"),
        keys,
      });
    }
    if (isGoalkeeper) {
      groups.push({
        key: "goalkeeping",
        label: t("attribute.group.goalkeeping"),
        note: t("edit.group.caWeighted"),
        keys: ATTRIBUTE_GROUPS.goalkeeping[0].keys,
      });
    }
    return groups;
  }, [isGoalkeeper, t]);

  const caChanged = player != null && ca != null && ca !== player.ca;
  const paChanged = player != null && pa !== (player.pa ?? null);
  const caWeightedAttributeChanged =
    player != null &&
    Object.entries(attributes).some(
      ([key, value]) =>
        value !== player.raw_attributes[key] && !CA_EXEMPT_ATTRIBUTE_KEYS.has(key),
    );
  const changedCount = useMemo(() => {
    if (!player) return 0;
    let count = 0;
    if (caChanged) count += 1;
    if (paChanged) count += 1;
    for (const [key, value] of Object.entries(attributes)) {
      if (value !== player.raw_attributes[key]) count += 1;
    }
    if (player.personality) {
      for (const [key, value] of Object.entries(personality)) {
        if (value !== player.personality[key as keyof typeof player.personality]) count += 1;
      }
    }
    for (const bit of traits) if (!originalTraits.has(bit)) count += 1;
    for (const bit of originalTraits) if (!traits.has(bit)) count += 1;
    return count;
  }, [attributes, caChanged, originalTraits, paChanged, personality, player, traits]);

  const warnings = useMemo(() => {
    const list: string[] = [];
    if (caChanged) list.push(t("edit.warning.ca"));
    if (caWeightedAttributeChanged) {
      list.push(caChanged ? t("edit.warning.caAttrs") : t("edit.warning.attrs"));
    }
    return list;
  }, [caChanged, caWeightedAttributeChanged, t]);

  const reset = () => {
    if (!player) return;
    removePendingEdit(player.uid);
    setCa(player.ca);
    setPa(player.pa ?? null);
    setAttributes({ ...player.raw_attributes });
    setPersonality(player.personality ? { ...player.personality } : {});
    setTraits(new Set(player.traits.map((trait) => trait.bit)));
    setTraitSearch("");
    setError(null);
  };

  const validate = (): string | null => {
    if (!player) return t("edit.noPlayer");
    if (ca == null || ca < 1 || ca > 200) return t("edit.invalidCa");
    if (pa != null && (pa < 1 || pa > 200)) return t("edit.invalidPa");
    if (pa != null && ca > pa) return t("edit.invalidCaPa");
    if (
      Object.entries(attributes).some(
        ([key, value]) =>
          value !== player.raw_attributes[key] && (value < 1 || value > 100),
      )
    ) {
      return t("edit.invalidAttribute");
    }
    if (
      player.personality &&
      Object.entries(personality).some(
        ([key, value]) =>
          value !== player.personality![key as keyof typeof player.personality] &&
          (value < 1 || value > 20),
      )
    ) {
      return t("edit.invalidPersonality");
    }
    if (changedCount === 0) return t("edit.noChanges");
    return null;
  };

  const stage = () => {
    const validation = validate();
    if (validation) {
      setError(validation);
      return;
    }
    if (!player || ca == null) return;

    const attributeEdits: AttributeEdit[] = Object.entries(attributes)
      .filter(([key, value]) => value !== player.raw_attributes[key])
      .map(([key, value]) => ({ key, value }));
    const personalityEdits: AttributeEdit[] = Object.entries(personality)
      .filter(
        ([key, value]) =>
          value !== player.personality?.[key as keyof typeof player.personality],
      )
      .map(([key, value]) => ({ key, value }));
    const traitsToAdd = [...traits].filter((bit) => !originalTraits.has(bit));
    const traitsToRemove = [...originalTraits].filter((bit) => !traits.has(bit));
    const request: PlayerEditRequest = {
      ca: caChanged ? ca : null,
      pa: paChanged ? pa : null,
      attributes: attributeEdits,
      personality: personalityEdits,
      traits: traitsToAdd,
      untraits: traitsToRemove,
    };

    const changes: PendingChange[] = [];
    if (caChanged) {
      changes.push({ field: "ca", before: player.ca, after: ca });
    }
    if (paChanged) {
      changes.push({ field: "pa", before: player.pa ?? 0, after: pa ?? 0 });
    }
    for (const edit of attributeEdits) {
      changes.push({
        field: `attribute.${edit.key}`,
        before: player.raw_attributes[edit.key] ?? 0,
        after: edit.value,
      });
    }
    for (const edit of personalityEdits) {
      changes.push({
        field: `personality.${edit.key}`,
        before:
          player.personality?.[edit.key as keyof typeof player.personality] ?? 0,
        after: edit.value,
      });
    }
    for (const bit of traitsToAdd) {
      changes.push({
        field: `trait.${bit}`,
        before: 0,
        after: 1,
        label: traitOptions.find((option) => option.bit === bit)?.name,
      });
    }
    for (const bit of traitsToRemove) {
      changes.push({
        field: `trait.${bit}`,
        before: 1,
        after: 0,
        label: traitOptions.find((option) => option.bit === bit)?.name,
      });
    }

    const displayName = player.name ?? t("common.unnamedPlayer");
    stageEdit({
      uid: player.uid,
      name: displayName,
      changes,
      request,
    });
    setOpen(false);
    pushToast({
      title: t("toast.staged"),
      description: t("toast.stagedDescription", {
        name: displayName,
        count: changes.length,
      }),
      tone: "success",
    });
  };

  const currentTab = tab;

  return (
    <Modal
      open={open}
      onClose={() => setOpen(false)}
      title={t("edit.title")}
      description={
        player
          ? `${player.name ?? t("common.unnamedPlayer")} · ${t("edit.description")}`
          : t("edit.description")
      }
      width="max-w-[900px]"
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={reset} disabled={!player}>
            <RotateCcw size={13} />
            {t("edit.reset")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={stage}
            disabled={!player || changedCount === 0}
          >
            <ClipboardPlus size={13} />
            {t("edit.stage")}
          </Button>
        </>
      }
    >
      {!player ? (
        <div className="p-6 text-xs text-[var(--muted)]">{t("edit.noPlayer")}</div>
      ) : (
        <div className="space-y-3 p-5">
          <div className="flex items-start gap-2 rounded-lg border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-2 text-[10px] leading-relaxed text-[var(--danger)]">
            <AlertTriangle size={13} className="mt-0.5 shrink-0" />
            <span>{t("edit.riskWarning")}</span>
          </div>
          <div className="flex flex-wrap gap-1 border-b border-[var(--line)] pb-2">
            {[
              ["ability", t("edit.tab.ability")],
              ["attributes", t("edit.tab.attributes")],
              ["hidden", t("edit.tab.hidden")],
              ["traits", t("edit.tab.traits")],
            ].map(([key, label]) => (
              <button
                key={key}
                type="button"
                onClick={() => setTab(key as Tab)}
                className={cn(
                  "focus-ring rounded-[6px] px-2.5 py-1 text-[11px] font-medium transition-colors",
                  currentTab === key
                    ? "bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                    : "text-[var(--muted)] hover:bg-[var(--surface-2)] hover:text-[var(--ink)]",
                )}
              >
                {label}
              </button>
            ))}
            <div className="ml-auto flex items-center gap-2 self-center text-[10px]">
              {estimate && (
                <span className="rounded-[4px] bg-[var(--accent-soft)] px-1.5 py-0.5 font-mono text-[var(--accent-strong)]">
                  {t("edit.budget.estimatedShort", { value: estimate.estimated_ca })}
                </span>
              )}
              <span className="text-[var(--muted)]">
                {t("edit.changed", { count: changedCount })}
              </span>
            </div>
          </div>

          {warnings.length > 0 && (
            <div className="space-y-1.5">
              {warnings.map((warning) => (
                <div
                  key={warning}
                  className="flex items-start gap-2 rounded-lg border border-[var(--gold-line)] bg-[var(--gold-soft)] px-3 py-2 text-[10px] leading-relaxed text-[var(--gold)]"
                >
                  <AlertTriangle size={13} className="mt-0.5 shrink-0" />
                  <span>{warning}</span>
                </div>
              ))}
            </div>
          )}

          {currentTab === "ability" && (
            <AbilityTab
              player={player}
              ca={ca}
              pa={pa}
              estimate={estimate}
              estimatePosition={estimatePosition}
              positionOptions={positionOptions}
              weakerFoot={Math.min(player.left_foot, player.right_foot)}
              onCa={setCa}
              onPa={setPa}
              onEstimatePosition={setEstimatePosition}
              onUseEstimate={setCa}
            />
          )}
          {currentTab === "attributes" && (
            <div className="space-y-4">
              <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] leading-relaxed text-[var(--muted)]">
                {t("edit.attributeScale")}
              </div>
              {attributeGroups.map((group) => (
                <section key={group.key}>
                  <div className="mb-1.5 flex items-center justify-between">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                      {group.label}
                    </span>
                    <span className="text-[9px] text-[var(--muted)]">{group.note}</span>
                  </div>
                  <div className="grid grid-cols-2 gap-1.5">
                    {group.keys.map((key) => (
                      <AttributeEditor
                        key={key}
                        label={attributeLabel(key, locale)}
                        value={attributes[key] ?? 1}
                        original={player.raw_attributes[key] ?? 1}
                        max={100}
                        onChange={(value) =>
                          setAttributes((current) => ({ ...current, [key]: value }))
                        }
                      />
                    ))}
                  </div>
                </section>
              ))}
            </div>
          )}
          {currentTab === "hidden" && (
            <div className="space-y-4">
              <section>
                <div className="mb-1.5 flex items-center justify-between">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                    {t("edit.group.hidden")}
                  </span>
                  <span className="text-[9px] text-[var(--muted)]">{t("edit.note.hidden")}</span>
                </div>
                <div className="grid grid-cols-2 gap-1.5">
                  {ATTRIBUTE_GROUPS.hidden[0].keys.map((key) => (
                    <AttributeEditor
                      key={key}
                      label={attributeLabel(key, locale)}
                      value={attributes[key] ?? 1}
                      original={player.raw_attributes[key] ?? 1}
                      max={100}
                      onChange={(value) =>
                        setAttributes((current) => ({ ...current, [key]: value }))
                      }
                    />
                  ))}
                </div>
              </section>
              <section>
                <div className="mb-1.5 flex items-center justify-between">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                    {t("edit.group.personality")}
                  </span>
                  <span className="text-[9px] text-[var(--muted)]">{t("edit.note.hidden")}</span>
                </div>
                {player.personality ? (
                  <div className="grid grid-cols-2 gap-1.5">
                    {PERSONALITY_KEYS.map((key) => (
                      <AttributeEditor
                        key={key}
                        label={personalityLabel(key, locale)}
                        value={personality[key] ?? 1}
                        original={player.personality![key as keyof typeof player.personality]}
                        max={20}
                        onChange={(value) =>
                          setPersonality((current) => ({ ...current, [key]: value }))
                        }
                      />
                    ))}
                  </div>
                ) : (
                  <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] text-[var(--muted)]">
                    {t("edit.personalityUnavailable")}
                  </div>
                )}
              </section>
            </div>
          )}
          {currentTab === "traits" && (
            <TraitsTab
              locale={locale}
              options={traitOptions}
              selected={traits}
              search={traitSearch}
              available={player.trait_bits != null}
              onSearch={setTraitSearch}
              onToggle={(bit, checked) =>
                setTraits((current) => {
                  const next = new Set(current);
                  if (checked) next.add(bit);
                  else next.delete(bit);
                  return next;
                })
              }
            />
          )}

          <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] leading-relaxed text-[var(--muted)]">
            {t("edit.outputHint")} {t("edit.warning.singlePlayer")}
          </div>
          {error && (
            <div className="rounded-lg border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-2 text-[11px] text-[var(--danger)]">
              {error}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

function AbilityTab({
  player,
  ca,
  pa,
  estimate,
  estimatePosition,
  positionOptions,
  weakerFoot,
  onCa,
  onPa,
  onEstimatePosition,
  onUseEstimate,
}: {
  player: Player;
  ca: number | null;
  pa: number | null;
  estimate: CaEstimate | null;
  estimatePosition: string;
  positionOptions: string[];
  weakerFoot: number;
  onCa: (value: number | null) => void;
  onPa: (value: number | null) => void;
  onEstimatePosition: (position: string) => void;
  onUseEstimate: (value: number) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("edit.ca")}
          </div>
          <Input
            type="number"
            min={1}
            max={200}
            value={ca ?? ""}
            onChange={(event) => {
              const value = event.target.value;
              onCa(value === "" ? null : Number(value));
            }}
            className="h-9 font-mono text-sm"
          />
          <div className="text-[10px] text-[var(--muted)]">
            {t("edit.original", { value: player.ca })}
          </div>
        </div>
        <div className="space-y-1.5">
          <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("edit.pa")}
          </div>
          <Input
            type="number"
            min={1}
            max={200}
            value={pa ?? ""}
            onChange={(event) => {
              const value = event.target.value;
              onPa(value === "" ? null : Number(value));
            }}
            className="h-9 font-mono text-sm"
          />
          <div className="text-[10px] text-[var(--muted)]">
            {t("edit.original", { value: player.pa ?? "—" })}
          </div>
        </div>
      </div>
      <CaBudget
        estimate={estimate}
        targetCa={ca ?? player.ca}
        position={estimatePosition}
        options={positionOptions}
        weakerFoot={weakerFoot}
        onPosition={onEstimatePosition}
        onUseEstimate={onUseEstimate}
      />
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[11px] leading-relaxed text-[var(--muted)]">
        {t("edit.attributeBudget")}
      </div>
    </div>
  );
}

function CaBudget({
  estimate,
  targetCa,
  position,
  options,
  weakerFoot,
  onPosition,
  onUseEstimate,
}: {
  estimate: CaEstimate | null;
  targetCa: number;
  position: string;
  options: string[];
  weakerFoot: number;
  onPosition: (position: string) => void;
  onUseEstimate: (value: number) => void;
}) {
  const { t } = useI18n();
  const delta = estimate ? estimate.estimated_ca - targetCa : 0;
  const outlier = estimate != null && Math.abs(delta) > 8;
  const status =
    !estimate
      ? t("edit.budget.loading")
      : outlier
        ? t("edit.budget.outlier", {
            estimate: estimate.estimated_ca,
            ca: targetCa,
          })
        : delta > 3
          ? t("edit.budget.over", {
              estimate: estimate.estimated_ca,
              ca: targetCa,
            })
          : delta < -3
            ? t("edit.budget.under", {
                estimate: estimate.estimated_ca,
                ca: targetCa,
              })
            : t("edit.budget.balanced");
  const dotColor = !estimate
    ? "bg-[var(--accent)]"
    : outlier
      ? "bg-[var(--danger)]"
      : Math.abs(delta) <= 3
        ? "bg-[var(--accent)]"
        : delta > 0
          ? "bg-[var(--gold)]"
          : "bg-[var(--blue)]";
  const estimatePercent = estimate ? Math.max(2, Math.min(100, (estimate.estimated_ca / 200) * 100)) : 0;
  const targetPercent = Math.max(0, Math.min(100, (targetCa / 200) * 100));
  return (
    <div
      className={cn(
        "rounded-xl border bg-[var(--surface)] p-3",
        outlier ? "border-[var(--danger-line)]" : "border-[var(--line)]",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("edit.budget.title")}
          </div>
          <div className="mt-1 flex items-baseline gap-1">
            <span className="font-mono text-[22px] font-semibold leading-none text-[var(--ink)]">
              {estimate?.estimated_ca ?? "—"}
            </span>
            <span className="text-[10px] text-[var(--muted)]">/ 200</span>
          </div>
          <div className="mt-1 text-[9px] text-[var(--muted)]">
            {t("edit.budget.estimated")}
            {estimate
              ? ` · ${t("edit.budget.average", {
                  value: estimate.weighted_average.toFixed(1),
                })}`
              : ""}
          </div>
          <div className="mt-0.5 text-[9px] text-[var(--muted)]">
            {t("edit.budget.basis", {
              position: estimate?.position ?? "—",
              foot: weakerFoot,
            })}
          </div>
        </div>
        <label className="flex flex-col items-end gap-1 text-[9px] text-[var(--muted)]">
          {t("edit.budget.position")}
          <select
            value={position}
            onChange={(event) => onPosition(event.target.value)}
            className="h-7 rounded-[6px] border border-[var(--line)] bg-[var(--surface)] px-1.5 text-[10px] text-[var(--ink)]"
          >
            {options.map((option) => (
              <option key={option} value={option}>
                {option === "AUTO"
                  ? t("edit.budget.auto")
                  : (POSITION_LABELS[option.toLowerCase()] ?? option)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="relative mt-3 h-2 overflow-hidden rounded-full bg-[var(--surface-3)]">
        <div
          className={cn(
            "h-full rounded-full transition-[width]",
            outlier
              ? "bg-[var(--danger)]"
              : delta > 3
                ? "bg-[var(--gold)]"
                : delta < -3
                  ? "bg-[var(--blue)]"
                  : "bg-[var(--accent)]",
          )}
          style={{ width: `${estimatePercent}%` }}
        />
        <div
          className="absolute top-[-2px] h-3 w-[2px] rounded-full bg-[var(--ink)]/70"
          style={{ left: `calc(${targetPercent}% - 1px)` }}
          title={t("edit.budget.currentCa", { value: targetCa })}
        />
      </div>
      <div className="mt-2 flex items-start justify-between gap-2 text-[10px] leading-relaxed">
        <div className="flex items-start gap-1.5">
          <span className={cn("mt-[3px] h-1.5 w-1.5 shrink-0 rounded-full", dotColor)} />
          <span className="text-[var(--muted)]">{status}</span>
        </div>
        {estimate && estimate.estimated_ca !== targetCa && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 shrink-0 px-1.5 text-[9px]"
            onClick={() => onUseEstimate(estimate.estimated_ca)}
          >
            {t("edit.budget.use", { value: estimate.estimated_ca })}
          </Button>
        )}
      </div>
      <div className="mt-1 text-[9px] leading-relaxed text-[var(--muted)]">
        {t("edit.budget.note")}
      </div>
    </div>
  );
}

function AttributeEditor({
  label,
  value,
  original,
  max,
  onChange,
}: {
  label: string;
  value: number;
  original: number;
  max: number;
  onChange: (value: number) => void;
}) {
  const { t } = useI18n();
  const changed = value !== original;
  const showGameValue = max === 100;
  return (
    <div
      className={cn(
        "rounded-lg border px-2 py-1.5 transition-colors",
        changed
          ? "border-[var(--accent-line)] bg-[var(--accent-soft)]/40"
          : "border-[var(--line)] bg-[var(--surface)]",
      )}
    >
      <div className="mb-1 flex items-center justify-between gap-2">
        <span className="truncate text-[10px] text-[var(--muted)]">{label}</span>
        <span className="flex shrink-0 items-center gap-1">
          <span
            className={cn(
              "font-mono text-[10px]",
              changed ? "font-semibold text-[var(--accent-strong)]" : "text-[var(--muted)]",
            )}
          >
            {changed ? `${original} → ${value}` : value}
          </span>
          {showGameValue && (
            <span className="rounded-[3px] bg-[var(--surface-2)] px-1 py-0.5 text-[8px] text-[var(--muted)]">
              {t("edit.gameValue", { value: displayAttributeValue(value) })}
            </span>
          )}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <input
          type="range"
          min={1}
          max={max}
          value={value}
          onChange={(event) => onChange(Number(event.target.value))}
          className="h-1 flex-1 accent-[var(--accent)]"
          aria-label={label}
        />
        <Input
          type="number"
          min={1}
          max={max}
          value={value}
          onChange={(event) => {
            const next = Number(event.target.value);
            if (Number.isFinite(next)) onChange(Math.max(1, Math.min(max, next)));
          }}
          className="h-6 w-14 px-1.5 font-mono text-[10px]"
        />
      </div>
    </div>
  );
}

function TraitsTab({
  locale,
  options,
  selected,
  search,
  available,
  onSearch,
  onToggle,
}: {
  locale: "en" | "zh-CN";
  options: { bit: number; name: string }[];
  selected: Set<number>;
  search: string;
  available: boolean;
  onSearch: (value: string) => void;
  onToggle: (bit: number, checked: boolean) => void;
}) {
  const { t } = useI18n();
  if (!available) {
    return (
      <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] text-[var(--muted)]">
        {t("edit.traitsUnavailable")}
      </div>
    );
  }
  const filtered = options.filter((option) =>
    traitLabel(option.bit, option.name, locale).toLowerCase().includes(search.toLowerCase()),
  );
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Search size={13} className="text-[var(--muted)]" />
        <Input
          value={search}
          onChange={(event) => onSearch(event.target.value)}
          placeholder={t("edit.traits.search")}
          className="h-7 text-[11px]"
        />
        <span className="shrink-0 text-[10px] text-[var(--muted)]">
          {t("edit.traitCount", { count: selected.size })}
        </span>
      </div>
      <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] leading-relaxed text-[var(--muted)]">
        {t("edit.note.traits")}
      </div>
      <div className="thin-scroll grid max-h-[430px] grid-cols-2 gap-x-4 gap-y-2 overflow-y-auto rounded-xl border border-[var(--line)] p-3">
        {filtered.map((option) => (
          <Checkbox
            key={option.bit}
            checked={selected.has(option.bit)}
            onChange={(checked) => onToggle(option.bit, checked)}
            label={
              <span className="inline-flex items-center gap-1.5">
                <Sparkles size={10} className="text-[var(--muted)]" />
                {traitLabel(option.bit, option.name, locale)}
              </span>
            }
          />
        ))}
        {filtered.length === 0 && (
          <div className="col-span-2 py-6 text-center text-[11px] text-[var(--muted)]">
            {t("edit.traits.none")}
          </div>
        )}
      </div>
    </div>
  );
}


