//! End-to-end parsing of one save into normalized player and club rows.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::NaiveDate;
use serde::Serialize;

use crate::clubs::ClubIndex;
use crate::competitions::CompetitionIndex;
use crate::container::SaveFile;
use crate::contracts::ContractDecoder;
use crate::error::{Error, Result};
use crate::finances::read_club_finances;
use crate::layout::{
    ATTRIBUTE_KEYS, MISSING_U32, PARSER_VERSION, POSITION_NAMES, PlayerLayout, decode_date,
    read_i16, read_u8, read_u16, read_u32, scale_attribute, stored_attribute_index,
};
use crate::metadata::{SaveMetadata, read_metadata};
use crate::model::{
    Attributes, Club, Competition, FinanceMonth, ParseReport, Player, Positions, Reputation,
    SaveSummary, Stadium, TransferValueState,
};
use crate::names::NamePools;
use crate::nations::NationIndex;
use crate::persons::{ClubLookup, PersonDecoder};
use crate::player_scan::PlayerRecords;
use crate::stadiums::read_stadium_index;

#[derive(Debug, Clone, Serialize)]
pub struct ParseProgress {
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Debug)]
pub struct ParsedSave {
    pub metadata: SaveMetadata,
    pub summary: SaveSummary,
    pub players: Vec<Player>,
    pub clubs: Vec<Club>,
    pub competitions: Vec<Competition>,
    pub club_finances: Vec<FinanceMonth>,
    pub stadiums: Vec<Stadium>,
    pub report: ParseReport,
}

pub fn parse_save(path: impl AsRef<Path>) -> Result<ParsedSave> {
    parse_save_with_progress(path, |_| {})
}

pub fn parse_save_with_progress(
    path: impl AsRef<Path>,
    mut progress: impl FnMut(ParseProgress),
) -> Result<ParsedSave> {
    let started = Instant::now();
    progress(ParseProgress {
        stage: "opening".to_owned(),
        current: 0,
        total: 0,
        message: "正在读取存档容器".to_owned(),
    });
    let path = path.as_ref();
    let save = SaveFile::open(path)?;
    let metadata = read_metadata(&save)?;
    let reference_date = metadata
        .game_date
        .as_deref()
        .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
    progress(ParseProgress {
        stage: "game_db".to_owned(),
        current: 0,
        total: 0,
        message: "正在解压 game_db".to_owned(),
    });
    let game_db = save.read_section("game_db")?;
    progress(ParseProgress {
        stage: "names".to_owned(),
        current: 0,
        total: 0,
        message: "正在定位姓名池".to_owned(),
    });
    let names = NamePools::locate(&game_db)?;
    let nations = NationIndex::read(&game_db).unwrap_or_default();
    progress(ParseProgress {
        stage: "players".to_owned(),
        current: 0,
        total: 0,
        message: "正在扫描球员记录".to_owned(),
    });
    let record_index = PlayerRecords::locate(&game_db, names.end_offset)?;
    progress(ParseProgress {
        stage: "clubs".to_owned(),
        current: 0,
        total: 0,
        message: "正在解析俱乐部与球队".to_owned(),
    });
    let mut clubs = ClubIndex::read(&game_db)?;
    for club in &mut clubs.clubs {
        club.nation_name = nations.name_for(club.nation_id as u16).map(str::to_owned);
        club.fa_nation_name = nations
            .name_for(club.fa_nation_id as u16)
            .map(str::to_owned);
    }
    let finance_data = read_club_finances(&game_db, &clubs, Some(reference_date));
    for club in &mut clubs.clubs {
        club.corporate_facilities = finance_data
            .corporate_facilities
            .get(&club.uid)
            .copied()
            .flatten();
    }
    let tracked_clubs: HashSet<u32> = finance_data.corporate_facilities.keys().copied().collect();
    let facility_data = crate::facilities::read_club_facilities(&game_db, &clubs, &tracked_clubs);
    for club in &mut clubs.clubs {
        let Some(facilities) = facility_data.get(&club.uid) else {
            continue;
        };
        club.training_facilities = facilities.training;
        club.youth_facilities = facilities.youth;
        club.junior_coaching = facilities.junior_coaching;
        club.youth_recruitment = facilities.youth_recruitment;
    }
    let youth_levels = crate::youth_level::read_english_youth_levels();
    if !youth_levels.is_empty() {
        for club in &mut clubs.clubs {
            club.youth_level = youth_levels.get(&club.uid).copied();
        }
    }
    let stadiums = read_stadium_index(&game_db, &clubs).stadiums;

    progress(ParseProgress {
        stage: "competitions".to_owned(),
        current: 0,
        total: 0,
        message: "正在解析赛事索引".to_owned(),
    });
    let mut warnings = Vec::new();
    if stadiums.is_empty() {
        warnings.push("stadium table unavailable".to_owned());
    }
    let competitions = match CompetitionIndex::read(&game_db) {
        Ok(index) => {
            if index.stage_gaps > 0 {
                warnings.push(format!(
                    "{} gaps were skipped while walking the stage table",
                    index.stage_gaps
                ));
            }
            index
        }
        Err(error) => {
            warnings.push(format!("competition index unavailable: {error}"));
            CompetitionIndex::default()
        }
    };

    let mut injuries = if save.sections.contains_key("injury_manager") {
        save.read_section("injury_manager")
            .ok()
            .map(|section| {
                crate::injuries::read_injuries(
                    &section,
                    &record_index,
                    &clubs,
                    metadata
                        .game_date
                        .as_deref()
                        .and_then(|text| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()),
                )
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let mut suspensions =
        crate::suspensions::read_suspensions(&game_db, &record_index, &nations, &competitions);

    let club_lookup = ClubLookup {
        name_by_uid: &clubs.name_by_uid,
        uid_by_index: &clubs.uid_by_index,
    };
    let person_decoder = PersonDecoder {
        names: &names,
        clubs: club_lookup,
        clock: reference_date,
    };
    let contract_decoder = ContractDecoder {
        club_names: &clubs.name_by_uid,
        club_by_team: &clubs.fielding_team_to_club,
        clock: reference_date,
    };

    let mut players = Vec::with_capacity(record_index.records.len());
    let mut with_person = 0usize;
    let mut with_club = 0usize;
    let mut with_contract = 0usize;

    for position in 0..record_index.records.len() {
        if position % 5000 == 0 {
            progress(ParseProgress {
                stage: "decode".to_owned(),
                current: position,
                total: record_index.records.len(),
                message: format!("正在解码球员 {}/{}", position, record_index.records.len()),
            });
        }
        let record = record_index.records[position];
        let offset = record.offset;
        let window_end = record_index.window_end(position, game_db.len());
        let next_uid = record_index.records.get(position + 1).map(|next| next.uid);

        let team_id_raw =
            read_u32(&game_db, offset + PlayerLayout::TEAM_ID_OFFSET).unwrap_or(MISSING_U32);
        let team_id = (team_id_raw != MISSING_U32).then_some(team_id_raw);
        let team_fields = team_id.and_then(|team| clubs.team_fields(team));

        let (
            club_uid,
            club_name,
            club_short_name,
            club_nation_id,
            club_nation_name,
            club_fa_nation_id,
            club_reputation,
            club_last_league_position,
            team_slot,
            gender,
        ) = match team_fields {
            Some((uid, club, slot)) => (
                Some(uid),
                Some(club.name.clone()),
                Some(club.short_name.clone()),
                Some(club.nation_id),
                club.nation_name.clone(),
                Some(club.fa_nation_id),
                club.reputation,
                club.last_league_position,
                Some(slot),
                club.gender,
            ),
            None => (None, None, None, None, None, None, None, None, None, None),
        };

        let closing = closing_person_header(
            &game_db,
            record.pindex,
            offset + PlayerLayout::DECODE_EXTENT,
            window_end,
            record.uid,
            next_uid,
        );
        let unique_id = closing.map(|(_, uid)| uid);
        let person_end = closing.map_or(window_end, |(at, _)| at);
        let person = person_decoder.decode(&game_db, offset + 100, person_end);
        if person.is_some() {
            with_person += 1;
        }
        if club_uid.is_some() {
            with_club += 1;
        }

        let contract = contract_decoder.decode(
            &game_db,
            offset,
            window_end,
            position + 1 == record_index.records.len(),
            record.pindex,
            club_uid,
        );
        if contract.is_some() {
            with_contract += 1;
        }

        let raw_left_foot = read_u8(
            &game_db,
            offset + PlayerLayout::ATTRIBUTES_OFFSET + PlayerLayout::LEFT_FOOT_INDEX,
        )
        .unwrap_or(0);
        let raw_right_foot = read_u8(
            &game_db,
            offset + PlayerLayout::ATTRIBUTES_OFFSET + PlayerLayout::RIGHT_FOOT_INDEX,
        )
        .unwrap_or(0);

        let mut raw_attributes = Attributes::default();
        let mut attributes = Attributes::default();
        let attribute_start = offset + PlayerLayout::ATTRIBUTES_OFFSET;
        for (index, key) in ATTRIBUTE_KEYS.iter().enumerate() {
            // The stored 54-byte slice contains two foot bytes at indices 24 and 25.
            let stored_index = stored_attribute_index(key).unwrap_or(index);
            let raw = read_u8(&game_db, attribute_start + stored_index).unwrap_or(0);
            raw_attributes.values[index] = raw;
            attributes.values[index] = scale_attribute(raw);
        }

        let mut positions = Positions::default();
        for index in 0..PlayerLayout::RATINGS_COUNT {
            positions.values[index] =
                read_u8(&game_db, offset + PlayerLayout::RATINGS_OFFSET + index).unwrap_or(1);
        }
        let (natural_positions, accomplished_positions) = derive_positions(&positions.values);

        let raw_pa = read_i16(&game_db, offset + PlayerLayout::PA_OFFSET).unwrap_or(-1);
        let (pa, pa_range_code) = if raw_pa < 0 {
            (None, Some(raw_pa))
        } else {
            (Some(raw_pa), None)
        };
        let ca = read_u16(&game_db, offset + PlayerLayout::CA_OFFSET).unwrap_or(0);
        let transfer_value_raw =
            read_u32(&game_db, offset + PlayerLayout::TRANSFER_VALUE_OFFSET).unwrap_or(MISSING_U32);
        let (transfer_value, transfer_value_state) = match transfer_value_raw {
            MISSING_U32 => (None, TransferValueState::Unset),
            PlayerLayout::TRANSFER_VALUE_PLACEHOLDER => (None, TransferValueState::Placeholder),
            0 => (None, TransferValueState::Zero),
            value => (Some(value), TransferValueState::Ok),
        };

        let (
            person_name,
            first_name,
            last_name,
            common_name,
            full_name,
            legal_name,
            birth_date,
            age,
            nation_id,
            second_nation_ids,
            home_grown_nation_ids,
            home_grown_club_uids,
            home_grown_club_names,
            personality,
            trait_bits,
            traits,
        ) = match person {
            Some(person) => (
                person.name,
                person.first_name,
                person.last_name,
                person.common_name,
                person.full_name,
                person.legal_name,
                person.birth_date,
                person.age,
                Some(person.nation_id),
                person.second_nation_ids,
                person.home_grown_nation_ids,
                person.home_grown_club_uids,
                person.home_grown_club_names,
                Some(person.personality),
                Some(person.trait_bits),
                person.traits,
            ),
            None => (
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                None,
                None,
                Vec::new(),
            ),
        };

        let reputation = Reputation {
            bucket: read_u8(&game_db, offset + PlayerLayout::REPUTATION_BUCKET_OFFSET).unwrap_or(0),
            home: read_u16(&game_db, offset - 6).unwrap_or(0),
            current: read_u16(&game_db, offset - 4).unwrap_or(0),
            world: read_u16(&game_db, offset - 2).unwrap_or(0),
        };
        let club_join_date = decode_date(&game_db, offset + PlayerLayout::CLUB_JOIN_DATE_OFFSET);
        let height_cm = read_u8(&game_db, offset + PlayerLayout::HEIGHT_OFFSET).unwrap_or(0);
        let raw_condition =
            read_u16(&game_db, offset + PlayerLayout::CONDITION_OFFSET).unwrap_or(0);
        let raw_match_sharpness =
            read_u16(&game_db, offset + PlayerLayout::MATCH_SHARPNESS_OFFSET).unwrap_or(0);

        let (on_loan, loan_parent_club_uid, loan_parent_club_name, loan_start, loan_end) =
            match &contract {
                Some(contract) => (
                    contract.on_loan,
                    contract.loan_parent_club_uid,
                    contract.loan_parent_club_name.clone(),
                    contract.loan_start.clone(),
                    contract.loan_end.clone(),
                ),
                None => (None, None, None, None, None),
            };

        let player_injuries = injuries.remove(&position).unwrap_or_default();
        let player_suspensions = suspensions.remove(&position).unwrap_or_default();
        let best_role = crate::roles::best_role(&attributes, &positions);
        let (transfer_listed, loan_listed) = contract.as_ref().map_or((false, false), |contract| {
            (contract.transfer_listed, contract.loan_listed)
        });

        players.push(Player {
            uid: record.uid,
            unique_id,
            name: person_name,
            first_name,
            last_name,
            common_name,
            full_name,
            legal_name,
            birth_date: birth_date.map(|date| date.to_string()),
            age,
            nation_id,
            nation_name: nation_id
                .and_then(|id| nations.name_for(id))
                .map(str::to_owned),
            gender,
            second_nation_ids,
            home_grown_nation_ids,
            home_grown_club_uids,
            home_grown_club_names,
            height_cm,
            ca,
            pa,
            pa_range_code,
            reputation,
            club_uid,
            club_name,
            club_short_name,
            club_nation_id,
            club_nation_name,
            club_fa_nation_id,
            club_reputation,
            club_last_league_position,
            team_id,
            team_slot,
            club_join_date: club_join_date.map(|date| date.to_string()),
            natural_positions,
            accomplished_positions,
            personality,
            attributes,
            raw_attributes,
            left_foot: scale_attribute(raw_left_foot),
            right_foot: scale_attribute(raw_right_foot),
            raw_left_foot,
            raw_right_foot,
            positions,
            transfer_value,
            transfer_value_state,
            transfer_listed,
            loan_listed,
            raw_condition,
            raw_match_sharpness,
            traits,
            trait_bits,
            best_role,
            injuries: player_injuries,
            suspensions: player_suspensions,
            on_loan,
            loan_parent_club_uid,
            loan_parent_club_name,
            loan_start,
            loan_end,
            contract,
        });
    }

    if record_index.markerless > 0 {
        warnings.push(format!(
            "{} player records were found only by the completeness scan",
            record_index.markerless
        ));
    }
    if players.is_empty() {
        return Err(Error::parse("no players decoded"));
    }

    progress(ParseProgress {
        stage: "done".to_owned(),
        current: players.len(),
        total: players.len(),
        message: "解析完成".to_owned(),
    });
    let elapsed_ms = started.elapsed().as_millis();
    let summary = SaveSummary {
        path: path.display().to_string(),
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        file_size: save.fingerprint.file_size,
        mtime_ns: save.fingerprint.mtime_ns,
        fingerprint: save.fingerprint.digest.clone(),
        save_name: save.save_name.clone(),
        game: metadata.game.clone(),
        build: metadata.build.clone(),
        game_date: metadata.game_date.clone(),
        db_version: metadata.db_version.clone(),
        section_count: save
            .directory
            .top_entries
            .iter()
            .filter(|entry| entry.is_section())
            .count(),
        decompressed_bytes: save.total_declared_bytes(),
    };
    let contracts = players
        .iter()
        .filter(|player| player.contract.is_some())
        .count();
    let clubs_with_finance = {
        let mut uids: Vec<u32> = finance_data.months.iter().map(|row| row.club_uid).collect();
        uids.sort_unstable();
        uids.dedup();
        uids.len()
    };
    let clubs_with_corporate_facilities = finance_data
        .corporate_facilities
        .values()
        .filter(|value| value.is_some())
        .count();
    let club_finance_months = finance_data.months.len();
    let stadium_count = stadiums.len();
    let report = ParseReport {
        save: summary.clone(),
        players: players.len(),
        clubs: clubs.clubs.len(),
        contracts,
        competitions: competitions.competitions.len(),
        competition_stages: competitions.stage_count,
        competitions_with_database_id: competitions.with_database_id,
        competitions_with_name: competitions.with_name,
        club_finance_months,
        clubs_with_finance,
        stadiums: stadium_count,
        clubs_with_corporate_facilities,
        markerless_players: record_index.markerless,
        players_with_person: with_person,
        players_with_club: with_club,
        players_with_contract: with_contract,
        elapsed_ms,
        warnings,
    };

    Ok(ParsedSave {
        metadata,
        summary,
        players,
        clubs: clubs.clubs,
        competitions: competitions.competitions,
        club_finances: finance_data.months,
        stadiums,
        report,
    })
}

fn derive_positions(ratings: &[u8; 15]) -> (Vec<String>, Vec<String>) {
    let mut qualifying: Vec<(u8, usize)> = ratings
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, rating)| *rating >= 15)
        .map(|(index, rating)| (rating, index))
        .collect();
    qualifying.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let natural = qualifying
        .iter()
        .filter(|(rating, _)| *rating >= 18)
        .map(|(_, index)| POSITION_NAMES[*index].to_owned())
        .collect();
    let accomplished = qualifying
        .iter()
        .filter(|(rating, _)| *rating < 18)
        .map(|(_, index)| POSITION_NAMES[*index].to_owned())
        .collect();
    (natural, accomplished)
}

pub(crate) fn closing_person_header(
    game_db: &[u8],
    person_id: u32,
    search_start: usize,
    search_end: usize,
    uid_floor: u32,
    uid_ceiling: Option<u32>,
) -> Option<(usize, u32)> {
    if person_id == MISSING_U32 {
        return None;
    }
    let needle = (person_id + 1).to_le_bytes();
    let mut found = None;
    let mut search_from = search_start;
    while let Some(relative) = find_bytes(game_db.get(search_from..search_end)?, &needle) {
        let hit = search_from + relative;
        search_from = hit + 1;
        let Some(uid) = read_u32(game_db, hit + 4) else {
            continue;
        };
        let Some(uid_copy) = read_u32(game_db, hit + 8) else {
            continue;
        };
        if uid != uid_copy || uid == 0 || uid == MISSING_U32 || uid <= uid_floor {
            continue;
        }
        if uid_ceiling.is_some_and(|ceiling| uid > ceiling) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some((hit, uid));
    }
    found
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

/// Cache key for parsed saves. Use [`parser_display_version`] for UI display.
pub fn parser_version() -> &'static str {
    PARSER_VERSION
}

/// Human-readable parser version without the cache-revision suffix.
pub fn parser_display_version() -> &'static str {
    PARSER_VERSION.split('+').next().unwrap_or(PARSER_VERSION)
}

#[allow(dead_code)]
fn club_names(clubs: &[Club]) -> HashMap<u32, String> {
    clubs
        .iter()
        .map(|club| (club.uid, club.name.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_display_version_strips_the_cache_suffix() {
        assert!(!parser_display_version().contains('+'));
        assert!(parser_version().starts_with(parser_display_version()));
    }
}
