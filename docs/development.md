# Development

## Repository layout

```text
apps/desktop/          Tauri 2 app + React UI
crates/fm27-core/      Rust parser, SQLite, queries, export
crates/fm27-cli/       fm27scout CLI
shared/schema.sql      SQLite schema
scripts/               Python oracle export and Rust/oracle comparison
docs/                  Layout, parser and validation notes
```

## Requirements

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

## Build artifacts

Release build paths:

```text
target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_<version>_universal.dmg
target/universal-apple-darwin/release/bundle/macos/FM27 Scout.app
```

Local development builds can also produce a `target/release/bundle` for the
current architecture only.

Universal Binary / signing / notarization use `scripts/release-macos.sh`; see
[release.md](release.md). CI publishes a GitHub Release with an unsigned Universal
DMG, signed updater artifacts and SHA-256 checksums on `v*` tags once the updater
secrets are configured. The CI DMG is not notarized; on first launch, use
**System Settings → Privacy & Security → Open Anyway**.

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

# Clubs with finance, facilities and stadiums
./target/release/fm27scout clubs "/path/to/save.fm" --details --output clubs.json

# Experimental: edit one player into a new save file (the source is never modified)
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --attr consistency=100 \
  --personality professionalism=20 --output edited.fm

# Experimental: edit low-risk club fields into a new save file
./target/release/fm27scout edit-club "/path/to/save.fm" --club-uid 1155 \
  --reputation 6500 --corporate-facilities 8 \
  --training-facilities 18 --youth-facilities 16 \
  --junior-coaching 15 --youth-recruitment 14 \
  --balance 30000000 --transfer-budget 6000000 --wage-budget 600000 \
  --stadium-uid 43156154 --all-seater 1300 --expansion 6000 \
  --output club-edited.fm
```

The `edit` subcommand is documented in [editor.md](editor.md). It supports builds
`27.0.0+2486695` and `27.0.0+2493426`, and always writes a separate output file. CA is the
attribute budget: FM rebalances CA-weighted visible attributes over roughly a
month, while hidden attributes, determination, natural fitness, flair, aggression,
personality and traits stay stable. Visible attributes alone are unstable; CA plus
a few key attributes is the controlled workflow.

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

# Club, finance, facilities and stadium comparison
.venv/bin/python scripts/oracle_clubs.py "/path/to/save.fm" --output /tmp/oracle_clubs.json
./target/release/fm27scout clubs "/path/to/save.fm" --details --output /tmp/rust_clubs.json
.venv/bin/python scripts/compare_clubs.py /tmp/oracle_clubs.json /tmp/rust_clubs.json
```

Reference results are in [validation.md](validation.md). The name-table format is
described in [name-tables.md](name-tables.md), and the release process in
[release.md](release.md).
