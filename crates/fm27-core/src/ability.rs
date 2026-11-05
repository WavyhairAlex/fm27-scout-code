//! Exact FM current-ability (RCA) estimator.
//!
//! This follows the community reverse-engineering of the game's recommended-CA
//! calculation published on PlayGM:
//! <https://www.playgm.cc/thread-986285-1-1.html>.
//!
//! The estimator works on the internal 1–100 attribute scale (displayed 1–20)
//! and reproduces the game's multi-pass calculation:
//!
//! 1. build a weighted average from the position column and the counted feet;
//! 2. start from `(weighted_average / 5 - 6) * 20`;
//! 3. repeatedly correct the candidate for attributes that deviate from the
//!    position baseline by more than a weight-dependent threshold;
//! 4. subtract the position-combination cost from the CA budget;
//! 5. scale the result by the position familiarity percentage;
//! 6. take the highest RCA across all valid positions.
//!
//! Unlike a regression fit, this implementation is intended to match the game's
//! stored recommended CA (RCA) exactly. The stored CA can still differ because
//! it is the authoritative value saved by FM.
//!
//! The browser mock mirrors this algorithm in
//! `apps/desktop/src/lib/ca.ts`; keep the two in sync.

use serde::{Deserialize, Serialize};

use crate::layout::POSITION_NAMES;
use crate::model::{Attributes, Positions};

/// Weight of the counted foot for each position column.
const FOOT_WEIGHTS: [f64; 10] = [3.0, 4.0, 4.5, 4.0, 5.0, 5.0, 6.0, 5.5, 7.0, 7.5];

const SLOT_COUNT: usize = 15;

/// Column layout: GK, DRL, DC, WBRL, DM, MRL, MC, AMRL, AMC, SC.
const ATTRIBUTE_WEIGHTS: &[(&str, [f64; 10])] = &[
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
        [8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "throwing",
        [3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
];

/// How one of the 15 position ratings participates in the position-cost formula.
#[derive(Clone, Copy)]
struct SlotInfo {
    /// Weight-table column.
    column: usize,
    /// One of six pitch layers: GK, defence, DM/WB, midfield, AM, attack.
    layer: usize,
    /// Left, right or centre route. GK has no route.
    route: Option<usize>,
    /// One of four macro regions: GK, defence, midfield, attack.
    region: usize,
}

const SLOT_INFO: [SlotInfo; SLOT_COUNT] = [
    // GK
    SlotInfo {
        column: 0,
        layer: 0,
        route: None,
        region: 0,
    },
    // SW (treated as a central defender)
    SlotInfo {
        column: 2,
        layer: 1,
        route: Some(2),
        region: 1,
    },
    // DL
    SlotInfo {
        column: 1,
        layer: 1,
        route: Some(0),
        region: 1,
    },
    // DC
    SlotInfo {
        column: 2,
        layer: 1,
        route: Some(2),
        region: 1,
    },
    // DR
    SlotInfo {
        column: 1,
        layer: 1,
        route: Some(1),
        region: 1,
    },
    // DM
    SlotInfo {
        column: 4,
        layer: 2,
        route: Some(2),
        region: 2,
    },
    // ML
    SlotInfo {
        column: 5,
        layer: 3,
        route: Some(0),
        region: 2,
    },
    // MC
    SlotInfo {
        column: 6,
        layer: 3,
        route: Some(2),
        region: 2,
    },
    // MR
    SlotInfo {
        column: 5,
        layer: 3,
        route: Some(1),
        region: 2,
    },
    // AML
    SlotInfo {
        column: 7,
        layer: 4,
        route: Some(0),
        region: 2,
    },
    // AMC
    SlotInfo {
        column: 8,
        layer: 4,
        route: Some(2),
        region: 2,
    },
    // AMR
    SlotInfo {
        column: 7,
        layer: 4,
        route: Some(1),
        region: 2,
    },
    // STC
    SlotInfo {
        column: 9,
        layer: 5,
        route: Some(2),
        region: 3,
    },
    // WBL
    SlotInfo {
        column: 3,
        layer: 2,
        route: Some(0),
        region: 2,
    },
    // WBR
    SlotInfo {
        column: 3,
        layer: 2,
        route: Some(1),
        region: 2,
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaEstimate {
    /// Position whose RCA was selected (e.g. `STC`).
    pub position: String,
    /// Final recommended CA.
    pub estimated_ca: u16,
    /// Weighted attribute average on the internal 1–100 scale.
    pub weighted_average: f64,
    /// Signed excess average from the final refinement pass.
    pub excess_average: f64,
    /// Sum of the weights that entered the calculation.
    pub total_weight: f64,
    /// Position-combination cost `P`.
    pub position_cost: u16,
    /// Initial candidate before distribution refinement.
    pub initial_ca: u16,
    /// Number of refinement rounds used (0–100).
    pub iterations: u16,
    /// Counted left-foot value on the internal 1–100 scale.
    pub left_foot: u8,
    /// Counted right-foot value on the internal 1–100 scale.
    pub right_foot: u8,
}

/// Estimate the best RCA across every position with familiarity at least 2.
///
/// `attributes` and the foot values use the internal 1–100 scale. `positions`
/// is the full 15-position familiarity table (0–20).
pub fn estimate_positions(
    attributes: &Attributes,
    positions: &Positions,
    left_foot: u8,
    right_foot: u8,
) -> Option<CaEstimate> {
    let position_cost = position_cost(positions);
    let mut best: Option<CaEstimate> = None;
    for (slot, info) in SLOT_INFO.iter().enumerate() {
        let proficiency = positions.values[slot].min(20);
        if proficiency < 2 {
            continue;
        }
        let Some(column) = estimate_column(
            attributes,
            info.column,
            left_foot,
            right_foot,
            position_cost,
        ) else {
            continue;
        };
        let Some(retention) = retention(proficiency) else {
            continue;
        };
        let estimated = ((column.candidate as f64 * retention) + 0.5)
            .floor()
            .clamp(1.0, 200.0) as u16;
        let candidate = CaEstimate {
            position: POSITION_NAMES[slot].to_owned(),
            estimated_ca: estimated,
            weighted_average: column.weighted_average,
            excess_average: column.excess_average,
            total_weight: column.total_weight,
            position_cost,
            initial_ca: column.initial_ca,
            iterations: column.iterations,
            left_foot: left_foot.min(100),
            right_foot: right_foot.min(100),
        };
        if best
            .as_ref()
            .is_none_or(|current| estimated > current.estimated_ca)
        {
            best = Some(candidate);
        }
    }
    best
}

/// Estimate RCA for a single position, using the full familiarity table for the
/// position-combination cost.
///
/// `position` accepts save position names such as `STC`, `AML`, `WBR` or the
/// combined community labels `SC`, `AMRL`, `MRL`, `DRL`, `WBRL`.
pub fn estimate_ca(
    attributes: &Attributes,
    positions: &Positions,
    left_foot: u8,
    right_foot: u8,
    position: &str,
) -> Option<CaEstimate> {
    let slot = position_slot(position)?;
    let proficiency = positions.values[slot].min(20);
    if proficiency < 2 {
        return None;
    }
    let position_cost = position_cost(positions);
    let info = SLOT_INFO[slot];
    let column = estimate_column(
        attributes,
        info.column,
        left_foot,
        right_foot,
        position_cost,
    )?;
    let retention = retention(proficiency)?;
    let estimated = ((column.candidate as f64 * retention) + 0.5)
        .floor()
        .clamp(1.0, 200.0) as u16;
    Some(CaEstimate {
        position: POSITION_NAMES[slot].to_owned(),
        estimated_ca: estimated,
        weighted_average: column.weighted_average,
        excess_average: column.excess_average,
        total_weight: column.total_weight,
        position_cost,
        initial_ca: column.initial_ca,
        iterations: column.iterations,
        left_foot: left_foot.min(100),
        right_foot: right_foot.min(100),
    })
}

struct ColumnResult {
    candidate: u16,
    initial_ca: u16,
    weighted_average: f64,
    excess_average: f64,
    total_weight: f64,
    iterations: u16,
}

fn estimate_column(
    attributes: &Attributes,
    column: usize,
    left_foot: u8,
    right_foot: u8,
    position_cost: u16,
) -> Option<ColumnResult> {
    let left_foot = left_foot.min(100);
    let right_foot = right_foot.min(100);
    let (feet, foot_count) = counted_feet(left_foot, right_foot);
    let foot_weight = FOOT_WEIGHTS[column];

    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut nonzero_count = 0usize;
    for (key, weights) in ATTRIBUTE_WEIGHTS {
        let weight = weights[column];
        if weight <= 0.0 {
            continue;
        }
        let value = attributes.get(key)? as f64;
        weighted_sum += value.clamp(1.0, 100.0) * weight;
        total_weight += weight;
        nonzero_count += 1;
    }
    for foot in feet.iter().take(foot_count) {
        weighted_sum += *foot as f64 * foot_weight;
        total_weight += foot_weight;
        nonzero_count += 1;
    }
    if total_weight <= 0.0 {
        return None;
    }

    let weighted_average = weighted_sum / total_weight;
    let initial_ca = (((weighted_average / 5.0) - 6.0) * 20.0).floor().max(1.0) as i32;
    let threshold = 3.75 * total_weight / nonzero_count as f64;

    let mut candidate = initial_ca;
    let mut previous_t: Option<i32> = None;
    let mut iterations = 0u16;
    let mut last_excess = 0.0;

    for round in 1..=100 {
        // The position-combination cost is deducted from the CA budget before
        // rebuilding the attribute baseline.
        let deduction = ((position_cost / 3) as i32).min(candidate.max(0) / 4);
        let base = (((candidate - deduction) as f64 / 20.0) + 6.0) * 5.0;

        let mut excess = 0.0;
        for (key, weights) in ATTRIBUTE_WEIGHTS {
            let weight = weights[column];
            if weight <= 0.0 {
                continue;
            }
            let value = attributes.get(key)? as f64;
            let deviation = (value.clamp(1.0, 100.0) - base) * weight / 5.0;
            if deviation > threshold {
                excess += deviation - threshold;
            } else if deviation < -threshold {
                excess += deviation + threshold;
            }
        }
        for foot in feet.iter().take(foot_count) {
            let deviation = (*foot as f64 - base) * foot_weight / 5.0;
            if deviation > threshold {
                excess += deviation - threshold;
            } else if deviation < -threshold {
                excess += deviation + threshold;
            }
        }
        last_excess = excess;

        // The signed excess moves the reconstructed average in the same
        // direction as the attribute distribution.
        let corrected_average = (weighted_sum + excess * 3.75) / total_weight;
        let error = base - corrected_average;
        let mut t = if error >= 0.0 {
            let t = (error + 0.5).floor() as i32;
            if t == 0 { 1 } else { t }
        } else {
            // C-style integer truncation toward zero.
            (error - 0.5).trunc() as i32
        };

        // The allowed correction range shrinks every ten rounds.
        let limit = (5 - round / 10).max(1);
        t = t.clamp(-limit, limit);

        if t != 0 {
            if let Some(previous) = previous_t {
                if (t - previous).abs() >= 6 && t.abs() >= 2 {
                    t /= 2;
                }
                if t.abs() > previous.abs() {
                    t = t.signum() * previous.abs();
                }
                if t == -previous && t.abs() > 1 {
                    t -= t.signum();
                }
                if previous > 0 && t > previous {
                    // The reconstructed code keeps the previous record and
                    // moves the candidate down one point immediately.
                    candidate -= 1;
                    iterations += 1;
                    if candidate <= 0 {
                        candidate = initial_ca;
                    }
                    continue;
                }
                if previous < 0 {
                    t = t.max(previous);
                }
            }
        }

        previous_t = Some(t);
        iterations += 1;
        if t > 0 {
            candidate -= 1;
        } else if t < 0 {
            candidate += 1;
        } else {
            break;
        }
        if candidate <= 0 {
            candidate = initial_ca;
        }
    }

    Some(ColumnResult {
        candidate: candidate.max(1) as u16,
        initial_ca: initial_ca.max(1) as u16,
        weighted_average,
        excess_average: last_excess / total_weight,
        total_weight,
        iterations,
    })
}

/// Which feet enter the calculation.
///
/// If one foot is at 100, only the other (weaker) foot is counted. If both are
/// at 100, the left foot alone is used. Otherwise both feet are counted.
fn counted_feet(left_foot: u8, right_foot: u8) -> ([u8; 2], usize) {
    if right_foot >= 100 {
        ([left_foot, 0], 1)
    } else if left_foot >= 100 {
        ([right_foot, 0], 1)
    } else {
        ([left_foot, right_foot], 2)
    }
}

/// Position-combination cost `P` (0–100).
fn position_cost(positions: &Positions) -> u16 {
    let mut familiarity_sum = 0i32;
    let mut layers = [false; 6];
    let mut routes = [false; 3];
    let mut regions = [false; 4];

    for (slot, info) in SLOT_INFO.iter().enumerate() {
        let familiarity = positions.values[slot].min(20) as i32;
        familiarity_sum += (familiarity - 10).max(0);
        if familiarity < 15 {
            continue;
        }
        layers[info.layer] = true;
        if let Some(route) = info.route {
            routes[route] = true;
        }
        regions[info.region] = true;
    }

    let layer_count = layers.iter().filter(|used| **used).count() as i32;
    let route_count = routes.iter().filter(|used| **used).count() as i32;

    let middle_layers = layers[2] as i32 + layers[3] as i32 + layers[4] as i32;
    let midfield_penalty = if middle_layers >= 2 { 3 } else { 0 };
    let side_penalty = if routes[0] || routes[1] { 3 } else { 0 };

    let mut region_count = regions.iter().filter(|used| **used).count() as i32;
    if regions[0] && region_count > 1 {
        region_count += 2;
    }
    if regions[1] && regions[3] {
        region_count = region_count.max(3);
    }
    if layers[2] && layers[4] {
        region_count = region_count.max(2);
    }
    let region_cost = if region_count < 2 {
        0
    } else {
        region_count * region_count * (region_count + 1)
    };

    let value = familiarity_sum + 3 * (layer_count + route_count) - midfield_penalty - side_penalty
        + region_cost;
    let value = value.clamp(40, 140);
    (value - 40) as u16
}

/// Familiarity retention percentages used for the final RCA.
fn retention(familiarity: u8) -> Option<f64> {
    Some(match familiarity {
        18..=20 => 1.0,
        17 => 0.99,
        16 => 0.98,
        15 => 0.96,
        14 => 0.94,
        13 => 0.92,
        12 => 0.88,
        11 => 0.84,
        10 => 0.80,
        9 => 0.72,
        8 => 0.64,
        7 => 0.56,
        6 => 0.40,
        5 => 0.24,
        4 => 0.08,
        2..=3 => 0.01,
        _ => return None,
    })
}

/// Map a save position name or community label onto one of the 15 slots.
fn position_slot(position: &str) -> Option<usize> {
    match position.trim().to_ascii_uppercase().as_str() {
        "GK" => Some(0),
        "SW" => Some(1),
        "DL" | "DRL" => Some(2),
        "DC" => Some(3),
        "DR" => Some(4),
        "DM" => Some(5),
        "ML" | "MRL" => Some(6),
        "MC" => Some(7),
        "MR" => Some(8),
        "AML" | "AMRL" => Some(9),
        "AMC" => Some(10),
        "AMR" => Some(11),
        "STC" | "ST" | "SC" => Some(12),
        "WBL" | "WBRL" => Some(13),
        "WBR" => Some(14),
        _ => None,
    }
}

/// Map a position name on the community 10-column weight table.
pub fn position_column(position: &str) -> Option<usize> {
    match position.trim().to_ascii_uppercase().as_str() {
        "GK" => Some(0),
        "DL" | "DR" | "DRL" => Some(1),
        "SW" | "DC" => Some(2),
        "WBL" | "WBR" | "WBRL" => Some(3),
        "DM" => Some(4),
        "ML" | "MR" | "MRL" => Some(5),
        "MC" => Some(6),
        "AML" | "AMR" | "AMRL" => Some(7),
        "AMC" => Some(8),
        "STC" | "ST" | "SC" => Some(9),
        _ => None,
    }
}

/// Normalize a save position name to the community weight-table label.
pub fn normalize_position(position: &str) -> String {
    match position.trim().to_ascii_uppercase().as_str() {
        "GK" => "GK",
        "SW" | "DC" => "DC",
        "DL" | "DR" | "DRL" => "DRL",
        "WBL" | "WBR" | "WBRL" => "WBRL",
        "DM" => "DM",
        "ML" | "MR" | "MRL" => "MRL",
        "MC" => "MC",
        "AML" | "AMR" | "AMRL" => "AMRL",
        "AMC" => "AMC",
        "STC" | "ST" | "SC" => "SC",
        _ => return position.to_owned(),
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ATTRIBUTE_KEYS, POSITION_KEYS};

    fn all(value: u8) -> Attributes {
        Attributes {
            values: [value; 52],
        }
    }

    fn positions(entries: &[(&str, u8)]) -> Positions {
        let mut positions = Positions::default();
        for (name, value) in entries {
            let index = POSITION_NAMES
                .iter()
                .position(|position| *position == *name)
                .expect("position name");
            positions.values[index] = *value;
        }
        positions
    }

    #[test]
    fn article_single_striker_example_is_135() {
        let estimate =
            estimate_positions(&all(65), &positions(&[("STC", 20)]), 100, 50).expect("estimate");
        assert_eq!(estimate.position, "STC");
        assert_eq!(estimate.estimated_ca, 135);
        assert_eq!(estimate.initial_ca, 136);
        assert_eq!(estimate.position_cost, 0);
    }

    #[test]
    fn article_two_feet_example_is_151() {
        let estimate =
            estimate_positions(&all(65), &positions(&[("STC", 20)]), 100, 100).expect("estimate");
        assert_eq!(estimate.estimated_ca, 151);
    }

    #[test]
    fn article_striker_centre_back_example_is_144() {
        let both = estimate_positions(&all(65), &positions(&[("STC", 20), ("DC", 20)]), 100, 50)
            .expect("estimate");
        assert_eq!(both.position_cost, 25);
        assert_eq!(both.estimated_ca, 144);

        let striker = estimate_ca(
            &all(65),
            &positions(&[("STC", 20), ("DC", 20)]),
            100,
            50,
            "STC",
        )
        .expect("striker");
        assert_eq!(striker.estimated_ca, 142);

        let centre_back = estimate_ca(
            &all(65),
            &positions(&[("STC", 20), ("DC", 20)]),
            100,
            50,
            "DC",
        )
        .expect("centre back");
        assert_eq!(centre_back.estimated_ca, 144);
    }

    #[test]
    fn article_striker_attacking_midfielder_example_is_135() {
        let both = estimate_positions(&all(65), &positions(&[("STC", 20), ("AMC", 20)]), 100, 50)
            .expect("estimate");
        assert_eq!(both.position_cost, 1);
        assert_eq!(both.estimated_ca, 135);

        let attacking_midfielder = estimate_ca(
            &all(65),
            &positions(&[("STC", 20), ("AMC", 20)]),
            100,
            50,
            "AMC",
        )
        .expect("attacking midfielder");
        assert_eq!(attacking_midfielder.estimated_ca, 134);
    }

    #[test]
    fn article_familiarity_scaling_example_is_130() {
        let estimate =
            estimate_positions(&all(65), &positions(&[("STC", 15)]), 100, 50).expect("estimate");
        assert_eq!(estimate.estimated_ca, 130);
    }

    #[test]
    fn article_three_position_cost_example_is_8() {
        let cost = position_cost(&positions(&[("DL", 20), ("WBL", 20), ("ML", 20)]));
        assert_eq!(cost, 8);
    }

    #[test]
    fn article_position_cost_examples() {
        assert_eq!(position_cost(&positions(&[("STC", 20)])), 0);
        assert_eq!(position_cost(&positions(&[("STC", 20), ("DC", 20)])), 25);
        assert_eq!(position_cost(&positions(&[("STC", 20), ("AMC", 20)])), 1);
    }

    #[test]
    fn zero_weight_attributes_do_not_change_the_estimate() {
        let mut low = all(65);
        let mut high = all(65);
        for key in ["aggression", "determination", "flair", "natural_fitness"] {
            let index = ATTRIBUTE_KEYS.iter().position(|name| *name == key).unwrap();
            low.values[index] = 1;
            high.values[index] = 100;
        }
        assert_eq!(
            estimate_positions(&low, &positions(&[("STC", 20)]), 100, 50)
                .unwrap()
                .estimated_ca,
            estimate_positions(&high, &positions(&[("STC", 20)]), 100, 50)
                .unwrap()
                .estimated_ca
        );
    }

    #[test]
    fn positions_use_different_weights() {
        let mut striker = all(65);
        let finishing = ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == "finishing")
            .unwrap();
        let tackling = ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == "tackling")
            .unwrap();
        striker.values[finishing] = 100;
        striker.values[tackling] = 1;
        let as_striker = estimate_ca(
            &striker,
            &positions(&[("STC", 20), ("DC", 20)]),
            100,
            50,
            "STC",
        )
        .unwrap()
        .estimated_ca;
        let as_centre_back = estimate_ca(
            &striker,
            &positions(&[("STC", 20), ("DC", 20)]),
            100,
            50,
            "DC",
        )
        .unwrap()
        .estimated_ca;
        assert!(as_striker > as_centre_back);
    }

    #[test]
    fn striker_weight_table_matches_the_article() {
        let striker_total: f64 = ATTRIBUTE_WEIGHTS
            .iter()
            .map(|(_, weights)| weights[9])
            .sum();
        let striker_nonzero = ATTRIBUTE_WEIGHTS
            .iter()
            .filter(|(_, weights)| weights[9] > 0.0)
            .count();
        assert_eq!(striker_total, 116.0);
        assert_eq!(striker_nonzero, 32);
    }

    #[test]
    fn position_mapping_covers_every_save_slot() {
        for (slot, key) in POSITION_KEYS.iter().enumerate() {
            assert!(position_slot(&key.to_ascii_uppercase()).is_some());
            let info = SLOT_INFO[slot];
            assert_eq!(position_column(POSITION_NAMES[slot]).unwrap(), info.column);
        }
    }
}
