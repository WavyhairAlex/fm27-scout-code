English | [中文](README.zh-CN.md)

# FM27 Scout

A macOS scouting workbench for Football Manager 27 with an optional, experimental save editor. Built with Tauri 2, a Rust parsing core and a React/TypeScript UI.

> Unofficial tool, not affiliated with Sports Interactive / SEGA. Reads are read-only. Editing writes a separate save file and never modifies the source. No save or player data is uploaded.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-dark.webp">
  <img src="docs/screenshots/main-light.webp" alt="Main window with filters, sortable player table and player profile panel">
</picture>

## Features

- Reads FM27 `27.0.0+2486695` and `27.0.0+2493426` saves directly; SQLite cache and server-side pagination keep 46k+ players responsive.
- Filters, multi-column sorting, player profiles, 2–4 player comparison, CSV/JSON export, role scores, growth history and an analysis centre.
- Club workspace: searchable club table, squad summary, monthly finances, owned stadiums and facility ratings (corporate, training, youth, junior coaching, youth recruitment and youth level). Finance rows and detailed facility ratings exist only for the league nations the save tracks.
- Optional save editor for CA/PA, attributes, hidden attributes, determination, personality and traits. Edits are staged across players and written in one batch to a new save.
- Low-risk club edits: club reputation, latest-month finance snapshot (balance, transfer budget, wage budget), corporate facilities, training/youth/junior-coaching/youth-recruitment ratings and owned-stadium capacities. The source save is never modified.
- CA budget estimator using the reverse-engineered FM RCA formula (position weights, both feet, multi-position cost), with over/under-budget guidance.
- Experimental Value finder: role/CA quality against transfer value and wages, with confidence and risk flags. Not a stats-based model yet.
- English / Simplified Chinese UI that follows the system language.
- Universal macOS release published by CI, with optional Developer ID signing/notarization.

See [docs/features.md](docs/features.md) for parser coverage, validation results and the full feature list.

## Install

Download the latest Universal DMG from [GitHub Releases](https://github.com/shlk20/fm27-scout-code/releases). Requires macOS 14+.

The CI build is not notarized. On first launch, use **System Settings → Privacy & Security → Open Anyway**.

## Development

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

See [docs/development.md](docs/development.md) for the workspace layout, CLI, build artifacts and validation commands.

## Documentation

- [Features and parser coverage](docs/features.md)
- [Save editor](docs/editor.md)
- [Development, CLI and validation](docs/development.md)
- [Safety, limitations and roadmap](docs/limitations.md)
- [Release process](docs/release.md)
- [FM27 save layout](docs/fm27-layout.md)
- [Name tables](docs/name-tables.md)

## Safety

- The source save is never modified. Editing writes a new file, refuses to overwrite existing output, and re-parses the result.
- No game memory reading, process injection, shell plugin or arbitrary filesystem access.
- No network requests except an optional update check; no save or player data leaves the machine.
- Hidden attributes are off by default.

See [docs/limitations.md](docs/limitations.md) for details.

## License

MIT. Parsing logic references and ports [fmsave](https://github.com/rhiever/fmsave). See [THIRD_PARTY.md](THIRD_PARTY.md).
