//! Locating player records in `game_db`.
//!
//! Two scans are merged: a fast scan for the `01 00 6c 07` marker, and a
//! completeness scan for the 15 rating bytes followed by 54 attribute bytes.

use std::collections::{HashMap, HashSet};

use crate::bytes::find_bytes;
use crate::error::{Error, Result};
use crate::layout::{MISSING_U32, PlayerLayout, read_i16, read_u8, read_u16, read_u32};

#[derive(Debug, Clone, Copy)]
pub struct PlayerRecord {
    pub offset: usize,
    pub pindex: u32,
    pub uid: u32,
}

#[derive(Debug, Default)]
pub struct PlayerRecords {
    pub records: Vec<PlayerRecord>,
    pub markerless: usize,
    pub by_pindex: HashMap<u32, usize>,
    pub by_uid: HashMap<u32, usize>,
}

impl PlayerRecords {
    pub fn window_end(&self, position: usize, game_db_len: usize) -> usize {
        self.records
            .get(position + 1)
            .map_or(game_db_len, |record| record.offset)
    }

    pub fn locate(game_db: &[u8], name_pools_end: usize) -> Result<Self> {
        let marker_candidates = scan_markers(game_db)?;
        let marker_offsets: HashSet<usize> = marker_candidates
            .iter()
            .map(|record| record.offset)
            .collect();
        let completeness = scan_completeness(game_db, name_pools_end, &marker_offsets);

        let mut records = marker_candidates;
        let markerless = completeness.len();
        records.extend(completeness);
        records.sort_unstable_by_key(|record| record.offset);
        records.dedup_by_key(|record| record.offset);

        if records.is_empty() {
            return Err(Error::parse("no player records found"));
        }

        let mut by_pindex = HashMap::with_capacity(records.len());
        let mut by_uid = HashMap::with_capacity(records.len());
        for (position, record) in records.iter().enumerate() {
            if by_pindex.insert(record.pindex, position).is_some() {
                return Err(Error::parse(format!(
                    "player pindex {} appears in two records",
                    record.pindex
                )));
            }
            if by_uid.insert(record.uid, position).is_some() {
                return Err(Error::parse(format!(
                    "player uid {} appears in two records",
                    record.uid
                )));
            }
        }

        Ok(Self {
            records,
            markerless,
            by_pindex,
            by_uid,
        })
    }
}

fn scan_markers(game_db: &[u8]) -> Result<Vec<PlayerRecord>> {
    let mut records = Vec::new();
    let marker = PlayerLayout::MARKER;
    let mut search_from = 0usize;
    while let Some(relative) = find_bytes(&game_db[search_from..], marker) {
        let marker_at = search_from + relative;
        search_from = marker_at + 1;
        let Some(record_offset) = marker_at.checked_sub(PlayerLayout::MARKER_OFFSET) else {
            continue;
        };
        if let Some((pindex, uid)) = accept_candidate(game_db, record_offset, true) {
            if record_offset + PlayerLayout::DECODE_EXTENT > game_db.len() {
                return Err(Error::parse("a player record runs past game_db"));
            }
            records.push(PlayerRecord {
                offset: record_offset,
                pindex,
                uid,
            });
        }
    }
    Ok(records)
}

/// Scan for the exact 15 ratings + 54 attributes shape. This is an O(n) pass:
/// a tiny state machine first finds 15 consecutive bytes in 1..=20, then checks
/// the following 54 bytes. Every true record's 15 rating bytes trigger the check.
fn scan_completeness(
    game_db: &[u8],
    name_pools_end: usize,
    marker_offsets: &HashSet<usize>,
) -> Vec<PlayerRecord> {
    let mut records = Vec::new();
    let start = name_pools_end.min(game_db.len());
    let mut run = 0usize;
    for (index, byte) in game_db.iter().enumerate().skip(start) {
        if (1..=20).contains(byte) {
            run += 1;
        } else {
            run = 0;
            continue;
        }
        if run < PlayerLayout::RATINGS_COUNT {
            continue;
        }
        let ratings_start = index + 1 - PlayerLayout::RATINGS_COUNT;
        let Some(record_offset) = ratings_start.checked_sub(PlayerLayout::RATINGS_OFFSET) else {
            continue;
        };
        if marker_offsets.contains(&record_offset) {
            continue;
        }
        let attrs_end = ratings_start + PlayerLayout::RATINGS_COUNT + PlayerLayout::ATTRIBUTE_COUNT;
        let Some(attribute_bytes) =
            game_db.get(ratings_start + PlayerLayout::RATINGS_COUNT..attrs_end)
        else {
            break;
        };
        if !attribute_bytes
            .iter()
            .all(|value| (1..=125).contains(value))
        {
            continue;
        }
        if let Some((pindex, uid)) = accept_candidate(game_db, record_offset, false) {
            records.push(PlayerRecord {
                offset: record_offset,
                pindex,
                uid,
            });
        }
    }
    records
}

fn accept_candidate(
    game_db: &[u8],
    record_offset: usize,
    check_pattern: bool,
) -> Option<(u32, u32)> {
    let uid_at = signed_offset(record_offset, PlayerLayout::UID_OFFSET)?;
    let uid_copy_at = signed_offset(record_offset, PlayerLayout::UID_COPY_OFFSET)?;
    let kind_at = signed_offset(record_offset, PlayerLayout::OBJECT_KIND_OFFSET)?;
    let pindex_at = signed_offset(record_offset, PlayerLayout::PINDEX_OFFSET)?;
    let attributes_end =
        record_offset + PlayerLayout::ATTRIBUTES_OFFSET + PlayerLayout::ATTRIBUTE_COUNT;
    if attributes_end > game_db.len() {
        return None;
    }

    let uid = read_u32(game_db, uid_at)?;
    let uid_copy = read_u32(game_db, uid_copy_at)?;
    if uid == 0 || uid == MISSING_U32 || uid_copy == 0 || uid_copy == MISSING_U32 {
        return None;
    }
    let kind = read_u8(game_db, kind_at)?;
    if uid != uid_copy && (uid < PlayerLayout::ALLOCATED_UID_MINIMUM || !matches!(kind, 2 | 3)) {
        return None;
    }

    let ca = read_u16(game_db, record_offset + PlayerLayout::CA_OFFSET)?;
    let pa = read_i16(game_db, record_offset + PlayerLayout::PA_OFFSET)?;
    let bucket = read_u8(
        game_db,
        record_offset + PlayerLayout::REPUTATION_BUCKET_OFFSET,
    )?;
    let raw_attributes = &game_db[record_offset + PlayerLayout::ATTRIBUTES_OFFSET..attributes_end];
    let highest_attribute = raw_attributes.iter().copied().max().unwrap_or(0);

    let edited = ca > 200 || pa > 200 || highest_attribute > 100;
    if edited && (uid != uid_copy || !matches!(kind, 2 | 3)) {
        return None;
    }
    let ability_max = if edited { 255 } else { 200 };
    if ca > ability_max || pa > ability_max as i16 || pa < -10 {
        return None;
    }
    if bucket > 200 {
        return None;
    }

    if check_pattern {
        let ratings = game_db.get(
            record_offset + PlayerLayout::RATINGS_OFFSET
                ..record_offset + PlayerLayout::RATINGS_OFFSET + PlayerLayout::RATINGS_COUNT,
        )?;
        if !ratings.iter().all(|rating| (1..=20).contains(rating)) {
            return None;
        }
        if !raw_attributes.iter().all(|value| (1..=125).contains(value)) {
            return None;
        }
    }

    Some((read_u32(game_db, pindex_at)?, uid))
}

fn signed_offset(base: usize, offset: i32) -> Option<usize> {
    if offset >= 0 {
        base.checked_add(offset as usize)
    } else {
        base.checked_sub(offset.unsigned_abs() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_offsets_are_bounds_checked() {
        assert_eq!(signed_offset(19, -19), Some(0));
        assert_eq!(signed_offset(18, -19), None);
        assert_eq!(signed_offset(10, 16), Some(26));
    }
}
