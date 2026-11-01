# Safety, limitations and roadmap

## Security boundaries

- Read commands and default browsing are read-only. The desktop edit dialog and
  the optional `edit` CLI subcommand write a **new** save file, refuse to
  overwrite their source, and verify the result by re-parsing it.
- Never reads game memory or injects into game processes.
- Does not enable Tauri `shell` / arbitrary filesystem plugins; only files
  selected by the user are used.
- Apart from a user-triggered update check, makes no network requests; the
  updater only contacts the configured GitHub Releases endpoint and downloads a
  signed package, and no save or player data is uploaded.
- Hidden attributes are off by default; enabling them shows a single-player-only
  warning.
- Databases, logs and snapshots are written only to
  `~/Library/Application Support/com.fm27scout.desktop/`; snapshots are deleted
  after parsing.

## Known limitations

- Only build `27.0.0+2486695` has been fully verified; other FM27 builds are read
  in degraded mode with a warning, and missing fields are never guessed.
- Country names are bundled; competition names are not stored in the save, so
  `shared/competition-names.csv` must supply them keyed by the editor
  `database_id` (the competition/stage index itself is parsed and cached).
- Injury data comes from the `injury_manager` section: FM27 keeps current
  expected-return windows in the two stride-14 arrays, while the log array holds
  the rolling injury history. "Injured" is derived from those expected-return
  windows, so it no longer depends on the typed array being populated.
- Contract clauses, loans and contract chains are parsed; a few unobserved
  fallback structures are explicitly left blank.
- Windows/Linux builds are not available yet.
- CI publishes an unsigned DMG; on first launch, use
  **System Settings → Privacy & Security → Open Anyway**. Developer ID
  signing/notarization is optional, not used by CI, and requires a paid Apple
  Developer account.
- CI publishes signed updater artifacts and `latest.json` only after the updater
  keypair has been configured as GitHub Actions secrets; without them, the in-app
  update check is disabled.
- Save editing is experimental. A malformed or unsupported edit can corrupt the
  output save or make it unreadable. Keep a backup and verify the result in game
  before relying on it.

## Roadmap

1. Accessibility polish, Intel hardware regression, and more real-save edge-case
   tests.
