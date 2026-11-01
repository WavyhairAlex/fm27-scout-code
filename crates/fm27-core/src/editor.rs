//! In-place editing of player records inside a decompressed `game_db` section.
//!
//! This module never writes a save itself. It mutates the `game_db` bytes and the
//! caller hands the result to [`SaveFile::write_with_replacements`], which writes a
//! new save file.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;

use crate::container::{SaveFile, SaveWriteReport};
use crate::error::{Error, Result};
use crate::layout::{
    ATTRIBUTE_KEYS, BUILD, GAME_DB_SCHEMA, PERSONALITY_KEYS, PersonLayout, PlayerLayout, read_i16,
    read_u8, read_u16, read_u64, stored_attribute_index, trait_name,
};
use crate::metadata::SaveMetadata;
use crate::names::NamePools;
use crate::parse::closing_person_header;
use crate::persons::{ClubLookup, PersonDecoder, PersonLocation};
use crate::player_scan::{PlayerRecord, PlayerRecords};

#[derive(Debug, Clone, Serialize)]
pub struct EditChange {
    pub field: String,
    pub before: i64,
    pub after: i64,
}

/// A mutable view over one save's `game_db` bytes.
pub struct PlayerEditor {
    game_db: Vec<u8>,
    records: PlayerRecords,
    names: NamePools,
    clock: NaiveDate,
    person_locations: HashMap<u32, PersonLocation>,
    edited_uids: HashSet<u32>,
    changes: Vec<EditChange>,
}

impl PlayerEditor {
    /// Open a save for editing. The build and `game_db` schema are checked so a
    /// future FM27 build with different offsets fails instead of guessing.
    pub fn open(save: &SaveFile, metadata: &SaveMetadata) -> Result<Self> {
        if metadata.build != BUILD {
            return Err(Error::Unsupported(format!(
                "editing supports build {BUILD}; this save was written by {}",
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
        save.write_with_replacements(
            &[("game_db", self.game_db.as_slice())],
            output,
            compression_level,
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
        let change = EditChange {
            field: field.into(),
            before,
            after,
        };
        self.changes.push(change.clone());
        change
    }
}
