//! Stats-based Moneyball scoring.
//!
//! The model has two tiers:
//!
//! - `core` only uses fields present for every player (npxG, xA, shots, passing,
//!   tackles, rating). It is the safe all-player ranking.
//! - `advanced` adds key passes, progressive passes, interceptions, headers,
//!   clearances, blocks, dribbles, crosses, save percentage and xGP. It is only
//!   valid for rows whose `advanced_available` flag is true; the rest are left
//!   unscored instead of treating the save's zero block as real data.
//!
//! Percentiles are computed inside a position group and, when the peer bucket is
//! large enough, inside a position group x reputation tier. The candidate set is
//! fixed at `SCORE_FLOOR_MINUTES`; the request's `min_minutes` only filters the
//! display, so changing it never moves the ranking.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::moneyball::position_group_for;

pub const STATS_CORE_MODEL: &str = "stats-core-v3";
pub const STATS_ADVANCED_MODEL: &str = "stats-advanced-v3";
const COST_HORIZON_YEARS: f64 = 3.0;
/// Every player with at least this many minutes is scored once. Requests filter
/// the scored rows afterwards.
pub const SCORE_FLOOR_MINUTES: u32 = 300;
/// A reputation-tier bucket thinner than this falls back to the position group.
const MIN_PEER_SAMPLE: usize = 30;
/// 80% interval on the production percentile.
const UNCERTAINTY_Z: f64 = 1.2816;

/// Normalize a model name to `core` or `advanced`.
pub fn normalized_model(model: Option<&str>) -> Result<&'static str> {
    match model.unwrap_or("core").trim().to_ascii_lowercase().as_str() {
        "core" => Ok("core"),
        "advanced" => Ok("advanced"),
        other => Err(Error::parse(format!("unknown stats model {other:?}"))),
    }
}

/// Normalize the requested statistics window to the save's `kind` value.
pub fn normalized_window(window: Option<&str>) -> Result<&'static str> {
    match window
        .unwrap_or("overall")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "overall" | "" => Ok("overall"),
        "league" => Ok("league"),
        "calendar" | "calendar_year" | "12m" => Ok("calendar_year_overall"),
        other => Err(Error::parse(format!("unknown stats window {other:?}"))),
    }
}

/// Normalize the visibility mode used by the analysis views.
pub fn normalized_mode(mode: Option<&str>) -> Result<&'static str> {
    match mode.unwrap_or("full").trim().to_ascii_lowercase().as_str() {
        "full" | "" => Ok("full"),
        "fair_play" | "fair-play" | "fairplay" => Ok("fair_play"),
        "moneyball" => Ok("moneyball"),
        other => Err(Error::parse(format!("unknown data mode {other:?}"))),
    }
}

pub fn window_label(kind: &str) -> &'static str {
    match kind {
        "league" => "league",
        "calendar_year_overall" => "calendar",
        _ => "overall",
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
    /// `overall` (default), `league`, or `calendar` (last 12 months).
    pub window: Option<String>,
    /// `full` (default), `fair_play` or `moneyball`. Non-full modes remove
    /// CA/PA from the price model.
    pub mode: Option<String>,
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
    pub non_penalty_expected_goals: f64,
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
    pub production_low: Option<f64>,
    pub production_high: Option<f64>,
    pub cost_pct_global: Option<f64>,
    pub cost_pct_tier: Option<f64>,
    pub surplus: Option<f64>,
    pub score: Option<f64>,
    /// Score computed from the production lower bound; used by the conservative
    /// sort so thin samples cannot win on a lucky percentile.
    pub score_low: Option<f64>,
    /// Expected market value from the fair-price model, in save units.
    pub fair_price: Option<f64>,
    /// `(fair_price - fee) / fair_price * 100`; positive means cheaper than the model.
    pub value_gap_pct: Option<f64>,
    /// Quality plus the position-relative value gap. Price is separate from
    /// `score`, which is quality only.
    pub bargain_score: Option<f64>,
    /// Players in the peer bucket that produced the tier percentiles.
    pub peer_count: u32,
    pub confidence: f64,
    pub flags: Vec<String>,
    pub drivers: Vec<StatsDriver>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
    pub injury_recent: bool,
    /// True when the season-stat row was recorded for another club.
    pub stats_from_other_club: bool,
    /// Metric percentile vector used by the similarity search. Internal: the
    /// UI reads the same numbers through drivers and does not need the full map.
    #[serde(skip_serializing, skip_deserializing)]
    pub metric_percentiles: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsMoneyballMeta {
    pub model: String,
    pub scored: usize,
    pub filtered: usize,
    pub min_minutes: u32,
    /// The candidate set's fixed minute floor, independent of `min_minutes`.
    pub score_floor_minutes: u32,
    pub cost_horizon_years: f64,
    pub leagues: Vec<String>,
    /// Statistics window actually used: `overall`, `league` or `calendar`.
    pub window: String,
    /// Visibility mode used for the price model: full, fair_play or moneyball.
    pub mode: String,
    /// Fair-price model name when at least one row was priced.
    pub price_model: Option<String>,
    /// Scored rows that received a fair price.
    pub priced: usize,
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
    /// Remaining contract years at the save's game date; drives the price model.
    pub contract_years: Option<f64>,
    pub club_reputation: Option<u16>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
    /// True when the most recent injury is inside the history window.
    pub injury_recent: bool,
    pub minutes: u32,
    pub starts: u32,
    pub substitute_appearances: u32,
    pub average_rating: Option<f64>,
    pub goals: u32,
    pub assists: u32,
    pub expected_goals: f64,
    pub non_penalty_expected_goals: f64,
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
    pub key_tackles: Option<u32>,
    pub possession_won: Option<u32>,
    pub clear_cut_chances_created: Option<u32>,
    pub key_headers: Option<u32>,
    pub distance_km: Option<f64>,
    pub mistakes_leading_to_goal: u32,
    pub fouls_made: u32,
    pub yellow_cards: u32,
    pub red_cards: u32,
    pub saves_held: Option<u32>,
    pub saves_parried: Option<u32>,
    pub saves_tipped: Option<u32>,
    pub shots_on_target_faced: Option<u32>,
    pub expected_goals_prevented: Option<f64>,
    pub advanced_available: bool,
    /// True when this is the row for the player's current club. Used only to
    /// pick one row per player before scoring.
    pub current_club: bool,
    /// True when the selected row belongs to a club the player has since left.
    pub stats_from_other_club: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Metric {
    Npxg90,
    Xa90,
    ShotsOnTarget90,
    KeyPasses90,
    Dribbles90,
    ProgressivePasses90,
    Interceptions90,
    HeadersPercent,
    Clearances90,
    Blocks90,
    Crosses90,
    KeyTackles90,
    PossessionWon90,
    ClearCutChances90,
    KeyHeaders90,
    Distance90,
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
            Self::Npxg90 => "npxg90",
            Self::Xa90 => "xa90",
            Self::ShotsOnTarget90 => "sot90",
            Self::KeyPasses90 => "kp90",
            Self::Dribbles90 => "drib90",
            Self::ProgressivePasses90 => "prog90",
            Self::Interceptions90 => "int90",
            Self::HeadersPercent => "headers_pct",
            Self::Clearances90 => "clearances90",
            Self::Blocks90 => "blocks90",
            Self::Crosses90 => "crosses90",
            Self::KeyTackles90 => "key_tackles90",
            Self::PossessionWon90 => "possession_won90",
            Self::ClearCutChances90 => "ccc90",
            Self::KeyHeaders90 => "key_headers90",
            Self::Distance90 => "distance90",
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
            Self::Npxg90 => "npxG/90",
            Self::Xa90 => "xA/90",
            Self::ShotsOnTarget90 => "Shots on target/90",
            Self::KeyPasses90 => "Key passes/90",
            Self::Dribbles90 => "Dribbles/90",
            Self::ProgressivePasses90 => "Progressive passes/90",
            Self::Interceptions90 => "Interceptions/90",
            Self::HeadersPercent => "Headers won %",
            Self::Clearances90 => "Clearances/90",
            Self::Blocks90 => "Blocks/90",
            Self::Crosses90 => "Crosses completed/90",
            Self::KeyTackles90 => "Key tackles/90",
            Self::PossessionWon90 => "Possessions won/90",
            Self::ClearCutChances90 => "Clear-cut chances/90",
            Self::KeyHeaders90 => "Key headers/90",
            Self::Distance90 => "Distance/90",
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
                | Self::KeyTackles90
                | Self::PossessionWon90
                | Self::ClearCutChances90
                | Self::KeyHeaders90
                | Self::Distance90
                | Self::SavePercent
                | Self::ExpectedGoalsPrevented90
        )
    }

    fn inverse(self) -> bool {
        matches!(self, Self::GoalsAllowed90)
    }

    /// Success rates need a Beta-Binomial prior measured in attempts, not in
    /// minutes. Everything else is a count scaled per 90 minutes.
    fn is_rate(self) -> bool {
        matches!(
            self,
            Self::PassCompletion
                | Self::TackleCompletion
                | Self::HeadersPercent
                | Self::SavePercent
        )
    }

    /// Attempts behind a success rate; `None` for count metrics.
    fn attempts(self, candidate: &StatsMoneyballCandidate) -> Option<f64> {
        match self {
            Self::PassCompletion => Some(f64::from(candidate.passes_attempted)),
            Self::TackleCompletion => Some(f64::from(candidate.tackles_attempted)),
            Self::HeadersPercent => candidate.aerial_challenges_attempted.map(f64::from),
            Self::SavePercent => candidate.shots_on_target_faced.map(f64::from),
            _ => None,
        }
    }

    /// Exposure for the empirical prior: attempts for a rate, minutes for a
    /// count (the Gamma-Poisson prior is measured in minutes).
    fn exposure(self, candidate: &StatsMoneyballCandidate) -> Option<f64> {
        if self.is_rate() {
            return self.attempts(candidate).filter(|value| *value > 0.0);
        }
        (candidate.minutes > 0).then(|| f64::from(candidate.minutes))
    }

    /// Independent evidence behind the value, used for the percentile
    /// uncertainty: attempts for a rate, 90-minute matches for a count.
    fn evidence(self, candidate: &StatsMoneyballCandidate) -> Option<f64> {
        if self.is_rate() {
            return self.attempts(candidate).filter(|value| *value > 0.0);
        }
        (candidate.minutes > 0).then(|| f64::from(candidate.minutes) / 90.0)
    }

    fn value(self, candidate: &StatsMoneyballCandidate) -> Option<f64> {
        if self.advanced() && !candidate.advanced_available {
            return None;
        }
        let per90 = |count: u32| {
            (candidate.minutes > 0).then(|| f64::from(count) * 90.0 / f64::from(candidate.minutes))
        };
        match self {
            Self::Npxg90 => per90_f64(candidate.non_penalty_expected_goals, candidate.minutes),
            Self::Xa90 => per90_f64(candidate.expected_assists, candidate.minutes),
            Self::ShotsOnTarget90 => per90(candidate.shots_on_target),
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
            Self::KeyTackles90 => candidate.key_tackles.and_then(per90),
            Self::PossessionWon90 => candidate.possession_won.and_then(per90),
            Self::ClearCutChances90 => candidate.clear_cut_chances_created.and_then(per90),
            Self::KeyHeaders90 => candidate.key_headers.and_then(per90),
            Self::Distance90 => candidate
                .distance_km
                .and_then(|km| per90_f64(km, candidate.minutes)),
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
        position_group_for(self.positions_text.as_deref(), None)
    }

    fn reputation_tier(&self) -> &'static str {
        reputation_tier(self.club_reputation)
    }
}

fn core_metrics(group: &str) -> &'static [(Metric, f64)] {
    match group {
        "ST" => &[
            (Metric::Npxg90, 0.45),
            (Metric::Xa90, 0.25),
            (Metric::ShotsOnTarget90, 0.30),
        ],
        "AMC" => &[
            (Metric::Npxg90, 0.25),
            (Metric::Xa90, 0.40),
            (Metric::ShotsOnTarget90, 0.15),
            (Metric::PassCompletion, 0.20),
        ],
        "WM" => &[
            (Metric::Xa90, 0.40),
            (Metric::ShotsOnTarget90, 0.30),
            (Metric::PassCompletion, 0.30),
        ],
        "MC" => &[
            (Metric::Xa90, 0.25),
            (Metric::PassCompletion, 0.25),
            (Metric::PassesCompleted90, 0.20),
            (Metric::Tackles90, 0.30),
        ],
        "DM" => &[
            (Metric::Tackles90, 0.30),
            (Metric::TackleCompletion, 0.20),
            (Metric::PassCompletion, 0.30),
            (Metric::PassesCompleted90, 0.20),
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
        // Save data is in the advanced block, so the core keeper ranking leans
        // on the game's own match rating instead of the team's goals conceded.
        "GK" => &[
            (Metric::Rating, 0.55),
            (Metric::GoalsAllowed90, 0.25),
            (Metric::CleanSheets90, 0.20),
        ],
        _ => &[],
    }
}

fn advanced_metrics(group: &str) -> &'static [(Metric, f64)] {
    match group {
        "ST" => &[
            (Metric::Npxg90, 0.35),
            (Metric::Xa90, 0.15),
            (Metric::KeyPasses90, 0.15),
            (Metric::ShotsOnTarget90, 0.20),
            (Metric::ClearCutChances90, 0.15),
        ],
        "AMC" => &[
            (Metric::Npxg90, 0.20),
            (Metric::Xa90, 0.25),
            (Metric::KeyPasses90, 0.20),
            (Metric::ClearCutChances90, 0.10),
            (Metric::Dribbles90, 0.15),
            (Metric::PassCompletion, 0.10),
        ],
        "WM" => &[
            (Metric::Xa90, 0.25),
            (Metric::KeyPasses90, 0.20),
            (Metric::Crosses90, 0.15),
            (Metric::Dribbles90, 0.20),
            (Metric::ClearCutChances90, 0.10),
            (Metric::PassCompletion, 0.10),
        ],
        "MC" => &[
            (Metric::Xa90, 0.15),
            (Metric::KeyPasses90, 0.15),
            (Metric::ProgressivePasses90, 0.20),
            (Metric::PassCompletion, 0.15),
            (Metric::Tackles90, 0.15),
            (Metric::Interceptions90, 0.10),
            (Metric::PossessionWon90, 0.10),
        ],
        "DM" => &[
            (Metric::Tackles90, 0.15),
            (Metric::KeyTackles90, 0.10),
            (Metric::Interceptions90, 0.20),
            (Metric::PossessionWon90, 0.15),
            (Metric::PassCompletion, 0.20),
            (Metric::ProgressivePasses90, 0.10),
            (Metric::HeadersPercent, 0.10),
        ],
        "DC" => &[
            (Metric::Tackles90, 0.15),
            (Metric::KeyTackles90, 0.10),
            (Metric::Interceptions90, 0.15),
            (Metric::HeadersPercent, 0.20),
            (Metric::KeyHeaders90, 0.05),
            (Metric::Clearances90, 0.15),
            (Metric::Blocks90, 0.10),
            (Metric::PassCompletion, 0.10),
        ],
        "FB" => &[
            (Metric::Tackles90, 0.15),
            (Metric::Interceptions90, 0.15),
            (Metric::HeadersPercent, 0.05),
            (Metric::Clearances90, 0.10),
            (Metric::KeyPasses90, 0.15),
            (Metric::Crosses90, 0.10),
            (Metric::PassCompletion, 0.20),
            (Metric::Distance90, 0.10),
        ],
        "GK" => &[
            (Metric::SavePercent, 0.40),
            (Metric::ExpectedGoalsPrevented90, 0.35),
            (Metric::Rating, 0.25),
        ],
        _ => &[],
    }
}

/// Reputation tier used as the peer bucket. Public so squad-level views can
/// compare like with like.
pub fn reputation_tier(reputation: Option<u16>) -> &'static str {
    match reputation.unwrap_or(0) {
        0 => "unknown",
        1..=1999 => "1 <2k",
        2000..=3999 => "2 2-4k",
        4000..=5999 => "3 4-6k",
        6000..=7999 => "4 6-8k",
        _ => "5 8k+",
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
    if candidate.injury_recent {
        flags.push("injury_history".to_owned());
    }
    if candidate.stats_from_other_club {
        flags.push("stats_from_other_club".to_owned());
    }
    if candidate.mistakes_leading_to_goal > 0 {
        flags.push("mistakes".to_owned());
    }
    if candidate.red_cards > 0 || candidate.yellow_cards >= 5 || candidate.fouls_made >= 40 {
        flags.push("discipline".to_owned());
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
        "score_low" => row.score_low,
        "bargain" => row.bargain_score,
        "value_gap" => row.value_gap_pct,
        "fair_price" => row.fair_price,
        "surplus" => row.surplus,
        "production" => row.production_tier.or(row.production_global),
        "production_low" => row.production_low,
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

/// Posterior mean under a Gamma-Poisson (counts) or Beta-Binomial (rates) prior.
/// `exposure` is minutes for counts and attempts for rates; `prior_strength`
/// uses the same unit.
fn shrink(value: f64, peer_mean: f64, exposure: f64, prior_strength: f64) -> f64 {
    if exposure <= 0.0 || prior_strength <= 0.0 {
        return value;
    }
    (value * exposure + peer_mean * prior_strength) / (exposure + prior_strength)
}

fn fallback_prior_strength(is_rate: bool) -> f64 {
    if is_rate { 200.0 } else { 900.0 }
}

/// Method-of-moments prior strength for one metric inside one peer group.
///
/// The within-player variance follows the Poisson (counts) or binomial (rates)
/// assumption; whatever spread remains is between players and sets the prior.
/// Thin groups and degenerate spreads fall back to one season of evidence.
fn prior_strength(pairs: &[(f64, f64)], is_rate: bool) -> f64 {
    const MIN_PLAYERS: usize = 30;
    if pairs.len() < MIN_PLAYERS {
        return fallback_prior_strength(is_rate);
    }
    // Rates arrive in percent; work in fractions so the Beta strength is in
    // attempts.
    let scale = if is_rate { 100.0 } else { 1.0 };
    let total: f64 = pairs.iter().map(|(_, exposure)| *exposure).sum();
    if total <= 0.0 {
        return fallback_prior_strength(is_rate);
    }
    let mean = pairs
        .iter()
        .map(|(value, exposure)| value / scale * exposure)
        .sum::<f64>()
        / total;
    if !mean.is_finite() || mean <= 0.0 {
        return fallback_prior_strength(is_rate);
    }
    let count = pairs.len() as f64;
    let variance = pairs
        .iter()
        .map(|(value, _)| (value / scale - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    let within = pairs
        .iter()
        .map(|(value, exposure)| {
            if is_rate {
                let value = value / scale;
                (value * (1.0 - value)).max(0.0) / exposure
            } else {
                90.0 * mean / exposure
            }
        })
        .sum::<f64>()
        / count;
    let tau2 = variance - within;
    if !tau2.is_finite() || tau2 <= 0.0 {
        return fallback_prior_strength(is_rate);
    }
    let raw = if is_rate {
        mean * (1.0 - mean) / tau2
    } else {
        90.0 * mean / tau2
    };
    // One season is the largest prior a single-season sample can justify; a
    // smaller lower bound keeps a genuinely stable trait from being flattened.
    let (low, high) = if is_rate {
        (10.0, 1000.0)
    } else {
        (90.0, 3600.0)
    };
    if raw.is_finite() {
        raw.clamp(low, high)
    } else {
        fallback_prior_strength(is_rate)
    }
}

fn assign_percentiles(
    candidates: &[StatsMoneyballCandidate],
    adjusted: &[HashMap<Metric, f64>],
    metrics: &HashSet<Metric>,
    basis: &str,
) -> (Vec<HashMap<Metric, f64>>, Vec<usize>) {
    let mut output: Vec<HashMap<Metric, f64>> = vec![HashMap::new(); candidates.len()];
    let mut counts = vec![0usize; candidates.len()];
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
        // A thin tier would produce 0/50/100 percentiles from a handful of
        // players; the caller falls back to the position group instead.
        if basis != "global" && indices.len() < MIN_PEER_SAMPLE {
            continue;
        }
        for index in indices {
            counts[*index] = indices.len();
        }
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
    (output, counts)
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

/// Feature vector for the fair-price model. Order matters; keep it in sync with
/// `price_model::FEATURE_COUNT` and the test in that module.
fn price_features(candidate: &StatsMoneyballCandidate, quality_pct: f64, mode: &str) -> Vec<f64> {
    let full = mode == "full";
    let ca = if full { f64::from(candidate.ca) } else { 0.0 };
    let pa = if full {
        candidate.pa.map(f64::from).unwrap_or(ca).max(ca)
    } else {
        0.0
    };
    let age = candidate.age.map(|age| (f64::from(age) - 26.0) / 10.0);
    let reputation = f64::from(candidate.club_reputation.unwrap_or(0));
    let reputation_known = f64::from(u8::from(candidate.club_reputation.is_some()));
    let contract = candidate
        .contract_years
        .map(|years| years.clamp(0.0, 5.0) / 5.0);
    let contract_known = f64::from(u8::from(contract.is_some()));
    let league_known = f64::from(u8::from(candidate.league_name.is_some()));
    let group = candidate.position_group();
    let quality = (quality_pct / 100.0).clamp(0.0, 1.0);
    let ca = (ca / 200.0).clamp(0.0, 1.0);
    let mut features = vec![
        1.0,
        quality,
        quality * quality,
        ca,
        ca * ca,
        (pa / 200.0).clamp(0.0, 1.0),
        age.unwrap_or(0.0),
        age.map(|value| value * value).unwrap_or(0.0),
        (f64::from(candidate.minutes) / 90.0).ln_1p() / 4.0,
        (reputation / 1000.0).ln_1p(),
        reputation_known,
        contract.unwrap_or(0.0),
        contract_known,
        league_known,
    ];
    for name in crate::moneyball::POSITION_GROUPS {
        features.push(f64::from(u8::from(group == name)));
    }
    debug_assert_eq!(features.len(), crate::price_model::FEATURE_COUNT);
    features
}

/// Parts of the bargain composite that come from the player context rather than
/// the price model: youth, contract leverage and minutes load.
#[derive(Debug, Clone, Copy, Default)]
struct BargainParts {
    youth: f64,
    contract: f64,
    minutes_share: f64,
}

fn bargain_parts(candidate: &StatsMoneyballCandidate) -> BargainParts {
    BargainParts {
        // Under 20 is full bonus, 27 and older is none.
        youth: f64::from(candidate.age.map(|age| (27 - age).clamp(0, 7)).unwrap_or(0)) / 7.0,
        // A player in the last year of his contract is cheaper to sign.
        contract: candidate
            .contract_years
            .map(|years| (1.0 - years / 5.0).clamp(0.0, 1.0))
            .unwrap_or(0.0),
        // 30 full matches is a regular starter.
        minutes_share: (f64::from(candidate.minutes) / 2700.0).clamp(0.0, 1.0),
    }
}

/// Attach fair prices and the value gap. Rows without a usable fee still get a
/// market price but no gap, because there is nothing to compare it with.
fn attach_fair_prices(
    rows: &mut [StatsMoneyballRow],
    inputs: &[crate::price_model::PriceInput],
    parts: &[BargainParts],
) {
    debug_assert_eq!(rows.len(), inputs.len());
    debug_assert_eq!(rows.len(), parts.len());
    let Some(model) = crate::price_model::PriceModel::fit(inputs) else {
        for row in rows.iter_mut() {
            row.flags.push("no_price_baseline".to_owned());
        }
        return;
    };
    // The model cannot support prices outside the observed market. Cap the fair
    // price at the position group's 99th percentile fee; without this a linear
    // extrapolation can value a 35M striker above 100M.
    let mut caps: HashMap<String, f64> = HashMap::new();
    for group in crate::moneyball::POSITION_GROUPS {
        let mut fees: Vec<f64> = rows
            .iter()
            .filter(|row| row.position_group == group)
            .filter_map(|row| row.fee.filter(|fee| *fee > 0).map(|fee| fee as f64))
            .collect();
        if fees.len() < 20 {
            continue;
        }
        fees.sort_by(f64::total_cmp);
        let index = ((fees.len() - 1) as f64 * 0.99).round() as usize;
        caps.insert(group.to_owned(), fees[index]);
    }

    for (row, input) in rows.iter_mut().zip(inputs) {
        let fair = if input.fee.is_some() {
            model.predict_oof(input.uid, &input.features)
        } else {
            model.predict_market(&input.features)
        };
        let fair = caps
            .get(&row.position_group)
            .map_or(fair, |cap| fair.min(*cap));
        let fair = (fair > 0.0).then_some(fair);
        row.fair_price = fair;
        row.value_gap_pct = match (fair, row.fee, row.fee_known) {
            (Some(fair), Some(fee), true) if fee > 0 => {
                let fee = fee as f64;
                // Symmetric relative difference: bounded and readable even when
                // a player costs several times the model price.
                Some((fair - fee) / ((fair + fee) / 2.0) * 100.0)
            }
            _ => None,
        };
    }

    // Gap percentiles are computed inside the position group, over every scored
    // row with a fee, before any request filter.
    let mut gap_pct = vec![None; rows.len()];
    for group in crate::moneyball::POSITION_GROUPS {
        let indices: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.position_group == group).then_some(index))
            .collect();
        if indices.is_empty() {
            continue;
        }
        let values: Vec<Option<f64>> = indices
            .iter()
            .map(|index| rows[*index].value_gap_pct)
            .collect();
        // One priced player cannot define a percentile; treat him as neutral.
        if values.iter().filter(|value| value.is_some()).count() < 2 {
            for index in &indices {
                gap_pct[*index] = Some(0.5);
            }
            continue;
        }
        let ranks = crate::moneyball::percentiles(&values);
        for (offset, index) in indices.iter().enumerate() {
            gap_pct[*index] = ranks[offset];
        }
    }
    for (index, row) in rows.iter_mut().enumerate() {
        // The lower bound keeps a 300-minute hot streak from winning the list.
        if let (Some(rank), Some(production)) = (gap_pct[index], row.production_low) {
            let parts = parts[index];
            row.bargain_score = Some(
                0.45 * production
                    + 0.30 * rank * 100.0
                    + 10.0 * parts.youth
                    + 10.0 * parts.contract
                    + 5.0 * parts.minutes_share,
            );
        }
    }
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
    mode: &str,
) -> Result<Vec<StatsMoneyballRow>> {
    let model = normalized_model(Some(model))?;
    let mode = normalized_mode(Some(mode))?;

    candidates.retain(|candidate| {
        candidate.minutes >= SCORE_FLOOR_MINUTES
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
    for group in crate::moneyball::POSITION_GROUPS {
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
        // One prior strength per metric and group, fitted on the whole group so
        // a thin reputation tier cannot pick its own prior.
        let priors: HashMap<Metric, f64> = metrics
            .iter()
            .map(|(metric, _)| {
                let pairs: Vec<(f64, f64)> = indices
                    .iter()
                    .filter_map(|index| {
                        let candidate = &candidates[*index];
                        let value = metric.value(candidate)?;
                        let exposure = metric.exposure(candidate)?;
                        (exposure > 0.0).then_some((value, exposure))
                    })
                    .collect();
                (*metric, prior_strength(&pairs, metric.is_rate()))
            })
            .collect();
        for index in indices {
            let candidate = &candidates[index];
            let tier_key = candidate.reputation_tier();
            for (metric, _) in metrics {
                let Some(value) = metric.value(candidate) else {
                    continue;
                };
                let Some(exposure) = metric.exposure(candidate) else {
                    continue;
                };
                let Some(prior) = priors.get(metric).copied() else {
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
                adjusted[index].insert(*metric, shrink(value, peer, exposure, prior));
            }
        }
    }

    let (pct_global, group_counts) =
        assign_percentiles(&candidates, &adjusted, &all_metrics, "global");
    let (mut pct_tier, tier_counts) =
        assign_percentiles(&candidates, &adjusted, &all_metrics, "tier");
    let mut peer_count = vec![0usize; candidates.len()];
    for index in 0..candidates.len() {
        peer_count[index] = if tier_counts[index] >= MIN_PEER_SAMPLE {
            tier_counts[index]
        } else {
            group_counts[index]
        };
        // Thin tiers keep the group percentile rather than dropping the row.
        for metric in &all_metrics {
            if let Some(value) = pct_global[index].get(metric).copied() {
                pct_tier[index].entry(*metric).or_insert(value);
            }
        }
    }

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
            if basis != "global" && indices.len() < MIN_PEER_SAMPLE {
                continue;
            }
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
    let mut price_inputs: Vec<crate::price_model::PriceInput> =
        Vec::with_capacity(candidates.len());
    let mut bargain_inputs: Vec<BargainParts> = Vec::with_capacity(candidates.len());
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

        // Returns (production percentile, 80% band half-width, drivers).
        let production = |pct: &[HashMap<Metric, f64>],
                          with_drivers: bool|
         -> Option<(f64, f64, Vec<StatsDriver>)> {
            let total_weight: f64 = metrics.iter().map(|(_, weight)| *weight).sum();
            if total_weight <= 0.0 {
                return None;
            }
            let mut available_weight = 0.0;
            let mut weighted = 0.0;
            let mut variance = 0.0;
            let mut drivers = Vec::new();
            for (metric, weight) in metrics {
                if let Some(value) = pct[index].get(metric) {
                    available_weight += weight;
                    weighted += value * weight;
                    // Percentile sampling error from the metric's own evidence.
                    if let Some(evidence) = metric.evidence(&candidate) {
                        let p = (value / 100.0).clamp(0.0, 1.0);
                        let sd = (p * (1.0 - p) / evidence.max(1.0)).sqrt() * 100.0;
                        variance += (weight * sd).powi(2);
                    }
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
            let band = UNCERTAINTY_Z * variance.sqrt() / available_weight;
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
            Some((production, band, drivers))
        };

        let Some((production_global, _, _)) = production(&pct_global, false) else {
            continue;
        };
        let Some((production_tier, band, drivers)) = production(&pct_tier, true) else {
            continue;
        };
        let cost_pct_global = cost_pct_global[index];
        // A thin cost bucket falls back to the position group, matching the
        // quality percentiles.
        let Some(cost_pct_tier) = cost_pct_tier[index].or(cost_pct_global) else {
            continue;
        };
        let production_low = (production_tier - band).clamp(0.0, 100.0);
        let production_high = (production_tier + band).clamp(0.0, 100.0);
        let surplus = Some(production_tier - cost_pct_tier);
        // `score` is quality only. Price lives in `fair_price` / `value_gap_pct`
        // and the explicit `bargain_score` composite.
        let score = Some(production_tier);
        let score_low = Some(production_low);
        // Confidence is how much of the 0-100 production scale the 80% band
        // still leaves open, so thin samples report low values instead of a
        // hand-tuned checklist that always reaches 1.0.
        let confidence = (1.0 - 2.0 * band / 100.0).clamp(0.0, 1.0);
        let flags = flags(&candidate, fee_known, wage_known, confidence);

        let metric_percentiles: HashMap<String, f64> = metrics
            .iter()
            .filter_map(|(metric, _)| {
                pct_tier[index]
                    .get(metric)
                    .map(|value| (metric.key().to_owned(), *value))
            })
            .collect();
        let save_percent = candidate.save_percent();
        let feature_vector = price_features(&candidate, production_tier, mode);
        let bargain_part = bargain_parts(&candidate);
        let price_uid = candidate.uid;
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
            non_penalty_expected_goals: candidate.non_penalty_expected_goals,
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
            production_low: Some(production_low),
            production_high: Some(production_high),
            cost_pct_global,
            cost_pct_tier: Some(cost_pct_tier),
            surplus,
            score,
            score_low,
            fair_price: None,
            value_gap_pct: None,
            bargain_score: None,
            peer_count: peer_count[index] as u32,
            confidence,
            flags,
            drivers,
            transfer_listed: candidate.transfer_listed,
            loan_listed: candidate.loan_listed,
            injury_status: candidate.injury_status,
            recent_injury: candidate.recent_injury,
            injury_recent: candidate.injury_recent,
            stats_from_other_club: candidate.stats_from_other_club,
            metric_percentiles,
        });
        price_inputs.push(crate::price_model::PriceInput {
            uid: price_uid,
            fee: fee.filter(|fee| *fee > 0),
            features: feature_vector,
        });
        bargain_inputs.push(bargain_part);
    }

    attach_fair_prices(&mut rows, &price_inputs, &bargain_inputs);
    Ok(rows)
}

/// Apply request filters and sorting without pagination. Exports need every
/// row; the paged response is built from this result.
pub fn filtered_stats_rows(
    rows: &[StatsMoneyballRow],
    request: &StatsMoneyballRequest,
) -> Result<(Vec<StatsMoneyballRow>, StatsMoneyballMeta)> {
    let model = normalized_model(request.model.as_deref())?;
    let window = normalized_window(request.window.as_deref())?;
    let mode = normalized_mode(request.mode.as_deref())?;
    let min_minutes = request.min_minutes.unwrap_or(900);
    let position_filter = request
        .position_group
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());
    if let Some(group) = position_filter.as_deref()
        && !crate::moneyball::POSITION_GROUPS.contains(&group)
        && !matches!(group, "MID" | "AM")
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
            if row.minutes < min_minutes {
                return false;
            }
            if !include_goalkeepers && row.position_group == "GK" {
                return false;
            }
            if !request.include_unknown_fee && !row.fee_known {
                return false;
            }
            if let Some(group) = position_filter.as_deref()
                && !crate::moneyball::position_filter_matches(group, &row.position_group)
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
            if request.exclude_injury_history && row.injury_recent {
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

    let priced = rows.iter().filter(|row| row.fair_price.is_some()).count();
    let meta = StatsMoneyballMeta {
        model: model_name(model).to_owned(),
        scored: rows.len(),
        filtered: filtered.len(),
        min_minutes,
        score_floor_minutes: SCORE_FLOOR_MINUTES,
        cost_horizon_years: COST_HORIZON_YEARS,
        leagues,
        window: window_label(window).to_owned(),
        mode: mode.to_owned(),
        price_model: (priced > 0).then(|| crate::price_model::PRICE_MODEL.to_owned()),
        priced,
    };
    Ok((filtered.iter().map(|row| (*row).clone()).collect(), meta))
}

/// Apply request filters, sorting and pagination to already-scored rows.
pub fn apply_stats_request(
    rows: &[StatsMoneyballRow],
    request: &StatsMoneyballRequest,
) -> Result<StatsMoneyballPage> {
    let (filtered, meta) = filtered_stats_rows(rows, request)?;
    let page = request.page.unwrap_or(0);
    let page_size = request.page_size.unwrap_or(100).clamp(1, 500);
    let start = (page as usize).saturating_mul(page_size as usize);
    let page_rows = if start >= filtered.len() {
        Vec::new()
    } else {
        filtered[start..(start + page_size as usize).min(filtered.len())].to_vec()
    };
    Ok(StatsMoneyballPage {
        total: filtered.len(),
        page,
        page_size,
        rows: page_rows,
        meta,
    })
}

/// Score candidates and return one filtered page. Percentiles are computed on
/// the fixed `SCORE_FLOOR_MINUTES` set, so request filters (including
/// `min_minutes`) never move the ranking.
pub fn score_stats_moneyball(
    candidates: Vec<StatsMoneyballCandidate>,
    request: &StatsMoneyballRequest,
) -> Result<StatsMoneyballPage> {
    let model = normalized_model(request.model.as_deref())?;
    let mode = normalized_mode(request.mode.as_deref())?;
    let rows = score_stats_candidates(candidates, model, mode)?;
    apply_stats_request(&rows, request)
}

/// The request for the squad needs view.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SquadNeedsRequest {
    pub save_id: i64,
    pub club_uid: u32,
    pub model: Option<String>,
    pub window: Option<String>,
    pub min_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadUpgrade {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_name: Option<String>,
    pub minutes: u32,
    pub quality: Option<f64>,
    pub quality_gain: Option<f64>,
    pub fee: Option<u64>,
    pub fair_price: Option<f64>,
    pub value_gap_pct: Option<f64>,
    pub bargain_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadNeed {
    pub position_group: String,
    pub best_quality: Option<f64>,
    pub best_uid: Option<u32>,
    pub best_name: Option<String>,
    pub upgrades: Vec<SquadUpgrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadNeeds {
    pub club_uid: u32,
    pub club_name: Option<String>,
    pub needs: Vec<SquadNeed>,
}

/// Weakest position groups in a squad, with affordable upgrades from other clubs.
/// Pure so the ranking can be tested without SQLite.
pub fn squad_needs_from_rows(
    rows: &[StatsMoneyballRow],
    club_uid: u32,
    club_name: Option<String>,
    min_minutes: u32,
) -> SquadNeeds {
    let mut needs = Vec::new();
    for group in crate::moneyball::POSITION_GROUPS {
        let best = rows
            .iter()
            .filter(|row| {
                row.position_group == group
                    && row.club_uid == Some(club_uid)
                    && row.minutes >= min_minutes
                    // A loanee's stats belong to his loan club; they do not define
                    // the parent club's quality in this position.
                    && !row.stats_from_other_club
            })
            .filter_map(|row| row.score.map(|score| (score, row)))
            .max_by(|left, right| left.0.total_cmp(&right.0));
        let Some((best_quality, best_row)) = best else {
            continue;
        };
        let tier = reputation_tier(best_row.club_reputation);
        let mut upgrades: Vec<&StatsMoneyballRow> = rows
            .iter()
            .filter(|row| {
                row.position_group == group
                    && row.club_uid.is_some()
                    && row.club_uid != Some(club_uid)
                    && row.minutes >= min_minutes
                    && row.fee_known
                    && !row.stats_from_other_club
                    // Percentiles are only comparable inside one reputation tier.
                    && reputation_tier(row.club_reputation) == tier
                    && row.score.is_some_and(|score| score > best_quality)
            })
            .collect();
        upgrades.sort_by(|left, right| {
            right
                .bargain_score
                .unwrap_or(0.0)
                .total_cmp(&left.bargain_score.unwrap_or(0.0))
                .then_with(|| {
                    right
                        .score
                        .unwrap_or(0.0)
                        .total_cmp(&left.score.unwrap_or(0.0))
                })
                .then_with(|| left.uid.cmp(&right.uid))
        });
        let upgrades = upgrades
            .into_iter()
            .take(3)
            .map(|row| SquadUpgrade {
                uid: row.uid,
                name: row.name.clone(),
                age: row.age,
                club_name: row.club_name.clone(),
                minutes: row.minutes,
                quality: row.score,
                quality_gain: row.score.map(|score| score - best_quality),
                fee: row.fee,
                fair_price: row.fair_price,
                value_gap_pct: row.value_gap_pct,
                bargain_score: row.bargain_score,
            })
            .collect();
        needs.push(SquadNeed {
            position_group: group.to_owned(),
            best_quality: Some(best_quality),
            best_uid: Some(best_row.uid),
            best_name: best_row.name.clone(),
            upgrades,
        });
    }
    needs.sort_by(|left, right| {
        left.best_quality
            .unwrap_or(-1.0)
            .total_cmp(&right.best_quality.unwrap_or(-1.0))
            .then_with(|| left.position_group.cmp(&right.position_group))
    });
    SquadNeeds {
        club_uid,
        club_name,
        needs,
    }
}

/// One hit from the style-similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarStatsPlayer {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_name: Option<String>,
    pub position_group: String,
    pub minutes: u32,
    pub quality: Option<f64>,
    pub fee: Option<u64>,
    pub fair_price: Option<f64>,
    pub value_gap_pct: Option<f64>,
    /// Weighted Euclidean distance in percentile space; 0 is identical style.
    pub distance: f64,
}

/// Metric keys and weights for a position template. Used by the similarity
/// search and by callers that want to explain which axes matter.
pub fn metric_weights(model: &str, group: &str) -> Vec<(&'static str, f64)> {
    let model = if model == "advanced" {
        "advanced"
    } else {
        "core"
    };
    model_metrics(model, group)
        .iter()
        .map(|(metric, weight)| (metric.key(), *weight))
        .collect()
}

/// Find players with a similar metric profile inside the same position group.
/// The distance is weighted by the group's template, so a striker is compared
/// on attacking axes rather than on clearances.
pub fn similar_stats_players(
    rows: &[StatsMoneyballRow],
    model: &str,
    uid: u32,
    limit: usize,
) -> Result<Vec<SimilarStatsPlayer>> {
    let model = normalized_model(Some(model))?;
    let Some(target) = rows.iter().find(|row| row.uid == uid) else {
        return Ok(Vec::new());
    };
    let weights = metric_weights(model, &target.position_group);
    let total_weight: f64 = weights.iter().map(|(_, weight)| weight).sum();
    if total_weight <= 0.0 {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    for row in rows {
        if row.uid == uid || row.position_group != target.position_group {
            continue;
        }
        let mut distance = 0.0;
        let mut shared_weight = 0.0;
        for (key, weight) in &weights {
            let (Some(left), Some(right)) = (
                target.metric_percentiles.get(*key),
                row.metric_percentiles.get(*key),
            ) else {
                continue;
            };
            distance += weight * (left - right).powi(2);
            shared_weight += weight;
        }
        // A row scored by the other model may not share enough axes.
        if shared_weight / total_weight < 0.6 {
            continue;
        }
        results.push(SimilarStatsPlayer {
            uid: row.uid,
            name: row.name.clone(),
            age: row.age,
            club_name: row.club_name.clone(),
            position_group: row.position_group.clone(),
            minutes: row.minutes,
            quality: row.score,
            fee: row.fee,
            fair_price: row.fair_price,
            value_gap_pct: row.value_gap_pct,
            distance: (distance / shared_weight).sqrt(),
        });
    }
    results.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| left.uid.cmp(&right.uid))
    });
    results.truncate(limit.clamp(1, 50));
    Ok(results)
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
            contract_years: Some(3.0),
            club_reputation: Some(6000),
            transfer_listed: false,
            loan_listed: false,
            injury_status: "fit".to_owned(),
            recent_injury: None,
            injury_recent: false,
            minutes,
            starts: minutes / 90,
            substitute_appearances: 0,
            average_rating: Some(7.0),
            goals: 10,
            assists: 5,
            expected_goals: xg * f64::from(minutes) / 90.0,
            non_penalty_expected_goals: xg * f64::from(minutes) / 90.0,
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
            key_tackles: Some(2),
            possession_won: Some(30),
            clear_cut_chances_created: Some(3),
            key_headers: Some(1),
            distance_km: Some(200.0),
            mistakes_leading_to_goal: 0,
            fouls_made: 10,
            yellow_cards: 2,
            red_cards: 0,
            saves_held: None,
            saves_parried: None,
            saves_tipped: None,
            shots_on_target_faced: None,
            expected_goals_prevented: None,
            advanced_available,
            current_club: true,
            stats_from_other_club: false,
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
        history.injury_recent = true;
        let old_injury = {
            let mut player = candidate(4, 0.8, 0.4, 1800, 1_000, 100, true);
            player.recent_injury = Some("2022-08-01".to_owned());
            player
        };
        let fit = candidate(3, 0.8, 0.4, 1800, 1_000, 100, true);

        let mut request = StatsMoneyballRequest {
            model: Some("core".to_owned()),
            min_minutes: Some(900),
            exclude_injured: true,
            ..StatsMoneyballRequest::default()
        };
        let page = score_stats_moneyball(
            vec![
                injured.clone(),
                history.clone(),
                old_injury.clone(),
                fit.clone(),
            ],
            &request,
        )
        .expect("score");
        assert_eq!(page.total, 3);
        assert!(page.rows.iter().all(|row| row.uid != 1));

        request.exclude_injury_history = true;
        let page = score_stats_moneyball(vec![injured, history, old_injury, fit], &request)
            .expect("score");
        assert_eq!(page.total, 2);
        assert!(page.rows.iter().all(|row| row.uid != 2));
        assert!(page.rows.iter().any(|row| row.uid == 3));
        // A career injury outside the 24-month window is not excluded.
        assert!(page.rows.iter().any(|row| row.uid == 4));
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

    #[test]
    fn scoring_floor_keeps_percentiles_stable_when_min_minutes_changes() {
        let candidates = vec![
            candidate(1, 0.9, 0.4, 1800, 1_000, 100, true),
            candidate(2, 0.5, 0.3, 1800, 1_000, 100, true),
            candidate(3, 0.3, 0.2, 1800, 1_000, 100, true),
            candidate(4, 0.7, 0.1, 400, 1_000, 100, true),
        ];
        let scored = score_stats_candidates(candidates.clone(), "core", "full").expect("score");
        let low = apply_stats_request(
            &scored,
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(300),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("low threshold");
        let high = apply_stats_request(
            &scored,
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("high threshold");
        assert_eq!(low.total, 4);
        assert_eq!(high.total, 3);
        for row in &high.rows {
            let same = low
                .rows
                .iter()
                .find(|other| other.uid == row.uid)
                .expect("row");
            assert_eq!(row.production_tier, same.production_tier);
            assert_eq!(row.peer_count, same.peer_count);
        }
    }

    #[test]
    fn thin_reputation_tiers_fall_back_to_the_position_group() {
        let mut candidates: Vec<StatsMoneyballCandidate> = (0..10)
            .map(|index| {
                let mut player = candidate(
                    index + 1,
                    0.5 + index as f64 * 0.01,
                    0.2,
                    1800,
                    1_000,
                    100,
                    true,
                );
                player.club_reputation = Some(7000);
                player
            })
            .collect();
        candidates.extend((0..40).map(|index| {
            let mut player = candidate(
                index + 100,
                0.3 + index as f64 * 0.005,
                0.2,
                1800,
                1_000,
                100,
                true,
            );
            player.club_reputation = Some(1000);
            player
        }));
        let rows = score_stats_candidates(candidates, "core", "full").expect("score");
        assert_eq!(rows.len(), 50);
        // The 10-player tier is too thin and is compared with the whole group.
        assert!(
            rows.iter()
                .filter(|row| row.club_reputation == Some(7000))
                .all(|row| row.peer_count == 50)
        );
        // The 40-player tier keeps its own bucket.
        assert!(
            rows.iter()
                .filter(|row| row.club_reputation == Some(1000))
                .all(|row| row.peer_count == 40)
        );
    }

    #[test]
    fn npxg_drives_the_striker_model() {
        let mut open_play = candidate(1, 0.6, 0.3, 1800, 1_000, 100, true);
        // Same total xG, but more of it from open play.
        open_play.non_penalty_expected_goals = 0.8 * 1800.0 / 90.0;
        let mut penalties = candidate(2, 0.6, 0.3, 1800, 1_000, 100, true);
        penalties.non_penalty_expected_goals = 0.3 * 1800.0 / 90.0;
        let page = score_stats_moneyball(
            vec![open_play, penalties],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.rows[0].uid, 1);
        assert!(page.rows[0].production_tier > page.rows[1].production_tier);
    }

    #[test]
    fn confidence_reflects_the_evidence_behind_a_percentile() {
        let mut small = candidate(1, 0.7, 0.3, 300, 1_000, 100, true);
        small.starts = 3;
        let mut large = candidate(2, 0.7, 0.3, 2700, 1_000, 100, true);
        large.starts = 30;
        let page = score_stats_moneyball(
            vec![small, large],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(300),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        let small = page.rows.iter().find(|row| row.uid == 1).expect("small");
        let large = page.rows.iter().find(|row| row.uid == 2).expect("large");
        assert!(small.confidence < large.confidence);
        let small_band = small.production_high.unwrap() - small.production_low.unwrap();
        let large_band = large.production_high.unwrap() - large.production_low.unwrap();
        assert!(small_band > large_band);
        assert!(small.production_low.unwrap() <= small.production_tier.unwrap());
        assert!(small.score_low.unwrap() <= small.score.unwrap());
    }

    #[test]
    fn prior_strength_grows_when_players_cluster() {
        let tight: Vec<(f64, f64)> = (0..60)
            .map(|index| (70.0 + (index % 3) as f64, 400.0))
            .collect();
        let wide: Vec<(f64, f64)> = (0..60).map(|index| (30.0 + index as f64, 400.0)).collect();
        assert!(prior_strength(&tight, true) > prior_strength(&wide, true));
        assert_eq!(prior_strength(&[], false), 900.0);
        assert_eq!(prior_strength(&[], true), 200.0);
    }

    #[test]
    fn shrink_uses_the_metric_exposure_and_prior() {
        // 200 attempts at 80% with a 200-attempt prior at 60% -> 70%.
        assert!((shrink(80.0, 60.0, 200.0, 200.0) - 70.0).abs() < 1e-9);
        // No prior strength leaves the observed value untouched.
        assert_eq!(shrink(80.0, 60.0, 200.0, 0.0), 80.0);
        assert_eq!(shrink(80.0, 60.0, 0.0, 200.0), 80.0);
    }

    #[test]
    fn fair_price_and_value_gap_are_separate_from_quality() {
        let mut candidates: Vec<StatsMoneyballCandidate> = (0..260u32)
            .map(|index| {
                let npxg = 0.1 + (index % 20) as f64 * 0.04;
                let fee = (npxg * 20_000_000.0) as u32;
                let mut player = candidate(index + 1, npxg, 0.2, 1800, fee, 100, true);
                // Keep the contract and age informative but identical.
                player.contract_years = Some(3.0);
                player
            })
            .collect();
        // A high-quality player listed far below the market for that profile.
        candidates[5].transfer_value = Some(1);
        let page = score_stats_moneyball(
            candidates,
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                position_group: Some("ST".to_owned()),
                page_size: Some(300),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.total, 260);
        assert_eq!(
            page.meta.price_model.as_deref(),
            Some(crate::price_model::PRICE_MODEL)
        );
        assert_eq!(page.meta.priced, 260);
        let bargain = page.rows.iter().find(|row| row.uid == 6).expect("row");
        assert!(bargain.fair_price.unwrap() > 1.0);
        assert!(bargain.value_gap_pct.unwrap() > 50.0);
        assert!(bargain.bargain_score.unwrap() > 0.0);
        // `score` is quality only: it equals the production percentile.
        assert_eq!(bargain.score, bargain.production_tier);
        assert_eq!(bargain.score_low, bargain.production_low);
        for row in &page.rows {
            assert!(row.fair_price.unwrap() > 0.0);
            assert!(row.value_gap_pct.is_some());
            let score = row.bargain_score.unwrap();
            assert!((0.0..=100.0).contains(&score));
        }
    }

    #[test]
    fn free_and_unknown_fees_get_a_price_but_no_gap() {
        let mut candidates: Vec<StatsMoneyballCandidate> = (0..240u32)
            .map(|index| {
                let npxg = 0.1 + (index % 20) as f64 * 0.04;
                let fee = (npxg * 20_000_000.0) as u32;
                candidate(index + 1, npxg, 0.2, 1800, fee, 100, true)
            })
            .collect();
        let mut free = candidate(500, 0.5, 0.2, 1800, 0, 100, true);
        free.club_uid = None;
        free.transfer_value = Some(0);
        free.transfer_value_state = "zero".to_owned();
        let mut unknown = candidate(501, 0.5, 0.2, 1800, 0, 100, true);
        unknown.transfer_value = Some(9_999_999);
        unknown.transfer_value_state = "unset".to_owned();
        candidates.push(free);
        candidates.push(unknown);
        let page = score_stats_moneyball(
            candidates,
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                position_group: Some("ST".to_owned()),
                include_unknown_fee: true,
                page_size: Some(300),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.total, 242);
        for uid in [500, 501] {
            let row = page.rows.iter().find(|row| row.uid == uid).expect("row");
            assert!(row.fair_price.unwrap() > 0.0, "uid={uid}");
            assert!(row.value_gap_pct.is_none(), "uid={uid}");
            assert!(row.bargain_score.is_none(), "uid={uid}");
        }
    }

    #[test]
    fn a_small_market_leaves_the_price_model_off() {
        let page = score_stats_moneyball(
            vec![
                candidate(1, 0.8, 0.4, 1800, 1_000, 100, true),
                candidate(2, 0.5, 0.2, 1800, 2_000, 100, true),
            ],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.total, 2);
        assert!(page.meta.price_model.is_none());
        assert_eq!(page.meta.priced, 0);
        for row in &page.rows {
            assert!(row.fair_price.is_none());
            assert!(row.value_gap_pct.is_none());
            assert!(row.bargain_score.is_none());
            assert!(row.flags.iter().any(|flag| flag == "no_price_baseline"));
        }
    }

    #[test]
    fn normalizes_the_statistics_window() {
        assert_eq!(normalized_window(None).expect("default"), "overall");
        assert_eq!(
            normalized_window(Some("overall")).expect("overall"),
            "overall"
        );
        assert_eq!(normalized_window(Some("league")).expect("league"), "league");
        assert_eq!(
            normalized_window(Some("calendar")).expect("calendar"),
            "calendar_year_overall"
        );
        assert!(normalized_window(Some("cup")).is_err());
    }

    #[test]
    fn bargain_parts_reward_youth_short_contracts_and_minutes() {
        let mut young = candidate(1, 0.5, 0.2, 2700, 1_000, 100, true);
        young.age = Some(19);
        young.contract_years = Some(1.0);
        let mut old = candidate(2, 0.5, 0.2, 900, 1_000, 100, true);
        old.age = Some(31);
        old.contract_years = Some(5.0);
        let a = bargain_parts(&young);
        let b = bargain_parts(&old);
        assert!(a.youth > b.youth);
        assert!(a.contract > b.contract);
        assert!(a.minutes_share > b.minutes_share);

        let mut unknown = young.clone();
        unknown.contract_years = None;
        assert_eq!(bargain_parts(&unknown).contract, 0.0);
    }

    #[test]
    fn legacy_mid_filter_still_matches_split_groups() {
        let mut midfielder = candidate(1, 0.5, 0.3, 1800, 1_000, 100, true);
        midfielder.positions_text = Some("MC".to_owned());
        let mut striker = candidate(2, 0.5, 0.3, 1800, 1_000, 100, true);
        striker.positions_text = Some("STC".to_owned());
        let page = score_stats_moneyball(
            vec![midfielder, striker],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                position_group: Some("MID".to_owned()),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].position_group, "MC");
    }

    #[test]
    fn every_position_template_sums_to_one() {
        for group in crate::moneyball::POSITION_GROUPS {
            let core: f64 = core_metrics(group).iter().map(|(_, weight)| weight).sum();
            let advanced: f64 = advanced_metrics(group)
                .iter()
                .map(|(_, weight)| weight)
                .sum();
            assert!((core - 1.0).abs() < 1e-9, "{group} core sums to {core}");
            assert!(
                (advanced - 1.0).abs() < 1e-9,
                "{group} advanced sums to {advanced}"
            );
            assert!(!core_metrics(group).is_empty(), "{group} core empty");
            assert!(
                !advanced_metrics(group).is_empty(),
                "{group} advanced empty"
            );
        }
    }

    #[test]
    fn advanced_metrics_use_the_new_possession_fields() {
        let mut strong = candidate(1, 0.4, 0.2, 1800, 1_000, 100, true);
        strong.positions_text = Some("MC".to_owned());
        strong.possession_won = Some(80);
        let mut weak = candidate(2, 0.4, 0.2, 1800, 1_000, 100, true);
        weak.positions_text = Some("MC".to_owned());
        weak.possession_won = Some(10);
        let page = score_stats_moneyball(
            vec![strong, weak],
            &StatsMoneyballRequest {
                model: Some("advanced".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        assert_eq!(page.total, 2);
        assert_eq!(page.rows[0].uid, 1);
        assert!(page.rows[0].production_tier > page.rows[1].production_tier);
    }

    #[test]
    fn exports_scored_rows_as_csv_and_json() {
        let page = score_stats_moneyball(
            vec![
                candidate(1, 0.8, 0.4, 1800, 1_000, 100, true),
                candidate(2, 0.5, 0.2, 1800, 2_000, 100, true),
            ],
            &StatsMoneyballRequest {
                model: Some("core".to_owned()),
                min_minutes: Some(900),
                ..StatsMoneyballRequest::default()
            },
        )
        .expect("score");
        let directory = tempfile::tempdir().expect("tempdir");
        for format in ["csv", "json"] {
            let path = directory.path().join(format!("stats.{format}"));
            let report =
                crate::export::write_stats_export(&path, format, &page.rows).expect("export");
            assert_eq!(report.rows, 2);
            let text = std::fs::read_to_string(&path).expect("file");
            if format == "csv" {
                let mut reader = csv::Reader::from_path(&path).expect("reader");
                let header_len = reader.headers().expect("header").len();
                assert!(header_len > 40);
                for record in reader.records() {
                    assert_eq!(record.expect("record").len(), header_len);
                }
            } else {
                let value: serde_json::Value = serde_json::from_str(&text).expect("json");
                assert_eq!(value.as_array().expect("array").len(), 2);
                assert!(value[0].get("fair_price").is_some());
            }
        }
    }

    #[test]
    fn similar_players_follow_the_metric_profile_within_the_group() {
        let close = candidate(1, 0.4, 0.5, 1800, 1_000, 100, true);
        let mut twin = candidate(2, 0.4, 0.5, 1800, 2_000, 100, true);
        twin.positions_text = Some("MC".to_owned());
        let mut far = candidate(3, 0.4, 0.05, 1800, 1_000, 100, true);
        far.positions_text = Some("MC".to_owned());
        let mut striker = candidate(4, 0.9, 0.1, 1800, 1_000, 100, true);
        striker.positions_text = Some("STC".to_owned());
        let rows = score_stats_candidates(
            vec![
                {
                    let mut target = close;
                    target.positions_text = Some("MC".to_owned());
                    target
                },
                twin,
                far,
                striker,
            ],
            "core",
            "full",
        )
        .expect("score");
        let hits = similar_stats_players(&rows, "core", 1, 5).expect("similar");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].uid, 2);
        assert!(hits[0].distance < hits[1].distance);
        assert!(hits.iter().all(|hit| hit.position_group == "MC"));
    }

    #[test]
    fn normalizes_the_data_mode() {
        assert_eq!(normalized_mode(None).expect("default"), "full");
        assert_eq!(
            normalized_mode(Some("fair_play")).expect("fair"),
            "fair_play"
        );
        assert_eq!(
            normalized_mode(Some("moneyball")).expect("moneyball"),
            "moneyball"
        );
        assert!(normalized_mode(Some("cheat")).is_err());
    }

    #[test]
    fn moneyball_mode_removes_ca_and_pa_from_the_price_features() {
        let player = candidate(1, 0.5, 0.2, 1800, 1_000, 100, true);
        let full = price_features(&player, 50.0, "full");
        let fair = price_features(&player, 50.0, "fair_play");
        let moneyball = price_features(&player, 50.0, "moneyball");
        assert!(full[3] > 0.0 && full[4] > 0.0 && full[5] > 0.0);
        assert_eq!(fair[3], 0.0);
        assert_eq!(fair[4], 0.0);
        assert_eq!(fair[5], 0.0);
        assert_eq!(full[1], moneyball[1]);
        assert_eq!(moneyball.len(), crate::price_model::FEATURE_COUNT);
    }

    #[test]
    fn squad_needs_finds_the_weakest_group_and_affordable_upgrades() {
        let mut club_best = candidate(1, 0.4, 0.4, 1800, 1_000, 100, true);
        club_best.club_uid = Some(10);
        club_best.positions_text = Some("MC".to_owned());
        let mut club_weak = candidate(2, 0.4, 0.2, 1800, 1_000, 100, true);
        club_weak.club_uid = Some(10);
        club_weak.positions_text = Some("MC".to_owned());
        let mut upgrade = candidate(3, 0.4, 0.8, 1800, 1_000, 100, true);
        upgrade.club_uid = Some(20);
        upgrade.positions_text = Some("MC".to_owned());
        let mut unknown_fee = candidate(4, 0.4, 0.9, 1800, 9_999, 100, true);
        unknown_fee.club_uid = Some(20);
        unknown_fee.positions_text = Some("MC".to_owned());
        unknown_fee.transfer_value_state = "unset".to_owned();
        let mut worse = candidate(5, 0.4, 0.05, 1800, 1_000, 100, true);
        worse.club_uid = Some(30);
        worse.positions_text = Some("MC".to_owned());
        let rows = score_stats_candidates(
            vec![club_best, club_weak, upgrade, unknown_fee, worse],
            "core",
            "full",
        )
        .expect("score");
        let needs = squad_needs_from_rows(&rows, 10, Some("Club".to_owned()), 900);
        assert_eq!(needs.needs.len(), 1);
        let need = &needs.needs[0];
        assert_eq!(need.position_group, "MC");
        assert!(need.best_quality.is_some());
        assert_eq!(need.upgrades.len(), 1);
        assert_eq!(need.upgrades[0].uid, 3);
        assert!(need.upgrades[0].quality_gain.unwrap() > 0.0);
        assert_eq!(squad_needs_from_rows(&rows, 99, None, 900).needs.len(), 0);
    }

    #[test]
    fn squad_needs_ignore_other_tiers_and_loan_stats() {
        let mut club = candidate(1, 0.4, 0.3, 1800, 1_000, 100, true);
        club.club_uid = Some(10);
        club.positions_text = Some("MC".to_owned());
        let mut same_tier = candidate(2, 0.4, 0.8, 1800, 1_000, 100, true);
        same_tier.club_uid = Some(20);
        same_tier.positions_text = Some("MC".to_owned());
        let mut other_tier = candidate(3, 0.4, 0.9, 1800, 1_000, 100, true);
        other_tier.club_uid = Some(30);
        other_tier.club_reputation = Some(1000);
        other_tier.positions_text = Some("MC".to_owned());
        let mut loanee = candidate(4, 0.4, 0.95, 1800, 1_000, 100, true);
        loanee.club_uid = Some(20);
        loanee.positions_text = Some("MC".to_owned());
        loanee.stats_from_other_club = true;
        let rows =
            score_stats_candidates(vec![club, same_tier, other_tier, loanee], "core", "full")
                .expect("score");
        let needs = squad_needs_from_rows(&rows, 10, None, 900);
        let mc = needs
            .needs
            .iter()
            .find(|need| need.position_group == "MC")
            .expect("MC");
        assert_eq!(mc.upgrades.len(), 1);
        assert_eq!(mc.upgrades[0].uid, 2);
    }
}
