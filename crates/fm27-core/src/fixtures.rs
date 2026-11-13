//! Fixture-calendar scan used to name a club's registered teams.
//!
//! FM27 stores no team names or age categories. The fixture calendar does store
//! the competition stage each side played in, so joining that stage to the
//! parsed competition index gives a factual squad label such as `U20` or `U18`.
//! This module reads only the calendar records needed for that join; it does not
//! decode scores, venues or match summaries.
//!
//! The calendar sits in the same unlisted region as the season statistics. One
//! zstd frame holds the career's own run of records; other frames hold stray
//! copies and template blocks. The scan finds the frame with the densest run,
//! keeps its largest packed run, and optionally merges the physically adjacent
//! frame when the calendar continues across the boundary.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate};

use crate::container::{FrameSpan, SaveFile};
use crate::error::Result;
use crate::layout::{FixtureCalendarLayout, read_u16, read_u32};
use crate::model::Competition;
use crate::season_stats::REGION_NAME;

/// One team's appearances in one competition during the selected season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamCompetition {
    pub team_id: u32,
    pub competition_id: u32,
    pub fixtures: u32,
}

#[derive(Debug, Clone, Copy)]
struct RawFixture {
    offset: usize,
    stage_id: u32,
    home_team_id: u32,
    away_team_id: u32,
    season: Option<u16>,
}

/// Read the fixture calendar and aggregate each team's stage appearances into
/// competitions for the save's current season.
pub fn read_team_competitions(
    save: &SaveFile,
    competitions: &[Competition],
    reference_date: NaiveDate,
) -> Result<Vec<TeamCompetition>> {
    let clock_year = reference_date.year();
    let spans = save.walk_region_frames(REGION_NAME)?;
    let heads = save.region_frame_heads(REGION_NAME, FixtureCalendarLayout::HEAD_BYTES)?;
    let scored: Vec<(FrameSpan, usize)> = heads
        .iter()
        .map(|(span, head)| (*span, count_records(head, clock_year)))
        .collect();
    let best_head = scored.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let mut candidates: Vec<FrameSpan> = scored
        .iter()
        .filter(|(_, count)| *count >= FixtureCalendarLayout::MIN_HEAD_RECORDS)
        .map(|(span, _)| *span)
        .collect();
    if best_head < FixtureCalendarLayout::MIN_HEAD_FALLBACK {
        // The calendar normally starts within the first few kilobytes of its
        // frame. If no head is dense enough, fall back to the largest frames so
        // a calendar that starts later still gets found.
        let mut largest: Vec<FrameSpan> = spans
            .iter()
            .copied()
            .filter(|span| !span.skippable)
            .collect();
        largest.sort_by_key(|span| Reverse(span.size));
        for span in largest
            .into_iter()
            .take(FixtureCalendarLayout::LARGE_FRAME_FALLBACK)
        {
            if !candidates
                .iter()
                .any(|candidate| candidate.offset == span.offset)
            {
                candidates.push(span);
            }
        }
    }
    candidates.sort_by_key(|span| span.offset);
    candidates.dedup_by_key(|span| span.offset);

    let mut scanned = HashSet::new();
    let mut runs: Vec<(FrameSpan, Vec<RawFixture>)> = Vec::new();
    for span in &candidates {
        scanned.insert(span.offset);
        let run = frame_run(save, *span, clock_year)?;
        if !run.is_empty() {
            runs.push((*span, run));
        }
    }
    let Some(best_index) = runs
        .iter()
        .enumerate()
        .max_by_key(|(_, (_, records))| records.len())
        .map(|(index, _)| index)
    else {
        return Ok(Vec::new());
    };
    let (best_span, best_records) = &runs[best_index];
    let best_len = best_records.len();
    if best_len < FixtureCalendarLayout::MIN_CALENDAR_RECORDS {
        // A handful of lookalikes is not a calendar. Leave the labels to the
        // age fallback instead of producing counts from a stray frame.
        return Ok(Vec::new());
    }
    let mut kept = best_records.clone();

    // A career's calendar can continue into the next physical frame. Merge a
    // neighbour only when it is adjacent and holds a meaningful share of the
    // run, so a stray frame cannot inflate the counts.
    if let Some(position) = spans
        .iter()
        .position(|span| span.offset == best_span.offset)
    {
        for neighbour in [position.checked_sub(1), Some(position + 1)]
            .into_iter()
            .flatten()
        {
            let Some(span) = spans.get(neighbour) else {
                continue;
            };
            if span.skippable || scanned.contains(&span.offset) {
                continue;
            }
            let adjacent = span.offset + span.size == best_span.offset
                || best_span.offset + best_span.size == span.offset;
            if !adjacent {
                continue;
            }
            let run = frame_run(save, *span, clock_year)?;
            if run.len() * 10 >= best_len {
                kept.extend(run);
            }
        }
    }

    let season = choose_season(&kept, current_season(reference_date));
    let stage_to_competition = stage_competitions(competitions);
    Ok(aggregate(&kept, season, &stage_to_competition))
}

fn frame_run(save: &SaveFile, span: FrameSpan, clock_year: i32) -> Result<Vec<RawFixture>> {
    let body = save.read_region_frame(REGION_NAME, span)?;
    let mut records = Vec::new();
    scan_records(&body, clock_year, &mut records);
    Ok(largest_run(records))
}

fn count_records(body: &[u8], clock_year: i32) -> usize {
    let (first_offset, first_value) = FixtureCalendarLayout::SENTINEL_OFFSETS[0];
    let (second_offset, second_value) = FixtureCalendarLayout::SENTINEL_OFFSETS[1];
    let gap = second_offset - first_offset;
    let mut count = 0;
    let mut offset = first_offset;
    while offset + gap < body.len() {
        if body[offset] == first_value
            && body[offset + gap] == second_value
            && accept_at(body, offset - first_offset, clock_year).is_some()
        {
            count += 1;
        }
        offset += 1;
    }
    count
}

fn scan_records(body: &[u8], clock_year: i32, records: &mut Vec<RawFixture>) {
    let (first_offset, first_value) = FixtureCalendarLayout::SENTINEL_OFFSETS[0];
    let (second_offset, second_value) = FixtureCalendarLayout::SENTINEL_OFFSETS[1];
    let gap = second_offset - first_offset;
    let mut offset = first_offset;
    while offset + gap < body.len() {
        if body[offset] == first_value && body[offset + gap] == second_value {
            if let Some(record) = accept_at(body, offset - first_offset, clock_year) {
                records.push(record);
            }
        }
        offset += 1;
    }
}

fn accept_at(body: &[u8], start: usize, clock_year: i32) -> Option<RawFixture> {
    let marker = start.checked_add_signed(FixtureCalendarLayout::MARKER_OFFSET)?;
    if start.checked_add(FixtureCalendarLayout::RECORD_BYTES)? > body.len() {
        return None;
    }
    if body.get(marker).copied() != Some(FixtureCalendarLayout::MARKER) {
        return None;
    }
    for (offset, value) in FixtureCalendarLayout::SENTINEL_OFFSETS {
        if body.get(start + offset).copied() != Some(value) {
            return None;
        }
    }
    let year = i32::from(read_u16(
        body,
        start + FixtureCalendarLayout::KICK_OFF_YEAR_OFFSET,
    )?);
    if !(clock_year - FixtureCalendarLayout::YEARS_BEFORE_CLOCK
        ..=clock_year + FixtureCalendarLayout::YEARS_AFTER_CLOCK)
        .contains(&year)
    {
        return None;
    }
    let stage_id = read_u32(
        body,
        start.checked_add_signed(FixtureCalendarLayout::STAGE_ID_OFFSET)?,
    )?;
    let home_team_id = read_u32(body, start + FixtureCalendarLayout::HOME_TEAM_ID_OFFSET)?;
    let away_team_id = read_u32(body, start + FixtureCalendarLayout::AWAY_TEAM_ID_OFFSET)?;
    if stage_id >= FixtureCalendarLayout::STAGE_ID_LIMIT
        || home_team_id >= FixtureCalendarLayout::TEAM_ID_LIMIT
        || away_team_id >= FixtureCalendarLayout::TEAM_ID_LIMIT
    {
        return None;
    }
    let season = read_u16(
        body,
        start + FixtureCalendarLayout::SEASON_START_YEAR_OFFSET,
    )
    .filter(|season| *season != 0 && *season != u16::MAX);
    Some(RawFixture {
        offset: start,
        stage_id,
        home_team_id,
        away_team_id,
        season,
    })
}

fn largest_run(mut records: Vec<RawFixture>) -> Vec<RawFixture> {
    if records.is_empty() {
        return records;
    }
    records.sort_by_key(|record| record.offset);
    let mut best_start = 0;
    let mut best_end = 0;
    let mut run_start = 0;
    for index in 1..=records.len() {
        let split = index == records.len()
            || records[index].offset - records[index - 1].offset
                > FixtureCalendarLayout::CLUSTER_GAP_BYTES;
        if split {
            if index - run_start > best_end - best_start {
                best_start = run_start;
                best_end = index;
            }
            run_start = index;
        }
    }
    records[best_start..best_end].to_vec()
}

fn current_season(date: NaiveDate) -> u16 {
    let year = if date.month() >= 7 {
        date.year()
    } else {
        date.year() - 1
    };
    u16::try_from(year).unwrap_or(0)
}

fn choose_season(records: &[RawFixture], current: u16) -> Option<u16> {
    let mut counts: HashMap<u16, usize> = HashMap::new();
    for record in records {
        if let Some(season) = record.season {
            *counts.entry(season).or_default() += 1;
        }
    }
    if counts.get(&current).copied().unwrap_or(0) >= FixtureCalendarLayout::MIN_SEASON_RECORDS {
        return Some(current);
    }
    counts
        .iter()
        .filter(|(season, _)| **season <= current)
        .max_by_key(|(season, count)| (*count, **season))
        .map(|(season, _)| *season)
        .or_else(|| {
            counts
                .iter()
                .max_by_key(|(season, count)| (*count, **season))
                .map(|(season, _)| *season)
        })
}

fn stage_competitions(competitions: &[Competition]) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    for competition in competitions {
        for stage_id in &competition.stage_ids {
            map.entry(*stage_id).or_insert(competition.id);
        }
    }
    map
}

fn aggregate(
    records: &[RawFixture],
    season: Option<u16>,
    stage_to_competition: &HashMap<u32, u32>,
) -> Vec<TeamCompetition> {
    let mut counts: HashMap<(u32, u32), u32> = HashMap::new();
    for record in records {
        if season.is_some() && record.season != season {
            continue;
        }
        let Some(competition_id) = stage_to_competition.get(&record.stage_id).copied() else {
            continue;
        };
        for team_id in [record.home_team_id, record.away_team_id] {
            if team_id == 0 {
                continue;
            }
            *counts.entry((team_id, competition_id)).or_default() += 1;
        }
    }
    let mut rows: Vec<TeamCompetition> = counts
        .into_iter()
        .map(|((team_id, competition_id), fixtures)| TeamCompetition {
            team_id,
            competition_id,
            fixtures,
        })
        .collect();
    rows.sort_by_key(|row| (row.team_id, row.competition_id));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(offset: usize, stage: u32, season: Option<u16>) -> RawFixture {
        RawFixture {
            offset,
            stage_id: stage,
            home_team_id: 1,
            away_team_id: 2,
            season,
        }
    }

    fn fixture(offset: usize, stage: u32, home: u32, away: u32, season: Option<u16>) -> RawFixture {
        RawFixture {
            offset,
            stage_id: stage,
            home_team_id: home,
            away_team_id: away,
            season,
        }
    }

    fn write_record(
        buffer: &mut [u8],
        start: usize,
        stage: u32,
        home: u32,
        away: u32,
        year: u16,
        season: u16,
    ) {
        let marker = start
            .checked_add_signed(FixtureCalendarLayout::MARKER_OFFSET)
            .expect("marker offset");
        let stage_offset = start
            .checked_add_signed(FixtureCalendarLayout::STAGE_ID_OFFSET)
            .expect("stage offset");
        buffer[marker] = FixtureCalendarLayout::MARKER;
        for (offset, value) in FixtureCalendarLayout::SENTINEL_OFFSETS {
            buffer[start + offset] = value;
        }
        let home_offset = start + FixtureCalendarLayout::HOME_TEAM_ID_OFFSET;
        let away_offset = start + FixtureCalendarLayout::AWAY_TEAM_ID_OFFSET;
        buffer[home_offset..home_offset + 4].copy_from_slice(&home.to_le_bytes());
        buffer[away_offset..away_offset + 4].copy_from_slice(&away.to_le_bytes());
        buffer[stage_offset..stage_offset + 4].copy_from_slice(&stage.to_le_bytes());
        buffer[start + 15..start + 17].copy_from_slice(&year.to_le_bytes());
        buffer[start + 22..start + 24].copy_from_slice(&season.to_le_bytes());
    }

    #[test]
    fn finds_fixture_records_and_rejects_a_missing_marker() {
        let mut buffer = vec![0u8; 256];
        write_record(&mut buffer, 12, 347, 883, 30028, 2027, 2027);
        let mut records = Vec::new();
        scan_records(&buffer, 2028, &mut records);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].stage_id, 347);
        assert_eq!(records[0].home_team_id, 883);
        assert_eq!(records[0].away_team_id, 30028);
        assert_eq!(records[0].season, Some(2027));

        buffer[0] = 0;
        let mut records = Vec::new();
        scan_records(&buffer, 2028, &mut records);
        assert!(records.is_empty());
    }

    #[test]
    fn keeps_the_largest_packed_run() {
        let kept = largest_run(vec![
            raw(12, 1, Some(2027)),
            raw(78, 2, Some(2027)),
            raw(144, 3, Some(2027)),
            raw(2_000_012, 4, Some(2027)),
        ]);
        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].stage_id, 1);
        assert_eq!(kept[2].stage_id, 3);
    }

    #[test]
    fn prefers_the_current_season_when_it_has_enough_records() {
        let mut records = vec![raw(0, 1, Some(2026)); 30];
        records.extend(vec![raw(0, 1, Some(2027)); 25]);
        assert_eq!(choose_season(&records, 2027), Some(2027));
    }

    #[test]
    fn falls_back_to_the_latest_completed_season() {
        let records = vec![raw(0, 1, Some(2026)); 30];
        assert_eq!(choose_season(&records, 2027), Some(2026));
    }

    #[test]
    fn aggregates_stages_of_one_competition_for_the_selected_season() {
        let records = vec![
            fixture(0, 347, 883, 30028, Some(2027)),
            fixture(0, 348, 883, 900, Some(2027)),
            fixture(0, 347, 30028, 33193, Some(2026)),
        ];
        let stage_map = HashMap::from([(347, 22), (348, 22)]);
        let rows = aggregate(&records, Some(2027), &stage_map);
        assert_eq!(
            rows,
            vec![
                TeamCompetition {
                    team_id: 883,
                    competition_id: 22,
                    fixtures: 2,
                },
                TeamCompetition {
                    team_id: 900,
                    competition_id: 22,
                    fixtures: 1,
                },
                TeamCompetition {
                    team_id: 30028,
                    competition_id: 22,
                    fixtures: 1,
                },
            ]
        );
    }

    #[test]
    fn maps_each_stage_to_its_competition() {
        let competitions = vec![
            Competition {
                id: 22,
                database_id: Some(29),
                name: Some("Serie A".to_owned()),
                stage_ids: vec![347, 348],
            },
            Competition {
                id: 3560,
                database_id: None,
                name: None,
                stage_ids: vec![354],
            },
        ];
        let map = stage_competitions(&competitions);
        assert_eq!(map.get(&347), Some(&22));
        assert_eq!(map.get(&348), Some(&22));
        assert_eq!(map.get(&354), Some(&3560));
        assert_eq!(map.get(&999), None);
    }
}
