import { BriefcaseBusiness, CalendarDays, Coins, Sparkles, UserRound } from "lucide-react";
import { useStaffDetail } from "@/lib/query";
import {
  STAFF_ATTRIBUTE_GROUPS,
  STAFF_ROLE_GREEN_THRESHOLD,
  STAFF_ROLE_KEYS,
  type StaffRow,
} from "@/lib/types";
import {
  nationLabel,
  personalityLabel,
  staffAttributeLabel,
  staffJobLabel,
  staffRoleLabel,
} from "@/lib/labels";
import { useI18n } from "@/lib/useI18n";
import { caLabelClass, cn, initials } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Badge, EmptyState } from "./ui";

export function StaffInspector() {
  const { t, locale, formatWage } = useI18n();
  const selectedUid = useAppStore((state) => state.selectedStaffUid);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const { data, isLoading } = useStaffDetail(selectedUid);
  const staff = data?.rows[0] ?? null;

  if (selectedUid == null) {
    return (
      <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]">
        <EmptyState
          icon={<UserRound size={20} />}
          title={t("staff.inspector.emptyTitle")}
          description={t("staff.inspector.emptyDescription")}
        />
      </aside>
    );
  }

  if (isLoading || !staff) {
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

  const roleRows = staff.roles
    ? STAFF_ROLE_KEYS.map((key, index) => ({
        key,
        score: staff.roles?.[index] ?? null,
        best: staff.best_role_index === index,
      })).sort((a, b) => (b.score ?? 0) - (a.score ?? 0))
    : [];

  return (
    <aside className="flex h-full w-[372px] shrink-0 flex-col border-l border-[var(--line)] bg-[var(--surface)]">
      <div className="flex items-center gap-2 border-b border-[var(--line)] px-3 py-2 text-[11px] font-semibold text-[var(--muted)]">
        <BriefcaseBusiness size={13} className="text-[var(--accent)]" />
        {t("staff.inspector.profile")}
      </div>

      <div className="thin-scroll flex-1 space-y-4 overflow-y-auto px-3 py-3">
        <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)]/60 p-3">
          <div className="flex items-start gap-2.5">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--surface)] text-[11px] font-bold text-[var(--muted)]">
              {initials(staff.name)}
            </div>
            <div className="min-w-0 flex-1">
              <div className="truncate text-[14px] font-semibold leading-tight text-[var(--ink)]">
                {staff.name ?? t("common.unnamedStaff")}
              </div>
              <div className="mt-0.5 text-[10px] text-[var(--muted)]">
                {t("common.yearsOld", { age: staff.age ?? "?" })} ·{" "}
                {nationLabel(staff.nation_name, locale) ??
                  (staff.nation_id != null
                    ? t("common.nationNumber", { id: staff.nation_id })
                    : t("common.unknownNation"))}
              </div>
              <div className="mt-1.5 flex flex-wrap items-center gap-1">
                <span className="rounded bg-[var(--accent-soft)] px-1.5 py-px text-[9px] font-semibold text-[var(--accent-strong)]">
                  {staffJobLabel(staff.job_code, staff.job_name, locale) ??
                    t("common.unknownJob")}
                </span>
                {!staff.has_contract && (
                  <span className="rounded bg-[var(--surface-3)] px-1.5 py-px text-[9px] font-semibold text-[var(--muted)]">
                    {t("staff.inspector.noContract")}
                  </span>
                )}
              </div>
            </div>
            <div className="flex shrink-0 flex-col items-end gap-1">
              <span
                className={cn(
                  "inline-flex h-5 min-w-9 items-center justify-center rounded border px-1.5 font-mono text-[10px] font-bold",
                  caLabelClass,
                )}
                title="CA"
              >
                {staff.ca ?? "—"}
              </span>
              <span className="font-mono text-[10px] font-semibold text-[var(--muted)]" title="PA">
                PA {staff.pa ?? "—"}
              </span>
            </div>
          </div>

          <div className="mt-3 grid grid-cols-2 gap-2 border-t border-[var(--line)] pt-2.5 text-[10px]">
            <div className="min-w-0">
              <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
                {t("staff.inspector.contract")}
              </div>
              <div className="mt-0.5 truncate font-medium text-[var(--ink)]">
                {staff.club_uid != null ? (
                  <button
                    type="button"
                    className="truncate text-left hover:text-[var(--accent-strong)] hover:underline"
                    onClick={() => {
                      setSelectedClubUid(staff.club_uid);
                      setViewMode("clubs");
                    }}
                  >
                    {staff.club_name ?? t("common.unknownClub")}
                  </button>
                ) : (
                  t("common.unknownClub")
                )}
              </div>
            </div>
            <div className="min-w-0">
              <div className="text-[9px] uppercase tracking-wider text-[var(--muted)]">
                {t("staff.inspector.weeklyWage")}
              </div>
              <div className="mt-0.5 flex items-center gap-1 font-mono font-medium text-[var(--ink)]">
                <Coins size={10} className="text-[var(--muted)]" />
                {staff.wage != null ? formatWage(staff.wage) : "—"}
              </div>
            </div>
          </div>
        </div>

        <section className="space-y-2">
          <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--ink)]">
            <Sparkles size={12} className="text-[var(--muted)]" />
            {t("staff.inspector.attributes")}
          </div>
          {staff.attributes ? (
            <div className="space-y-2.5">
              {STAFF_ATTRIBUTE_GROUPS.map((group) => (
                <div key={group.labelKey}>
                  <div className="mb-1 text-[9px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                    {t(group.labelKey)}
                  </div>
                  <div className="grid grid-cols-2 gap-1">
                    {group.keys.map((key) => (
                      <AttributeCell
                        key={key}
                        label={staffAttributeLabel(key, locale)}
                        value={staff.attributes?.[key]}
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-[10px] text-[var(--muted)]">{t("staff.inspector.noAttributes")}</p>
          )}
        </section>

        <section className="space-y-2">
          <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--ink)]">
            <BriefcaseBusiness size={12} className="text-[var(--muted)]" />
            {t("staff.inspector.roles")}
          </div>
          {staff.roles ? (
            <>
              <div className="space-y-1">
                {roleRows.map((role) => (
                  <RoleBar
                    key={role.key}
                    label={staffRoleLabel(role.key, null, locale) ?? role.key}
                    score={role.score}
                    best={role.best}
                  />
                ))}
              </div>
              <p className="text-[9px] leading-snug text-[var(--muted)]">
                {t("staff.inspector.roleHint")}
              </p>
            </>
          ) : (
            <p className="text-[10px] text-[var(--muted)]">{t("staff.inspector.noRoles")}</p>
          )}
        </section>

        <ContractSection staff={staff} />
        {staff.personality && <PersonalitySection staff={staff} />}
      </div>
    </aside>
  );
}

function AttributeCell({ label, value }: { label: string; value: number | undefined }) {
  return (
    <div className="flex items-center justify-between gap-1 rounded-[5px] border border-[var(--line)] bg-[var(--surface-2)] px-1.5 py-1">
      <span className="min-w-0 truncate text-[9px] text-[var(--muted)]" title={label}>
        {label}
      </span>
      <span className="shrink-0 font-mono text-[10px] font-semibold text-[var(--ink)]">
        {value ?? "—"}
      </span>
    </div>
  );
}

function RoleBar({
  label,
  score,
  best,
}: {
  label: string;
  score: number | null;
  best: boolean;
}) {
  const width = `${Math.max(0, Math.min(20, score ?? 0)) * 5}%`;
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-[5px] border px-1.5 py-1",
        best
          ? "border-[var(--accent-line)] bg-[var(--accent-soft)]"
          : "border-[var(--line)] bg-[var(--surface-2)]",
      )}
    >
      <div
        className={cn("absolute inset-y-0 left-0 opacity-20", best ? "bg-[var(--accent)]" : "bg-[var(--muted)]")}
        style={{ width }}
      />
      <div className="relative flex items-center justify-between gap-2">
        <span className="min-w-0 truncate text-[10px] font-medium text-[var(--ink)]">{label}</span>
        <span
          className={cn(
            "shrink-0 font-mono text-[10px] font-semibold",
            (score ?? 0) >= STAFF_ROLE_GREEN_THRESHOLD
              ? "text-[var(--accent-strong)]"
              : "text-[var(--muted)]",
          )}
        >
          {score ?? "—"}
        </span>
      </div>
    </div>
  );
}

function ContractSection({ staff }: { staff: StaffRow }) {
  const { t, formatWage } = useI18n();
  return (
    <section className="space-y-2">
      <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--ink)]">
        <CalendarDays size={12} className="text-[var(--muted)]" />
        {t("staff.inspector.contract")}
      </div>
      {staff.has_contract ? (
        <div className="space-y-1">
          <KeyValue label={t("staff.inspector.weeklyWage")} value={staff.wage != null ? formatWage(staff.wage) : "—"} />
          <KeyValue label={t("staff.inspector.start")} value={staff.contract_start ?? "—"} />
          <KeyValue label={t("staff.inspector.end")} value={staff.contract_end ?? "—"} />
        </div>
      ) : (
        <Badge tone="default">{t("staff.inspector.noContract")}</Badge>
      )}
    </section>
  );
}

function PersonalitySection({ staff }: { staff: StaffRow }) {
  const { t, locale } = useI18n();
  if (!staff.personality) return null;
  return (
    <section className="space-y-2">
      <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--ink)]">
        <UserRound size={12} className="text-[var(--muted)]" />
        {t("staff.inspector.personality")}
      </div>
      <div className="grid grid-cols-2 gap-1">
        {Object.entries(staff.personality).map(([key, value]) => (
          <AttributeCell key={key} label={personalityLabel(key, locale)} value={value} />
        ))}
      </div>
    </section>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-2 rounded-[5px] border border-[var(--line)] bg-[var(--surface-2)] px-1.5 py-1">
      <span className="text-[9px] text-[var(--muted)]">{label}</span>
      <span className="font-mono text-[10px] font-medium text-[var(--ink)]">{value}</span>
    </div>
  );
}
