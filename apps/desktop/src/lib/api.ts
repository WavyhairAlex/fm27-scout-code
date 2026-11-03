import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writeText as writeClipboardText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  AnalysisOverview,
  BatchEditResult,
  CaEstimate,
  CaEstimateRequest,
  ClubDetail,
  ClubEditRequest,
  ClubFilterOptions,
  ClubPage,
  ClubQuery,
  Competition,
  EditClubResult,
  EditPlayerResult,
  ExportReport,
  FilterOptions,
  OpenSaveResult,
  ParseProgress,
  Player,
  PlayerEditBatch,
  PlayerEditRequest,
  PlayerPage,
  PlayerQuery,
  RoleScore,
  SaveRecord,
  SnapshotPoint,
  SquadAnalysis,
  TraitOption,
  ValueFinderPage,
  ValueFinderRequest,
} from "./types";
import { estimateCaExact } from "./ca";
import {
  MOCK_PLAYERS,
  MOCK_SAVE,
  MOCK_SAVE_ID,
  MOCK_SUMMARY,
  MOCK_TRAIT_OPTIONS,
  mockAnalysisOverview,
  mockClubDetail,
  mockClubFilterOptions,
  mockFilterOptions,
  mockGrowthHistory,
  mockPlayerRoles,
  mockQuery,
  mockQueryClubs,
  mockSquadAnalysis,
  mockValueFinder,
} from "./mock";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Write plain text to the system clipboard. */
export async function copyTextToClipboard(text: string): Promise<void> {
  if (isTauri) {
    await writeClipboardText(text);
    return;
  }
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const area = document.createElement("textarea");
  area.value = text;
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.select();
  const copied = document.execCommand("copy");
  area.remove();
  if (!copied) throw new Error("clipboard write failed");
}

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

export async function queryValueFinder(
  request: ValueFinderRequest,
): Promise<ValueFinderPage> {
  if (!isTauri) return mockValueFinder(request);
  return invoke<ValueFinderPage>("value_finder", { request });
}

export async function getPlayer(
  saveId: number,
  uid: number,
  includeHidden = false,
): Promise<Player | null> {
  if (!isTauri) return MOCK_PLAYERS.find((player) => player.uid === uid) ?? null;
  return invoke<Player | null>("get_player", { saveId, uid, includeHidden });
}

export async function queryClubs(query: ClubQuery): Promise<ClubPage> {
  if (!isTauri) return mockQueryClubs(query);
  return invoke<ClubPage>("query_clubs", { query });
}

export async function getClub(saveId: number, uid: number): Promise<ClubDetail | null> {
  if (!isTauri) return mockClubDetail(uid);
  return invoke<ClubDetail | null>("get_club", { saveId, uid });
}

export async function getClubFilterOptions(saveId: number): Promise<ClubFilterOptions> {
  if (!isTauri) return mockClubFilterOptions();
  return invoke<ClubFilterOptions>("club_filter_options", { saveId });
}

export async function editClub(
  saveId: number,
  clubUid: number,
  outputPath: string,
  request: ClubEditRequest,
): Promise<EditClubResult> {
  if (!isTauri) {
    const changes = [
      ...(request.reputation != null
        ? [{ field: "reputation", before: 0, after: request.reputation }]
        : []),
      ...(request.corporate_facilities != null
        ? [{ field: "corporate_facilities", before: 0, after: request.corporate_facilities }]
        : []),
      ...(
        [
          ["training_facilities", request.training_facilities],
          ["youth_facilities", request.youth_facilities],
          ["junior_coaching", request.junior_coaching],
          ["youth_recruitment", request.youth_recruitment],
        ] as const
      ).flatMap(([field, value]) =>
        value != null ? [{ field, before: 0, after: value }] : [],
      ),
      ...(request.finances?.balance != null
        ? [{ field: "finance.balance", before: 0, after: request.finances.balance }]
        : []),
      ...(request.finances?.transfer_budget_remaining != null
        ? [
            {
              field: "finance.transfer_budget_remaining",
              before: 0,
              after: request.finances.transfer_budget_remaining,
            },
          ]
        : []),
      ...(request.finances?.wage_budget_weekly != null
        ? [
            {
              field: "finance.wage_budget_weekly",
              before: 0,
              after: request.finances.wage_budget_weekly,
            },
          ]
        : []),
      ...request.stadiums.flatMap((stadium) => [
        ...(stadium.all_seater_capacity != null
          ? [
              {
                field: "stadium.all_seater_capacity",
                before: 0,
                after: stadium.all_seater_capacity,
              },
            ]
          : []),
        ...(stadium.expansion_capacity != null
          ? [
              {
                field: "stadium.expansion_capacity",
                before: 0,
                after: stadium.expansion_capacity,
              },
            ]
          : []),
      ]),
    ];
    return { output: outputPath, file_size: 0, changes, warnings: [] };
  }
  return invoke<EditClubResult>("edit_club", { saveId, clubUid, outputPath, request });
}

export async function savePlayerEdits(
  saveId: number,
  outputPath: string,
  edits: PlayerEditBatch[],
): Promise<BatchEditResult> {
  if (!isTauri) {
    const changes = edits.reduce(
      (total, edit) =>
        total +
        (edit.request.ca != null ? 1 : 0) +
        (edit.request.pa != null ? 1 : 0) +
        edit.request.attributes.length +
        edit.request.personality.length +
        edit.request.traits.length +
        edit.request.untraits.length,
      0,
    );
    return { output: outputPath, file_size: 0, players: edits.length, changes };
  }
  return invoke<BatchEditResult>("save_player_edits", { saveId, outputPath, edits });
}

export async function getTraitOptions(): Promise<TraitOption[]> {
  if (!isTauri) return MOCK_TRAIT_OPTIONS;
  return invoke<TraitOption[]>("trait_options");
}

export async function estimatePlayerCa(
  request: CaEstimateRequest,
): Promise<CaEstimate | null> {
  if (!isTauri) return estimateCaExact(request);
  return invoke<CaEstimate>("estimate_player_ca", { request });
}

export async function editPlayer(
  saveId: number,
  uid: number,
  outputPath: string,
  request: PlayerEditRequest,
): Promise<EditPlayerResult> {
  if (!isTauri) {
    const changes = [
      ...(request.ca != null ? [{ field: "ca", before: 0, after: request.ca }] : []),
      ...(request.pa != null ? [{ field: "pa", before: 0, after: request.pa }] : []),
      ...request.attributes.map((edit) => ({
        field: `attribute.${edit.key}`,
        before: 0,
        after: edit.value,
      })),
      ...request.personality.map((edit) => ({
        field: `personality.${edit.key}`,
        before: 0,
        after: edit.value,
      })),
      ...request.traits.map((bit) => ({ field: `trait.${bit}`, before: 0, after: 1 })),
      ...request.untraits.map((bit) => ({ field: `trait.${bit}`, before: 1, after: 0 })),
    ];
    return {
      output: outputPath,
      file_size: 0,
      changes,
      warnings: [],
    };
  }
  return invoke<EditPlayerResult>("edit_player", { saveId, uid, outputPath, request });
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

export async function pickEditOutputFile(defaultPath: string): Promise<string | null> {
  if (!isTauri) return defaultPath;
  const selected = await save({
    defaultPath,
    filters: [{ name: "Football Manager saves", extensions: ["fm"] }],
  });
  return selected ?? null;
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
  parser_version: string;
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
      version: __APP_VERSION__,
      parser_version: __APP_VERSION__,
      nation_names: 228,
      competition_names: 0,
    };
  }
  return invoke<AppInfo>("app_info");
}

export async function updaterInfo(): Promise<UpdaterInfo> {
  if (!isTauri) return { configured: false, version: __APP_VERSION__ };
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
