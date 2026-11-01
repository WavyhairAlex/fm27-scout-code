//! Real-save check for the approximate CA estimator.
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -p fm27-core \
//!   --test ability_real_save -- --ignored --nocapture
//! ```

use fm27_core::ability::{estimate_best_position, estimate_ca};
use fm27_core::parse_save;

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn estimates_be_compared_with_stored_current_ability() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");

    let mut single = Vec::new();
    let mut multi = Vec::new();
    for player in &parsed.players {
        if player.ca == 0 || player.natural_positions.is_empty() {
            continue;
        }
        let weaker_foot = player.left_foot.min(player.right_foot);
        let estimate = if player.natural_positions.len() == 1 {
            estimate_ca(
                &player.attributes,
                weaker_foot,
                &player.natural_positions[0],
            )
        } else {
            estimate_best_position(&player.attributes, weaker_foot, &player.natural_positions)
        };
        if let Some(estimate) = estimate {
            let error = estimate.estimated_ca as i32 - player.ca as i32;
            if player.natural_positions.len() == 1 {
                single.push(error);
            } else {
                multi.push(error);
            }
        }
    }

    report("single position", &single);
    report("multiple positions", &multi);
}

fn report(label: &str, errors: &[i32]) {
    if errors.is_empty() {
        println!("{label}: no samples");
        return;
    }
    let count = errors.len() as f64;
    let mean = errors.iter().map(|value| *value as f64).sum::<f64>() / count;
    let mae = errors.iter().map(|value| value.abs() as f64).sum::<f64>() / count;
    let within_5 = errors.iter().filter(|value| value.abs() <= 5).count();
    let within_10 = errors.iter().filter(|value| value.abs() <= 10).count();
    println!(
        "{label}: n={} bias={mean:+.1} MAE={mae:.1} within5={:.1}% within10={:.1}%",
        errors.len(),
        100.0 * within_5 as f64 / count,
        100.0 * within_10 as f64 / count
    );
}
