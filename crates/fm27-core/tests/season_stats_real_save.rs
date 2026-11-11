//! Real-save regression test for the season-statistics frame.
//!
//! Set `FM27_TEST_SAVE` to a verified-build career save and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -- --ignored season_stats
//! ```

use fm27_core::parse_save;
use fm27_core::season_stats::SeasonStatsKind;
use fm27_core::stats_moneyball::StatsMoneyballRequest;
use fm27_core::store::Store;

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn reads_the_reference_career_season_stats() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");

    // The reference career has ~156k rows; a small test database can have ~56k.
    assert!(
        parsed.season_stats.len() > 50_000,
        "expected a full season-statistics section, got {} rows",
        parsed.season_stats.len()
    );
    let player_uids: std::collections::HashSet<u32> =
        parsed.players.iter().map(|player| player.uid).collect();
    assert!(
        parsed
            .season_stats
            .iter()
            .all(|row| player_uids.contains(&row.player_uid)),
        "every statistics row should join a decoded player"
    );
    assert!(
        parsed
            .season_stats
            .iter()
            .any(|row| row.kind == SeasonStatsKind::Overall && row.minutes >= 900),
        "expected at least one overall row with a meaningful sample"
    );
    let stat_keys: std::collections::HashSet<(u32, SeasonStatsKind, u32)> = parsed
        .season_stats
        .iter()
        .map(|row| (row.player_uid, row.kind, row.team_id.unwrap_or(0)))
        .collect();
    assert_eq!(
        stat_keys.len(),
        parsed.season_stats.len(),
        "the SQLite cache needs one row per player, competition kind and team"
    );
    assert!(
        parsed
            .season_stats
            .iter()
            .any(|row| !row.advanced_available),
        "expected at least one row without the advanced-stat block"
    );

    let mut store = Store::open_in_memory().expect("open in-memory store");
    let save_id = store.insert_parsed(&parsed).expect("insert parsed save");
    assert_eq!(
        store.season_stats_count(save_id).expect("count"),
        parsed.season_stats.len()
    );

    let page = store
        .stats_moneyball(&StatsMoneyballRequest {
            save_id,
            ca_min: Some(120),
            age_max: Some(30),
            page_size: Some(50),
            ..StatsMoneyballRequest::default()
        })
        .expect("stats moneyball");
    assert!(page.total > 100, "expected a non-trivial constrained list");
    assert!(page.rows.iter().all(|row| row.ca >= 120));
    assert!(page.rows.iter().all(|row| row.age.unwrap_or(0) <= 30));
    assert!(page.rows.iter().all(|row| row.score.is_some()));

    let page_two = store
        .stats_moneyball(&StatsMoneyballRequest {
            save_id,
            ca_min: Some(120),
            age_max: Some(30),
            page: Some(1),
            page_size: Some(50),
            ..StatsMoneyballRequest::default()
        })
        .expect("cached second page");
    assert_eq!(page_two.total, page.total);
    assert!(!page_two.rows.is_empty());
    assert!(
        page_two
            .rows
            .iter()
            .all(|row| !page.rows.iter().any(|first| first.uid == row.uid))
    );

    store.clear_stats_cache();
    let from_sqlite = store
        .stats_moneyball(&StatsMoneyballRequest {
            save_id,
            ca_min: Some(120),
            age_max: Some(30),
            page_size: Some(50),
            ..StatsMoneyballRequest::default()
        })
        .expect("persistent cache");
    assert_eq!(from_sqlite.total, page.total);
    assert_eq!(from_sqlite.rows[0].uid, page.rows[0].uid);
    assert_eq!(from_sqlite.rows[0].name, page.rows[0].name);
    assert_eq!(from_sqlite.rows[0].ca, page.rows[0].ca);
    assert_eq!(from_sqlite.rows[0].minutes, page.rows[0].minutes);
    assert_eq!(from_sqlite.rows[0].score, page.rows[0].score);
    assert_eq!(
        from_sqlite.rows[0].production_tier,
        page.rows[0].production_tier
    );
    assert_eq!(from_sqlite.rows[0].surplus, page.rows[0].surplus);
    assert_eq!(
        from_sqlite.rows[0].drivers.len(),
        page.rows[0].drivers.len()
    );
    assert_eq!(
        from_sqlite.rows[0]
            .drivers
            .iter()
            .map(|driver| driver.key.as_str())
            .collect::<Vec<_>>(),
        page.rows[0]
            .drivers
            .iter()
            .map(|driver| driver.key.as_str())
            .collect::<Vec<_>>()
    );

    let metric_count = store
        .player_metrics_count(save_id, "core")
        .expect("metric count");
    let metric_rows = store
        .player_metrics_rows(save_id, "core")
        .expect("metric rows");
    assert!(metric_count > 1_000);
    assert_eq!(metric_rows.len(), metric_count);
    let target = metric_rows
        .iter()
        .find(|row| row.uid == page.rows[0].uid)
        .expect("page row in materialized metrics");
    assert_eq!(target.score, page.rows[0].score);

    let advanced = store
        .stats_moneyball(&StatsMoneyballRequest {
            save_id,
            model: Some("advanced".to_owned()),
            ca_min: Some(120),
            age_max: Some(30),
            page_size: Some(50),
            ..StatsMoneyballRequest::default()
        })
        .expect("advanced stats moneyball");
    assert!(advanced.total > 100);
    assert!(advanced.rows.iter().all(|row| row.advanced_available));
}
