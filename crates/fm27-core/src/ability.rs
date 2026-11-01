//! Approximate current-ability estimation from visible attributes.
//!
//! The weight table is the community FM attribute-weight table (GK, DRL, DC,
//! WBRL, DM, MRL, MC, AMRL, AMC, SC). The fit is calibrated against 48,026
//! players in an FM27 reference save:
//!
//! `CA = 18.13 * weighted_average + 18.74 * weighted_excess_above_16 - 97.60`
//!
//! The excess term captures the extra CA cost of high attributes. For a
//! multi-position player, each attribute uses the highest weight across the
//! natural positions; the weaker foot is included with its position weight.
//! This is an estimate, not a replacement for the stored CA.

use serde::{Deserialize, Serialize};

use crate::model::Attributes;

/// Weight of the weaker foot for each position column.
const WEAKER_FOOT_WEIGHTS: [f32; 10] = [3.0, 4.0, 4.5, 4.0, 5.0, 5.0, 6.0, 5.5, 7.0, 7.5];

const CA_INTERCEPT: f32 = -97.597_76;
const CA_AVERAGE_SLOPE: f32 = 18.130_968;
const CA_EXCESS_SLOPE: f32 = 18.737_27;
const HIGH_ATTRIBUTE_THRESHOLD: f32 = 16.0;
const POSITION_KEYS: [&str; 10] = [
    "GK", "DRL", "DC", "WBRL", "DM", "MRL", "MC", "AMRL", "AMC", "SC",
];

/// Attribute weights in the same column order as [`POSITION_KEYS`].
///
/// Zero-weight mental attributes (aggression, determination, flair and natural
/// fitness) are intentionally absent.
const ATTRIBUTE_WEIGHTS: &[(&str, [f32; 10])] = &[
    // Technical
    (
        "corners",
        [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "crossing",
        [0.0, 2.0, 1.0, 3.0, 1.0, 5.0, 1.0, 5.0, 1.0, 2.0],
    ),
    (
        "dribbling",
        [0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0, 5.0, 3.0, 5.0],
    ),
    (
        "finishing",
        [0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 8.0],
    ),
    (
        "first_touch",
        [1.0, 3.0, 2.0, 3.0, 4.0, 4.0, 6.0, 5.0, 5.0, 6.0],
    ),
    (
        "free_kick_taking",
        [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "heading",
        [1.0, 2.0, 5.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 6.0],
    ),
    (
        "long_shots",
        [0.0, 1.0, 1.0, 1.0, 3.0, 2.0, 3.0, 2.0, 3.0, 2.0],
    ),
    (
        "long_throws",
        [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "marking",
        [0.0, 3.0, 8.0, 2.0, 3.0, 1.0, 3.0, 1.0, 1.0, 1.0],
    ),
    (
        "passing",
        [3.0, 2.0, 2.0, 3.0, 4.0, 3.0, 6.0, 2.0, 4.0, 2.0],
    ),
    (
        "penalty_taking",
        [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "tackling",
        [0.0, 4.0, 5.0, 3.0, 7.0, 2.0, 3.0, 2.0, 2.0, 1.0],
    ),
    (
        "technique",
        [1.0, 2.0, 1.0, 3.0, 3.0, 4.0, 4.0, 4.0, 5.0, 4.0],
    ),
    // Mental
    (
        "anticipation",
        [3.0, 3.0, 5.0, 3.0, 5.0, 3.0, 3.0, 3.0, 3.0, 5.0],
    ),
    (
        "bravery",
        [6.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "composure",
        [2.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 3.0, 6.0],
    ),
    (
        "concentration",
        [6.0, 4.0, 4.0, 3.0, 3.0, 2.0, 2.0, 2.0, 2.0, 2.0],
    ),
    (
        "decisions",
        [10.0, 7.0, 10.0, 5.0, 8.0, 5.0, 7.0, 5.0, 6.0, 5.0],
    ),
    (
        "leadership",
        [2.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    ),
    (
        "off_the_ball",
        [0.0, 1.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 6.0],
    ),
    (
        "positioning",
        [5.0, 4.0, 8.0, 3.0, 5.0, 1.0, 3.0, 1.0, 2.0, 2.0],
    ),
    (
        "teamwork",
        [2.0, 2.0, 1.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 1.0],
    ),
    ("vision", [1.0, 2.0, 1.0, 2.0, 4.0, 3.0, 6.0, 3.0, 6.0, 2.0]),
    (
        "work_rate",
        [1.0, 2.0, 2.0, 2.0, 4.0, 3.0, 3.0, 3.0, 3.0, 2.0],
    ),
    // Physical
    (
        "acceleration",
        [6.0, 7.0, 6.0, 8.0, 6.0, 8.0, 6.0, 10.0, 9.0, 10.0],
    ),
    (
        "agility",
        [8.0, 6.0, 6.0, 5.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0],
    ),
    (
        "balance",
        [2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0],
    ),
    (
        "jumping_reach",
        [1.0, 2.0, 6.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0],
    ),
    ("pace", [3.0, 5.0, 5.0, 6.0, 4.0, 6.0, 5.0, 10.0, 7.0, 7.0]),
    (
        "stamina",
        [1.0, 6.0, 3.0, 7.0, 4.0, 5.0, 6.0, 7.0, 6.0, 6.0],
    ),
    (
        "strength",
        [4.0, 4.0, 6.0, 4.0, 5.0, 3.0, 4.0, 3.0, 3.0, 6.0],
    ),
    // Goalkeeping (GK column only)
    (
        "aerial_reach",
        [6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "command_of_area",
        [6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "communication",
        [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "handling",
        [8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "kicking",
        [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "one_on_ones",
        [4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "reflexes",
        [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "throwing",
        [3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaEstimate {
    pub position: String,
    pub estimated_ca: u16,
    pub weighted_average: f32,
    pub excess_average: f32,
    pub total_weight: f32,
}

/// Estimate CA for one position from display attributes (1..=20).
pub fn estimate_ca(attributes: &Attributes, weaker_foot: u8, position: &str) -> Option<CaEstimate> {
    let column = position_column(position)?;
    estimate_columns(
        attributes,
        weaker_foot,
        &[column],
        &normalize_position(position),
    )
}

/// Estimate CA from several natural positions. FM uses the highest weight for
/// each attribute across the player's natural positions; this mirrors that.
pub fn estimate_positions(
    attributes: &Attributes,
    weaker_foot: u8,
    positions: &[String],
) -> Option<CaEstimate> {
    let mut columns = Vec::new();
    let mut labels = Vec::new();
    for position in positions {
        let Some(column) = position_column(position) else {
            continue;
        };
        if !columns.contains(&column) {
            columns.push(column);
            labels.push(POSITION_KEYS[column]);
        }
    }
    if columns.is_empty() {
        return None;
    }
    estimate_columns(attributes, weaker_foot, &columns, &labels.join("/"))
}

fn estimate_columns(
    attributes: &Attributes,
    weaker_foot: u8,
    columns: &[usize],
    label: &str,
) -> Option<CaEstimate> {
    let mut weighted_sum = 0.0f32;
    let mut excess_sum = 0.0f32;
    let mut total_weight = 0.0f32;
    for (key, weights) in ATTRIBUTE_WEIGHTS {
        let weight = columns
            .iter()
            .map(|column| weights[*column])
            .fold(0.0f32, f32::max);
        if weight <= 0.0 {
            continue;
        }
        let value = attributes.get(key)? as f32;
        weighted_sum += value * weight;
        excess_sum += (value - HIGH_ATTRIBUTE_THRESHOLD).max(0.0) * weight;
        total_weight += weight;
    }
    let weaker_foot_weight = columns
        .iter()
        .map(|column| WEAKER_FOOT_WEIGHTS[*column])
        .fold(0.0f32, f32::max);
    if weaker_foot_weight > 0.0 {
        let weaker_foot = weaker_foot.clamp(1, 20) as f32;
        weighted_sum += weaker_foot * weaker_foot_weight;
        excess_sum += (weaker_foot - HIGH_ATTRIBUTE_THRESHOLD).max(0.0) * weaker_foot_weight;
        total_weight += weaker_foot_weight;
    }
    if total_weight <= 0.0 {
        return None;
    }
    let weighted_average = weighted_sum / total_weight;
    let excess_average = excess_sum / total_weight;
    let estimated =
        (CA_INTERCEPT + CA_AVERAGE_SLOPE * weighted_average + CA_EXCESS_SLOPE * excess_average)
            .round()
            .clamp(1.0, 200.0) as u16;
    Some(CaEstimate {
        position: label.to_owned(),
        estimated_ca: estimated,
        weighted_average,
        excess_average,
        total_weight,
    })
}

/// Estimate every natural position and keep the highest result. FM uses the
/// highest weight for each attribute across a player's natural positions, so the
/// best single-position estimate is a useful approximation.
pub fn estimate_best_position(
    attributes: &Attributes,
    weaker_foot: u8,
    positions: &[String],
) -> Option<CaEstimate> {
    positions
        .iter()
        .filter_map(|position| estimate_ca(attributes, weaker_foot, position))
        .max_by_key(|estimate| estimate.estimated_ca)
}

/// The FMScout weight table uses ten position groups. Map the save's 15
/// position keys onto them.
pub fn position_column(position: &str) -> Option<usize> {
    match normalize_position(position).as_str() {
        "GK" => Some(0),
        "DRL" => Some(1),
        "DC" => Some(2),
        "WBRL" => Some(3),
        "DM" => Some(4),
        "MRL" => Some(5),
        "MC" => Some(6),
        "AMRL" => Some(7),
        "AMC" => Some(8),
        "SC" => Some(9),
        _ => None,
    }
}

pub fn normalize_position(position: &str) -> String {
    match position {
        "GK" => "GK",
        "SW" | "DC" => "DC",
        "DL" | "DR" | "DRL" => "DRL",
        "WBL" | "WBR" | "WBRL" => "WBRL",
        "DM" => "DM",
        "ML" | "MR" | "MRL" => "MRL",
        "MC" => "MC",
        "AML" | "AMR" | "AMRL" => "AMRL",
        "AMC" => "AMC",
        "STC" | "SC" => "SC",
        _ => position,
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ATTRIBUTE_KEYS;

    fn all(value: u8) -> Attributes {
        Attributes {
            values: [value; 52],
        }
    }

    #[test]
    fn uniform_attributes_follow_the_calibrated_curve() {
        assert_eq!(estimate_ca(&all(16), 16, "SC").unwrap().estimated_ca, 192);
        assert_eq!(estimate_ca(&all(14), 14, "SC").unwrap().estimated_ca, 156);
        assert_eq!(estimate_ca(&all(12), 12, "SC").unwrap().estimated_ca, 120);
        assert_eq!(estimate_ca(&all(10), 10, "SC").unwrap().estimated_ca, 84);
        assert_eq!(estimate_ca(&all(8), 8, "SC").unwrap().estimated_ca, 47);
        assert_eq!(estimate_ca(&all(6), 6, "SC").unwrap().estimated_ca, 11);
    }

    #[test]
    fn high_attributes_add_extra_ca_cost() {
        let mut player = all(12);
        let finishing = ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == "finishing")
            .unwrap();
        let baseline = estimate_ca(&player, 12, "SC").unwrap().estimated_ca;
        player.values[finishing] = 20;
        assert!(estimate_ca(&player, 12, "SC").unwrap().estimated_ca > baseline);
    }

    #[test]
    fn zero_weight_attributes_do_not_change_the_estimate() {
        let mut low = all(12);
        let mut high = all(12);
        for key in ["aggression", "determination", "flair", "natural_fitness"] {
            let index = ATTRIBUTE_KEYS.iter().position(|name| *name == key).unwrap();
            low.values[index] = 1;
            high.values[index] = 20;
        }
        assert_eq!(
            estimate_ca(&low, 10, "SC").unwrap().estimated_ca,
            estimate_ca(&high, 10, "SC").unwrap().estimated_ca
        );
    }

    #[test]
    fn multiple_positions_are_combined() {
        let estimate = estimate_positions(&all(12), 12, &["STC".to_owned(), "AML".to_owned()])
            .expect("combined estimate");
        assert_eq!(estimate.position, "SC/AMRL");
        assert!(estimate.total_weight > 0.0);
    }

    #[test]
    fn positions_use_different_weights() {
        let mut striker = all(10);
        let finishing = ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == "finishing")
            .unwrap();
        let tackling = ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == "tackling")
            .unwrap();
        striker.values[finishing] = 20;
        striker.values[tackling] = 1;
        let as_striker = estimate_ca(&striker, 10, "SC").unwrap().estimated_ca;
        let as_centre_back = estimate_ca(&striker, 10, "DC").unwrap().estimated_ca;
        assert!(as_striker > as_centre_back);
    }
}
