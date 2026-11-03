//! Result backtest: what happened to the previous top players at the next save?
//!
//! Usage:
//!   cargo run --release -p fm27-core --example stats_result_backtest -- \
//!     "/path/previous.fm" "/path/next.fm"
//!
//! Prints one JSON object with the previous top 50 and their next-stage
//! minutes, CA, transfer value and score.

use std::collections::HashMap;

use fm27_core::parse_save;
use fm27_core::season_stats::{SeasonStatRow, SeasonStatsKind};
use fm27_core::stats_moneyball::{StatsMoneyballRequest, StatsMoneyballRow};
use fm27_core::store::Store;

fn overall_minutes(stats: &[SeasonStatRow]) -> HashMap<u32, u32> {
    let mut minutes: HashMap<u32, u32> = HashMap::new();
    for row in stats {
        if row.kind != SeasonStatsKind::Overall {
            continue;
        }
        let entry = minutes.entry(row.player_uid).or_insert(0);
        *entry = (*entry).max(row.minutes);
    }
    minutes
}

fn all_rows(path: &str) -> Vec<StatsMoneyballRow> {
    let parsed = parse_save(path).expect("parse save");
    let mut store = Store::open_in_memory().expect("open store");
    let save_id = store.insert_parsed(&parsed).expect("insert save");
    // Running one request materializes the table; then read every row.
    let _ = store
        .stats_moneyball(&StatsMoneyballRequest {
            save_id,
            model: Some("core".to_owned()),
            min_minutes: Some(900),
            ..StatsMoneyballRequest::default()
        })
        .expect("materialize stats moneyball");
    store
        .player_metrics_rows(save_id, "core", 900)
        .expect("materialized rows")
}

fn previous_top(path: &str) -> Vec<StatsMoneyballRow> {
    let rows = all_rows(path);
    fm27_core::stats_moneyball::apply_stats_request(
        &rows,
        &StatsMoneyballRequest {
            model: Some("core".to_owned()),
            ca_min: Some(120),
            age_max: Some(30),
            min_minutes: Some(900),
            page_size: Some(50),
            ..StatsMoneyballRequest::default()
        },
    )
    .expect("previous top")
    .rows
}

fn main() {
    let mut args = std::env::args().skip(1);
    let previous_path = args.next().expect("previous save path");
    let next_path = args.next().expect("next save path");

    let previous = previous_top(&previous_path);
    let next_parsed = parse_save(&next_path).expect("parse next save");
    let next_minutes = overall_minutes(&next_parsed.season_stats);
    let next_players: HashMap<u32, _> = next_parsed
        .players
        .iter()
        .map(|player| (player.uid, player))
        .collect();
    let next_top: HashMap<u32, f64> = all_rows(&next_path)
        .into_iter()
        .filter_map(|row| row.score.map(|score| (row.uid, score)))
        .collect();

    let rows: Vec<_> = previous
        .iter()
        .take(50)
        .map(|row| {
            let next = next_players.get(&row.uid);
            serde_json::json!({
                "uid": row.uid,
                "name": row.name,
                "group": row.position_group,
                "previous_ca": row.ca,
                "next_ca": next.map(|player| player.ca),
                "previous_value": row.transfer_value,
                "next_value": next.and_then(|player| player.transfer_value),
                "previous_minutes": row.minutes,
                "next_minutes": next_minutes.get(&row.uid).copied(),
                "previous_score": row.score,
                "next_score": next_top.get(&row.uid).copied(),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::json!({
            "previous": previous_path,
            "next": next_path,
            "rows": rows,
        })
    );
}
