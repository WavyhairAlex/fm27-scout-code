import { useEffect, useRef, useState } from "react";
import { RotateCcw, Search } from "lucide-react";
import { nationLabel } from "@/lib/labels";
import { useClubFilterOptions } from "@/lib/query";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";
import { Button, Input, Select } from "./ui";

export function ClubFilterBar() {
  const { t, locale } = useI18n();
  const filters = useAppStore((state) => state.clubFilters);
  const patch = useAppStore((state) => state.patchClubFilters);
  const reset = useAppStore((state) => state.resetClubFilters);
  const options = useClubFilterOptions();
  const [search, setSearch] = useState(filters.search);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    setSearch(filters.search);
  }, [filters.search]);

  useEffect(() => {
    return () => {
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, []);

  const updateSearch = (value: string) => {
    setSearch(value);
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => patch({ search: value }), 180);
  };

  return (
    <div className="flex flex-wrap items-end gap-2 border-b border-[var(--line)] bg-[var(--surface)] px-3 py-2">
      <div className="relative min-w-[240px] flex-1">
        <Search
          size={13}
          className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted)]"
        />
        <Input
          id="club-search"
          value={search}
          onChange={(event) => updateSearch(event.target.value)}
          placeholder={t("clubs.searchPlaceholder")}
          className="h-8 pl-8"
        />
      </div>
      <label className="space-y-0.5">
        <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
          {t("clubs.filter.nation")}
        </span>
        <Select
          value={filters.nationIds[0] ?? ""}
          onChange={(event) =>
            patch({ nationIds: event.target.value ? [Number(event.target.value)] : [] })
          }
          className="h-8 w-[150px]"
        >
          <option value="">{t("clubs.filter.any")}</option>
          {(options.data?.nations ?? []).map((nation) => (
            <option key={nation.id} value={nation.id}>
              {nationLabel(nation.name, locale) ?? nation.name} ({nation.count})
            </option>
          ))}
        </Select>
      </label>
      <NumberFilter
        label={t("clubs.filter.reputationMin")}
        value={filters.reputationMin}
        onChange={(value) => patch({ reputationMin: value })}
      />
      <NumberFilter
        label={t("clubs.filter.reputationMax")}
        value={filters.reputationMax}
        onChange={(value) => patch({ reputationMax: value })}
      />
      <NumberFilter
        label={t("clubs.filter.balanceMin")}
        value={filters.balanceMin}
        onChange={(value) => patch({ balanceMin: value })}
      />
      <NumberFilter
        label={t("clubs.filter.transferBudgetMin")}
        value={filters.transferBudgetMin}
        onChange={(value) => patch({ transferBudgetMin: value })}
      />
      <NumberFilter
        label={t("clubs.filter.wageBudgetMin")}
        value={filters.wageBudgetMin}
        onChange={(value) => patch({ wageBudgetMin: value })}
      />
      <NumberFilter
        label={t("clubs.filter.corporateFacilitiesMin")}
        value={filters.corporateFacilitiesMin}
        onChange={(value) => patch({ corporateFacilitiesMin: value })}
      />
      <label className="space-y-0.5">
        <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
          {t("clubs.filter.finances")}
        </span>
        <Select
          value={filters.hasFinances == null ? "" : String(filters.hasFinances)}
          onChange={(event) =>
            patch({
              hasFinances: event.target.value === "" ? null : event.target.value === "true",
            })
          }
          className="h-8 w-[130px]"
        >
          <option value="">{t("clubs.filter.any")}</option>
          <option value="true">{t("clubs.filter.withFinance")}</option>
          <option value="false">{t("clubs.filter.withoutFinance")}</option>
        </Select>
      </label>
      <label className="space-y-0.5">
        <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
          {t("clubs.filter.stadium")}
        </span>
        <Select
          value={filters.hasStadium == null ? "" : String(filters.hasStadium)}
          onChange={(event) =>
            patch({
              hasStadium: event.target.value === "" ? null : event.target.value === "true",
            })
          }
          className="h-8 w-[130px]"
        >
          <option value="">{t("clubs.filter.any")}</option>
          <option value="true">{t("clubs.filter.withStadium")}</option>
          <option value="false">{t("clubs.filter.withoutStadium")}</option>
        </Select>
      </label>
      <Button variant="ghost" size="sm" className="h-8" onClick={reset}>
        <RotateCcw size={12} />
        {t("clubs.reset")}
      </Button>
    </div>
  );
}

function NumberFilter({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number | null;
  onChange: (value: number | null) => void;
}) {
  return (
    <label className="space-y-0.5">
      <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
        {label}
      </span>
      <Input
        type="number"
        value={value ?? ""}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          onChange(event.target.value === "" || !Number.isFinite(parsed) ? null : parsed);
        }}
        className="h-8 w-[100px] font-mono"
      />
    </label>
  );
}
