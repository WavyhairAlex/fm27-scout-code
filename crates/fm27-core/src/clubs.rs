//! Club records, team lists and the club-status table in `game_db`.
//!
//! Full saves are bounded by the counted club table, which also supplies the initial
//! club (uid 0) and exact record ends. Small fragments fall back to the anchor scan.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::layout::{ClubLayout, MISSING_U32, read_u8, read_u16, read_u32};
use crate::model::Club;

#[derive(Debug, Clone, Copy, Default)]
pub struct ClubRecordSpan {
    pub record_start: usize,
    pub record_end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ClubIndex {
    pub clubs: Vec<Club>,
    pub uid_by_index: HashMap<u32, u32>,
    pub club_by_uid: HashMap<u32, usize>,
    pub name_by_uid: HashMap<u32, String>,
    pub team_to_club: HashMap<u32, (u32, u8)>,
    pub affiliate_team_to_club: HashMap<u32, (u32, u8)>,
    pub fielding_team_to_club: HashMap<u32, (u32, u8)>,
    pub record_spans: HashMap<u32, ClubRecordSpan>,
    pub status_offsets: HashMap<u32, usize>,
}

#[derive(Debug, Clone)]
struct ClubRecord {
    record_start: usize,
    record_end: usize,
    team_counts_start: Option<usize>,
    club_index: u32,
    uid: u32,
    unique_id: Option<u32>,
    nation_id: u32,
    fa_nation_id: u32,
    city_id: Option<u32>,
    name: String,
    short_name: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct ClubStatus {
    reputation: Option<u16>,
    last_league_position: Option<u8>,
    /// Offset of the reputation word in the status table, when the row carries one.
    reputation_offset: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct TeamList {
    team_ids: Vec<u32>,
    affiliate_team_ids: Vec<u32>,
    gender: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
struct RecordStep {
    count_bytes: usize,
    lead_bytes: usize,
    tail_bytes: usize,
}

const AFTER_NAMES: [RecordStep; 2] = [
    RecordStep {
        count_bytes: 0,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 4,
        tail_bytes: 0,
    },
];
const BASE_FINANCE_BYTES: usize = 45;
const FULL_FINANCE: [RecordStep; 19] = [
    RecordStep {
        count_bytes: 0,
        lead_bytes: 66,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 235,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 235,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 235,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 235,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 8,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 67,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 230,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 4,
        lead_bytes: 49,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 71,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 19,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 67,
        tail_bytes: 0,
    },
];
const AFTER_FINANCE: [RecordStep; 10] = [
    RecordStep {
        count_bytes: 1,
        lead_bytes: 25,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 12,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 0,
        lead_bytes: 9,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 4,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 4,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 2,
        lead_bytes: 4,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 2,
        lead_bytes: 4,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 2,
        lead_bytes: 4,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 1,
        lead_bytes: 16,
        tail_bytes: 0,
    },
    RecordStep {
        count_bytes: 2,
        lead_bytes: 28,
        tail_bytes: 0,
    },
];

#[derive(Debug, Clone, Copy)]
struct TableRow {
    body_start: usize,
    footer_start: usize,
    unique_id: u32,
    team_counts_start: usize,
}

impl ClubIndex {
    pub fn read(game_db: &[u8]) -> Result<Self> {
        let (anchor_records, anchor_scan_end) = scan_club_records(game_db);
        if anchor_records.is_empty() {
            return Err(Error::parse("no club records found"));
        }

        let records = match read_counted_table(game_db, &anchor_records) {
            Some(records) => records,
            None => {
                let mut records = anchor_records;
                let mut record_ends = Vec::with_capacity(records.len());
                for position in 0..records.len() {
                    record_ends.push(
                        records
                            .get(position + 1)
                            .map_or(anchor_scan_end, |next| next.record_start),
                    );
                }
                for (record, end) in records.iter_mut().zip(record_ends) {
                    record.record_end = end;
                }
                if let Some(initial) = find_initial_club(game_db, &records) {
                    records.insert(0, initial);
                }
                records
            }
        };
        if records.is_empty() {
            return Err(Error::parse("no club records found"));
        }

        let statuses = read_statuses(game_db, &records);
        let mut clubs = Vec::with_capacity(records.len());
        let mut uid_by_index = HashMap::with_capacity(records.len());
        let mut name_by_uid = HashMap::with_capacity(records.len());
        let mut status_offsets = HashMap::new();
        let mut record_spans = HashMap::with_capacity(records.len());
        let mut team_to_club = HashMap::new();
        let mut affiliate_lists: Vec<Vec<u32>> = Vec::with_capacity(records.len());

        for (position, record) in records.iter().enumerate() {
            if uid_by_index.insert(record.club_index, record.uid).is_some() {
                return Err(Error::parse(format!(
                    "club index {} appears twice",
                    record.club_index
                )));
            }
            if name_by_uid
                .insert(record.uid, record.name.clone())
                .is_some()
            {
                return Err(Error::parse(format!(
                    "club uid {} appears twice",
                    record.uid
                )));
            }
            let team_list = read_team_list(
                game_db,
                record.record_start,
                record.record_end,
                record.team_counts_start,
            );
            affiliate_lists.push(team_list.affiliate_team_ids.clone());
            let mut seen_team_ids = Vec::new();
            for (slot, team_id) in team_list.team_ids.iter().copied().enumerate() {
                if seen_team_ids.contains(&team_id) {
                    return Err(Error::parse(format!(
                        "team id {team_id} is listed twice by club {}",
                        record.uid
                    )));
                }
                seen_team_ids.push(team_id);
                if let Some((other_club, _)) =
                    team_to_club.insert(team_id, (record.uid, slot as u8))
                {
                    return Err(Error::parse(format!(
                        "team id {team_id} is stored by clubs {other_club} and {}",
                        record.uid
                    )));
                }
            }
            let status = statuses.get(position).copied().unwrap_or_default();
            if let Some(offset) = status.reputation_offset {
                status_offsets.insert(record.uid, offset);
            }
            record_spans.insert(
                record.uid,
                ClubRecordSpan {
                    record_start: record.record_start,
                    record_end: record.record_end,
                },
            );
            clubs.push(Club {
                uid: record.uid,
                unique_id: record.unique_id,
                name: record.name.clone(),
                short_name: record.short_name.clone(),
                nation_id: record.nation_id,
                nation_name: None,
                fa_nation_id: record.fa_nation_id,
                fa_nation_name: None,
                city_id: record.city_id,
                gender: team_list.gender,
                reputation: status.reputation,
                last_league_position: status.last_league_position,
                parent_club_uid: None,
                parent_club_name: None,
                corporate_facilities: None,
                training_facilities: None,
                youth_facilities: None,
                junior_coaching: None,
                youth_recruitment: None,
                youth_level: None,
                teams: team_list.team_ids,
            });
        }

        let mut affiliate_team_to_club = HashMap::new();
        for (position, affiliate_ids) in affiliate_lists.iter().enumerate() {
            let parent_uid = records[position].uid;
            let own_slots = clubs[position].teams.len();
            for (index, team_id) in affiliate_ids.iter().copied().enumerate() {
                if team_to_club
                    .get(&team_id)
                    .is_some_and(|(uid, _)| *uid == parent_uid)
                {
                    continue;
                }
                affiliate_team_to_club
                    .entry(team_id)
                    .or_insert((parent_uid, (own_slots + index) as u8));
            }
        }
        for (position, affiliate_ids) in affiliate_lists.iter().enumerate() {
            let parent_uid = records[position].uid;
            for team_id in affiliate_ids {
                if affiliate_team_to_club
                    .get(team_id)
                    .is_some_and(|(uid, _)| *uid == parent_uid)
                    && !clubs[position].teams.contains(team_id)
                {
                    clubs[position].teams.push(*team_id);
                }
            }
        }

        // An affiliate team is a team of the controlled club. Resolve it back to that
        // club so the detail view can show the parent relationship.
        let position_by_uid: HashMap<u32, usize> = clubs
            .iter()
            .enumerate()
            .map(|(position, club)| (club.uid, position))
            .collect();
        for (team_id, (parent_uid, _)) in &affiliate_team_to_club {
            let Some((child_uid, _)) = team_to_club.get(team_id) else {
                continue;
            };
            if child_uid == parent_uid {
                continue;
            }
            let Some(position) = position_by_uid.get(child_uid).copied() else {
                continue;
            };
            let child = &mut clubs[position];
            if child.parent_club_uid.is_none() {
                child.parent_club_uid = Some(*parent_uid);
                child.parent_club_name = name_by_uid.get(parent_uid).cloned();
            }
        }

        let mut fielding_team_to_club = team_to_club.clone();
        fielding_team_to_club.extend(affiliate_team_to_club.iter().map(|(k, v)| (*k, *v)));

        clubs.sort_by_key(|club| club.uid);
        let club_by_uid = clubs
            .iter()
            .enumerate()
            .map(|(position, club)| (club.uid, position))
            .collect();
        Ok(Self {
            clubs,
            uid_by_index,
            club_by_uid,
            name_by_uid,
            team_to_club,
            affiliate_team_to_club,
            fielding_team_to_club,
            record_spans,
            status_offsets,
        })
    }

    pub fn club(&self, uid: u32) -> Option<&Club> {
        self.club_by_uid
            .get(&uid)
            .and_then(|position| self.clubs.get(*position))
    }

    pub fn record_span(&self, uid: u32) -> Option<ClubRecordSpan> {
        self.record_spans.get(&uid).copied()
    }

    pub fn team_fields(&self, team_id: u32) -> Option<(u32, &Club, u8)> {
        let (club_uid, slot) = self
            .affiliate_team_to_club
            .get(&team_id)
            .or_else(|| self.team_to_club.get(&team_id))
            .copied()?;
        let club = &self.clubs[*self.club_by_uid.get(&club_uid)?];
        Some((club_uid, club, slot))
    }
}

fn read_counted_table(game_db: &[u8], anchor_records: &[ClubRecord]) -> Option<Vec<ClubRecord>> {
    let first_start = anchor_records.first()?.record_start;
    let prefix_end = (first_start + ClubLayout::ANCHOR_OFFSET + 4).min(game_db.len());
    let anchor_distance = ClubLayout::ANCHOR_OFFSET - ClubLayout::ZERO_BYTE_OFFSET;
    let mut heads = Vec::new();
    let mut search_from = 0usize;
    while let Some(relative) = find_bytes(&game_db[search_from..prefix_end], ClubLayout::ANCHOR) {
        let hit = search_from + relative;
        search_from = hit + 1;
        for extra in [0usize, 4] {
            let Some(body) = hit
                .checked_sub(anchor_distance)
                .and_then(|value| value.checked_sub(extra))
            else {
                continue;
            };
            let Some(head) = body.checked_sub(5) else {
                continue;
            };
            if head + 5 > game_db.len() || game_db[head + 4] != 0 {
                continue;
            }
            let variant = if extra == 0 { 0 } else { 12 };
            if game_db.get(body) != Some(&variant) {
                continue;
            }
            let Some(row) = club_table_record(game_db, body) else {
                continue;
            };
            let count = read_u32(game_db, head).unwrap_or(0);
            let plausible = (1..=200_000).contains(&count);
            if (plausible || row.footer_start == first_start)
                && row.unique_id != 0
                && row.unique_id != u32::MAX
            {
                heads.push(head);
            }
        }
    }
    if heads.is_empty() {
        return None;
    }
    heads.sort_unstable();
    heads.dedup();

    let mut tables = Vec::new();
    for head in heads {
        let Some(rows) = walk_club_table(game_db, head) else {
            continue;
        };
        if rows.is_empty() {
            continue;
        }
        let table_start = rows[0].body_start;
        let table_end = rows.last()?.footer_start + 12;
        if anchor_records
            .iter()
            .all(|record| table_start <= record.record_start && record.record_start < table_end)
        {
            tables.push(rows);
        }
    }
    if tables.len() != 1 {
        return None;
    }
    let rows = tables.pop()?;
    let table_end = rows.last()?.footer_start + 12;
    let starts: Vec<usize> = rows
        .iter()
        .map(|row| {
            let extra = usize::from(game_db.get(row.body_start) == Some(&12)) * 4;
            row.body_start + extra - ClubLayout::ZERO_BYTE_OFFSET
        })
        .collect();

    let mut records = Vec::with_capacity(rows.len());
    for (ordinal, row) in rows.iter().enumerate() {
        let start = starts[ordinal];
        if ordinal > 0
            && !game_db
                .get(start + ClubLayout::ANCHOR_OFFSET..start + ClubLayout::ANCHOR_OFFSET + 4)
                .is_some_and(|anchor| anchor == ClubLayout::ANCHOR)
        {
            continue;
        }
        let uid = if ordinal == 0 {
            0
        } else {
            rows[ordinal - 1].unique_id + 1
        };
        let record_end = starts.get(ordinal + 1).copied().unwrap_or(table_end);
        let Some(record) = accept_table_record(
            game_db,
            start,
            record_end,
            row.team_counts_start,
            ordinal as u32,
            uid,
            Some(row.unique_id),
        ) else {
            if ordinal == 0 {
                return None;
            }
            continue;
        };
        records.push(record);
    }
    if records.is_empty() {
        None
    } else {
        Some(records)
    }
}

fn walk_club_table(game_db: &[u8], head: usize) -> Option<Vec<TableRow>> {
    let count = read_u32(game_db, head)?;
    if !(1..=200_000).contains(&count) || read_u8(game_db, head + 4) != Some(0) {
        return None;
    }
    let mut cursor = head + 5;
    let mut rows = Vec::with_capacity(count as usize);
    let mut seen = std::collections::HashSet::new();
    for ordinal in 0..count {
        let row = club_table_record(game_db, cursor)?;
        let stored = read_u32(game_db, row.footer_start)?;
        if stored != ordinal || row.unique_id == 0 || row.unique_id == u32::MAX {
            return None;
        }
        if !seen.insert(row.unique_id) {
            return None;
        }
        rows.push(row);
        cursor = row.footer_start + 12;
    }
    Some(rows)
}

fn club_table_record(game_db: &[u8], body: usize) -> Option<TableRow> {
    let kind = *game_db.get(body)?;
    if kind != 0 && kind != 12 {
        return None;
    }
    let mut cursor = body + 1 + 26 + if kind == 12 { 4 } else { 0 };
    for _ in 0..2 {
        let length = read_u32(game_db, cursor)? as usize;
        if length > 16_384 {
            return None;
        }
        cursor = cursor.checked_add(4 + length)?;
        if cursor > game_db.len() {
            return None;
        }
    }
    cursor = walk_steps(game_db, cursor, &AFTER_NAMES)?;
    let finance = *game_db.get(cursor)?;
    cursor += 1;
    match finance {
        0 => cursor = cursor.checked_add(BASE_FINANCE_BYTES)?,
        1 => cursor = walk_steps(game_db, cursor, &FULL_FINANCE)?,
        2 => {}
        _ => return None,
    }
    if cursor > game_db.len() {
        return None;
    }
    let team_counts_start = cursor;
    cursor = walk_steps(game_db, cursor, &AFTER_FINANCE)?;
    if cursor + 12 > game_db.len() {
        return None;
    }
    let unique_id = read_u32(game_db, cursor + 4)?;
    Some(TableRow {
        body_start: body,
        footer_start: cursor,
        unique_id,
        team_counts_start,
    })
}

fn walk_steps(game_db: &[u8], mut position: usize, steps: &[RecordStep]) -> Option<usize> {
    for step in steps {
        if step.count_bytes > 0 {
            let count = match step.count_bytes {
                1 => read_u8(game_db, position)? as usize,
                2 => read_u16(game_db, position)? as usize,
                4 => read_u32(game_db, position)? as usize,
                _ => return None,
            };
            position += step.count_bytes;
            if count > 100_000 {
                return None;
            }
            let entries = count.checked_mul(step.lead_bytes + step.tail_bytes)?;
            position = position.checked_add(entries)?;
        } else {
            position = position.checked_add(step.lead_bytes + step.tail_bytes)?;
        }
        if position > game_db.len() {
            return None;
        }
    }
    Some(position)
}

fn accept_table_record(
    game_db: &[u8],
    record_start: usize,
    record_end: usize,
    team_counts_start: usize,
    club_index: u32,
    uid: u32,
    unique_id: Option<u32>,
) -> Option<ClubRecord> {
    let nation_id = read_u32(game_db, record_start + ClubLayout::NATION_OFFSET)?;
    let fa_nation_id = read_u32(game_db, record_start + ClubLayout::FA_NATION_OFFSET)?;
    let nation_copy = read_u32(game_db, record_start + ClubLayout::NATION_COPY_OFFSET)?;
    let (lowest, highest) = ClubLayout::NATION_RANGE;
    if [nation_id, fa_nation_id, nation_copy]
        .iter()
        .any(|value| !(lowest..=highest).contains(value))
    {
        return None;
    }
    let city_id = read_u32(game_db, record_start + ClubLayout::CITY_OFFSET)
        .filter(|value| *value != MISSING_U32);
    let (name, short_name) = read_club_names(game_db, record_start)?;
    Some(ClubRecord {
        record_start,
        record_end,
        team_counts_start: Some(team_counts_start),
        club_index,
        uid,
        unique_id,
        nation_id,
        fa_nation_id,
        city_id,
        name,
        short_name,
    })
}

fn read_team_list(
    game_db: &[u8],
    record_start: usize,
    record_end: usize,
    team_counts_start: Option<usize>,
) -> TeamList {
    if let Some(counts_start) = team_counts_start {
        if let Some(list_start) = counts_start.checked_sub(ClubLayout::TEAM_COUNTS_OFFSET) {
            if let Some(list) = parse_team_list(game_db, list_start, record_end) {
                return list;
            }
        }
    }

    let mut candidates: Vec<(usize, TeamList)> = Vec::new();
    let mut triple_search = record_start;
    while let Some(relative) = find_bytes(
        game_db.get(triple_search..record_end).unwrap_or(&[]),
        &ClubLayout::NULL_DATE_TRIPLE,
    ) {
        let absolute = triple_search + relative;
        triple_search = absolute + 1;
        if let Some(list) = parse_team_list(game_db, absolute, record_end) {
            candidates.push((absolute, list));
        }
    }

    let mut search_from = record_start + ClubLayout::MINIMUM_FLOAT_ANCHOR_RECORD_OFFSET;
    while let Some(relative) = find_bytes(
        game_db.get(search_from..record_end).unwrap_or(&[]),
        &ClubLayout::FLOAT_ANCHOR,
    ) {
        let hit = search_from + relative;
        search_from = hit + 1;
        let Some(list_start) = hit.checked_sub(ClubLayout::FLOAT_ANCHOR_OFFSET) else {
            continue;
        };
        if let Some(list) = parse_team_list(game_db, list_start, record_end) {
            candidates.push((list_start, list));
        }
    }

    candidates
        .into_iter()
        .min_by_key(|(start, _)| *start)
        .map(|(_, list)| list)
        .unwrap_or_default()
}

fn parse_team_list(game_db: &[u8], list_start: usize, record_end: usize) -> Option<TeamList> {
    let mut cursor = list_start + ClubLayout::TEAM_COUNTS_OFFSET;
    for (entry_bytes, lead) in [
        (ClubLayout::FIRST_ENTRY_BYTES, ClubLayout::FIRST_ENTRY_LEAD),
        (
            ClubLayout::SECOND_ENTRY_BYTES,
            ClubLayout::SECOND_ENTRY_LEAD,
        ),
    ] {
        if cursor >= record_end {
            return None;
        }
        let count = *game_db.get(cursor)? as usize;
        let entries_start = cursor + 1;
        let entries_end = entries_start + count * entry_bytes;
        if entries_end > record_end {
            return None;
        }
        if (0..count).any(|entry| game_db[entries_start + entry * entry_bytes] != lead) {
            return None;
        }
        cursor = entries_end;
    }
    let team_count_at = cursor + ClubLayout::FILLER_BYTES;
    if team_count_at >= record_end {
        return None;
    }
    let team_count = *game_db.get(team_count_at)?;
    let (lowest, highest) = ClubLayout::TEAM_COUNT_RANGE;
    if !(lowest..=highest).contains(&team_count) {
        return None;
    }
    let team_ids_start = team_count_at + 1;
    let team_ids_end = team_ids_start + team_count as usize * 4;
    if team_ids_end > record_end {
        return None;
    }
    let mut team_ids = Vec::with_capacity(team_count as usize);
    let (lowest_id, highest_id) = ClubLayout::TEAM_ID_RANGE;
    for index in 0..team_count as usize {
        let team_id = read_u32(game_db, team_ids_start + index * 4)?;
        if !(lowest_id..=highest_id).contains(&team_id) {
            return None;
        }
        team_ids.push(team_id);
    }
    let mut affiliate_team_ids = Vec::new();
    let affiliate_count_at = team_ids_end;
    if let Some(affiliate_count) = read_u8(game_db, affiliate_count_at) {
        let (lowest, highest) = ClubLayout::AFFILIATE_COUNT_RANGE;
        if affiliate_count > 0 && (lowest..=highest).contains(&affiliate_count) {
            let start = affiliate_count_at + 1;
            let end = start + affiliate_count as usize * 4;
            if end <= record_end {
                for index in 0..affiliate_count as usize {
                    if let Some(team_id) = read_u32(game_db, start + index * 4) {
                        if (lowest_id..=highest_id).contains(&team_id) {
                            affiliate_team_ids.push(team_id);
                        }
                    }
                }
            }
        }
    }
    let gender = team_count_at
        .checked_sub(1)
        .and_then(|at| read_u8(game_db, at));
    Some(TeamList {
        team_ids,
        affiliate_team_ids,
        gender,
    })
}

fn find_initial_club(game_db: &[u8], records: &[ClubRecord]) -> Option<ClubRecord> {
    let first_start = records.first()?.record_start;
    let search_start = first_start.saturating_sub(4096);
    let mut found = None;
    let mut search_from = search_start;
    while let Some(relative) = find_bytes(
        game_db.get(search_from..first_start).unwrap_or(&[]),
        ClubLayout::ANCHOR,
    ) {
        let hit = search_from + relative;
        search_from = hit + 1;
        let Some(start) = hit.checked_sub(5).and_then(|body| body.checked_sub(12)) else {
            continue;
        };
        if read_u8(game_db, start + ClubLayout::ZERO_BYTE_OFFSET) != Some(0) {
            continue;
        }
        let Some(nation_id) = read_u32(game_db, start + ClubLayout::NATION_OFFSET) else {
            continue;
        };
        let Some(fa_nation_id) = read_u32(game_db, start + ClubLayout::FA_NATION_OFFSET) else {
            continue;
        };
        let (lowest, highest) = ClubLayout::NATION_RANGE;
        if !(lowest..=highest).contains(&nation_id) || !(lowest..=highest).contains(&fa_nation_id) {
            continue;
        }
        let Some((name, short_name)) = read_club_names(game_db, start) else {
            continue;
        };
        found = Some(ClubRecord {
            record_start: start,
            record_end: first_start,
            team_counts_start: None,
            club_index: 0,
            uid: 0,
            unique_id: None,
            nation_id,
            fa_nation_id,
            city_id: read_u32(game_db, start + ClubLayout::CITY_OFFSET)
                .filter(|value| *value != MISSING_U32),
            name,
            short_name,
        });
        break;
    }
    found
}

fn scan_club_records(game_db: &[u8]) -> (Vec<ClubRecord>, usize) {
    let mut records = Vec::new();
    let mut last_accepted_start = 0usize;
    let mut search_from = ClubLayout::ANCHOR_OFFSET;
    while let Some(relative) = find_bytes(&game_db[search_from..], ClubLayout::ANCHOR) {
        let hit = search_from + relative;
        search_from = hit + 1;
        let Some(record_start) = hit.checked_sub(ClubLayout::ANCHOR_OFFSET) else {
            continue;
        };
        if !records.is_empty()
            && record_start.saturating_sub(last_accepted_start) >= ClubLayout::STOP_GAP_BYTES
        {
            break;
        }
        if read_u8(game_db, record_start + ClubLayout::ZERO_BYTE_OFFSET) != Some(0) {
            continue;
        }
        if let Some(record) = accept_club_record(game_db, record_start) {
            records.push(record);
            last_accepted_start = record_start;
        }
    }
    let scan_end = if records.is_empty() {
        game_db.len()
    } else {
        (last_accepted_start + ClubLayout::STOP_GAP_BYTES).min(game_db.len())
    };
    (records, scan_end)
}

fn accept_club_record(game_db: &[u8], record_start: usize) -> Option<ClubRecord> {
    let nation_id = read_u32(game_db, record_start + ClubLayout::NATION_OFFSET)?;
    let fa_nation_id = read_u32(game_db, record_start + ClubLayout::FA_NATION_OFFSET)?;
    let nation_copy = read_u32(game_db, record_start + ClubLayout::NATION_COPY_OFFSET)?;
    let (lowest, highest) = ClubLayout::NATION_RANGE;
    for nation in [nation_id, fa_nation_id, nation_copy] {
        if !(lowest..=highest).contains(&nation) {
            return None;
        }
    }
    let uid = read_u32(game_db, record_start + ClubLayout::UID_OFFSET)?;
    if read_u32(game_db, record_start + ClubLayout::UID_COPY_OFFSET)? != uid {
        return None;
    }
    let club_index = read_u32(game_db, record_start + ClubLayout::CLUB_INDEX_OFFSET)? + 1;
    let (name, short_name) = read_club_names(game_db, record_start)?;
    Some(ClubRecord {
        record_start,
        record_end: game_db.len(),
        team_counts_start: None,
        club_index,
        uid: uid + 1,
        unique_id: None,
        nation_id,
        fa_nation_id,
        city_id: read_u32(game_db, record_start + ClubLayout::CITY_OFFSET)
            .filter(|value| *value != MISSING_U32),
        name,
        short_name,
    })
}

fn read_club_names(game_db: &[u8], record_start: usize) -> Option<(String, String)> {
    let long_length_at = record_start + ClubLayout::LONG_NAME_OFFSET;
    let long_length = read_u32(game_db, long_length_at)? as usize;
    if !(1..=ClubLayout::MAX_NAME_BYTES).contains(&long_length) {
        return None;
    }
    let long_start = long_length_at + 4;
    let short_length_at = long_start + long_length;
    let short_length = read_u32(game_db, short_length_at)? as usize;
    if !(1..=ClubLayout::MAX_NAME_BYTES).contains(&short_length) {
        return None;
    }
    let short_start = short_length_at + 4;
    let short_end = short_start + short_length;
    let name = std::str::from_utf8(game_db.get(long_start..short_length_at)?).ok()?;
    let short_name = std::str::from_utf8(game_db.get(short_start..short_end)?).ok()?;
    Some((name.to_owned(), short_name.to_owned()))
}

fn read_statuses(game_db: &[u8], records: &[ClubRecord]) -> Vec<ClubStatus> {
    let mut order: Vec<usize> = (0..records.len()).collect();
    order.sort_by_key(|index| records[*index].club_index);
    let mut statuses: Vec<ClubStatus> = vec![ClubStatus::default(); records.len()];
    let mut cursor = 0usize;
    let mut previous_ordinal: u32 = 0;
    let mut found_first = false;
    let mut leading_misses = 0usize;
    const SEARCH_WINDOW: usize = 256 * 1024;

    for index in order {
        let record = &records[index];
        if record.club_index == 0 {
            continue;
        }
        let stored_uid = record.uid - 1;
        let pair = [stored_uid.to_le_bytes(), stored_uid.to_le_bytes()].concat();
        let search_end = if found_first {
            (cursor + SEARCH_WINDOW + 8).min(game_db.len())
        } else {
            game_db.len()
        };
        if cursor >= search_end {
            leading_misses += 1;
            continue;
        }
        let mut accepted = false;
        let mut search_from = cursor;
        while let Some(relative) = find_bytes(&game_db[search_from..search_end], &pair) {
            let hit = search_from + relative;
            search_from = hit + 1;
            let Some(ordinal_at) = hit.checked_sub(4) else {
                continue;
            };
            let Some(ordinal) = read_u32(game_db, ordinal_at) else {
                continue;
            };
            let Some(kind) = read_u8(game_db, hit + 8) else {
                continue;
            };
            if previous_ordinal < ordinal && ordinal < 200_000 && matches!(kind, 0x0A | 0x0B) {
                accepted = true;
                previous_ordinal = ordinal;
                cursor = hit + 8;
                if kind == 0x0A {
                    let position = read_u8(game_db, hit + 10);
                    let reputation =
                        read_u16(game_db, hit + 11).filter(|value| (1..=10_000).contains(value));
                    statuses[index] = ClubStatus {
                        reputation,
                        last_league_position: position,
                        reputation_offset: reputation.map(|_| hit + 11),
                    };
                }
                break;
            }
        }
        if accepted {
            found_first = true;
        } else if !found_first {
            leading_misses += 1;
            if leading_misses >= 16 {
                break;
            }
        }
    }
    statuses
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
