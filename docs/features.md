# Features and parser coverage

## Save parsing

- Reads FM27 `27.0.0+2486695`, `27.0.0+2493426`, `27.0.1+2494687` and
  `27.0.1+2503222` saves directly; the container, zstd frames and section
  directories are parsed in Rust.
- Finder double-click and drag-and-drop open `.fm` files. A read-only snapshot is
  copied before parsing so a save in progress cannot be read half-written.
- Players: 46,273 records with attributes, hidden attributes, CA/PA, positions,
  personality, traits, contracts, value and status.
- Staff: club staff directory with the current job, CA/PA, wage and contract dates,
  21 profile attributes and a 15-role suitability vector. Unreadable attributes or
  roles stay absent instead of becoming zero.
- Clubs: more than 26,000 records with team mapping, reputation, last season's
  league position, the current league for the top divisions the resolver covers,
  and, for tracked clubs, training, youth, junior-coaching and youth-recruitment
  facility ratings. English Youth Level (EPPP category 1–4) is
  read from the game's `eng_youth_grades.edt` file.
- Club finances: monthly snapshots (balance, transfer budget, wage budget,
  payroll, income/expenditure and net) plus corporate facilities, read from each
  club record. Only the league nations the save tracks keep these rows.
- Stadiums: the save's ground table with capacities, pitch dimensions, dates and
  owner club. The save stores no direct club -> ground pointer, so home-ground
  use is not derived yet.
- Club search: full club table with nation, reputation, finance, facilities and
  stadium filters, plus a club profile with squad, finance history and owned
  stadiums.
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
- Separate staff workspace with club/job/nation filters, attribute and role-suitability
  rules, profile attributes, role scores, contract terms and sortable role columns.
  Staff are read-only.
- Separate club workspace with searchable club table, overview, squad, finance
  history and owned-stadium/facility-rating panels. The squad lists CA and PA
  with sortable headers, and facility ratings use the game's ten named tiers.
- 2–4 player comparison.
- CSV/JSON export.
- Scouting modes: Full shows CA/PA and attributes; Fair play hides CA/PA and
  growth; Moneyball also hides attributes, hidden attributes, personality and
  role scores and prices players without CA/PA. Exports still contain CA/PA and
  show a warning outside Full mode.
- Hidden attributes and personality are always included in the attribute views
  (table, filters, profile and comparison); no separate toggle is required.
- Light and dark themes.
- Analysis centre: the save's managed club is detected from the `humans` section
  and checked against the save summary. The Overview tab opens on that club's
  squad: size, age, CA/PA, wages, value, position depth and contract expiry.
- Quick filters: wonderkids, expiring contracts, high potential, injured /
  injury history / suspended / on loan / free agent / available.
- Player origin: system-generated newgens are marked in the table and profile and
  can be included, excluded or isolated with the player-origin filter.
- Other nationalities: the profile lists every other eligible nation the save
  stores, resolved through the bundled nation-name table.
- Role scores: 29 FM-style roles weighted by key attributes and combined with
  position familiarity; the detail panel shows every role and the best role.
- Value finder (experimental): role/CA quality against transfer value plus three
  years of wages, with position-group percentiles, confidence, market/contract
  flags and explainable score components. It is not a stats-based model; unknown
  fees and wages are excluded by default. Contract-expiry, market, confidence and
  injury-history filters are available.
- Stats moneyball (experimental, v3): npxG/xA, shooting, passing, defensive and
  possession percentiles in eight position groups (GK/DC/FB/DM/MC/WM/AMC/ST),
  with overall / league-only / last-12-months windows. Each row carries a quality
  percentile, an out-of-fold fair price and value gap, a separate bargain score,
  an 80% confidence band, risk flags, per-metric drivers and same-group similar
  players. The core model scores every player; goalkeepers are ranked separately
  and the advanced model only scores rows with the advanced stat block. Contract,
  market, confidence, injury and fee filters are available, and the current view
  exports to CSV or JSON.
- Squad needs: the club inspector's squad tab ranks a club's weakest position
  groups by current stats quality and lists same-tier, fee-known upgrades with
  quality gain, fair price and value gap. It uses 300+ minute samples and excludes
  loanee stats.
- Transfer scouting (Full mode): the Scouting tab works from the managed club and
  separates role fit from signing probability. Advice is relative to the club's own
  squad (first team, rotation, future), uses an optional role filter, and explains
  every probability with named reasons. Fee and wage estimates account for contract
  length, listing, squad status, release clauses, the transfer budget and wage room.
  The same tab shows a line-by-line squad depth chart and the players a scout would
  sell, loan out or release. Unknown values stay unknown and are never treated as
  zero.
- Growth history: each parse records a snapshot keyed by save name + game date;
  the detail panel shows CA / PA / value history.
- Linked filtering: club, nationality, gender and status counts update live with
  the current filters.

## Save editor

- CA/PA, attributes, hidden attributes, determination, personality and traits.
- Current permanent contract: weekly wage and end date, with the same in-effect
  contract selection the parser uses; free agents, contracts without terms,
  expired contracts and loan terms are read-only.
- Edits are staged per player and written together to one new save file.
- CA rebalancing warnings before writing.
- CA budget estimator: live RCA estimate from edited attributes using the
  reverse-engineered FM formula (position weights, both feet, position-combination
  cost and familiarity scaling), with over/under-budget guidance and position
  selection.
- Low-risk club edits: reputation, latest-month balance, transfer budget
  remaining, weekly wage budget, corporate facilities, training/youth/
  junior-coaching/youth-recruitment ratings and owned-stadium capacities. Club
  edits write one new save copy and verify every changed field.
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
