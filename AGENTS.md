# AGENTS.md

This file provides guidance when working with code in this repository.

## What this project is

FM27 Scout is a macOS scouting workbench for Football Manager 27 with an optional experimental save editor. It is a Tauri 2 app with a Rust parsing/query core and a React 19 + TypeScript UI. The source save is always read-only; editing writes a separate file that is re-parsed and verified before it is accepted.

## Workspace layout

- `crates/fm27-core/` — platform-independent save parser, SQLite cache, query/filter/sort layer, scoring models and byte-level editors. No Tauri dependency.
- `crates/fm27-cli/` — the `fm27scout` CLI (`inspect`, `validate`, `players`, `clubs`, `competitions`, `index`, `export`, `edit`, `edit-club`).
- `apps/desktop/src-tauri/` — Tauri commands, app state, menu and file-open handling.
- `apps/desktop/src/` — React UI (`components/`, `lib/`, `store/`).
- `shared/schema.sql` — SQLite schema, compiled into `fm27-core` with `include_str!`.
- `scripts/` — version bump/check, Python oracle exports and comparisons, release scripts.
- `docs/` — reverse-engineered save layout, parser notes, validation, editor and Moneyball documentation.
- `.ref-fmsave/` — gitignored local clone of the Python `fmsave` reference implementation, used only for oracle comparison.

`HANDOFF.md` is gitignored and historical; `docs/` and the code are authoritative.

## Commands

Node and Cargo are not on the default shell PATH on this machine. Set this first:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.nvm/versions/node/v24.21.0/bin:$PATH"
```

Toolchain: Rust stable (workspace `rust-version = 1.85`, edition 2024), Node 24 (`.nvmrc`), pnpm 12.3.4 (`packageManager`).

### Check, lint and test

```bash
# Rust (matches CI)
cargo fmt --all -- --check
cargo clippy -p fm27-core -p fm27-cli --all-targets -- -D warnings
cargo test -p fm27-core

# Full workspace tests (includes CLI and desktop unit tests)
cargo test --workspace

# Desktop backend check; tauri-build requires the dist directory to exist
mkdir -p apps/desktop/dist
cargo check -p fm27-scout-desktop

# Frontend
cd apps/desktop
pnpm install --frozen-lockfile
pnpm build                 # tsc --noEmit && vite build
pnpm test                  # vitest run
pnpm vitest run src/lib/mock.test.ts   # single test file
```

Inexact test-name filtering works as usual: `cargo test -p fm27-core moneyball`.

Real-save regression tests are `#[ignore]` and need a local save; fixtures are never committed:

```bash
FM27_TEST_SAVE="/path/to/save.fm" cargo test -p fm27-core --release -- --ignored
# one integration-test binary
FM27_TEST_SAVE="/path/to/save.fm" cargo test -p fm27-core --release \
  --test season_stats_real_save -- --ignored
```

### Run the app

```bash
cd apps/desktop
pnpm tauri dev      # full desktop app
pnpm dev            # browser-only UI; api.ts falls back to mock data outside Tauri
```

### Build and release

```bash
cd apps/desktop
pnpm tauri build    # local bundle for the current architecture
```

Universal DMG / signing / notarization: `scripts/release-macos.sh`; CI publishes unsigned Universal DMGs from `v*` tags (see `docs/release.md`).

The version has one source of truth: `[workspace.package].version` in the root `Cargo.toml`. Sync the other copies and `Cargo.lock` with:

```bash
node scripts/version.mjs --check
node scripts/version.mjs 1.4.1
```

### CLI

```bash
cargo build --release -p fm27-cli
./target/release/fm27scout inspect "/path/to/save.fm"
./target/release/fm27scout validate "/path/to/save.fm"
./target/release/fm27scout players "/path/to/save.fm" --limit 1000 --format json
./target/release/fm27scout clubs "/path/to/save.fm" --details --output clubs.json
./target/release/fm27scout index "/path/to/save.fm" --db fm27scout.db
```

### Oracle validation

The Python reference (`fmsave`, FM27 experimental branch) is the ground truth for shared fields:

```bash
python3 -m venv .venv
.venv/bin/pip install -e .ref-fmsave
.venv/bin/python scripts/oracle_export.py --output /tmp/oracle.json
cargo build --release -p fm27-cli
./target/release/fm27scout players "/path/to/save.fm" --limit 50000 \
  --format json --output /tmp/rust.json
.venv/bin/python scripts/compare_outputs.py /tmp/oracle.json /tmp/rust.json
```

`scripts/oracle_clubs.py` + `compare_clubs.py` and `scripts/oracle_competitions.py` + `compare_competitions.py` cover clubs and competitions. Baseline results are in `docs/validation.md`.

## Architecture

### Data flow

```text
.fm save
  → container::SaveFile   (26-byte header, zstd frames, compressed trailer directory)
  → parse::parse_save     (metadata, players, clubs, contracts, stats, clubs/finances, stadiums)
  → store::Store          (SQLite cache at the app data directory; shared/schema.sql)
  → Tauri commands        (apps/desktop/src-tauri/src/lib.rs)
  → lib/api.ts            (invoke wrapper, mock fallback outside Tauri)
  → lib/query.ts          (TanStack Query hooks)
  → React components
```

`ParsedSave` from `parse.rs` is the boundary between decoding and the cache: `Store::insert_parsed` writes all rows in one transaction and assigns a `save_id` used by every later query.

### Save parsing

- `container.rs` reads the header, validates the trailer directory, decompresses frames with per-frame/total caps, and handles both listed sections and unlisted frame regions. Season statistics live in the unlisted region after `non_pl_hist_ls`.
- `parse.rs` orchestrates decoding: metadata → `game_db` → name pools → player records → clubs/teams → finances/facilities → stadiums → competitions/leagues → injuries/suspensions → players and contracts → season stats.
- All reverse-engineered offsets and key tables live in `layout.rs`. Do not scatter magic offsets through decoders.
- `player_scan.rs`, `persons.rs` and `names.rs` locate player records and person blocks; `clubs.rs`, `contracts.rs`, `competitions.rs`, `finances.rs`, `facilities.rs`, `stadiums.rs`, `injuries.rs` and `suspensions.rs` decode their sections.
- Supported builds are `27.0.0+2486695` and `27.0.0+2493426`, `game_db` schema 4000. Unknown builds are read in degraded mode where safe, and editing refuses them. The parser fails closed instead of guessing when framing or record shapes are invalid.

`docs/fm27-layout.md` is the quick offset reference; `docs/parser-notes.md` records the observed FM27 facts and the intentional stopping points.

### SQLite cache and queries

- `Store` wraps one WAL-mode SQLite connection. `shared/schema.sql` defines players, contracts, clubs, club finances, stadiums, competitions, season stats, materialized metrics, and growth snapshots.
- Save cache identity is the save fingerprint (`SHA-256` of header + trailer) plus `layout::PARSER_VERSION`; `find_cached` hits only when both match.
- Parsing a save also appends `player_snapshots` rows keyed by `career_key` (save name) + game date, which powers growth history across saves of the same career.
- Filtering, multi-column sorting and pagination are server-side in `store.rs`; the UI never sees the database path.
- `migrate()` in `store.rs` applies additive `ALTER TABLE` changes for existing caches. When adding a column, update both `shared/schema.sql` and the `COLUMNS` list in `migrate()`.
- Value Finder and Stats Moneyball keep in-memory baseline caches on `Store`; `insert_parsed` clears them.

### Tauri backend

- `AppState` holds the writable `Store` behind a mutex, a pool of four independent read-only `Store` connections, and the data directory. Read queries use `run_read_command`; expensive or mutating work uses the writable store inside `spawn_blocking`.
- `open_save` checks the fingerprint cache, copies the selected save to a temporary snapshot, parses it with `parse://progress` events, verifies the source did not change during the copy, then inserts the result.
- Editing commands (`edit_player`, `edit_club`, `save_player_edits`) mutate decompressed `game_db` bytes in memory, write a **new** file through `SaveFile::write_with_replacements`, refuse to overwrite an output, re-parse the result and verify every requested field, and delete the output if verification fails.
- Every command must be registered in `tauri::generate_handler!` in `apps/desktop/src-tauri/src/lib.rs`.
- Menu actions and Finder file-open events are bridged to the frontend by emitted events; a pending open path survives a cold launch. Undo/redo/cut/copy/paste use predefined native menu items.

### React frontend

- `lib/api.ts` is the only Tauri boundary. Data functions check `isTauri` and fall back to `lib/mock.ts`, which is why `pnpm dev` works in a plain browser. Mock data must remain consistent with the Rust response shapes in `lib/types.ts`.
- Data fetching lives in `lib/query.ts` (TanStack Query keys such as `["players", query]`). UI state, saved filter views, pending edits and dialogs live in the Zustand store `store/app.ts`.
- `lib/i18n.ts` contains typed `en` and `zh-CN` dictionaries; `TranslationKey` is derived from the English dictionary, so adding a key requires adding it to both languages (or the build fails). `useI18n()` also provides currency formatting via `lib/currency.ts` and `lib/utils.ts`.
- Reusable primitives are in `components/ui.tsx`; the large feature dialogs (`AnalysisDialog`, `Inspector`, `PlayerTable`, `FilterSidebar`, `EditPlayerDialog`, `ClubInspector`) own their tab/filter/table logic.
- Frontend unit tests are colocated as `*.test.ts(x)` and run with Vitest. Browser-level Playwright tests live in `apps/desktop/tests/*.spec.ts`, use the Vite mock mode, and run with `pnpm exec playwright test`.

### Scoring models

- `roles.rs` computes 29 FM-style role scores from display attributes and position familiarity; the best role is stored on the player row.
- `ability.rs` implements the community RCA (recommended CA) estimator used by the editor's budget card.
- `moneyball.rs` is the Value Finder: role/CA quality versus transfer value + three years of wages, with position-group percentiles, confidence and risk flags.
- `season_stats.rs` decodes 155-byte season-stat lines from the unlisted span frame; `stats_moneyball.rs` turns them into per-90 metrics and two models: `core` (fields present for every player) and `advanced` (only rows with `advanced_available`, because FM zeroes the advanced block for roughly 37% of players).
- Percentiles are always computed across the full candidate set before request filters are applied, so changing a filter does not move every player's rank. Unknown transfer values and wages are never treated as zero; they are excluded or flagged.

## Invariants and gotchas

- **Never modify the source save.** All read paths are read-only; all edit paths write a new file, refuse to overwrite, and verify by re-parsing.
- **Do not guess layout offsets.** Unsupported builds or malformed sections must return an error or a warning, not a plausible-looking value. Update `docs/fm27-layout.md` and `docs/parser-notes.md` when reverse-engineering new fields.
- **Bump `layout::PARSER_VERSION` whenever parsing behavior or the cached data shape changes.** This invalidates old caches. Do not bump it for ordinary app releases; the app version is independent.
- **Keep schema changes additive and mirrored** in `shared/schema.sql` and `migrate()` in `store.rs`.
- **Unknown values are not zero.** In particular, `transfer_value_state` is a JSON string in SQLite (`"ok"`, `"unset"`, …) and must be deserialized with `serde_json`, and missing fees/wages must stay `None`.
- **Add new Tauri commands in all three places**: the `#[tauri::command]` function plus `generate_handler!`, `lib/api.ts`, and `lib/types.ts`; add a mock fallback if the UI should work in browser mode.
- **All user-facing strings go through `i18n.ts`** in both languages. Code comments and identifiers stay English.
- **Security boundary:** no shell plugin, no arbitrary filesystem access, no network requests except the optional signed updater check. Do not add features that upload save or player data.
- **Real saves are not committed.** Use `FM27_TEST_SAVE` for ignored tests and keep `.venv`/`.ref-fmsave` local.
- Run `cargo fmt --all` before committing; CI denies clippy warnings.

## Documentation map

- `docs/development.md` — repository layout, CLI, build artifacts and validation commands.
- `docs/fm27-layout.md` — verified byte offsets for the save container and records.
- `docs/parser-notes.md` — parser facts, acceptance rules and intentional limits.
- `docs/validation.md` — oracle comparison method and reference counts.
- `docs/features.md` — implemented feature and parser coverage.
- `docs/limitations.md` — supported builds, data gaps, safety boundaries and roadmap.
- `docs/editor.md` — editing workflow, CA/PA behaviour and the RCA estimator.
- `docs/moneyball-feasibility.md`, `docs/moneyball-phase0.md`, `docs/moneyball-phase2.md`, `docs/moneyball-phase3.md`, `docs/moneyball-phase4.md`, `docs/moneyball-phase5.md`, `docs/moneyball-phase6.md`, `docs/moneyball-phase7.md`, `docs/moneyball-phase8.md`, `docs/moneyball-phase9.md`, `docs/league-fixtures-spike.md` — model design, validation, parser spike and known biases.
- `docs/performance-optimization.md` — performance review and implemented optimizations.
- `docs/release.md` — versioning, CI release artifacts, updater signing and notarization.
