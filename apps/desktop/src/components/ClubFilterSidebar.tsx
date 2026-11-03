import { useMemo } from "react";
import { Building2, Filter, RotateCcw, SlidersHorizontal } from "lucide-react";
import { nationLabel } from "@/lib/labels";
import { useClubFilterOptions } from "@/lib/query";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";
import { Button, Input, Label, Select } from "./ui";
import { MultiSelect } from "./MultiSelect";
import { RangeInput } from "./RangeInput";

export function ClubFilterSidebar() {
  const { t, locale } = useI18n();
  const filters = useAppStore((state) => state.clubFilters);
  const patch = useAppStore((state) => state.patchClubFilters);
  const reset = useAppStore((state) => state.resetClubFilters);
  const options = useClubFilterOptions();

  const nationOptions = useMemo(
    () =>
      (options.data?.nations ?? []).map((option) => ({
        ...option,
        name: nationLabel(option.name, locale) ?? option.name,
      })),
    [locale, options.data?.nations],
  );

  const activeCount =
    (filters.search ? 1 : 0) +
    (filters.nationIds.length ? 1 : 0) +
    (filters.genders.length ? 1 : 0) +
    (filters.reputationMin != null || filters.reputationMax != null ? 1 : 0) +
    (filters.hasFinances != null ? 1 : 0) +
    (filters.balanceMin != null ? 1 : 0) +
    (filters.transferBudgetMin != null ? 1 : 0) +
    (filters.wageBudgetMin != null ? 1 : 0) +
    (filters.corporateFacilitiesMin != null ? 1 : 0) +
    (filters.hasStadium != null ? 1 : 0);

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
          onClick={reset}
          disabled={activeCount === 0}
          className="h-6 px-1.5 text-[10px]"
        >
          <RotateCcw size={11} />
          {t("clubs.reset")}
        </Button>
      </div>

      <div className="thin-scroll flex-1 space-y-5 overflow-y-auto px-3 py-3.5">
        <section className="space-y-1.5">
          <Label>{t("clubs.filter.nation")}</Label>
          <MultiSelect
            options={nationOptions}
            selected={filters.nationIds}
            onChange={(nationIds) => patch({ nationIds })}
            placeholder={t("filter.allNations")}
            maxLabel={2}
          />
        </section>

        <Section title={t("clubs.filter.gender")} icon={<SlidersHorizontal size={12} />}>
          <div className="grid grid-cols-3 gap-1">
            {[
              { ids: [] as number[], label: t("filter.allGenders") },
              { ids: [0], label: t("filter.men") },
              { ids: [1], label: t("filter.women") },
            ].map((option) => {
              const active =
                option.ids.length === 0
                  ? filters.genders.length === 0
                  : filters.genders.length === option.ids.length &&
                    option.ids.every((id) => filters.genders.includes(id));
              const count =
                option.ids.length === 0
                  ? options.data?.genders.reduce((sum, item) => sum + item.count, 0)
                  : options.data?.genders
                      .filter((item) => option.ids.includes(item.id))
                      .reduce((sum, item) => sum + item.count, 0);
              return (
                <button
                  key={option.label}
                  type="button"
                  onClick={() => patch({ genders: option.ids })}
                  className={
                    active
                      ? "flex items-center justify-between rounded-[6px] border border-[var(--accent)] bg-[var(--accent-soft)] px-2 py-1.5 text-[10px] font-semibold text-[var(--accent-strong)]"
                      : "flex items-center justify-between rounded-[6px] border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1.5 text-[10px] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]"
                  }
                >
                  <span className="truncate">{option.label}</span>
                  {count != null && (
                    <span className="font-mono text-[9px] opacity-70">{count}</span>
                  )}
                </button>
              );
            })}
          </div>
        </Section>

        <RangeInput
          label={t("clubs.table.reputation")}
          min={filters.reputationMin}
          max={filters.reputationMax}
          onMin={(reputationMin) => patch({ reputationMin })}
          onMax={(reputationMax) => patch({ reputationMax })}
          placeholderMin="0"
          placeholderMax="10000"
        />

        <Section title={t("clubs.filter.finances")} icon={<SlidersHorizontal size={12} />}>
          <Select
            value={filters.hasFinances == null ? "" : String(filters.hasFinances)}
            onChange={(event) =>
              patch({
                hasFinances: event.target.value === "" ? null : event.target.value === "true",
              })
            }
            className="h-8"
          >
            <option value="">{t("clubs.filter.any")}</option>
            <option value="true">{t("clubs.filter.withFinance")}</option>
            <option value="false">{t("clubs.filter.withoutFinance")}</option>
          </Select>
          <NumberFilter
            label={t("clubs.filter.balanceMin")}
            value={filters.balanceMin}
            onChange={(balanceMin) => patch({ balanceMin })}
            step={1000}
          />
          <NumberFilter
            label={t("clubs.filter.transferBudgetMin")}
            value={filters.transferBudgetMin}
            onChange={(transferBudgetMin) => patch({ transferBudgetMin })}
            step={1000}
          />
          <NumberFilter
            label={t("clubs.filter.wageBudgetMin")}
            value={filters.wageBudgetMin}
            onChange={(wageBudgetMin) => patch({ wageBudgetMin })}
            step={100}
          />
        </Section>

        <Section title={t("clubs.inspector.facilities")} icon={<Building2 size={12} />}>
          <NumberFilter
            label={t("clubs.filter.corporateFacilitiesMin")}
            value={filters.corporateFacilitiesMin}
            onChange={(corporateFacilitiesMin) => patch({ corporateFacilitiesMin })}
            min={1}
            max={20}
          />
          <div className="space-y-1.5">
            <Label>{t("clubs.filter.stadium")}</Label>
            <Select
              value={filters.hasStadium == null ? "" : String(filters.hasStadium)}
              onChange={(event) =>
                patch({
                  hasStadium: event.target.value === "" ? null : event.target.value === "true",
                })
              }
              className="h-8"
            >
              <option value="">{t("clubs.filter.any")}</option>
              <option value="true">{t("clubs.filter.withStadium")}</option>
              <option value="false">{t("clubs.filter.withoutStadium")}</option>
            </Select>
          </div>
        </Section>
      </div>
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

function NumberFilter({
  label,
  value,
  onChange,
  step = 1,
  min,
  max,
}: {
  label: string;
  value: number | null;
  onChange: (value: number | null) => void;
  step?: number;
  min?: number;
  max?: number;
}) {
  return (
    <label className="block space-y-1.5">
      <Label>{label}</Label>
      <Input
        type="number"
        inputMode="numeric"
        step={step}
        min={min}
        max={max}
        value={value ?? ""}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          onChange(event.target.value === "" || !Number.isFinite(parsed) ? null : parsed);
        }}
        className="h-8 font-mono text-[11px]"
      />
    </label>
  );
}
