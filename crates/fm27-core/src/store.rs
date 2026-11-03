//! SQLite cache and query layer. The UI never sees the save bytes or the database path.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{Months, NaiveDate, Utc};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::{ATTRIBUTE_KEYS, HIDDEN_ATTRIBUTE_KEYS, PERSONALITY_KEYS, POSITION_KEYS};
use crate::model::{
    Attributes, Club, ClubDetail, ClubLink, Competition, Contract, FinanceMonth,
    GENERATED_UNIQUE_ID_MINIMUM, ParseReport, Personality, Player, Positions, SaveSummary, Stadium,
    Trait, is_generated_unique_id,
};
use crate::parse::ParsedSave;

const SCHEMA: &str = include_str!("../../../shared/schema.sql");

const CLUB_CTE: &str = "WITH stats AS (
        SELECT p.club_uid,
               COUNT(*) AS player_count,
               AVG(p.age) AS average_age,
               AVG(p.ca) AS average_ca,
               SUM(COALESCE(p.transfer_value, 0)) AS total_value,
               SUM(COALESCE(c2.wage, 0)) AS total_wage
        FROM players p
        LEFT JOIN contracts c2 ON c2.save_id = p.save_id AND c2.player_uid = p.uid
        WHERE p.save_id = ? AND p.club_uid IS NOT NULL
        GROUP BY p.club_uid
      ),
      stads AS (
        SELECT owner_club_uid,
               COUNT(*) AS stadium_count,
               MAX(all_seater_capacity) AS stadium_capacity
        FROM stadiums
        WHERE save_id = ? AND owner_club_uid IS NOT NULL
        GROUP BY owner_club_uid
      )";

pub struct Store {
    conn: Connection,
    stats_cache: Mutex<HashMap<StatsCacheKey, Arc<Vec<crate::stats_moneyball::StatsMoneyballRow>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StatsCacheKey {
    save_id: i64,
    model: String,
    min_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AttributeFilter {
    pub key: String,
    pub min: Option<u8>,
    pub max: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SortRule {
    pub by: String,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlayerQuery {
    pub save_id: i64,
    pub search: Option<String>,
    pub uids: Vec<u32>,
    pub club_uids: Vec<u32>,
    pub nation_ids: Vec<u16>,
    pub genders: Vec<u8>,
    /// One of `injured`, `suspended`, `loan`, `free`.
    pub availability: Option<String>,
    /// Any of `sale` (transfer listed) and `loan` (loan listed).
    pub listings: Vec<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub ca_min: Option<u16>,
    pub ca_max: Option<u16>,
    pub pa_min: Option<u16>,
    pub pa_max: Option<u16>,
    /// Minimum (potential - current) ability gap, used by the wonderkid view.
    pub pa_ca_gap_min: Option<u16>,
    pub pa_unknown: Option<bool>,
    /// `true` keeps only system-generated players, `false` keeps only database players.
    pub generated: Option<bool>,
    pub positions: Vec<String>,
    pub position_min: Option<u8>,
    pub attributes: Vec<AttributeFilter>,
    pub value_min: Option<u32>,
    pub value_max: Option<u32>,
    pub wage_min: Option<u32>,
    pub wage_max: Option<u32>,
    pub contract_end_before: Option<String>,
    pub contract_end_after: Option<String>,
    pub on_loan: Option<bool>,
    pub include_hidden: bool,
    pub sort: Vec<SortRule>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRow {
    pub uid: u32,
    pub generated: bool,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub league_competition_id: Option<u32>,
    pub league_name: Option<String>,
    pub nation_id: Option<u16>,
    pub nation_name: Option<String>,
    pub gender: Option<u8>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub pa_range_code: Option<i16>,
    pub positions_text: Option<String>,
    pub positions: Positions,
    pub attributes: Attributes,
    pub personality: Option<Personality>,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: String,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub height_cm: Option<u8>,
    pub left_foot: Option<u8>,
    pub right_foot: Option<u8>,
    pub on_loan: Option<bool>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub traits: Vec<Trait>,
    pub best_role_id: Option<String>,
    pub best_role_name: Option<String>,
    pub best_role_score: Option<u8>,
    pub best_role_position: Option<String>,
    pub injury_status: String,
    pub injury_until: Option<String>,
    pub recent_injury: Option<String>,
    pub suspension_count: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerPage {
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub rows: Vec<PlayerRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClubQuery {
    pub save_id: i64,
    pub search: Option<String>,
    pub nation_ids: Vec<u32>,
    pub genders: Vec<u8>,
    pub reputation_min: Option<u16>,
    pub reputation_max: Option<u16>,
    pub last_position_min: Option<u8>,
    pub last_position_max: Option<u8>,
    pub has_finances: Option<bool>,
    pub balance_min: Option<i64>,
    pub balance_max: Option<i64>,
    pub transfer_budget_min: Option<i64>,
    pub transfer_budget_max: Option<i64>,
    pub wage_budget_min: Option<i64>,
    pub wage_budget_max: Option<i64>,
    pub corporate_facilities_min: Option<u8>,
    pub corporate_facilities_max: Option<u8>,
    pub training_facilities_min: Option<u8>,
    pub training_facilities_max: Option<u8>,
    pub youth_facilities_min: Option<u8>,
    pub youth_facilities_max: Option<u8>,
    pub junior_coaching_min: Option<u8>,
    pub junior_coaching_max: Option<u8>,
    pub youth_recruitment_min: Option<u8>,
    pub youth_recruitment_max: Option<u8>,
    pub player_count_min: Option<i64>,
    pub player_count_max: Option<i64>,
    pub average_ca_min: Option<f64>,
    pub average_ca_max: Option<f64>,
    pub total_value_min: Option<i64>,
    pub total_value_max: Option<i64>,
    pub has_stadium: Option<bool>,
    pub stadium_capacity_min: Option<i64>,
    pub sort: Vec<SortRule>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubRow {
    pub uid: u32,
    pub unique_id: Option<u32>,
    pub name: String,
    pub short_name: String,
    pub nation_id: u32,
    pub nation_name: Option<String>,
    pub fa_nation_id: u32,
    pub fa_nation_name: Option<String>,
    pub city_id: Option<u32>,
    pub gender: Option<u8>,
    pub reputation: Option<u16>,
    pub last_league_position: Option<u8>,
    pub league_competition_id: Option<u32>,
    pub league_name: Option<String>,
    pub parent_club_uid: Option<u32>,
    pub parent_club_name: Option<String>,
    pub corporate_facilities: Option<u8>,
    pub finance_month: Option<String>,
    pub balance: Option<i64>,
    pub transfer_budget_allocated: Option<i64>,
    pub transfer_budget_remaining: Option<i64>,
    pub wage_budget_weekly: Option<i64>,
    pub wage_payroll_weekly: Option<i64>,
    pub net: Option<i64>,
    pub player_count: u64,
    pub average_age: Option<f64>,
    pub average_ca: Option<f64>,
    pub total_value: Option<i64>,
    pub total_wage: Option<i64>,
    pub stadium_count: u64,
    pub stadium_capacity: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubPage {
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub rows: Vec<ClubRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubFilterOptions {
    pub nations: Vec<FilterOption>,
    pub genders: Vec<FilterOption>,
    pub min_reputation: Option<u16>,
    pub max_reputation: Option<u16>,
    pub min_balance: Option<i64>,
    pub max_balance: Option<i64>,
    pub min_transfer_budget: Option<i64>,
    pub max_transfer_budget: Option<i64>,
    pub min_wage_budget: Option<i64>,
    pub max_wage_budget: Option<i64>,
    pub clubs_with_finances: u64,
    pub clubs_with_stadiums: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DistributionBand {
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubOverview {
    pub club_uid: u32,
    pub club_name: String,
    pub players: u64,
    pub avg_ca: f64,
    pub max_ca: u16,
    pub avg_age: f64,
    pub total_value: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisOverview {
    pub total_players: u64,
    pub total_clubs: u64,
    pub average_age: f64,
    pub median_ca: u16,
    pub total_value: u64,
    pub free_agents: u64,
    pub loanees: u64,
    pub expiring_contracts: u64,
    pub game_date: Option<String>,
    pub age_bands: Vec<DistributionBand>,
    pub ca_bands: Vec<DistributionBand>,
    pub contract_years: Vec<DistributionBand>,
    pub top_clubs: Vec<ClubOverview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SquadAnalysis {
    pub club_uid: u32,
    pub club_name: String,
    pub player_count: u64,
    pub average_age: f64,
    pub average_ca: f64,
    pub average_pa: f64,
    pub total_wage: u64,
    pub total_value: u64,
    pub loanees: u64,
    pub position_groups: Vec<DistributionBand>,
    pub age_bands: Vec<DistributionBand>,
    pub contract_years: Vec<DistributionBand>,
    pub players: Vec<PlayerRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveRecord {
    pub id: i64,
    pub path: String,
    pub file_name: String,
    pub game: String,
    pub build: String,
    pub game_date: Option<String>,
    pub parser_version: String,
    pub parsed_at: String,
    pub players: u64,
    pub clubs: u64,
    pub contracts: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotPoint {
    pub game_date: String,
    pub ca: Option<u16>,
    pub pa: Option<i16>,
    pub transfer_value: Option<u32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilterOption {
    pub id: u32,
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilterOptions {
    pub clubs: Vec<FilterOption>,
    pub nations: Vec<FilterOption>,
    pub genders: Vec<FilterOption>,
    pub availability: Vec<FilterOption>,
    pub listing: Vec<FilterOption>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub min_ca: Option<u16>,
    pub max_ca: Option<u16>,
    pub min_pa: Option<u16>,
    pub max_pa: Option<u16>,
    pub min_value: Option<u32>,
    pub max_value: Option<u32>,
    pub min_wage: Option<u32>,
    pub max_wage: Option<u32>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; \
             PRAGMA foreign_keys=ON; PRAGMA wal_autocheckpoint=1000;",
        )?;
        migrate(&conn)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn,
            stats_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn,
            stats_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn find_cached(&self, fingerprint: &str, parser_version: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM saves WHERE fingerprint = ?1 AND parser_version = ?2 LIMIT 1",
                params![fingerprint, parser_version],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn insert_parsed(&mut self, parsed: &ParsedSave) -> Result<i64> {
        if let Ok(mut cache) = self.stats_cache.lock() {
            cache.clear();
        }
        let career_key = career_key(parsed);
        let transaction = self.conn.transaction()?;
        transaction.execute(
            "INSERT INTO saves
             (path, file_name, file_size, mtime_ns, fingerprint, game, build, save_name,
              career_key, game_date, db_version, parser_version, parsed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                parsed.summary.path,
                parsed.summary.file_name,
                parsed.summary.file_size as i64,
                parsed.summary.mtime_ns,
                parsed.summary.fingerprint,
                parsed.summary.game,
                parsed.summary.build,
                parsed.summary.save_name,
                career_key,
                parsed.summary.game_date,
                parsed.summary.db_version,
                crate::layout::PARSER_VERSION,
                Utc::now().to_rfc3339(),
            ],
        )?;
        let save_id = transaction.last_insert_rowid();

        {
            let mut player_stmt = transaction.prepare(
                "INSERT INTO players
                 (save_id, uid, unique_id, name, first_name, last_name, common_name, birth_date,
                  age, nation_id, nation_name, gender, club_uid, club_name, ca, pa, pa_range_code,
                  height_cm, left_foot, right_foot, transfer_value, transfer_value_state,
                  positions_json, positions_text, personality_json, traits_json, attributes_json,
                  raw_attributes_json, best_role_id, best_role_name, best_role_score,
                  best_role_position, injury_status, injury_until, recent_injury,
                  suspension_count, injuries_json, suspensions_json, contract_end, wage, on_loan,
                  transfer_listed, loan_listed, league_competition_id, league_name, data_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
                         ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,
                         ?37,?38,?39,?40,?41,?42,?43,?44,?45,?46)",
            )?;
            let mut snapshot_stmt = transaction.prepare(
                "INSERT INTO player_snapshots
                 (career_key, game_date, player_uid, save_id, ca, pa, transfer_value, club_uid,
                  club_name, wage, contract_end, recorded_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                 ON CONFLICT(career_key, game_date, player_uid) DO UPDATE SET
                   save_id = excluded.save_id,
                   ca = excluded.ca,
                   pa = excluded.pa,
                   transfer_value = excluded.transfer_value,
                   club_uid = excluded.club_uid,
                   club_name = excluded.club_name,
                   wage = excluded.wage,
                   contract_end = excluded.contract_end,
                   recorded_at = excluded.recorded_at",
            )?;
            let recorded_at = Utc::now().to_rfc3339();
            let clock = parsed
                .summary
                .game_date
                .as_deref()
                .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok());
            let mut contract_stmt = transaction.prepare(
                "INSERT INTO contracts
                 (save_id, player_uid, club_uid, club_name, team_id, wage, start_date, end_date,
                  squad_status, contract_type, on_loan, loan_parent_club_uid,
                  loan_parent_club_name, loan_start, loan_end, clauses_json, chain_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            )?;
            for player in &parsed.players {
                let contract = player.contract.as_ref();
                let contract_end = contract.and_then(|value| value.end.clone());
                let wage = contract.and_then(|value| value.wage);
                let on_loan = contract.and_then(|value| value.on_loan);
                let injury_until = crate::injuries::injury_until(&player.injuries, clock);
                let recent_injury = crate::injuries::recent_injury_date(&player.injuries);
                let injury_status = if injury_until.is_some() {
                    "injured"
                } else {
                    "fit"
                };
                let best_role = player.best_role.as_ref();
                player_stmt.execute(params![
                    save_id,
                    player.uid,
                    player.unique_id,
                    player.name,
                    player.first_name,
                    player.last_name,
                    player.common_name,
                    player.birth_date,
                    player.age,
                    player.nation_id,
                    player.nation_name,
                    player.gender,
                    player.club_uid,
                    player.club_name,
                    player.ca,
                    player.pa,
                    player.pa_range_code,
                    player.height_cm,
                    player.left_foot,
                    player.right_foot,
                    player.transfer_value,
                    serde_json::to_string(&player.transfer_value_state)?,
                    serde_json::to_string(&player.positions)?,
                    player.natural_positions.join(" / "),
                    player
                        .personality
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                    serde_json::to_string(&player.traits)?,
                    serde_json::to_string(&player.attributes)?,
                    serde_json::to_string(&player.raw_attributes)?,
                    best_role.map(|role| role.id.as_str()),
                    best_role.map(|role| role.name.as_str()),
                    best_role.map(|role| role.score),
                    best_role.map(|role| role.position.as_str()),
                    injury_status,
                    injury_until,
                    recent_injury,
                    player.suspensions.len() as i64,
                    serde_json::to_string(&player.injuries)?,
                    serde_json::to_string(&player.suspensions)?,
                    contract_end,
                    wage,
                    on_loan,
                    player.transfer_listed,
                    player.loan_listed,
                    player.league_competition_id,
                    player.league_name,
                    serde_json::to_string(player)?,
                ])?;

                if let Some(game_date) = parsed.summary.game_date.as_deref() {
                    snapshot_stmt.execute(params![
                        career_key,
                        game_date,
                        player.uid,
                        save_id,
                        player.ca,
                        player.pa,
                        player.transfer_value,
                        player.club_uid,
                        player.club_name,
                        wage,
                        contract_end,
                        recorded_at,
                    ])?;
                }

                if let Some(contract) = contract {
                    contract_stmt.execute(params![
                        save_id,
                        player.uid,
                        contract.club_uid,
                        contract.club_name,
                        contract.team_id,
                        contract.wage,
                        contract.start,
                        contract.end,
                        contract.squad_status,
                        contract.contract_type,
                        contract.on_loan,
                        contract.loan_parent_club_uid,
                        contract.loan_parent_club_name,
                        contract.loan_start,
                        contract.loan_end,
                        serde_json::to_string(&contract.clauses)?,
                        serde_json::to_string(&contract.chain)?,
                    ])?;
                }
            }
        }

        {
            let mut season_stmt = transaction.prepare(
                "INSERT INTO player_season_stats
                 (save_id, player_uid, kind, team_id, club_uid, club_name, club_short_name,
                  team_slot, minutes, starts, substitute_appearances, rated_appearances,
                  average_rating, player_of_the_match, goals, assists, expected_goals,
                  non_penalty_expected_goals, expected_assists, shots, shots_on_target,
                  shots_outside_box, goals_outside_box, free_kick_shots, penalties_taken,
                  penalties_scored, passes_attempted, passes_completed, progressive_passes,
                  key_passes, open_play_key_passes, clear_cut_chances_created, crosses_attempted,
                  crosses_completed, open_play_crosses_attempted, open_play_crosses_completed,
                  dribbles, offsides, distance_km, high_intensity_sprints,
                  aerial_challenges_attempted, headers_won, key_headers, tackles_attempted,
                  tackles_completed, key_tackles, interceptions, possession_won, possession_lost,
                  pressures_attempted, pressures_completed, blocks, shots_blocked, clearances,
                  fouls_made, fouls_against, yellow_cards, red_cards,
                  mistakes_leading_to_goal, clean_sheets, goals_allowed, saves_held,
                  saves_parried, saves_tipped, shots_on_target_faced, expected_goals_prevented,
                  advanced_available)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )?;
            for row in &parsed.season_stats {
                // The save zeroes the whole advanced block for some clubs and
                // competitions. Those zeroes are not measurements; write NULL and
                // keep the availability flag so scoring cannot mistake them for data.
                let advanced_u32 = |value: u32| row.advanced_available.then_some(value);
                let advanced_f64 = |value: f64| row.advanced_available.then_some(value);
                let advanced_opt =
                    |value: Option<u32>| row.advanced_available.then_some(value).flatten();
                season_stmt.execute(params![
                    save_id,
                    row.player_uid,
                    row.kind.as_str(),
                    row.team_id.unwrap_or(0),
                    row.club_uid,
                    row.club_name,
                    row.club_short_name,
                    row.team_slot,
                    row.minutes,
                    row.starts,
                    row.substitute_appearances,
                    row.rated_appearances,
                    row.average_rating,
                    row.player_of_the_match,
                    row.goals,
                    row.assists,
                    row.expected_goals,
                    row.non_penalty_expected_goals,
                    row.expected_assists,
                    row.shots,
                    row.shots_on_target,
                    advanced_u32(row.shots_outside_box),
                    row.goals_outside_box,
                    advanced_u32(row.free_kick_shots),
                    row.penalties_taken,
                    row.penalties_scored,
                    row.passes_attempted,
                    row.passes_completed,
                    advanced_u32(row.progressive_passes),
                    advanced_u32(row.key_passes),
                    advanced_u32(row.open_play_key_passes),
                    advanced_u32(row.clear_cut_chances_created),
                    advanced_u32(row.crosses_attempted),
                    advanced_u32(row.crosses_completed),
                    advanced_u32(row.open_play_crosses_attempted),
                    advanced_opt(row.open_play_crosses_completed),
                    advanced_u32(row.dribbles),
                    advanced_u32(row.offsides),
                    advanced_f64(row.distance_km),
                    advanced_u32(row.high_intensity_sprints),
                    advanced_opt(row.aerial_challenges_attempted),
                    advanced_opt(row.headers_won),
                    advanced_u32(row.key_headers),
                    row.tackles_attempted,
                    row.tackles_completed,
                    advanced_u32(row.key_tackles),
                    advanced_u32(row.interceptions),
                    advanced_u32(row.possession_won),
                    advanced_u32(row.possession_lost),
                    row.pressures_attempted,
                    row.pressures_completed,
                    advanced_opt(row.blocks),
                    advanced_u32(row.shots_blocked),
                    advanced_opt(row.clearances),
                    row.fouls_made,
                    row.fouls_against,
                    row.yellow_cards,
                    row.red_cards,
                    row.mistakes_leading_to_goal,
                    row.clean_sheets,
                    row.goals_allowed,
                    advanced_opt(row.saves_held),
                    advanced_opt(row.saves_parried),
                    advanced_opt(row.saves_tipped),
                    advanced_opt(row.shots_on_target_faced),
                    advanced_f64(row.expected_goals_prevented),
                    row.advanced_available,
                ])?;
            }
        }

        {
            let mut latest_finance: HashMap<u32, &FinanceMonth> = HashMap::new();
            for finance in &parsed.club_finances {
                match latest_finance.get(&finance.club_uid) {
                    Some(current) if current.month >= finance.month => {}
                    _ => {
                        latest_finance.insert(finance.club_uid, finance);
                    }
                }
            }
            let mut club_stmt = transaction.prepare(
                "INSERT INTO clubs
                 (save_id, uid, unique_id, name, short_name, nation_id, nation_name, fa_nation_id,
                  fa_nation_name, city_id, gender, reputation, last_league_position,
                  parent_club_uid, parent_club_name, corporate_facilities, training_facilities,
                  youth_facilities, junior_coaching, youth_recruitment, youth_level,
                  finance_month, balance, transfer_budget_allocated, transfer_budget_remaining,
                  wage_budget_weekly, wage_payroll_weekly, net, league_competition_id, league_name,
                  teams_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
                         ?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31)",
            )?;
            for club in &parsed.clubs {
                let finance = latest_finance.get(&club.uid);
                club_stmt.execute(params![
                    save_id,
                    club.uid,
                    club.unique_id,
                    club.name,
                    club.short_name,
                    club.nation_id,
                    club.nation_name,
                    club.fa_nation_id,
                    club.fa_nation_name,
                    club.city_id,
                    club.gender,
                    club.reputation,
                    club.last_league_position,
                    club.parent_club_uid,
                    club.parent_club_name,
                    club.corporate_facilities,
                    club.training_facilities,
                    club.youth_facilities,
                    club.junior_coaching,
                    club.youth_recruitment,
                    club.youth_level,
                    finance.map(|row| row.month.as_str()),
                    finance.map(|row| row.balance),
                    finance.map(|row| row.transfer_budget_allocated),
                    finance.map(|row| row.transfer_budget_remaining),
                    finance.map(|row| row.wage_budget_weekly),
                    finance.map(|row| row.wage_payroll_weekly),
                    finance.map(|row| row.net),
                    club.league_competition_id,
                    club.league_name,
                    serde_json::to_string(&club.teams)?,
                ])?;
            }
        }

        {
            let mut finance_stmt = transaction.prepare(
                "INSERT INTO club_finances
                 (save_id, club_uid, month, balance, transfer_budget_allocated,
                  transfer_budget_remaining, wage_budget_weekly, wage_payroll_weekly,
                  income_excluding_transfers, net_transfers, wage_bill, net,
                  expenditure_excluding_transfers, total_income, total_expenditure)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            )?;
            for finance in &parsed.club_finances {
                finance_stmt.execute(params![
                    save_id,
                    finance.club_uid,
                    finance.month,
                    finance.balance,
                    finance.transfer_budget_allocated,
                    finance.transfer_budget_remaining,
                    finance.wage_budget_weekly,
                    finance.wage_payroll_weekly,
                    finance.income_excluding_transfers,
                    finance.net_transfers,
                    finance.wage_bill,
                    finance.net,
                    finance.expenditure_excluding_transfers,
                    finance.total_income,
                    finance.total_expenditure,
                ])?;
            }
        }

        {
            let mut stadium_stmt = transaction.prepare(
                "INSERT INTO stadiums
                 (save_id, uid, name, all_seater_capacity, expansion_capacity, capacity,
                  owner_club_uid, owner_club_name, pitch_length_dm, pitch_width_dm,
                  pitch_min_length_dm, pitch_min_width_dm, pitch_max_length_dm,
                  pitch_max_width_dm, built_date, rebuilt_date)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            )?;
            for stadium in &parsed.stadiums {
                stadium_stmt.execute(params![
                    save_id,
                    stadium.uid,
                    stadium.name,
                    stadium.all_seater_capacity,
                    stadium.expansion_capacity,
                    stadium.capacity,
                    stadium.owner_club_uid,
                    stadium.owner_club_name,
                    stadium.pitch_length_dm,
                    stadium.pitch_width_dm,
                    stadium.pitch_min_length_dm,
                    stadium.pitch_min_width_dm,
                    stadium.pitch_max_length_dm,
                    stadium.pitch_max_width_dm,
                    stadium.built_date,
                    stadium.rebuilt_date,
                ])?;
            }
        }

        {
            let mut competition_stmt = transaction.prepare(
                "INSERT INTO competitions
                 (save_id, id, database_id, name, stage_count, stage_ids_json)
                 VALUES (?1,?2,?3,?4,?5,?6)",
            )?;
            for competition in &parsed.competitions {
                competition_stmt.execute(params![
                    save_id,
                    competition.id,
                    competition.database_id,
                    competition.name,
                    competition.stage_ids.len() as i64,
                    serde_json::to_string(&competition.stage_ids)?,
                ])?;
            }
        }

        transaction.commit()?;
        // A full-career insert creates a large WAL. Fold it back into the database now
        // so the cache does not keep two copies of the same rows on disk.
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(save_id)
    }

    pub fn save_path(&self, save_id: i64) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT path FROM saves WHERE id = ?1",
                params![save_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Error::from)
    }

    pub fn list_saves(&self) -> Result<Vec<SaveRecord>> {
        let mut statement = self.conn.prepare(
            "SELECT s.id, s.path, s.file_name, s.game, s.build, s.game_date, s.parser_version,
                    s.parsed_at,
                    (SELECT COUNT(*) FROM players p WHERE p.save_id = s.id),
                    (SELECT COUNT(*) FROM clubs c WHERE c.save_id = s.id),
                    (SELECT COUNT(*) FROM contracts c WHERE c.save_id = s.id)
             FROM saves s ORDER BY s.parsed_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SaveRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                file_name: row.get(2)?,
                game: row.get(3)?,
                build: row.get(4)?,
                game_date: row.get(5)?,
                parser_version: row.get(6)?,
                parsed_at: row.get(7)?,
                players: row.get(8)?,
                clubs: row.get(9)?,
                contracts: row.get(10)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    pub fn competitions(&self, save_id: i64) -> Result<Vec<Competition>> {
        let mut statement = self.conn.prepare(
            "SELECT id, database_id, name, stage_ids_json
             FROM competitions WHERE save_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map(params![save_id], |row| {
            let stage_ids_json: String = row.get(3)?;
            Ok(Competition {
                id: row.get(0)?,
                database_id: row.get(1)?,
                name: row.get(2)?,
                stage_ids: serde_json::from_str(&stage_ids_json).unwrap_or_default(),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    pub fn season_stats_count(&self, save_id: i64) -> Result<usize> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM player_season_stats WHERE save_id = ?1",
                params![save_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as usize)
            .map_err(Error::from)
    }

    pub fn player_metrics_count(
        &self,
        save_id: i64,
        model: &str,
        min_minutes: u32,
    ) -> Result<usize> {
        let model = crate::stats_moneyball::normalized_model(Some(model))?;
        let model = crate::stats_moneyball::model_name(model);
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM player_metrics
                 WHERE save_id = ?1 AND model = ?2 AND min_minutes = ?3",
                params![save_id, model, min_minutes],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as usize)
            .map_err(Error::from)
    }

    /// Read the materialized rows directly, without recomputing. Callers that
    /// need to guarantee materialization should run `stats_moneyball` first.
    pub fn player_metrics_rows(
        &self,
        save_id: i64,
        model: &str,
        min_minutes: u32,
    ) -> Result<Vec<crate::stats_moneyball::StatsMoneyballRow>> {
        let model = crate::stats_moneyball::normalized_model(Some(model))?;
        let model = crate::stats_moneyball::model_name(model);
        let key = StatsCacheKey {
            save_id,
            model: model.to_owned(),
            min_minutes,
        };
        Ok(self.read_player_metrics(&key)?.unwrap_or_default())
    }

    pub fn delete_save(&mut self, save_id: i64) -> Result<()> {
        if let Ok(mut cache) = self.stats_cache.lock() {
            cache.clear();
        }
        self.conn
            .execute("DELETE FROM saves WHERE id = ?1", params![save_id])?;
        Ok(())
    }

    pub fn query_players(&self, query: &PlayerQuery) -> Result<PlayerPage> {
        let mut params: Vec<Value> = Vec::new();
        let where_sql = build_where(query, &mut params)?;
        let count_sql = format!(
            "SELECT COUNT(*) FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE {where_sql}"
        );
        let total: u64 =
            self.conn
                .query_row(&count_sql, params_from_iter(params.iter()), |row| {
                    row.get(0)
                })?;

        let page = query.page.unwrap_or(0);
        let page_size = query.page_size.unwrap_or(150).clamp(1, 500);
        let order = order_by(query);
        let sql = format!(
            "SELECT p.uid, p.name, p.age, p.club_uid, p.club_name, p.league_competition_id,
                    p.league_name, p.nation_id, p.ca, p.pa,
                    p.pa_range_code, p.positions_text, p.positions_json, p.attributes_json,
                    p.personality_json, p.transfer_value, p.transfer_value_state, c.wage,
                    c.end_date, p.height_cm, p.left_foot, p.right_foot, c.on_loan, p.traits_json,
                    p.nation_name, p.gender, p.best_role_id, p.best_role_name,
                    p.best_role_score, p.best_role_position, p.injury_status, p.injury_until,
                    p.recent_injury, p.suspension_count, p.transfer_listed, p.loan_listed,
                    p.unique_id
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE {where_sql}
             {order}
             LIMIT ? OFFSET ?"
        );
        let mut page_params = params;
        page_params.push(Value::Integer(page_size as i64));
        page_params.push(Value::Integer((page as i64) * page_size as i64));
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(page_params.iter()), |row| {
            Ok(PlayerRow {
                uid: row.get(0)?,
                generated: is_generated_unique_id(row.get::<_, Option<u32>>(36)?),
                name: row.get(1)?,
                age: row.get(2)?,
                club_uid: row.get(3)?,
                club_name: row.get(4)?,
                league_competition_id: row.get(5)?,
                league_name: row.get(6)?,
                nation_id: row.get(7)?,
                ca: row.get(8)?,
                pa: row.get(9)?,
                pa_range_code: row.get(10)?,
                positions_text: row.get(11)?,
                positions: parse_json_column(row.get::<_, String>(12)?, "positions"),
                attributes: parse_json_column(row.get::<_, String>(13)?, "attributes"),
                personality: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|text| serde_json::from_str::<Personality>(&text).ok()),
                transfer_value: row.get(15)?,
                transfer_value_state: row
                    .get::<_, Option<String>>(16)?
                    .and_then(|text| serde_json::from_str::<String>(&text).ok())
                    .unwrap_or_else(|| "ok".to_owned()),
                wage: row.get(17)?,
                contract_end: row.get(18)?,
                height_cm: row.get(19)?,
                left_foot: row.get(20)?,
                right_foot: row.get(21)?,
                on_loan: row.get(22)?,
                traits: row
                    .get::<_, Option<String>>(23)?
                    .and_then(|text| serde_json::from_str::<Vec<Trait>>(&text).ok())
                    .unwrap_or_default(),
                nation_name: row.get(24)?,
                gender: row.get(25)?,
                best_role_id: row.get(26)?,
                best_role_name: row.get(27)?,
                best_role_score: row.get(28)?,
                best_role_position: row.get(29)?,
                injury_status: row.get(30)?,
                injury_until: row.get(31)?,
                recent_injury: row.get(32)?,
                suspension_count: row.get(33)?,
                transfer_listed: row.get(34)?,
                loan_listed: row.get(35)?,
            })
        })?;
        let mut rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        if !query.include_hidden {
            for row in &mut rows {
                redact_attributes(&mut row.attributes);
                row.personality = None;
            }
        }
        Ok(PlayerPage {
            total,
            page,
            page_size,
            rows,
        })
    }

    pub fn query_all(&self, query: &PlayerQuery) -> Result<Vec<PlayerRow>> {
        let mut query = query.clone();
        query.page = Some(0);
        query.page_size = Some(500);
        let mut rows = Vec::new();
        loop {
            let page = self.query_players(&query)?;
            let done = page.rows.is_empty() || rows.len() >= page.total as usize;
            rows.extend(page.rows);
            if done {
                break;
            }
            query.page = Some(query.page.unwrap_or(0) + 1);
            if rows.len() > 500_000 {
                break;
            }
        }
        Ok(rows)
    }

    /// Value Finder: load a lightweight player row once and rank it in Rust.
    ///
    fn read_player_metrics(
        &self,
        key: &StatsCacheKey,
    ) -> Result<Option<Vec<crate::stats_moneyball::StatsMoneyballRow>>> {
        let mut statement = self.conn.prepare(
            "SELECT player_uid,
                    name,
                    age,
                    club_uid,
                    club_name,
                    league_name,
                    nation_name,
                    ca,
                    pa,
                    position_group,
                    minutes,
                    starts,
                    substitute_appearances,
                    average_rating,
                    goals,
                    assists,
                    expected_goals,
                    expected_assists,
                    shots,
                    shots_on_target,
                    passes_attempted,
                    passes_completed,
                    pass_completion_percent,
                    tackles_attempted,
                    tackles_completed,
                    interceptions,
                    key_passes,
                    progressive_passes,
                    headers_won,
                    aerial_challenges_attempted,
                    clearances,
                    blocks,
                    crosses_completed,
                    saves_held,
                    shots_on_target_faced,
                    save_percent,
                    expected_goals_prevented,
                    advanced_available,
                    transfer_value,
                    transfer_value_state,
                    wage,
                    contract_end,
                    club_reputation,
                    fee,
                    cost,
                    fee_known,
                    wage_known,
                    production_global,
                    production_tier,
                    cost_pct_global,
                    cost_pct_tier,
                    surplus,
                    score,
                    confidence,
                    flags_json,
                    drivers_json,
                    transfer_listed,
                    loan_listed,
                    injury_status,
                    recent_injury
             FROM player_metrics
             WHERE save_id = ?1 AND model = ?2 AND min_minutes = ?3
             ORDER BY score DESC",
        )?;
        let rows =
            statement.query_map(params![key.save_id, key.model, key.min_minutes], |row| {
                let flags_json: String = row.get(54)?;
                let drivers_json: String = row.get(55)?;
                Ok(crate::stats_moneyball::StatsMoneyballRow {
                    uid: row.get::<_, i64>(0)? as u32,
                    name: row.get(1)?,
                    age: row.get(2)?,
                    club_uid: row.get::<_, Option<i64>>(3)?.map(|value| value as u32),
                    club_name: row.get(4)?,
                    league_name: row.get(5)?,
                    nation_name: row.get(6)?,
                    ca: row.get::<_, i64>(7)? as u16,
                    pa: row.get::<_, Option<i64>>(8)?.map(|value| value as i16),
                    position_group: row.get(9)?,
                    minutes: row.get::<_, i64>(10)? as u32,
                    starts: row.get::<_, i64>(11)? as u32,
                    substitute_appearances: row.get::<_, i64>(12)? as u32,
                    average_rating: row.get(13)?,
                    goals: row.get::<_, i64>(14)? as u32,
                    assists: row.get::<_, i64>(15)? as u32,
                    expected_goals: row.get(16)?,
                    expected_assists: row.get(17)?,
                    shots: row.get::<_, i64>(18)? as u32,
                    shots_on_target: row.get::<_, i64>(19)? as u32,
                    passes_attempted: row.get::<_, i64>(20)? as u32,
                    passes_completed: row.get::<_, i64>(21)? as u32,
                    pass_completion_percent: row.get(22)?,
                    tackles_attempted: row.get::<_, i64>(23)? as u32,
                    tackles_completed: row.get::<_, i64>(24)? as u32,
                    interceptions: row.get::<_, Option<i64>>(25)?.map(|value| value as u32),
                    key_passes: row.get::<_, Option<i64>>(26)?.map(|value| value as u32),
                    progressive_passes: row.get::<_, Option<i64>>(27)?.map(|value| value as u32),
                    headers_won: row.get::<_, Option<i64>>(28)?.map(|value| value as u32),
                    aerial_challenges_attempted: row
                        .get::<_, Option<i64>>(29)?
                        .map(|value| value as u32),
                    clearances: row.get::<_, Option<i64>>(30)?.map(|value| value as u32),
                    blocks: row.get::<_, Option<i64>>(31)?.map(|value| value as u32),
                    crosses_completed: row.get::<_, Option<i64>>(32)?.map(|value| value as u32),
                    saves_held: row.get::<_, Option<i64>>(33)?.map(|value| value as u32),
                    shots_on_target_faced: row.get::<_, Option<i64>>(34)?.map(|value| value as u32),
                    save_percent: row.get(35)?,
                    expected_goals_prevented: row.get(36)?,
                    advanced_available: row.get::<_, i64>(37)? != 0,
                    transfer_value: row.get::<_, Option<i64>>(38)?.map(|value| value as u32),
                    transfer_value_state: row.get(39)?,
                    wage: row.get::<_, Option<i64>>(40)?.map(|value| value as u32),
                    contract_end: row.get(41)?,
                    club_reputation: row.get::<_, Option<i64>>(42)?.map(|value| value as u16),
                    fee: row.get::<_, Option<i64>>(43)?.map(|value| value as u64),
                    cost: row.get::<_, i64>(44)? as u64,
                    fee_known: row.get::<_, i64>(45)? != 0,
                    wage_known: row.get::<_, i64>(46)? != 0,
                    production_global: Some(row.get(47)?),
                    production_tier: Some(row.get(48)?),
                    cost_pct_global: row.get(49)?,
                    cost_pct_tier: Some(row.get(50)?),
                    surplus: Some(row.get(51)?),
                    score: Some(row.get(52)?),
                    confidence: row.get(53)?,
                    flags: serde_json::from_str(&flags_json).unwrap_or_default(),
                    drivers: serde_json::from_str(&drivers_json).unwrap_or_default(),
                    transfer_listed: row.get::<_, i64>(56)? != 0,
                    loan_listed: row.get::<_, i64>(57)? != 0,
                    injury_status: row.get(58)?,
                    recent_injury: row.get(59)?,
                })
            })?;
        let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok((!rows.is_empty()).then_some(rows))
    }

    fn write_player_metrics(
        &self,
        key: &StatsCacheKey,
        rows: &[crate::stats_moneyball::StatsMoneyballRow],
    ) -> Result<()> {
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM player_metrics
             WHERE save_id = ?1 AND model = ?2 AND min_minutes = ?3",
            params![key.save_id, key.model, key.min_minutes],
        )?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO player_metrics
                 (save_id,
                 player_uid,
                 model,
                 min_minutes,
                 name,
                 age,
                 club_uid,
                 club_name,
                 league_name,
                 nation_name,
                 ca,
                 pa,
                 position_group,
                 minutes,
                 starts,
                 substitute_appearances,
                 average_rating,
                 goals,
                 assists,
                 expected_goals,
                 expected_assists,
                 shots,
                 shots_on_target,
                 passes_attempted,
                 passes_completed,
                 pass_completion_percent,
                 tackles_attempted,
                 tackles_completed,
                 interceptions,
                 key_passes,
                 progressive_passes,
                 headers_won,
                 aerial_challenges_attempted,
                 clearances,
                 blocks,
                 crosses_completed,
                 saves_held,
                 shots_on_target_faced,
                 save_percent,
                 expected_goals_prevented,
                 advanced_available,
                 transfer_value,
                 transfer_value_state,
                 wage,
                 contract_end,
                 club_reputation,
                 fee,
                 cost,
                 fee_known,
                 wage_known,
                 production_global,
                 production_tier,
                 cost_pct_global,
                 cost_pct_tier,
                 surplus,
                 score,
                 confidence,
                 flags_json,
                 drivers_json,
                 transfer_listed,
                 loan_listed,
                 injury_status,
                 recent_injury,
                 computed_at)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )?;
            let computed_at = Utc::now().to_rfc3339();
            for row in rows {
                statement.execute(params![
                    key.save_id,
                    row.uid,
                    key.model,
                    key.min_minutes,
                    row.name,
                    row.age,
                    row.club_uid,
                    row.club_name,
                    row.league_name,
                    row.nation_name,
                    row.ca,
                    row.pa,
                    row.position_group,
                    row.minutes,
                    row.starts,
                    row.substitute_appearances,
                    row.average_rating,
                    row.goals,
                    row.assists,
                    row.expected_goals,
                    row.expected_assists,
                    row.shots,
                    row.shots_on_target,
                    row.passes_attempted,
                    row.passes_completed,
                    row.pass_completion_percent,
                    row.tackles_attempted,
                    row.tackles_completed,
                    row.interceptions,
                    row.key_passes,
                    row.progressive_passes,
                    row.headers_won,
                    row.aerial_challenges_attempted,
                    row.clearances,
                    row.blocks,
                    row.crosses_completed,
                    row.saves_held,
                    row.shots_on_target_faced,
                    row.save_percent,
                    row.expected_goals_prevented,
                    row.advanced_available,
                    row.transfer_value,
                    row.transfer_value_state,
                    row.wage,
                    row.contract_end,
                    row.club_reputation,
                    row.fee.map(|value| value as i64),
                    row.cost as i64,
                    row.fee_known,
                    row.wage_known,
                    row.production_global,
                    row.production_tier,
                    row.cost_pct_global,
                    row.cost_pct_tier,
                    row.surplus,
                    row.score,
                    row.confidence,
                    serde_json::to_string(&row.flags)?,
                    serde_json::to_string(&row.drivers)?,
                    row.transfer_listed,
                    row.loan_listed,
                    row.injury_status,
                    row.recent_injury,
                    computed_at,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn clear_stats_cache(&self) {
        if let Ok(mut cache) = self.stats_cache.lock() {
            cache.clear();
        }
    }

    pub fn stats_moneyball(
        &self,
        request: &crate::stats_moneyball::StatsMoneyballRequest,
    ) -> Result<crate::stats_moneyball::StatsMoneyballPage> {
        let model = crate::stats_moneyball::normalized_model(request.model.as_deref())?;
        let min_minutes = request.min_minutes.unwrap_or(900);
        let cache_key = StatsCacheKey {
            save_id: request.save_id,
            model: crate::stats_moneyball::model_name(model).to_owned(),
            min_minutes,
        };
        if let Ok(cache) = self.stats_cache.lock()
            && let Some(rows) = cache.get(&cache_key)
        {
            return crate::stats_moneyball::apply_stats_request(rows.as_slice(), request);
        }
        if let Some(rows) = self.read_player_metrics(&cache_key)? {
            let rows = Arc::new(rows);
            if let Ok(mut cache) = self.stats_cache.lock() {
                cache.insert(cache_key.clone(), Arc::clone(&rows));
            }
            return crate::stats_moneyball::apply_stats_request(rows.as_slice(), request);
        }
        let mut statement = self.conn.prepare(
            "SELECT p.uid, p.name, p.age, p.club_uid, p.club_name, p.league_name,
                    p.nation_name, p.ca, p.pa, p.transfer_value, p.transfer_value_state,
                    p.transfer_listed, p.loan_listed, p.injury_status, p.recent_injury,
                    p.positions_text, c.wage, c.end_date, cl.reputation,
                    s.club_uid, s.minutes, s.starts, s.substitute_appearances,
                    s.average_rating, s.goals, s.assists, s.expected_goals,
                    s.expected_assists, s.shots, s.shots_on_target, s.passes_attempted,
                    s.passes_completed, s.tackles_attempted, s.tackles_completed,
                    s.goals_allowed, s.clean_sheets, s.interceptions, s.key_passes,
                    s.progressive_passes, s.dribbles, s.headers_won,
                    s.aerial_challenges_attempted, s.clearances, s.blocks,
                    s.crosses_completed, s.saves_held, s.saves_parried, s.saves_tipped,
                    s.shots_on_target_faced, s.expected_goals_prevented, s.advanced_available
             FROM player_season_stats s
             JOIN players p ON p.save_id = s.save_id AND p.uid = s.player_uid
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             LEFT JOIN clubs cl ON cl.save_id = p.save_id AND cl.uid = p.club_uid
             WHERE s.save_id = ?1 AND s.kind = 'overall' AND s.minutes >= ?2",
        )?;
        let rows = statement.query_map(params![request.save_id, min_minutes], |row| {
            let transfer_value_state = row
                .get::<_, Option<String>>(10)?
                .and_then(|text| serde_json::from_str::<String>(&text).ok())
                .unwrap_or_else(|| "ok".to_owned());
            let player_club_uid: Option<u32> = row.get(3)?;
            let stats_club_uid: Option<u32> = row.get(19)?;
            Ok(crate::stats_moneyball::StatsMoneyballCandidate {
                uid: row.get(0)?,
                name: row.get(1)?,
                age: row.get(2)?,
                club_uid: player_club_uid,
                club_name: row.get(4)?,
                league_name: row.get(5)?,
                nation_name: row.get(6)?,
                ca: row.get(7)?,
                pa: row.get(8)?,
                positions_text: row.get(15)?,
                transfer_value: row.get(9)?,
                transfer_value_state,
                wage: row.get(16)?,
                contract_end: row.get(17)?,
                club_reputation: row.get(18)?,
                transfer_listed: row.get(11)?,
                loan_listed: row.get(12)?,
                injury_status: row.get(13)?,
                recent_injury: row.get(14)?,
                minutes: row.get(20)?,
                starts: row.get(21)?,
                substitute_appearances: row.get(22)?,
                average_rating: row.get(23)?,
                goals: row.get(24)?,
                assists: row.get(25)?,
                expected_goals: row.get(26)?,
                expected_assists: row.get(27)?,
                shots: row.get(28)?,
                shots_on_target: row.get(29)?,
                passes_attempted: row.get(30)?,
                passes_completed: row.get(31)?,
                tackles_attempted: row.get(32)?,
                tackles_completed: row.get(33)?,
                goals_allowed: row.get(34)?,
                clean_sheets: row.get(35)?,
                interceptions: row.get(36)?,
                key_passes: row.get(37)?,
                progressive_passes: row.get(38)?,
                dribbles: row.get(39)?,
                headers_won: row.get(40)?,
                aerial_challenges_attempted: row.get(41)?,
                clearances: row.get(42)?,
                blocks: row.get(43)?,
                crosses_completed: row.get(44)?,
                saves_held: row.get(45)?,
                saves_parried: row.get(46)?,
                saves_tipped: row.get(47)?,
                shots_on_target_faced: row.get(48)?,
                expected_goals_prevented: row.get(49)?,
                advanced_available: row.get::<_, i64>(50)? != 0,
                current_club: player_club_uid.is_some() && player_club_uid == stats_club_uid,
            })
        })?;
        let mut by_uid: HashMap<u32, crate::stats_moneyball::StatsMoneyballCandidate> =
            HashMap::new();
        for row in rows {
            let candidate = row?;
            match by_uid.entry(candidate.uid) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let existing = entry.get();
                    let better = (candidate.current_club && !existing.current_club)
                        || (candidate.current_club == existing.current_club
                            && candidate.minutes > existing.minutes);
                    if better {
                        entry.insert(candidate);
                    }
                }
            }
        }
        let scored = crate::stats_moneyball::score_stats_candidates(
            by_uid.into_values().collect(),
            model,
            min_minutes,
        )?;
        self.write_player_metrics(&cache_key, &scored)?;
        let rows = Arc::new(scored);
        if let Ok(mut cache) = self.stats_cache.lock() {
            cache.insert(cache_key, Arc::clone(&rows));
        }
        crate::stats_moneyball::apply_stats_request(rows.as_slice(), request)
    }

    /// The ranking deliberately lives outside SQL: percentile groups, missing
    /// cost handling and the explainable score are easier to test and evolve as
    /// pure functions. The candidate query returns one row per player, not per
    /// contract or per match.
    pub fn value_finder(
        &self,
        request: &crate::moneyball::ValueFinderRequest,
    ) -> Result<crate::moneyball::ValueFinderPage> {
        let mut statement = self.conn.prepare(
            "SELECT p.uid, p.name, p.age, p.club_uid, p.club_name, p.league_name,
                    p.nation_name, p.ca, p.pa, p.best_role_id, p.best_role_name,
                    p.best_role_score, p.best_role_position, p.transfer_value,
                    p.transfer_value_state, c.wage, c.end_date, cl.reputation,
                    p.transfer_listed, p.loan_listed, p.injury_status, p.recent_injury
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             LEFT JOIN clubs cl ON cl.save_id = p.save_id AND cl.uid = p.club_uid
             WHERE p.save_id = ?1",
        )?;
        let rows = statement.query_map(params![request.save_id], |row| {
            let transfer_value_state = row
                .get::<_, Option<String>>(14)?
                .and_then(|text| serde_json::from_str::<String>(&text).ok())
                .unwrap_or_else(|| "ok".to_owned());
            Ok(crate::moneyball::ValueFinderCandidate {
                uid: row.get(0)?,
                name: row.get(1)?,
                age: row.get(2)?,
                club_uid: row.get(3)?,
                club_name: row.get(4)?,
                league_name: row.get(5)?,
                nation_name: row.get(6)?,
                ca: row.get(7)?,
                pa: row.get(8)?,
                role_id: row.get(9)?,
                role_name: row.get(10)?,
                role_score: row.get(11)?,
                role_position: row.get(12)?,
                transfer_value: row.get(13)?,
                transfer_value_state,
                wage: row.get(15)?,
                contract_end: row.get(16)?,
                club_reputation: row.get(17)?,
                transfer_listed: row.get(18)?,
                loan_listed: row.get(19)?,
                injury_status: row.get(20)?,
                recent_injury: row.get(21)?,
            })
        })?;
        let candidates = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        crate::moneyball::score_value_finder(candidates, request)
    }

    pub fn get_player(
        &self,
        save_id: i64,
        uid: u32,
        include_hidden: bool,
    ) -> Result<Option<Player>> {
        let data: Option<String> = self
            .conn
            .query_row(
                "SELECT data_json FROM players WHERE save_id = ?1 AND uid = ?2",
                params![save_id, uid],
                |row| row.get(0),
            )
            .optional()?;
        let mut player = data
            .map(|text| serde_json::from_str::<Player>(&text).map_err(Error::from))
            .transpose()?;
        if let Some(player) = &mut player {
            // Cached rows written before the flag existed still carry the Unique ID.
            player.generated = is_generated_unique_id(player.unique_id);
            if !include_hidden {
                redact_player(player);
            }
        }
        Ok(player)
    }

    pub fn get_contract(&self, save_id: i64, player_uid: u32) -> Result<Option<Contract>> {
        Ok(self
            .get_player(save_id, player_uid, true)?
            .and_then(|player| player.contract))
    }

    pub fn growth_history(&self, save_id: i64, player_uid: u32) -> Result<Vec<SnapshotPoint>> {
        let save: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT career_key, game_date FROM saves WHERE id = ?1",
                params![save_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((career_key, game_date)) = save else {
            return Ok(Vec::new());
        };
        let mut statement = self.conn.prepare(
            "SELECT game_date, ca, pa, transfer_value, club_uid, club_name, wage, contract_end
             FROM player_snapshots
             WHERE career_key = ?1 AND player_uid = ?2
               AND (?3 IS NULL OR game_date <= ?3)
             ORDER BY game_date",
        )?;
        let mut points = statement
            .query_map(params![career_key, player_uid, game_date], |row| {
                Ok(SnapshotPoint {
                    game_date: row.get(0)?,
                    ca: row.get(1)?,
                    pa: row.get(2)?,
                    transfer_value: row.get(3)?,
                    club_uid: row.get(4)?,
                    club_name: row.get(5)?,
                    wage: row.get(6)?,
                    contract_end: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if points.is_empty() {
            let fallback = self
                .conn
                .query_row(
                    "SELECT s.game_date, p.ca, p.pa, p.transfer_value, p.club_uid, p.club_name,
                        c.wage, c.end_date
                 FROM players p
                 JOIN saves s ON s.id = p.save_id
                 LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
                 WHERE p.save_id = ?1 AND p.uid = ?2",
                    params![save_id, player_uid],
                    |row| {
                        Ok(SnapshotPoint {
                            game_date: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                            ca: row.get(1)?,
                            pa: row.get(2)?,
                            transfer_value: row.get(3)?,
                            club_uid: row.get(4)?,
                            club_name: row.get(5)?,
                            wage: row.get(6)?,
                            contract_end: row.get(7)?,
                        })
                    },
                )
                .optional()?;
            if let Some(point) = fallback {
                points.push(point);
            }
        }
        Ok(points)
    }

    pub fn analysis_overview(&self, save_id: i64) -> Result<AnalysisOverview> {
        let game_date_text: Option<String> = self
            .conn
            .query_row(
                "SELECT game_date FROM saves WHERE id = ?1",
                params![save_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let game_date = game_date_text
            .as_deref()
            .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok());
        let cutoff = game_date.and_then(|date| date.checked_add_months(Months::new(12)));
        let total_clubs: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clubs WHERE save_id = ?1",
            params![save_id],
            |row| row.get(0),
        )?;

        let mut statement = self.conn.prepare(
            "SELECT p.age, p.ca, p.pa, p.transfer_value, p.club_uid, p.club_name,
                    c.end_date, c.on_loan
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE p.save_id = ?1",
        )?;
        let rows = statement.query_map(params![save_id], |row| {
            Ok((
                row.get::<_, Option<i32>>(0)?,
                row.get::<_, u16>(1)?,
                row.get::<_, Option<i16>>(2)?,
                row.get::<_, Option<u32>>(3)?,
                row.get::<_, Option<u32>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<bool>>(7)?,
            ))
        })?;

        let mut player_count = 0u64;
        let mut age_sum = 0i64;
        let mut age_count = 0u64;
        let mut ca_values = Vec::new();
        let mut total_value = 0u64;
        let mut free_agents = 0u64;
        let mut loanees = 0u64;
        let mut expiring = 0u64;
        let mut age_counts: HashMap<&'static str, u64> = HashMap::new();
        let mut ca_counts: HashMap<&'static str, u64> = HashMap::new();
        let mut contract_years: HashMap<u16, u64> = HashMap::new();
        let mut clubs: HashMap<u32, (String, u64, u64, u16, u64, u64)> = HashMap::new();

        for row in rows {
            let (age, ca, _pa, value, club_uid, club_name, contract_end, on_loan) = row?;
            player_count += 1;
            if let Some(age) = age {
                age_sum += age as i64;
                age_count += 1;
                *age_counts.entry(age_band_label(age)).or_default() += 1;
            }
            if ca > 0 {
                ca_values.push(ca);
                *ca_counts.entry(ca_band_label(ca)).or_default() += 1;
            }
            if let Some(value) = value {
                total_value += value as u64;
            }
            if club_uid.is_none() {
                free_agents += 1;
            }
            if on_loan == Some(true) {
                loanees += 1;
            }
            if let (Some(end), Some(cutoff)) = (contract_end.as_deref(), cutoff) {
                if end <= cutoff.format("%Y-%m-%d").to_string().as_str()
                    && game_date
                        .is_none_or(|start| end >= start.format("%Y-%m-%d").to_string().as_str())
                {
                    expiring += 1;
                }
            }
            if let Some(year) = contract_end.as_deref().and_then(contract_year) {
                *contract_years.entry(year).or_default() += 1;
            }
            if let (Some(uid), Some(name)) = (club_uid, club_name) {
                let entry = clubs.entry(uid).or_insert((name, 0, 0, 0, 0, 0));
                entry.1 += 1;
                entry.2 += ca as u64;
                entry.3 = entry.3.max(ca);
                if let Some(age) = age {
                    entry.4 += age as u64;
                }
                if let Some(value) = value {
                    entry.5 += value as u64;
                }
            }
        }

        ca_values.sort_unstable();
        let median_ca = ca_values.get(ca_values.len() / 2).copied().unwrap_or(0);
        let mut top_clubs: Vec<ClubOverview> = clubs
            .into_iter()
            .filter(|(_, (_, players, _, _, _, _))| *players >= 10)
            .map(
                |(club_uid, (club_name, players, ca_sum, max_ca, age_sum, value))| ClubOverview {
                    club_uid,
                    club_name,
                    players,
                    avg_ca: round1(ca_sum as f64 / players as f64),
                    max_ca,
                    avg_age: round1(age_sum as f64 / players as f64),
                    total_value: value,
                },
            )
            .collect();
        top_clubs.sort_by(|left, right| {
            right
                .avg_ca
                .total_cmp(&left.avg_ca)
                .then_with(|| right.players.cmp(&left.players))
        });
        top_clubs.truncate(12);

        Ok(AnalysisOverview {
            total_players: player_count,
            total_clubs,
            average_age: round1(age_sum as f64 / age_count.max(1) as f64),
            median_ca,
            total_value,
            free_agents,
            loanees,
            expiring_contracts: expiring,
            game_date: game_date_text,
            age_bands: distribution(&AGE_BANDS, &age_counts),
            ca_bands: distribution(&CA_BANDS, &ca_counts),
            contract_years: contract_year_bands(contract_years),
            top_clubs,
        })
    }

    pub fn squad_analysis(&self, save_id: i64, club_uid: u32) -> Result<SquadAnalysis> {
        let page = self.query_players(&PlayerQuery {
            save_id,
            club_uids: vec![club_uid],
            page_size: Some(500),
            sort_by: Some("ca".to_owned()),
            sort_dir: Some("desc".to_owned()),
            ..PlayerQuery::default()
        })?;
        let club_name: String = self
            .conn
            .query_row(
                "SELECT name FROM clubs WHERE save_id = ?1 AND uid = ?2",
                params![save_id, club_uid],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| {
                page.rows
                    .first()
                    .and_then(|row| row.club_name.clone())
                    .unwrap_or_else(|| format!("Club {club_uid}"))
            });

        let mut age_sum = 0i64;
        let mut age_count = 0u64;
        let mut ca_sum = 0u64;
        let mut pa_sum = 0u64;
        let mut pa_count = 0u64;
        let mut total_wage = 0u64;
        let mut total_value = 0u64;
        let mut loanees = 0u64;
        let mut age_counts: HashMap<&'static str, u64> = HashMap::new();
        let mut contract_years: HashMap<u16, u64> = HashMap::new();
        let mut position_counts: HashMap<&'static str, u64> = HashMap::new();

        for player in &page.rows {
            if let Some(age) = player.age {
                age_sum += age as i64;
                age_count += 1;
                *age_counts.entry(age_band_label(age)).or_default() += 1;
            }
            ca_sum += player.ca as u64;
            if let Some(pa) = player.pa {
                pa_sum += pa as u64;
                pa_count += 1;
            }
            if let Some(wage) = player.wage {
                total_wage += wage as u64;
            }
            if let Some(value) = player.transfer_value {
                total_value += value as u64;
            }
            if player.on_loan == Some(true) {
                loanees += 1;
            }
            if let Some(year) = player.contract_end.as_deref().and_then(contract_year) {
                *contract_years.entry(year).or_default() += 1;
            }
            let positions = &player.positions;
            let rating = |key: &str| positions.get(key).unwrap_or(0);
            if rating("gk") >= 15 {
                *position_counts.entry("门将").or_default() += 1;
            }
            if ["dl", "dc", "dr", "wbl", "wbr"]
                .iter()
                .any(|key| rating(key) >= 15)
            {
                *position_counts.entry("后卫").or_default() += 1;
            }
            if ["dm", "ml", "mc", "mr"].iter().any(|key| rating(key) >= 15) {
                *position_counts.entry("中场").or_default() += 1;
            }
            if ["stc", "aml", "amc", "amr"]
                .iter()
                .any(|key| rating(key) >= 15)
            {
                *position_counts.entry("前锋").or_default() += 1;
            }
        }

        Ok(SquadAnalysis {
            club_uid,
            club_name,
            player_count: page.rows.len() as u64,
            average_age: round1(age_sum as f64 / age_count.max(1) as f64),
            average_ca: round1(ca_sum as f64 / page.rows.len().max(1) as f64),
            average_pa: round1(pa_sum as f64 / pa_count.max(1) as f64),
            total_wage,
            total_value,
            loanees,
            position_groups: distribution(&SQUAD_GROUPS, &position_counts),
            age_bands: distribution(&AGE_BANDS, &age_counts),
            contract_years: contract_year_bands(contract_years),
            players: page.rows,
        })
    }

    pub fn query_clubs(&self, query: &ClubQuery) -> Result<ClubPage> {
        let mut params: Vec<Value> = Vec::new();
        params.push(Value::Integer(query.save_id));
        params.push(Value::Integer(query.save_id));
        let where_sql = build_club_where(query, &mut params)?;
        let count_sql = format!(
            "{} SELECT COUNT(*)
             FROM clubs c
             LEFT JOIN stats s ON s.club_uid = c.uid
             LEFT JOIN stads st ON st.owner_club_uid = c.uid
             WHERE {where_sql}",
            CLUB_CTE
        );
        let total: u64 =
            self.conn
                .query_row(&count_sql, params_from_iter(params.iter()), |row| {
                    row.get(0)
                })?;

        let page = query.page.unwrap_or(0);
        let page_size = query.page_size.unwrap_or(100).clamp(1, 500);
        let order = club_order_by(query);
        let sql = format!(
            "{} SELECT c.uid, c.unique_id, COALESCE(c.name, ''), COALESCE(c.short_name, ''),
                    c.nation_id, c.nation_name, c.fa_nation_id, c.fa_nation_name, c.city_id,
                    c.gender, c.reputation, c.last_league_position, c.league_competition_id,
                    c.league_name, c.parent_club_uid, c.parent_club_name, c.corporate_facilities,
                    c.finance_month, c.balance,
                    c.transfer_budget_allocated, c.transfer_budget_remaining,
                    c.wage_budget_weekly, c.wage_payroll_weekly, c.net,
                    COALESCE(s.player_count, 0), s.average_age, s.average_ca, s.total_value,
                    s.total_wage, COALESCE(st.stadium_count, 0), st.stadium_capacity
             FROM clubs c
             LEFT JOIN stats s ON s.club_uid = c.uid
             LEFT JOIN stads st ON st.owner_club_uid = c.uid
             WHERE {where_sql}
             {order}
             LIMIT ? OFFSET ?",
            CLUB_CTE
        );
        let mut page_params = params;
        page_params.push(Value::Integer(page_size as i64));
        page_params.push(Value::Integer((page as i64) * page_size as i64));
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(page_params.iter()), |row| {
            Ok(ClubRow {
                uid: row.get(0)?,
                unique_id: row.get(1)?,
                name: row.get(2)?,
                short_name: row.get(3)?,
                nation_id: row.get(4)?,
                nation_name: row.get(5)?,
                fa_nation_id: row.get(6)?,
                fa_nation_name: row.get(7)?,
                city_id: row.get(8)?,
                gender: row.get(9)?,
                reputation: row.get(10)?,
                last_league_position: row.get(11)?,
                league_competition_id: row.get(12)?,
                league_name: row.get(13)?,
                parent_club_uid: row.get(14)?,
                parent_club_name: row.get(15)?,
                corporate_facilities: row.get(16)?,
                finance_month: row.get(17)?,
                balance: row.get(18)?,
                transfer_budget_allocated: row.get(19)?,
                transfer_budget_remaining: row.get(20)?,
                wage_budget_weekly: row.get(21)?,
                wage_payroll_weekly: row.get(22)?,
                net: row.get(23)?,
                player_count: row.get::<_, i64>(24)? as u64,
                average_age: row.get(25)?,
                average_ca: row.get(26)?,
                total_value: row.get(27)?,
                total_wage: row.get(28)?,
                stadium_count: row.get::<_, i64>(29)? as u64,
                stadium_capacity: row.get(30)?,
            })
        })?;
        let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(ClubPage {
            total,
            page,
            page_size,
            rows,
        })
    }

    pub fn get_club(&self, save_id: i64, uid: u32) -> Result<Option<ClubDetail>> {
        let row = self
            .conn
            .query_row(
                "SELECT uid, unique_id, COALESCE(name, ''), COALESCE(short_name, ''),
                        nation_id, nation_name, fa_nation_id, fa_nation_name, city_id, gender,
                        reputation, last_league_position, league_competition_id, league_name,
                        parent_club_uid, parent_club_name, corporate_facilities,
                        training_facilities, youth_facilities, junior_coaching, youth_recruitment,
                        youth_level, teams_json
                 FROM clubs WHERE save_id = ?1 AND uid = ?2",
                params![save_id, uid],
                |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, Option<u32>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, u32>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, u32>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<u32>>(8)?,
                        row.get::<_, Option<u8>>(9)?,
                        row.get::<_, Option<u16>>(10)?,
                        row.get::<_, Option<u8>>(11)?,
                        row.get::<_, Option<u32>>(12)?,
                        row.get::<_, Option<String>>(13)?,
                        row.get::<_, Option<u32>>(14)?,
                        row.get::<_, Option<String>>(15)?,
                        row.get::<_, Option<u8>>(16)?,
                        row.get::<_, Option<u8>>(17)?,
                        row.get::<_, Option<u8>>(18)?,
                        row.get::<_, Option<u8>>(19)?,
                        row.get::<_, Option<u8>>(20)?,
                        row.get::<_, Option<u8>>(21)?,
                        row.get::<_, Option<String>>(22)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            uid,
            unique_id,
            name,
            short_name,
            nation_id,
            nation_name,
            fa_nation_id,
            fa_nation_name,
            city_id,
            gender,
            reputation,
            last_league_position,
            league_competition_id,
            league_name,
            parent_club_uid,
            parent_club_name,
            corporate_facilities,
            training_facilities,
            youth_facilities,
            junior_coaching,
            youth_recruitment,
            youth_level,
            teams_json,
        )) = row
        else {
            return Ok(None);
        };
        let club = Club {
            uid,
            unique_id,
            name,
            short_name,
            nation_id,
            nation_name,
            fa_nation_id,
            fa_nation_name,
            city_id,
            gender,
            reputation,
            last_league_position,
            league_competition_id,
            league_name,
            parent_club_uid,
            parent_club_name,
            corporate_facilities,
            training_facilities,
            youth_facilities,
            junior_coaching,
            youth_recruitment,
            youth_level,
            teams: teams_json
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default(),
        };
        let finance_months = self.club_finance_months(save_id, uid)?;
        let stadiums = self.club_stadiums(save_id, uid)?;
        let child_clubs = self.club_children(save_id, uid)?;
        Ok(Some(ClubDetail {
            club,
            finance_months,
            stadiums,
            child_clubs,
        }))
    }

    fn club_finance_months(&self, save_id: i64, club_uid: u32) -> Result<Vec<FinanceMonth>> {
        let mut statement = self.conn.prepare(
            "SELECT f.club_uid, COALESCE(c.name, ''), f.month, f.balance,
                    f.transfer_budget_allocated, f.transfer_budget_remaining,
                    f.wage_budget_weekly, f.wage_payroll_weekly, f.income_excluding_transfers,
                    f.net_transfers, f.wage_bill, f.net, f.expenditure_excluding_transfers,
                    f.total_income, f.total_expenditure
             FROM club_finances f
             LEFT JOIN clubs c ON c.save_id = f.save_id AND c.uid = f.club_uid
             WHERE f.save_id = ?1 AND f.club_uid = ?2
             ORDER BY f.month",
        )?;
        let rows = statement.query_map(params![save_id, club_uid], |row| {
            Ok(FinanceMonth {
                club_uid: row.get(0)?,
                club_name: row.get(1)?,
                month: row.get(2)?,
                balance: row.get(3)?,
                transfer_budget_allocated: row.get(4)?,
                transfer_budget_remaining: row.get(5)?,
                wage_budget_weekly: row.get(6)?,
                wage_payroll_weekly: row.get(7)?,
                income_excluding_transfers: row.get(8)?,
                net_transfers: row.get(9)?,
                wage_bill: row.get(10)?,
                net: row.get(11)?,
                expenditure_excluding_transfers: row.get(12)?,
                total_income: row.get(13)?,
                total_expenditure: row.get(14)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    fn club_stadiums(&self, save_id: i64, club_uid: u32) -> Result<Vec<Stadium>> {
        let mut statement = self.conn.prepare(
            "SELECT uid, name, all_seater_capacity, expansion_capacity, capacity,
                    owner_club_uid, owner_club_name, pitch_length_dm, pitch_width_dm,
                    pitch_min_length_dm, pitch_min_width_dm, pitch_max_length_dm,
                    pitch_max_width_dm, built_date, rebuilt_date
             FROM stadiums WHERE save_id = ?1 AND owner_club_uid = ?2
             ORDER BY all_seater_capacity DESC, uid",
        )?;
        let rows = statement.query_map(params![save_id, club_uid], |row| {
            Ok(Stadium {
                uid: row.get(0)?,
                name: row.get(1)?,
                all_seater_capacity: row.get(2)?,
                expansion_capacity: row.get(3)?,
                capacity: row.get(4)?,
                owner_club_uid: row.get(5)?,
                owner_club_name: row.get(6)?,
                pitch_length_dm: row.get(7)?,
                pitch_width_dm: row.get(8)?,
                pitch_min_length_dm: row.get(9)?,
                pitch_min_width_dm: row.get(10)?,
                pitch_max_length_dm: row.get(11)?,
                pitch_max_width_dm: row.get(12)?,
                built_date: row.get(13)?,
                rebuilt_date: row.get(14)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    fn club_children(&self, save_id: i64, parent_uid: u32) -> Result<Vec<ClubLink>> {
        let mut statement = self.conn.prepare(
            "SELECT uid, COALESCE(name, ''), COALESCE(short_name, '')
             FROM clubs WHERE save_id = ?1 AND parent_club_uid = ?2 ORDER BY name",
        )?;
        let rows = statement.query_map(params![save_id, parent_uid], |row| {
            Ok(ClubLink {
                uid: row.get(0)?,
                name: row.get(1)?,
                short_name: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    pub fn club_filter_options(&self, save_id: i64) -> Result<ClubFilterOptions> {
        let mut nation_stmt = self.conn.prepare(
            "SELECT nation_id, COALESCE(MAX(nation_name), 'Nation #' || nation_id), COUNT(*)
             FROM clubs WHERE save_id = ?1 GROUP BY nation_id
             ORDER BY COUNT(*) DESC, nation_id",
        )?;
        let nations = nation_stmt
            .query_map(params![save_id], |row| {
                Ok(FilterOption {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    count: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut gender_stmt = self.conn.prepare(
            "SELECT gender, CASE gender WHEN 0 THEN 'Men' WHEN 1 THEN 'Women' ELSE 'Unknown' END,
                    COUNT(*)
             FROM clubs WHERE save_id = ?1 GROUP BY gender ORDER BY gender",
        )?;
        let genders = gender_stmt
            .query_map(params![save_id], |row| {
                Ok(FilterOption {
                    id: row.get::<_, Option<u8>>(0)?.unwrap_or(255) as u32,
                    name: row.get(1)?,
                    count: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let ranges = self.conn.query_row(
            "SELECT MIN(reputation), MAX(reputation), MIN(balance), MAX(balance),
                    MIN(transfer_budget_remaining), MAX(transfer_budget_remaining),
                    MIN(wage_budget_weekly), MAX(wage_budget_weekly)
             FROM clubs WHERE save_id = ?1",
            params![save_id],
            |row| {
                Ok((
                    row.get::<_, Option<u16>>(0)?,
                    row.get::<_, Option<u16>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                ))
            },
        )?;
        let clubs_with_finances = self.conn.query_row(
            "SELECT COUNT(*) FROM clubs WHERE save_id = ?1 AND finance_month IS NOT NULL",
            params![save_id],
            |row| row.get::<_, u64>(0),
        )?;
        let clubs_with_stadiums = self.conn.query_row(
            "SELECT COUNT(DISTINCT owner_club_uid) FROM stadiums
             WHERE save_id = ?1 AND owner_club_uid IS NOT NULL",
            params![save_id],
            |row| row.get::<_, u64>(0),
        )?;
        Ok(ClubFilterOptions {
            nations,
            genders,
            min_reputation: ranges.0,
            max_reputation: ranges.1,
            min_balance: ranges.2,
            max_balance: ranges.3,
            min_transfer_budget: ranges.4,
            max_transfer_budget: ranges.5,
            min_wage_budget: ranges.6,
            max_wage_budget: ranges.7,
            clubs_with_finances,
            clubs_with_stadiums,
        })
    }

    pub fn filter_options(&self, save_id: i64) -> Result<FilterOptions> {
        self.filter_options_with_query(save_id, None)
    }

    pub fn filter_options_with_query(
        &self,
        save_id: i64,
        query: Option<&PlayerQuery>,
    ) -> Result<FilterOptions> {
        let base = query.cloned().unwrap_or_else(|| PlayerQuery {
            save_id,
            ..PlayerQuery::default()
        });
        let mut base = base;
        base.save_id = save_id;

        let mut club_query = base.clone();
        club_query.club_uids.clear();
        let clubs = self.facet_counts(
            &club_query,
            "p.club_uid",
            "MAX(p.club_name)",
            Some("p.club_uid IS NOT NULL"),
            "MAX(p.club_name) COLLATE NOCASE",
        )?;

        let mut nation_query = base.clone();
        nation_query.nation_ids.clear();
        let nations = self.facet_counts(
            &nation_query,
            "p.nation_id",
            "COALESCE(MAX(p.nation_name), '国家 #' || p.nation_id)",
            Some("p.nation_id IS NOT NULL"),
            "COUNT(*) DESC, p.nation_id",
        )?;

        let mut gender_query = base.clone();
        gender_query.genders.clear();
        let mut genders = self.facet_counts(
            &gender_query,
            "p.gender",
            "CASE p.gender WHEN 0 THEN '男子' WHEN 1 THEN '女子' ELSE '未知' END",
            Some("p.gender IS NOT NULL"),
            "p.gender",
        )?;
        for option in &mut genders {
            option.name = match option.id {
                0 => "男子".to_owned(),
                1 => "女子".to_owned(),
                _ => "未知".to_owned(),
            };
        }

        let mut status_query = base.clone();
        status_query.availability = None;
        status_query.on_loan = None;
        let availability = self.status_counts(&status_query)?;

        let mut listing_query = base.clone();
        listing_query.listings.clear();
        let listing = self.listing_counts(&listing_query)?;

        let ranges = self.conn.query_row(
            "SELECT MIN(age), MAX(age), MIN(ca), MAX(ca), MIN(pa), MAX(pa),
                    MIN(transfer_value), MAX(transfer_value)
             FROM players WHERE save_id = ?1",
            params![save_id],
            |row| {
                Ok((
                    row.get::<_, Option<i32>>(0)?,
                    row.get::<_, Option<i32>>(1)?,
                    row.get::<_, Option<u16>>(2)?,
                    row.get::<_, Option<u16>>(3)?,
                    row.get::<_, Option<u16>>(4)?,
                    row.get::<_, Option<u16>>(5)?,
                    row.get::<_, Option<u32>>(6)?,
                    row.get::<_, Option<u32>>(7)?,
                ))
            },
        )?;
        let (min_wage, max_wage) = self.conn.query_row(
            "SELECT MIN(wage), MAX(wage) FROM contracts WHERE save_id = ?1",
            params![save_id],
            |row| Ok((row.get::<_, Option<u32>>(0)?, row.get::<_, Option<u32>>(1)?)),
        )?;

        Ok(FilterOptions {
            clubs,
            nations,
            genders,
            availability,
            listing,
            min_age: ranges.0,
            max_age: ranges.1,
            min_ca: ranges.2,
            max_ca: ranges.3,
            min_pa: ranges.4,
            max_pa: ranges.5,
            min_value: ranges.6,
            max_value: ranges.7,
            min_wage,
            max_wage,
        })
    }

    fn facet_counts(
        &self,
        query: &PlayerQuery,
        group: &str,
        name: &str,
        extra: Option<&str>,
        order: &str,
    ) -> Result<Vec<FilterOption>> {
        let mut params: Vec<Value> = Vec::new();
        let mut where_sql = build_where(query, &mut params)?;
        if let Some(extra) = extra {
            where_sql.push_str(" AND ");
            where_sql.push_str(extra);
        }
        let sql = format!(
            "SELECT {group}, {name}, COUNT(*)
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE {where_sql}
             GROUP BY {group}
             ORDER BY {order}"
        );
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(params.iter()), |row| {
            Ok(FilterOption {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }

    fn status_counts(&self, query: &PlayerQuery) -> Result<Vec<FilterOption>> {
        let mut params: Vec<Value> = Vec::new();
        let where_sql = build_where(query, &mut params)?;
        let sql = format!(
            "SELECT
               COALESCE(SUM(CASE WHEN p.injury_status = 'injured' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN p.suspension_count > 0 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN c.on_loan = 1 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN p.club_uid IS NULL THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN p.injury_status <> 'injured' AND p.suspension_count = 0 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN p.recent_injury IS NOT NULL THEN 1 ELSE 0 END), 0)
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE {where_sql}"
        );
        let counts = self
            .conn
            .query_row(&sql, params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })?;
        Ok(vec![
            FilterOption {
                id: 1,
                name: "伤病".to_owned(),
                count: counts.0 as u64,
            },
            FilterOption {
                id: 2,
                name: "停赛".to_owned(),
                count: counts.1 as u64,
            },
            FilterOption {
                id: 3,
                name: "租借".to_owned(),
                count: counts.2 as u64,
            },
            FilterOption {
                id: 4,
                name: "自由球员".to_owned(),
                count: counts.3 as u64,
            },
            FilterOption {
                id: 5,
                name: "可出战".to_owned(),
                count: counts.4 as u64,
            },
            FilterOption {
                id: 6,
                name: "伤病史".to_owned(),
                count: counts.5 as u64,
            },
        ])
    }

    fn listing_counts(&self, query: &PlayerQuery) -> Result<Vec<FilterOption>> {
        let mut params: Vec<Value> = Vec::new();
        let where_sql = build_where(query, &mut params)?;
        let sql = format!(
            "SELECT
               COALESCE(SUM(CASE WHEN p.transfer_listed = 1 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN p.loan_listed = 1 THEN 1 ELSE 0 END), 0)
             FROM players p
             LEFT JOIN contracts c ON c.save_id = p.save_id AND c.player_uid = p.uid
             WHERE {where_sql}"
        );
        let counts = self
            .conn
            .query_row(&sql, params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
        Ok(vec![
            FilterOption {
                id: 1,
                name: "挂牌出售".to_owned(),
                count: counts.0 as u64,
            },
            FilterOption {
                id: 2,
                name: "挂牌出租".to_owned(),
                count: counts.1 as u64,
            },
        ])
    }
}

fn migrate(conn: &Connection) -> Result<()> {
    const COLUMNS: &[(&str, &str, &str)] = &[
        ("saves", "save_name", "TEXT NOT NULL DEFAULT ''"),
        ("saves", "career_key", "TEXT NOT NULL DEFAULT ''"),
        ("players", "nation_name", "TEXT"),
        ("players", "league_competition_id", "INTEGER"),
        ("players", "league_name", "TEXT"),
        ("players", "gender", "INTEGER"),
        ("players", "best_role_id", "TEXT"),
        ("players", "best_role_name", "TEXT"),
        ("players", "best_role_score", "INTEGER"),
        ("players", "best_role_position", "TEXT"),
        ("players", "injury_status", "TEXT NOT NULL DEFAULT 'fit'"),
        ("players", "injury_until", "TEXT"),
        ("players", "recent_injury", "TEXT"),
        ("players", "suspension_count", "INTEGER NOT NULL DEFAULT 0"),
        ("players", "injuries_json", "TEXT NOT NULL DEFAULT '[]'"),
        ("players", "suspensions_json", "TEXT NOT NULL DEFAULT '[]'"),
        ("players", "transfer_listed", "INTEGER NOT NULL DEFAULT 0"),
        ("players", "loan_listed", "INTEGER NOT NULL DEFAULT 0"),
        ("clubs", "nation_name", "TEXT"),
        ("clubs", "league_competition_id", "INTEGER"),
        ("clubs", "league_name", "TEXT"),
        ("clubs", "fa_nation_name", "TEXT"),
        ("clubs", "gender", "INTEGER"),
        ("clubs", "unique_id", "INTEGER"),
        ("clubs", "city_id", "INTEGER"),
        ("clubs", "parent_club_name", "TEXT"),
        ("clubs", "corporate_facilities", "INTEGER"),
        ("clubs", "training_facilities", "INTEGER"),
        ("clubs", "youth_facilities", "INTEGER"),
        ("clubs", "junior_coaching", "INTEGER"),
        ("clubs", "youth_recruitment", "INTEGER"),
        ("clubs", "youth_level", "INTEGER"),
        ("clubs", "finance_month", "TEXT"),
        ("clubs", "balance", "INTEGER"),
        ("clubs", "transfer_budget_allocated", "INTEGER"),
        ("clubs", "transfer_budget_remaining", "INTEGER"),
        ("clubs", "wage_budget_weekly", "INTEGER"),
        ("clubs", "wage_payroll_weekly", "INTEGER"),
        ("clubs", "net", "INTEGER"),
    ];
    for (table, column, definition) in COLUMNS {
        if !table_exists(conn, table)? {
            continue;
        }
        let exists = conn
            .prepare(&format!("PRAGMA table_info({table})"))?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .iter()
            .any(|name| name == column);
        if !exists {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
    }
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn parse_json_column<T: serde::de::DeserializeOwned + Default>(text: String, _label: &str) -> T {
    serde_json::from_str(&text).unwrap_or_default()
}

fn build_club_where(query: &ClubQuery, params: &mut Vec<Value>) -> Result<String> {
    let mut clauses = vec!["c.save_id = ?".to_owned()];
    params.push(Value::Integer(query.save_id));

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        clauses.push(
            "(c.name LIKE ? COLLATE NOCASE OR c.short_name LIKE ? COLLATE NOCASE \
              OR CAST(c.uid AS TEXT) = ?)"
                .to_owned(),
        );
        let pattern = format!("%{search}%");
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
        params.push(Value::Text(search.to_owned()));
    }
    if !query.nation_ids.is_empty() {
        let placeholders = vec!["?"; query.nation_ids.len()].join(",");
        clauses.push(format!("c.nation_id IN ({placeholders})"));
        for id in &query.nation_ids {
            params.push(Value::Integer(*id as i64));
        }
    }
    if !query.genders.is_empty() {
        let placeholders = vec!["?"; query.genders.len()].join(",");
        clauses.push(format!("c.gender IN ({placeholders})"));
        for gender in &query.genders {
            params.push(Value::Integer(*gender as i64));
        }
    }
    add_range(
        query.reputation_min,
        query.reputation_max,
        "c.reputation",
        params,
        &mut clauses,
    );
    add_range(
        query.last_position_min,
        query.last_position_max,
        "c.last_league_position",
        params,
        &mut clauses,
    );
    if let Some(has_finances) = query.has_finances {
        clauses.push(if has_finances {
            "c.finance_month IS NOT NULL".to_owned()
        } else {
            "c.finance_month IS NULL".to_owned()
        });
    }
    add_range(
        query.balance_min,
        query.balance_max,
        "c.balance",
        params,
        &mut clauses,
    );
    add_range(
        query.transfer_budget_min,
        query.transfer_budget_max,
        "c.transfer_budget_remaining",
        params,
        &mut clauses,
    );
    add_range(
        query.wage_budget_min,
        query.wage_budget_max,
        "c.wage_budget_weekly",
        params,
        &mut clauses,
    );
    add_range(
        query.corporate_facilities_min,
        query.corporate_facilities_max,
        "c.corporate_facilities",
        params,
        &mut clauses,
    );
    add_range(
        query.training_facilities_min,
        query.training_facilities_max,
        "c.training_facilities",
        params,
        &mut clauses,
    );
    add_range(
        query.youth_facilities_min,
        query.youth_facilities_max,
        "c.youth_facilities",
        params,
        &mut clauses,
    );
    add_range(
        query.junior_coaching_min,
        query.junior_coaching_max,
        "c.junior_coaching",
        params,
        &mut clauses,
    );
    add_range(
        query.youth_recruitment_min,
        query.youth_recruitment_max,
        "c.youth_recruitment",
        params,
        &mut clauses,
    );
    add_range(
        query.player_count_min,
        query.player_count_max,
        "s.player_count",
        params,
        &mut clauses,
    );
    if let Some(min) = query.average_ca_min {
        clauses.push("s.average_ca >= ?".to_owned());
        params.push(Value::Real(min));
    }
    if let Some(max) = query.average_ca_max {
        clauses.push("s.average_ca <= ?".to_owned());
        params.push(Value::Real(max));
    }
    add_range(
        query.total_value_min,
        query.total_value_max,
        "s.total_value",
        params,
        &mut clauses,
    );
    if let Some(has_stadium) = query.has_stadium {
        clauses.push(if has_stadium {
            "COALESCE(st.stadium_count, 0) > 0".to_owned()
        } else {
            "COALESCE(st.stadium_count, 0) = 0".to_owned()
        });
    }
    if let Some(min) = query.stadium_capacity_min {
        clauses.push("COALESCE(st.stadium_capacity, 0) >= ?".to_owned());
        params.push(Value::Integer(min));
    }
    Ok(clauses.join(" AND "))
}

fn club_order_by(query: &ClubQuery) -> String {
    let mut rules: Vec<(String, bool)> = query
        .sort
        .iter()
        .filter(|rule| !rule.by.trim().is_empty())
        .map(|rule| (rule.by.clone(), rule.dir.eq_ignore_ascii_case("asc")))
        .collect();
    if rules.is_empty() {
        if let Some(by) = query.sort_by.as_deref().filter(|value| !value.is_empty()) {
            rules.push((
                by.to_owned(),
                query
                    .sort_dir
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case("asc")),
            ));
        }
    }
    if rules.is_empty() {
        rules.push(("reputation".to_owned(), false));
    }
    let mut parts = Vec::new();
    for (by, ascending) in rules {
        let expression = club_sort_expression(&by);
        let direction = if ascending { "ASC" } else { "DESC" };
        parts.push(format!("CASE WHEN {expression} IS NULL THEN 1 ELSE 0 END"));
        parts.push(format!("{expression} {direction}"));
    }
    parts.push("c.uid ASC".to_owned());
    format!("ORDER BY {}", parts.join(", "))
}

fn club_sort_expression(by: &str) -> String {
    match by {
        "name" => "c.name".to_owned(),
        "short_name" => "c.short_name".to_owned(),
        "nation" | "nation_id" => "c.nation_id".to_owned(),
        "league" | "league_name" => "c.league_name".to_owned(),
        "reputation" => "c.reputation".to_owned(),
        "last_position" | "last_league_position" => "c.last_league_position".to_owned(),
        "balance" => "c.balance".to_owned(),
        "transfer_budget" | "transfer_budget_remaining" => "c.transfer_budget_remaining".to_owned(),
        "wage_budget" | "wage_budget_weekly" => "c.wage_budget_weekly".to_owned(),
        "corporate_facilities" => "c.corporate_facilities".to_owned(),
        "player_count" => "s.player_count".to_owned(),
        "average_ca" => "s.average_ca".to_owned(),
        "total_value" => "s.total_value".to_owned(),
        "total_wage" => "s.total_wage".to_owned(),
        "stadium_capacity" => "st.stadium_capacity".to_owned(),
        "uid" => "c.uid".to_owned(),
        _ => "c.reputation".to_owned(),
    }
}

fn build_where(query: &PlayerQuery, params: &mut Vec<Value>) -> Result<String> {
    let mut clauses = vec!["p.save_id = ?".to_owned()];
    params.push(Value::Integer(query.save_id));

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        clauses.push(
            "(p.name LIKE ? COLLATE NOCASE OR p.club_name LIKE ? COLLATE NOCASE \
              OR p.common_name LIKE ? COLLATE NOCASE OR p.nation_name LIKE ? COLLATE NOCASE)"
                .to_owned(),
        );
        let pattern = format!("%{search}%");
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
    }
    if !query.uids.is_empty() {
        let placeholders = vec!["?"; query.uids.len()].join(",");
        clauses.push(format!("p.uid IN ({placeholders})"));
        for id in &query.uids {
            params.push(Value::Integer(*id as i64));
        }
    }
    if !query.club_uids.is_empty() {
        let placeholders = vec!["?"; query.club_uids.len()].join(",");
        clauses.push(format!("p.club_uid IN ({placeholders})"));
        for id in &query.club_uids {
            params.push(Value::Integer(*id as i64));
        }
    }
    if !query.nation_ids.is_empty() {
        let placeholders = vec!["?"; query.nation_ids.len()].join(",");
        clauses.push(format!("p.nation_id IN ({placeholders})"));
        for id in &query.nation_ids {
            params.push(Value::Integer(*id as i64));
        }
    }
    if !query.genders.is_empty() {
        let placeholders = vec!["?"; query.genders.len()].join(",");
        clauses.push(format!("p.gender IN ({placeholders})"));
        for gender in &query.genders {
            params.push(Value::Integer(*gender as i64));
        }
    }
    if let Some(availability) = query.availability.as_deref() {
        match availability {
            "injured" => clauses.push("p.injury_status = 'injured'".to_owned()),
            "injury_history" => clauses.push("p.recent_injury IS NOT NULL".to_owned()),
            "suspended" => clauses.push("p.suspension_count > 0".to_owned()),
            "loan" => clauses.push("c.on_loan = 1".to_owned()),
            "free" => clauses.push("p.club_uid IS NULL".to_owned()),
            "available" => {
                clauses.push("p.injury_status <> 'injured'".to_owned());
                clauses.push("p.suspension_count = 0".to_owned());
            }
            other => {
                return Err(Error::parse(format!(
                    "unknown availability filter {other:?}"
                )));
            }
        }
    }
    if !query.listings.is_empty() {
        let mut listing_clauses = Vec::with_capacity(query.listings.len());
        for listing in &query.listings {
            match listing.as_str() {
                "sale" => listing_clauses.push("p.transfer_listed = 1"),
                "loan" => listing_clauses.push("p.loan_listed = 1"),
                other => {
                    return Err(Error::parse(format!("unknown listing filter {other:?}")));
                }
            }
        }
        clauses.push(format!("({})", listing_clauses.join(" OR ")));
    }
    add_range(query.age_min, query.age_max, "p.age", params, &mut clauses);
    add_range(query.ca_min, query.ca_max, "p.ca", params, &mut clauses);
    add_range(query.pa_min, query.pa_max, "p.pa", params, &mut clauses);
    if let Some(gap) = query.pa_ca_gap_min {
        clauses.push("(p.pa - p.ca) >= ?".to_owned());
        params.push(Value::Integer(gap as i64));
    }
    if let Some(unknown) = query.pa_unknown {
        clauses.push(if unknown {
            "p.pa_range_code IS NOT NULL".to_owned()
        } else {
            "p.pa IS NOT NULL".to_owned()
        });
    }
    if let Some(generated) = query.generated {
        if generated {
            clauses.push("p.unique_id >= ?".to_owned());
            params.push(Value::Integer(GENERATED_UNIQUE_ID_MINIMUM as i64));
        } else {
            clauses.push("(p.unique_id IS NULL OR p.unique_id < ?)".to_owned());
            params.push(Value::Integer(GENERATED_UNIQUE_ID_MINIMUM as i64));
        }
    }
    let threshold = query.position_min.unwrap_or(15);
    for position in &query.positions {
        if !POSITION_KEYS.contains(&position.as_str()) {
            return Err(Error::parse(format!(
                "unknown position filter {position:?}"
            )));
        }
        clauses.push(format!(
            "json_extract(p.positions_json, '$.{position}') >= ?"
        ));
        params.push(Value::Integer(threshold as i64));
    }
    for filter in &query.attributes {
        let is_attribute = ATTRIBUTE_KEYS.contains(&filter.key.as_str());
        let is_personality = PERSONALITY_KEYS.contains(&filter.key.as_str());
        if !is_attribute && !is_personality {
            return Err(Error::parse(format!(
                "unknown attribute filter {:?}",
                filter.key
            )));
        }
        if !query.include_hidden && (is_personality || is_hidden_attribute(&filter.key)) {
            return Err(Error::parse(
                "hidden attribute filters require hidden attributes to be enabled",
            ));
        }
        let expression = if is_personality {
            format!("json_extract(p.personality_json, '$.{}')", filter.key)
        } else {
            format!("json_extract(p.attributes_json, '$.{}')", filter.key)
        };
        add_range(filter.min, filter.max, &expression, params, &mut clauses);
    }
    add_range(
        query.value_min,
        query.value_max,
        "p.transfer_value",
        params,
        &mut clauses,
    );
    add_range(
        query.wage_min,
        query.wage_max,
        "c.wage",
        params,
        &mut clauses,
    );
    if let Some(before) = &query.contract_end_before {
        clauses.push("c.end_date <= ?".to_owned());
        params.push(Value::Text(before.clone()));
    }
    if let Some(after) = &query.contract_end_after {
        clauses.push("c.end_date >= ?".to_owned());
        params.push(Value::Text(after.clone()));
    }
    if let Some(on_loan) = query.on_loan {
        clauses.push("c.on_loan = ?".to_owned());
        params.push(Value::Integer(if on_loan { 1 } else { 0 }));
    }
    Ok(clauses.join(" AND "))
}

fn add_range<T: Into<i64>>(
    min: Option<T>,
    max: Option<T>,
    expression: &str,
    params: &mut Vec<Value>,
    clauses: &mut Vec<String>,
) {
    if let Some(min) = min {
        clauses.push(format!("{expression} >= ?"));
        params.push(Value::Integer(min.into()));
    }
    if let Some(max) = max {
        clauses.push(format!("{expression} <= ?"));
        params.push(Value::Integer(max.into()));
    }
}

fn is_hidden_attribute(key: &str) -> bool {
    HIDDEN_ATTRIBUTE_KEYS.contains(&key)
}

fn redact_attributes(attributes: &mut Attributes) {
    for key in HIDDEN_ATTRIBUTE_KEYS {
        if let Some(index) = ATTRIBUTE_KEYS.iter().position(|name| *name == key) {
            attributes.values[index] = 0;
        }
    }
}

fn redact_player(player: &mut Player) {
    redact_attributes(&mut player.attributes);
    redact_attributes(&mut player.raw_attributes);
    player.personality = None;
}

fn order_by(query: &PlayerQuery) -> String {
    let mut rules: Vec<(String, bool)> = query
        .sort
        .iter()
        .filter(|rule| !rule.by.trim().is_empty())
        .map(|rule| (rule.by.clone(), rule.dir.eq_ignore_ascii_case("asc")))
        .collect();
    if rules.is_empty() {
        if let Some(by) = query.sort_by.as_deref().filter(|value| !value.is_empty()) {
            rules.push((
                by.to_owned(),
                query
                    .sort_dir
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case("asc")),
            ));
        }
    }
    if rules.is_empty() {
        rules.push(("ca".to_owned(), false));
    }
    let mut parts = Vec::new();
    for (by, ascending) in rules {
        let expression = sort_expression(&by);
        let direction = if ascending { "ASC" } else { "DESC" };
        parts.push(format!("CASE WHEN {expression} IS NULL THEN 1 ELSE 0 END"));
        parts.push(format!("{expression} {direction}"));
    }
    parts.push("p.uid ASC".to_owned());
    format!("ORDER BY {}", parts.join(", "))
}

fn sort_expression(by: &str) -> String {
    match by {
        "name" => "p.name".to_owned(),
        "age" => "p.age".to_owned(),
        "club" | "club_name" => "p.club_name".to_owned(),
        "league" | "league_name" => "p.league_name".to_owned(),
        "nation" | "nation_id" => "p.nation_id".to_owned(),
        "nation_name" => "p.nation_name".to_owned(),
        "gender" => "p.gender".to_owned(),
        "ca" => "p.ca".to_owned(),
        "pa" => "p.pa".to_owned(),
        "value" | "transfer_value" => "p.transfer_value".to_owned(),
        "wage" => "c.wage".to_owned(),
        "contract_end" => "c.end_date".to_owned(),
        "height" | "height_cm" => "p.height_cm".to_owned(),
        "left_foot" => "p.left_foot".to_owned(),
        "positions" => "p.positions_text".to_owned(),
        "uid" => "p.uid".to_owned(),
        "best_role" | "best_role_name" => "p.best_role_name".to_owned(),
        "role" | "best_role_score" => "p.best_role_score".to_owned(),
        "injury" | "injury_status" => "p.injury_status".to_owned(),
        "injury_until" => "p.injury_until".to_owned(),
        "suspensions" | "suspension_count" => "p.suspension_count".to_owned(),
        other => {
            if let Some(key) = other.strip_prefix("attr_")
                && ATTRIBUTE_KEYS.contains(&key)
            {
                return format!("json_extract(p.attributes_json, '$.{key}')");
            }
            "p.ca".to_owned()
        }
    }
}

const AGE_BANDS: [&str; 6] = ["≤17", "18–21", "22–25", "26–29", "30–33", "34+"];
const CA_BANDS: [&str; 6] = ["<100", "100–119", "120–139", "140–159", "160–179", "180+"];
const SQUAD_GROUPS: [&str; 4] = ["门将", "后卫", "中场", "前锋"];

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn age_band_label(age: i32) -> &'static str {
    match age {
        ..=17 => "≤17",
        18..=21 => "18–21",
        22..=25 => "22–25",
        26..=29 => "26–29",
        30..=33 => "30–33",
        _ => "34+",
    }
}

fn ca_band_label(ca: u16) -> &'static str {
    match ca {
        0..=99 => "<100",
        100..=119 => "100–119",
        120..=139 => "120–139",
        140..=159 => "140–159",
        160..=179 => "160–179",
        _ => "180+",
    }
}

fn distribution(
    labels: &[&'static str],
    counts: &HashMap<&'static str, u64>,
) -> Vec<DistributionBand> {
    labels
        .iter()
        .map(|label| DistributionBand {
            label: (*label).to_owned(),
            count: counts.get(label).copied().unwrap_or(0),
        })
        .collect()
}

fn contract_year(end: &str) -> Option<u16> {
    end.get(0..4)?.parse().ok()
}

fn contract_year_bands(years: HashMap<u16, u64>) -> Vec<DistributionBand> {
    let mut years: Vec<(u16, u64)> = years.into_iter().collect();
    years.sort_unstable_by_key(|(year, _)| *year);
    years
        .into_iter()
        .map(|(year, count)| DistributionBand {
            label: year.to_string(),
            count,
        })
        .collect()
}

fn career_key(parsed: &ParsedSave) -> String {
    let save_name = parsed.summary.save_name.trim();
    if !save_name.is_empty() {
        return save_name.to_lowercase();
    }
    let file_name = parsed.summary.file_name.trim();
    if !file_name.is_empty() {
        return file_name.trim_end_matches(".fm").to_lowercase();
    }
    parsed
        .summary
        .path
        .rsplit('/')
        .next()
        .unwrap_or("career")
        .trim_end_matches(".fm")
        .to_lowercase()
}

pub fn summary_from_parse(parsed: &ParsedSave) -> SaveSummary {
    parsed.summary.clone()
}

pub fn report_from_parse(parsed: &ParsedSave) -> ParseReport {
    parsed.report.clone()
}

pub fn club_count(clubs: &[Club]) -> usize {
    clubs.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn personality_filter(key: &str) -> AttributeFilter {
        AttributeFilter {
            key: key.to_owned(),
            min: Some(15),
            max: None,
        }
    }

    #[test]
    fn routes_personality_filters_to_the_personality_column() {
        let query = PlayerQuery {
            attributes: vec![personality_filter("professionalism")],
            include_hidden: true,
            ..PlayerQuery::default()
        };
        let mut params = Vec::new();
        let sql = build_where(&query, &mut params).expect("personality filter should be valid");
        assert!(sql.contains("json_extract(p.personality_json, '$.professionalism') >= ?"));
        assert_eq!(params.len(), 2);
        assert!(matches!(params[1], Value::Integer(15)));
    }

    #[test]
    fn requires_hidden_attributes_for_personality_filters() {
        let query = PlayerQuery {
            attributes: vec![personality_filter("ambition")],
            include_hidden: false,
            ..PlayerQuery::default()
        };
        let mut params = Vec::new();
        let error = build_where(&query, &mut params).expect_err("hidden filter should be gated");
        assert!(error.to_string().contains("hidden attribute filters"));
    }

    #[test]
    fn rejects_unknown_attribute_filters() {
        let query = PlayerQuery {
            attributes: vec![personality_filter("not_a_real_attribute")],
            include_hidden: true,
            ..PlayerQuery::default()
        };
        let mut params = Vec::new();
        assert!(build_where(&query, &mut params).is_err());
    }

    #[test]
    fn filters_generated_players_by_unique_id() {
        let query = PlayerQuery {
            generated: Some(true),
            ..PlayerQuery::default()
        };
        let mut params = Vec::new();
        let sql = build_where(&query, &mut params).expect("generated filter should be valid");
        assert!(sql.contains("p.unique_id >= ?"));
        assert!(matches!(
            params.last(),
            Some(Value::Integer(value)) if *value == GENERATED_UNIQUE_ID_MINIMUM as i64
        ));

        let query = PlayerQuery {
            generated: Some(false),
            ..PlayerQuery::default()
        };
        let mut params = Vec::new();
        let sql = build_where(&query, &mut params).expect("database filter should be valid");
        assert!(sql.contains("p.unique_id IS NULL OR p.unique_id < ?"));
    }

    #[test]
    fn value_finder_reads_the_cache_and_keeps_unknown_fees_out_by_default() {
        let store = Store::open_in_memory().expect("store");
        store
            .conn
            .execute_batch(
                "INSERT INTO saves
                   (id, path, file_name, file_size, mtime_ns, fingerprint, game, build,
                    parser_version, parsed_at)
                 VALUES (1, '/tmp/test.fm', 'test.fm', 1, 1, 'fp', 'FM27', '27.0.0', 'test', 'now');
                 INSERT INTO clubs (save_id, uid, name, short_name, reputation)
                 VALUES (1, 10, 'Test Club', 'Test', 5000);
                 INSERT INTO players
                   (save_id, uid, name, age, club_uid, club_name, league_name, ca, pa,
                    transfer_value, transfer_value_state, positions_json, positions_text,
                    attributes_json, raw_attributes_json, best_role_id, best_role_name,
                    best_role_score, best_role_position, data_json)
                 VALUES
                   (1, 1, 'Cheap', 24, 10, 'Test Club', 'Test League', 150, 160,
                    1000, '\"ok\"', '{}', 'MC', '{}', '{}', 'central_midfielder',
                    'Central Midfielder', 16, 'MC', '{}'),
                   (1, 2, 'Expensive', 24, 10, 'Test Club', 'Test League', 150, 160,
                    1000000, '\"ok\"', '{}', 'MC', '{}', '{}', 'central_midfielder',
                    'Central Midfielder', 16, 'MC', '{}'),
                   (1, 3, 'Unknown fee', 24, 10, 'Test Club', 'Test League', 150, 160,
                    0, '\"unset\"', '{}', 'MC', '{}', '{}', 'central_midfielder',
                    'Central Midfielder', 16, 'MC', '{}');
                 INSERT INTO contracts (save_id, player_uid, wage, end_date)
                 VALUES (1, 1, 100, '2028-06-30'),
                        (1, 2, 1000, '2028-06-30'),
                        (1, 3, 1000, '2028-06-30');",
            )
            .expect("seed");

        let request = crate::moneyball::ValueFinderRequest {
            save_id: 1,
            ca_min: Some(100),
            ..crate::moneyball::ValueFinderRequest::default()
        };
        let page = store.value_finder(&request).expect("value finder");
        assert_eq!(page.total, 2);
        assert_eq!(page.rows[0].name.as_deref(), Some("Cheap"));
        assert_eq!(page.meta.leagues, vec!["Test League".to_owned()]);

        let mut with_unknown = request;
        with_unknown.include_unknown_fee = true;
        let page = store.value_finder(&with_unknown).expect("value finder");
        assert_eq!(page.total, 3);
    }
}
