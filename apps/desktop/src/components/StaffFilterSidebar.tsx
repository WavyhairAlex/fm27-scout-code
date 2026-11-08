import { useMemo } from "react";
import { BriefcaseBusiness, Filter, Plus, RotateCcw, SlidersHorizontal, X } from "lucide-react";
import { useStaffFilterOptions } from "@/lib/query";
import { STAFF_ATTRIBUTE_GROUPS, STAFF_ROLE_KEYS, STAFF_ROLE_GREEN_THRESHOLD } from "@/lib/types";
import { nationLabel, staffAttributeLabel, staffJobLabel, staffRoleLabel } from "@/lib/labels";
import { useI18n } from "@/lib/useI18n";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Button, Label, Select } from "./ui";
import { MultiSelect } from "./MultiSelect";
import { RangeInput } from "./RangeInput";

export function StaffFilterSidebar() {
  const { t, locale } = useI18n();
  const filters = useAppStore((state) => state.staffFilters);
  const patchFilters = useAppStore((state) => state.patchStaffFilters);
  const resetFilters = useAppStore((state) => state.resetStaffFilters);
  const addAttribute = useAppStore((state) => state.addStaffAttribute);
  const updateAttribute = useAppStore((state) => state.updateStaffAttribute);
  const removeAttribute = useAppStore((state) => state.removeStaffAttribute);
  const addRole = useAppStore((state) => state.addStaffRole);
  const updateRole = useAppStore((state) => state.updateStaffRole);
  const removeRole = useAppStore((state) => state.removeStaffRole);
  const { data: options } = useStaffFilterOptions();

  const activeCount = useMemo(() => {
    let count = 0;
    if (filters.search) count++;
    if (filters.clubUids.length) count++;
    if (filters.nationIds.length) count++;
    if (filters.jobCodes.length) count++;
    if (filters.ageMin != null || filters.ageMax != null) count++;
    if (filters.caMin != null || filters.caMax != null) count++;
    if (filters.paMin != null || filters.paMax != null) count++;
    if (filters.wageMin != null || filters.wageMax != null) count++;
    if (filters.contractEndBefore || filters.contractEndAfter) count++;
    if (filters.hasContract != null) count++;
    count += filters.attributes.length;
    count += filters.roles.length;
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

  const jobOptions = useMemo(
    () =>
      (options?.jobs ?? []).map((option) => ({
        ...option,
        name: staffJobLabel(option.id, option.name, locale) ?? option.name,
      })),
    [locale, options?.jobs],
  );

  const attributeGroups = useMemo(
    () =>
      STAFF_ATTRIBUTE_GROUPS.map((group) => ({
        labelKey: group.labelKey,
        options: group.keys
          .map((key) => [key, staffAttributeLabel(key, locale)] as const)
          .sort((a, b) => a[1].localeCompare(b[1], locale)),
      })),
    [locale],
  );

  const roleOptions = useMemo(
    () =>
      STAFF_ROLE_KEYS.map((key) => ({
        key,
        label: staffRoleLabel(key, null, locale) ?? key,
      })),
    [locale],
  );

  return (
    <aside className="flex h-full w-[252px] shrink-0 flex-col border-r border-[var(--line)] bg-[var(--surface)]/80">
      <div className="flex items-center justify-between border-b border-[var(--line)] px-3 py-2.5">
        <div className="flex items-center gap-2">
          <Filter size={14} className="text-[var(--accent)]" />
          <span className="text-xs font-semibold tracking-wide text-[var(--ink)]">
            {t("staff.filter.title")}
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

        <section className="space-y-1.5">
          <Label>{t("staff.filter.job")}</Label>
          <MultiSelect
            options={jobOptions}
            selected={filters.jobCodes}
            onChange={(jobCodes) => patchFilters({ jobCodes })}
            placeholder={t("staff.filter.allJobs")}
            maxLabel={2}
          />
        </section>

        <Section title={t("staff.filter.contract")} icon={<BriefcaseBusiness size={12} />}>
          <div className="grid grid-cols-1 gap-1">
            {[
              { value: null, label: t("staff.filter.anyContract"), count: options ? options.with_contract + options.without_contract : null },
              { value: true, label: t("staff.filter.withContract"), count: options?.with_contract ?? null },
              { value: false, label: t("staff.filter.withoutContract"), count: options?.without_contract ?? null },
            ].map((option) => {
              const active = filters.hasContract === option.value;
              return (
                <button
                  key={String(option.value)}
                  type="button"
                  onClick={() => patchFilters({ hasContract: option.value })}
                  className={cn(
                    "flex items-center justify-between rounded-[6px] border px-2 py-1.5 text-[10px] transition-colors",
                    active
                      ? "border-[var(--accent)] bg-[var(--accent-soft)] font-semibold text-[var(--accent-strong)]"
                      : "border-[var(--line)] bg-[var(--surface-2)] text-[var(--muted)] hover:border-[var(--line-strong)] hover:text-[var(--ink)]",
                  )}
                >
                  <span>{option.label}</span>
                  {option.count != null && (
                    <span className="font-mono text-[9px] opacity-70">{option.count}</span>
                  )}
                </button>
              );
            })}
          </div>
        </Section>

        <Section title={t("filter.ageAbility")} icon={<SlidersHorizontal size={12} />}>
          <RangeInput
            label={t("filter.age")}
            min={filters.ageMin}
            max={filters.ageMax}
            onMin={(ageMin) => patchFilters({ ageMin })}
            onMax={(ageMax) => patchFilters({ ageMax })}
            placeholderMin="20"
            placeholderMax="70"
          />
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
        </Section>

        <Section title={t("filter.contractValue")} icon={<SlidersHorizontal size={12} />}>
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

        <Section title={t("staff.filter.roleRules")} icon={<BriefcaseBusiness size={12} />}>
          <Select
            value=""
            onChange={(event) => {
              if (event.target.value) addRole(event.target.value);
            }}
            className="h-7 text-[11px]"
          >
            <option value="">{t("staff.filter.addRole")}</option>
            {roleOptions.map((role) => (
              <option key={role.key} value={role.key}>
                {role.label}
              </option>
            ))}
          </Select>
          <p className="text-[10px] leading-snug text-[var(--muted)]">
            {t("staff.filter.roleHint")}
          </p>
          <div className="space-y-1.5">
            {filters.roles.map((rule) => (
              <div
                key={rule.id}
                className="rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] p-2"
              >
                <div className="mb-1.5 flex items-center justify-between">
                  <span className="text-[11px] font-medium text-[var(--ink)]">
                    {staffRoleLabel(rule.key, null, locale)}
                  </span>
                  <button
                    type="button"
                    onClick={() => removeRole(rule.id)}
                    className="rounded p-0.5 text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--danger)]"
                    aria-label={t("staff.filter.removeRole")}
                  >
                    <X size={11} />
                  </button>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[10px] text-[var(--muted)]">
                    {t("staff.filter.minScore")}
                  </span>
                  <div className="flex items-center gap-1.5">
                    <input
                      type="range"
                      min={1}
                      max={20}
                      value={rule.min ?? STAFF_ROLE_GREEN_THRESHOLD}
                      onChange={(event) =>
                        updateRole(rule.id, { min: Number(event.target.value) })
                      }
                      className="h-1 w-24 accent-[var(--accent)]"
                    />
                    <span className="w-5 text-right font-mono text-[10px] text-[var(--ink)]">
                      {rule.min ?? STAFF_ROLE_GREEN_THRESHOLD}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </Section>

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
                    {staffAttributeLabel(rule.key, locale)}
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
