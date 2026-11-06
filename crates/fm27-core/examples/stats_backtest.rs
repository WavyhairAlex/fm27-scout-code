//! Developer backtest for the stats-based Moneyball model.
//!
//! Usage:
//!   cargo run --release -p fm27-core --example stats_backtest -- \
//!     "/path/base.fm" "/path/v02.fm" "/path/v03.fm"
//!
//! Prints one JSON object per save and model. The output is intentionally
//! line-oriented so a small analysis script can compare player sets.

use fm27_core::parse_save;
use fm27_core::stats_moneyball::StatsMoneyballRequest;
use fm27_core::store::Store;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: stats_backtest SAVE [SAVE ...]");
        std::process::exit(2);
    }
    for path in paths {
        let parsed = match parse_save(&path) {
            Ok(parsed) => parsed,
            Err(error) => {
                eprintln!("{path}: {error}");
                continue;
            }
        };
        let label = parsed.summary.file_name.trim_end_matches(".fm").to_owned();
        let mut store = match Store::open_in_memory() {
            Ok(store) => store,
            Err(error) => {
                eprintln!("{label}: store: {error}");
                continue;
            }
        };
        let save_id = match store.insert_parsed(&parsed) {
            Ok(save_id) => save_id,
            Err(error) => {
                eprintln!("{label}: insert: {error}");
                continue;
            }
        };
        for model in ["core", "advanced"] {
            let page = match store.stats_moneyball(&StatsMoneyballRequest {
                save_id,
                model: Some(model.to_owned()),
                ca_min: Some(120),
                age_max: Some(30),
                min_minutes: Some(900),
                page_size: Some(500),
                ..StatsMoneyballRequest::default()
            }) {
                Ok(page) => page,
                Err(error) => {
                    eprintln!("{label} {model}: {error}");
                    continue;
                }
            };
            let rows: Vec<_> = page
                .rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "uid": row.uid,
                        "name": row.name,
                        "club": row.club_name,
                        "group": row.position_group,
                        "age": row.age,
                        "ca": row.ca,
                        "minutes": row.minutes,
                        "production": row.production_tier,
                        "cost_pct": row.cost_pct_tier,
                        "surplus": row.surplus,
                        "score": row.score,
                        "advanced": row.advanced_available,
                        "flags": row.flags,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({
                    "save": label,
                    "model": model,
                    "total": page.total,
                    "rows": rows,
                })
            );
        }
    }
}
