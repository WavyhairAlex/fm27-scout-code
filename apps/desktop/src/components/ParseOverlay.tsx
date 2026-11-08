import { Loader2, ScanSearch } from "lucide-react";
import type { TranslationKey } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";

const STAGE_KEYS: Record<string, TranslationKey> = {
  snapshot: "parse.stage.snapshot",
  opening: "parse.stage.opening",
  game_db: "parse.stage.game_db",
  names: "parse.stage.names",
  players: "parse.stage.players",
  clubs: "parse.stage.clubs",
  competitions: "parse.stage.competitions",
  decode: "parse.stage.decode",
  staff: "parse.stage.staff",
  season_stats: "parse.stage.season_stats",
  done: "parse.stage.done",
};

const STAGE_ORDER = [
  "snapshot",
  "opening",
  "game_db",
  "names",
  "players",
  "clubs",
  "competitions",
  "decode",
  "staff",
  "season_stats",
];

export function ParseOverlay() {
  const parsing = useAppStore((state) => state.parsing);
  const progress = useAppStore((state) => state.parseProgress);
  const { t } = useI18n();
  if (!parsing) return null;
  const total = progress?.total ?? 0;
  const current = progress?.current ?? 0;
  const percent = total > 0 ? Math.min(100, (current / total) * 100) : null;
  const activeStage = progress?.stage ?? "opening";
  const activeIndex = STAGE_ORDER.indexOf(activeStage);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/30 backdrop-blur-[3px] dark:bg-black/50">
      <div className="w-[420px] rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-5 shadow-[var(--shadow-pop)]">
        <div className="flex items-center gap-3">
          <div className="relative flex h-10 w-10 items-center justify-center rounded-xl border border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]">
            <ScanSearch size={20} />
            <Loader2 size={12} className="absolute -right-1 -top-1 animate-spin text-[var(--gold)]" />
          </div>
          <div>
            <div className="text-sm font-semibold text-[var(--ink)]">{t("parse.title")}</div>
            <div className="mt-0.5 text-[10px] text-[var(--muted)]">
              {activeStage === "decode" && total > 0
                ? t("parse.decoding", { current, total })
                : t(STAGE_KEYS[activeStage] ?? "parse.preparing")}
            </div>
          </div>
        </div>

        <div className="mt-4 h-1.5 overflow-hidden rounded-full bg-[var(--surface-3)]">
          <div
            className={`h-full rounded-full bg-[var(--accent)] transition-all duration-300 ${
              percent == null ? "w-1/3 animate-pulse" : ""
            }`}
            style={percent == null ? undefined : { width: `${Math.max(3, percent)}%` }}
          />
        </div>
        {total > 0 && (
          <div className="mt-1 text-right font-mono text-[9px] text-[var(--muted)]">
            {current.toLocaleString("en-US")} / {total.toLocaleString("en-US")}
          </div>
        )}

        <div className="mt-4 flex flex-wrap gap-1">
          {STAGE_ORDER.map((stage, index) => (
            <span
              key={stage}
              className={`rounded-full border px-2 py-0.5 text-[9px] ${
                stage === activeStage
                  ? "border-[var(--accent)] bg-[var(--accent)] text-white"
                  : index < activeIndex
                    ? "border-[var(--success-line)] bg-[var(--success-soft)] text-[var(--success)]"
                    : "border-[var(--line)] text-[var(--muted)]"
              }`}
            >
              {t(STAGE_KEYS[stage] ?? "parse.preparing")}
            </span>
          ))}
        </div>
        <p className="mt-3 text-[9px] leading-relaxed text-[var(--muted)]">
          {t("parse.note")}
        </p>
      </div>
    </div>
  );
}
