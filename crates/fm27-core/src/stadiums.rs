//! Stadium table in `game_db`.
//!
//! The save stores capacities, pitch dimensions, dates and an owner club index.
//! It does not store a direct club -> ground pointer, so this module exposes
//! ownership only; home-ground use would require the fixture calendar.

use std::collections::{HashMap, HashSet};

use crate::bytes::find_bytes;
use crate::clubs::ClubIndex;
use crate::layout::{MISSING_U32, StadiumLayout, decode_date, read_u8, read_u16, read_u32};
use crate::model::Stadium;

#[derive(Debug, Clone, Default)]
pub struct StadiumIndex {
    pub stadiums: Vec<Stadium>,
    pub row_offsets: HashMap<u32, usize>,
    position_by_uid: HashMap<u32, usize>,
}

impl StadiumIndex {
    pub fn stadium(&self, uid: u32) -> Option<&Stadium> {
        self.position_by_uid
            .get(&uid)
            .and_then(|position| self.stadiums.get(*position))
    }

    pub fn row_offset(&self, uid: u32) -> Option<usize> {
        self.row_offsets.get(&uid).copied()
    }
}

/// Read the stadium table. Returns an empty index when the table cannot be located;
/// callers that need editing should treat that as an error.
pub fn read_stadium_index(game_db: &[u8], club_index: &ClubIndex) -> StadiumIndex {
    let Some(head) = locate_stadium_table(game_db) else {
        return StadiumIndex::default();
    };
    let mut stadiums = Vec::new();
    let mut row_offsets = HashMap::new();
    let mut position_by_uid = HashMap::new();
    let mut seen = HashSet::new();
    let mut row_offset = head;
    let mut ordinal = 1u32;
    while !reached_table_end(game_db, row_offset, ordinal) {
        if !row_heads_at(game_db, row_offset, ordinal) {
            break;
        }
        let Some((mut stadium, row_length)) = decode_row(game_db, row_offset, club_index) else {
            break;
        };
        if !seen.insert(stadium.uid) {
            break;
        }
        stadium.owner_club_name = stadium
            .owner_club_uid
            .and_then(|uid| club_index.club(uid))
            .map(|club| club.name.clone());
        row_offsets.insert(stadium.uid, row_offset);
        position_by_uid.insert(stadium.uid, stadiums.len());
        stadiums.push(stadium);
        row_offset += row_length;
        ordinal += 1;
    }
    StadiumIndex {
        stadiums,
        row_offsets,
        position_by_uid,
    }
}

fn locate_stadium_table(game_db: &[u8]) -> Option<usize> {
    let needle = 1u32.to_le_bytes();
    let mut search_from = 0usize;
    while let Some(relative) = find_bytes(&game_db[search_from..], &needle) {
        let candidate = search_from + relative;
        search_from = candidate + 1;
        if table_starts_here(game_db, candidate) {
            return Some(candidate);
        }
    }
    None
}

fn table_starts_here(game_db: &[u8], head: usize) -> bool {
    (0..StadiumLayout::LOCATOR_ROWS).all(|step| {
        row_heads_at(
            game_db,
            head + step * StadiumLayout::ROW_BYTES,
            (step + 1) as u32,
        )
    })
}

fn row_heads_at(game_db: &[u8], row_offset: usize, expected: u32) -> bool {
    let Some(ordinal) = read_u32(game_db, row_offset + StadiumLayout::ORDINAL_OFFSET) else {
        return false;
    };
    let Some(uid) = read_u32(game_db, row_offset + StadiumLayout::UID_OFFSET) else {
        return false;
    };
    let Some(uid_copy) = read_u32(game_db, row_offset + StadiumLayout::UID_COPY_OFFSET) else {
        return false;
    };
    ordinal == expected
        && uid == uid_copy
        && read_u8(game_db, row_offset + StadiumLayout::ZERO_BYTE_OFFSET) == Some(0)
}

fn decode_row(
    game_db: &[u8],
    row_offset: usize,
    club_index: &ClubIndex,
) -> Option<(Stadium, usize)> {
    let flags = read_u8(game_db, row_offset + StadiumLayout::FLAGS_OFFSET)?;
    let mut row_length = StadiumLayout::ROW_BYTES;
    let mut name = None;
    if flags & StadiumLayout::INLINE_NAME_FLAG != 0 {
        let length = read_u32(game_db, row_offset + StadiumLayout::NAME_LENGTH_OFFSET)?;
        let (lowest, highest) = StadiumLayout::NAME_LENGTH_RANGE;
        if !(lowest..=highest).contains(&length) {
            return None;
        }
        let start = row_offset + StadiumLayout::NAME_OFFSET;
        let end = start.checked_add(length as usize)?;
        let text = std::str::from_utf8(game_db.get(start..end)?).ok()?;
        name = Some(text.to_owned());
        row_length = StadiumLayout::ROW_BYTES
            .checked_add(StadiumLayout::NAMED_ROW_EXTRA_BYTES)?
            .checked_add(length as usize)?;
        if row_offset.checked_add(row_length)? > game_db.len() {
            return None;
        }
    }
    let uid = read_u32(game_db, row_offset + StadiumLayout::UID_OFFSET)?.checked_add(1)?;
    let all_seater_capacity = read_u32(game_db, row_offset + StadiumLayout::ALL_SEATER_OFFSET)?;
    let expansion_capacity = read_u32(game_db, row_offset + StadiumLayout::EXPANSION_OFFSET)?;
    let capacity =
        read_u32(game_db, row_offset + StadiumLayout::CAPACITY_OFFSET).filter(|value| *value != 0);
    let owner_index = read_u32(game_db, row_offset + StadiumLayout::OWNER_OFFSET)
        .filter(|value| *value != MISSING_U32);
    let owner_club_uid = owner_index.and_then(|index| club_index.uid_by_index.get(&index).copied());
    let stadium = Stadium {
        uid,
        name,
        all_seater_capacity,
        expansion_capacity,
        capacity,
        owner_club_uid,
        owner_club_name: None,
        pitch_length_dm: read_u16(game_db, row_offset + StadiumLayout::PITCH_LENGTH_OFFSET)?,
        pitch_width_dm: read_u16(game_db, row_offset + StadiumLayout::PITCH_WIDTH_OFFSET)?,
        pitch_min_length_dm: read_u16(
            game_db,
            row_offset + StadiumLayout::PITCH_MIN_LENGTH_OFFSET,
        )?,
        pitch_min_width_dm: read_u16(game_db, row_offset + StadiumLayout::PITCH_MIN_WIDTH_OFFSET)?,
        pitch_max_length_dm: read_u16(
            game_db,
            row_offset + StadiumLayout::PITCH_MAX_LENGTH_OFFSET,
        )?,
        pitch_max_width_dm: read_u16(game_db, row_offset + StadiumLayout::PITCH_MAX_WIDTH_OFFSET)?,
        built_date: decode_date(game_db, row_offset + StadiumLayout::BUILT_OFFSET)
            .map(|date| date.format("%Y-%m-%d").to_string()),
        rebuilt_date: decode_date(game_db, row_offset + StadiumLayout::REBUILT_OFFSET)
            .map(|date| date.format("%Y-%m-%d").to_string()),
    };
    Some((stadium, row_length))
}

fn reached_table_end(game_db: &[u8], row_offset: usize, expected_ordinal: u32) -> bool {
    let Some(ordinal) = read_u32(game_db, row_offset + StadiumLayout::ORDINAL_OFFSET) else {
        return false;
    };
    let Some(uid) = read_u32(game_db, row_offset + StadiumLayout::UID_OFFSET) else {
        return false;
    };
    let Some(uid_copy) = read_u32(game_db, row_offset + StadiumLayout::UID_COPY_OFFSET) else {
        return false;
    };
    let marker_start = row_offset + StadiumLayout::CLOSING_MARKER_OFFSET;
    let marker_end = marker_start + StadiumLayout::CLOSING_MARKER.len();
    let uid_start = row_offset + StadiumLayout::CLOSING_UID_OFFSET;
    ordinal == expected_ordinal
        && uid_copy == uid
        && game_db.get(marker_start..marker_end) == Some(&StadiumLayout::CLOSING_MARKER)
        && read_u32(game_db, uid_start) == Some(uid)
}
