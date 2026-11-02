import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  Database,
  Download,
  Eye,
  Info,
  Languages,
  LockKeyhole,
  Monitor,
  Moon,
  RefreshCw,
  RotateCw,
  Sun,
  Trash2,
} from "lucide-react";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { appInfo, deleteSave, updaterInfo, type UpdaterInfo } from "@/lib/api";
import { useSaves } from "@/lib/query";
import { formatNumber } from "@/lib/i18n";
import { LANGUAGE_OPTIONS, type Language } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { relativeTime } from "@/lib/utils";
import { useAppStore, type DisplayMode, type Theme } from "@/store/app";
import { Badge, Button, Modal, Select, Switch } from "./ui";

type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; version: string; notes: string }
  | { kind: "downloading"; version: string; downloaded: number; total: number }
  | { kind: "ready"; version: string }
  | { kind: "error"; message: string };

function updateErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function SettingsDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.settingsOpen);
  const setOpen = useAppStore((state) => state.setSettingsOpen);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const setHiddenEnabled = useAppStore((state) => state.setHiddenEnabled);
  const displayMode = useAppStore((state) => state.displayMode);
  const setDisplayMode = useAppStore((state) => state.setDisplayMode);
  const theme = useAppStore((state) => state.theme);
  const setTheme = useAppStore((state) => state.setTheme);
  const language = useAppStore((state) => state.language);
  const setLanguage = useAppStore((state) => state.setLanguage);
  const pushToast = useAppStore((state) => state.pushToast);
  const currentSaveId = useAppStore((state) => state.saveId);
  const clearSession = useAppStore((state) => state.clearSession);
  const [dataDir, setDataDir] = useState("");
  const [info, setInfo] = useState<Awaited<ReturnType<typeof appInfo>> | null>(null);
  const [updater, setUpdater] = useState<UpdaterInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ kind: "idle" });
  const pendingUpdate = useRef<Update | null>(null);
  const { data: saves = [] } = useSaves();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!open) return;
    void appInfo().then((value) => {
      setInfo(value);
      setDataDir(value.data_dir);
    });
    void updaterInfo().then(setUpdater);
  }, [open]);

  async function checkForUpdates() {
    if (!updater?.configured) return;
    await pendingUpdate.current?.close();
    pendingUpdate.current = null;
    setUpdateStatus({ kind: "checking" });
    try {
      const update = await check();
      if (!update) {
        setUpdateStatus({ kind: "current" });
        return;
      }
      pendingUpdate.current = update;
      setUpdateStatus({
        kind: "available",
        version: update.version,
        notes: update.body?.trim() ?? "",
      });
    } catch (error) {
      setUpdateStatus({ kind: "error", message: updateErrorMessage(error) });
    }
  }

  async function installUpdate() {
    const update = pendingUpdate.current;
    if (!update) return;
    setUpdateStatus({
      kind: "downloading",
      version: update.version,
      downloaded: 0,
      total: 0,
    });
    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          setUpdateStatus({
            kind: "downloading",
            version: update.version,
            downloaded: 0,
            total: event.data.contentLength ?? 0,
          });
        } else if (event.event === "Progress") {
          setUpdateStatus((current) =>
            current.kind === "downloading"
              ? {
                  ...current,
                  downloaded: current.downloaded + event.data.chunkLength,
                }
              : current,
          );
        }
      });
      setUpdateStatus({ kind: "ready", version: update.version });
    } catch (error) {
      setUpdateStatus({ kind: "error", message: updateErrorMessage(error) });
    }
  }

  return (
    <Modal
      open={open}
      onClose={() => setOpen(false)}
      title={t("settings.title")}
      description={t("settings.description")}
      width="max-w-[660px]"
      footer={
        <Button variant="secondary" size="sm" onClick={() => setOpen(false)}>
          {t("common.done")}
        </Button>
      }
    >
      <div className="space-y-5 p-5">
        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.language")}
          </h3>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-start gap-2">
                <Languages size={14} className="mt-0.5 text-[var(--accent)]" />
                <p className="max-w-sm text-[10px] leading-relaxed text-[var(--muted)]">
                  {t("settings.languageDescription")}
                </p>
              </div>
              <Select
                value={language}
                onChange={(event) => setLanguage(event.target.value as Language)}
                className="max-w-[160px]"
              >
                {LANGUAGE_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.value === "system"
                      ? t("settings.languageSystem")
                      : option.label}
                  </option>
                ))}
              </Select>
            </div>
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.display")}
          </h3>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <div className="text-[11px] text-[var(--muted)]">{t("settings.theme")}</div>
              <div className="flex gap-1">
                {[
                  ["system", t("settings.themeSystem"), <Monitor size={13} key="system" />],
                  ["light", t("settings.themeLight"), <Sun size={13} key="light" />],
                  ["dark", t("settings.themeDark"), <Moon size={13} key="dark" />],
                ].map(([value, label, icon]) => (
                  <button
                    key={value as string}
                    type="button"
                    onClick={() => setTheme(value as Theme)}
                    className={`flex flex-1 items-center justify-center gap-1 rounded-[7px] border px-2 py-1.5 text-[10px] transition-colors ${
                      theme === value
                        ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--accent-strong)]"
                        : "border-[var(--line)] bg-[var(--surface)] text-[var(--muted)] hover:border-[var(--line-strong)]"
                    }`}
                  >
                    {icon}
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <div className="space-y-1.5">
              <div className="text-[11px] text-[var(--muted)]">{t("settings.displayMode")}</div>
              <Select
                value={displayMode}
                onChange={(event) => setDisplayMode(event.target.value as DisplayMode)}
              >
                <option value="display">{t("settings.displayValues")}</option>
                <option value="raw">{t("settings.rawValues")}</option>
              </Select>
            </div>
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.hiddenAttributes")}
          </h3>
          <div className="flex items-center justify-between rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
            <div>
              <div className="flex items-center gap-2 text-[12px] font-semibold text-[var(--ink)]">
                {hiddenEnabled ? <Eye size={14} /> : <LockKeyhole size={14} />}
                {t("settings.showHidden")}
              </div>
              <p className="mt-1 max-w-sm text-[10px] leading-relaxed text-[var(--muted)]">
                {t("settings.hiddenDescription")}
              </p>
            </div>
            <Switch
              checked={hiddenEnabled}
              onChange={(enabled) => {
                setHiddenEnabled(enabled);
                if (enabled) {
                  pushToast({
                    title: t("toast.hiddenOn.title"),
                    description: t("toast.hiddenOn.description"),
                    tone: "default",
                  });
                }
              }}
              label={t("settings.hiddenLabel")}
            />
          </div>
        </section>

        <section className="space-y-2">
          <div className="flex items-center justify-between">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
              {t("settings.localCache")}
            </h3>
            <span className="font-mono text-[9px] text-[var(--muted)]">{dataDir}</span>
          </div>
          <div className="divide-y divide-[var(--line)] overflow-hidden rounded-xl border border-[var(--line)]">
            {saves.length === 0 && (
              <div className="px-3 py-6 text-center text-[11px] text-[var(--muted)]">
                {t("settings.noSaves")}
              </div>
            )}
            {saves.map((save) => (
              <div key={save.id} className="flex items-center gap-3 bg-[var(--surface)] px-3 py-2.5">
                <Database size={14} className="text-[var(--accent)]" />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[11px] font-semibold text-[var(--ink)]">{save.file_name}</div>
                  <div className="mt-0.5 text-[9px] text-[var(--muted)]">
                    {t("settings.saveMeta", {
                      players: formatNumber(save.players, locale),
                      date: save.game_date ?? t("common.unknownDate"),
                      relative: relativeTime(save.parsed_at, locale),
                    })}
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={async () => {
                    await deleteSave(save.id);
                    await queryClient.invalidateQueries({ queryKey: ["saves"] });
                    if (save.id === currentSaveId) clearSession();
                    pushToast({
                      title: t("settings.deleted"),
                      description: save.file_name,
                      tone: "success",
                    });
                  }}
                  aria-label={t("settings.deleteCache")}
                >
                  <Trash2 size={13} />
                </Button>
              </div>
            ))}
          </div>
          <p className="text-[9px] leading-relaxed text-[var(--muted)]">
            {t("settings.cacheNote")}
          </p>
        </section>

        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.nameTables")}
          </h3>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[10px] leading-relaxed text-[var(--muted)]">
            <div className="flex items-center justify-between">
              <span>{t("settings.nationNames")}</span>
              <span className="font-mono text-[var(--ink)]">
                {t("common.entriesCount", { count: info?.nation_names ?? 0 })}
              </span>
            </div>
            <div className="mt-1 flex items-center justify-between">
              <span>{t("settings.competitionNames")}</span>
              <span className="font-mono text-[var(--ink)]">
                {t("common.entriesCount", { count: info?.competition_names ?? 0 })}
              </span>
            </div>
            <p className="mt-2">{t("settings.nameTableNote")}</p>
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.updates")}
          </h3>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[10px] text-[var(--muted)]">
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <div className="text-[11px] font-semibold text-[var(--ink)]">
                  {t("settings.updateVersion", {
                    version: updater?.version ?? info?.version ?? "1.1.0",
                  })}
                </div>
                <p className="mt-1 max-w-sm leading-relaxed">
                  {t("settings.updateDescription")}
                </p>
              </div>
              {(!updater?.configured || updateStatus.kind === "idle") && (
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={!updater?.configured}
                  onClick={() => void checkForUpdates()}
                >
                  <RefreshCw size={12} />
                  {t("settings.updateCheck")}
                </Button>
              )}
              {updateStatus.kind === "checking" && (
                <Button variant="secondary" size="sm" disabled>
                  <RefreshCw size={12} className="animate-spin" />
                  {t("settings.updateChecking")}
                </Button>
              )}
              {updateStatus.kind === "current" && (
                <div className="flex shrink-0 items-center gap-2">
                  <span className="flex items-center gap-1 text-[var(--success)]">
                    <CheckCircle2 size={13} />
                    {t("settings.updateCurrent")}
                  </span>
                  <Button variant="ghost" size="sm" onClick={() => void checkForUpdates()}>
                    <RefreshCw size={12} />
                    {t("settings.updateCheckAgain")}
                  </Button>
                </div>
              )}
            </div>

            {!updater?.configured && (
              <p className="mt-2 border-t border-[var(--line)] pt-2 leading-relaxed">
                {t("settings.updateDisabled")}
              </p>
            )}

            {updateStatus.kind === "available" && (
              <div className="mt-2 space-y-2 border-t border-[var(--line)] pt-2">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-semibold text-[var(--ink)]">
                      {t("settings.updateAvailable", { version: updateStatus.version })}
                    </div>
                    {updateStatus.notes && (
                      <p className="mt-1 whitespace-pre-line leading-relaxed">
                        {updateStatus.notes}
                      </p>
                    )}
                  </div>
                  <Button variant="primary" size="sm" onClick={() => void installUpdate()}>
                    <Download size={12} />
                    {t("settings.updateInstall")}
                  </Button>
                </div>
              </div>
            )}

            {updateStatus.kind === "downloading" && (
              <div className="mt-2 space-y-1.5 border-t border-[var(--line)] pt-2">
                <div className="flex items-center justify-between">
                  <span>{t("settings.updateDownloading")}</span>
                  <span className="font-mono text-[var(--ink)]">
                    {updateStatus.total > 0
                      ? `${Math.min(100, Math.round((updateStatus.downloaded / updateStatus.total) * 100))}%`
                      : formatNumber(updateStatus.downloaded, locale)}
                  </span>
                </div>
                <div className="h-1.5 overflow-hidden rounded-full bg-[var(--surface-3)]">
                  <div
                    className="h-full rounded-full bg-[var(--accent)] transition-all"
                    style={{
                      width:
                        updateStatus.total > 0
                          ? `${Math.min(100, (updateStatus.downloaded / updateStatus.total) * 100)}%`
                          : "10%",
                    }}
                  />
                </div>
              </div>
            )}

            {updateStatus.kind === "ready" && (
              <div className="mt-2 flex items-center justify-between gap-3 border-t border-[var(--line)] pt-2">
                <span>{t("settings.updateReady", { version: updateStatus.version })}</span>
                <Button variant="primary" size="sm" onClick={() => void relaunch()}>
                  <RotateCw size={12} />
                  {t("settings.updateRestart")}
                </Button>
              </div>
            )}

            {updateStatus.kind === "error" && (
              <div className="mt-2 space-y-1.5 border-t border-[var(--line)] pt-2">
                <p className="text-[var(--danger)]">
                  {t("settings.updateError", { message: updateStatus.message })}
                </p>
                <Button variant="ghost" size="sm" onClick={() => void checkForUpdates()}>
                  <RefreshCw size={12} />
                  {t("settings.updateCheckAgain")}
                </Button>
              </div>
            )}
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted)]">
            {t("settings.about")}
          </h3>
          <div className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[10px] leading-relaxed text-[var(--muted)]">
            <div className="flex items-center gap-2 text-[11px] font-semibold text-[var(--ink)]">
              <Info size={13} />
              FM27 Scout {info?.version ?? "1.1.0"}
            </div>
            <div className="mt-1">{t("settings.aboutDescription")}</div>
            <div className="mt-2 flex flex-wrap items-center gap-1.5">
              <Badge tone="outline">
                {t("settings.parserLabel", {
                  version: saves[0]?.parser_version ?? "0.1.0",
                })}
              </Badge>
              <Badge tone="outline">Tauri 2</Badge>
              <Badge tone="outline">Rust</Badge>
              <Badge tone="outline">fmsave · MIT</Badge>
            </div>
            <div className="mt-2">{t("settings.disclaimer")}</div>
          </div>
        </section>
      </div>
    </Modal>
  );
}
