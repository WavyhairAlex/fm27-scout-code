import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  AnalysisOverview,
  Competition,
  ExportReport,
  FilterOptions,
  OpenSaveResult,
  ParseProgress,
  Player,
  PlayerPage,
  PlayerQuery,
  RoleScore,
  SaveRecord,
  SnapshotPoint,
  SquadAnalysis,
} from "./types";
import {
  MOCK_PLAYERS,
  MOCK_SAVE,
  MOCK_SAVE_ID,
  MOCK_SUMMARY,
  mockAnalysisOverview,
  mockFilterOptions,
  mockGrowthHistory,
  mockPlayerRoles,
  mockQuery,
  mockSquadAnalysis,
} from "./mock";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function listSaves(): Promise<SaveRecord[]> {
  if (!isTauri) return [MOCK_SAVE];
  return invoke<SaveRecord[]>("list_saves");
}

export async function openSave(path: string, force = false): Promise<OpenSaveResult> {
  if (!isTauri) {
    return { save_id: MOCK_SAVE_ID, cached: true, summary: MOCK_SUMMARY, report: null };
  }
  return invoke<OpenSaveResult>("open_save", { path, force });
}

export async function queryPlayers(query: PlayerQuery): Promise<PlayerPage> {
  if (!isTauri) return mockQuery(query);
  return invoke<PlayerPage>("query_players", { query });
}

export async function getPlayer(
  saveId: number,
  uid: number,
  includeHidden = false,
): Promise<Player | null> {
  if (!isTauri) return MOCK_PLAYERS.find((player) => player.uid === uid) ?? null;
  return invoke<Player | null>("get_player", { saveId, uid, includeHidden });
}

export async function getFilterOptions(
  saveId: number,
  query?: PlayerQuery | null,
): Promise<FilterOptions> {
  if (!isTauri) return mockFilterOptions();
  return invoke<FilterOptions>("get_filter_options", { saveId, query: query ?? null });
}

export async function getPlayerRoles(
  saveId: number,
  uid: number,
): Promise<RoleScore[]> {
  if (!isTauri) return mockPlayerRoles(uid);
  return invoke<RoleScore[]>("player_roles", { saveId, uid });
}

export async function getGrowthHistory(
  saveId: number,
  uid: number,
): Promise<SnapshotPoint[]> {
  if (!isTauri) return mockGrowthHistory(uid);
  return invoke<SnapshotPoint[]>("growth_history", { saveId, uid });
}

export async function deleteSave(saveId: number): Promise<void> {
  if (!isTauri) return;
  return invoke<void>("delete_save", { saveId });
}

export async function exportPlayers(
  query: PlayerQuery,
  format: "csv" | "json",
  path: string,
): Promise<ExportReport> {
  if (!isTauri) {
    const rows = mockQuery({ ...query, page: 0, page_size: 100000 }).rows;
    const blob = new Blob([JSON.stringify(rows, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = path;
    anchor.click();
    URL.revokeObjectURL(url);
    return { path, format, rows: rows.length };
  }
  return invoke<ExportReport>("export_players", { query, format, path });
}

export async function pickSaveFile(filterName = "Football Manager saves"): Promise<string | null> {
  if (!isTauri) return MOCK_SUMMARY.path;
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: filterName, extensions: ["fm"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickExportFile(
  defaultName: string,
  format: "csv" | "json",
): Promise<string | null> {
  if (!isTauri) return defaultName;
  const selected = await save({
    defaultPath: defaultName,
    filters: [{ name: format.toUpperCase(), extensions: [format] }],
  });
  return selected ?? null;
}

export async function listenParseProgress(
  callback: (progress: ParseProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => undefined;
  return listen<ParseProgress>("parse://progress", (event) => callback(event.payload));
}

export async function listenParseDone(
  callback: (payload: unknown) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => undefined;
  return listen("parse://done", (event) => callback(event.payload));
}

export interface AppInfo {
  data_dir: string;
  version: string;
  nation_names: number;
  competition_names: number;
}

export interface UpdaterInfo {
  configured: boolean;
  version: string;
}

export async function setAppLanguage(locale: "en" | "zh-CN"): Promise<void> {
  if (!isTauri) return;
  try {
    await invoke("set_app_language", { locale });
  } catch {
    // The native menu is a polish layer; the in-app UI works without it.
  }
}

export async function appInfo(): Promise<AppInfo> {
  if (!isTauri) {
    return {
      data_dir: "~/Library/Application Support/com.fm27scout.desktop",
      version: "0.2.1",
      nation_names: 228,
      competition_names: 0,
    };
  }
  return invoke<AppInfo>("app_info");
}

export async function updaterInfo(): Promise<UpdaterInfo> {
  if (!isTauri) return { configured: false, version: "0.2.1" };
  return invoke<UpdaterInfo>("updater_info");
}

export async function appDataDir(): Promise<string> {
  return (await appInfo()).data_dir;
}

export async function listenMenuAction(
  callback: (action: string) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => undefined;
  return listen<string>("menu://action", (event) => callback(event.payload));
}

export async function listenOpenFile(
  callback: (path: string) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => undefined;
  return listen<string>("open://file", (event) => callback(event.payload));
}

export async function takePendingOpen(): Promise<string | null> {
  if (!isTauri) return null;
  return invoke<string | null>("take_pending_open");
}

export async function logMessage(message: string): Promise<void> {
  if (!isTauri) return;
  try {
    await invoke("log_message", { message });
  } catch {
    // Diagnostics must never break the UI.
  }
}

export async function getAnalysisOverview(saveId: number): Promise<AnalysisOverview> {
  if (!isTauri) return mockAnalysisOverview();
  return invoke<AnalysisOverview>("analysis_overview", { saveId });
}

export async function listCompetitions(saveId: number): Promise<Competition[]> {
  if (!isTauri) return [];
  return invoke<Competition[]>("list_competitions", { saveId });
}

export async function getSquadAnalysis(
  saveId: number,
  clubUid: number,
): Promise<SquadAnalysis> {
  if (!isTauri) return mockSquadAnalysis(clubUid);
  return invoke<SquadAnalysis>("squad_analysis", { saveId, clubUid });
}
