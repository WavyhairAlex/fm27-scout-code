//! Season statistics from the unlisted span frame after `non_pl_hist_ls`.
//!
//! FM27 stores one record per player keyed by `pindex + 1`. A record holds up to
//! eight lines for the player's own team (one per competition kind) and six
//! lines for each other team he played for. A line is 155 bytes and is decoded
//! by fixed offsets; there is no search-based resynchronisation, so a layout
//! change fails the walk instead of shifting later records.
//!
//! Goalkeepers and outfield players share some line words. The natural-position
//! flag decides which set is decoded, exactly as the game's player profile does.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::clubs::ClubIndex;
use crate::container::{FrameSpan, SaveFile};
use crate::error::{Error, Result};
use crate::layout::{read_i16, read_u8, read_u16, read_u32};

pub const SECTION_MAGIC: &[u8] = b"\x03\x01tad.";
pub const SECTION_SCHEMA: u16 = 15;
pub const LINE_MARKER: &[u8] = b"\x01\x07";
pub const LINE_BYTES: usize = 155;
const SECTION_HEADER_BYTES: usize = SECTION_MAGIC.len() + 2;
const RECORD_PRESENT: u8 = 1;
const RECORD_END: u8 = 0;
const HEADER_BYTES: usize = 16;
const OWN_SLOTS: usize = 8;
const OTHER_TEAM_SLOTS: usize = 6;
const BLOCK_BYTES: usize = 20;
const OTHER_TEAM_LEAD_BYTES: usize = 1;
const MAX_OTHER_TEAMS: u32 = 16;
const RATING_ITEM_BYTES: usize = 8;
const MAX_RATING_ITEMS: u32 = 256;
const FORM_ENTRY_BYTES: usize = 3;
const MAX_FORM_ENTRIES: u8 = 16;
const FOOTER_BYTES: usize = 16;
const RATING_SUM_OFFSET: usize = 2;
const RATED_APPEARANCES_OFFSET: usize = 8;
const RATING_SCALE: f64 = 10.0;

/// The region name the reference implementation uses for the season-statistics
/// frame. The directory does not list that frame; it lives in the gap after
/// `non_pl_hist_ls`.
pub const REGION_NAME: &str = "unlisted_after_non_pl_hist_ls";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeasonStatsKind {
    NonCompetitive,
    League,
    Cup,
    Continental,
    International,
    Overall,
    CalendarYearOverall,
    CalendarYearInternational,
}

impl SeasonStatsKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NonCompetitive => "non_competitive",
            Self::League => "league",
            Self::Cup => "cup",
            Self::Continental => "continental",
            Self::International => "international",
            Self::Overall => "overall",
            Self::CalendarYearOverall => "calendar_year_overall",
            Self::CalendarYearInternational => "calendar_year_international",
        }
    }
}

const SLOT_KINDS: [SeasonStatsKind; OWN_SLOTS] = [
    SeasonStatsKind::NonCompetitive,
    SeasonStatsKind::League,
    SeasonStatsKind::Cup,
    SeasonStatsKind::Continental,
    SeasonStatsKind::International,
    SeasonStatsKind::Overall,
    SeasonStatsKind::CalendarYearOverall,
    SeasonStatsKind::CalendarYearInternational,
];

/// Everything the season-statistics decoder needs from a player record.
#[derive(Debug, Clone)]
pub struct SeasonPlayer {
    pub uid: u32,
    pub name: Option<String>,
    pub team_id: Option<u32>,
    pub goalkeeper: bool,
}

/// One statistics line after decoding. Counts are the game's stored counts;
/// the `_per_90` and percentage rates are computed by callers.
#[derive(Debug, Clone, Serialize)]
pub struct SeasonStatRow {
    pub player_uid: u32,
    pub player_name: Option<String>,
    pub kind: SeasonStatsKind,
    pub team_id: Option<u32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub club_short_name: Option<String>,
    pub team_slot: Option<u8>,
    pub starts: u32,
    pub substitute_appearances: u32,
    pub minutes: u32,
    pub rated_appearances: u32,
    pub average_rating: Option<f64>,
    pub player_of_the_match: u32,
    pub goals: u32,
    pub assists: u32,
    pub expected_goals: f64,
    pub non_penalty_expected_goals: f64,
    pub expected_assists: f64,
    pub shots: u32,
    pub shots_on_target: u32,
    pub shots_outside_box: u32,
    pub goals_outside_box: Option<u32>,
    pub free_kick_shots: u32,
    pub penalties_taken: u32,
    pub penalties_scored: u32,
    pub passes_attempted: u32,
    pub passes_completed: u32,
    pub progressive_passes: u32,
    pub key_passes: u32,
    pub open_play_key_passes: u32,
    pub clear_cut_chances_created: u32,
    pub crosses_attempted: u32,
    pub crosses_completed: u32,
    pub open_play_crosses_attempted: u32,
    pub open_play_crosses_completed: Option<u32>,
    pub dribbles: u32,
    pub offsides: u32,
    pub distance_km: f64,
    pub high_intensity_sprints: u32,
    pub aerial_challenges_attempted: Option<u32>,
    pub headers_won: Option<u32>,
    pub key_headers: u32,
    pub tackles_attempted: u32,
    pub tackles_completed: u32,
    pub key_tackles: u32,
    pub interceptions: u32,
    pub possession_won: u32,
    pub possession_lost: u32,
    pub pressures_attempted: Option<u32>,
    pub pressures_completed: Option<u32>,
    pub blocks: Option<u32>,
    pub shots_blocked: u32,
    pub clearances: Option<u32>,
    pub fouls_made: u32,
    pub fouls_against: u32,
    pub yellow_cards: u32,
    pub red_cards: u32,
    pub mistakes_leading_to_goal: u32,
    pub clean_sheets: u32,
    pub goals_allowed: u32,
    pub saves_held: Option<u32>,
    pub saves_parried: Option<u32>,
    pub saves_tipped: Option<u32>,
    pub shots_on_target_faced: Option<u32>,
    pub expected_goals_prevented: f64,
    /// False when FM27 zeroed the advanced block for this club or competition.
    /// The zero values are not real zeroes; callers must treat them as missing.
    pub advanced_available: bool,
}

impl SeasonStatRow {
    pub fn pass_completion_percent(&self) -> Option<f64> {
        (self.passes_attempted > 0)
            .then(|| 100.0 * f64::from(self.passes_completed) / f64::from(self.passes_attempted))
    }

    pub fn headers_won_percent(&self) -> Option<f64> {
        self.aerial_challenges_attempted
            .filter(|attempted| *attempted > 0)
            .map(|attempted| {
                100.0 * f64::from(self.headers_won.unwrap_or(0)) / f64::from(attempted)
            })
    }

    pub fn save_percent(&self) -> Option<f64> {
        let faced = self.shots_on_target_faced?;
        if faced == 0 {
            return None;
        }
        let saves = self.saves_held? + self.saves_parried? + self.saves_tipped?;
        Some(100.0 * f64::from(saves) / f64::from(faced))
    }

    pub fn per_90(&self, count: u32) -> Option<f64> {
        (self.minutes > 0).then(|| f64::from(count) * 90.0 / f64::from(self.minutes))
    }
}

#[derive(Debug, Clone)]
struct RawLine {
    starts: u32,
    substitute_appearances: u32,
    minutes: u32,
    rated_appearances: u32,
    rating_sum: u32,
    player_of_the_match: u32,
    goals: u32,
    assists: u32,
    expected_goals: f64,
    non_penalty_expected_goals: f64,
    expected_assists: f64,
    shots: u32,
    shots_on_target: u32,
    shots_outside_box: u32,
    free_kick_shots: u32,
    penalties_taken: u32,
    penalties_scored: u32,
    passes_attempted: u32,
    passes_completed: u32,
    progressive_passes: u32,
    key_passes: u32,
    open_play_key_passes: u32,
    clear_cut_chances_created: u32,
    crosses_attempted: u32,
    crosses_completed: u32,
    open_play_crosses_attempted: u32,
    open_play_crosses_completed: Option<u32>,
    dribbles: u32,
    offsides: u32,
    distance_km: f64,
    high_intensity_sprints: u32,
    aerial_challenges_attempted: Option<u32>,
    headers_won: Option<u32>,
    key_headers: u32,
    tackles_attempted: u32,
    tackles_completed: u32,
    key_tackles: u32,
    interceptions: u32,
    possession_won: u32,
    possession_lost: u32,
    blocks: Option<u32>,
    shots_blocked: u32,
    clearances: Option<u32>,
    fouls_made: u32,
    fouls_against: u32,
    yellow_cards: u32,
    red_cards: u32,
    mistakes_leading_to_goal: u32,
    clean_sheets: u32,
    goals_allowed: u32,
    saves_held: Option<u32>,
    saves_parried: Option<u32>,
    saves_tipped: Option<u32>,
    shots_on_target_faced: Option<u32>,
    expected_goals_prevented: f64,
}

#[derive(Debug, Clone)]
pub struct RawSeasonRecord {
    pub key: u32,
    pub own_lines: [Option<usize>; OWN_SLOTS],
    pub other_teams: Vec<(u32, [Option<usize>; OTHER_TEAM_SLOTS])>,
}

#[derive(Debug, Clone)]
pub struct SeasonStatsWalk {
    pub records: Vec<RawSeasonRecord>,
}

/// Read the one season-statistics frame, walk it, and decode every line that
/// belongs to a known player.
pub fn read_season_stats(
    save: &SaveFile,
    players_by_key: &HashMap<u32, SeasonPlayer>,
    clubs: &ClubIndex,
) -> Result<Vec<SeasonStatRow>> {
    let header = section_header();
    let heads = save.region_frame_heads(REGION_NAME, header.len())?;
    let candidates: Vec<FrameSpan> = heads
        .into_iter()
        .filter(|(_, head)| head == &header)
        .map(|(span, _)| span)
        .collect();
    if candidates.is_empty() {
        return Err(Error::parse(format!(
            "no frame of {REGION_NAME:?} holds the season-statistics section"
        )));
    }
    if candidates.len() > 1 {
        return Err(Error::parse(format!(
            "{} frames of {REGION_NAME:?} each start with the season-statistics header",
            candidates.len()
        )));
    }
    let body = save.read_region_frame(REGION_NAME, candidates[0])?;
    let walk = walk_season_stats(&body)?;
    let rows = decode_records(&body, &walk, players_by_key, clubs)?;
    Ok(merge_duplicate_rows(rows))
}

/// FM can store more than one line for the same player, competition kind and
/// team when a season contains two spells there. The SQLite cache identifies a
/// row by that key, so fold those lines into one season total instead of
/// failing the insert.
fn merge_duplicate_rows(rows: Vec<SeasonStatRow>) -> Vec<SeasonStatRow> {
    let mut merged: Vec<SeasonStatRow> = Vec::with_capacity(rows.len());
    let mut seen: HashMap<(u32, SeasonStatsKind, u32), usize> = HashMap::new();
    for row in rows {
        let key = (row.player_uid, row.kind, row.team_id.unwrap_or(0));
        if let Some(index) = seen.get(&key).copied() {
            merge_row(&mut merged[index], row);
        } else {
            seen.insert(key, merged.len());
            merged.push(row);
        }
    }
    merged
}

fn merge_row(target: &mut SeasonStatRow, source: SeasonStatRow) {
    let target_rated = target.rated_appearances;
    let source_rated = source.rated_appearances;
    let target_rating_sum =
        target.average_rating.unwrap_or(0.0) * f64::from(target_rated);
    let source_rating_sum =
        source.average_rating.unwrap_or(0.0) * f64::from(source_rated);
    let total_rated = target_rated + source_rated;

    target.player_name = target.player_name.take().or(source.player_name);
    target.club_uid = target.club_uid.or(source.club_uid);
    target.club_name = target.club_name.take().or(source.club_name);
    target.club_short_name = target.club_short_name.take().or(source.club_short_name);
    target.team_slot = target.team_slot.or(source.team_slot);
    target.starts += source.starts;
    target.substitute_appearances += source.substitute_appearances;
    target.minutes += source.minutes;
    target.rated_appearances += source.rated_appearances;
    target.average_rating = (total_rated > 0)
        .then(|| (target_rating_sum + source_rating_sum) / f64::from(total_rated));
    target.player_of_the_match += source.player_of_the_match;
    target.goals += source.goals;
    target.assists += source.assists;
    target.expected_goals += source.expected_goals;
    target.non_penalty_expected_goals += source.non_penalty_expected_goals;
    target.expected_assists += source.expected_assists;
    target.shots += source.shots;
    target.shots_on_target += source.shots_on_target;
    target.shots_outside_box += source.shots_outside_box;
    target.goals_outside_box = add_optional(target.goals_outside_box, source.goals_outside_box);
    target.free_kick_shots += source.free_kick_shots;
    target.penalties_taken += source.penalties_taken;
    target.penalties_scored += source.penalties_scored;
    target.passes_attempted += source.passes_attempted;
    target.passes_completed += source.passes_completed;
    target.progressive_passes += source.progressive_passes;
    target.key_passes += source.key_passes;
    target.open_play_key_passes += source.open_play_key_passes;
    target.clear_cut_chances_created += source.clear_cut_chances_created;
    target.crosses_attempted += source.crosses_attempted;
    target.crosses_completed += source.crosses_completed;
    target.open_play_crosses_attempted += source.open_play_crosses_attempted;
    target.open_play_crosses_completed = add_optional(
        target.open_play_crosses_completed,
        source.open_play_crosses_completed,
    );
    target.dribbles += source.dribbles;
    target.offsides += source.offsides;
    target.distance_km += source.distance_km;
    target.high_intensity_sprints += source.high_intensity_sprints;
    target.aerial_challenges_attempted = add_optional(
        target.aerial_challenges_attempted,
        source.aerial_challenges_attempted,
    );
    target.headers_won = add_optional(target.headers_won, source.headers_won);
    target.key_headers += source.key_headers;
    target.tackles_attempted += source.tackles_attempted;
    target.tackles_completed += source.tackles_completed;
    target.key_tackles += source.key_tackles;
    target.interceptions += source.interceptions;
    target.possession_won += source.possession_won;
    target.possession_lost += source.possession_lost;
    target.pressures_attempted =
        add_optional(target.pressures_attempted, source.pressures_attempted);
    target.pressures_completed =
        add_optional(target.pressures_completed, source.pressures_completed);
    target.blocks = add_optional(target.blocks, source.blocks);
    target.shots_blocked += source.shots_blocked;
    target.clearances = add_optional(target.clearances, source.clearances);
    target.fouls_made += source.fouls_made;
    target.fouls_against += source.fouls_against;
    target.yellow_cards += source.yellow_cards;
    target.red_cards += source.red_cards;
    target.mistakes_leading_to_goal += source.mistakes_leading_to_goal;
    target.clean_sheets += source.clean_sheets;
    target.goals_allowed += source.goals_allowed;
    target.saves_held = add_optional(target.saves_held, source.saves_held);
    target.saves_parried = add_optional(target.saves_parried, source.saves_parried);
    target.saves_tipped = add_optional(target.saves_tipped, source.saves_tipped);
    target.shots_on_target_faced =
        add_optional(target.shots_on_target_faced, source.shots_on_target_faced);
    target.expected_goals_prevented += source.expected_goals_prevented;
    target.advanced_available |= source.advanced_available;
}

fn add_optional(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn section_header() -> Vec<u8> {
    let mut header = SECTION_MAGIC.to_vec();
    header.extend_from_slice(&SECTION_SCHEMA.to_le_bytes());
    header
}

/// Walk the section body without decoding line fields. Every count, slot and
/// footer is checked so a moved layout fails closed.
pub fn walk_season_stats(body: &[u8]) -> Result<SeasonStatsWalk> {
    let header = section_header();
    if !body.starts_with(&header) {
        return Err(Error::parse(
            "the season-statistics section does not start with its header",
        ));
    }
    let mut records = Vec::new();
    let mut keys = HashSet::new();
    let mut position = SECTION_HEADER_BYTES;
    loop {
        if position >= body.len() {
            return Err(Error::parse(
                "the season-statistics record list runs past the section",
            ));
        }
        let marker = body[position];
        if marker == RECORD_END {
            break;
        }
        if marker != RECORD_PRESENT || position + 1 + 4 + HEADER_BYTES > body.len() {
            return Err(Error::parse(format!(
                "the season-statistics record at byte {position} does not start as one"
            )));
        }
        let key = read_u32(body, position + 1)
            .ok_or_else(|| Error::parse("a season-statistics record key is truncated"))?;
        if !keys.insert(key) {
            return Err(Error::parse(format!(
                "the season-statistics records repeat player key {key}"
            )));
        }
        let mut own_lines = [None; OWN_SLOTS];
        position += 1 + 4 + HEADER_BYTES;
        for line in &mut own_lines {
            let (slot, next) = read_slot(body, position)?;
            *line = slot;
            position = next;
        }
        position += BLOCK_BYTES;
        let other_count = read_count(body, position, MAX_OTHER_TEAMS, "other teams")?;
        position += 4;
        let mut other_teams = Vec::with_capacity(other_count as usize);
        for _ in 0..other_count {
            let team_at = position + OTHER_TEAM_LEAD_BYTES;
            let team_id = read_u32(body, team_at)
                .ok_or_else(|| Error::parse("a season-statistics team id is truncated"))?;
            position = team_at + 4;
            let mut lines = [None; OTHER_TEAM_SLOTS];
            for line in &mut lines {
                let (slot, next) = read_slot(body, position)?;
                *line = slot;
                position = next;
            }
            other_teams.push((team_id, lines));
        }
        let rating_items = read_count(body, position, MAX_RATING_ITEMS, "rating items")?;
        position += 4;
        let rating_bytes = usize::try_from(rating_items)
            .map_err(|_| Error::parse("a season-statistics rating list is too large"))?
            .checked_mul(RATING_ITEM_BYTES)
            .ok_or_else(|| Error::parse("a season-statistics rating list is too large"))?;
        position = position
            .checked_add(rating_bytes)
            .ok_or_else(|| Error::parse("a season-statistics rating list runs past the section"))?;
        let form_entries = *body
            .get(position)
            .ok_or_else(|| Error::parse("a season-statistics form list is truncated"))?;
        if form_entries > MAX_FORM_ENTRIES {
            return Err(Error::parse(format!(
                "a season-statistics form list counts {form_entries} entries"
            )));
        }
        position = position
            .checked_add(1 + usize::from(form_entries) * FORM_ENTRY_BYTES)
            .ok_or_else(|| Error::parse("a season-statistics form list runs past the section"))?;
        records.push(RawSeasonRecord {
            key,
            own_lines,
            other_teams,
        });
    }
    let footer_end = position
        .checked_add(1 + FOOTER_BYTES)
        .ok_or_else(|| Error::parse("the season-statistics footer overflows the section"))?;
    if footer_end != body.len() {
        return Err(Error::parse(format!(
            "the season-statistics list ends {} bytes away from its footer",
            body.len() as isize - footer_end as isize
        )));
    }
    Ok(SeasonStatsWalk { records })
}

fn read_slot(body: &[u8], position: usize) -> Result<(Option<usize>, usize)> {
    let Some(byte) = body.get(position) else {
        return Err(Error::parse(
            "a season-statistics slot runs past the section",
        ));
    };
    if *byte == 0 {
        return Ok((None, position + 1));
    }
    let end = position
        .checked_add(LINE_BYTES)
        .ok_or_else(|| Error::parse("a season-statistics line overflows the section"))?;
    if end > body.len() || !body[position..].starts_with(LINE_MARKER) {
        return Err(Error::parse(format!(
            "the season-statistics slot at byte {position} is neither absent nor a line"
        )));
    }
    Ok((Some(position), end))
}

fn read_count(body: &[u8], position: usize, maximum: u32, what: &str) -> Result<u32> {
    let count = read_u32(body, position)
        .ok_or_else(|| Error::parse(format!("a season-statistics {what} count is truncated")))?;
    if count > maximum {
        return Err(Error::parse(format!(
            "a season-statistics record counts {count} {what}"
        )));
    }
    Ok(count)
}

fn decode_records(
    body: &[u8],
    walk: &SeasonStatsWalk,
    players_by_key: &HashMap<u32, SeasonPlayer>,
    clubs: &ClubIndex,
) -> Result<Vec<SeasonStatRow>> {
    let mut rows = Vec::new();
    for record in &walk.records {
        let Some(player) = players_by_key.get(&record.key) else {
            continue;
        };
        for (index, line) in record.own_lines.iter().enumerate() {
            let Some(offset) = line else {
                continue;
            };
            let raw = decode_line(body, *offset, player.goalkeeper)?;
            rows.push(build_row(
                raw,
                player,
                player.team_id,
                SLOT_KINDS[index],
                clubs,
            ));
        }
        for (team_id, lines) in &record.other_teams {
            for (index, line) in lines.iter().enumerate() {
                let Some(offset) = line else {
                    continue;
                };
                let raw = decode_line(body, *offset, player.goalkeeper)?;
                rows.push(build_row(
                    raw,
                    player,
                    Some(*team_id),
                    SLOT_KINDS[index],
                    clubs,
                ));
            }
        }
    }
    Ok(rows)
}

fn build_row(
    raw: RawLine,
    player: &SeasonPlayer,
    team_id: Option<u32>,
    kind: SeasonStatsKind,
    clubs: &ClubIndex,
) -> SeasonStatRow {
    let (club_uid, club_name, club_short_name, team_slot) = team_id
        .and_then(|team| clubs.team_fields(team))
        .map_or((None, None, None, None), |(uid, club, slot)| {
            (
                Some(uid),
                Some(club.name.clone()),
                Some(club.short_name.clone()),
                Some(slot),
            )
        });
    SeasonStatRow {
        player_uid: player.uid,
        player_name: player.name.clone(),
        kind,
        team_id,
        club_uid,
        club_name,
        club_short_name,
        team_slot,
        starts: raw.starts,
        substitute_appearances: raw.substitute_appearances,
        minutes: raw.minutes,
        rated_appearances: raw.rated_appearances,
        average_rating: (raw.rated_appearances > 0)
            .then(|| f64::from(raw.rating_sum) / (f64::from(raw.rated_appearances) * RATING_SCALE)),
        player_of_the_match: raw.player_of_the_match,
        goals: raw.goals,
        assists: raw.assists,
        expected_goals: raw.expected_goals,
        non_penalty_expected_goals: raw.non_penalty_expected_goals,
        expected_assists: raw.expected_assists,
        shots: raw.shots,
        shots_on_target: raw.shots_on_target,
        shots_outside_box: raw.shots_outside_box,
        // FM27 does not store this field; keep it explicit rather than guessing.
        goals_outside_box: None,
        free_kick_shots: raw.free_kick_shots,
        penalties_taken: raw.penalties_taken,
        penalties_scored: raw.penalties_scored,
        passes_attempted: raw.passes_attempted,
        passes_completed: raw.passes_completed,
        progressive_passes: raw.progressive_passes,
        key_passes: raw.key_passes,
        open_play_key_passes: raw.open_play_key_passes,
        clear_cut_chances_created: raw.clear_cut_chances_created,
        crosses_attempted: raw.crosses_attempted,
        crosses_completed: raw.crosses_completed,
        open_play_crosses_attempted: raw.open_play_crosses_attempted,
        open_play_crosses_completed: raw.open_play_crosses_completed,
        dribbles: raw.dribbles,
        offsides: raw.offsides,
        distance_km: raw.distance_km,
        high_intensity_sprints: raw.high_intensity_sprints,
        aerial_challenges_attempted: raw.aerial_challenges_attempted,
        headers_won: raw.headers_won,
        key_headers: raw.key_headers,
        tackles_attempted: raw.tackles_attempted,
        tackles_completed: raw.tackles_completed,
        key_tackles: raw.key_tackles,
        interceptions: raw.interceptions,
        possession_won: raw.possession_won,
        possession_lost: raw.possession_lost,
        pressures_attempted: None,
        pressures_completed: None,
        blocks: raw.blocks,
        shots_blocked: raw.shots_blocked,
        clearances: raw.clearances,
        fouls_made: raw.fouls_made,
        fouls_against: raw.fouls_against,
        yellow_cards: raw.yellow_cards,
        red_cards: raw.red_cards,
        mistakes_leading_to_goal: raw.mistakes_leading_to_goal,
        clean_sheets: raw.clean_sheets,
        goals_allowed: raw.goals_allowed,
        saves_held: raw.saves_held,
        saves_parried: raw.saves_parried,
        saves_tipped: raw.saves_tipped,
        shots_on_target_faced: raw.shots_on_target_faced,
        expected_goals_prevented: raw.expected_goals_prevented,
        advanced_available: raw.distance_km > 0.0,
    }
}

fn decode_line(body: &[u8], offset: usize, goalkeeper: bool) -> Result<RawLine> {
    let line = body
        .get(offset..offset + LINE_BYTES)
        .ok_or_else(|| Error::parse("a season-statistics line runs past the section"))?;
    let u8_at = |at: usize| -> Result<u32> {
        read_u8(line, at)
            .map(u32::from)
            .ok_or_else(|| Error::parse("a season-statistics line field is truncated"))
    };
    let u16_at = |at: usize| -> Result<u32> {
        read_u16(line, at)
            .map(u32::from)
            .ok_or_else(|| Error::parse("a season-statistics line field is truncated"))
    };
    let rating_sum = u16_at(RATING_SUM_OFFSET)?;
    let rated_appearances = u8_at(RATED_APPEARANCES_OFFSET)?;
    let (
        aerial_challenges_attempted,
        headers_won,
        open_play_crosses_completed,
        blocks,
        clearances,
        saves,
    ) = if goalkeeper {
        (
            None,
            None,
            None,
            None,
            None,
            Some((u16_at(35)?, u16_at(37)?, u16_at(69)?, u16_at(83)?)),
        )
    } else {
        (
            Some(u16_at(35)?),
            Some(u16_at(37)?),
            Some(u16_at(71)?),
            Some(u16_at(83)?),
            Some(u16_at(95)?),
            None,
        )
    };
    let (saves_held, saves_parried, saves_tipped, shots_on_target_faced) = saves.map_or(
        (None, None, None, None),
        |(held, parried, tipped, faced)| (Some(held), Some(parried), Some(tipped), Some(faced)),
    );
    Ok(RawLine {
        starts: u8_at(6)?,
        substitute_appearances: u8_at(7)?,
        minutes: u16_at(4)?,
        rated_appearances,
        rating_sum,
        player_of_the_match: u8_at(14)?,
        goals: u8_at(9)?,
        assists: u8_at(10)?,
        expected_goals: f64::from(u16_at(91)?) / 100.0,
        non_penalty_expected_goals: f64::from(u16_at(127)?) / 100.0,
        expected_assists: f64::from(u16_at(93)?) / 100.0,
        shots: u16_at(45)?,
        shots_on_target: u16_at(47)?,
        shots_outside_box: u16_at(111)?,
        free_kick_shots: u16_at(99)?,
        penalties_taken: u8_at(12)?,
        penalties_scored: u8_at(13)?,
        passes_attempted: u16_at(27)?,
        passes_completed: u16_at(29)?,
        progressive_passes: u16_at(119)?,
        key_passes: u16_at(73)?,
        open_play_key_passes: u16_at(125)?,
        clear_cut_chances_created: u16_at(81)?,
        crosses_attempted: u16_at(131)?,
        crosses_completed: u16_at(133)?,
        open_play_crosses_attempted: u16_at(109)?,
        open_play_crosses_completed,
        dribbles: u16_at(39)?,
        offsides: u16_at(67)?,
        distance_km: f64::from(u16_at(65)?) / 10.0,
        high_intensity_sprints: u16_at(115)?,
        aerial_challenges_attempted,
        headers_won,
        key_headers: u16_at(77)?,
        tackles_attempted: u16_at(31)?,
        tackles_completed: u16_at(33)?,
        key_tackles: u16_at(75)?,
        interceptions: u16_at(79)?,
        possession_won: u16_at(89)?,
        possession_lost: u16_at(87)?,
        blocks,
        shots_blocked: u16_at(135)?,
        clearances,
        fouls_made: u16_at(41)?,
        fouls_against: u16_at(43)?,
        yellow_cards: u8_at(16)?,
        red_cards: u8_at(15)?,
        mistakes_leading_to_goal: u16_at(59)?,
        clean_sheets: u8_at(17)?,
        goals_allowed: u8_at(11)?,
        saves_held,
        saves_parried,
        saves_tipped,
        shots_on_target_faced,
        expected_goals_prevented: f64::from(
            read_i16(line, 113)
                .ok_or_else(|| Error::parse("a season-statistics line field is truncated"))?,
        ) / 100.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Vec<u8> {
        let mut line = vec![0u8; LINE_BYTES];
        line[..2].copy_from_slice(LINE_MARKER);
        line[4..6].copy_from_slice(&900u16.to_le_bytes());
        line[6] = 10;
        line[7] = 2;
        line[8] = 12;
        line[9] = 4;
        line[10] = 3;
        line[27..29].copy_from_slice(&500u16.to_le_bytes());
        line[29..31].copy_from_slice(&400u16.to_le_bytes());
        line[45..47].copy_from_slice(&20u16.to_le_bytes());
        line[47..49].copy_from_slice(&8u16.to_le_bytes());
        line[91..93].copy_from_slice(&450u16.to_le_bytes());
        line[93..95].copy_from_slice(&300u16.to_le_bytes());
        line[35..37].copy_from_slice(&40u16.to_le_bytes());
        line[37..39].copy_from_slice(&25u16.to_le_bytes());
        line[2..4].copy_from_slice(&840u16.to_le_bytes());
        line
    }

    fn body_with_one_record() -> Vec<u8> {
        let mut body = section_header();
        body.push(RECORD_PRESENT);
        body.extend_from_slice(&42u32.to_le_bytes());
        body.extend_from_slice(&[0u8; HEADER_BYTES]);
        let line_offset = body.len();
        body.extend_from_slice(&line());
        for _ in 1..OWN_SLOTS {
            body.push(0);
        }
        body.extend_from_slice(&[0u8; BLOCK_BYTES]);
        body.extend_from_slice(&0u32.to_le_bytes()); // other teams
        body.extend_from_slice(&0u32.to_le_bytes()); // rating items
        body.push(0); // form entries
        body.push(RECORD_END);
        body.extend_from_slice(&[0u8; FOOTER_BYTES]);
        assert_eq!(
            body.len(),
            line_offset + LINE_BYTES + 7 + BLOCK_BYTES + 4 + 4 + 1 + 1 + 16
        );
        body
    }

    #[test]
    fn walks_and_decodes_a_record() {
        let body = body_with_one_record();
        let walk = walk_season_stats(&body).unwrap();
        assert_eq!(walk.records.len(), 1);
        let record = &walk.records[0];
        assert_eq!(record.key, 42);
        assert!(record.own_lines[0].is_some());
        assert!(record.own_lines[1..].iter().all(Option::is_none));

        let players = HashMap::from([(
            42,
            SeasonPlayer {
                uid: 7,
                name: Some("Test Player".to_owned()),
                team_id: None,
                goalkeeper: false,
            },
        )]);
        let clubs = ClubIndex::default();
        let rows = decode_records(&body, &walk, &players, &clubs).unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.player_uid, 7);
        assert_eq!(row.kind, SeasonStatsKind::NonCompetitive);
        assert_eq!(row.minutes, 900);
        assert_eq!(row.starts, 10);
        assert_eq!(row.goals, 4);
        assert_eq!(row.expected_goals, 4.5);
        assert_eq!(row.expected_assists, 3.0);
        assert_eq!(row.average_rating, Some(7.0));
        assert_eq!(row.headers_won, Some(25));
        assert_eq!(row.aerial_challenges_attempted, Some(40));
        assert_eq!(row.headers_won_percent(), Some(62.5));
    }

    #[test]
    fn rejects_a_repeated_player_key() {
        let mut body = body_with_one_record();
        // Add a second identical record before the terminator: the key repeats.
        let second = body[8..body.len() - 17].to_vec();
        let insert_at = body.len() - 17;
        body.splice(insert_at..insert_at, second);
        let error = walk_season_stats(&body).unwrap_err();
        assert!(error.to_string().contains("repeat"));
    }

    #[test]
    fn merges_duplicate_lines_for_the_same_team_and_kind() {
        let mut body = section_header();
        body.push(RECORD_PRESENT);
        body.extend_from_slice(&42u32.to_le_bytes());
        body.extend_from_slice(&[0u8; HEADER_BYTES]);
        let line = line();
        body.extend_from_slice(&line); // own line, non-competitive slot
        for _ in 1..OWN_SLOTS {
            body.push(0);
        }
        body.extend_from_slice(&[0u8; BLOCK_BYTES]);
        body.extend_from_slice(&1u32.to_le_bytes()); // one other team
        body.push(0); // other-team lead byte
        body.extend_from_slice(&123u32.to_le_bytes());
        body.extend_from_slice(&line); // same team and kind again
        for _ in 1..OTHER_TEAM_SLOTS {
            body.push(0);
        }
        body.extend_from_slice(&0u32.to_le_bytes()); // rating items
        body.push(0); // form entries
        body.push(RECORD_END);
        body.extend_from_slice(&[0u8; FOOTER_BYTES]);

        let walk = walk_season_stats(&body).unwrap();
        let players = HashMap::from([(
            42,
            SeasonPlayer {
                uid: 7,
                name: Some("Test Player".to_owned()),
                team_id: Some(123),
                goalkeeper: false,
            },
        )]);
        let clubs = ClubIndex::default();
        let rows = decode_records(&body, &walk, &players, &clubs).unwrap();
        assert_eq!(rows.len(), 2);

        let rows = merge_duplicate_rows(rows);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].minutes, 1_800);
        assert_eq!(rows[0].starts, 20);
        assert_eq!(rows[0].goals, 8);
        assert_eq!(rows[0].rated_appearances, 24);
        assert_eq!(rows[0].average_rating, Some(7.0));
    }

    #[test]
    fn rejects_a_bad_footer_length() {
        let mut body = body_with_one_record();
        body.pop();
        let error = walk_season_stats(&body).unwrap_err();
        assert!(error.to_string().contains("footer"));
    }
}
