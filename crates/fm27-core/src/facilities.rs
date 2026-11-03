//! Club facility ratings from the save's club-info table.
//!
//! FM27 keeps the training and youth facility lines outside the club record, in
//! the same table that carries supporter shares and supporter traits. A row
//! starts with the stored club index plus one and the training-facilities byte;
//! the youth, junior-coaching and youth-recruitment bytes sit at a fixed offset
//! from the supporter-share block in the same row.
//!
//! The table is not contiguous, so rows are found by their club-index key and
//! then validated against the supporter-share block. Only clubs that keep a
//! monthly finance chain have a row; other clubs stay unset.

use std::collections::{HashMap, HashSet};

use crate::clubs::ClubIndex;
use crate::layout::read_u32;

#[derive(Debug, Clone, Copy, Default)]
pub struct ClubFacilities {
    pub training: Option<u8>,
    pub youth: Option<u8>,
    pub junior_coaching: Option<u8>,
    pub youth_recruitment: Option<u8>,
}

/// The offsets of one club's facility row inside `game_db`.
#[derive(Debug, Clone, Copy)]
pub struct FacilityRowOffsets {
    pub row_start: usize,
    pub info_start: usize,
}

#[derive(Debug, Clone, Copy)]
struct FacilityCandidate {
    row_start: usize,
    info_start: usize,
    key: u32,
    uid: u32,
    training: u8,
    youth: u8,
    junior_coaching: u8,
    youth_recruitment: u8,
}

impl FacilityRowOffsets {
    pub fn training_offset(self) -> usize {
        self.row_start + 4
    }

    pub fn youth_offset(self) -> usize {
        self.info_start + 63
    }

    pub fn junior_coaching_offset(self) -> usize {
        self.info_start + 64
    }

    pub fn youth_recruitment_offset(self) -> usize {
        self.info_start + 65
    }
}

/// Read the facility ratings for the tracked clubs that store a club-info row.
pub fn read_club_facilities(
    game_db: &[u8],
    club_index: &ClubIndex,
    tracked_clubs: &HashSet<u32>,
) -> HashMap<u32, ClubFacilities> {
    selected_rows(game_db, club_index, tracked_clubs)
        .into_iter()
        .map(|row| {
            (
                row.uid,
                ClubFacilities {
                    training: Some(row.training),
                    youth: Some(row.youth),
                    junior_coaching: Some(row.junior_coaching),
                    youth_recruitment: Some(row.youth_recruitment),
                },
            )
        })
        .collect()
}

/// Locate the facility row that belongs to one club, for in-place editing.
///
/// The row key is the stored club index plus one, matching
/// [`read_club_facilities`]. The same sorted-table selection as the read pass is
/// used so a key that merely occurs inside another record cannot be mistaken for
/// the club's own row.
pub fn locate_facility_row(
    game_db: &[u8],
    club_index: &ClubIndex,
    club_uid: u32,
    tracked_clubs: &HashSet<u32>,
) -> Option<FacilityRowOffsets> {
    selected_rows(game_db, club_index, tracked_clubs)
        .into_iter()
        .find(|row| row.uid == club_uid)
        .map(|row| FacilityRowOffsets {
            row_start: row.row_start,
            info_start: row.info_start,
        })
}

/// Collect every plausible facility row and keep the longest sequence ordered by
/// club index. The facility table is sorted by club key; a key can also occur
/// inside another table's record, and those matches are not in key order with
/// their neighbours. Picking the longest increasing sequence discards them
/// without relying on the first byte match.
fn selected_rows(
    game_db: &[u8],
    club_index: &ClubIndex,
    tracked_clubs: &HashSet<u32>,
) -> Vec<FacilityCandidate> {
    if game_db.len() < 500 {
        return Vec::new();
    }
    let mut index_to_uid = HashMap::with_capacity(tracked_clubs.len());
    for (&uid, span) in &club_index.record_spans {
        if !tracked_clubs.contains(&uid) {
            continue;
        }
        let Some(stored) = read_u32(game_db, span.record_start) else {
            continue;
        };
        let Some(key) = stored.checked_add(1) else {
            continue;
        };
        index_to_uid.insert(key, uid);
    }

    let mut candidates = Vec::new();
    for start in 0..game_db.len() - 500 {
        let training = game_db[start + 4];
        if !(1..=20).contains(&training) {
            continue;
        }
        let Some(key) = read_u32(game_db, start) else {
            continue;
        };
        let Some(&uid) = index_to_uid.get(&key) else {
            continue;
        };
        let Some(info_start) = locate_info_row(game_db, start) else {
            continue;
        };
        let youth = game_db[info_start + 63];
        let junior_coaching = game_db[info_start + 64];
        let youth_recruitment = game_db[info_start + 65];
        candidates.push(FacilityCandidate {
            row_start: start,
            info_start,
            key,
            uid,
            training,
            youth,
            junior_coaching,
            youth_recruitment,
        });
    }
    longest_key_ordered(candidates)
}

fn longest_key_ordered(candidates: Vec<FacilityCandidate>) -> Vec<FacilityCandidate> {
    if candidates.is_empty() {
        return candidates;
    }
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![None; candidates.len()];
    for (index, candidate) in candidates.iter().enumerate() {
        let position = tails.partition_point(|&tail| candidates[tail].key < candidate.key);
        if position > 0 {
            previous[index] = Some(tails[position - 1]);
        }
        if position == tails.len() {
            tails.push(index);
        } else {
            tails[position] = index;
        }
    }

    let mut selected = Vec::with_capacity(tails.len());
    let mut current = tails.last().copied();
    while let Some(index) = current {
        selected.push(candidates[index]);
        current = previous[index];
    }
    selected.reverse();
    selected
}

/// Find the supporter-share block that belongs to the row starting at `start`.
///
/// The row header is followed by a variable-length list, so the share block is
/// not at a fixed offset. Its shape is distinctive: six shares summing to 100,
/// six supporter traits, six zero bytes and a `2` tag.
fn locate_info_row(game_db: &[u8], start: usize) -> Option<usize> {
    let last = (start + 450).min(game_db.len().checked_sub(66)?);
    for info in (start + 50)..=last {
        let Some(shares) = game_db.get(info..info + 6) else {
            continue;
        };
        if shares.iter().any(|value| *value > 100) {
            continue;
        }
        if shares.iter().map(|value| u32::from(*value)).sum::<u32>() != 100 {
            continue;
        }
        let Some(traits) = game_db.get(info + 15..info + 21) else {
            continue;
        };
        if !traits.iter().all(|value| (1..=20).contains(value)) {
            continue;
        }
        if game_db.get(info + 21..info + 27) != Some(&[0u8; 6][..]) {
            continue;
        }
        if game_db.get(info + 27) != Some(&2) {
            continue;
        }
        let Some(ratings) = game_db.get(info + 63..info + 66) else {
            continue;
        };
        if !ratings.iter().all(|value| (1..=20).contains(value)) {
            continue;
        }
        return Some(info);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clubs::ClubRecordSpan;

    #[test]
    fn reads_the_facility_row_of_a_tracked_club() {
        let mut game_db = vec![0u8; 2048];
        game_db[0..4].copy_from_slice(&7u32.to_le_bytes());
        let start = 1000usize;
        game_db[start..start + 4].copy_from_slice(&8u32.to_le_bytes());
        game_db[start + 4] = 16;
        let info = start + 160;
        game_db[info..info + 6].copy_from_slice(&[10, 30, 25, 15, 5, 15]);
        game_db[info + 15..info + 21].copy_from_slice(&[10, 10, 10, 10, 10, 10]);
        game_db[info + 27] = 2;
        game_db[info + 63..info + 66].copy_from_slice(&[13, 10, 10]);

        let mut club_index = ClubIndex::default();
        club_index.record_spans.insert(
            100,
            ClubRecordSpan {
                record_start: 0,
                record_end: 500,
            },
        );
        let tracked = HashSet::from([100]);
        let facilities = read_club_facilities(&game_db, &club_index, &tracked);
        let row = facilities.get(&100).expect("a facility row");
        assert_eq!(row.training, Some(16));
        assert_eq!(row.youth, Some(13));
        assert_eq!(row.junior_coaching, Some(10));
        assert_eq!(row.youth_recruitment, Some(10));

        let offsets =
            locate_facility_row(&game_db, &club_index, 100, &tracked).expect("facility offsets");
        assert_eq!(offsets.training_offset(), start + 4);
        assert_eq!(offsets.youth_offset(), info + 63);
        assert_eq!(offsets.junior_coaching_offset(), info + 64);
        assert_eq!(offsets.youth_recruitment_offset(), info + 65);
    }

    #[test]
    fn ignores_clubs_without_a_tracked_row() {
        let game_db = vec![0u8; 2048];
        let club_index = ClubIndex::default();
        let facilities = read_club_facilities(&game_db, &club_index, &HashSet::new());
        assert!(facilities.is_empty());
    }

    #[test]
    fn a_key_inside_another_record_is_not_the_club_row() {
        let candidate = |row_start, key| FacilityCandidate {
            row_start,
            info_start: row_start + 100,
            key,
            uid: key,
            training: 10,
            youth: 10,
            junior_coaching: 10,
            youth_recruitment: 10,
        };
        let selected = longest_key_ordered(vec![
            candidate(100, 30),
            candidate(200, 10),
            candidate(300, 20),
            candidate(400, 30),
        ]);
        assert_eq!(
            selected
                .iter()
                .map(|row| (row.row_start, row.key))
                .collect::<Vec<_>>(),
            vec![(200, 10), (300, 20), (400, 30)]
        );
    }
}
