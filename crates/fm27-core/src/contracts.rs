//! Decoding a player's contract chain. This mirrors the verified fmsave layout for
//! chain records, tails, clauses and loan blocks; unobserved fallback blocks are
//! deliberately left unsupported rather than guessed.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::bytes::find_bytes;
use crate::layout::{ContractLayout, MISSING_U32, decode_date, read_u8, read_u16, read_u32};
use crate::model::{Clause, Contract, ContractChainEntry};

#[derive(Debug, Clone)]
struct ChainRecord {
    club_uid: Option<u32>,
    club_name: Option<String>,
    team_id: u32,
    wage: u32,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    has_terms: bool,
    squad_status: Option<u8>,
    list_flags: Option<u8>,
    list_code: Option<u8>,
    event_count: Option<u32>,
    clauses: Vec<Clause>,
    contract_type: Option<u8>,
}

pub struct ContractDecoder<'a> {
    pub club_names: &'a HashMap<u32, String>,
    pub club_by_team: &'a HashMap<u32, (u32, u8)>,
    pub clock: NaiveDate,
}

impl ContractDecoder<'_> {
    pub fn decode(
        &self,
        game_db: &[u8],
        record_offset: usize,
        record_window_end: usize,
        is_last_record: bool,
        pindex: u32,
        player_club_uid: Option<u32>,
    ) -> Option<Contract> {
        let (offsets, records) = self.find_chain_records(
            game_db,
            record_offset,
            record_window_end,
            is_last_record,
            pindex,
        );

        let any_tail = records
            .as_ref()
            .is_some_and(|records| records.iter().any(|r| r.has_terms));
        let fallback = if records.is_none() || !any_tail {
            find_fallback_dates(game_db, record_offset, record_window_end)
        } else {
            (None, None)
        };

        if records.is_none() && fallback.0.is_none() && fallback.1.is_none() {
            return None;
        }

        let (
            contract_club_uid,
            contract_club_name,
            contract_team_id,
            wage,
            start,
            in_effect_end,
            squad_status,
            transfer_listed,
            loan_listed,
            _event_count,
            clauses,
            contract_type,
            chain,
            in_effect_has_terms,
            in_effect_index,
        ) = if let Some(records) = &records {
            let selected = select_in_effect(records, player_club_uid, self.clock);
            let chain = records
                .iter()
                .map(|record| ContractChainEntry {
                    club_uid: record.club_uid,
                    club_name: record.club_name.clone(),
                    team_id: record.team_id,
                    wage: record.wage,
                    start: record.start.map(|d| d.to_string()),
                    end: record.end.map(|d| d.to_string()),
                    has_terms: record.has_terms,
                })
                .collect();
            if let Some((index, record)) = selected {
                let transfer_listed = record.has_terms
                    && (record.list_code == Some(ContractLayout::LIST_CODE_TRANSFER)
                        || record.list_flags.is_some_and(|flags| flags & 1 != 0));
                let loan_listed =
                    record.has_terms && record.list_flags.is_some_and(|flags| flags & 2 != 0);
                (
                    record.club_uid,
                    record.club_name.clone(),
                    Some(record.team_id),
                    Some(record.wage),
                    record.start,
                    record.end,
                    if record.has_terms {
                        record.squad_status
                    } else {
                        None
                    },
                    transfer_listed,
                    loan_listed,
                    if record.has_terms {
                        record.event_count
                    } else {
                        None
                    },
                    if record.has_terms {
                        record.clauses.clone()
                    } else {
                        Vec::new()
                    },
                    if record.has_terms {
                        record.contract_type
                    } else {
                        None
                    },
                    chain,
                    record.has_terms,
                    Some(index),
                )
            } else {
                (
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    Vec::new(),
                    None,
                    chain,
                    false,
                    None,
                )
            }
        } else {
            (
                None,
                None,
                None,
                None,
                fallback.0,
                None,
                None,
                false,
                false,
                None,
                Vec::new(),
                None,
                Vec::new(),
                false,
                None,
            )
        };

        let end = if let Some(end) = in_effect_end {
            Some(end)
        } else if !any_tail && fallback.1.is_some_and(|end| end >= self.clock) {
            fallback.1
        } else {
            None
        };

        let mut on_loan = None;
        let mut loan_parent_club_uid = None;
        let mut loan_parent_club_name = None;
        let mut loan_start = None;
        let mut loan_end = None;
        if let (Some(player_club), Some(contract_club)) = (player_club_uid, contract_club_uid) {
            if contract_club != player_club {
                let loan = self.loan_dates(
                    game_db,
                    records.as_deref().unwrap_or(&[]),
                    offsets.as_deref().unwrap_or(&[]),
                    player_club,
                );
                on_loan = Some(loan.is_some());
                if let Some((start, end)) = loan {
                    loan_start = Some(start.to_string());
                    loan_end = Some(end.to_string());
                    loan_parent_club_uid = Some(contract_club);
                    loan_parent_club_name = contract_club_name.clone();
                }
            } else {
                on_loan = Some(false);
            }
        }

        // A record without terms still needs a start for display; the fallback dates are
        // the only source when no tail exists.
        let _ = in_effect_has_terms;
        let _ = in_effect_index;
        Some(Contract {
            club_uid: contract_club_uid,
            club_name: contract_club_name,
            team_id: contract_team_id,
            wage,
            start: start.map(|d| d.to_string()),
            end: end.map(|d| d.to_string()),
            squad_status,
            contract_type,
            transfer_listed,
            loan_listed,
            clauses,
            on_loan,
            loan_parent_club_uid,
            loan_parent_club_name,
            loan_start,
            loan_end,
            chain,
        })
    }

    fn find_chain_records(
        &self,
        game_db: &[u8],
        record_offset: usize,
        record_window_end: usize,
        is_last_record: bool,
        pindex: u32,
    ) -> (Option<Vec<usize>>, Option<Vec<ChainRecord>>) {
        let search_start = (record_offset as i64 + ContractLayout::CHAIN_WINDOW_START_OFFSET as i64)
            .max(0) as usize;
        let search_end = if is_last_record {
            record_window_end
        } else {
            (record_window_end as i64 + ContractLayout::CHAIN_WINDOW_END_OFFSET as i64)
                .max(search_start as i64) as usize
        };
        if search_start >= search_end {
            return (None, None);
        }
        let selector = (pindex + 1).to_le_bytes();
        let mut offsets = Vec::new();
        let mut records = Vec::new();
        let mut search_from = search_start + ContractLayout::SELECTOR_OFFSET;
        let search_limit = search_end + ContractLayout::SELECTOR_OFFSET;
        while let Some(relative) = find_bytes(
            game_db.get(search_from..search_limit).unwrap_or(&[]),
            &selector,
        ) {
            let hit = search_from + relative;
            search_from = hit + 1;
            let Some(chain_tag_offset) = hit.checked_sub(ContractLayout::SELECTOR_OFFSET) else {
                continue;
            };
            if chain_tag_offset < search_start || chain_tag_offset >= search_end {
                continue;
            }
            if game_db.get(chain_tag_offset..chain_tag_offset + ContractLayout::TAG.len())
                == Some(ContractLayout::TAG)
                || self.is_date_marked_record(game_db, chain_tag_offset)
            {
                offsets.push(chain_tag_offset);
                records.push(self.decode_chain_record(game_db, chain_tag_offset));
            }
        }
        if records.is_empty() {
            (None, None)
        } else {
            (Some(offsets), Some(records))
        }
    }

    fn decode_chain_record(&self, game_db: &[u8], chain_tag_offset: usize) -> ChainRecord {
        let team_id =
            read_u32(game_db, chain_tag_offset + ContractLayout::TEAM_ID_OFFSET).unwrap_or(0);
        let wage = read_u32(game_db, chain_tag_offset + ContractLayout::WAGE_OFFSET).unwrap_or(0);
        let (club_uid, club_name) = self
            .club_by_team
            .get(&team_id)
            .and_then(|(uid, _)| {
                self.club_names
                    .get(uid)
                    .map(|name| (Some(*uid), Some(name.clone())))
            })
            .unwrap_or((None, None));

        let start = positive_offset(chain_tag_offset, ContractLayout::START_OFFSET)
            .and_then(|offset| decode_date(game_db, offset));

        let Some(tail) = locate_tail(game_db, chain_tag_offset) else {
            return ChainRecord {
                club_uid,
                club_name,
                team_id,
                wage,
                start,
                end: None,
                has_terms: false,
                squad_status: None,
                list_flags: None,
                list_code: None,
                event_count: None,
                clauses: Vec::new(),
                contract_type: None,
            };
        };

        let end = decode_date(game_db, tail + ContractLayout::TAIL_END_OFFSET);
        let squad_status = read_u8(game_db, tail + ContractLayout::TAIL_SQUAD_STATUS_OFFSET);
        let list_flags = read_u8(game_db, tail + ContractLayout::TAIL_LIST_FLAGS_OFFSET);
        let list_code = read_u8(game_db, tail + ContractLayout::TAIL_LIST_CODE_OFFSET);
        let event_count = read_u32(game_db, tail + ContractLayout::TAIL_EVENT_COUNT_OFFSET);
        let (clauses, contract_type) = if let Some((base, count)) =
            locate_clause_base(game_db, tail).or_else(|| locate_clause_base_fallback(game_db, tail))
        {
            (
                read_clauses(game_db, base, count),
                read_contract_type(game_db, base),
            )
        } else {
            (Vec::new(), None)
        };

        ChainRecord {
            club_uid,
            club_name,
            team_id,
            wage,
            start,
            end,
            has_terms: true,
            squad_status,
            list_flags,
            list_code,
            event_count,
            clauses,
            contract_type,
        }
    }

    fn is_date_marked_record(&self, game_db: &[u8], chain_tag_offset: usize) -> bool {
        if decode_date(game_db, chain_tag_offset).is_none() {
            return false;
        }
        let Some(start_offset) = positive_offset(chain_tag_offset, ContractLayout::START_OFFSET)
        else {
            return false;
        };
        if decode_date(game_db, start_offset).is_none() {
            return false;
        }
        let team_id =
            read_u32(game_db, chain_tag_offset + ContractLayout::TEAM_ID_OFFSET).unwrap_or(0);
        self.club_by_team
            .get(&team_id)
            .is_some_and(|(uid, _)| self.club_names.contains_key(uid))
    }

    fn loan_dates(
        &self,
        game_db: &[u8],
        records: &[ChainRecord],
        offsets: &[usize],
        player_club_uid: u32,
    ) -> Option<(NaiveDate, NaiveDate)> {
        let mut current: Option<(usize, &ChainRecord)> = None;
        let mut current_key: Option<(NaiveDate, usize)> = None;
        for (record, offset) in records.iter().zip(offsets.iter().copied()) {
            if record.club_uid != Some(player_club_uid) {
                continue;
            }
            if record.start.is_some_and(|start| start > self.clock) {
                continue;
            }
            let key = (record.start.unwrap_or(self.clock), offset);
            if current_key.is_none_or(|existing| key > existing) {
                current = Some((offset, record));
                current_key = Some(key);
            }
        }
        let (offset, record) = current?;
        if record.has_terms {
            return None;
        }
        let block = locate_loan_block(game_db, offset)?;
        if block.0 != MISSING_U32 || block.1.is_none() || block.1? < self.clock {
            return None;
        }
        Some((block.2?, block.1?))
    }
}

fn locate_tail(game_db: &[u8], chain_tag_offset: usize) -> Option<usize> {
    let first = chain_tag_offset.checked_sub(ContractLayout::TAIL_BASE_OFFSET)?;
    if tail_matches(game_db, first, 0) {
        return Some(first);
    }
    for event_count in 1..=ContractLayout::TAIL_MAX_EVENT_COUNT {
        let candidate = first.checked_sub(ContractLayout::TAIL_STEP_BYTES * event_count)?;
        if tail_matches(game_db, candidate, event_count) {
            return Some(candidate);
        }
    }
    None
}

fn tail_matches(game_db: &[u8], tail: usize, expected_event_count: usize) -> bool {
    tail + ContractLayout::TAIL_STRUCT_END <= game_db.len()
        && game_db[tail] == 0
        && game_db[tail + 1] == 0
        && game_db[tail + ContractLayout::TAIL_SENTINEL_BYTE_OFFSET]
            == ContractLayout::TAIL_SENTINEL_BYTE_VALUE
        && read_u32(game_db, tail + ContractLayout::TAIL_SENTINEL_WORD_OFFSET)
            == Some(ContractLayout::TAIL_SENTINEL_WORD_VALUE)
        && read_u32(game_db, tail + ContractLayout::TAIL_EVENT_COUNT_OFFSET)
            == Some(expected_event_count as u32)
}

fn locate_clause_base(game_db: &[u8], tail: usize) -> Option<(usize, usize)> {
    for count in 0..=ContractLayout::CLAUSE_MAX_COUNT {
        let base = tail.checked_sub(ContractLayout::CLAUSE_STEP_BYTES * count)?;
        let ff_start = base as i64 + ContractLayout::CLAUSE_FF_OFFSET as i64;
        let count_at = base as i64 + ContractLayout::CLAUSE_COUNT_OFFSET as i64;
        if ff_start < 0 || count_at < 0 || count_at as usize >= game_db.len() {
            return None;
        }
        let ff_start = ff_start as usize;
        let count_at = count_at as usize;
        if game_db[count_at] as usize != count {
            continue;
        }
        let ff_end = ff_start + ContractLayout::CLAUSE_FF_COUNT;
        let zero_end = ff_end + ContractLayout::CLAUSE_ZERO_COUNT;
        if zero_end > game_db.len() {
            return None;
        }
        if game_db[ff_start..ff_end].iter().all(|byte| *byte == 0xFF)
            && game_db[ff_end..zero_end].iter().all(|byte| *byte == 0)
        {
            return Some((base, count));
        }
    }
    None
}

/// The nearest clause table that ends exactly at the tail, including tables with
/// bonus lists between their entries and the tail. Ported from fmsave's fallback walk.
fn locate_clause_base_fallback(game_db: &[u8], tail: usize) -> Option<(usize, usize)> {
    let trailer_start = tail.checked_sub(ContractLayout::CLAUSE_TRAILER_BYTES)?;
    if !game_db
        .get(trailer_start..tail)?
        .iter()
        .all(|byte| *byte == 0)
    {
        return None;
    }
    let mut award_list_starts = Vec::new();
    let absent_flag_offset = trailer_start.checked_sub(ContractLayout::CLAUSE_AWARD_FLAG_BYTES)?;
    if read_u8(game_db, absent_flag_offset)? == 0 {
        award_list_starts.push(absent_flag_offset);
    }

    let award_count_bytes = ContractLayout::CLAUSE_AWARD_COUNT_BYTES;
    let award_item_bytes = ContractLayout::CLAUSE_AWARD_ITEM_BYTES;
    for award_count in 0..=ContractLayout::CLAUSE_AWARD_MAX_COUNT {
        let Some(award_items_start) = trailer_start.checked_sub(award_item_bytes * award_count)
        else {
            break;
        };
        let Some(award_count_offset) = award_items_start.checked_sub(award_count_bytes) else {
            break;
        };
        let Some(flag_offset) =
            award_count_offset.checked_sub(ContractLayout::CLAUSE_AWARD_FLAG_BYTES)
        else {
            break;
        };
        if read_u32(game_db, award_count_offset) == Some(award_count as u32)
            && read_u8(game_db, flag_offset) == Some(1)
            && (0..award_count).all(|index| {
                prefix_at(
                    game_db,
                    award_items_start + award_item_bytes * index,
                    ContractLayout::CLAUSE_AWARD_ITEM_PREFIX,
                )
            })
        {
            award_list_starts.push(flag_offset);
        }
    }

    let entry_bytes = ContractLayout::CLAUSE_ENTRY_BYTES;
    let ff_offset = ContractLayout::CLAUSE_FF_OFFSET;
    let zero_offset = ContractLayout::CLAUSE_ZERO_OFFSET;
    let competition_item_bytes = ContractLayout::CLAUSE_COMPETITION_ITEM_BYTES;
    let mut nearest: Option<(usize, usize)> = None;

    for award_list_start in award_list_starts {
        for competition_count in 0..=ContractLayout::CLAUSE_COMPETITION_MAX_COUNT {
            let Some(competition_items_start) =
                award_list_start.checked_sub(competition_item_bytes * competition_count)
            else {
                break;
            };
            let Some(entries_end) =
                competition_items_start.checked_sub(ContractLayout::CLAUSE_COMPETITION_COUNT_BYTES)
            else {
                break;
            };
            if read_u8(game_db, entries_end) != Some(competition_count as u8) {
                continue;
            }
            if !(0..competition_count).all(|index| {
                prefix_at(
                    game_db,
                    competition_items_start + competition_item_bytes * index,
                    ContractLayout::CLAUSE_COMPETITION_ITEM_PREFIX,
                )
            }) {
                continue;
            }
            for clause_count in 0..=ContractLayout::CLAUSE_MAX_COUNT {
                let Some(base) = entries_end
                    .checked_add(4)
                    .and_then(|value| value.checked_sub(entry_bytes * clause_count))
                else {
                    break;
                };
                if nearest.is_some_and(|(nearest_base, _)| base <= nearest_base) {
                    break;
                }
                let Some(marker_offset) = base.checked_add_signed(ff_offset as isize) else {
                    break;
                };
                let Some(zero_marker_offset) = base.checked_add_signed(zero_offset as isize) else {
                    break;
                };
                if read_u8(game_db, zero_marker_offset) != Some(0)
                    || read_u8(game_db, zero_marker_offset + 1) != Some(0)
                    || read_u8(game_db, zero_marker_offset + 2) != Some(0)
                    || read_u8(game_db, zero_marker_offset + 3) != Some(clause_count as u8)
                {
                    continue;
                }
                let marker = game_db.get(marker_offset..marker_offset + 8)?;
                let all_ff = marker.iter().all(|byte| *byte == 0xFF);
                let first_id = &marker[..ContractLayout::CLAUSE_TEAM_MARKER_ID_BYTES];
                let team_marker = !first_id.iter().all(|byte| *byte == 0xFF)
                    && !first_id.iter().all(|byte| *byte == 0)
                    && marker[ContractLayout::CLAUSE_TEAM_MARKER_ID_BYTES..]
                        .iter()
                        .all(|byte| *byte == 0);
                if all_ff || team_marker {
                    nearest = Some((base, clause_count));
                    break;
                }
            }
        }
    }
    nearest
}

fn prefix_at(game_db: &[u8], offset: usize, prefix: &[u8]) -> bool {
    game_db.get(offset..offset + prefix.len()) == Some(prefix)
}

fn read_clauses(game_db: &[u8], base: usize, count: usize) -> Vec<Clause> {
    let Ok(entries_start): Result<usize, _> =
        (base as i64 + ContractLayout::CLAUSE_ENTRIES_OFFSET as i64).try_into()
    else {
        return Vec::new();
    };
    let mut clauses = Vec::with_capacity(count);
    for index in 0..count {
        let at = entries_start + index * ContractLayout::CLAUSE_ENTRY_BYTES;
        let Some(value) = read_u32(game_db, at) else {
            break;
        };
        let parameter = read_u16(game_db, at + 4);
        let kind = read_u16(game_db, at + 6).unwrap_or(0);
        clauses.push(Clause {
            kind,
            parameter: parameter.filter(|value| *value != 0xFFFF),
            value: (value != MISSING_U32).then_some(value),
        });
    }
    clauses
}

fn read_contract_type(game_db: &[u8], clause_base: usize) -> Option<u8> {
    let gate_at = clause_base as i64 + ContractLayout::HEAD_GATE_OFFSET as i64;
    let type_at = clause_base as i64 + ContractLayout::HEAD_TYPE_OFFSET as i64;
    if gate_at < 0 || type_at < 0 {
        return None;
    }
    if read_u16(game_db, gate_at as usize)? != ContractLayout::HEAD_GATE_VALUE {
        return None;
    }
    read_u8(game_db, type_at as usize)
}

fn select_in_effect(
    records: &[ChainRecord],
    player_club_uid: Option<u32>,
    clock: NaiveDate,
) -> Option<(usize, &ChainRecord)> {
    let mut latest: Option<(usize, &ChainRecord, NaiveDate, NaiveDate)> = None;
    let mut at_own_club: Option<(usize, &ChainRecord, NaiveDate, NaiveDate)> = None;
    let mut earliest_running: Option<(usize, &ChainRecord, NaiveDate, NaiveDate)> = None;
    for (index, record) in records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.has_terms)
    {
        let Some(start) = record.start else { continue };
        if start > clock {
            continue;
        }
        let end = record.end.unwrap_or(start);
        let key = (start, end.max(start));
        if latest
            .as_ref()
            .is_none_or(|(_, _, latest_start, latest_end)| key > (*latest_start, *latest_end))
        {
            latest = Some((index, record, key.0, key.1));
        }
        if record.end.is_some_and(|end| end < clock) {
            continue;
        }
        if player_club_uid.is_some_and(|club| record.club_uid == Some(club)) {
            if at_own_club
                .as_ref()
                .is_none_or(|(_, _, start0, end0)| key > (*start0, *end0))
            {
                at_own_club = Some((index, record, key.0, key.1));
            }
        } else if earliest_running
            .as_ref()
            .is_none_or(|(_, _, start0, end0)| key < (*start0, *end0))
        {
            earliest_running = Some((index, record, key.0, key.1));
        }
    }
    at_own_club
        .or(latest)
        .map(|(index, record, _, _)| (index, record))
        .or(earliest_running.map(|(index, record, _, _)| (index, record)))
}

fn locate_loan_block(
    game_db: &[u8],
    chain_tag_offset: usize,
) -> Option<(u32, Option<NaiveDate>, Option<NaiveDate>)> {
    let first = chain_tag_offset.checked_sub(ContractLayout::TAIL_BASE_OFFSET)?;
    for count in 0..=ContractLayout::LOAN_BLOCK_MAX_EVENT_COUNT {
        let block = first.checked_sub(ContractLayout::TAIL_STEP_BYTES * count)?;
        let event_count = read_u32(game_db, block + ContractLayout::TAIL_EVENT_COUNT_OFFSET)?;
        if event_count as usize != count {
            continue;
        }
        let raw_end = read_u32(game_db, block + ContractLayout::TAIL_END_OFFSET)?;
        let end = decode_date(game_db, block + ContractLayout::TAIL_END_OFFSET);
        if end.is_none() && raw_end != MISSING_U32 {
            continue;
        }
        let marker = read_u32(game_db, block + ContractLayout::TAIL_E24_OFFSET)?;
        let start = decode_date(game_db, block + ContractLayout::TAIL_PRINTED_START_OFFSET);
        return Some((marker, end, start));
    }
    None
}

fn find_fallback_dates(
    game_db: &[u8],
    record_offset: usize,
    record_window_end: usize,
) -> (Option<NaiveDate>, Option<NaiveDate>) {
    let search_start = record_offset + ContractLayout::FALLBACK_START_FROM_RECORD;
    let limit = record_window_end
        .saturating_sub(ContractLayout::FALLBACK_END_MARGIN)
        .max(search_start);
    let find_end = limit.min(game_db.len());
    let mut best: Option<(NaiveDate, NaiveDate)> = None;
    let mut search_from = search_start;
    while let Some(relative) = find_bytes(&game_db[search_from..find_end], &[0xFF; 4]) {
        let hit = search_from + relative;
        search_from = hit + 1;
        let dates_offset = hit + 8;
        let nonzero_start = dates_offset + ContractLayout::FALLBACK_NONZERO_OFFSET;
        if dates_offset + ContractLayout::FALLBACK_GATE_LENGTH > limit
            || nonzero_start + ContractLayout::FALLBACK_NONZERO_LENGTH > game_db.len()
        {
            continue;
        }
        if game_db[nonzero_start..nonzero_start + ContractLayout::FALLBACK_NONZERO_LENGTH]
            .iter()
            .all(|byte| *byte == 0)
        {
            continue;
        }
        let Some(end) = decode_date(game_db, dates_offset) else {
            continue;
        };
        let Some(start) = decode_date(game_db, dates_offset + 4) else {
            continue;
        };
        if end <= start {
            continue;
        }
        if best.is_none_or(|(_, best_end)| end > best_end) {
            best = Some((start, end));
        }
    }
    best.map_or((None, None), |(start, end)| (Some(start), Some(end)))
}

fn positive_offset(base: usize, offset: i32) -> Option<usize> {
    if offset >= 0 {
        base.checked_add(offset as usize)
    } else {
        base.checked_sub(offset.unsigned_abs() as usize)
    }
}
