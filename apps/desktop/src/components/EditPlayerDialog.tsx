import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  Check,
  Loader2,
  RotateCcw,
  Save,
  Search,
  Sparkles,
} from "lucide-react";
import { editPlayer, getTraitOptions, pickEditOutputFile } from "@/lib/api";
import { attributeLabel, personalityLabel, traitLabel } from "@/lib/labels";
import { useOpenSave } from "@/lib/openSave";
import { usePlayerEditDetail } from "@/lib/query";
import {
  ATTRIBUTE_GROUPS,
  CA_EXEMPT_ATTRIBUTE_KEYS,
  PERSONALITY_KEYS,
  type AttributeEdit,
  type EditPlayerResult,
  type Player,
  type PlayerEditRequest,
} from "@/lib/types";
import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";
import { Button, Checkbox, Input, Modal } from "./ui";

type Tab = "ability" | "attributes" | "hidden" | "traits";

export function EditPlayerDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.editOpen);
  const setOpen = useAppStore((state) => state.setEditOpen);
  const saveId = useAppStore((state) => state.saveId);
  const summary = useAppStore((state) => state.summary);
  const selectedUid = useAppStore((state) => state.selectedUid);
  const pushToast = useAppStore((state) => state.pushToast);
  const { data: player } = usePlayerEditDetail(open ? selectedUid : null);
  const { openPath } = useOpenSave();
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<EditPlayerResult | null>(null);

  useEffect(() => {
    if (!open || !player) return;
    setCa(player.ca);
    setPa(player.pa ?? null);
    setAttributes({ ...player.raw_attributes });
    setPersonality(player.personality ? { ...player.personality } : {});
    setTraits(new Set(player.traits.map((trait) => trait.bit)));
    setTraitSearch("");
    setTab("ability");
    setError(null);
    setResult(null);
  }, [open, player?.uid]);

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
        keys: ["determination"],
      },
    ];
    for (const group of ATTRIBUTE_GROUPS.technical) {
      const keys =
        group.labelKey === "mental"
          ? group.keys.filter((key) => key !== "determination")
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
    setCa(player.ca);
    setPa(player.pa ?? null);
    setAttributes({ ...player.raw_attributes });
    setPersonality(player.personality ? { ...player.personality } : {});
    setTraits(new Set(player.traits.map((trait) => trait.bit)));
    setTraitSearch("");
    setError(null);
    setResult(null);
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

  const save = async () => {
    const validation = validate();
    if (validation) {
      setError(validation);
      return;
    }
    if (!player || saveId == null || ca == null) return;
    const output = await pickEditOutputFile(suggestedPath(summary?.path, summary?.save_name));
    if (!output) return;

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

    setBusy(true);
    setError(null);
    try {
      const written = await editPlayer(saveId, player.uid, output, request);
      setResult(written);
      pushToast({
        title: t("toast.editDone"),
        description: written.output,
        tone: "success",
      });
    } catch (writeError) {
      const message = String(writeError);
      setError(message);
      pushToast({ title: t("toast.editFailed"), description: message, tone: "error" });
    } finally {
      setBusy(false);
    }
  };

  const currentTab = result ? "ability" : tab;

  return (
    <Modal
      open={open}
      onClose={() => {
        if (!busy) setOpen(false);
      }}
      title={t("edit.title")}
      description={
        player
          ? `${player.name ?? t("common.unnamedPlayer")} · ${t("edit.description")}`
          : t("edit.description")
      }
      width="max-w-[900px]"
      footer={
        result ? (
          <>
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
              {t("common.close")}
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => {
                setOpen(false);
                void openPath(result.output, true);
              }}
            >
              {t("edit.openEdited")}
            </Button>
          </>
        ) : (
          <>
            <Button variant="ghost" size="sm" onClick={reset} disabled={busy || !player}>
              <RotateCcw size={13} />
              {t("edit.reset")}
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)} disabled={busy}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => void save()}
              disabled={busy || !player || changedCount === 0}
            >
              {busy ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} />}
              {busy ? t("edit.saving") : t("edit.saveCopy")}
            </Button>
          </>
        )
      }
    >
      {!player ? (
        <div className="p-6 text-xs text-[var(--muted)]">{t("edit.noPlayer")}</div>
      ) : result ? (
        <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]">
            <Check size={22} />
          </div>
          <div className="text-sm font-semibold text-[var(--ink)]">{t("edit.successTitle")}</div>
          <div className="max-w-full break-all text-[11px] text-[var(--muted)]">
            {t("edit.successDescription", { path: result.output })}
          </div>
          <div className="text-[10px] text-[var(--muted)]">
            {t("edit.changed", { count: result.changes.length })}
          </div>
        </div>
      ) : (
        <div className="space-y-3 p-5">
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
            <span className="ml-auto self-center text-[10px] text-[var(--muted)]">
              {t("edit.changed", { count: changedCount })}
            </span>
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
            <AbilityTab player={player} ca={ca} pa={pa} onCa={setCa} onPa={setPa} />
          )}
          {currentTab === "attributes" && (
            <div className="space-y-4">
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
  onCa,
  onPa,
}: {
  player: Player;
  ca: number | null;
  pa: number | null;
  onCa: (value: number | null) => void;
  onPa: (value: number | null) => void;
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
      <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[11px] leading-relaxed text-[var(--muted)]">
        {t("edit.attributeBudget")}
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
  const changed = value !== original;
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
        <span
          className={cn(
            "shrink-0 font-mono text-[10px]",
            changed ? "font-semibold text-[var(--accent-strong)]" : "text-[var(--muted)]",
          )}
        >
          {changed ? `${original} → ${value}` : value}
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

function suggestedPath(source: string | undefined, saveName: string | undefined): string {
  const cleanName = (saveName || "fm27-save")
    .replace(/[\\/:*?"<>|]/g, "_")
    .replace(/\.fm$/i, "");
  const separator = source ? Math.max(source.lastIndexOf("/"), source.lastIndexOf("\\")) : -1;
  const directory = separator >= 0 ? source!.slice(0, separator + 1) : "";
  return `${directory}${cleanName} - edited.fm`;
}
