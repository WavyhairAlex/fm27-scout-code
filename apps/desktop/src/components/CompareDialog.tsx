import { useMemo, useState } from "react";
import { useQueries } from "@tanstack/react-query";
import { GitCompareArrows, X } from "lucide-react";
import { getPlayer } from "@/lib/api";
import { ATTRIBUTE_GROUPS, type Player } from "@/lib/types";
import { attributeLabel } from "@/lib/labels";
import { useI18n } from "@/lib/useI18n";
import { caLabelClass, cn, paTone } from "@/lib/utils";
import { useAppStore } from "@/store/app";
import { Badge, Button, Modal } from "./ui";

export function CompareDialog() {
  const { t, locale, formatValue } = useI18n();
  const open = useAppStore((state) => state.compareOpen);
  const setOpen = useAppStore((state) => state.setCompareOpen);
  const compareUids = useAppStore((state) => state.compareUids);
  const toggleCompare = useAppStore((state) => state.toggleCompare);
  const clearCompare = useAppStore((state) => state.clearCompare);
  const dataMode = useAppStore((state) => state.dataMode);
  const displayMode = useAppStore((state) => state.displayMode);
  const saveId = useAppStore((state) => state.saveId);
  const [differencesOnly, setDifferencesOnly] = useState(false);
  const [hiddenOnly, setHiddenOnly] = useState(false);

  const results = useQueries({
    queries: compareUids.map((uid) => ({
      queryKey: ["player", saveId, uid],
      queryFn: () => getPlayer(saveId!, uid),
      enabled: open && saveId != null,
      staleTime: 60_000,
    })),
  });
  const players = results.map((result) => result.data).filter(Boolean) as Player[];

  const groups = useMemo(() => {
    if (dataMode === "moneyball") return [];
    const selected = hiddenOnly
      ? ATTRIBUTE_GROUPS.hidden
      : [...ATTRIBUTE_GROUPS.technical, ...ATTRIBUTE_GROUPS.hidden];
    return selected;
  }, [dataMode, hiddenOnly]);

  const values = (player: Player, key: string) =>
    displayMode === "display" ? player.attributes[key] ?? 0 : player.raw_attributes[key] ?? 0;
  const max = displayMode === "display" ? 20 : 100;

  return (
    <Modal
      open={open}
      onClose={() => setOpen(false)}
      title={
        <span className="inline-flex items-center gap-2">
          <GitCompareArrows size={16} className="text-[var(--gold)]" />
          {t("compare.title")}
        </span>
      }
      description={t("compare.description", { count: players.length })}
      width="max-w-[1040px]"
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={clearCompare}>
            {t("compare.clear")}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setOpen(false)}>
            {t("compare.close")}
          </Button>
        </>
      }
    >
      <div className="p-4">
        <div className="mb-3 flex items-center gap-3">
          <label className="flex cursor-pointer items-center gap-1.5 text-[11px] text-[var(--muted)]">
            <input
              type="checkbox"
              checked={differencesOnly}
              onChange={(event) => setDifferencesOnly(event.target.checked)}
              className="accent-[var(--accent)]"
            />
            {t("compare.differencesOnly")}
          </label>
          {dataMode !== "moneyball" && (
            <label className="flex cursor-pointer items-center gap-1.5 text-[11px] text-[var(--muted)]">
              <input
                type="checkbox"
                checked={hiddenOnly}
                onChange={(event) => setHiddenOnly(event.target.checked)}
                className="accent-[var(--accent)]"
              />
              {t("compare.hiddenOnly")}
            </label>
          )}
          <span className="ml-auto text-[10px] text-[var(--muted)]">
            {displayMode === "display"
              ? t("compare.displayValues")
              : t("compare.rawValues")}
          </span>
        </div>

        <div
          className="grid gap-2"
          style={{ gridTemplateColumns: `160px repeat(${Math.max(players.length, 1)}, minmax(0, 1fr))` }}
        >
          <div />
          {players.map((player) => (
            <div
              key={player.uid}
              className="relative rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5"
            >
              <button
                type="button"
                onClick={() => toggleCompare(player.uid)}
                className="absolute right-1.5 top-1.5 rounded p-0.5 text-[var(--muted)] hover:bg-[var(--surface-3)] hover:text-[var(--danger)]"
                aria-label={t("compare.remove")}
              >
                <X size={11} />
              </button>
              <div className="pr-3 text-[12px] font-semibold leading-tight text-[var(--ink)]">
                {player.name}
              </div>
              <div className="mt-0.5 truncate text-[10px] text-[var(--muted)]">
                {player.club_name ?? t("common.freeAgent")}
              </div>
              {dataMode === "full" && (
                <div className="mt-2 flex items-center gap-1">
                  <span className={cn("rounded border px-1.5 py-0.5 font-mono text-[10px] font-bold", caLabelClass)}>
                    CA {player.ca}
                  </span>
                  <span className={cn("rounded border px-1.5 py-0.5 font-mono text-[10px] font-bold", paTone(player.pa))}>
                    PA {player.pa ?? "?"}
                  </span>
                </div>
              )}
              <div className="mt-1.5 text-[9px] text-[var(--muted)]">
                {t("common.yearsOld", { age: player.age ?? "?" })} ·{" "}
                {player.natural_positions.join(" / ")} · {formatValue(player.transfer_value)}
              </div>
            </div>
          ))}
        </div>

        {dataMode === "moneyball" && (
          <div className="mt-4 rounded-lg border border-dashed border-[var(--line)] px-3 py-2 text-[10px] text-[var(--muted)]">
            {t("compare.moneyballHidden")}
          </div>
        )}

        <div className="mt-4 space-y-4">
          {groups.map((group) => {
            const rows = group.keys
              .map((key) => {
                const rowValues = players.map((player) => values(player, key));
                const best = Math.max(...rowValues);
                const differs = new Set(rowValues).size > 1;
                return { key, rowValues, best, differs };
              })
              .filter((row) => !differencesOnly || row.differs);
            if (rows.length === 0) return null;
            return (
              <div key={group.labelKey}>
                <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                  {t(`attribute.group.${group.labelKey}`)}
                </div>
                <div
                  className="grid gap-1"
                  style={{ gridTemplateColumns: `160px repeat(${Math.max(players.length, 1)}, minmax(0, 1fr))` }}
                >
                  {rows.map((row) => (
                    <div key={row.key} className="contents">
                      <div className="flex items-center rounded-[6px] bg-[var(--surface-2)] px-2 py-1 text-[10px] text-[var(--muted)]">
                        {attributeLabel(row.key, locale)}
                      </div>
                      {row.rowValues.map((value, index) => (
                        <div
                          key={index}
                          className={cn(
                            "flex items-center justify-center rounded-[6px] border px-2 py-1 font-mono text-[10px]",
                            value === row.best && value > 0
                              ? "border-[var(--gold-line)] bg-[var(--gold-soft)] font-bold text-[var(--gold)]"
                              : "border-[var(--line)] bg-[var(--surface)] text-[var(--ink)]",
                          )}
                        >
                          {value > 0 ? value : "—"}
                          {value === row.best && value > 0 && (
                            <span className="ml-1 text-[8px] uppercase opacity-60">
                              {t("compare.best")}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>

        {players.length < 2 && (
          <div className="mt-4 rounded-xl border border-dashed border-[var(--line)] px-4 py-8 text-center text-[11px] text-[var(--muted)]">
            {t("compare.empty")}
          </div>
        )}
      </div>
    </Modal>
  );
}
