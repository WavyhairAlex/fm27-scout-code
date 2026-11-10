//! Finding the club run by the save's first human manager.
//!
//! The `humans` section supplies the manager's selector (person id plus one). The
//! selector appears in the contract chain record that names the manager's club; the
//! strings of `save_game_summary` provide an independent link and the manager name.
//! Both routes are the verified fmsave FM27 rules. A disagreement leaves the contract
//! record in charge and records a warning instead of guessing.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::bytes::find_bytes;
use crate::clubs::ClubIndex;
use crate::container::SaveFile;
use crate::contracts::ContractDecoder;
use crate::layout::{
    ContractLayout, HumansLayout, MISSING_U32, SummaryStringsLayout, read_u16, read_u32,
};
use crate::model::ManagedClub;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryString {
    length_offset: usize,
    end_offset: usize,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryLink {
    club_uid: u32,
    manager_name: Option<String>,
}

/// Read the club run by the save's first human manager.
///
/// Returns `None` when the save lists no human manager, when neither route resolves a
/// club (for example while the manager is between jobs), or when the summary links
/// more than one club without a contract record to settle the choice. The caller keeps
/// parsing: a missing managed club only narrows the analysis centre.
pub fn read_managed_club(
    save: &SaveFile,
    game_db: &[u8],
    clubs: &ClubIndex,
    clock: NaiveDate,
    warnings: &mut Vec<String>,
) -> Option<ManagedClub> {
    let humans = match save.read_section("humans") {
        Ok(bytes) => bytes,
        Err(error) => {
            warnings.push(format!("managed club unavailable: {error}"));
            return None;
        }
    };
    let human_count = read_u16(&humans, HumansLayout::HUMAN_COUNT_OFFSET)?;
    if human_count == 0 {
        return None;
    }
    let selector = read_u32(&humans, HumansLayout::FIRST_SELECTOR_OFFSET)?;
    if selector == 0 || selector == MISSING_U32 {
        return None;
    }

    let decoder = ContractDecoder {
        club_names: &clubs.name_by_uid,
        club_by_team: &clubs.fielding_team_to_club,
        clock,
    };
    let chain_club_uid = chain_record_club(game_db, selector, &decoder, clock);

    let short_names: HashMap<u32, String> = clubs
        .clubs
        .iter()
        .filter(|club| !club.short_name.is_empty())
        .map(|club| (club.uid, club.short_name.clone()))
        .collect();
    let links = match save.read_section("save_game_summary") {
        Ok(summary) => summary_links(&summary, &short_names),
        Err(error) => {
            warnings.push(format!("managed club summary links unavailable: {error}"));
            Vec::new()
        }
    };

    if let Some(club_uid) = chain_club_uid {
        let conflicts: Vec<u32> = links
            .iter()
            .map(|link| link.club_uid)
            .filter(|uid| *uid != club_uid)
            .collect();
        if !conflicts.is_empty() {
            warnings.push(format!(
                "managed club {club_uid} from the contract record differs from save summary club {}; using the contract record",
                conflicts
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let manager_name = links
            .iter()
            .find(|link| link.club_uid == club_uid)
            .and_then(|link| link.manager_name.clone());
        return Some(managed_club(club_uid, clubs, manager_name));
    }

    let mut linked_uids: Vec<u32> = links.iter().map(|link| link.club_uid).collect();
    linked_uids.sort_unstable();
    linked_uids.dedup();
    match linked_uids.as_slice() {
        [club_uid] => {
            let manager_name = links
                .iter()
                .find(|link| link.club_uid == *club_uid)
                .and_then(|link| link.manager_name.clone());
            Some(managed_club(*club_uid, clubs, manager_name))
        }
        [] => None,
        _ => {
            warnings.push(format!(
                "managed club unknown: no contract links the first human manager to a club, and the save summary links {} clubs",
                linked_uids.len()
            ));
            None
        }
    }
}

fn managed_club(club_uid: u32, clubs: &ClubIndex, manager_name: Option<String>) -> ManagedClub {
    let club_name = clubs
        .name_by_uid
        .get(&club_uid)
        .cloned()
        .unwrap_or_else(|| format!("Club {club_uid}"));
    ManagedClub {
        club_uid,
        club_name,
        manager_name,
    }
}

/// Route 1: the club of the best chain record carrying the manager's selector.
///
/// A hit counts only when the contract tag sits before the selector. Records whose
/// team resolves to no club and records that ended before the in-game date are
/// skipped. A record with a parsed tail beats one without, then a later end wins.
fn chain_record_club(
    game_db: &[u8],
    selector: u32,
    decoder: &ContractDecoder<'_>,
    clock: NaiveDate,
) -> Option<u32> {
    let selector_bytes = selector.to_le_bytes();
    let mut best: Option<(bool, NaiveDate, u32)> = None;
    let mut search_from = 0usize;
    while let Some(relative) =
        find_bytes(game_db.get(search_from..).unwrap_or(&[]), &selector_bytes)
    {
        let hit = search_from + relative;
        search_from = hit + 1;
        let Some(tag_offset) = hit.checked_sub(ContractLayout::SELECTOR_OFFSET) else {
            continue;
        };
        let tag_end = tag_offset + ContractLayout::TAG.len();
        if game_db.get(tag_offset..tag_end) != Some(ContractLayout::TAG) {
            continue;
        }
        let record = decoder.decode_chain_record_summary(game_db, tag_offset);
        let Some(club_uid) = record.club_uid else {
            continue;
        };
        if record.end.is_some_and(|end| end < clock) {
            continue;
        }
        let rank = (record.has_terms, record.end.unwrap_or(NaiveDate::MIN));
        if best.is_none_or(|current| rank > (current.0, current.1)) {
            best = Some((rank.0, rank.1, club_uid));
        }
    }
    best.map(|(_, _, club_uid)| club_uid)
}

/// Route 2: every summary string that is a club short name followed by that club's uid.
/// The string that ends exactly where the club name starts is the manager name.
fn summary_links(summary: &[u8], short_names: &HashMap<u32, String>) -> Vec<SummaryLink> {
    let mut links = Vec::new();
    let mut previous: Option<SummaryString> = None;
    for found in scan_summary_strings(summary) {
        if let Some(club_uid) = read_u32(summary, found.end_offset) {
            if short_names
                .get(&club_uid)
                .is_some_and(|short_name| *short_name == found.text)
            {
                let manager_name = previous
                    .as_ref()
                    .filter(|value| value.end_offset == found.length_offset)
                    .map(|value| value.text.clone());
                links.push(SummaryLink {
                    club_uid,
                    manager_name,
                });
            }
        }
        previous = Some(found);
    }
    links
}

fn scan_summary_strings(summary: &[u8]) -> Vec<SummaryString> {
    let mut strings = Vec::new();
    let mut offset = SummaryStringsLayout::STRINGS_START_OFFSET;
    while offset + 4 <= summary.len() {
        let Some(length) = read_u32(summary, offset).map(|value| value as usize) else {
            break;
        };
        let start = offset + 4;
        let Some(end) = start.checked_add(length) else {
            break;
        };
        if (SummaryStringsLayout::MIN_LENGTH..=SummaryStringsLayout::MAX_LENGTH).contains(&length)
            && end <= summary.len()
            && let Ok(text) = std::str::from_utf8(&summary[start..end])
            && text
                .chars()
                .all(|character| character >= SummaryStringsLayout::LOWEST_CODE_POINT)
        {
            strings.push(SummaryString {
                length_offset: offset,
                end_offset: end,
                text: text.to_owned(),
            });
            offset = end;
            continue;
        }
        offset += 1;
    }
    strings
}

#[cfg(test)]
mod tests {
    use super::{SummaryLink, summary_links};
    use std::collections::HashMap;

    fn push_string(bytes: &mut Vec<u8>, text: &str) {
        bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }

    fn milan_names() -> HashMap<u32, String> {
        HashMap::from([(1099u32, "Milan".to_owned())])
    }

    #[test]
    fn summary_link_takes_the_adjacent_manager_name() {
        let mut summary = vec![0u8; 8];
        push_string(&mut summary, "Alice");
        push_string(&mut summary, "Milan");
        summary.extend_from_slice(&1099u32.to_le_bytes());

        assert_eq!(
            summary_links(&summary, &milan_names()),
            vec![SummaryLink {
                club_uid: 1099,
                manager_name: Some("Alice".to_owned()),
            }]
        );
    }

    #[test]
    fn summary_link_ignores_a_club_name_with_another_uid() {
        let mut summary = vec![0u8; 8];
        push_string(&mut summary, "Milan");
        summary.extend_from_slice(&1098u32.to_le_bytes());

        assert!(summary_links(&summary, &milan_names()).is_empty());
    }

    #[test]
    fn summary_link_does_not_invent_a_manager_name_across_a_gap() {
        let mut summary = vec![0u8; 8];
        push_string(&mut summary, "Alice");
        summary.push(0);
        push_string(&mut summary, "Milan");
        summary.extend_from_slice(&1099u32.to_le_bytes());

        assert_eq!(
            summary_links(&summary, &milan_names()),
            vec![SummaryLink {
                club_uid: 1099,
                manager_name: None,
            }]
        );
    }
}
