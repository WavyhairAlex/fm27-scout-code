//! SQLite cache and query layer. The UI never sees the save bytes or the database path.

use std::collections::HashMap;
use std::path::Path;

use chrono::{Months, NaiveDate, Utc};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::{ATTRIBUTE_KEYS, POSITION_KEYS};
use crate::model::{
    Attributes, Club, Competition, Contract, ParseReport, Personality, Player, Positions,
    SaveSummary, Trait,
};
use crate::parse::ParsedSave;

const SCHEMA: &str = include_str!("../../../shared/schema.sql");

pub struct Store {
    conn: Connection,
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
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
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
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
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
                  transfer_listed, loan_listed, data_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
                         ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,
                         ?37,?38,?39,?40,?41,?42,?43,?44)",
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
            let mut club_stmt = transaction.prepare(
                "INSERT INTO clubs
                 (save_id, uid, name, short_name, nation_id, nation_name, fa_nation_id,
                  fa_nation_name, gender, reputation, last_league_position, parent_club_uid,
                  teams_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            )?;
            for club in &parsed.clubs {
                club_stmt.execute(params![
                    save_id,
                    club.uid,
                    club.name,
                    club.short_name,
                    club.nation_id,
                    club.nation_name,
                    club.fa_nation_id,
                    club.fa_nation_name,
                    club.gender,
                    club.reputation,
                    club.last_league_position,
                    club.parent_club_uid,
                    serde_json::to_string(&club.teams)?,
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

    pub fn delete_save(&mut self, save_id: i64) -> Result<()> {
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
            "SELECT p.uid, p.name, p.age, p.club_uid, p.club_name, p.nation_id, p.ca, p.pa,
                    p.pa_range_code, p.positions_text, p.positions_json, p.attributes_json,
                    p.personality_json, p.transfer_value, p.transfer_value_state, c.wage,
                    c.end_date, p.height_cm, p.left_foot, p.right_foot, c.on_loan, p.traits_json,
                    p.nation_name, p.gender, p.best_role_id, p.best_role_name,
                    p.best_role_score, p.best_role_position, p.injury_status, p.injury_until,
                    p.recent_injury, p.suspension_count, p.transfer_listed, p.loan_listed
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
                name: row.get(1)?,
                age: row.get(2)?,
                club_uid: row.get(3)?,
                club_name: row.get(4)?,
                nation_id: row.get(5)?,
                ca: row.get(6)?,
                pa: row.get(7)?,
                pa_range_code: row.get(8)?,
                positions_text: row.get(9)?,
                positions: parse_json_column(row.get::<_, String>(10)?, "positions"),
                attributes: parse_json_column(row.get::<_, String>(11)?, "attributes"),
                personality: row
                    .get::<_, Option<String>>(12)?
                    .and_then(|text| serde_json::from_str::<Personality>(&text).ok()),
                transfer_value: row.get(13)?,
                transfer_value_state: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|text| serde_json::from_str::<String>(&text).ok())
                    .unwrap_or_else(|| "ok".to_owned()),
                wage: row.get(15)?,
                contract_end: row.get(16)?,
                height_cm: row.get(17)?,
                left_foot: row.get(18)?,
                right_foot: row.get(19)?,
                on_loan: row.get(20)?,
                traits: row
                    .get::<_, Option<String>>(21)?
                    .and_then(|text| serde_json::from_str::<Vec<Trait>>(&text).ok())
                    .unwrap_or_default(),
                nation_name: row.get(22)?,
                gender: row.get(23)?,
                best_role_id: row.get(24)?,
                best_role_name: row.get(25)?,
                best_role_score: row.get(26)?,
                best_role_position: row.get(27)?,
                injury_status: row.get(28)?,
                injury_until: row.get(29)?,
                recent_injury: row.get(30)?,
                suspension_count: row.get(31)?,
                transfer_listed: row.get(32)?,
                loan_listed: row.get(33)?,
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
        ("clubs", "fa_nation_name", "TEXT"),
        ("clubs", "gender", "INTEGER"),
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
        if !ATTRIBUTE_KEYS.contains(&filter.key.as_str()) {
            return Err(Error::parse(format!(
                "unknown attribute filter {:?}",
                filter.key
            )));
        }
        if !query.include_hidden && is_hidden_attribute(&filter.key) {
            return Err(Error::parse(
                "hidden attribute filters require hidden attributes to be enabled",
            ));
        }
        let expression = format!("json_extract(p.attributes_json, '$.{}')", filter.key);
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

const HIDDEN_ATTRIBUTES: [&str; 5] = [
    "dirtiness",
    "consistency",
    "important_matches",
    "injury_proneness",
    "versatility",
];

fn is_hidden_attribute(key: &str) -> bool {
    HIDDEN_ATTRIBUTES.contains(&key)
}

fn redact_attributes(attributes: &mut Attributes) {
    for key in HIDDEN_ATTRIBUTES {
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
