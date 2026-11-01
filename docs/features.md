# Features and parser coverage

## Save parsing

- Reads FM27 `27.0.0+2486695` saves directly; the container, zstd frames and
  section directories are parsed in Rust.
- Finder double-click and drag-and-drop open `.fm` files. A read-only snapshot is
  copied before parsing so a save in progress cannot be read half-written.
- Players: 46,273 records with attributes, hidden attributes, CA/PA, positions,
  personality, traits, contracts, value and status.
- Clubs: 26,214 records with team mapping, reputation and last season's league
  position.
- Competitions: 1,724 competitions and 3,459 stage rows from the save's stage
  table; 1,586 carry an editor database id, which links to the optional user name
  CSV.
- Contracts: 44,341 records with wages, start/end dates, loans and clauses.
- SQLite cache and server-side pagination; the CLI and desktop app share
  `fm27-core`.
- Parsing takes about 1.3 seconds in a release build on an M-series Mac,
  including the competition index.

## Validation

- Full field-by-field comparison against the Python `fmsave` oracle for shared
  fields: **0 differences**.
- Derived fields such as names, roles and injuries are covered by regression
  tests.
- Reference results: [validation.md](validation.md).

## Desktop UI

- Filters, multi-column sorting and player profiles.
- 2–4 player comparison.
- CSV/JSON export.
- Hidden-attribute toggle, off by default.
- Light and dark themes.
- Analysis centre: age/CA distributions, club strength ranking, squad depth and
  a wonderkid radar (age + PA + CA gap).
- Quick filters: wonderkids, expiring contracts, high potential, injured /
  injury history / suspended / on loan / free agent / available.
- Role scores: 29 FM-style roles weighted by key attributes and combined with
  position familiarity; the detail panel shows every role and the best role.
- Growth history: each parse records a snapshot keyed by save name + game date;
  the detail panel shows CA / PA / value history.
- Linked filtering: club, nationality, gender and status counts update live with
  the current filters.

## Save editor

- CA/PA, attributes, hidden attributes, determination, personality and traits.
- Edits are staged per player and written together to one new save file.
- CA rebalancing warnings before writing.
- CA budget estimator: live estimate from edited attributes using community FM
  position weights, with over/under-budget guidance and position selection.
- The source save is never modified; existing output files are not overwritten;
  the result is verified by re-parsing it.
- See [editor.md](editor.md) for the workflow and safety notes.

## Localization and release

- English / Simplified Chinese UI, following the system language by default and
  falling back to English for unsupported languages.
- The native menu follows the selected language.
- Universal Binary DMG built by CI; optional Developer ID signing/notarization
  for local releases.
- In-app updates: Tauri Updater checks the GitHub Releases manifest, verifies the
  signed update package and restarts after installation. Signing keys are set up
  once with `scripts/setup-updater-signing.sh`.
- See [release.md](release.md) for the release process.

Real saves are not committed to the repository; specify one with an environment
variable during development.
