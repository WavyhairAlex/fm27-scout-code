//! Club facility ratings from the save's club-info table.
//!
//! FM27 keeps the training and youth facility lines outside the club record, in
//! the same table that carries supporter shares and supporter traits. A row
//! starts with the stored club index plus one and the training-facilities byte;
//! the youth, junior-coaching and youth-recruitment ratings sit at a fixed
//! offset from the supporter-share block in the same row.
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

/// Read the facility ratings for the tracked clubs that store a club-info row.
pub fn read_club_facilities(
    game_db: &[u8],
    club_index: &ClubIndex,
    tracked_clubs: &HashSet<u32>,
) -> HashMap<u32, ClubFacilities> {
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

    let mut facilities = HashMap::with_capacity(index_to_uid.len());
    let mut used_rows = HashSet::new();
    if game_db.len() < 500 {
        return facilities;
    }
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
        if facilities.contains_key(&uid) {
            continue;
        }
        let Some(info) = locate_info_row(game_db, start) else {
            continue;
        };
        if !used_rows.insert(info) {
            continue;
        }
        let youth = game_db[info + 63];
        let junior_coaching = game_db[info + 64];
        let youth_recruitment = game_db[info + 65];
        if !(1..=20).contains(&youth)
            || !(1..=20).contains(&junior_coaching)
            || !(1..=20).contains(&youth_recruitment)
        {
            continue;
        }
        facilities.insert(
            uid,
            ClubFacilities {
                training: Some(training),
                youth: Some(youth),
                junior_coaching: Some(junior_coaching),
                youth_recruitment: Some(youth_recruitment),
            },
        );
    }
    facilities
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
    }

    #[test]
    fn ignores_clubs_without_a_tracked_row() {
        let game_db = vec![0u8; 2048];
        let club_index = ClubIndex::default();
        let facilities = read_club_facilities(&game_db, &club_index, &HashSet::new());
        assert!(facilities.is_empty());
    }
}
