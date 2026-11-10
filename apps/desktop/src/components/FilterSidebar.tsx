import { useMemo, useState } from "react";
import {
  ChevronDown,
  CircleHelp,
  Filter,
  Plus,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
  X,
} from "lucide-react";
import { useFilterOptions } from "@/lib/query";
import { ATTRIBUTE_FILTER_GROUPS, POSITION_LABELS, UNKNOWN_GENDER_ID } from "@/lib/types";
import { attributeLabel, nationLabel } from "@/lib/labels";
import { useI18n } from "@/lib/useI18n";
import { cn } from "@/lib/utils";
import { EMPTY_FILTERS, useAppStore, type Filters } from "@/store/app";
import { Badge, Button, Checkbox, Input, Label, Select, Switch } from "./ui";
import { MultiSelect } from "./MultiSelect";
import { RangeInput } from "./RangeInput";

const POSITION_ROWS: (string | null)[][] = [
  [null, "stc", null],
  ["aml", "amc", "amr"],
  ["ml", "mc", "mr"],
  ["wbl", "dm", "wbr"],
  ["dl", "dc", "dr"],
  [null, "gk", null],
];

export function FilterSidebar() {
  const { t, locale } = useI18n();
  const filters = useAppStore((state) => state.filters);
  const patchFilters = useAppStore((state) => state.patchFilters);
  const resetFilters = useAppStore((state) => state.resetFilters);
  const togglePosition = useAppStore((state) => state.togglePosition);
  const addAttribute = useAppStore((state) => state.addAttribute);
  const updateAttribute = useAppStore((state) => state.updateAttribute);
  const removeAttribute = useAppStore((state) => state.removeAttribute);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const dataMode = useAppStore((state) => state.dataMode);
  const setHiddenEnabled = useAppStore((state) => state.setHiddenEnabled);
  const pushToast = useAppStore((state) => state.pushToast);
  const summary = useAppStore((state) => state.summary);
  const savedViews = useAppStore((state) => state.savedViews);
  const saveView = useAppStore((state) => state.saveView);
  const applyView = useAppStore((state) => state.applyView);
  const deleteView = useAppStore((state) => state.deleteView);
  const { data: options } = useFilterOptions();
  const [savingView, setSavingView] = useState(false);
  const [viewNameDraft, setViewNameDraft] = useState("");

  const activeCount = useMemo(() => {
    let count = 0;
    if (filters.search) count++;
    if (filters.clubUids.length) count++;
    if (filters.nationIds.length) count++;
    if (filters.genders.length) count++;
    if (filters.availability) count++;
    if (filters.listings.length) count++;
    if (filters.ageMin != null || filters.ageMax != null) count++;
    if (filters.caMin != null || filters.caMax != null) count++;
    if (
      filters.paMin != null ||
      filters.paMax != null ||
      filters.paCaGapMin != null
    )
      count++;
    if (filters.generated != null) count++;
    if (filters.positions.length) count++;
    if (filters.attributes.length) count += filters.attributes.length;
    if (filters.valueMin != null || filters.valueMax != null) count++;
    if (filters.wageMin != null || filters.wageMax != null) count++;
    if (filters.contractEndBefore || filters.contractEndAfter) count++;
    return count;
  }, [filters]);

  const nationOptions = useMemo(
    () =>
      (options?.nations ?? []).map((option) => ({
        ...option,
        name: nationLabel(option.name, locale) ?? option.name,
      })),
    [locale, options?.nations],
  );

  const attributeGroups = useMemo(
    () =>
      ATTRIBUTE_FILTER_GROUPS.map((group) => ({
        labelKey: group.labelKey,
        options: group.keys
          .map((key) => [key, attributeLabel(key, locale)] as const)
          .sort((a, b) => a[1].localeCompare(b[1], locale)),
      })),
    [locale],
  );

  const listingOptions = useMemo(
    () =>
      (options?.listing ?? []).map((option) => ({
        ...option,
        name:
          option.id === 1
            ? t("filter.listing.sale")
            : t("filter.listing.loan"),
      })),
    [options?.listing, t],
  );

  const presets = useMemo(() => {
    const gameDate = summary?.game_date ?? "2026-07-13";
    const options: {
      id: "all" | "wonderkid" | "contractYear" | "highPotential";
      label: string;
      patch: Partial<Filters>;
    }[] = [
      { id: "all", label: t("filter.preset.all"), patch: {} },
      {
        id: "wonderkid",
        label: t("filter.preset.wonderkid"),
        patch: { ageMax: 21, paMin: 140, paCaGapMin: 10 },
      },
      {
        id: "contractYear",
        label: t("filter.preset.contractYear"),
        patch: {
          contractEndAfter: gameDate,
          contractEndBefore: nextYear(gameDate),
        },
      },
      {
        id: "highPotential",
        label: t("filter.preset.highPotential"),
        patch: { caMin: 140, paMin: 160, ageMax: 23 },
      },
    ];
    return options
      .filter(
        (option) =>
          dataMode === "full" ||
          (option.id !== "wonderkid" && option.id !== "highPotential"),
      )
      .map((option) => ({
        ...option,
        active: matchesFilters(filters, option.patch),
      }));
  }, [dataMode, filters, summary?.game_date, t]);

  return (
    <aside className="flex h-full w-[252px] shrink-0 flex-col border-r border-[var(--line)] bg-[var(--surface)]/80">
      <div className="flex items-center justify-between border-b border-[var(--line)] px-3 py-2.5">
        <div className="flex items-center gap-2">
          <Filter size={14} className="text-[var(--accent)]" />
          <span className="text-xs font-semibold tracking-wide text-[var(--ink)]">
            {t("filter.title")}
          </span>
          {activeCount > 0 && (
            <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-[var(--accent)] px-1 text-[10px] font-bold text-white">
              {activeCount}
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={resetFilters}
          disabled={activeCount === 0}
          className="h-6 px-1.5 text-[10px]"
        >
          <RotateCcw size={11} />
          {t("filter.reset")}
        </Button>
      </div>

      <div className="thin-scroll flex-1 space-y-5 overflow-y-auto px-3 py-3.5">
        <section className="grid grid-cols-2 gap-1">
          {presets.map(({ id, label, patch, active }) => (
            <button
              key={id}
              type="button"
              onClick={() => {
                resetFilters();
                if (id !== "all") patchFilters(patch);
              }}
              className={
                active
                  ? "rounded-[7px] border border-[var(--accent)] bg-[var(--accent-soft)] px-2 py-1.5 text-[10px] font-semibold text-[var(--accent-strong)]"
                  : "rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5 text-[10px] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]"
              }
            >
              {label}
            </button>
          ))}
        </section>

        <section className="space-y-1.5">
          <div className="flex items-center justify-between">
            <Label>{t("filter.myViews")}</Label>
            <button
              type="button"
              onClick={() => {
                setViewNameDraft(t("filter.viewDefaultName", { count: savedViews.length + 1 }));
                setSavingView(true);
              }}
              className="rounded px-1 text-[10px] font-medium text-[var(--accent-strong)] hover:bg-[var(--accent-soft)]"
            >
              {t("filter.saveView")}
            </button>
          </div>
          {savingView && (
            <div className="flex items-center gap-1">
              <Input
                autoFocus
                value={viewNameDraft}
                onChange={(event) => setViewNameDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && viewNameDraft.trim()) {
                    saveView(viewNameDraft);
                    setSavingView(false);
                    setViewNameDraft("");
                  }
                  if (event.key === "Escape") setSavingView(false);
                }}
                placeholder={t("filter.viewNamePlaceholder")}
                className="h-6 flex-1 text-[10px]"
              />
              <Button
                size="sm"
                className="h-6 px-1.5 text-[10px]"
                onClick={() => {
                  if (viewNameDraft.trim()) {
                    saveView(viewNameDraft);
                    setSavingView(false);
                    setViewNameDraft("");
                  }
                }}
              >
                {t("common.save")}
              </Button>
            </div>
          )}
          {savedViews.length === 0 ? (
            <div className="rounded-[6px] border border-dashed border-[var(--line)] px-2 py-1.5 text-[9px] text-[var(--muted)]">
              {t("filter.noViews")}
            </div>
          ) : (
            <div className="space-y-1">
              {savedViews.map((view) => (
                <div
                  key={view.id}
                  className="flex items-center gap-1 rounded-[6px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1"
                >
                  <button
                    type="button"
                    onClick={() => applyView(view.id)}
                    className="min-w-0 flex-1 truncate text-left text-[10px] text-[var(--ink)] hover:text-[var(--accent-strong)]"
                  >
                    {view.name}
                  </button>
                  <button
                    type="button"
                    onClick={() => deleteView(view.id)}
                    className="rounded px-0.5 text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--danger)]"
                    aria-label={t("filter.deleteView", { name: view.name })}
                  >
                    <X size={10} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="space-y-1.5">
          <Label>{t("filter.club")}</Label>
          <MultiSelect
            options={options?.clubs ?? []}
            selected={filters.clubUids}
            onChange={(clubUids) => patchFilters({ clubUids })}
            placeholder={t("filter.allClubs")}
          />
        </section>

        <section className="space-y-1.5">
          <Label>{t("filter.nation")}</Label>
          <MultiSelect
            options={nationOptions}
            selected={filters.nationIds}
            onChange={(nationIds) => patchFilters({ nationIds })}
            placeholder={t("filter.allNations")}
            maxLabel={2}
          />
        </section>

        <Section title={t("filter.genderStatus")} icon={<SlidersHorizontal size={12} />}>
          <div className="grid grid-cols-2 gap-1">
            {[
              { ids: [] as number[], label: t("filter.allGenders"), testId: "gender-all" },
              { ids: [0], label: t("filter.men"), testId: "gender-men" },
              { ids: [1], label: t("filter.women"), testId: "gender-women" },
              {
                ids: [UNKNOWN_GENDER_ID],
                label: t("filter.unknownGender"),
                testId: "gender-unknown",
              },
            ].map((option) => {
              const active =
                option.ids.length === 0
                  ? filters.genders.length === 0
                  : filters.genders.length === option.ids.length &&
                    option.ids.every((id) => filters.genders.includes(id));
              const count =
                option.ids.length === 0
                  ? options?.genders.reduce((sum, item) => sum + item.count, 0)
                  : options?.genders
                      .filter((item) => option.ids.includes(item.id))
                      .reduce((sum, item) => sum + item.count, 0);
              return (
                <button
                  key={option.testId}
                  type="button"
                  data-testid={option.testId}
                  onClick={() => patchFilters({ genders: option.ids })}
                  className={cn(
                    "flex items-center justify-between rounded-[6px] border px-2 py-1.5 text-[10px] transition-colors",
                    active
                      ? "border-[var(--accent)] bg-[var(--accent-soft)] font-semibold text-[var(--accent-strong)]"
                      : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]",
                  )}
                >
                  <span>{option.label}</span>
                  {count != null && <span className="font-mono text-[9px] opacity-70">{count}</span>}
                </button>
              );
            })}
            <button
              type="button"
              onClick={() => patchFilters({ genders: [] })}
              className="rounded-[6px] border border-dashed border-[var(--line)] px-2 py-1.5 text-[10px] text-[var(--muted)] hover:text-[var(--ink)]"
            >
              {t("filter.clearGender")}
            </button>
          </div>
          <div className="grid grid-cols-2 gap-1">
            {[
              ["", t("filter.allStatuses")],
              ["injured", t("filter.status.injured")],
              // "injury_history" is temporarily hidden from the sidebar; the backend
              // filter and its translations are kept so it can be restored easily.
              ["suspended", t("filter.status.suspended")],
              ["loan", t("filter.status.loan")],
              ["free", t("filter.status.free")],
              ["available", t("filter.status.available")],
            ].map(([value, label]) => {
              const active = (filters.availability ?? "") === value;
              const statusId = {
                injured: 1,
                suspended: 2,
                loan: 3,
                free: 4,
                available: 5,
              }[value as "injured" | "suspended" | "loan" | "free" | "available"];
              const count = value
                ? options?.availability.find((item) => item.id === statusId)?.count
                : null;
              return (
                <button
                  key={value || "all"}
                  type="button"
                  onClick={() =>
                    patchFilters({ availability: value || null })
                  }
                  className={cn(
                    "flex items-center justify-between rounded-[6px] border px-2 py-1.5 text-[10px] transition-colors",
                    active
                      ? "border-[var(--accent)] bg-[var(--accent-soft)] font-semibold text-[var(--accent-strong)]"
                      : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]",
                  )}
                >
                  <span>{label}</span>
                  {count != null && <span className="font-mono text-[9px] opacity-70">{count}</span>}
                </button>
              );
            })}
          </div>
        </Section>

        <Section title={t("filter.origin")} icon={<Sparkles size={12} />}>
          <Checkbox
            checked={filters.generated === true}
            onChange={(checked) => patchFilters({ generated: checked ? true : null })}
            label={t("filter.generatedOnly")}
          />
          <p className="text-[10px] leading-snug text-[var(--muted)]">
            {t("filter.originHint")}
          </p>
        </Section>

        <Section title={t("filter.listing")} icon={<SlidersHorizontal size={12} />}>
          <MultiSelect
            options={listingOptions}
            selected={filters.listings.map((listing) =>
              listing === "sale" ? 1 : 2,
            )}
            onChange={(ids) =>
              patchFilters({
                listings: ids.map((id) => (id === 1 ? "sale" : "loan")),
              })
            }
            placeholder={t("filter.allListings")}
            searchable={false}
            maxLabel={1}
          />
        </Section>

        <Section title={t("filter.ageAbility")} icon={<SlidersHorizontal size={12} />}>
          <RangeInput
            label={t("filter.age")}
            min={filters.ageMin}
            max={filters.ageMax}
            onMin={(ageMin) => patchFilters({ ageMin })}
            onMax={(ageMax) => patchFilters({ ageMax })}
            placeholderMin="16"
            placeholderMax="45"
          />
          {dataMode === "full" && (
            <>
              <RangeInput
                label="CA"
                min={filters.caMin}
                max={filters.caMax}
                onMin={(caMin) => patchFilters({ caMin })}
                onMax={(caMax) => patchFilters({ caMax })}
                placeholderMin="0"
                placeholderMax="200"
              />
              <RangeInput
                label="PA"
                min={filters.paMin}
                max={filters.paMax}
                onMin={(paMin) => patchFilters({ paMin })}
                onMax={(paMax) => patchFilters({ paMax })}
                placeholderMin="0"
                placeholderMax="200"
              />
              <div className="flex items-center justify-between gap-2">
                <Label>{t("filter.paCaGap")}</Label>
                <Input
                  type="number"
                  min={0}
                  max={80}
                  value={filters.paCaGapMin ?? ""}
                  placeholder="0"
                  onChange={(event) =>
                    patchFilters({
                      paCaGapMin: event.target.value === "" ? null : Number(event.target.value),
                    })
                  }
                  className="h-7 w-16 font-mono text-[10px]"
                />
              </div>
            </>
          )}
        </Section>

        <Section title={t("filter.positions")} icon={<CircleHelp size={12} />}>
          <div className="grid grid-cols-3 gap-1">
            {POSITION_ROWS.flat().map((position, index) =>
              position ? (
                <button
                  key={position}
                  type="button"
                  onClick={() => togglePosition(position)}
                  className={cn(
                    "focus-ring flex h-7 items-center justify-center rounded-[6px] border font-mono text-[10px] font-semibold transition-colors",
                    filters.positions.includes(position)
                      ? "border-[var(--accent)] bg-[var(--accent)] text-white"
                      : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]",
                  )}
                >
                  {POSITION_LABELS[position]}
                </button>
              ) : (
                <div key={`blank-${index}`} />
              ),
            )}
          </div>
          <div className="flex items-center justify-between text-[10px] text-[var(--muted)]">
            <span>{t("filter.minRating")}</span>
            <div className="flex items-center gap-1.5">
              <input
                type="range"
                min={1}
                max={20}
                value={filters.positionMin}
                onChange={(event) =>
                  patchFilters({ positionMin: Number(event.target.value) })
                }
                className="h-1 w-24 accent-[var(--accent)]"
              />
              <span className="w-5 text-right font-mono text-[var(--ink)]">
                {filters.positionMin}
              </span>
            </div>
          </div>
        </Section>

        {dataMode !== "moneyball" && (
        <Section title={t("filter.attributeRules")} icon={<Plus size={12} />}>
          <Select
            value=""
            onChange={(event) => {
              if (event.target.value) addAttribute(event.target.value);
            }}
            className="h-7 text-[11px]"
          >
            <option value="">{t("filter.addAttribute")}</option>
            {attributeGroups.map((group) => (
              <optgroup key={group.labelKey} label={t(group.labelKey)}>
                {group.options.map(([key, label]) => (
                  <option key={key} value={key}>
                    {label}
                  </option>
                ))}
              </optgroup>
            ))}
          </Select>
          <div className="space-y-1.5">
            {filters.attributes.map((rule) => (
              <div
                key={rule.id}
                className="rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] p-2"
              >
                <div className="mb-1.5 flex items-center justify-between">
                  <span className="text-[11px] font-medium text-[var(--ink)]">
                    {attributeLabel(rule.key, locale)}
                  </span>
                  <button
                    type="button"
                    onClick={() => removeAttribute(rule.id)}
                    className="rounded p-0.5 text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--danger)]"
                    aria-label={t("filter.removeAttribute")}
                  >
                    <X size={11} />
                  </button>
                </div>
                <div className="flex items-center gap-1.5">
                  <input
                    type="number"
                    min={1}
                    max={20}
                    placeholder={t("common.min")}
                    value={rule.min ?? ""}
                    onChange={(event) =>
                      updateAttribute(rule.id, {
                        min: event.target.value === "" ? null : Number(event.target.value),
                      })
                    }
                    className="h-6 w-full rounded-[5px] border border-[var(--line)] bg-[var(--surface)] px-1.5 font-mono text-[10px]"
                  />
                  <span className="text-[10px] text-[var(--muted)]">–</span>
                  <input
                    type="number"
                    min={1}
                    max={20}
                    placeholder={t("common.max")}
                    value={rule.max ?? ""}
                    onChange={(event) =>
                      updateAttribute(rule.id, {
                        max: event.target.value === "" ? null : Number(event.target.value),
                      })
                    }
                    className="h-6 w-full rounded-[5px] border border-[var(--line)] bg-[var(--surface)] px-1.5 font-mono text-[10px]"
                  />
                </div>
              </div>
            ))}
          </div>
        </Section>
        )}

        <Section title={t("filter.contractValue")} icon={<ChevronDown size={12} />}>
          <RangeInput
            label={t("filter.value")}
            min={filters.valueMin}
            max={filters.valueMax}
            onMin={(valueMin) => patchFilters({ valueMin })}
            onMax={(valueMax) => patchFilters({ valueMax })}
            step={1000}
            placeholderMin="0"
            placeholderMax="∞"
          />
          <RangeInput
            label={t("filter.wage")}
            min={filters.wageMin}
            max={filters.wageMax}
            onMin={(wageMin) => patchFilters({ wageMin })}
            onMax={(wageMax) => patchFilters({ wageMax })}
            step={100}
            placeholderMin="0"
            placeholderMax="∞"
          />
          <div className="grid grid-cols-2 gap-1.5">
            <div className="space-y-1">
              <Label>{t("filter.expiresBefore")}</Label>
              <input
                type="date"
                value={filters.contractEndBefore ?? ""}
                onChange={(event) =>
                  patchFilters({ contractEndBefore: event.target.value || null })
                }
                className="h-7 w-full rounded-[6px] border border-[var(--line)] bg-[var(--surface)] px-1.5 text-[10px]"
              />
            </div>
            <div className="space-y-1">
              <Label>{t("filter.expiresAfter")}</Label>
              <input
                type="date"
                value={filters.contractEndAfter ?? ""}
                onChange={(event) =>
                  patchFilters({ contractEndAfter: event.target.value || null })
                }
                className="h-7 w-full rounded-[6px] border border-[var(--line)] bg-[var(--surface)] px-1.5 text-[10px]"
              />
            </div>
          </div>
        </Section>
      </div>

      {dataMode === "full" && (
      <div className="border-t border-[var(--line)] px-3 py-3">
        <div className="flex items-start justify-between gap-2">
          <div>
            <div className="text-[11px] font-semibold text-[var(--ink)]">
              {t("filter.hiddenAttributes")}
            </div>
            <div className="mt-0.5 text-[10px] leading-snug text-[var(--muted)]">
              {t("filter.hiddenWarning")}
            </div>
          </div>
          <Switch
            checked={hiddenEnabled}
            onChange={(enabled) => {
              if (!enabled) {
                setHiddenEnabled(false);
                return;
              }
              pushToast({
                title: t("toast.hiddenOn.title"),
                description: t("toast.hiddenOn.description"),
                tone: "default",
              });
              setHiddenEnabled(true);
            }}
            label={t("filter.hiddenAttributes")}
          />
        </div>
        {hiddenEnabled && (
          <div className="mt-2 flex items-center gap-1.5">
            <Badge tone="danger">{t("filter.soloOnly")}</Badge>
            <span className="text-[10px] text-[var(--muted)]">{t("filter.hiddenEnabled")}</span>
          </div>
        )}
      </div>
      )}
    </aside>
  );
}

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-2.5">
      <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--ink)]">
        <span className="text-[var(--muted)]">{icon}</span>
        {title}
      </div>
      {children}
    </section>
  );
}

function nextYear(date: string): string {
  const year = Number(date.slice(0, 4));
  return `${year + 1}${date.slice(4)}`;
}

function matchesFilters(filters: Filters, patch: Partial<Filters>): boolean {
  return (Object.keys(EMPTY_FILTERS) as (keyof Filters)[]).every((key) => {
    const expected = (key in patch ? patch[key] : EMPTY_FILTERS[key]) as unknown;
    const actual = filters[key] as unknown;
    if (Array.isArray(expected) || Array.isArray(actual)) {
      return JSON.stringify(actual) === JSON.stringify(expected);
    }
    return actual === expected;
  });
}
