//! End-to-end parsing of one save into normalized player and club rows.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::NaiveDate;
use serde::Serialize;

use crate::bytes::find_bytes;
use crate::clubs::ClubIndex;
use crate::competitions::CompetitionIndex;
use crate::container::SaveFile;
use crate::contracts::ContractDecoder;
use crate::error::{Error, Result};
use crate::finances::read_club_finances;
use crate::layout::{
    ATTRIBUTE_KEYS, MISSING_U32, PARSER_VERSION, POSITION_NAMES, PlayerLayout, decode_date,
    read_i16, read_u8, read_u16, read_u32, scale_attribute,
};
use crate::metadata::{SaveMetadata, read_metadata};
use crate::model::{
    Attributes, Club, Competition, FinanceMonth, ManagedClub, ParseReport, Player, Positions,
    Reputation, SaveSummary, Stadium, Staff, TransferValueState,
};
use crate::names::NamePools;
use crate::nations::NationIndex;
use crate::persons::{ClubLookup, PersonDecoder};
use crate::player_scan::PlayerRecords;
use crate::season_stats::{SeasonPlayer, SeasonStatRow};
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
    pub managed_club: Option<ManagedClub>,
    pub players: Vec<Player>,
    pub staff: Vec<Staff>,
    pub clubs: Vec<Club>,
    pub season_stats: Vec<SeasonStatRow>,
    pub competitions: Vec<Competition>,
    pub team_competitions: Vec<crate::fixtures::TeamCompetition>,
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

    let club_leagues =
        crate::leagues::resolve_club_leagues(&game_db, &clubs, &competitions, &nations);
    for club in &mut clubs.clubs {
        let Some(competition_id) = club_leagues.get(&club.uid).copied() else {
            continue;
        };
        club.league_competition_id = Some(competition_id);
        club.league_name = competitions.name_for(competition_id).map(str::to_owned);
    }

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
    let managed_club = if crate::layout::is_editable_build(&metadata.build) {
        crate::managed::read_managed_club(&save, &game_db, &clubs, reference_date, &mut warnings)
    } else {
        warnings.push(format!(
            "managed club unavailable: build {} is not verified",
            metadata.build
        ));
        None
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
            club_league_competition_id,
            club_league_name,
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
                club.league_competition_id,
                club.league_name.clone(),
                Some(slot),
                club.gender,
            ),
            None => (
                None, None, None, None, None, None, None, None, None, None, None, None,
            ),
        };

        let closing = closing_person_header(
            &game_db,
            record.pindex,
            offset + PlayerLayout::DECODE_EXTENT,
            window_end,
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
        for index in 0..ATTRIBUTE_KEYS.len() {
            // The stored 54-byte slice contains two foot bytes at indices 24 and 25.
            let stored_index = if index < PlayerLayout::LEFT_FOOT_INDEX {
                index
            } else {
                index + 2
            };
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
        let second_nation_names = resolve_nation_names(&nations, &second_nation_ids);

        players.push(Player {
            uid: record.uid,
            unique_id,
            generated: crate::model::is_generated_unique_id(unique_id),
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
            second_nation_names,
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
            league_competition_id: club_league_competition_id,
            league_name: club_league_name,
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
        stage: "staff".to_owned(),
        current: 0,
        total: 0,
        message: "正在解析职员".to_owned(),
    });
    let staff = crate::staff::read_staff(&game_db, &person_decoder, &contract_decoder, &nations);
    if staff.is_empty() {
        warnings.push("staff objects unavailable".to_owned());
    }

    progress(ParseProgress {
        stage: "season_stats".to_owned(),
        current: 0,
        total: 0,
        message: "正在解析赛季统计".to_owned(),
    });
    let mut season_players = HashMap::with_capacity(record_index.records.len());
    for record in &record_index.records {
        let Some(position) = record_index.by_pindex.get(&record.pindex) else {
            continue;
        };
        let Some(player) = players.get(*position) else {
            continue;
        };
        let Some(key) = record.pindex.checked_add(1) else {
            continue;
        };
        season_players.insert(
            key,
            SeasonPlayer {
                uid: player.uid,
                name: player.name.clone(),
                team_id: player.team_id,
                goalkeeper: player
                    .natural_positions
                    .iter()
                    .any(|position| position == "GK"),
            },
        );
    }
    let season_stats = match crate::season_stats::read_season_stats(&save, &season_players, &clubs)
    {
        Ok(rows) => rows,
        Err(error) => {
            warnings.push(format!("season statistics unavailable: {error}"));
            Vec::new()
        }
    };

    progress(ParseProgress {
        stage: "fixtures".to_owned(),
        current: 0,
        total: 0,
        message: "正在解析赛程与梯队".to_owned(),
    });
    let team_competitions = match crate::fixtures::read_team_competitions(
        &save,
        &competitions.competitions,
        reference_date,
    ) {
        Ok(rows) => {
            if rows.is_empty() {
                warnings.push("fixture calendar produced no team competitions".to_owned());
            }
            rows
        }
        Err(error) => {
            warnings.push(format!("fixture calendar unavailable: {error}"));
            Vec::new()
        }
    };

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
    let staff_with_attributes = staff.iter().filter(|row| row.attributes.is_some()).count();
    let staff_with_roles = staff.iter().filter(|row| row.roles.is_some()).count();
    let report = ParseReport {
        save: summary.clone(),
        players: players.len(),
        staff: staff.len(),
        staff_with_attributes,
        staff_with_roles,
        clubs: clubs.clubs.len(),
        season_stats: season_stats.len(),
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
        managed_club,
        players,
        staff,
        clubs: clubs.clubs,
        season_stats,
        competitions: competitions.competitions,
        team_competitions,
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

/// Resolve save-local second-nation ids to the bundled names, keeping the
/// one-to-one alignment with `second_nation_ids`. `None` marks an id the name
/// table does not cover or a value outside the nation-table index range.
fn resolve_nation_names(nations: &NationIndex, ids: &[u32]) -> Vec<Option<String>> {
    ids.iter()
        .map(|id| {
            u16::try_from(*id)
                .ok()
                .and_then(|id| nations.name_for(id))
                .map(str::to_owned)
        })
        .collect()
}

pub(crate) fn closing_person_header(
    game_db: &[u8],
    person_id: u32,
    search_start: usize,
    search_end: usize,
) -> Option<(usize, u32)> {
    if person_id == MISSING_U32 {
        return None;
    }
    let needle = person_id.checked_add(1)?.to_le_bytes();
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
        if uid != uid_copy || uid == 0 || uid == MISSING_U32 {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some((hit, uid));
    }
    found
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

    #[test]
    fn second_nation_names_stay_aligned_with_ids() {
        let mut nations = NationIndex::default();
        nations.name_by_id.insert(70, "克罗地亚".to_owned());
        let names = resolve_nation_names(&nations, &[70, 999, 70, 70_000]);
        assert_eq!(
            names,
            vec![
                Some("克罗地亚".to_owned()),
                None,
                Some("克罗地亚".to_owned()),
                None
            ]
        );
    }

    #[test]
    fn closing_header_accepts_ids_below_the_record_uid() {
        // System-generated people can be stored in descending Unique ID order, and the
        // header after a player may sit beyond intervening non-player objects. The
        // closing header is identified by the pindex, not by an ascending uid.
        let mut data = vec![0u8; 64];
        let pindex = 5u32;
        data[20..24].copy_from_slice(&(pindex + 1).to_le_bytes());
        data[24..28].copy_from_slice(&1_999_999_999u32.to_le_bytes());
        data[28..32].copy_from_slice(&1_999_999_999u32.to_le_bytes());

        assert_eq!(
            closing_person_header(&data, pindex, 0, data.len()),
            Some((20, 1_999_999_999))
        );
    }

    #[test]
    fn closing_header_requires_exactly_one_doubled_uid() {
        let mut data = vec![0u8; 64];
        let pindex = 5u32;

        // An unpaired uid copy is ignored, not treated as a candidate.
        data[8..12].copy_from_slice(&(pindex + 1).to_le_bytes());
        data[12..16].copy_from_slice(&42u32.to_le_bytes());

        // One doubled header is the closing header.
        data[20..24].copy_from_slice(&(pindex + 1).to_le_bytes());
        data[24..28].copy_from_slice(&1_999_999_999u32.to_le_bytes());
        data[28..32].copy_from_slice(&1_999_999_999u32.to_le_bytes());
        assert_eq!(
            closing_person_header(&data, pindex, 0, data.len()),
            Some((20, 1_999_999_999))
        );

        // A second doubled header makes the match ambiguous.
        data[40..44].copy_from_slice(&(pindex + 1).to_le_bytes());
        data[44..48].copy_from_slice(&43u32.to_le_bytes());
        data[48..52].copy_from_slice(&43u32.to_le_bytes());
        assert_eq!(closing_person_header(&data, pindex, 0, data.len()), None);
    }

    #[test]
    fn closing_header_needs_a_valid_person_id() {
        let data = vec![0u8; 32];
        assert_eq!(
            closing_person_header(&data, MISSING_U32, 0, data.len()),
            None
        );
        assert_eq!(
            closing_person_header(&data, MISSING_U32 - 1, 0, data.len()),
            None
        );
    }
}
