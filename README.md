English | [中文](README.zh-CN.md)

# FM27 Scout

A read-only Football Manager 27 scouting tool for macOS: Tauri 2 + Rust parsing core + React/TypeScript UI.

> Unofficial tool, not affiliated with Sports Interactive / SEGA. Saves are read-only; the game is never modified, and no data is sent over the network or uploaded anywhere.

## Screenshots

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-dark.webp">
  <img src="docs/screenshots/main-light.webp" alt="Main window with filters, sortable player table and player profile panel">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/analysis-dark.webp">
  <img src="docs/screenshots/analysis-light.webp" alt="Analysis centre with age and CA distributions, club strength and contract expiry">
</picture>

## Current status

The 0.2.1 runnable release is complete:

- Reads FM27 `27.0.0+2486695` saves directly; container, zstd and section directories are all parsed by Rust.
- Supports Finder double-click / drag-and-drop for `.fm` files; a read-only snapshot is copied before parsing so a save in progress cannot be read half-written.
- Players: 46,273 records with attributes, hidden attributes, CA/PA, positions, personality, traits, contracts, value and status.
- Clubs: 26,214 records with team mapping, reputation and last season's league position.
- Competitions: 1,724 competitions and 3,459 stage rows from the save's stage table; 1,586 carry an editor database id, which is linked to the optional user name CSV.
- Contracts: 44,341 records with wages, start/end dates, loans and clauses.
- Full field-by-field comparison against the Python `fmsave` oracle for shared fields: **0 differences**; derived fields such as names, roles and injuries are covered by regression tests.
- Parsing takes about **1.3 seconds** (release build, M-series Mac), including the competition index.
- SQLite cache + server-side pagination; the CLI and desktop app share `fm27-core`.
- React UI: filters, multi-column sorting, player details, 2–4 player comparison, CSV/JSON export, hidden-attribute toggle, light/dark themes.
- Multilingual UI: English / Simplified Chinese, follows the system language by default and falls back to English for unsupported languages; can be switched in Settings, and the native menu follows the selection.
- Analysis centre: age/CA distributions, club strength ranking, squad depth and wonderkid radar (age + PA + CA gap).
- Quick filters: wonderkids, expiring contracts, high potential, injured / injury history / suspended / on loan / free agent / available.
- Role scores: 29 FM-style roles weighted by key attributes and combined with position familiarity; the detail panel shows every role and the best role.
- Growth history: each parse records a snapshot keyed by save name + game date; the detail panel shows CA / PA / value history.
- Gender and names: parses club gender (men's / women's) and bundles names for 208 countries/regions; competition names are supplied through a user CSV.
- Linked filtering: club, nationality, gender and status counts update live with the current filters.
- Release setup: Universal Binary DMG published by CI; optional Developer ID signing/notarization for local releases.
- In-app updates: Tauri Updater checks the GitHub Releases manifest, verifies the signed update package, and restarts after installation; signing keys are configured once through `scripts/setup-updater-signing.sh`.

Real saves are not committed to the repository; specify one with an environment variable during development.

## Layout

```text
apps/desktop/          Tauri 2 app + React UI
crates/fm27-core/      Rust parser, SQLite, queries, export
crates/fm27-cli/       fm27scout CLI
shared/schema.sql      SQLite schema
scripts/               Python oracle export and Rust/oracle comparison
docs/                  Layout, parser and validation notes
```

## Build artifacts

Locally verified builds:

```text
target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_0.2.1_universal.dmg
target/universal-apple-darwin/release/bundle/macos/FM27 Scout.app
```

Local development builds can also produce a `target/release/bundle` for the current architecture only.

Universal Binary / signing / notarization use `scripts/release-macos.sh`; see
[docs/release.md](docs/release.md). CI publishes a GitHub Release with an unsigned Universal DMG,
signed updater artifacts and SHA-256 checksums on `v*` tags once the updater secrets are configured.
The CI DMG is not notarized; on first launch, use **System Settings → Privacy & Security → Open Anyway**.

## Development environment

- macOS 14+
- Rust stable (`rustup`)
- Node 20+ / pnpm
- Python 3.12 + `fmsave` (only for oracle comparison)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Frontend dependencies
cd apps/desktop
pnpm install

# Development mode
pnpm tauri dev
```

## CLI

```bash
cargo build --release -p fm27-cli

# Container information (does not decompress game_db)
./target/release/fm27scout inspect "/path/to/save.fm"

# Full parse report
./target/release/fm27scout validate "/path/to/save.fm"

# First 1,000 players
./target/release/fm27scout players "/path/to/save.fm" --limit 1000 --format json

# Export CSV
./target/release/fm27scout players "/path/to/save.fm" --limit 50000 --format csv --output players.csv

# Parse and write the SQLite cache
./target/release/fm27scout index "/path/to/save.fm" --db fm27scout.db

# Competition index and editor database ids
./target/release/fm27scout competitions "/path/to/save.fm" --output competitions.json
```

## Validation

```bash
# Rust unit tests
cargo test -p fm27-core

# Real-save regression tests (requires a save)
FM27_TEST_SAVE="$HOME/Library/Application Support/Sports Interactive/Football Manager 27/games/BB KING - Casciavit.fm" \
  cargo test -p fm27-core --release -- --ignored

# Python oracle export + comparison
python3 -m venv .venv
.venv/bin/pip install -e .ref-fmsave    # local clone for development
.venv/bin/python scripts/oracle_export.py --output /tmp/oracle.json
.venv/bin/python scripts/compare_outputs.py /tmp/oracle.json /tmp/rust.json

# Competition index comparison
.venv/bin/python scripts/oracle_competitions.py --output /tmp/oracle_competitions.json
./target/release/fm27scout competitions "/path/to/save.fm" --output /tmp/rust_competitions.json
.venv/bin/python scripts/compare_competitions.py /tmp/oracle_competitions.json /tmp/rust_competitions.json
```

Reference results are in [docs/validation.md](docs/validation.md). The name-table format is
described in [docs/name-tables.md](docs/name-tables.md), and the release process in
[docs/release.md](docs/release.md).

## Security boundaries

- Never modifies `.fm` saves, reads game memory, or injects into game processes.
- Does not enable Tauri `shell` / arbitrary filesystem plugins; only files selected by the user are used.
- Apart from a user-triggered update check, makes no network requests; the updater only contacts the configured GitHub Releases endpoint and downloads a signed package, and no save or player data is uploaded.
- Hidden attributes are off by default; enabling them shows a single-player-only warning.
- Databases, logs and snapshots are written only to `~/Library/Application Support/com.fm27scout.desktop/`; snapshots are deleted after parsing.

## Known limitations

- Only build `27.0.0+2486695` has been fully verified; other FM27 builds are read in degraded mode with a warning, and missing fields are never guessed.
- Country names are bundled; competition names are not stored in the save, so `shared/competition-names.csv` must supply them keyed by the editor `database_id` (the competition/stage index itself is parsed and cached).
- Injury data currently comes from the `injury_manager` history. If a save has no FM "expected return" typed rows, "injured" is empty while "injury history" still works.
- Contract clauses, loans and contract chains are parsed; a few unobserved fallback structures are explicitly left blank.
- Windows/Linux builds are not available yet.
- CI publishes an unsigned DMG; on first launch, use **System Settings → Privacy & Security → Open Anyway**. Developer ID signing/notarization is optional, not used by CI, and requires a paid Apple Developer account.
- CI publishes signed updater artifacts and `latest.json` only after the updater keypair has been configured as GitHub Actions secrets; without them, the in-app update check is disabled.

## Roadmap

1. Accessibility polish, Intel hardware regression, and more real-save edge-case tests.

## License and attribution

The project code is MIT licensed. Parsing logic references and ports [fmsave](https://github.com/rhiever/fmsave) (MIT). See [THIRD_PARTY.md](THIRD_PARTY.md).
