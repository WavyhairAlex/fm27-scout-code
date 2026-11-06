# Release notes

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
