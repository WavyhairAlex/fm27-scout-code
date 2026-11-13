import { useEffect, useState } from "react";
import {
  Database,
  Eye,
  FolderOpen,
  HardDrive,
  LockKeyhole,
  ShieldCheck,
  Sparkles,
  Upload,
  WifiOff,
} from "lucide-react";
import { isTauri } from "@/lib/api";
import { useSaves } from "@/lib/query";
import { relativeTime } from "@/lib/utils";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { useOpenSave } from "@/lib/openSave";
import { useAppStore } from "@/store/app";
import { Badge, Button } from "./ui";

export function Onboarding() {
  const { t, locale } = useI18n();
  const { chooseAndOpen, openPath } = useOpenSave();
  const { data: saves = [] } = useSaves();
  const parsing = useAppStore((state) => state.parsing);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(async ({ getCurrentWebview }) => {
      unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") setDragging(true);
        if (event.payload.type === "drop") {
          setDragging(false);
          const path = event.payload.paths.find((item) => item.toLowerCase().endsWith(".fm"));
          if (path) void openPath(path);
        }
        if (event.payload.type === "leave") setDragging(false);
      });
    });
    return () => unlisten?.();
  }, [openPath]);

  return (
    <div
      className="app-bg flex h-full min-w-0 flex-1 items-center justify-center overflow-auto px-8 py-10"
      onDragOver={(event) => {
        event.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(event) => {
        event.preventDefault();
        setDragging(false);
        const file = event.dataTransfer.files?.[0];
        if (file) {
          const path = (file as File & { path?: string }).path;
          if (path) void openPath(path);
        }
      }}
    >
      <div className="w-full max-w-[860px]">
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)] shadow-sm">
            <Sparkles size={22} />
          </div>
          <div>
            <h1 className="text-[26px] font-semibold tracking-tight text-[var(--ink)]">
              FM27 Scout
            </h1>
            <p className="text-xs text-[var(--muted)]">{t("onboarding.subtitle")}</p>
          </div>
          <div className="ml-auto flex items-center gap-1.5">
            <Badge tone="accent">
              <ShieldCheck size={10} /> {t("onboarding.readOnly")}
            </Badge>
            <Badge tone="outline">
              <WifiOff size={10} /> {t("onboarding.offline")}
            </Badge>
            <Badge tone="outline">
              <LockKeyhole size={10} /> {t("onboarding.localCache")}
            </Badge>
          </div>
        </div>

        <div
          className={`mt-8 rounded-3xl border-2 border-dashed bg-[var(--surface)]/80 p-8 text-center shadow-[var(--shadow)] transition-colors ${
            dragging
              ? "border-[var(--accent)] bg-[var(--accent-soft)]"
              : "border-[var(--line-strong)]"
          }`}
        >
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-3xl border border-[var(--line)] bg-[var(--surface-2)] text-[var(--accent)]">
            <Upload size={28} />
          </div>
          <h2 className="mt-4 text-lg font-semibold tracking-tight text-[var(--ink)]">
            {dragging ? t("onboarding.dropToOpen") : t("onboarding.openSave")}
          </h2>
          <p className="mx-auto mt-1.5 max-w-md text-xs leading-relaxed text-[var(--muted)]">
            {t("onboarding.description")}
          </p>
          <div className="mt-5 flex items-center justify-center gap-2">
            <Button variant="primary" size="lg" onClick={() => void chooseAndOpen()} disabled={parsing}>
              <FolderOpen size={16} />
              {t("onboarding.choose")}
            </Button>
            {saves[0] && (
              <Button variant="secondary" size="lg" onClick={() => void openPath(saves[0].path)} disabled={parsing}>
                <Database size={15} />
                {t("onboarding.openLast")}
              </Button>
            )}
          </div>
          <div className="mt-4 flex items-center justify-center gap-4 text-[10px] text-[var(--muted)]">
            <span className="inline-flex items-center gap-1">
              <HardDrive size={11} />
              {t("onboarding.autoDetect")}
            </span>
            <span className="inline-flex items-center gap-1">
              <Eye size={11} />
              {t("onboarding.hiddenAlwaysOn")}
            </span>
            <span className="inline-flex items-center gap-1">
              <WifiOff size={11} />
              {t("onboarding.fullyOffline")}
            </span>
          </div>
        </div>

        {saves.length > 0 && (
          <div className="mt-6">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                {t("onboarding.recentSaves")}
              </h3>
              <span className="text-[10px] text-[var(--muted)]">{t("onboarding.cachedHint")}</span>
            </div>
            <div className="grid gap-2">
              {saves.slice(0, 4).map((save) => (
                <button
                  key={save.id}
                  type="button"
                  onClick={() => void openPath(save.path)}
                  className="group flex items-center gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface)] px-3 py-2.5 text-left shadow-sm transition-colors hover:border-[var(--accent-line)] hover:bg-[var(--surface-2)]"
                >
                  <div className="flex h-9 w-9 items-center justify-center rounded-xl border border-[var(--line)] bg-[var(--surface-2)] text-[var(--accent)]">
                    <Database size={16} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[12px] font-semibold text-[var(--ink)]">
                      {save.file_name}
                    </div>
                    <div className="mt-0.5 flex items-center gap-2 text-[10px] text-[var(--muted)]">
                      <span>{save.build}</span>
                      <span>·</span>
                      <span>{save.game_date ?? t("common.unknownDate")}</span>
                      <span>·</span>
                      <span>
                        {t("common.playersCount", {
                          count: formatNumber(save.players, locale),
                        })}
                      </span>
                    </div>
                  </div>
                  <div className="text-right text-[9px] text-[var(--muted)]">
                    <div>{relativeTime(save.parsed_at, locale)}</div>
                    <div className="mt-0.5 font-mono">
                      {t("common.clubsCount", {
                        count: formatNumber(save.clubs, locale),
                      })}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        <p className="mt-6 text-center text-[10px] leading-relaxed text-[var(--muted)]">
          {t("onboarding.disclaimer")}
        </p>
      </div>
    </div>
  );
}
