//! Decoding FM27 staff objects: profile attributes, role suitability and the
//! current job. The save stores staff separately from players; this module mirrors
//! the verified fmsave layout while keeping the same fail-closed rules as the
//! player parser.

use std::collections::HashMap;

use crate::bytes::find_bytes;
use crate::contracts::{ContractDecoder, StaffContractRecord};
use crate::layout::{
    ContractLayout, MISSING_U32, StaffLayout, read_i16, read_u8, read_u16, read_u32,
    scale_attribute, staff_job_name, staff_role_name,
};
use crate::model::{Staff, StaffAttributes};
use crate::nations::NationIndex;
use crate::persons::{PersonData, PersonDecoder};

#[derive(Debug, Clone, Copy)]
struct StaffHeader {
    header: usize,
    index: u32,
    uid: u32,
    ability_at: usize,
}

/// Decode every staff object that holds a club contract or a no-contract club role
/// and has a readable person block. Rows without a club record are left out; the
/// current scan cannot tell an unemployed staff object from a chance header.
pub fn read_staff(
    game_db: &[u8],
    person_decoder: &PersonDecoder<'_>,
    contract_decoder: &ContractDecoder<'_>,
    nations: &NationIndex,
) -> Vec<Staff> {
    let headers = scan_headers(game_db);
    let mut rows: Vec<Staff> = Vec::with_capacity(headers.len());
    for header in headers {
        let search_start = header
            .header
            .saturating_add(StaffLayout::KIND_OFFSET + 2)
            .min(game_db.len());
        let closing = closing_header(game_db, header.index, search_start);
        let object_end = closing.map_or_else(
            || {
                header
                    .header
                    .saturating_add(StaffLayout::CONTRACT_SEARCH_BYTES)
                    .min(game_db.len())
            },
            |(at, _)| at,
        );
        if object_end <= header.header {
            continue;
        }

        let contract = find_staff_contract(
            game_db,
            header.header,
            header.index,
            object_end,
            contract_decoder,
        )
        .or_else(|| {
            find_no_contract_record(
                game_db,
                header.header,
                header.index,
                object_end,
                contract_decoder,
            )
        });
        let Some(contract) = contract else {
            continue;
        };

        // A no-contract role's name block sits after the header, not after its tag.
        let person_tag = contract.has_terms.then_some(contract.tag_offset);
        let Some(person) = decode_person(
            game_db,
            header.header,
            person_tag,
            object_end,
            person_decoder,
        ) else {
            continue;
        };
        let Some(name) = person.name.clone() else {
            continue;
        };
        // National-team appointments have a team id but no club mapping. The staff
        // list is club-focused, so they stay out until team nationality is decoded.
        if contract.club_uid.is_none() {
            continue;
        }

        // The ability block's r4 byte shares values with job codes but is not the
        // displayed job, so only the held contract or role record supplies it.
        let job_code = contract.job_code.filter(|code| *code != 0);
        let job_name = job_code.and_then(staff_job_name).map(str::to_owned);
        let attributes = decode_attributes(game_db, header.ability_at);
        let roles = find_roles(game_db, header.header, object_end);
        let best = roles.as_ref().and_then(best_role);
        let current = read_u16(
            game_db,
            header.ability_at + StaffLayout::CURRENT_ABILITY_OFFSET,
        );
        let potential = read_i16(
            game_db,
            header.ability_at + StaffLayout::POTENTIAL_ABILITY_OFFSET,
        );
        let unique_id = closing.map(|(_, uid)| uid);
        let nation_name = nations.name_for(person.nation_id).map(str::to_owned);

        rows.push(Staff {
            uid: header.uid,
            unique_id,
            first_name: person.first_name,
            last_name: person.last_name,
            common_name: person.common_name,
            full_name: person.full_name,
            legal_name: person.legal_name,
            name: Some(name),
            birth_date: person.birth_date.map(|date| date.to_string()),
            age: person.age,
            nation_id: Some(person.nation_id),
            nation_name,
            club_uid: contract.club_uid,
            club_name: contract.club_name,
            team_id: contract.team_id,
            job_code,
            job_name,
            ca: current,
            pa: potential,
            wage: contract.wage,
            contract_start: contract.start.map(|date| date.to_string()),
            contract_end: contract.end.map(|date| date.to_string()),
            has_contract: contract.has_terms,
            attributes: attributes.map(|(display, _)| display),
            raw_attributes: attributes.map(|(_, raw)| raw),
            roles,
            best_role_index: best.as_ref().map(|(index, _, _)| *index),
            best_role_name: best.as_ref().map(|(_, name, _)| name.clone()),
            best_role_score: best.as_ref().map(|(_, _, score)| *score),
            personality: Some(person.personality),
        });
    }

    dedupe_rows(rows)
}

fn scan_headers(game_db: &[u8]) -> Vec<StaffHeader> {
    let mut headers = Vec::new();
    for (position, byte) in game_db.iter().enumerate() {
        if *byte != StaffLayout::STAFF_KIND {
            continue;
        }
        let Some(header) = position.checked_sub(StaffLayout::KIND_OFFSET) else {
            continue;
        };
        if let Some(candidate) = accept_header(game_db, header) {
            headers.push(candidate);
        }
    }
    headers
}

fn accept_header(game_db: &[u8], header: usize) -> Option<StaffHeader> {
    if header + StaffLayout::ENTRY_COUNT_OFFSET + 1 > game_db.len() {
        return None;
    }
    let index = read_u32(game_db, header)?;
    let uid = read_u32(game_db, header + StaffLayout::UID_OFFSET)?;
    let uid_copy = read_u32(game_db, header + StaffLayout::UID_COPY_OFFSET)?;
    if index == 0 || uid == 0 || uid == MISSING_U32 || uid_copy == 0 || uid_copy == MISSING_U32 {
        return None;
    }
    if uid != uid_copy && uid < StaffLayout::ALLOCATED_UID_MINIMUM {
        return None;
    }
    let entry_count = usize::from(*game_db.get(header + StaffLayout::ENTRY_COUNT_OFFSET)?);
    let ability_at = header
        .checked_add(StaffLayout::ABILITY_BASE_OFFSET)?
        .checked_add(StaffLayout::ENTRY_BYTES.checked_mul(entry_count)?)?;
    if ability_at + StaffLayout::BLOCK_40_OFFSET + StaffLayout::BLOCK_40_COUNT > game_db.len() {
        return None;
    }
    let current = read_u16(game_db, ability_at + StaffLayout::CURRENT_ABILITY_OFFSET)?;
    let potential = read_i16(game_db, ability_at + StaffLayout::POTENTIAL_ABILITY_OFFSET)?;
    if !(StaffLayout::ABILITY_RANGE.0..=StaffLayout::ABILITY_RANGE.1).contains(&current)
        || !(StaffLayout::ABILITY_RANGE.0 as i16..=StaffLayout::ABILITY_RANGE.1 as i16)
            .contains(&potential)
        || *game_db.get(ability_at + StaffLayout::SENTINEL_OFFSET)? != StaffLayout::SENTINEL_VALUE
    {
        return None;
    }
    Some(StaffHeader {
        header,
        index,
        uid,
        ability_at,
    })
}

/// The next person header whose id is `index + 1`; its uid is this person's unique id.
fn closing_header(game_db: &[u8], index: u32, search_start: usize) -> Option<(usize, u32)> {
    let needle = index.checked_add(1)?.to_le_bytes();
    let limit = search_start
        .saturating_add(StaffLayout::CONTRACT_SEARCH_BYTES)
        .min(game_db.len());
    let mut found = None;
    let mut from = search_start;
    while from + needle.len() <= limit {
        let Some(relative) = find_bytes(&game_db[from..limit], &needle) else {
            break;
        };
        let at = from + relative;
        from = at + 1;
        let Some(uid) = read_u32(game_db, at + StaffLayout::UID_OFFSET) else {
            continue;
        };
        let Some(uid_copy) = read_u32(game_db, at + StaffLayout::UID_COPY_OFFSET) else {
            continue;
        };
        if uid != uid_copy || uid == 0 || uid == MISSING_U32 {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some((at, uid));
    }
    found
}

fn find_staff_contract(
    game_db: &[u8],
    header: usize,
    index: u32,
    object_end: usize,
    decoder: &ContractDecoder<'_>,
) -> Option<StaffContractRecord> {
    let selector = index.checked_add(1)?.to_le_bytes();
    let search_start = header.checked_add(ContractLayout::SELECTOR_OFFSET)?;
    let search_end = object_end.min(game_db.len());
    let mut best: Option<StaffContractRecord> = None;
    let mut from = search_start;
    while from + selector.len() <= search_end {
        let Some(relative) = find_bytes(&game_db[from..search_end], &selector) else {
            break;
        };
        let hit = from + relative;
        from = hit + 1;
        let Some(tag) = hit.checked_sub(ContractLayout::SELECTOR_OFFSET) else {
            continue;
        };
        if tag < header || tag + ContractLayout::TAG.len() > search_end {
            continue;
        }
        if game_db.get(tag..tag + ContractLayout::TAG.len()) != Some(ContractLayout::TAG) {
            continue;
        }
        // A second tag after the selector marks a contract offer, not a job.
        let offer_at = hit.checked_add(StaffLayout::OFFER_MARKER_OFFSET);
        if offer_at.is_some_and(|at| {
            game_db
                .get(at..at + ContractLayout::TAG.len())
                .is_some_and(|bytes| bytes == ContractLayout::TAG)
        }) {
            continue;
        }
        let record = decoder.decode_staff_record(game_db, tag);
        if !record.has_terms {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|current| contract_is_newer(&record, current))
        {
            best = Some(record);
        }
    }
    best
}

fn find_no_contract_record(
    game_db: &[u8],
    header: usize,
    index: u32,
    object_end: usize,
    decoder: &ContractDecoder<'_>,
) -> Option<StaffContractRecord> {
    let selector = index.checked_add(1)?.to_le_bytes();
    let search_end = object_end.min(game_db.len());
    let mut from = header;
    while from + selector.len() <= search_end {
        let Some(relative) = find_bytes(&game_db[from..search_end], &selector) else {
            break;
        };
        let hit = from + relative;
        from = hit + 1;
        let Some(tag) = hit.checked_sub(StaffLayout::NO_CONTRACT_SELECTOR_OFFSET) else {
            continue;
        };
        if tag < header
            || tag + StaffLayout::NO_CONTRACT_JOB_OFFSET + 1 > search_end
            || tag + ContractLayout::TAG.len() > search_end
        {
            continue;
        }
        if game_db.get(tag..tag + ContractLayout::TAG.len()) != Some(ContractLayout::TAG) {
            continue;
        }
        let lead_at =
            tag + StaffLayout::NO_CONTRACT_SELECTOR_OFFSET - StaffLayout::NO_CONTRACT_LEAD.len();
        if game_db.get(lead_at..lead_at + StaffLayout::NO_CONTRACT_LEAD.len())
            != Some(StaffLayout::NO_CONTRACT_LEAD.as_slice())
        {
            continue;
        }
        let team_id = read_u32(game_db, tag + StaffLayout::NO_CONTRACT_TEAM_OFFSET);
        let job_code =
            read_u8(game_db, tag + StaffLayout::NO_CONTRACT_JOB_OFFSET).filter(|value| *value != 0);
        let (club_uid, club_name) = team_id
            .and_then(|team| club_for_team(decoder, team))
            .unwrap_or((None, None));
        return Some(StaffContractRecord {
            tag_offset: tag,
            club_uid,
            club_name,
            team_id,
            wage: None,
            start: None,
            end: None,
            has_terms: false,
            job_code,
        });
    }
    None
}

fn club_for_team(
    decoder: &ContractDecoder<'_>,
    team_id: u32,
) -> Option<(Option<u32>, Option<String>)> {
    let (club_uid, _) = decoder.club_by_team.get(&team_id)?;
    Some((Some(*club_uid), decoder.club_names.get(club_uid).cloned()))
}

fn contract_is_newer(candidate: &StaffContractRecord, current: &StaffContractRecord) -> bool {
    match (candidate.start, current.start) {
        (Some(candidate_start), Some(current_start)) => {
            candidate_start > current_start
                || (candidate_start == current_start && candidate.tag_offset > current.tag_offset)
        }
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => candidate.tag_offset > current.tag_offset,
    }
}

fn decode_person(
    game_db: &[u8],
    header: usize,
    tag: Option<usize>,
    object_end: usize,
    decoder: &PersonDecoder<'_>,
) -> Option<PersonData> {
    let (start, end) = match tag {
        Some(tag) => (
            tag.saturating_add(StaffLayout::PERSON_BLOCK_OFFSET_AFTER_TAG),
            tag.saturating_add(StaffLayout::PERSON_SEARCH_BYTES)
                .min(object_end),
        ),
        None => (
            header.saturating_add(StaffLayout::KIND_OFFSET),
            header
                .saturating_add(StaffLayout::LIST_PERSON_SEARCH_BYTES)
                .min(object_end),
        ),
    };
    if start < end {
        if let Some(person) = decoder.decode(game_db, start, end) {
            return Some(person);
        }
    }
    let fallback_start = header.saturating_add(StaffLayout::KIND_OFFSET);
    if fallback_start < object_end {
        return decoder.decode(game_db, fallback_start, object_end);
    }
    None
}

fn decode_attributes(
    game_db: &[u8],
    ability_at: usize,
) -> Option<(StaffAttributes, StaffAttributes)> {
    let profile_at = ability_at.checked_add(StaffLayout::PREFERENCES_OFFSET)?;
    let authority = read_u8(
        game_db,
        profile_at + StaffLayout::PREFERENCE_AUTHORITY_OFFSET,
    )?;
    let youngsters = read_u8(
        game_db,
        profile_at + StaffLayout::PREFERENCE_YOUNGSTERS_OFFSET,
    )?;
    let determination = read_u8(
        game_db,
        profile_at + StaffLayout::PREFERENCE_DETERMINATION_OFFSET,
    )?;
    if !(1..=20).contains(&authority)
        || !(1..=20).contains(&youngsters)
        || !(1..=100).contains(&determination)
    {
        return None;
    }
    let block_at = ability_at.checked_add(StaffLayout::BLOCK_40_OFFSET)?;
    let block = game_db.get(block_at..block_at + StaffLayout::BLOCK_40_COUNT)?;
    if block.iter().any(|value| {
        !(StaffLayout::BLOCK_40_RANGE.0..=StaffLayout::BLOCK_40_RANGE.1).contains(value)
    }) {
        return None;
    }

    let mut display = StaffAttributes::default();
    let mut raw = StaffAttributes::default();
    raw.values[0] = authority;
    display.values[0] = authority;
    raw.values[1] = youngsters;
    display.values[1] = youngsters;
    raw.values[2] = determination;
    display.values[2] = scale_attribute(determination);

    // Attribute key index -> 26-byte block index. The three preference-area
    // attributes above and the unnamed block slots are not repeated here.
    for (attribute, slot) in StaffLayout::BLOCK_40_ATTRIBUTE_SLOTS {
        let value = block[slot];
        raw.values[attribute] = value;
        display.values[attribute] = scale_attribute(value);
    }
    Some((display, raw))
}

fn find_roles(game_db: &[u8], header: usize, object_end: usize) -> Option<[u8; 15]> {
    let search_end = header
        .saturating_add(StaffLayout::PERSON_SEARCH_BYTES)
        .min(object_end)
        .min(game_db.len());
    let search_start = header.max(1);
    let mut found: Option<[u8; 15]> = None;
    let last = search_end.saturating_sub(StaffLayout::ROLE_VECTOR_BYTES + 4);
    for at in search_start..last {
        if game_db[at - 1] != StaffLayout::ROLE_VECTOR_PRECEDING {
            continue;
        }
        let sequence = &game_db[at..at + StaffLayout::ROLE_VECTOR_BYTES];
        if !sequence.iter().all(|value| {
            (StaffLayout::ROLE_SCORE_MIN..=StaffLayout::ROLE_SCORE_MAX).contains(value)
        }) {
            continue;
        }
        let trailer =
            &game_db[at + StaffLayout::ROLE_VECTOR_BYTES..at + StaffLayout::ROLE_VECTOR_BYTES + 4];
        if trailer.iter().any(|value| *value != 0) {
            continue;
        }
        if sequence.iter().copied().max().unwrap_or(0) < StaffLayout::ROLE_GREEN_THRESHOLD {
            continue;
        }
        if found.is_some() {
            // More than one candidate means the window is not trustworthy.
            return None;
        }
        found = Some(sequence.try_into().ok()?);
    }
    found
}

fn best_role(roles: &[u8; 15]) -> Option<(u8, String, u8)> {
    let mut best: Option<(usize, u8)> = None;
    for (index, score) in roles.iter().enumerate() {
        if best.is_none_or(|(_, best_score)| *score > best_score) {
            best = Some((index, *score));
        }
    }
    let (index, score) = best?;
    let index = u8::try_from(index).ok()?;
    Some((index, staff_role_name(index)?.to_owned(), score))
}

fn dedupe_rows(rows: Vec<Staff>) -> Vec<Staff> {
    let mut by_uid: HashMap<u32, Staff> = HashMap::with_capacity(rows.len());
    for row in rows {
        match by_uid.entry(row.uid) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(row);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if row_is_better(&row, entry.get()) {
                    entry.insert(row);
                }
            }
        }
    }
    let mut rows: Vec<Staff> = by_uid.into_values().collect();
    rows.sort_unstable_by_key(|row| (row.name.is_none(), row.uid));
    rows
}

fn row_is_better(candidate: &Staff, current: &Staff) -> bool {
    match (candidate.has_contract, current.has_contract) {
        (true, false) => return true,
        (false, true) => return false,
        _ => {}
    }
    match (candidate.club_uid.is_some(), current.club_uid.is_some()) {
        (true, false) => return true,
        (false, true) => return false,
        _ => {}
    }
    candidate.ca.unwrap_or(0) > current.ca.unwrap_or(0)
}
