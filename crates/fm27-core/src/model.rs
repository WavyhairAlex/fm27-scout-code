use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};

use crate::layout::{ATTRIBUTE_KEYS, POSITION_KEYS};

#[derive(Debug, Clone, Copy)]
pub struct Attributes {
    pub values: [u8; 52],
}

impl Default for Attributes {
    fn default() -> Self {
        Self { values: [0; 52] }
    }
}

impl Attributes {
    pub fn get(&self, key: &str) -> Option<u8> {
        ATTRIBUTE_KEYS
            .iter()
            .position(|name| *name == key)
            .map(|index| self.values[index])
    }

    pub fn to_map(&self) -> BTreeMap<&'static str, u8> {
        ATTRIBUTE_KEYS
            .iter()
            .copied()
            .zip(self.values.iter().copied())
            .collect()
    }
}

impl<'de> Deserialize<'de> for Attributes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, u8>::deserialize(deserializer)?;
        let mut values = [0u8; 52];
        for (index, key) in ATTRIBUTE_KEYS.iter().enumerate() {
            if let Some(value) = map.get(*key) {
                values[index] = *value;
            }
        }
        Ok(Self { values })
    }
}

impl Serialize for Attributes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(52))?;
        for (key, value) in ATTRIBUTE_KEYS.iter().zip(self.values.iter()) {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Positions {
    pub values: [u8; 15],
}

impl Positions {
    pub fn get(&self, key: &str) -> Option<u8> {
        POSITION_KEYS
            .iter()
            .position(|name| *name == key)
            .map(|index| self.values[index])
    }
}

impl<'de> Deserialize<'de> for Positions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, u8>::deserialize(deserializer)?;
        let mut values = [0u8; 15];
        for (index, key) in POSITION_KEYS.iter().enumerate() {
            if let Some(value) = map.get(*key) {
                values[index] = *value;
            }
        }
        Ok(Self { values })
    }
}

impl Serialize for Positions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(15))?;
        for (key, value) in POSITION_KEYS.iter().zip(self.values.iter()) {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Personality {
    pub adaptability: u8,
    pub ambition: u8,
    pub loyalty: u8,
    pub pressure: u8,
    pub professionalism: u8,
    pub sportsmanship: u8,
    pub temperament: u8,
    pub controversy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferValueState {
    Ok,
    Unset,
    Placeholder,
    Zero,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    pub bucket: u8,
    pub home: u16,
    pub current: u16,
    pub world: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractChainEntry {
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub team_id: u32,
    pub wage: u32,
    pub start: Option<String>,
    pub end: Option<String>,
    pub has_terms: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub team_id: Option<u32>,
    pub wage: Option<u32>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub squad_status: Option<u8>,
    pub contract_type: Option<u8>,
    #[serde(default)]
    pub transfer_listed: bool,
    #[serde(default)]
    pub loan_listed: bool,
    pub clauses: Vec<Clause>,
    pub on_loan: Option<bool>,
    pub loan_parent_club_uid: Option<u32>,
    pub loan_parent_club_name: Option<String>,
    pub loan_start: Option<String>,
    pub loan_end: Option<String>,
    pub chain: Vec<ContractChainEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clause {
    pub kind: u16,
    pub parameter: Option<u16>,
    pub value: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub bit: u8,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleScore {
    pub id: String,
    pub name: String,
    pub group: String,
    pub score: u8,
    pub position: String,
    pub position_rating: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Injury {
    /// `current` for an expected return the save still tracks, `history` for a past injury.
    pub kind: String,
    pub date: Option<String>,
    pub expected_return: Option<String>,
    pub type_id: Option<u16>,
    pub type_name: Option<String>,
    pub cause: Option<u8>,
    pub severity: Option<u8>,
    pub team_id: Option<u32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suspension {
    pub scope: String,
    pub scope_code: u8,
    pub competition_id: Option<u16>,
    pub competition_name: Option<String>,
    pub nation_id: Option<u16>,
    pub nation_name: Option<String>,
    pub issued_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub uid: u32,
    pub unique_id: Option<u32>,
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub common_name: Option<String>,
    pub full_name: Option<String>,
    pub legal_name: Option<String>,
    pub birth_date: Option<String>,
    pub age: Option<i32>,
    pub nation_id: Option<u16>,
    #[serde(default)]
    pub nation_name: Option<String>,
    #[serde(default)]
    pub gender: Option<u8>,
    pub second_nation_ids: Vec<u32>,
    pub home_grown_nation_ids: Vec<u32>,
    pub home_grown_club_uids: Vec<Option<u32>>,
    pub home_grown_club_names: Vec<Option<String>>,
    pub height_cm: u8,
    pub ca: u16,
    pub pa: Option<i16>,
    pub pa_range_code: Option<i16>,
    pub reputation: Reputation,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub club_short_name: Option<String>,
    pub club_nation_id: Option<u32>,
    #[serde(default)]
    pub club_nation_name: Option<String>,
    pub club_fa_nation_id: Option<u32>,
    pub club_reputation: Option<u16>,
    pub club_last_league_position: Option<u8>,
    pub team_id: Option<u32>,
    pub team_slot: Option<u8>,
    pub club_join_date: Option<String>,
    pub natural_positions: Vec<String>,
    pub accomplished_positions: Vec<String>,
    pub personality: Option<Personality>,
    pub attributes: Attributes,
    pub raw_attributes: Attributes,
    pub left_foot: u8,
    pub right_foot: u8,
    pub raw_left_foot: u8,
    pub raw_right_foot: u8,
    pub positions: Positions,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: TransferValueState,
    #[serde(default)]
    pub transfer_listed: bool,
    #[serde(default)]
    pub loan_listed: bool,
    pub raw_condition: u16,
    pub raw_match_sharpness: u16,
    pub traits: Vec<Trait>,
    pub trait_bits: Option<u64>,
    #[serde(default)]
    pub best_role: Option<RoleScore>,
    #[serde(default)]
    pub injuries: Vec<Injury>,
    #[serde(default)]
    pub suspensions: Vec<Suspension>,
    pub on_loan: Option<bool>,
    pub loan_parent_club_uid: Option<u32>,
    pub loan_parent_club_name: Option<String>,
    pub loan_start: Option<String>,
    pub loan_end: Option<String>,
    pub contract: Option<Contract>,
}

impl Player {
    pub fn age_at(birth: NaiveDate, clock: NaiveDate) -> i32 {
        let mut age = clock.year() - birth.year();
        if (clock.month(), clock.day()) < (birth.month(), birth.day()) {
            age -= 1;
        }
        age
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Club {
    pub uid: u32,
    #[serde(default)]
    pub unique_id: Option<u32>,
    pub name: String,
    pub short_name: String,
    pub nation_id: u32,
    pub nation_name: Option<String>,
    pub fa_nation_id: u32,
    pub fa_nation_name: Option<String>,
    #[serde(default)]
    pub city_id: Option<u32>,
    pub gender: Option<u8>,
    pub reputation: Option<u16>,
    pub last_league_position: Option<u8>,
    pub parent_club_uid: Option<u32>,
    #[serde(default)]
    pub parent_club_name: Option<String>,
    #[serde(default)]
    pub corporate_facilities: Option<u8>,
    #[serde(default)]
    pub training_facilities: Option<u8>,
    #[serde(default)]
    pub youth_facilities: Option<u8>,
    #[serde(default)]
    pub junior_coaching: Option<u8>,
    #[serde(default)]
    pub youth_recruitment: Option<u8>,
    #[serde(default)]
    pub youth_academy: Option<u8>,
    pub teams: Vec<u32>,
}

/// One club's money in one month, read from the finance chain inside its club record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceMonth {
    pub club_uid: u32,
    pub club_name: String,
    pub month: String,
    pub balance: i64,
    pub transfer_budget_allocated: i64,
    pub transfer_budget_remaining: i64,
    pub wage_budget_weekly: i64,
    pub wage_payroll_weekly: i64,
    pub income_excluding_transfers: i64,
    pub net_transfers: i64,
    pub wage_bill: i64,
    pub net: i64,
    pub expenditure_excluding_transfers: i64,
    pub total_income: i64,
    pub total_expenditure: i64,
}

/// A ground from the save's stadium table. The save stores no direct club -> ground
/// pointer; ownership is a separate field, and home use would require the fixture
/// calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stadium {
    pub uid: u32,
    pub name: Option<String>,
    pub all_seater_capacity: u32,
    pub expansion_capacity: u32,
    pub capacity: Option<u32>,
    pub owner_club_uid: Option<u32>,
    pub owner_club_name: Option<String>,
    pub pitch_length_dm: u16,
    pub pitch_width_dm: u16,
    pub pitch_min_length_dm: u16,
    pub pitch_min_width_dm: u16,
    pub pitch_max_length_dm: u16,
    pub pitch_max_width_dm: u16,
    pub built_date: Option<String>,
    pub rebuilt_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubLink {
    pub uid: u32,
    pub name: String,
    pub short_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubDetail {
    pub club: Club,
    pub finance_months: Vec<FinanceMonth>,
    pub stadiums: Vec<Stadium>,
    pub child_clubs: Vec<ClubLink>,
}

/// One competition named by the save's stage table.
///
/// A save stores no competition name: `database_id` is the game's editor
/// database id and the key for the optional user-supplied name table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Competition {
    /// Competition id in the save's stage id space; suspension scope ids use this space.
    pub id: u32,
    pub database_id: Option<u32>,
    pub name: Option<String>,
    pub stage_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSummary {
    pub path: String,
    pub file_name: String,
    pub file_size: u64,
    pub mtime_ns: i64,
    pub fingerprint: String,
    pub save_name: String,
    pub game: String,
    pub build: String,
    pub game_date: Option<String>,
    pub db_version: String,
    pub section_count: usize,
    pub decompressed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseReport {
    pub save: SaveSummary,
    pub players: usize,
    pub clubs: usize,
    pub contracts: usize,
    pub competitions: usize,
    pub competition_stages: usize,
    pub competitions_with_database_id: usize,
    pub competitions_with_name: usize,
    pub club_finance_months: usize,
    pub clubs_with_finance: usize,
    pub stadiums: usize,
    pub clubs_with_corporate_facilities: usize,
    pub markerless_players: usize,
    pub players_with_person: usize,
    pub players_with_club: usize,
    pub players_with_contract: usize,
    pub elapsed_ms: u128,
    pub warnings: Vec<String>,
}
