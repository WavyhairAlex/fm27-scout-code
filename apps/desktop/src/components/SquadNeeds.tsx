import { useSquadNeeds } from "@/lib/query";
import { useI18n } from "@/lib/useI18n";

/**
 * Weakest position groups in a squad plus same-tier upgrades. Used by the
 * analysis centre's Needs tab and by the club inspector's squad tab.
 */
export function SquadNeedsSection({
  clubUid,
  onPlayer,
  showEmpty = false,
}: {
  clubUid: number | null;
  onPlayer: (uid: number) => void;
  /** Show hints instead of rendering nothing when there is no data yet. */
  showEmpty?: boolean;
}) {
  const { t, formatValue } = useI18n();
  const needs = useSquadNeeds(clubUid);

  if (clubUid == null) {
    return showEmpty ? (
      <div className="rounded-xl border border-dashed border-[var(--line)] px-3 py-6 text-center text-[10px] text-[var(--muted)]">
        {t("analysis.chooseFirst")}
      </div>
    ) : null;
  }
  if (needs.isLoading) {
    return (
      <div className="rounded-xl border border-dashed border-[var(--line)] px-3 py-6 text-center text-[10px] text-[var(--muted)]">
        {t("analysis.loading")}
      </div>
    );
  }
  if (needs.isError) {
    return (
      <div className="rounded-xl border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-4 text-[10px] text-[var(--danger)]">
        {t("analysis.loadError")}
      </div>
    );
  }
  const groups = needs.data?.needs.slice(0, 6) ?? [];
  if (groups.length === 0) {
    return showEmpty ? (
      <div className="rounded-xl border border-dashed border-[var(--line)] px-3 py-6 text-center text-[10px] text-[var(--muted)]">
        {t("analysis.squadNeedsEmpty")}
      </div>
    ) : null;
  }

  return (
    <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-[11px] font-semibold text-[var(--ink)]">
          {t("analysis.squadNeeds")}
        </div>
        <span className="text-[9px] text-[var(--muted)]">{t("analysis.squadNeedsHint")}</span>
      </div>
      <div className="mt-2 space-y-2">
        {groups.map((need) => (
          <div
            key={need.position_group}
            className="rounded-lg border border-[var(--line)] bg-[var(--surface)] p-2"
          >
            <div className="flex items-center justify-between gap-2 text-[10px]">
              <span className="truncate font-semibold text-[var(--ink)]">
                {need.position_group}
                <span className="font-normal text-[var(--muted)]">
                  {" "}
                  · {need.best_name ?? "—"}
                </span>
              </span>
              <span className="shrink-0 font-mono text-[var(--muted)]">
                {t("analysis.squadNeedsCurrent")} {Math.round(need.best_quality ?? 0)}
              </span>
            </div>
            {need.upgrades.length === 0 ? (
              <div className="mt-1 text-[9px] text-[var(--muted)]">
                {t("analysis.squadNeedsNone")}
              </div>
            ) : (
              <div className="mt-1 grid gap-1 md:grid-cols-3">
                {need.upgrades.map((upgrade) => (
                  <button
                    key={upgrade.uid}
                    type="button"
                    onClick={() => onPlayer(upgrade.uid)}
                    className="rounded border border-[var(--line)] bg-[var(--surface-2)] px-2 py-1 text-left hover:border-[var(--accent-line)] hover:bg-[var(--accent-soft)]"
                  >
                    <div className="truncate text-[10px] text-[var(--ink)]">
                      {upgrade.name ?? t("common.unnamed")}
                      <span className="text-[var(--muted)]">
                        {" "}
                        · {upgrade.club_name ?? t("common.freeAgent")}
                      </span>
                    </div>
                    <div className="truncate font-mono text-[9px] text-[var(--muted)]">
                      Q {Math.round(upgrade.quality ?? 0)}
                      {upgrade.quality_gain != null && (
                        <span className="text-[var(--success)]">
                          {" "}
                          +{Math.round(upgrade.quality_gain)}
                        </span>
                      )}
                      {upgrade.fee != null && <> · {formatValue(upgrade.fee)}</>}
                      {upgrade.value_gap_pct != null && (
                        <span
                          className={
                            upgrade.value_gap_pct >= 0
                              ? "text-[var(--success)]"
                              : "text-[var(--danger)]"
                          }
                        >
                          {" "}
                          {upgrade.value_gap_pct >= 0 ? "+" : ""}
                          {Math.round(upgrade.value_gap_pct)}%
                        </span>
                      )}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
