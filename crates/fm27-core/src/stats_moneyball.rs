//! Stats-based Moneyball scoring.
//!
//! The model has two tiers:
//!
//! - `core` only uses fields present for every player (xG, xA, shots, passing,
//!   tackles, rating). It is the safe all-player ranking.
//! - `advanced` adds key passes, progressive passes, interceptions, headers,
//!   clearances, blocks, dribbles, crosses, save percentage and xGP. It is only
//!   valid for rows whose `advanced_available` flag is true; the rest are left
//!   unscored instead of treating the save's zero block as real data.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const STATS_CORE_MODEL: &str = "stats-core-v1";
pub const STATS_ADVANCED_MODEL: &str = "stats-advanced-v1";
const COST_HORIZON_YEARS: f64 = 3.0;
const SHRINK_MINUTES: f64 = 900.0;

/// Normalize a model name to `core` or `advanced`.
pub fn normalized_model(model: Option<&str>) -> Result<&'static str> {
    match model.unwrap_or("core").trim().to_ascii_lowercase().as_str() {
        "core" => Ok("core"),
        "advanced" => Ok("advanced"),
        other => Err(Error::parse(format!("unknown stats model {other:?}"))),
    }
}

pub fn model_name(model: &str) -> &'static str {
    if model == "advanced" {
        STATS_ADVANCED_MODEL
    } else {
        STATS_CORE_MODEL
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StatsMoneyballRequest {
    pub save_id: i64,
    /// `core` (default) or `advanced`.
    pub model: Option<String>,
    /// Include goalkeepers in a mixed-position list. The UI usually leaves this
    /// false and opens GK through the position filter instead.
    pub include_goalkeepers: bool,
    pub position_group: Option<String>,
    pub league: Option<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub ca_min: Option<u16>,
    pub ca_max: Option<u16>,
    pub min_minutes: Option<u32>,
    pub max_fee: Option<u64>,
    pub max_wage: Option<u64>,
    pub contract_end_before: Option<String>,
    /// Any of `sale`, `loan` and `free`.
    pub listings: Vec<String>,
    pub min_confidence: Option<f64>,
    pub exclude_injured: bool,
    pub exclude_injury_history: bool,
    pub include_unknown_fee: bool,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsDriver {
    pub key: String,
    pub label: String,
    pub percentile: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsMoneyballRow {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub league_name: Option<String>,
    pub nation_name: Option<String>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub position_group: String,
    pub minutes: u32,
    pub starts: u32,
    pub substitute_appearances: u32,
    pub average_rating: Option<f64>,
    pub goals: u32,
    pub assists: u32,
    pub expected_goals: f64,
    pub expected_assists: f64,
    pub shots: u32,
    pub shots_on_target: u32,
    pub passes_attempted: u32,
    pub passes_completed: u32,
    pub pass_completion_percent: Option<f64>,
    pub tackles_attempted: u32,
    pub tackles_completed: u32,
    pub interceptions: Option<u32>,
    pub key_passes: Option<u32>,
    pub progressive_passes: Option<u32>,
    pub headers_won: Option<u32>,
    pub aerial_challenges_attempted: Option<u32>,
    pub clearances: Option<u32>,
    pub blocks: Option<u32>,
    pub crosses_completed: Option<u32>,
    pub saves_held: Option<u32>,
    pub shots_on_target_faced: Option<u32>,
    pub save_percent: Option<f64>,
    pub expected_goals_prevented: Option<f64>,
    pub advanced_available: bool,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: String,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub club_reputation: Option<u16>,
    pub fee: Option<u64>,
    pub cost: u64,
    pub fee_known: bool,
    pub wage_known: bool,
    pub production_global: Option<f64>,
    pub production_tier: Option<f64>,
    pub cost_pct_global: Option<f64>,
    pub cost_pct_tier: Option<f64>,
    pub surplus: Option<f64>,
    pub score: Option<f64>,
    pub confidence: f64,
    pub flags: Vec<String>,
    pub drivers: Vec<StatsDriver>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsMoneyballMeta {
    pub model: String,
    pub scored: usize,
    pub filtered: usize,
    pub min_minutes: u32,
    pub cost_horizon_years: f64,
    pub leagues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsMoneyballPage {
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub rows: Vec<StatsMoneyballRow>,
    pub meta: StatsMoneyballMeta,
}

/// One player loaded from SQLite with his `overall` season-statistics row.
#[derive(Debug, Clone)]
pub struct StatsMoneyballCandidate {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub league_name: Option<String>,
    pub nation_name: Option<String>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub positions_text: Option<String>,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: String,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub club_reputation: Option<u16>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
    pub minutes: u32,
    pub starts: u32,
    pub substitute_appearances: u32,
    pub average_rating: Option<f64>,
    pub goals: u32,
    pub assists: u32,
    pub expected_goals: f64,
    pub expected_assists: f64,
    pub shots: u32,
    pub shots_on_target: u32,
    pub passes_attempted: u32,
    pub passes_completed: u32,
    pub tackles_attempted: u32,
    pub tackles_completed: u32,
    pub goals_allowed: Option<u32>,
    pub clean_sheets: Option<u32>,
    pub interceptions: Option<u32>,
    pub key_passes: Option<u32>,
    pub progressive_passes: Option<u32>,
    pub dribbles: Option<u32>,
    pub headers_won: Option<u32>,
    pub aerial_challenges_attempted: Option<u32>,
    pub clearances: Option<u32>,
    pub blocks: Option<u32>,
    pub crosses_completed: Option<u32>,
    pub saves_held: Option<u32>,
    pub saves_parried: Option<u32>,
    pub saves_tipped: Option<u32>,
    pub shots_on_target_faced: Option<u32>,
    pub expected_goals_prevented: Option<f64>,
    pub advanced_available: bool,
    /// True when this is the row for the player's current club. Used only to
    /// pick one row per player before scoring.
    pub current_club: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Metric {
    Xg90,
    Xa90,
    ShotsOnTarget90,
    Conversion,
    KeyPasses90,
    Dribbles90,
    ProgressivePasses90,
    Interceptions90,
    HeadersPercent,
    Clearances90,
    Blocks90,
    Crosses90,
    PassCompletion,
    PassesCompleted90,
    Tackles90,
    TackleCompletion,
    Rating,
    GoalsAllowed90,
    CleanSheets90,
    SavePercent,
    ExpectedGoalsPrevented90,
}

impl Metric {
    fn key(self) -> &'static str {
        match self {
            Self::Xg90 => "xg90",
            Self::Xa90 => "xa90",
            Self::ShotsOnTarget90 => "sot90",
            Self::Conversion => "conversion",
            Self::KeyPasses90 => "kp90",
            Self::Dribbles90 => "drib90",
            Self::ProgressivePasses90 => "prog90",
            Self::Interceptions90 => "int90",
            Self::HeadersPercent => "headers_pct",
            Self::Clearances90 => "clearances90",
            Self::Blocks90 => "blocks90",
            Self::Crosses90 => "crosses90",
            Self::PassCompletion => "pass_pct",
            Self::PassesCompleted90 => "passes90",
            Self::Tackles90 => "tackles90",
            Self::TackleCompletion => "tackle_pct",
            Self::Rating => "rating",
            Self::GoalsAllowed90 => "goals_allowed90",
            Self::CleanSheets90 => "clean_sheets90",
            Self::SavePercent => "save_pct",
            Self::ExpectedGoalsPrevented90 => "xgp90",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Xg90 => "xG/90",
            Self::Xa90 => "xA/90",
            Self::ShotsOnTarget90 => "Shots on target/90",
            Self::Conversion => "Conversion %",
            Self::KeyPasses90 => "Key passes/90",
            Self::Dribbles90 => "Dribbles/90",
            Self::ProgressivePasses90 => "Progressive passes/90",
            Self::Interceptions90 => "Interceptions/90",
            Self::HeadersPercent => "Headers won %",
            Self::Clearances90 => "Clearances/90",
            Self::Blocks90 => "Blocks/90",
            Self::Crosses90 => "Crosses completed/90",
            Self::PassCompletion => "Pass completion %",
            Self::PassesCompleted90 => "Passes completed/90",
            Self::Tackles90 => "Tackles won/90",
            Self::TackleCompletion => "Tackle completion %",
            Self::Rating => "Average rating",
            Self::GoalsAllowed90 => "Goals allowed/90",
            Self::CleanSheets90 => "Clean sheets/90",
            Self::SavePercent => "Save %",
            Self::ExpectedGoalsPrevented90 => "xGP/90",
        }
    }

    fn advanced(self) -> bool {
        matches!(
            self,
            Self::KeyPasses90
                | Self::Dribbles90
                | Self::ProgressivePasses90
                | Self::Interceptions90
                | Self::HeadersPercent
                | Self::Clearances90
                | Self::Blocks90
                | Self::Crosses90
                | Self::SavePercent
                | Self::ExpectedGoalsPrevented90
        )
    }

    fn inverse(self) -> bool {
        matches!(self, Self::GoalsAllowed90)
    }

    fn value(self, candidate: &StatsMoneyballCandidate) -> Option<f64> {
        if self.advanced() && !candidate.advanced_available {
            return None;
        }
        let per90 = |count: u32| {
            (candidate.minutes > 0).then(|| f64::from(count) * 90.0 / f64::from(candidate.minutes))
        };
        match self {
            Self::Xg90 => per90_f64(candidate.expected_goals, candidate.minutes),
            Self::Xa90 => per90_f64(candidate.expected_assists, candidate.minutes),
            Self::ShotsOnTarget90 => per90(candidate.shots_on_target),
            Self::Conversion => percent(candidate.goals, candidate.shots),
            Self::KeyPasses90 => candidate.key_passes.and_then(per90),
            Self::Dribbles90 => candidate.dribbles.and_then(per90),
            Self::ProgressivePasses90 => candidate.progressive_passes.and_then(per90),
            Self::Interceptions90 => candidate.interceptions.and_then(per90),
            Self::HeadersPercent => percent(
                candidate.headers_won.unwrap_or(0),
                candidate.aerial_challenges_attempted.unwrap_or(0),
            ),
            Self::Clearances90 => candidate.clearances.and_then(per90),
            Self::Blocks90 => candidate.blocks.and_then(per90),
            Self::Crosses90 => candidate.crosses_completed.and_then(per90),
            Self::PassCompletion => percent(candidate.passes_completed, candidate.passes_attempted),
            Self::PassesCompleted90 => per90(candidate.passes_completed),
            Self::Tackles90 => per90(candidate.tackles_completed),
            Self::TackleCompletion => {
                percent(candidate.tackles_completed, candidate.tackles_attempted)
            }
            Self::Rating => candidate.average_rating,
            Self::GoalsAllowed90 => per90(candidate.goals_allowed()),
            Self::CleanSheets90 => per90(candidate.clean_sheets()),
            Self::SavePercent => candidate.save_percent(),
            Self::ExpectedGoalsPrevented90 => per90_f64(
                candidate.expected_goals_prevented.unwrap_or(0.0),
                candidate.minutes,
            ),
        }
    }
}

fn per90_f64(count: f64, minutes: u32) -> Option<f64> {
    (minutes > 0).then(|| count * 90.0 / f64::from(minutes))
}

fn percent(part: u32, whole: u32) -> Option<f64> {
    (whole > 0).then(|| 100.0 * f64::from(part) / f64::from(whole))
}

impl StatsMoneyballCandidate {
    fn goals_allowed(&self) -> u32 {
        self.goals_allowed.unwrap_or(0)
    }

    fn clean_sheets(&self) -> u32 {
        self.clean_sheets.unwrap_or(0)
    }

    fn save_percent(&self) -> Option<f64> {
        let faced = self.shots_on_target_faced?;
        if faced == 0 {
            return None;
        }
        let saves = self.saves_held? + self.saves_parried? + self.saves_tipped?;
        Some(100.0 * f64::from(saves) / f64::from(faced))
    }

    fn position_group(&self) -> &'static str {
        let Some(text) = self.positions_text.as_deref() else {
            return "OTHER";
        };
        let positions: Vec<&str> = text
            .split('/')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect();
        if positions.contains(&"GK") {
            return "GK";
        }
        let Some(first) = positions.first() else {
            return "OTHER";
        };
        match first.to_ascii_uppercase().as_str() {
            "DC" | "SW" => "DC",
            "DL" | "DR" | "WBL" | "WBR" => "FB",
            "DM" | "MC" | "ML" | "MR" => "MID",
            "AMC" | "AML" | "AMR" => "AM",
            "STC" | "ST" | "SC" => "ST",
            _ => "OTHER",
        }
    }

    fn reputation_tier(&self) -> &'static str {
        match self.club_reputation.unwrap_or(0) {
            0 => "unknown",
            1..=1999 => "1 <2k",
            2000..=3999 => "2 2-4k",
            4000..=5999 => "3 4-6k",
            6000..=7999 => "4 6-8k",
            _ => "5 8k+",
        }
    }
}

fn core_metrics(group: &str) -> &'static [(Metric, f64)] {
    match group {
        "ST" => &[
            (Metric::Xg90, 0.35),
            (Metric::Xa90, 0.25),
            (Metric::ShotsOnTarget90, 0.20),
            (Metric::Conversion, 0.20),
        ],
        "AM" => &[
            (Metric::Xg90, 0.30),
            (Metric::Xa90, 0.35),
            (Metric::ShotsOnTarget90, 0.20),
            (Metric::PassCompletion, 0.15),
        ],
        "MID" => &[
            (Metric::Xa90, 0.30),
            (Metric::PassCompletion, 0.25),
            (Metric::PassesCompleted90, 0.20),
            (Metric::Tackles90, 0.25),
        ],
        "DC" => &[
            (Metric::Tackles90, 0.40),
            (Metric::TackleCompletion, 0.25),
            (Metric::PassCompletion, 0.35),
        ],
        "FB" => &[
            (Metric::Tackles90, 0.35),
            (Metric::TackleCompletion, 0.20),
            (Metric::PassCompletion, 0.30),
            (Metric::Xa90, 0.15),
        ],
        "GK" => &[
            (Metric::GoalsAllowed90, 0.50),
            (Metric::CleanSheets90, 0.30),
            (Metric::Rating, 0.20),
        ],
        _ => &[],
    }
}

fn advanced_metrics(group: &str) -> &'static [(Metric, f64)] {
    match group {
        "ST" => &[
            (Metric::Xg90, 0.30),
            (Metric::Xa90, 0.20),
            (Metric::KeyPasses90, 0.15),
            (Metric::ShotsOnTarget90, 0.20),
            (Metric::Conversion, 0.15),
        ],
        "AM" => &[
            (Metric::Xg90, 0.20),
            (Metric::Xa90, 0.25),
            (Metric::KeyPasses90, 0.20),
            (Metric::ShotsOnTarget90, 0.15),
            (Metric::Dribbles90, 0.20),
        ],
        "MID" => &[
            (Metric::Xa90, 0.20),
            (Metric::KeyPasses90, 0.15),
            (Metric::ProgressivePasses90, 0.20),
            (Metric::PassCompletion, 0.15),
            (Metric::Tackles90, 0.15),
            (Metric::Interceptions90, 0.15),
        ],
        "DC" => &[
            (Metric::Tackles90, 0.20),
            (Metric::Interceptions90, 0.20),
            (Metric::HeadersPercent, 0.20),
            (Metric::Clearances90, 0.15),
            (Metric::Blocks90, 0.10),
            (Metric::PassCompletion, 0.15),
        ],
        "FB" => &[
            (Metric::Tackles90, 0.20),
            (Metric::Interceptions90, 0.15),
            (Metric::HeadersPercent, 0.10),
            (Metric::Clearances90, 0.10),
            (Metric::KeyPasses90, 0.15),
            (Metric::Crosses90, 0.10),
            (Metric::PassCompletion, 0.20),
        ],
        "GK" => &[
            (Metric::SavePercent, 0.45),
            (Metric::ExpectedGoalsPrevented90, 0.35),
            (Metric::GoalsAllowed90, 0.20),
        ],
        _ => &[],
    }
}

fn fee_for(candidate: &StatsMoneyballCandidate) -> (Option<u64>, bool) {
    if candidate.transfer_value_state == "ok"
        && let Some(value) = candidate.transfer_value
    {
        return (Some(u64::from(value)), true);
    }
    if candidate.club_uid.is_none() && candidate.transfer_value_state == "zero" {
        return (Some(0), true);
    }
    (None, false)
}

fn cost_for(fee: Option<u64>, wage: Option<u32>) -> Option<u64> {
    if fee.is_none() && wage.is_none() {
        return None;
    }
    if wage.is_none() && fee.is_some_and(|value| value == 0) {
        return None;
    }
    let fee = fee.unwrap_or(0);
    let wages = u64::from(wage.unwrap_or(0)) * 52 * COST_HORIZON_YEARS as u64;
    Some(fee.saturating_add(wages))
}

fn confidence(candidate: &StatsMoneyballCandidate, fee_known: bool, wage_known: bool) -> f64 {
    let mut value = 0.55f64;
    if candidate.minutes >= 900 {
        value += 0.15;
    } else if candidate.minutes >= 450 {
        value += 0.08;
    }
    if candidate.average_rating.is_some() {
        value += 0.10;
    }
    if candidate.advanced_available {
        value += 0.05;
    }
    if fee_known {
        value += 0.05;
    }
    if wage_known {
        value += 0.05;
    }
    if candidate.club_reputation.is_some() {
        value += 0.05;
    }
    value.clamp(0.0, 1.0)
}

fn flags(
    candidate: &StatsMoneyballCandidate,
    fee_known: bool,
    wage_known: bool,
    confidence: f64,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !fee_known {
        flags.push("missing_fee".to_owned());
    }
    if !wage_known {
        flags.push("missing_wage".to_owned());
    }
    if candidate.club_uid.is_none() {
        flags.push("free_agent".to_owned());
    }
    if candidate.transfer_listed {
        flags.push("transfer_listed".to_owned());
    }
    if candidate.loan_listed {
        flags.push("loan_listed".to_owned());
    }
    if candidate.injury_status == "injured" {
        flags.push("injured".to_owned());
    }
    if candidate.recent_injury.is_some() {
        flags.push("injury_history".to_owned());
    }
    if candidate.league_name.is_none() {
        flags.push("unknown_league".to_owned());
    }
    if !candidate.advanced_available {
        flags.push("advanced_missing".to_owned());
    }
    if candidate.minutes < 450 {
        flags.push("small_sample".to_owned());
    }
    if confidence < 0.7 {
        flags.push("low_confidence".to_owned());
    }
    flags
}

fn sort_value(row: &StatsMoneyballRow, by: &str) -> Option<f64> {
    match by {
        "score" => row.score,
        "surplus" => row.surplus,
        "production" => row.production_tier.or(row.production_global),
        "cost" => Some(row.cost as f64),
        "fee" | "value" => row.fee.map(|value| value as f64),
        "wage" => row.wage.map(f64::from),
        "age" => row.age.map(f64::from),
        "ca" => Some(f64::from(row.ca)),
        "minutes" => Some(f64::from(row.minutes)),
        "rating" => row.average_rating,
        "confidence" => Some(row.confidence),
        _ => row.score,
    }
}

fn compare_rows(
    left: &StatsMoneyballRow,
    right: &StatsMoneyballRow,
    by: &str,
    ascending: bool,
) -> Ordering {
    if by == "name" {
        let order = left
            .name
            .as_deref()
            .unwrap_or("")
            .cmp(right.name.as_deref().unwrap_or(""));
        let order = if ascending { order } else { order.reverse() };
        return order.then_with(|| left.uid.cmp(&right.uid));
    }
    let left_value = sort_value(left, by);
    let right_value = sort_value(right, by);
    match (left_value, right_value) {
        (None, None) => left.uid.cmp(&right.uid),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left_value), Some(right_value)) => {
            let order = left_value
                .partial_cmp(&right_value)
                .unwrap_or(Ordering::Equal);
            let order = if ascending { order } else { order.reverse() };
            order.then_with(|| left.uid.cmp(&right.uid))
        }
    }
}

fn shrink(value: f64, peer_mean: f64, minutes: u32) -> f64 {
    (value * f64::from(minutes) + peer_mean * SHRINK_MINUTES)
        / (f64::from(minutes) + SHRINK_MINUTES)
}

fn assign_percentiles(
    candidates: &[StatsMoneyballCandidate],
    adjusted: &[HashMap<Metric, f64>],
    metrics: &HashSet<Metric>,
    basis: &str,
) -> Vec<HashMap<Metric, f64>> {
    let mut output: Vec<HashMap<Metric, f64>> = vec![HashMap::new(); candidates.len()];
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let key = match basis {
            "global" => candidate.position_group().to_owned(),
            _ => format!(
                "{}|{}",
                candidate.position_group(),
                candidate.reputation_tier()
            ),
        };
        buckets.entry(key).or_default().push(index);
    }
    for indices in buckets.values() {
        for metric in metrics {
            let values: Vec<Option<f64>> = indices
                .iter()
                .map(|index| adjusted[*index].get(metric).copied())
                .collect();
            let ranks = crate::moneyball::percentiles(&values);
            for (offset, index) in indices.iter().enumerate() {
                if let Some(rank) = ranks[offset] {
                    let pct = if metric.inverse() {
                        100.0 * (1.0 - rank)
                    } else {
                        100.0 * rank
                    };
                    output[*index].insert(*metric, pct);
                }
            }
        }
    }
    output
}

fn metric_means(
    candidates: &[StatsMoneyballCandidate],
    indices: &[usize],
    metrics: &[(Metric, f64)],
) -> HashMap<Metric, f64> {
    let mut cell = HashMap::new();
    for (metric, _) in metrics {
        let mut total = 0.0;
        let mut count = 0usize;
        for index in indices {
            if let Some(value) = metric.value(&candidates[*index]) {
                total += value;
                count += 1;
            }
        }
        if count > 0 {
            cell.insert(*metric, total / count as f64);
        }
    }
    cell
}

fn tier_means(
    candidates: &[StatsMoneyballCandidate],
    indices: &[usize],
    metrics: &[(Metric, f64)],
) -> HashMap<&'static str, HashMap<Metric, f64>> {
    let mut tiers: HashMap<&'static str, Vec<usize>> = HashMap::new();
    for index in indices {
        tiers
            .entry(candidates[*index].reputation_tier())
            .or_default()
            .push(*index);
    }
    tiers
        .into_iter()
        .map(|(tier, indices)| (tier, metric_means(candidates, &indices, metrics)))
        .collect()
}

fn model_metrics(model: &str, group: &str) -> &'static [(Metric, f64)] {
    if model == "advanced" {
        advanced_metrics(group)
    } else {
        core_metrics(group)
    }
}

/// Score every candidate without applying request filters. Percentiles stay
/// stable while the user changes filters; callers can cache the result.
pub fn score_stats_candidates(
    mut candidates: Vec<StatsMoneyballCandidate>,
    model: &str,
    min_minutes: u32,
) -> Result<Vec<StatsMoneyballRow>> {
    let model = normalized_model(Some(model))?;

    candidates.retain(|candidate| {
        candidate.minutes >= min_minutes
            && candidate.position_group() != "OTHER"
            && (model == "core" || candidate.advanced_available)
    });

    let groups: Vec<&'static str> = candidates
        .iter()
        .map(StatsMoneyballCandidate::position_group)
        .collect();

    // Peer means are computed per position group and per position group x
    // reputation tier. A thin tier falls back to the group mean.
    let mut adjusted: Vec<HashMap<Metric, f64>> = vec![HashMap::new(); candidates.len()];
    let mut all_metrics: HashSet<Metric> = HashSet::new();
    for group in ["GK", "DC", "FB", "MID", "AM", "ST"] {
        let indices: Vec<usize> = groups
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value == group).then_some(index))
            .collect();
        if indices.is_empty() {
            continue;
        }
        let metrics = model_metrics(model, group);
        for (metric, _) in metrics {
            all_metrics.insert(*metric);
        }
        let group_means = metric_means(&candidates, &indices, metrics);
        let tier_means_by_bucket = tier_means(&candidates, &indices, metrics);
        for index in indices {
            let candidate = &candidates[index];
            let tier_key = candidate.reputation_tier();
            for (metric, _) in metrics {
                let Some(value) = metric.value(candidate) else {
                    continue;
                };
                let peer = tier_means_by_bucket
                    .get(tier_key)
                    .and_then(|cell| cell.get(metric))
                    .or_else(|| group_means.get(metric))
                    .copied();
                let Some(peer) = peer else {
                    continue;
                };
                adjusted[index].insert(*metric, shrink(value, peer, candidate.minutes));
            }
        }
    }

    let pct_global = assign_percentiles(&candidates, &adjusted, &all_metrics, "global");
    let pct_tier = assign_percentiles(&candidates, &adjusted, &all_metrics, "tier");

    // Cost percentiles are computed inside the same peer buckets.
    let mut cost_pct_global: Vec<Option<f64>> = vec![None; candidates.len()];
    let mut cost_pct_tier: Vec<Option<f64>> = vec![None; candidates.len()];
    let mut cost_values: Vec<Option<f64>> = vec![None; candidates.len()];
    for (index, candidate) in candidates.iter().enumerate() {
        let (fee, _) = fee_for(candidate);
        cost_values[index] = cost_for(fee, candidate.wage).map(|cost| cost as f64);
    }
    for basis in ["global", "tier"] {
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if cost_values[index].is_none() {
                continue;
            }
            let key = if basis == "global" {
                groups[index].to_owned()
            } else {
                format!("{}|{}", groups[index], candidate.reputation_tier())
            };
            buckets.entry(key).or_default().push(index);
        }
        for indices in buckets.values() {
            let values: Vec<Option<f64>> =
                indices.iter().map(|index| cost_values[*index]).collect();
            let ranks = crate::moneyball::percentiles(&values);
            for (offset, index) in indices.iter().enumerate() {
                if let Some(rank) = ranks[offset] {
                    if basis == "global" {
                        cost_pct_global[*index] = Some(100.0 * rank);
                    } else {
                        cost_pct_tier[*index] = Some(100.0 * rank);
                    }
                }
            }
        }
    }

    let mut rows = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let group = groups[index];
        let metrics = model_metrics(model, group);
        let (fee, fee_known) = fee_for(&candidate);
        let wage_known = candidate.wage.is_some();
        let Some(cost) = cost_for(fee, candidate.wage) else {
            continue;
        };
        if cost == 0 {
            continue;
        }

        let production =
            |pct: &[HashMap<Metric, f64>], with_drivers: bool| -> Option<(f64, Vec<StatsDriver>)> {
                let total_weight: f64 = metrics.iter().map(|(_, weight)| *weight).sum();
                if total_weight <= 0.0 {
                    return None;
                }
                let mut available_weight = 0.0;
                let mut weighted = 0.0;
                let mut drivers = Vec::new();
                for (metric, weight) in metrics {
                    if let Some(value) = pct[index].get(metric) {
                        available_weight += weight;
                        weighted += value * weight;
                        if with_drivers {
                            drivers.push(StatsDriver {
                                key: metric.key().to_owned(),
                                label: metric.label().to_owned(),
                                percentile: *value,
                                contribution: 0.0,
                            });
                        }
                    }
                }
                let coverage = available_weight / total_weight;
                let minimum = if model == "advanced" { 0.99 } else { 0.60 };
                if coverage < minimum {
                    return None;
                }
                let production = weighted / available_weight;
                if with_drivers {
                    for driver in &mut drivers {
                        let weight = metrics
                            .iter()
                            .find(|(metric, _)| metric.key() == driver.key)
                            .map(|(_, weight)| *weight)
                            .unwrap_or(0.0);
                        driver.contribution = driver.percentile * weight / available_weight;
                    }
                    drivers.sort_by(|left, right| {
                        right
                            .contribution
                            .partial_cmp(&left.contribution)
                            .unwrap_or(Ordering::Equal)
                    });
                    drivers.truncate(3);
                }
                Some((production, drivers))
            };

        let Some((production_global, _)) = production(&pct_global, false) else {
            continue;
        };
        let Some((production_tier, drivers)) = production(&pct_tier, true) else {
            continue;
        };
        let cost_pct_global = cost_pct_global[index];
        let Some(cost_pct_tier) = cost_pct_tier[index] else {
            continue;
        };
        let surplus = Some(production_tier - cost_pct_tier);
        let score = Some(0.55 * production_tier + 0.45 * (100.0 - cost_pct_tier));
        let confidence = confidence(&candidate, fee_known, wage_known);
        let flags = flags(&candidate, fee_known, wage_known, confidence);

        let save_percent = candidate.save_percent();
        rows.push(StatsMoneyballRow {
            uid: candidate.uid,
            name: candidate.name,
            age: candidate.age,
            club_uid: candidate.club_uid,
            club_name: candidate.club_name,
            league_name: candidate.league_name,
            nation_name: candidate.nation_name,
            ca: candidate.ca,
            pa: candidate.pa,
            position_group: group.to_owned(),
            minutes: candidate.minutes,
            starts: candidate.starts,
            substitute_appearances: candidate.substitute_appearances,
            average_rating: candidate.average_rating,
            goals: candidate.goals,
            assists: candidate.assists,
            expected_goals: candidate.expected_goals,
            expected_assists: candidate.expected_assists,
            shots: candidate.shots,
            shots_on_target: candidate.shots_on_target,
            passes_attempted: candidate.passes_attempted,
            passes_completed: candidate.passes_completed,
            pass_completion_percent: percent(
                candidate.passes_completed,
                candidate.passes_attempted,
            ),
            tackles_attempted: candidate.tackles_attempted,
            tackles_completed: candidate.tackles_completed,
            interceptions: candidate.interceptions,
            key_passes: candidate.key_passes,
            progressive_passes: candidate.progressive_passes,
            headers_won: candidate.headers_won,
            aerial_challenges_attempted: candidate.aerial_challenges_attempted,
            clearances: candidate.clearances,
            blocks: candidate.blocks,
            crosses_completed: candidate.crosses_completed,
            saves_held: candidate.saves_held,
            shots_on_target_faced: candidate.shots_on_target_faced,
            save_percent,
            expected_goals_prevented: candidate.expected_goals_prevented,
            advanced_available: candidate.advanced_available,
            transfer_value: candidate.transfer_value,
            transfer_value_state: candidate.transfer_value_state,
            wage: candidate.wage,
            contract_end: candidate.contract_end,
            club_reputation: candidate.club_reputation,
            fee,
            cost,
            fee_known,
            wage_known,
            production_global: Some(production_global),
            production_tier: Some(production_tier),
            cost_pct_global,
            cost_pct_tier: Some(cost_pct_tier),
            surplus,
            score,
            confidence,
            flags,
            drivers,
            transfer_listed: candidate.transfer_listed,
            loan_listed: candidate.loan_listed,
            injury_status: candidate.injury_status,
            recent_injury: candidate.recent_injury,
        });
    }

    Ok(rows)
}

/// Apply request filters, sorting and pagination to already-scored rows. This
/// is deliberately separate from scoring so page changes and filters can reuse
/// the expensive percentile pass.
pub fn apply_stats_request(
    rows: &[StatsMoneyballRow],
    request: &StatsMoneyballRequest,
) -> Result<StatsMoneyballPage> {
    let model = normalized_model(request.model.as_deref())?;
    let min_minutes = request.min_minutes.unwrap_or(900);
    let position_filter = request
        .position_group
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());
    if let Some(group) = position_filter.as_deref()
        && !matches!(group, "GK" | "DC" | "FB" | "MID" | "AM" | "ST")
    {
        return Err(Error::parse(format!("unknown position group {group:?}")));
    }
    for listing in &request.listings {
        if !matches!(listing.as_str(), "sale" | "loan" | "free") {
            return Err(Error::parse(format!("unknown listing filter {listing:?}")));
        }
    }
    let include_goalkeepers =
        request.include_goalkeepers || position_filter.as_deref() == Some("GK");

    let mut filtered: Vec<&StatsMoneyballRow> = rows
        .iter()
        .filter(|row| {
            if !include_goalkeepers && row.position_group == "GK" {
                return false;
            }
            if !request.include_unknown_fee && !row.fee_known {
                return false;
            }
            if let Some(group) = position_filter.as_deref()
                && row.position_group != group
            {
                return false;
            }
            if let Some(league) = request.league.as_deref()
                && row.league_name.as_deref() != Some(league)
            {
                return false;
            }
            if let Some(min) = request.age_min
                && row.age.is_none_or(|age| age < min)
            {
                return false;
            }
            if let Some(max) = request.age_max
                && row.age.is_none_or(|age| age > max)
            {
                return false;
            }
            if let Some(min) = request.ca_min
                && row.ca < min
            {
                return false;
            }
            if let Some(max) = request.ca_max
                && row.ca > max
            {
                return false;
            }
            if let Some(max_fee) = request.max_fee {
                match row.fee {
                    Some(value) if value <= max_fee => {}
                    Some(_) => return false,
                    None if request.include_unknown_fee => {}
                    None => return false,
                }
            }
            if let Some(max_wage) = request.max_wage {
                match row.wage {
                    Some(value) if u64::from(value) <= max_wage => {}
                    _ => return false,
                }
            }
            if let Some(before) = request.contract_end_before.as_deref()
                && row.contract_end.as_deref().is_none_or(|end| end > before)
            {
                return false;
            }
            if !request.listings.is_empty()
                && !request
                    .listings
                    .iter()
                    .any(|listing| match listing.as_str() {
                        "sale" => row.transfer_listed,
                        "loan" => row.loan_listed,
                        "free" => row.club_uid.is_none(),
                        _ => false,
                    })
            {
                return false;
            }
            if let Some(min) = request.min_confidence
                && row.confidence < min
            {
                return false;
            }
            if request.exclude_injured && row.injury_status == "injured" {
                return false;
            }
            if request.exclude_injury_history && row.recent_injury.is_some() {
                return false;
            }
            true
        })
        .collect();

    let leagues: Vec<String> = {
        let mut names = std::collections::BTreeSet::new();
        for row in rows {
            if let Some(league) = row.league_name.as_deref() {
                names.insert(league.to_owned());
            }
        }
        names.into_iter().collect()
    };

    let by = request.sort_by.as_deref().unwrap_or("score");
    let ascending = request
        .sort_dir
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("asc"));
    filtered.sort_by(|left, right| compare_rows(left, right, by, ascending));

    let page = request.page.unwrap_or(0);
    let page_size = request.page_size.unwrap_or(100).clamp(1, 500);
    let start = (page as usize).saturating_mul(page_size as usize);
    let page_rows = if start >= filtered.len() {
        Vec::new()
    } else {
        filtered[start..(start + page_size as usize).min(filtered.len())]
            .iter()
            .map(|row| (*row).clone())
            .collect()
    };
    Ok(StatsMoneyballPage {
        total: filtered.len(),
        page,
        page_size,
        rows: page_rows,
        meta: StatsMoneyballMeta {
            model: model_name(model).to_owned(),
            scored: rows.len(),
            filtered: filtered.len(),
            min_minutes,
            cost_horizon_years: COST_HORIZON_YEARS,
            leagues,
        },
    })
}

/// Score candidates and return one filtered page. Percentiles and filters are
/// applied in that order so changing a filter does not move the ranking.
pub fn score_stats_moneyball(
    candidates: Vec<StatsMoneyballCandidate>,
    request: &StatsMoneyballRequest,
) -> Result<StatsMoneyballPage> {
    let model = normalized_model(request.model.as_deref())?;
    let min_minutes = request.min_minutes.unwrap_or(900);
    let rows = score_stats_candidates(candidates, model, min_minutes)?;
    apply_stats_request(&rows, request)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        uid: u32,
        xg: f64,
        xa: f64,
        minutes: u32,
        fee: u32,
        wage: u32,
        advanced_available: bool,
    ) -> StatsMoneyballCandidate {
        StatsMoneyballCandidate {
            uid,
            name: Some(format!("Player {uid}")),
            age: Some(24),
            club_uid: Some(10),
            club_name: Some("Club".to_owned()),
            league_name: Some("League".to_owned()),
            nation_name: Some("Nation".to_owned()),
            ca: 150,
            pa: Some(160),
            positions_text: Some("STC".to_owned()),
            transfer_value: Some(fee),
            transfer_value_state: "ok".to_owned(),
            wage: Some(wage),
            contract_end: Some("2029-06-30".to_owned()),
            club_reputation: Some(6000),
            transfer_listed: false,
            loan_listed: false,
            injury_status: "fit".to_owned(),
            recent_injury: None,
            minutes,
            starts: minutes / 90,
            substitute_appearances: 0,
            average_rating: Some(7.0),
            goals: 10,
            assists: 5,
            expected_goals: xg * f64::from(minutes) / 90.0,
            expected_assists: xa * f64::from(minutes) / 90.0,
            shots: 40,
            shots_on_target: 20,
            passes_attempted: 400,
            passes_completed: 320,
            tackles_attempted: 10,
            tackles_completed: 7,
            goals_allowed: None,
            clean_sheets: None,
            interceptions: Some(10),
            key_passes: Some(20),
            progressive_passes: Some(30),
            dribbles: Some(20),
            headers_won: Some(20),
            aerial_challenges_attempted: Some(40),
            clearances: Some(10),
            blocks: Some(5),
            crosses_completed: Some(5),
            saves_held: None,
            saves_parried: None,
            saves_tipped: None,
            shots_on_target_faced: None,
            expected_goals_prevented: None,
            advanced_available,
            current_club: true,
        }
    }

    #[test]
    fn cheap_production_beats_expensive_production() {
        let page = score_stats_moneyball(
            vec![
                candidate(1, 0.8, 0.4, 1800, 1_000, 100, true),
                candidate(2, 0.5, 0.2, 1800, 10_000_000, 1_000, true),
            ],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.rows[0].uid, 1);
        assert!(page.rows[0].score.unwrap() > page.rows[1].score.unwrap());
    }

    #[test]
    fn injury_filters_exclude_unavailable_players() {
        let mut injured = candidate(1, 0.8, 0.4, 1800, 1_000, 100, true);
        injured.injury_status = "injured".to_owned();
        injured.recent_injury = Some("2026-08-01".to_owned());
        let mut history = candidate(2, 0.8, 0.4, 1800, 1_000, 100, true);
        history.recent_injury = Some("2026-08-01".to_owned());
        let fit = candidate(3, 0.8, 0.4, 1800, 1_000, 100, true);

        let mut request = StatsMoneyballRequest {
            model: Some("core".to_owned()),
            min_minutes: Some(900),
            exclude_injured: true,
            ..StatsMoneyballRequest::default()
        };
        let page = score_stats_moneyball(
            vec![injured.clone(), history.clone(), fit.clone()],
            &request,
        )
        .expect("score");
        assert_eq!(page.total, 2);
        assert!(page.rows.iter().all(|row| row.uid != 1));

        request.exclude_injury_history = true;
        let page = score_stats_moneyball(vec![injured, history, fit], &request).expect("score");
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].uid, 3);
    }

    #[test]
    fn advanced_model_excludes_rows_without_the_advanced_block() {
        let page = score_stats_moneyball(
            vec![
                candidate(1, 0.8, 0.4, 1800, 1_000, 100, true),
                candidate(2, 0.7, 0.3, 1800, 1_000, 100, false),
            ],
            &StatsMoneyballRequest {
                model: Some("advanced".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].uid, 1);
    }

    #[test]
    fn goalkeepers_are_separate_by_default() {
        let mut gk = candidate(2, 0.0, 0.0, 1800, 1_000, 100, true);
        gk.positions_text = Some("GK".to_owned());
        gk.goals_allowed = Some(20);
        gk.clean_sheets = Some(8);
        gk.saves_held = Some(60);
        gk.saves_parried = Some(20);
        gk.saves_tipped = Some(10);
        gk.shots_on_target_faced = Some(100);
        let outfield = candidate(1, 0.8, 0.4, 1800, 1_000, 100, true);
        let page = score_stats_moneyball(
            vec![outfield.clone(), gk.clone()],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].uid, 1);

        let gk_page = score_stats_moneyball(
            vec![outfield, gk],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                position_group: Some("GK".to_owned()),
                ..StatsMoneyballRequest::default()
            },
        )
        .unwrap();
        assert_eq!(gk_page.total, 1);
        assert_eq!(gk_page.rows[0].position_group, "GK");
    }

    #[test]
    fn filters_are_applied_after_scoring() {
        let page = score_stats_moneyball(
            vec![
                candidate(1, 0.8, 0.4, 1800, 1_000, 100, true),
                candidate(2, 0.5, 0.2, 1800, 10_000_000, 1_000, true),
            ],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ca_min: Some(160),
                ..StatsMoneyballRequest::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 0);
        assert!(page.rows.is_empty());
    }
}
