use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use fm27_core::ability::{CaEstimate, estimate_ca, estimate_positions};
use fm27_core::club_editor::{ClubEditRequest, ClubEditor};
use fm27_core::container::SaveFile;
use fm27_core::editor::{EditChange, PlayerEditRequest, PlayerEditor};
use fm27_core::export::ExportWriter;
use fm27_core::metadata::read_metadata;
use fm27_core::model::{Attributes, ClubDetail, Positions};
use fm27_core::store::{
    ClubFilterOptions, ClubPage, ClubQuery, FilterOptions, PlayerPage, PlayerQuery, SaveRecord,
    SnapshotPoint, SquadAnalysis, StaffFilterOptions, StaffPage, StaffQuery, Store,
};
use fm27_core::{
    ParseReport, Player, SaveSummary, inspect_save, parse_save_with_progress,
    parser_display_version, parser_version,
};
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, State};

const READ_POOL_SIZE: usize = 4;

struct AppState {
    store: std::sync::Arc<Mutex<Store>>,
    /// Independent read-only connections for queries that do not depend on the
    /// writable store's in-memory caches. WAL keeps these reads concurrent.
    readers: std::sync::Arc<ReadPool>,
    data_dir: std::path::PathBuf,
}

struct ReadPool {
    stores: Vec<Mutex<Store>>,
    next: AtomicUsize,
}

impl ReadPool {
    fn open(path: &Path, size: usize) -> Result<Self, String> {
        let mut stores = Vec::with_capacity(size);
        for _ in 0..size {
            stores.push(Mutex::new(
                Store::open_read_only(path).map_err(error_string)?,
            ));
        }
        Ok(Self {
            stores,
            next: AtomicUsize::new(0),
        })
    }

    fn with<T>(&self, operation: impl FnOnce(&Store) -> Result<T, String>) -> Result<T, String> {
        if self.stores.is_empty() {
            return Err("the read-only connection pool is empty".to_owned());
        }
        let index = self.next.fetch_add(1, Ordering::Relaxed) % self.stores.len();
        let store = self.stores[index].lock().map_err(error_string)?;
        operation(&store)
    }
}

/// File-open events can arrive before the webview has registered its listener, and on
/// a cold launch they can even arrive before `setup`. Keep one path in a process-global
/// slot until the frontend asks for it.
static PENDING_OPEN: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Serialize)]
struct OpenSaveResult {
    save_id: i64,
    cached: bool,
    summary: SaveSummary,
    report: Option<ParseReport>,
}

#[derive(Debug, Serialize)]
struct TraitOption {
    bit: u8,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CaEstimateRequest {
    /// Internal 1–100 attribute values keyed by attribute name.
    attributes: BTreeMap<String, u8>,
    /// Full 15-position familiarity table (0–20).
    positions: BTreeMap<String, u8>,
    /// Optional single position; omitted means best RCA across all positions.
    #[serde(default)]
    position: Option<String>,
    /// Internal 1–100 foot values.
    left_foot: u8,
    right_foot: u8,
}

#[derive(Debug, Deserialize)]
struct PlayerEditBatch {
    uid: u32,
    request: PlayerEditRequest,
}

#[derive(Debug, Serialize)]
struct BatchEditResult {
    output: String,
    file_size: u64,
    players: usize,
    changes: usize,
}

#[derive(Debug, Serialize)]
struct EditPlayerResult {
    output: String,
    file_size: u64,
    changes: Vec<EditChange>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EditClubResult {
    output: String,
    file_size: u64,
    changes: Vec<EditChange>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AppInfo {
    version: String,
    parser_version: String,
    data_dir: String,
    nation_names: usize,
    competition_names: usize,
}

#[derive(Debug, Serialize)]
struct UpdaterInfo {
    /// True when this build embedded an updater public key.
    configured: bool,
    version: String,
}

#[tauri::command]
fn updater_info(app: AppHandle) -> UpdaterInfo {
    let configured = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|config| config.get("pubkey"))
        .and_then(|pubkey| pubkey.as_str())
        .is_some_and(|pubkey| !pubkey.trim().is_empty());
    UpdaterInfo {
        configured,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

fn error_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

async fn run_store_command<T, F>(state: State<'_, AppState>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Store) -> Result<T, String> + Send + 'static,
{
    let store = std::sync::Arc::clone(&state.store);
    tauri::async_runtime::spawn_blocking(move || {
        let store = store.lock().map_err(error_string)?;
        operation(&store)
    })
    .await
    .map_err(error_string)?
}

async fn run_read_command<T, F>(state: State<'_, AppState>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Store) -> Result<T, String> + Send + 'static,
{
    let readers = std::sync::Arc::clone(&state.readers);
    tauri::async_runtime::spawn_blocking(move || readers.with(operation))
        .await
        .map_err(error_string)?
}

fn append_log(app: &AppHandle, line: &str) {
    if let Ok(directory) = app.path().app_data_dir() {
        let logs = directory.join("logs");
        if std::fs::create_dir_all(&logs).is_ok() {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logs.join("open-events.log"))
            {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}

/// Copy a save into the app cache before parsing so a game that is still writing
/// cannot be read half-way through. The snapshot is removed after parsing.
fn create_snapshot(data_dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let before = std::fs::metadata(source).map_err(error_string)?;
    let snapshots = data_dir.join("snapshots");
    std::fs::create_dir_all(&snapshots).map_err(error_string)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    let name = source
        .file_stem()
        .map(|value| value.to_string_lossy().replace(['/', '\\'], "_"))
        .unwrap_or_else(|| "save".to_owned());
    let destination = snapshots.join(format!("{name}-{}-{stamp}.fm", std::process::id()));
    std::fs::copy(source, &destination).map_err(|error| {
        format!(
            "could not copy the save to the local snapshot cache: {error}. \
             If the game is saving, wait for it to finish and try again."
        )
    })?;
    let after = std::fs::metadata(source).map_err(error_string)?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        let _ = std::fs::remove_file(&destination);
        return Err(
            "the save changed while it was being copied. Wait for the game to finish saving and try again."
                .to_owned(),
        );
    }
    Ok(destination)
}

#[tauri::command]
fn list_saves(state: State<'_, AppState>) -> Result<Vec<SaveRecord>, String> {
    let store = state.store.lock().map_err(error_string)?;
    store.list_saves().map_err(error_string)
}

#[tauri::command]
async fn open_save(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    force: bool,
) -> Result<OpenSaveResult, String> {
    let inspected = inspect_save(&path).map_err(error_string)?;

    if !force {
        let store = state.store.lock().map_err(error_string)?;
        if let Some(save_id) = store
            .find_cached(&inspected.fingerprint, parser_version())
            .map_err(error_string)?
        {
            append_log(&app, &format!("cache hit save_id={save_id}"));
            drop(store);
            let _ = app.emit("parse://cached", &inspected);
            return Ok(OpenSaveResult {
                save_id,
                cached: true,
                summary: inspected,
                report: None,
            });
        }
    }

    let progress_app = app.clone();
    let _ = app.emit(
        "parse://progress",
        fm27_core::ParseProgress {
            stage: "snapshot".to_owned(),
            current: 0,
            total: 0,
            message: "正在创建只读快照".to_owned(),
        },
    );
    let snapshot = create_snapshot(&state.data_dir, Path::new(&path))?;
    let parse_path = snapshot.clone();
    let parse_result = tauri::async_runtime::spawn_blocking(move || {
        parse_save_with_progress(&parse_path, |progress| {
            let _ = progress_app.emit("parse://progress", &progress);
        })
    })
    .await
    .map_err(error_string)?;
    let _ = std::fs::remove_file(&snapshot);
    let mut parsed = parse_result.map_err(error_string)?;
    if parsed.summary.fingerprint != inspected.fingerprint {
        return Err(
            "the save changed while it was being opened. Wait for the game to finish saving and try again."
                .to_owned(),
        );
    }
    parsed.summary.path = path.clone();
    parsed.summary.file_name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // The snapshot has its own mtime. Cache lookup must use the original save's
    // fingerprint, while the bytes (and therefore the digest) are identical.
    parsed.summary.file_size = inspected.file_size;
    parsed.summary.mtime_ns = inspected.mtime_ns;
    parsed.summary.fingerprint = inspected.fingerprint.clone();
    parsed.report.save = parsed.summary.clone();

    let mut store = state.store.lock().map_err(error_string)?;
    if force
        && let Some(existing) = store
            .find_cached(&parsed.summary.fingerprint, parser_version())
            .map_err(error_string)?
    {
        store.delete_save(existing).map_err(error_string)?;
    }
    let save_id = store.insert_parsed(&parsed).map_err(error_string)?;
    drop(store);

    let _ = app.emit("parse://done", &parsed.report);
    Ok(OpenSaveResult {
        save_id,
        cached: false,
        summary: parsed.summary,
        report: Some(parsed.report),
    })
}

#[tauri::command]
async fn query_players(
    state: State<'_, AppState>,
    query: PlayerQuery,
) -> Result<PlayerPage, String> {
    run_read_command(state, move |store| {
        store.query_players(&query).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn value_finder(
    state: State<'_, AppState>,
    request: fm27_core::moneyball::ValueFinderRequest,
) -> Result<fm27_core::moneyball::ValueFinderPage, String> {
    run_store_command(state, move |store| {
        store.value_finder(&request).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn stats_moneyball(
    state: State<'_, AppState>,
    request: fm27_core::stats_moneyball::StatsMoneyballRequest,
) -> Result<fm27_core::stats_moneyball::StatsMoneyballPage, String> {
    run_store_command(state, move |store| {
        store.stats_moneyball(&request).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn export_stats(
    state: State<'_, AppState>,
    request: fm27_core::stats_moneyball::StatsMoneyballRequest,
    format: String,
    path: String,
) -> Result<fm27_core::export::ExportReport, String> {
    run_store_command(state, move |store| {
        let rows = store
            .stats_moneyball_export(&request)
            .map_err(error_string)?;
        fm27_core::export::write_stats_export(std::path::Path::new(&path), &format, &rows)
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn similar_stats_players(
    state: State<'_, AppState>,
    request: fm27_core::stats_moneyball::StatsMoneyballRequest,
    uid: u32,
    limit: usize,
) -> Result<Vec<fm27_core::stats_moneyball::SimilarStatsPlayer>, String> {
    run_store_command(state, move |store| {
        store
            .similar_stats_players(&request, uid, limit)
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn squad_needs(
    state: State<'_, AppState>,
    request: fm27_core::stats_moneyball::SquadNeedsRequest,
) -> Result<fm27_core::stats_moneyball::SquadNeeds, String> {
    run_store_command(state, move |store| {
        store.squad_needs(&request).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn query_staff(state: State<'_, AppState>, query: StaffQuery) -> Result<StaffPage, String> {
    run_read_command(state, move |store| {
        store.query_staff(&query).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn staff_filter_options(
    state: State<'_, AppState>,
    save_id: i64,
    query: Option<StaffQuery>,
) -> Result<StaffFilterOptions, String> {
    run_read_command(state, move |store| {
        store
            .staff_filter_options(save_id, query.as_ref())
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn get_player(
    state: State<'_, AppState>,
    save_id: i64,
    uid: u32,
    include_hidden: bool,
) -> Result<Option<Player>, String> {
    run_read_command(state, move |store| {
        store
            .get_player(save_id, uid, include_hidden)
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn query_clubs(state: State<'_, AppState>, query: ClubQuery) -> Result<ClubPage, String> {
    run_read_command(state, move |store| {
        store.query_clubs(&query).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn get_club(
    state: State<'_, AppState>,
    save_id: i64,
    uid: u32,
) -> Result<Option<ClubDetail>, String> {
    run_read_command(state, move |store| {
        store.get_club(save_id, uid).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn club_filter_options(
    state: State<'_, AppState>,
    save_id: i64,
) -> Result<ClubFilterOptions, String> {
    run_read_command(state, move |store| {
        store.club_filter_options(save_id).map_err(error_string)
    })
    .await
}

#[tauri::command]
fn estimate_player_ca(request: CaEstimateRequest) -> Result<CaEstimate, String> {
    let mut attributes = Attributes::default();
    for (index, key) in fm27_core::layout::ATTRIBUTE_KEYS.iter().enumerate() {
        attributes.values[index] = request.attributes.get(*key).copied().unwrap_or(1).max(1);
    }

    let mut positions = Positions::default();
    for (index, key) in fm27_core::layout::POSITION_KEYS.iter().enumerate() {
        positions.values[index] = request.positions.get(*key).copied().unwrap_or(0).min(20);
    }

    let estimate = match request.position.as_deref() {
        Some(position) if !position.eq_ignore_ascii_case("AUTO") => estimate_ca(
            &attributes,
            &positions,
            request.left_foot,
            request.right_foot,
            position,
        ),
        _ => estimate_positions(
            &attributes,
            &positions,
            request.left_foot,
            request.right_foot,
        ),
    };
    estimate.ok_or_else(|| {
        format!(
            "cannot estimate CA: no valid position in player position table (requested {:?})",
            request.position
        )
    })
}

#[tauri::command]
async fn save_player_edits(
    state: State<'_, AppState>,
    save_id: i64,
    output_path: String,
    edits: Vec<PlayerEditBatch>,
) -> Result<BatchEditResult, String> {
    let source = {
        let store = state.store.lock().map_err(error_string)?;
        store
            .save_path(save_id)
            .map_err(error_string)?
            .ok_or_else(|| "save not found".to_owned())?
    };
    tauri::async_runtime::spawn_blocking(move || save_players_file(source, output_path, edits))
        .await
        .map_err(error_string)?
}

fn save_players_file(
    source: String,
    output_path: String,
    edits: Vec<PlayerEditBatch>,
) -> Result<BatchEditResult, String> {
    if edits.is_empty() {
        return Err("no edits requested".to_owned());
    }
    let save = SaveFile::open(&source).map_err(error_string)?;
    let metadata = read_metadata(&save).map_err(error_string)?;
    let mut editor = PlayerEditor::open(&save, &metadata).map_err(error_string)?;
    for edit in &edits {
        if edit.request.is_empty() {
            return Err(format!("player uid {} has no edits", edit.uid));
        }
        edit.request
            .apply(&mut editor, edit.uid)
            .map_err(error_string)?;
    }
    editor.validate().map_err(error_string)?;

    let output = PathBuf::from(&output_path);
    if output.exists() {
        return Err(format!(
            "output already exists: {}; choose a new .fm file",
            output.display()
        ));
    }
    let save_name = output
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty());
    let report = editor
        .write_as(&save, &output, 3, save_name.as_deref())
        .map_err(error_string)?;

    let verify = fm27_core::parse_save(&output)
        .map_err(error_string)
        .and_then(|parsed| {
            for edit in &edits {
                let player = parsed
                    .players
                    .iter()
                    .find(|player| player.uid == edit.uid)
                    .ok_or_else(|| {
                        format!("edited save no longer contains player uid {}", edit.uid)
                    })?;
                edit.request.verify(player).map_err(error_string)?;
            }
            Ok(())
        });
    if let Err(error) = verify {
        let _ = std::fs::remove_file(&output);
        return Err(error);
    }

    Ok(BatchEditResult {
        output: report.output,
        file_size: report.file_size,
        players: edits.len(),
        changes: editor.changes().len(),
    })
}

#[tauri::command]
fn trait_options() -> Vec<TraitOption> {
    fm27_core::layout::TRAIT_NAMES
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.is_empty())
        .map(|(bit, name)| TraitOption {
            bit: bit as u8,
            name: (*name).to_owned(),
        })
        .collect()
}

#[tauri::command]
async fn edit_player(
    state: State<'_, AppState>,
    save_id: i64,
    uid: u32,
    output_path: String,
    request: PlayerEditRequest,
) -> Result<EditPlayerResult, String> {
    let source = {
        let store = state.store.lock().map_err(error_string)?;
        store
            .save_path(save_id)
            .map_err(error_string)?
            .ok_or_else(|| "save not found".to_owned())?
    };
    tauri::async_runtime::spawn_blocking(move || {
        edit_player_file(source, uid, output_path, request)
    })
    .await
    .map_err(error_string)?
}

fn edit_player_file(
    source: String,
    uid: u32,
    output_path: String,
    request: PlayerEditRequest,
) -> Result<EditPlayerResult, String> {
    if request.is_empty() {
        return Err("no edits requested".to_owned());
    }
    let warnings = request.warnings();
    let save = SaveFile::open(&source).map_err(error_string)?;
    let metadata = read_metadata(&save).map_err(error_string)?;
    let mut editor = PlayerEditor::open(&save, &metadata).map_err(error_string)?;
    request.apply(&mut editor, uid).map_err(error_string)?;
    editor.validate().map_err(error_string)?;

    let output = PathBuf::from(&output_path);
    if output.exists() {
        return Err(format!(
            "output already exists: {}; choose a new .fm file",
            output.display()
        ));
    }
    let save_name = output
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty());
    let report = editor
        .write_as(&save, &output, 3, save_name.as_deref())
        .map_err(error_string)?;

    let verify = fm27_core::parse_save(&output)
        .map_err(error_string)
        .and_then(|parsed| {
            parsed
                .players
                .iter()
                .find(|player| player.uid == uid)
                .ok_or_else(|| format!("edited save no longer contains player uid {uid}"))
                .and_then(|player| request.verify(player).map_err(error_string))
        });
    if let Err(error) = verify {
        let _ = std::fs::remove_file(&output);
        return Err(error);
    }

    Ok(EditPlayerResult {
        output: report.output,
        file_size: report.file_size,
        changes: editor.changes().to_vec(),
        warnings,
    })
}

#[tauri::command]
async fn edit_club(
    state: State<'_, AppState>,
    save_id: i64,
    club_uid: u32,
    output_path: String,
    request: ClubEditRequest,
) -> Result<EditClubResult, String> {
    let source = {
        let store = state.store.lock().map_err(error_string)?;
        store
            .save_path(save_id)
            .map_err(error_string)?
            .ok_or_else(|| "save not found".to_owned())?
    };
    tauri::async_runtime::spawn_blocking(move || {
        edit_club_file(source, club_uid, output_path, request)
    })
    .await
    .map_err(error_string)?
}

fn edit_club_file(
    source: String,
    club_uid: u32,
    output_path: String,
    request: ClubEditRequest,
) -> Result<EditClubResult, String> {
    if request.is_empty() {
        return Err("no edits requested".to_owned());
    }
    let warnings = request.warnings();
    let save = SaveFile::open(&source).map_err(error_string)?;
    let metadata = read_metadata(&save).map_err(error_string)?;
    let mut editor = ClubEditor::open(&save, &metadata).map_err(error_string)?;
    request.apply(&mut editor, club_uid).map_err(error_string)?;
    editor.validate().map_err(error_string)?;

    let output = PathBuf::from(&output_path);
    if output.exists() {
        return Err(format!(
            "output already exists: {}; choose a new .fm file",
            output.display()
        ));
    }
    let save_name = output
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty());
    let report = editor
        .write_as(&save, &output, 3, save_name.as_deref())
        .map_err(error_string)?;

    let verify = fm27_core::parse_save(&output)
        .map_err(error_string)
        .and_then(|parsed| request.verify(&parsed, club_uid).map_err(error_string));
    if let Err(error) = verify {
        let _ = std::fs::remove_file(&output);
        return Err(error);
    }

    Ok(EditClubResult {
        output: report.output,
        file_size: report.file_size,
        changes: editor.changes().to_vec(),
        warnings,
    })
}

#[tauri::command]
async fn player_roles(
    state: State<'_, AppState>,
    save_id: i64,
    uid: u32,
) -> Result<Vec<fm27_core::model::RoleScore>, String> {
    run_read_command(state, move |store| {
        let player = store
            .get_player(save_id, uid, true)
            .map_err(error_string)?
            .ok_or_else(|| "player not found".to_owned())?;
        Ok(fm27_core::roles::score_roles(
            &player.attributes,
            &player.positions,
        ))
    })
    .await
}

#[tauri::command]
async fn growth_history(
    state: State<'_, AppState>,
    save_id: i64,
    uid: u32,
) -> Result<Vec<SnapshotPoint>, String> {
    run_read_command(state, move |store| {
        store.growth_history(save_id, uid).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn managed_club(
    state: State<'_, AppState>,
    save_id: i64,
) -> Result<Option<fm27_core::ManagedClub>, String> {
    run_read_command(state, move |store| {
        store.managed_club(save_id).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn squad_analysis(
    state: State<'_, AppState>,
    save_id: i64,
    club_uid: u32,
) -> Result<SquadAnalysis, String> {
    run_read_command(state, move |store| {
        store
            .squad_analysis(save_id, club_uid)
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn scout_overview(
    state: State<'_, AppState>,
    save_id: i64,
    club_uid: u32,
) -> Result<fm27_core::scouting::ScoutOverview, String> {
    run_read_command(state, move |store| {
        store
            .scout_overview(save_id, club_uid)
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn scout_advice(
    state: State<'_, AppState>,
    request: fm27_core::scouting::ScoutRequest,
) -> Result<fm27_core::scouting::Scouted, String> {
    run_read_command(state, move |store| {
        store.scout_advice(&request).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn list_competitions(
    state: State<'_, AppState>,
    save_id: i64,
) -> Result<Vec<fm27_core::model::Competition>, String> {
    run_read_command(state, move |store| {
        store.competitions(save_id).map_err(error_string)
    })
    .await
}

#[tauri::command]
async fn get_filter_options(
    state: State<'_, AppState>,
    save_id: i64,
    query: Option<PlayerQuery>,
) -> Result<FilterOptions, String> {
    run_read_command(state, move |store| {
        store
            .filter_options_with_query(save_id, query.as_ref())
            .map_err(error_string)
    })
    .await
}

#[tauri::command]
fn delete_save(state: State<'_, AppState>, save_id: i64) -> Result<(), String> {
    let mut store = state.store.lock().map_err(error_string)?;
    store.delete_save(save_id).map_err(error_string)
}

#[tauri::command]
async fn export_players(
    state: State<'_, AppState>,
    query: PlayerQuery,
    format: String,
    path: String,
) -> Result<fm27_core::export::ExportReport, String> {
    let db_path = state.data_dir.join("fm27scout.db");
    tauri::async_runtime::spawn_blocking(move || {
        let mut writer =
            ExportWriter::create(std::path::Path::new(&path), &format).map_err(error_string)?;
        // A dedicated read-only connection keeps the global store lock out of the
        // whole filesystem write. WAL allows this read to overlap normal queries.
        let store = Store::open_read_only(&db_path).map_err(error_string)?;
        store
            .stream_players(&query, |row| writer.write_row(&row))
            .map_err(error_string)?;
        writer.finish().map_err(error_string)
    })
    .await
    .map_err(error_string)?
}

#[tauri::command]
async fn export_staff(
    state: State<'_, AppState>,
    query: StaffQuery,
    format: String,
    path: String,
) -> Result<fm27_core::export::ExportReport, String> {
    let db_path = state.data_dir.join("fm27scout.db");
    tauri::async_runtime::spawn_blocking(move || {
        let mut writer =
            fm27_core::export::StaffExportWriter::create(std::path::Path::new(&path), &format)
                .map_err(error_string)?;
        let store = Store::open_read_only(&db_path).map_err(error_string)?;
        store
            .stream_staff(&query, |row| writer.write_row(&row))
            .map_err(error_string)?;
        writer.finish().map_err(error_string)
    })
    .await
    .map_err(error_string)?
}

#[tauri::command]
fn take_pending_open(app: AppHandle) -> Option<String> {
    let value = PENDING_OPEN
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    append_log(&app, &format!("take_pending_open -> {value:?}"));
    value
}

#[tauri::command]
fn log_message(app: AppHandle, message: String) {
    append_log(&app, &format!("frontend: {message}"));
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        parser_version: parser_display_version().to_owned(),
        data_dir: state.data_dir.display().to_string(),
        nation_names: fm27_core::nations::name_count(),
        competition_names: fm27_core::reference::name_count(),
    }
}

struct MenuLabels {
    open: &'static str,
    export: &'static str,
    close_window: &'static str,
    settings: &'static str,
    sidebar: &'static str,
    inspector: &'static str,
    compare: &'static str,
    analysis: &'static str,
    undo: &'static str,
    redo: &'static str,
    cut: &'static str,
    copy: &'static str,
    paste: &'static str,
    select_all: &'static str,
    about: &'static str,
    quit: &'static str,
    help: &'static str,
    file_menu: &'static str,
    edit_menu: &'static str,
    view_menu: &'static str,
    help_menu: &'static str,
}

fn menu_labels(locale: &str) -> MenuLabels {
    if locale.to_ascii_lowercase().starts_with("zh") {
        MenuLabels {
            open: "打开存档…",
            export: "导出…",
            close_window: "关闭窗口",
            settings: "设置…",
            sidebar: "显示/隐藏筛选栏",
            inspector: "显示/隐藏球员档案",
            compare: "球员对比",
            analysis: "分析中心",
            undo: "撤销",
            redo: "重做",
            cut: "剪切",
            copy: "复制",
            paste: "粘贴",
            select_all: "全选",
            about: "关于 FM27 Scout",
            quit: "退出 FM27 Scout",
            help: "帮助与快捷键",
            file_menu: "文件",
            edit_menu: "编辑",
            view_menu: "视图",
            help_menu: "帮助",
        }
    } else {
        MenuLabels {
            open: "Open Save…",
            export: "Export…",
            close_window: "Close Window",
            settings: "Settings…",
            sidebar: "Show/Hide Filters",
            inspector: "Show/Hide Player Profile",
            compare: "Compare Players",
            analysis: "Analysis Centre",
            undo: "Undo",
            redo: "Redo",
            cut: "Cut",
            copy: "Copy",
            paste: "Paste",
            select_all: "Select All",
            about: "About FM27 Scout",
            quit: "Quit FM27 Scout",
            help: "Help & Shortcuts",
            file_menu: "File",
            edit_menu: "Edit",
            view_menu: "View",
            help_menu: "Help",
        }
    }
}

fn build_menu<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    locale: &str,
) -> tauri::Result<Menu<R>> {
    let labels = menu_labels(locale);
    let open = MenuItem::with_id(handle, "open", labels.open, true, Some("CmdOrCtrl+O"))?;
    let export = MenuItem::with_id(handle, "export", labels.export, true, Some("CmdOrCtrl+E"))?;
    let close = PredefinedMenuItem::close_window(handle, Some(labels.close_window))?;
    let settings = MenuItem::with_id(
        handle,
        "settings",
        labels.settings,
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let sidebar = MenuItem::with_id(
        handle,
        "sidebar",
        labels.sidebar,
        true,
        Some("CmdOrCtrl+\\"),
    )?;
    let inspector = MenuItem::with_id(
        handle,
        "inspector",
        labels.inspector,
        true,
        Some("CmdOrCtrl+Shift+I"),
    )?;
    let compare = MenuItem::with_id(
        handle,
        "compare",
        labels.compare,
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let analysis = MenuItem::with_id(
        handle,
        "analysis",
        labels.analysis,
        true,
        Some("CmdOrCtrl+Shift+A"),
    )?;
    let undo = PredefinedMenuItem::undo(handle, Some(labels.undo))?;
    let redo = PredefinedMenuItem::redo(handle, Some(labels.redo))?;
    let cut = PredefinedMenuItem::cut(handle, Some(labels.cut))?;
    let copy = PredefinedMenuItem::copy(handle, Some(labels.copy))?;
    let paste = PredefinedMenuItem::paste(handle, Some(labels.paste))?;
    let select_all = PredefinedMenuItem::select_all(handle, Some(labels.select_all))?;
    let about = PredefinedMenuItem::about(handle, Some(labels.about), None)?;
    let quit = PredefinedMenuItem::quit(handle, Some(labels.quit))?;
    let helper = MenuItem::with_id(handle, "help", labels.help, true, Some("CmdOrCtrl+Shift+/"))?;
    let sep1 = PredefinedMenuItem::separator(handle)?;
    let sep2 = PredefinedMenuItem::separator(handle)?;
    let sep3 = PredefinedMenuItem::separator(handle)?;
    let sep4 = PredefinedMenuItem::separator(handle)?;
    let sep5 = PredefinedMenuItem::separator(handle)?;

    let app_menu = Submenu::with_items(
        handle,
        "FM27 Scout",
        true,
        &[&about, &sep1, &settings, &sep2, &quit],
    )?;
    let file_menu = Submenu::with_items(
        handle,
        labels.file_menu,
        true,
        &[&open, &export, &sep3, &close],
    )?;
    let edit_menu = Submenu::with_items(
        handle,
        labels.edit_menu,
        true,
        &[&undo, &redo, &sep4, &cut, &copy, &paste, &select_all],
    )?;
    let view_menu = Submenu::with_items(
        handle,
        labels.view_menu,
        true,
        &[&sidebar, &inspector, &sep5, &compare, &analysis],
    )?;
    let help_menu = Submenu::with_items(handle, labels.help_menu, true, &[&helper])?;
    Menu::with_items(
        handle,
        &[&app_menu, &file_menu, &edit_menu, &view_menu, &help_menu],
    )
}

fn install_menu(app: &tauri::App) -> tauri::Result<()> {
    // The frontend resolves the persisted/system locale immediately after launch
    // and calls `set_app_language`, which rebuilds this menu in the right language.
    app.set_menu(build_menu(app.handle(), "en")?)?;
    app.on_menu_event(|app, event| {
        let action = event.id().as_ref();
        if !action.is_empty() {
            let _ = app.emit("menu://action", action);
        }
    });
    Ok(())
}

#[tauri::command]
fn set_app_language(app: AppHandle, locale: String) -> Result<(), String> {
    let menu = build_menu(&app, &locale).map_err(error_string)?;
    app.set_menu(menu).map_err(error_string)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            install_menu(app)?;
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let snapshots = data_dir.join("snapshots");
            std::fs::create_dir_all(&snapshots)?;
            if let Ok(entries) = std::fs::read_dir(&snapshots) {
                for entry in entries.flatten() {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
            let db_path = data_dir.join("fm27scout.db");
            let store = Store::open(&db_path)?;
            let readers =
                ReadPool::open(&db_path, READ_POOL_SIZE).map_err(std::io::Error::other)?;
            app.manage(AppState {
                store: std::sync::Arc::new(Mutex::new(store)),
                readers: std::sync::Arc::new(readers),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_saves,
            open_save,
            query_players,
            value_finder,
            stats_moneyball,
            export_stats,
            similar_stats_players,
            squad_needs,
            query_staff,
            staff_filter_options,
            query_clubs,
            get_player,
            get_club,
            club_filter_options,
            edit_player,
            edit_club,
            save_player_edits,
            estimate_player_ca,
            trait_options,
            player_roles,
            growth_history,
            managed_club,
            squad_analysis,
            scout_overview,
            scout_advice,
            list_competitions,
            get_filter_options,
            delete_save,
            export_players,
            export_staff,
            app_info,
            updater_info,
            set_app_language,
            take_pending_open,
            log_message
        ])
        .build(tauri::generate_context!())
        .expect("error while building FM27 Scout")
        .run(|app, event| {
            if let tauri::RunEvent::Opened { urls } = event {
                append_log(app, &format!("opened urls={urls:?}"));
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        if let Ok(mut pending) = PENDING_OPEN.lock() {
                            *pending = Some(path.display().to_string());
                            append_log(app, &format!("pending set -> {}", path.display()));
                        }
                        let _ = app.emit("open://file", path.display().to_string());
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_ca_from_internal_attributes_and_position_table() {
        let request = CaEstimateRequest {
            attributes: fm27_core::layout::ATTRIBUTE_KEYS
                .iter()
                .map(|key| ((*key).to_owned(), 65))
                .collect(),
            positions: fm27_core::layout::POSITION_KEYS
                .iter()
                .map(|key| ((*key).to_owned(), if *key == "stc" { 20 } else { 0 }))
                .collect(),
            position: Some("STC".to_owned()),
            left_foot: 100,
            right_foot: 50,
        };
        let estimate = estimate_player_ca(request).expect("estimate");
        assert_eq!(estimate.position, "STC");
        assert_eq!(estimate.estimated_ca, 135);
        assert_eq!(estimate.initial_ca, 136);
        assert_eq!(estimate.position_cost, 0);
    }

    #[test]
    #[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
    fn saves_several_player_edits_in_one_file() {
        let source = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
        let parsed = fm27_core::parse_save(&source).expect("parse source save");
        let players: Vec<_> = parsed
            .players
            .iter()
            .filter(|player| player.pa.is_some() && player.ca < 200)
            .take(2)
            .collect();
        assert_eq!(players.len(), 2);
        let directory = tempfile::tempdir().expect("tempdir");
        let output = directory.path().join("batch-edited.fm");
        let edits = players
            .iter()
            .map(|player| PlayerEditBatch {
                uid: player.uid,
                request: PlayerEditRequest {
                    pa: Some(200),
                    ..PlayerEditRequest::default()
                },
            })
            .collect();
        let result =
            save_players_file(source, output.display().to_string(), edits).expect("save batch");
        assert_eq!(result.players, 2);

        let edited = fm27_core::parse_save(&result.output).expect("parse edited save");
        for player in &players {
            let changed = edited
                .players
                .iter()
                .find(|candidate| candidate.uid == player.uid)
                .expect("edited player");
            assert_eq!(changed.pa, Some(200));
        }
    }

    #[test]
    #[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
    fn edits_a_player_and_refuses_to_overwrite_an_existing_output() {
        let source = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
        let parsed = fm27_core::parse_save(&source).expect("parse source save");
        let player = parsed
            .players
            .iter()
            .find(|player| player.pa.is_some())
            .expect("a player with potential ability");
        let uid = player.uid;
        let directory = tempfile::tempdir().expect("tempdir");
        let output = directory.path().join("desktop-edited.fm");

        let request = PlayerEditRequest {
            pa: Some(200),
            ..PlayerEditRequest::default()
        };
        let result = edit_player_file(source.clone(), uid, output.display().to_string(), request)
            .expect("edit player");
        assert!(Path::new(&result.output).exists());
        assert_eq!(result.changes.len(), 1);

        let edited = fm27_core::parse_save(&result.output).expect("parse edited save");
        assert_eq!(
            edited
                .players
                .iter()
                .find(|player| player.uid == uid)
                .map(|player| player.pa),
            Some(Some(200))
        );

        let error = edit_player_file(
            source,
            uid,
            output.display().to_string(),
            PlayerEditRequest {
                pa: Some(199),
                ..PlayerEditRequest::default()
            },
        )
        .expect_err("existing output must be refused");
        assert!(error.contains("already exists"));
    }
}
