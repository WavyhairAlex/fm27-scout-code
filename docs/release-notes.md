# Release notes

## 1.4.0 — Contract editing and display currency

### Added

- Player contract editing in the editor and CLI:
  - weekly wage and end date for the current permanent contract;
  - the same in-effect contract selection as the profile, so the written record
    is the one the app displays;
  - free agents, contracts without terms, expired contracts and loan-only
    records stay read-only; loan end dates and future pre-contracts are
    unchanged;
  - validation plus warnings for wage-bill and financial-rule effects.
- Display currency in Settings:
  - save units by default, plus common currencies and the full in-game
    currency list;
  - approximate mid-market rates (2026-11-06) applied to transfer values,
    wages, club finances and contract clauses;
  - filters and edit inputs stay in save units; the save itself is never
    converted.
- Contract changes in the pending-edits summary, with English and Simplified
  Chinese copy for the new fields.

### Fixed

- Season stats: duplicate lines for the same player, competition kind and team
  (for example after a mid-season transfer) are merged into one season total
  instead of failing the SQLite insert.
- Contract decoding keeps the club UID when the club name is unavailable, so
  the editor can still locate the in-effect contract.

### Notes

- Contract editing keeps the existing single-player/backup warning. FM may
  recalculate club wage totals, budgets and financial-rule totals after the
  game advances.
- Currency conversion is display-only; rates are a snapshot and can differ
  from FM's in-game rates.

## 1.3.0 — Stats Moneyball (Phase 2)

Experimental only; this is not a promise of transfer-market accuracy.

### Added

- FM27 season-statistics parser for the unlisted frame after `non_pl_hist_ls`:
  minutes, xG/xA, shooting, passing, tackles, cards, goalkeeper saves/xGP.
- `player_season_stats` SQLite table, with advanced fields stored as `NULL`
  when FM did not record the advanced-stat block.
- Stats Moneyball tab in the analysis centre:
  - core model (all outfield players) and advanced model (tracked subset);
  - position-aware percentiles against three-year cost;
  - position quick tabs and separate goalkeepers;
  - Win-now / Resale / Budget presets and local saved views;
  - contract-expiry, market-status, minimum-confidence and injury-history filters;
  - expandable “Why?” panel with drivers, sample, confidence and cost breakdown.
- `player_metrics` materialized table plus in-memory cache; scores survive a
  restart and can be queried directly with SQL.
- Multi-save top-50 stability and short-horizon result backtests.

### Known limits

- League strength is proxied by club reputation; direct competition coefficients
  are not available from the save.
- Advanced stats are missing for roughly 37% of players; the advanced model
  excludes them instead of treating zeroes as data.
- Transfer value is not an actual fee; signing fees, agent fees and wage clauses
  are not included.
- CSV/JSON export of the stats table is intentionally not part of this release.
