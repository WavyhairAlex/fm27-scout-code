//! In-place editing of player records inside a decompressed `game_db` section.
//!
//! This module never writes a save itself. It mutates the `game_db` bytes and the
//! caller hands the result to [`SaveFile::write_with_replacements`], which writes a
//! new save file.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::clubs::ClubIndex;
use crate::container::{SaveFile, SaveWriteReport};
use crate::contracts::{ContractDecoder, ContractLocation};
use crate::error::{Error, Result};
use crate::layout::{
    ATTRIBUTE_KEYS, ContractLayout, FinanceLayout, GAME_DB_SCHEMA, PERSONALITY_KEYS, PersonLayout,
    PlayerLayout, decode_date, editable_builds, encode_date, is_ca_exempt_attribute,
    is_editable_build, read_i16, read_u8, read_u16, read_u32, read_u64, stored_attribute_index,
    trait_name,
};
use crate::metadata::SaveMetadata;
use crate::model::Player;
use crate::names::NamePools;
use crate::parse::closing_person_header;
use crate::persons::{ClubLookup, PersonDecoder, PersonLocation};
use crate::player_scan::{PlayerRecord, PlayerRecords};

#[derive(Debug, Clone, Serialize)]
pub struct EditChange {
    pub field: String,
    pub before: i64,
    pub after: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeEdit {
    pub key: String,
    pub value: u8,
}

/// A set of player edits shared by the CLI and the desktop app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerEditRequest {
    pub ca: Option<u16>,
    pub pa: Option<i16>,
    pub attributes: Vec<AttributeEdit>,
    pub personality: Vec<AttributeEdit>,
    pub traits: Vec<u8>,
    pub untraits: Vec<u8>,
    pub wage: Option<u32>,
    pub contract_end: Option<NaiveDate>,
}

impl PlayerEditRequest {
    pub fn is_empty(&self) -> bool {
        self.ca.is_none()
            && self.pa.is_none()
            && self.attributes.is_empty()
            && self.personality.is_empty()
            && self.traits.is_empty()
            && self.untraits.is_empty()
            && self.wage.is_none()
            && self.contract_end.is_none()
    }

    /// Warnings for the UI/CLI. CA drives CA-weighted attributes; hidden
    /// attributes and determination are exempt.
    pub fn warnings(&self) -> Vec<String> {
        let ca_weighted_attribute_edit = self
            .attributes
            .iter()
            .any(|edit| !is_ca_exempt_attribute(&edit.key));
        let mut warnings = Vec::new();
        if self.ca.is_some() {
            warnings.push(
                "FM recalculates visible attributes toward CA over roughly a month; hidden attributes, personality and traits are stable."
                    .to_owned(),
            );
        }
        if ca_weighted_attribute_edit {
            if self.ca.is_some() {
                warnings.push(
                    "FM may trim visible attributes to fit CA; key attributes are usually preserved near their edited values. Raise CA or edit fewer key attributes if the trim is too large."
                        .to_owned(),
                );
            } else {
                warnings.push(
                    "visible attribute edits without --ca will be trimmed toward the current CA; set --ca/--pa to keep them."
                        .to_owned(),
                );
            }
        }
        if self.wage.is_some() {
            warnings.push(
                "Weekly wage affects the club's wage bill, budget and financial rules; FM may recalculate those totals after the game advances."
                    .to_owned(),
            );
        }
        if self.contract_end.is_some() {
            warnings.push(
                "Contract expiry controls free agency, compensation and squad planning; only the player's current permanent contract is edited."
                    .to_owned(),
            );
        }
        warnings
    }

    pub fn apply(&self, editor: &mut PlayerEditor, uid: u32) -> Result<()> {
        if let Some(value) = self.ca {
            editor.set_ca(uid, value)?;
        }
        if let Some(value) = self.pa {
            editor.set_pa(uid, value)?;
        }
        for edit in &self.attributes {
            editor.set_attribute(uid, &edit.key, edit.value)?;
        }
        for edit in &self.personality {
            editor.set_personality(uid, &edit.key, edit.value)?;
        }
        for bit in &self.traits {
            editor.set_trait(uid, *bit, true)?;
        }
        for bit in &self.untraits {
            editor.set_trait(uid, *bit, false)?;
        }
        if let Some(value) = self.wage {
            editor.set_wage(uid, value)?;
        }
        if let Some(value) = self.contract_end {
            editor.set_contract_end(uid, value)?;
        }
        Ok(())
    }

    pub fn verify(&self, player: &Player) -> Result<()> {
        if let Some(expected) = self.ca {
            if player.ca != expected {
                return Err(Error::parse(format!(
                    "CA verification failed: expected {expected}, got {}",
                    player.ca
                )));
            }
        }
        if let Some(expected) = self.pa {
            if player.pa != Some(expected) {
                return Err(Error::parse(format!(
                    "PA verification failed: expected {expected}, got {:?}",
                    player.pa
                )));
            }
        }
        for edit in &self.attributes {
            if player.raw_attributes.get(&edit.key) != Some(edit.value) {
                return Err(Error::parse(format!(
                    "attribute {} verification failed: expected {}, got {:?}",
                    edit.key,
                    edit.value,
                    player.raw_attributes.get(&edit.key)
                )));
            }
        }
        for edit in &self.personality {
            if personality_value(player, &edit.key) != Some(edit.value) {
                return Err(Error::parse(format!(
                    "personality {} verification failed: expected {}",
                    edit.key, edit.value
                )));
            }
        }
        for bit in &self.traits {
            if player
                .trait_bits
                .is_none_or(|bits| bits & (1u64 << *bit) == 0)
            {
                return Err(Error::parse(format!(
                    "trait {bit} verification failed: it is not set"
                )));
            }
        }
        for bit in &self.untraits {
            if player
                .trait_bits
                .is_some_and(|bits| bits & (1u64 << *bit) != 0)
            {
                return Err(Error::parse(format!(
                    "trait {bit} verification failed: it is still set"
                )));
            }
        }
        if let Some(expected) = self.wage {
            let actual = player.contract.as_ref().and_then(|contract| contract.wage);
            if actual != Some(expected) {
                return Err(Error::parse(format!(
                    "wage verification failed: expected {expected}, got {actual:?}"
                )));
            }
        }
        if let Some(expected) = self.contract_end {
            let expected_text = expected.to_string();
            let actual = player
                .contract
                .as_ref()
                .and_then(|contract| contract.end.as_deref());
            if actual != Some(expected_text.as_str()) {
                return Err(Error::parse(format!(
                    "contract end verification failed: expected {expected_text}, got {actual:?}"
                )));
            }
        }
        Ok(())
    }
}

fn personality_value(player: &Player, key: &str) -> Option<u8> {
    let personality = player.personality?;
    match key {
        "adaptability" => Some(personality.adaptability),
        "ambition" => Some(personality.ambition),
        "loyalty" => Some(personality.loyalty),
        "pressure" => Some(personality.pressure),
        "professionalism" => Some(personality.professionalism),
        "sportsmanship" => Some(personality.sportsmanship),
        "temperament" => Some(personality.temperament),
        "controversy" => Some(personality.controversy),
        _ => None,
    }
}

/// A mutable view over one save's `game_db` bytes.
pub struct PlayerEditor {
    game_db: Vec<u8>,
    records: PlayerRecords,
    names: NamePools,
    clock: NaiveDate,
    club_names: HashMap<u32, String>,
    club_by_team: HashMap<u32, (u32, u8)>,
    person_locations: HashMap<u32, PersonLocation>,
    edited_uids: HashSet<u32>,
    changes: Vec<EditChange>,
}

impl PlayerEditor {
    /// Open a save for editing. The build and `game_db` schema are checked so a
    /// future FM27 build with different offsets fails instead of guessing.
    pub fn open(save: &SaveFile, metadata: &SaveMetadata) -> Result<Self> {
        if !is_editable_build(&metadata.build) {
            return Err(Error::Unsupported(format!(
                "editing supports builds {}; this save was written by {}",
                editable_builds(),
                metadata.build
            )));
        }
        let game_db = save.read_section("game_db")?;
        let schema =
            read_u16(&game_db, 6).ok_or_else(|| Error::parse("game_db has no schema header"))?;
        if u32::from(schema) != GAME_DB_SCHEMA {
            return Err(Error::Unsupported(format!(
                "editing supports game_db schema {GAME_DB_SCHEMA}; this save uses {schema}"
            )));
        }
        let names = NamePools::locate(&game_db)?;
        let records = PlayerRecords::locate(&game_db, names.end_offset)?;
        let clubs = ClubIndex::read(&game_db)?;
        let clock = metadata
            .game_date
            .as_deref()
            .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        Ok(Self {
            game_db,
            records,
            names,
            clock,
            club_names: clubs.name_by_uid,
            club_by_team: clubs.fielding_team_to_club,
            person_locations: HashMap::new(),
            edited_uids: HashSet::new(),
            changes: Vec::new(),
        })
    }

    pub fn game_db(&self) -> &[u8] {
        &self.game_db
    }

    pub fn into_game_db(self) -> Vec<u8> {
        self.game_db
    }

    pub fn changes(&self) -> &[EditChange] {
        &self.changes
    }

    pub fn record_for_uid(&self, uid: u32) -> Option<PlayerRecord> {
        self.records
            .by_uid
            .get(&uid)
            .map(|&position| self.records.records[position])
    }

    /// Locate the contract currently shown for this player. Refuses records
    /// without terms, loan-only spells and expired contracts.
    pub fn current_contract_location(&self, uid: u32) -> Result<ContractLocation> {
        let record = self.require_record(uid)?;
        let position = *self
            .records
            .by_uid
            .get(&uid)
            .ok_or_else(|| Error::parse(format!("player uid {uid} was not found")))?;
        let window_end = self.records.window_end(position, self.game_db.len());
        let is_last = position + 1 == self.records.records.len();
        let team_id = read_u32(&self.game_db, record.offset + PlayerLayout::TEAM_ID_OFFSET)
            .unwrap_or(u32::MAX);
        let player_club_uid = self
            .club_by_team
            .get(&team_id)
            .map(|(club_uid, _)| *club_uid);
        let decoder = ContractDecoder {
            club_names: &self.club_names,
            club_by_team: &self.club_by_team,
            clock: self.clock,
        };
        let location = decoder
            .locate_in_effect(
                &self.game_db,
                record.offset,
                window_end,
                is_last,
                record.pindex,
                player_club_uid,
            )
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "player uid {uid} has no current permanent contract with terms"
                ))
            })?;
        if location.is_expired(self.clock) {
            return Err(Error::Unsupported(format!(
                "player uid {uid} has an expired contract (ended {}); only a current contract can be edited",
                location
                    .end
                    .map(|date| date.to_string())
                    .unwrap_or_default()
            )));
        }
        Ok(location)
    }

    /// Set the weekly wage on the current permanent contract. The value is in
    /// the save's base currency, not a converted display currency.
    pub fn set_wage(&mut self, uid: u32, value: u32) -> Result<EditChange> {
        if value > FinanceLayout::WEEKLY_MAXIMUM {
            return Err(Error::Unsupported(format!(
                "weekly wage must be at most {}, got {value}",
                FinanceLayout::WEEKLY_MAXIMUM
            )));
        }
        let location = self.current_contract_location(uid)?;
        let at = location.tag_offset + ContractLayout::WAGE_OFFSET;
        let before = read_u32(&self.game_db, at)
            .ok_or_else(|| Error::parse("the contract record ends before its wage field"))?;
        self.game_db[at..at + 4].copy_from_slice(&value.to_le_bytes());
        self.edited_uids.insert(uid);
        Ok(self.push_change("contract.wage", before as i64, value as i64))
    }

    /// Set the end date on the current permanent contract. The date must be
    /// after the contract start and not before the in-game date.
    pub fn set_contract_end(&mut self, uid: u32, value: NaiveDate) -> Result<EditChange> {
        let location = self.current_contract_location(uid)?;
        let start = location
            .start
            .ok_or_else(|| Error::parse("the current contract has no start date"))?;
        if value <= start {
            return Err(Error::Unsupported(format!(
                "contract end {value} must be after the contract start {start}"
            )));
        }
        if value < self.clock {
            return Err(Error::Unsupported(format!(
                "contract end {value} is before the in-game date {}",
                self.clock
            )));
        }
        let at = location.tail_offset + ContractLayout::TAIL_END_OFFSET;
        let old_packed = read_u16(&self.game_db, at)
            .ok_or_else(|| Error::parse("the contract tail ends before its end date"))?;
        let old_end = decode_date(&self.game_db, at);
        let encoded = encode_date(value, old_packed).ok_or_else(|| {
            Error::Unsupported(format!(
                "contract end {value} is outside the supported date range"
            ))
        })?;
        self.game_db[at..at + 4].copy_from_slice(&encoded);
        self.edited_uids.insert(uid);
        Ok(self.push_change_display(
            "contract.end",
            old_end.map(compact_date).unwrap_or(0),
            compact_date(value),
            old_end.map(|date| date.to_string()),
            Some(value.to_string()),
        ))
    }

    pub fn set_ca(&mut self, uid: u32, value: u16) -> Result<EditChange> {
        if !(1..=200).contains(&value) {
            return Err(Error::Unsupported(format!(
                "CA must be between 1 and 200, got {value}"
            )));
        }
        let record = self.require_record(uid)?;
        let at = record.offset + PlayerLayout::CA_OFFSET;
        let before = read_u16(&self.game_db, at)
            .ok_or_else(|| Error::parse("the player record ends before its CA field"))?;
        self.game_db[at..at + 2].copy_from_slice(&value.to_le_bytes());
        self.edited_uids.insert(uid);
        Ok(self.push_change("ca", before as i64, value as i64))
    }

    pub fn set_pa(&mut self, uid: u32, value: i16) -> Result<EditChange> {
        if !(1..=200).contains(&value) {
            return Err(Error::Unsupported(format!(
                "PA must be between 1 and 200; negative random-range codes are not supported, got {value}"
            )));
        }
        let record = self.require_record(uid)?;
        let at = record.offset + PlayerLayout::PA_OFFSET;
        let before = read_i16(&self.game_db, at)
            .ok_or_else(|| Error::parse("the player record ends before its PA field"))?;
        self.game_db[at..at + 2].copy_from_slice(&value.to_le_bytes());
        self.edited_uids.insert(uid);
        Ok(self.push_change("pa", before as i64, value as i64))
    }

    /// Set a raw attribute byte (1..=100). This covers the five hidden attributes
    /// (`consistency`, `important_matches`, `injury_proneness`, `versatility`,
    /// `dirtiness`) as well as the visible attributes.
    pub fn set_attribute(&mut self, uid: u32, key: &str, value: u8) -> Result<EditChange> {
        if !(1..=100).contains(&value) {
            return Err(Error::Unsupported(format!(
                "attribute {key:?} must be between 1 and 100, got {value}"
            )));
        }
        let stored_index = stored_attribute_index(key).ok_or_else(|| {
            Error::Unsupported(format!(
                "unknown attribute {key:?}; known keys: {}",
                ATTRIBUTE_KEYS.join(", ")
            ))
        })?;
        let record = self.require_record(uid)?;
        let at = record.offset + PlayerLayout::ATTRIBUTES_OFFSET + stored_index;
        let before = read_u8(&self.game_db, at)
            .ok_or_else(|| Error::parse("the player record ends before its attributes"))?;
        self.game_db[at] = value;
        self.edited_uids.insert(uid);
        Ok(self.push_change(format!("attribute.{key}"), before as i64, value as i64))
    }

    /// Set one of the eight personality bytes (1..=20).
    pub fn set_personality(&mut self, uid: u32, key: &str, value: u8) -> Result<EditChange> {
        if !(1..=20).contains(&value) {
            return Err(Error::Unsupported(format!(
                "personality {key:?} must be between 1 and 20, got {value}"
            )));
        }
        let index = PERSONALITY_KEYS
            .iter()
            .position(|name| *name == key)
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "unknown personality key {key:?}; known keys: {}",
                    PERSONALITY_KEYS.join(", ")
                ))
            })?;
        let location = self.person_location(uid)?;
        let at = location.birth_offset + PersonLayout::PERSONALITY_OFFSET_FROM_BIRTH + index;
        let before = read_u8(&self.game_db, at)
            .ok_or_else(|| Error::parse("the person block ends before its personality bytes"))?;
        self.game_db[at] = value;
        self.edited_uids.insert(uid);
        Ok(self.push_change(format!("personality.{key}"), before as i64, value as i64))
    }

    pub fn set_trait(&mut self, uid: u32, bit: u8, enabled: bool) -> Result<EditChange> {
        if bit > 63 {
            return Err(Error::Unsupported(format!(
                "trait bit must be between 0 and 63, got {bit}"
            )));
        }
        if enabled && trait_name(bit).is_none() {
            return Err(Error::Unsupported(format!(
                "trait bit {bit} has no known name in this build"
            )));
        }
        let location = self.person_location(uid)?;
        let at = location
            .block_start
            .checked_sub(8)
            .ok_or_else(|| Error::parse("the person block has no trait word"))?;
        let before = read_u64(&self.game_db, at)
            .ok_or_else(|| Error::parse("the person block ends before its trait word"))?;
        let mask = 1u64 << bit;
        let after = if enabled {
            before | mask
        } else {
            before & !mask
        };
        self.game_db[at..at + 8].copy_from_slice(&after.to_le_bytes());
        self.edited_uids.insert(uid);
        Ok(self.push_change(
            format!("trait.{bit}"),
            if before & mask != 0 { 1 } else { 0 },
            if after & mask != 0 { 1 } else { 0 },
        ))
    }

    /// Reject CA/PA combinations the game is likely to clamp or rewrite.
    pub fn validate(&self) -> Result<()> {
        for uid in &self.edited_uids {
            let record = self.require_record(*uid)?;
            let ca = read_u16(&self.game_db, record.offset + PlayerLayout::CA_OFFSET)
                .ok_or_else(|| Error::parse("the player record ends before its CA field"))?;
            let pa = read_i16(&self.game_db, record.offset + PlayerLayout::PA_OFFSET)
                .ok_or_else(|| Error::parse("the player record ends before its PA field"))?;
            if ca > 200 {
                return Err(Error::Unsupported(format!(
                    "player uid {uid} has CA {ca}; the supported maximum is 200"
                )));
            }
            if pa > 200 {
                return Err(Error::Unsupported(format!(
                    "player uid {uid} has PA {pa}; the supported maximum is 200"
                )));
            }
            if pa >= 0 && ca > pa as u16 {
                return Err(Error::Unsupported(format!(
                    "player uid {uid} has CA {ca} above PA {pa}; lower CA or raise PA"
                )));
            }
        }
        Ok(())
    }

    /// Write a new save with the edited `game_db`. The source save is never touched.
    pub fn write(
        &self,
        save: &SaveFile,
        output: impl AsRef<Path>,
        compression_level: i32,
    ) -> Result<SaveWriteReport> {
        self.write_as(save, output, compression_level, None)
    }

    /// Write a new save and optionally give it a new internal save name.
    pub fn write_as(
        &self,
        save: &SaveFile,
        output: impl AsRef<Path>,
        compression_level: i32,
        save_name: Option<&str>,
    ) -> Result<SaveWriteReport> {
        save.write_with_replacements_and_name(
            &[("game_db", self.game_db.as_slice())],
            output,
            compression_level,
            save_name,
        )
    }

    fn require_record(&self, uid: u32) -> Result<PlayerRecord> {
        self.record_for_uid(uid)
            .ok_or_else(|| Error::parse(format!("player uid {uid} was not found")))
    }

    fn person_location(&mut self, uid: u32) -> Result<PersonLocation> {
        if let Some(location) = self.person_locations.get(&uid) {
            return Ok(*location);
        }
        let position = *self
            .records
            .by_uid
            .get(&uid)
            .ok_or_else(|| Error::parse(format!("player uid {uid} was not found")))?;
        let record = self.records.records[position];
        let window_start = record.offset + PersonLayout::WINDOW_START_OFFSET;
        let window_end = self.records.window_end(position, self.game_db.len());
        let next_uid = self.records.records.get(position + 1).map(|next| next.uid);
        let closing = closing_person_header(
            &self.game_db,
            record.pindex,
            record.offset + PlayerLayout::DECODE_EXTENT,
            window_end,
            record.uid,
            next_uid,
        );
        let person_end = closing.map_or(window_end, |(at, _)| at);
        let decoder = PersonDecoder {
            names: &self.names,
            clubs: ClubLookup::empty(),
            clock: self.clock,
        };
        let (location, _) = decoder
            .locate(&self.game_db, window_start, person_end)
            .ok_or_else(|| {
                Error::parse(format!(
                    "the person block for player uid {uid} was not found"
                ))
            })?;
        self.person_locations.insert(uid, location);
        Ok(location)
    }

    fn push_change(&mut self, field: impl Into<String>, before: i64, after: i64) -> EditChange {
        self.push_change_display(field, before, after, None, None)
    }

    fn push_change_display(
        &mut self,
        field: impl Into<String>,
        before: i64,
        after: i64,
        before_display: Option<String>,
        after_display: Option<String>,
    ) -> EditChange {
        let change = EditChange {
            field: field.into(),
            before,
            after,
            before_display,
            after_display,
        };
        self.changes.push(change.clone());
        change
    }
}

fn compact_date(date: NaiveDate) -> i64 {
    date.format("%Y%m%d")
        .to_string()
        .parse()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_request_deserializes_from_json() {
        let request: PlayerEditRequest = serde_json::from_str(
            r#"{"wage":50000,"contract_end":"2030-06-30","ca":null,"pa":null,"attributes":[],"personality":[],"traits":[],"untraits":[]}"#,
        )
        .expect("contract request");
        assert_eq!(request.wage, Some(50_000));
        assert_eq!(request.contract_end, NaiveDate::from_ymd_opt(2030, 6, 30));
        assert!(!request.is_empty());
    }
}
