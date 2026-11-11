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

- Builds `27.0.0+2486695`, `27.0.0+2493426`, `27.0.1+2494687` and
  `27.0.1+2503222` have been fully verified; other FM27 builds are read in
  degraded mode with a warning, and missing fields are never guessed. Editing is
  limited to the verified builds.
- Club leagues are resolved from the competition entries in a full club record. The
  current resolver covers the top divisions of England, Italy, Spain, Germany,
  France, the Netherlands and Portugal; lower divisions and other nations stay
  blank rather than showing a guessed league.
- Country names are bundled; competition names are not stored in the save.
  `shared/competition-names.csv` maps the editor `database_id` to a name and is
  populated for the local save; competitions without a database id, a
  conflicting id, or a CSV row still fall back to `Competition #id` (the
  competition/stage index itself is parsed and cached).
- Injury data comes from the `injury_manager` section: FM27 keeps current
  expected-return windows in the two stride-14 arrays, while the log array holds
  the rolling injury history. "Injured" is derived from those expected-return
  windows, so it no longer depends on the typed array being populated.
- Contract clauses, loans and contract chains are parsed; a few unobserved
  fallback structures are explicitly left blank.
- The managed club is read from the first human manager only, and only on the
  verified builds. When the manager is between jobs, or when the summary links
  several clubs and no contract record settles the choice, the club stays unknown
  and the analysis centre falls back to save-wide figures and the club picker.
- Staff rows cover people with a club job. National-team appointments have a
  team id but no club mapping and are excluded, as are people a club lists
  without any held or no-contract role record and the human manager, whose
  object has no standard ability block. On the reference saves that leaves 39 of
  13,809 rows for Milan and 52 of 14,225 for Parma. Staff profile attributes and
  role suitability stay blank when their blocks cannot be read uniquely; staff
  are read-only.
- System-generated players are identified by the game's Unique ID range
  (`2,002,000,000` and above). A generated person that reuses a deleted database
  ID cannot be told apart from a database person by the save alone.
- Monthly club finances, corporate facilities and the detailed facility ratings
  exist only for the league nations the save tracks. In the reference FM27 save
  that is 122 clubs out of more than 26,000; other clubs show their basic
  profile, squad and owned stadiums but no finance series or facility ratings.
- Corporate, training, youth, junior-coaching and youth-recruitment ratings are
  parsed for the clubs that keep a finance chain. English Youth Level is read
  from the installed game's `eng_youth_grades.edt`; it is not stored in the save,
  so it is blank when the game files are unavailable or the club is not English.
- The stadium table stores ownership but no direct club -> ground pointer.
  Home-ground use would require the fixture calendar and is not implemented yet.
- The Value finder is an experimental value-for-money ranking, not a
  stats-based Moneyball model. It uses role/CA quality and transfer value plus
  wages; `transfer_value` is not an actual transfer fee, players with unknown
  fees or wages are excluded by default, and league strength is not adjusted.
- Transfer scouting is an estimator. Fit uses the editor's role weights and
  position familiarity, not FM's own role ratings. Signing probability and the
  fee/wage estimates are hand-tuned from transfer value, contract status, club
  reputation, personality, budget and wage room; they are not calibrated against
  completed transfers. Release clauses use the FM27 clause kinds
  (`0, 1, 2, 16, 17, 18, 27, 31, 55, 56`); a clause of zero is ignored because
  its behaviour is unverified. Relegation and promotion conditions are not
  modelled, so those clauses are treated as live caps. The squad view works from
  every player whose `club_uid` is the managed club, not a separately parsed
  senior-team list, so reserve and youth players can appear. Tactics are not
  read; the role filter is manual.
- Stats moneyball is also experimental. FM27 zeroes the advanced-stat block
  (key passes, progressive passes, interceptions, headers, clearances, blocks,
  sprints, saves, xGP) for about 37% of players; those zeroes are stored as
  `NULL`, and the advanced model excludes those rows. The core model only uses
  fields present for every player (xG/xA, shots, passing, tackles, rating).
  Goalkeeper save/xGP coverage is lower, transfer value is still not an actual
  fee, and league strength is proxied by club reputation rather than adjusted
  directly.
- Club editing is limited to fixed-width, low-risk fields: reputation, the
  latest finance snapshot, corporate facilities, training/youth/junior-coaching/
  youth-recruitment ratings and owned-stadium capacities. It does not edit names,
  nations, team lists, affiliations, Youth Level or stadium owners.
- Player contract editing is limited to the current permanent contract's weekly
  wage and end date. Free agents, loan-only records, contracts without terms,
  expired contracts, loan end dates, staff contracts and contract clauses
  (release fees, extension options, bonuses) are read-only. FM may recalculate
  club wage totals and budgets after a wage edit.
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
- The CA budget estimator implements a community reconstruction of FM's
  recommended-CA calculation, not an official formula. It matches FM27 closely,
  but the stored CA remains authoritative and developed or custom/edited players
  can differ from the RCA.

## Roadmap

1. Accessibility polish, Intel hardware regression, and more real-save edge-case
   tests.
