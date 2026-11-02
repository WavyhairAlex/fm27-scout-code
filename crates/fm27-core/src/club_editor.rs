//! In-place editing of low-risk club fields inside a decompressed `game_db`.
//!
//! This module never writes a save itself. It mutates the `game_db` bytes and the
//! caller hands the result to [`SaveFile::write_with_replacements`], which writes a
//! new save file.

use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::clubs::ClubIndex;
use crate::container::{SaveFile, SaveWriteReport};
use crate::editor::EditChange;
use crate::error::{Error, Result};
use crate::finances::locate_finance_chain;
use crate::layout::{
    BUILD, FinanceLayout, GAME_DB_SCHEMA, StadiumLayout, read_i32, read_u8, read_u16, read_u32,
};
use crate::metadata::SaveMetadata;
use crate::parse::ParsedSave;
use crate::stadiums::read_stadium_index;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClubFinanceEdit {
    pub balance: Option<i64>,
    pub transfer_budget_remaining: Option<i64>,
    pub wage_budget_weekly: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StadiumCapacityEdit {
    pub stadium_uid: u32,
    pub all_seater_capacity: Option<u32>,
    pub expansion_capacity: Option<u32>,
}

/// A set of low-risk club edits shared by the CLI and the desktop app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClubEditRequest {
    pub reputation: Option<u16>,
    pub corporate_facilities: Option<u8>,
    pub finances: Option<ClubFinanceEdit>,
    pub stadiums: Vec<StadiumCapacityEdit>,
}

impl ClubEditRequest {
    pub fn is_empty(&self) -> bool {
        self.reputation.is_none()
            && self.corporate_facilities.is_none()
            && self.finances.as_ref().is_none_or(ClubFinanceEdit::is_empty)
            && self.stadiums.iter().all(StadiumCapacityEdit::is_empty)
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.reputation.is_some() {
            warnings.push(
                "Club reputation affects transfer interest, board expectations and media attention; FM may use it broadly across the simulation."
                    .to_owned(),
            );
        }
        if self.corporate_facilities.is_some() {
            warnings.push(
                "Corporate facilities feed commercial income. The value is stored only for clubs that keep a monthly finance series."
                    .to_owned(),
            );
        }
        if self.finances.as_ref().is_some_and(|edit| !edit.is_empty()) {
            warnings.push(
                "Finance figures are stored snapshots. FM may recalculate budgets or balances when the game advances."
                    .to_owned(),
            );
        }
        if !self.stadiums.is_empty() {
            warnings.push(
                "Stadium capacity affects attendance and matchday income. Only stadiums owned by the club can be edited."
                    .to_owned(),
            );
        }
        warnings
    }

    pub fn apply(&self, editor: &mut ClubEditor, club_uid: u32) -> Result<()> {
        if let Some(value) = self.reputation {
            editor.set_reputation(club_uid, value)?;
        }
        if let Some(value) = self.corporate_facilities {
            editor.set_corporate_facilities(club_uid, value)?;
        }
        if let Some(finances) = &self.finances {
            editor.set_finances(club_uid, finances)?;
        }
        for stadium in &self.stadiums {
            editor.set_stadium_capacity(club_uid, stadium)?;
        }
        Ok(())
    }

    pub fn verify(&self, parsed: &ParsedSave, club_uid: u32) -> Result<()> {
        let club = parsed
            .clubs
            .iter()
            .find(|club| club.uid == club_uid)
            .ok_or_else(|| Error::parse(format!("club uid {club_uid} was not found")))?;
        if let Some(expected) = self.reputation {
            if club.reputation != Some(expected) {
                return Err(Error::parse(format!(
                    "club reputation verification failed: expected {expected}, got {:?}",
                    club.reputation
                )));
            }
        }
        if let Some(expected) = self.corporate_facilities {
            if club.corporate_facilities != Some(expected) {
                return Err(Error::parse(format!(
                    "corporate facilities verification failed: expected {expected}, got {:?}",
                    club.corporate_facilities
                )));
            }
        }
        if let Some(finances) = &self.finances {
            let latest = parsed
                .club_finances
                .iter()
                .filter(|row| row.club_uid == club_uid)
                .max_by(|left, right| left.month.cmp(&right.month))
                .ok_or_else(|| {
                    Error::parse(format!(
                        "club uid {club_uid} has no finance row after editing"
                    ))
                })?;
            if let Some(expected) = finances.balance
                && latest.balance != expected
            {
                return Err(Error::parse(format!(
                    "balance verification failed: expected {expected}, got {}",
                    latest.balance
                )));
            }
            if let Some(expected) = finances.transfer_budget_remaining
                && latest.transfer_budget_remaining != expected
            {
                return Err(Error::parse(format!(
                    "transfer budget verification failed: expected {expected}, got {}",
                    latest.transfer_budget_remaining
                )));
            }
            if let Some(expected) = finances.wage_budget_weekly
                && latest.wage_budget_weekly != expected
            {
                return Err(Error::parse(format!(
                    "wage budget verification failed: expected {expected}, got {}",
                    latest.wage_budget_weekly
                )));
            }
        }
        for edit in &self.stadiums {
            let stadium = parsed
                .stadiums
                .iter()
                .find(|stadium| stadium.uid == edit.stadium_uid)
                .ok_or_else(|| {
                    Error::parse(format!(
                        "stadium uid {} was not found after editing",
                        edit.stadium_uid
                    ))
                })?;
            if let Some(expected) = edit.all_seater_capacity
                && stadium.all_seater_capacity != expected
            {
                return Err(Error::parse(format!(
                    "stadium {} all-seater verification failed: expected {expected}, got {}",
                    edit.stadium_uid, stadium.all_seater_capacity
                )));
            }
            if let Some(expected) = edit.expansion_capacity
                && stadium.expansion_capacity != expected
            {
                return Err(Error::parse(format!(
                    "stadium {} expansion verification failed: expected {expected}, got {}",
                    edit.stadium_uid, stadium.expansion_capacity
                )));
            }
        }
        Ok(())
    }
}

impl ClubFinanceEdit {
    pub fn is_empty(&self) -> bool {
        self.balance.is_none()
            && self.transfer_budget_remaining.is_none()
            && self.wage_budget_weekly.is_none()
    }
}

impl StadiumCapacityEdit {
    pub fn is_empty(&self) -> bool {
        self.all_seater_capacity.is_none() && self.expansion_capacity.is_none()
    }
}

/// A mutable view over one save's `game_db` for low-risk club fields.
pub struct ClubEditor {
    game_db: Vec<u8>,
    club_index: ClubIndex,
    stadium_index: crate::stadiums::StadiumIndex,
    clock: NaiveDate,
    changes: Vec<EditChange>,
}

impl ClubEditor {
    pub fn open(save: &SaveFile, metadata: &SaveMetadata) -> Result<Self> {
        if metadata.build != BUILD {
            return Err(Error::Unsupported(format!(
                "club editing supports build {BUILD}; this save was written by {}",
                metadata.build
            )));
        }
        let game_db = save.read_section("game_db")?;
        let schema =
            read_u16(&game_db, 6).ok_or_else(|| Error::parse("game_db has no schema header"))?;
        if u32::from(schema) != GAME_DB_SCHEMA {
            return Err(Error::Unsupported(format!(
                "club editing supports game_db schema {GAME_DB_SCHEMA}; this save uses {schema}"
            )));
        }
        let club_index = ClubIndex::read(&game_db)?;
        let stadium_index = read_stadium_index(&game_db, &club_index);
        let clock = metadata
            .game_date
            .as_deref()
            .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        Ok(Self {
            game_db,
            club_index,
            stadium_index,
            clock,
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

    pub fn clock(&self) -> NaiveDate {
        self.clock
    }

    pub fn club_index(&self) -> &ClubIndex {
        &self.club_index
    }

    pub fn stadium_index(&self) -> &crate::stadiums::StadiumIndex {
        &self.stadium_index
    }

    pub fn set_reputation(&mut self, club_uid: u32, value: u16) -> Result<EditChange> {
        if !(1..=10_000).contains(&value) {
            return Err(Error::Unsupported(format!(
                "club reputation must be between 1 and 10000, got {value}"
            )));
        }
        let at = *self
            .club_index
            .status_offsets
            .get(&club_uid)
            .ok_or_else(|| {
                Error::parse(format!(
                    "club uid {club_uid} has no writable reputation status row"
                ))
            })?;
        let before = read_u16(&self.game_db, at)
            .ok_or_else(|| Error::parse("the club status row ends before its reputation"))?;
        self.game_db[at..at + 2].copy_from_slice(&value.to_le_bytes());
        Ok(self.push_change("reputation", before as i64, value as i64))
    }

    pub fn set_corporate_facilities(&mut self, club_uid: u32, value: u8) -> Result<EditChange> {
        let (lowest, highest) = FinanceLayout::FACILITY_RANGE;
        if !(lowest..=highest).contains(&value) {
            return Err(Error::Unsupported(format!(
                "corporate facilities must be between {lowest} and {highest}, got {value}"
            )));
        }
        let offset = self.facility_offset(club_uid)?;
        let before = read_u8(&self.game_db, offset)
            .ok_or_else(|| Error::parse("the club record ends before its facilities rating"))?;
        self.game_db[offset] = value;
        Ok(self.push_change("corporate_facilities", before as i64, value as i64))
    }

    pub fn set_finances(&mut self, club_uid: u32, edit: &ClubFinanceEdit) -> Result<()> {
        if edit.is_empty() {
            return Ok(());
        }
        let row = self.latest_finance_row(club_uid)?;
        if let Some(value) = edit.balance {
            let value = i32::try_from(value).map_err(|_| {
                Error::Unsupported(format!(
                    "balance {value} does not fit in the save's 32-bit field"
                ))
            })?;
            let (lowest, highest) = FinanceLayout::BALANCE_RANGE;
            if !(lowest..=highest).contains(&value) {
                return Err(Error::Unsupported(format!(
                    "balance must be between {lowest} and {highest}, got {value}"
                )));
            }
            let at = row + FinanceLayout::BALANCE_OFFSET;
            let before = read_i32(&self.game_db, at)
                .ok_or_else(|| Error::parse("the finance row ends before its balance"))?;
            self.game_db[at..at + 4].copy_from_slice(&value.to_le_bytes());
            self.push_change("finance.balance", before as i64, value as i64);
        }
        if let Some(value) = edit.transfer_budget_remaining {
            let value = i32::try_from(value).map_err(|_| {
                Error::Unsupported(format!(
                    "transfer budget {value} does not fit in the save's 32-bit field"
                ))
            })?;
            let (lowest, highest) = FinanceLayout::BALANCE_RANGE;
            if !(lowest..=highest).contains(&value) {
                return Err(Error::Unsupported(format!(
                    "transfer budget must be between {lowest} and {highest}, got {value}"
                )));
            }
            let at = row + FinanceLayout::TRANSFER_REMAINING_OFFSET;
            let before = read_i32(&self.game_db, at)
                .ok_or_else(|| Error::parse("the finance row ends before its transfer budget"))?;
            self.game_db[at..at + 4].copy_from_slice(&value.to_le_bytes());
            self.push_change(
                "finance.transfer_budget_remaining",
                before as i64,
                value as i64,
            );
        }
        if let Some(value) = edit.wage_budget_weekly {
            let value = u32::try_from(value).map_err(|_| {
                Error::Unsupported(format!(
                    "weekly wage budget {value} does not fit in the save's 32-bit field"
                ))
            })?;
            if value > FinanceLayout::WEEKLY_MAXIMUM {
                return Err(Error::Unsupported(format!(
                    "weekly wage budget must be at most {}, got {value}",
                    FinanceLayout::WEEKLY_MAXIMUM
                )));
            }
            let at = row + FinanceLayout::WAGE_BUDGET_OFFSET;
            let before = read_u32(&self.game_db, at)
                .ok_or_else(|| Error::parse("the finance row ends before its wage budget"))?;
            self.game_db[at..at + 4].copy_from_slice(&value.to_le_bytes());
            self.push_change("finance.wage_budget_weekly", before as i64, value as i64);
        }
        Ok(())
    }

    pub fn set_stadium_capacity(
        &mut self,
        club_uid: u32,
        edit: &StadiumCapacityEdit,
    ) -> Result<()> {
        if edit.is_empty() {
            return Ok(());
        }
        let stadium = self
            .stadium_index
            .stadium(edit.stadium_uid)
            .ok_or_else(|| {
                Error::parse(format!("stadium uid {} was not found", edit.stadium_uid))
            })?;
        if stadium.owner_club_uid != Some(club_uid) {
            return Err(Error::Unsupported(format!(
                "stadium uid {} is not owned by club uid {club_uid}",
                edit.stadium_uid
            )));
        }
        let row = self
            .stadium_index
            .row_offset(edit.stadium_uid)
            .ok_or_else(|| Error::parse("the stadium row offset is unavailable"))?;
        if let Some(value) = edit.all_seater_capacity {
            self.write_capacity(
                row,
                StadiumLayout::ALL_SEATER_OFFSET,
                "stadium.all_seater_capacity",
                value,
            )?;
        }
        if let Some(value) = edit.expansion_capacity {
            self.write_capacity(
                row,
                StadiumLayout::EXPANSION_OFFSET,
                "stadium.expansion_capacity",
                value,
            )?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(value) = self
            .changes
            .iter()
            .find(|change| change.field == "reputation")
            .map(|change| change.after)
        {
            if !(1..=10_000).contains(&value) {
                return Err(Error::Unsupported(format!(
                    "club reputation must be between 1 and 10000, got {value}"
                )));
            }
        }
        Ok(())
    }

    pub fn write(
        &self,
        save: &SaveFile,
        output: impl AsRef<Path>,
        compression_level: i32,
    ) -> Result<SaveWriteReport> {
        self.write_as(save, output, compression_level, None)
    }

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

    fn facility_offset(&self, club_uid: u32) -> Result<usize> {
        let span = self.club_index.record_span(club_uid).ok_or_else(|| {
            Error::parse(format!(
                "club uid {club_uid} has no record span for editing"
            ))
        })?;
        let chain = locate_finance_chain(&self.game_db, span).ok_or_else(|| {
            Error::Unsupported(format!(
                "club uid {club_uid} has no monthly finance chain, so it has no corporate facilities field"
            ))
        })?;
        let offset = chain.facility_offset();
        if offset >= span.record_end {
            return Err(Error::Unsupported(format!(
                "club uid {club_uid} has no stored corporate facilities byte inside its record"
            )));
        }
        Ok(offset)
    }

    fn latest_finance_row(&self, club_uid: u32) -> Result<usize> {
        let span = self.club_index.record_span(club_uid).ok_or_else(|| {
            Error::parse(format!(
                "club uid {club_uid} has no record span for editing"
            ))
        })?;
        let chain = locate_finance_chain(&self.game_db, span).ok_or_else(|| {
            Error::Unsupported(format!(
                "club uid {club_uid} has no monthly finance chain to edit"
            ))
        })?;
        Ok(chain.last_row())
    }

    fn write_capacity(&mut self, row: usize, offset: usize, field: &str, value: u32) -> Result<()> {
        if value > 10_000_000 {
            return Err(Error::Unsupported(format!(
                "{field} is implausibly large ({value}); the supported maximum is 10000000"
            )));
        }
        let at = row + offset;
        let before = read_u32(&self.game_db, at)
            .ok_or_else(|| Error::parse(format!("the stadium row ends before {field}")))?;
        self.game_db[at..at + 4].copy_from_slice(&value.to_le_bytes());
        self.push_change(field, before as i64, value as i64);
        Ok(())
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
