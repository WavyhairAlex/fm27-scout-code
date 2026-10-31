//! Competitions collected from the save's stage table, and their editor database ids.
//!
//! The save has no competition table. A competition exists because stages name it, so one
//! [`Competition`] is built per distinct competition id in the stage rows. The save also
//! stores no competition names; the game renders them from its installed database. Names
//! therefore come from `shared/competition-names.csv`, keyed on the editor database id that
//! the save pairs with each stage-space entity.
//!
//! Layouts are ported from the verified fmsave reference implementation. The stage table and
//! id-pair records use the same shapes as the FM26 layout that FM27 still stores.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::{Error, Result};
use crate::layout::{CompetitionIdPairLayout, MISSING_U32, StageTableLayout, read_u32};
use crate::model::Competition;

#[derive(Debug, Clone, Default)]
pub struct CompetitionIndex {
    pub competitions: Vec<Competition>,
    pub by_id: HashMap<u32, usize>,
    pub stage_count: usize,
    pub stage_gaps: usize,
    pub with_database_id: usize,
    pub database_id_conflicts: usize,
    pub with_name: usize,
}

impl CompetitionIndex {
    /// Read the stage table and pair its competition ids with editor database ids.
    ///
    /// The optional name table is applied through [`crate::reference::competition_name`].
    pub fn read(game_db: &[u8]) -> Result<Self> {
        let stages = read_stage_table(game_db)?;
        let database_ids = locate_database_ids(game_db);
        Ok(Self::build(
            stages,
            &database_ids,
            crate::reference::competition_name,
        ))
    }

    fn build(
        stages: StageIndex,
        database_ids: &HashMap<u32, u32>,
        name_for_database_id: impl Fn(u32) -> Option<&'static str>,
    ) -> Self {
        let mut competitions_by_database_id: HashMap<u32, Vec<u32>> = HashMap::new();
        for competition_id in stages.stage_ids_by_competition_id.keys() {
            if let Some(database_id) = database_ids.get(competition_id) {
                competitions_by_database_id
                    .entry(*database_id)
                    .or_default()
                    .push(*competition_id);
            }
        }
        let contested: HashSet<u32> = competitions_by_database_id
            .values()
            .filter(|competition_ids| competition_ids.len() > 1)
            .flatten()
            .copied()
            .collect();

        let mut competitions = Vec::with_capacity(stages.stage_ids_by_competition_id.len());
        let mut by_id = HashMap::with_capacity(competitions.capacity());
        let mut with_database_id = 0;
        let mut with_name = 0;
        for (competition_id, stage_ids) in stages.stage_ids_by_competition_id {
            let database_id = if contested.contains(&competition_id) {
                None
            } else {
                database_ids.get(&competition_id).copied()
            };
            let name = database_id
                .and_then(&name_for_database_id)
                .map(str::to_owned);
            if database_id.is_some() {
                with_database_id += 1;
            }
            if name.is_some() {
                with_name += 1;
            }
            by_id.insert(competition_id, competitions.len());
            competitions.push(Competition {
                id: competition_id,
                database_id,
                name,
                stage_ids,
            });
        }

        Self {
            competitions,
            by_id,
            stage_count: stages.row_count,
            stage_gaps: stages.gaps,
            with_database_id,
            database_id_conflicts: contested.len(),
            with_name,
        }
    }

    pub fn name_for(&self, competition_id: u32) -> Option<&str> {
        self.competition(competition_id)
            .and_then(|competition| competition.name.as_deref())
    }

    pub fn competition(&self, competition_id: u32) -> Option<&Competition> {
        self.by_id
            .get(&competition_id)
            .and_then(|index| self.competitions.get(*index))
    }
}

#[derive(Debug)]
struct StageIndex {
    stage_ids_by_competition_id: BTreeMap<u32, Vec<u32>>,
    row_count: usize,
    gaps: usize,
}

/// Walk the tail of `game_db` for fixed-size stage rows.
fn read_stage_table(game_db: &[u8]) -> Result<StageIndex> {
    let table_start = find_table_start(game_db)
        .ok_or_else(|| Error::parse("no stage table was found in game_db"))?;
    let (rows, gaps) = walk_rows(game_db, table_start);

    let mut stage_ids_by_competition_id: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut seen_stage_ids = HashSet::with_capacity(rows.len());
    for row_offset in rows {
        let stage_id = read_u32(game_db, row_offset + StageTableLayout::STAGE_ID_OFFSET)
            .ok_or_else(|| Error::parse("a stage row ends past game_db"))?;
        if !seen_stage_ids.insert(stage_id) {
            return Err(Error::parse(format!(
                "stage id {stage_id} appears in two stage rows"
            )));
        }
        let stored_competition_id = read_u32(
            game_db,
            row_offset + StageTableLayout::COMPETITION_ID_OFFSET,
        )
        .ok_or_else(|| Error::parse("a stage row ends past game_db"))?;
        if stored_competition_id == MISSING_U32
            || stored_competition_id >= StageTableLayout::COMPETITION_ID_LIMIT
        {
            continue;
        }
        stage_ids_by_competition_id
            .entry(stored_competition_id)
            .or_default()
            .push(stage_id);
    }

    Ok(StageIndex {
        row_count: seen_stage_ids.len(),
        stage_ids_by_competition_id,
        gaps,
    })
}

fn row_decodes(game_db: &[u8], row_offset: usize) -> bool {
    if row_offset
        .checked_add(StageTableLayout::ROW_BYTES)
        .is_none_or(|end| end > game_db.len())
    {
        return false;
    }
    if game_db[row_offset + StageTableLayout::ZERO_BYTE_OFFSET] != 0 {
        return false;
    }
    let Some(stage_id) = read_u32(game_db, row_offset + StageTableLayout::STAGE_ID_OFFSET) else {
        return false;
    };
    let Some(stage_id_copy) =
        read_u32(game_db, row_offset + StageTableLayout::STAGE_ID_COPY_OFFSET)
    else {
        return false;
    };
    if stage_id_copy != stage_id {
        return false;
    }
    let (lowest, highest) = StageTableLayout::STAGE_ID_EXCLUSIVE_RANGE;
    if !(stage_id > lowest && stage_id < highest) {
        return false;
    }
    let Some(previous_stage_id) = read_u32(
        game_db,
        row_offset + StageTableLayout::PREVIOUS_STAGE_ID_OFFSET,
    ) else {
        return false;
    };
    previous_stage_id == stage_id.wrapping_sub(1) || previous_stage_id == MISSING_U32
}

fn chain_starts_here(game_db: &[u8], row_offset: usize) -> bool {
    (0..StageTableLayout::CHAIN_ROWS).all(|step| {
        let Some(offset) = step
            .checked_mul(StageTableLayout::ROW_BYTES)
            .and_then(|delta| row_offset.checked_add(delta))
        else {
            return false;
        };
        row_decodes(game_db, offset)
    })
}

fn chain_id_steps(game_db: &[u8], row_offset: usize) -> usize {
    let mut previous = None;
    let mut steps = 0;
    for step in 0..StageTableLayout::CHAIN_ROWS {
        let Some(offset) = step
            .checked_mul(StageTableLayout::ROW_BYTES)
            .and_then(|delta| row_offset.checked_add(delta))
        else {
            break;
        };
        let Some(stage_id) = read_u32(game_db, offset + StageTableLayout::STAGE_ID_OFFSET) else {
            break;
        };
        if previous.is_some_and(|earlier: u32| stage_id == earlier + 1) {
            steps += 1;
        }
        previous = Some(stage_id);
    }
    steps
}

fn find_table_start(game_db: &[u8]) -> Option<usize> {
    let buffer_length = game_db.len();
    let mut searched_from = buffer_length;
    let mut window_bytes = StageTableLayout::SEARCH_BYTES;
    while searched_from > 0 {
        let window_start = buffer_length.saturating_sub(window_bytes);
        if let Some(table_start) = find_table_start_between(game_db, window_start, searched_from) {
            return Some(table_start);
        }
        searched_from = window_start;
        window_bytes = window_bytes.saturating_mul(2);
    }
    None
}

fn find_table_start_between(
    game_db: &[u8],
    first_candidate: usize,
    candidates_end: usize,
) -> Option<usize> {
    let buffer_length = game_db.len();
    let last_candidate =
        candidates_end.min(buffer_length.saturating_sub(StageTableLayout::ROW_BYTES) + 1);
    let mut candidate = first_candidate;
    while candidate < last_candidate {
        if game_db[candidate + StageTableLayout::ZERO_BYTE_OFFSET] == 0
            && row_decodes(game_db, candidate)
            && chain_starts_here(game_db, candidate)
        {
            let chain_end = (candidate + StageTableLayout::ROW_BYTES).min(buffer_length);
            let mut table_start = candidate;
            let mut best_steps = chain_id_steps(game_db, candidate);
            for offset in candidate..chain_end {
                if !chain_starts_here(game_db, offset) {
                    continue;
                }
                let steps = chain_id_steps(game_db, offset);
                if steps > best_steps {
                    best_steps = steps;
                    table_start = offset;
                }
            }
            while table_start >= StageTableLayout::ROW_BYTES
                && row_decodes(game_db, table_start - StageTableLayout::ROW_BYTES)
            {
                table_start -= StageTableLayout::ROW_BYTES;
            }
            return Some(table_start);
        }
        candidate += 1;
    }
    None
}

/// Step through the table, resynchronising one byte at a time over gaps.
fn walk_rows(game_db: &[u8], table_start: usize) -> (Vec<usize>, usize) {
    let buffer_length = game_db.len();
    let mut rows = Vec::new();
    let mut gaps = 0;
    let mut row_offset = table_start;
    while row_offset
        .checked_add(StageTableLayout::ROW_BYTES)
        .is_some_and(|end| end <= buffer_length)
    {
        if row_decodes(game_db, row_offset) {
            rows.push(row_offset);
            row_offset += StageTableLayout::ROW_BYTES;
            continue;
        }
        let furthest = (row_offset + StageTableLayout::RESYNCHRONISATION_BYTES)
            .min(buffer_length.saturating_sub(StageTableLayout::ROW_BYTES));
        let mut candidate = row_offset + 1;
        let mut resynchronised_at = None;
        while candidate <= furthest {
            if game_db[candidate + StageTableLayout::ZERO_BYTE_OFFSET] == 0
                && row_decodes(game_db, candidate)
            {
                resynchronised_at = Some(candidate);
                break;
            }
            candidate += 1;
        }
        let Some(next_row) = resynchronised_at else {
            break;
        };
        gaps += 1;
        row_offset = next_row;
    }
    (rows, gaps)
}

/// The editor database id of every stage-space entity the save pairs with exactly one.
///
/// These records cover stages and other entities too, so the result holds about three times
/// as many entries as there are competitions. Only [`CompetitionIndex::build`] restricts it
/// to the ids the stage table calls competitions.
fn locate_database_ids(game_db: &[u8]) -> HashMap<u32, u32> {
    let marker = CompetitionIdPairLayout::MARKER;
    let (lowest_entity_id, highest_entity_id) = CompetitionIdPairLayout::ENTITY_ID_RANGE;
    let (lowest_database_id, highest_database_id) = CompetitionIdPairLayout::DATABASE_ID_RANGE;
    let mut values: HashMap<u32, u32> = HashMap::new();
    let mut conflicts: HashSet<u32> = HashSet::new();
    let mut marker_at = 0;
    while marker_at + marker.len() <= game_db.len() {
        if game_db[marker_at] != 0xFF || game_db[marker_at..marker_at + marker.len()] != *marker {
            marker_at += 1;
            continue;
        }
        let record_start = marker_at + CompetitionIdPairLayout::RECORD_OFFSET_FROM_MARKER;
        marker_at += 1;
        let Some(entity_id) = read_u32(
            game_db,
            record_start + CompetitionIdPairLayout::ENTITY_ID_OFFSET,
        ) else {
            continue;
        };
        let Some(database_id) = read_u32(
            game_db,
            record_start + CompetitionIdPairLayout::DATABASE_ID_OFFSET,
        ) else {
            continue;
        };
        let Some(database_id_copy) = read_u32(
            game_db,
            record_start + CompetitionIdPairLayout::DATABASE_ID_COPY_OFFSET,
        ) else {
            continue;
        };
        if database_id != database_id_copy
            || !(lowest_entity_id..=highest_entity_id).contains(&entity_id)
            || !(lowest_database_id..=highest_database_id).contains(&database_id)
        {
            continue;
        }
        if !CompetitionIdPairLayout::CONSTANT_BYTES
            .iter()
            .all(|(offset, value)| {
                let at = record_start as isize + offset;
                at >= 0 && (at as usize) < game_db.len() && game_db[at as usize] == *value
            })
        {
            continue;
        }
        if let Some(existing) = values.get(&entity_id) {
            if *existing != database_id {
                conflicts.insert(entity_id);
            }
        } else {
            values.insert(entity_id, database_id);
        }
    }
    values.retain(|entity_id, _| !conflicts.contains(entity_id));
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn competition_ids() -> BTreeMap<u32, Vec<u32>> {
        BTreeMap::from([
            (0, vec![735]),
            (1, vec![187]),
            (2, vec![1003]),
            (3, vec![42]),
        ])
    }

    #[test]
    fn names_are_linked_through_the_database_id() {
        let database_ids = HashMap::from([(1, 2), (2, 3), (3, 4)]);
        let index = CompetitionIndex::build(
            StageIndex {
                stage_ids_by_competition_id: competition_ids(),
                row_count: 4,
                gaps: 0,
            },
            &database_ids,
            |database_id| match database_id {
                2 => Some("Alpha League"),
                3 => Some("Beta Cup"),
                _ => None,
            },
        );
        assert_eq!(index.competitions.len(), 4);
        assert_eq!(index.with_database_id, 3);
        assert_eq!(index.with_name, 2);
        assert_eq!(index.name_for(1), Some("Alpha League"));
        assert_eq!(index.name_for(2), Some("Beta Cup"));
        assert_eq!(index.name_for(0), None);
        assert_eq!(index.name_for(99), None);
        assert_eq!(index.stage_count, 4);
        assert_eq!(index.competition(3).unwrap().stage_ids, vec![42]);
    }

    #[test]
    fn a_database_id_claimed_by_two_competitions_names_neither() {
        let database_ids = HashMap::from([(1, 2), (2, 2)]);
        let index = CompetitionIndex::build(
            StageIndex {
                stage_ids_by_competition_id: competition_ids(),
                row_count: 4,
                gaps: 0,
            },
            &database_ids,
            |_| Some("Shared"),
        );
        assert_eq!(index.database_id_conflicts, 2);
        assert_eq!(index.with_database_id, 0);
        assert_eq!(index.with_name, 0);
        assert_eq!(index.name_for(1), None);
        assert_eq!(index.name_for(2), None);
    }

    #[test]
    fn id_pair_records_require_every_constant_and_one_database_id() {
        let mut bytes = vec![0u8; 512];
        let marker_at = 32;
        bytes[marker_at..marker_at + CompetitionIdPairLayout::MARKER.len()]
            .copy_from_slice(CompetitionIdPairLayout::MARKER);
        let record = marker_at + CompetitionIdPairLayout::RECORD_OFFSET_FROM_MARKER;
        bytes[record..record + 4].copy_from_slice(&7u32.to_le_bytes());
        bytes[record + 4..record + 8].copy_from_slice(&123u32.to_le_bytes());
        bytes[record + 8..record + 12].copy_from_slice(&123u32.to_le_bytes());
        for (offset, value) in CompetitionIdPairLayout::CONSTANT_BYTES {
            bytes[(record as isize + offset) as usize] = value;
        }
        assert_eq!(locate_database_ids(&bytes).get(&7), Some(&123));

        // A second record that disagrees leaves the entity without an id.
        let second = 300;
        bytes[second - CompetitionIdPairLayout::RECORD_OFFSET_FROM_MARKER
            ..second - CompetitionIdPairLayout::RECORD_OFFSET_FROM_MARKER
                + CompetitionIdPairLayout::MARKER.len()]
            .copy_from_slice(CompetitionIdPairLayout::MARKER);
        bytes[second..second + 4].copy_from_slice(&7u32.to_le_bytes());
        bytes[second + 4..second + 8].copy_from_slice(&124u32.to_le_bytes());
        bytes[second + 8..second + 12].copy_from_slice(&124u32.to_le_bytes());
        for (offset, value) in CompetitionIdPairLayout::CONSTANT_BYTES {
            bytes[(second as isize + offset) as usize] = value;
        }
        assert_eq!(locate_database_ids(&bytes).get(&7), None);
    }
}
