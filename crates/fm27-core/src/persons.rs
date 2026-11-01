//! Decoding a player's person block: names, birth date, nations, relations, traits
//! and personality. The block is found by its eight-byte personality run.

use chrono::{Datelike, NaiveDate};

use crate::layout::{
    MISSING_U32, PersonLayout, decode_date, read_u8, read_u16, read_u32, read_u64, trait_name,
};
use crate::model::{Personality, Trait};
use crate::names::NamePools;

#[derive(Debug, Clone)]
pub struct PersonData {
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub common_name: Option<String>,
    pub full_name: Option<String>,
    pub legal_name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub age: Option<i32>,
    pub nation_id: u16,
    pub second_nation_ids: Vec<u32>,
    pub home_grown_nation_ids: Vec<u32>,
    pub home_grown_club_uids: Vec<Option<u32>>,
    pub home_grown_club_names: Vec<Option<String>>,
    pub personality: Personality,
    pub trait_bits: u64,
    pub traits: Vec<Trait>,
}

/// Where a decoded person block lives in `game_db`.
#[derive(Debug, Clone, Copy)]
pub struct PersonLocation {
    pub birth_offset: usize,
    pub block_start: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ClubLookup<'a> {
    pub name_by_uid: &'a std::collections::HashMap<u32, String>,
    pub uid_by_index: &'a std::collections::HashMap<u32, u32>,
}

impl ClubLookup<'_> {
    pub fn empty() -> ClubLookup<'static> {
        use std::sync::OnceLock;
        static NAMES: OnceLock<std::collections::HashMap<u32, String>> = OnceLock::new();
        static IDS: OnceLock<std::collections::HashMap<u32, u32>> = OnceLock::new();
        ClubLookup {
            name_by_uid: NAMES.get_or_init(std::collections::HashMap::new),
            uid_by_index: IDS.get_or_init(std::collections::HashMap::new),
        }
    }
}

pub struct PersonDecoder<'a> {
    pub names: &'a NamePools,
    pub clubs: ClubLookup<'a>,
    pub clock: NaiveDate,
}

impl PersonDecoder<'_> {
    pub fn decode(
        &self,
        game_db: &[u8],
        window_start: usize,
        window_end: usize,
    ) -> Option<PersonData> {
        self.locate(game_db, window_start, window_end)
            .map(|(_, person)| person)
    }

    /// Decode a person block and also return where its fields live.
    pub fn locate(
        &self,
        game_db: &[u8],
        window_start: usize,
        window_end: usize,
    ) -> Option<(PersonLocation, PersonData)> {
        let search_start =
            window_start.checked_add(PersonLayout::PERSONALITY_OFFSET_FROM_BIRTH - 1)?;
        if search_start >= window_end {
            return None;
        }
        let mut selected: Option<(PersonLocation, PersonData)> = None;
        let end = window_end.min(game_db.len());
        let mut zero_position = search_start;
        while zero_position + 1 + PersonLayout::PERSONALITY_COUNT <= end {
            if game_db[zero_position] != 0 {
                zero_position += 1;
                continue;
            }
            let personality =
                &game_db[zero_position + 1..zero_position + 1 + PersonLayout::PERSONALITY_COUNT];
            if !personality.iter().all(|value| (1..=20).contains(value)) {
                zero_position += 1;
                continue;
            }
            let birth_offset = zero_position - (PersonLayout::PERSONALITY_OFFSET_FROM_BIRTH - 1);
            if let Some((location, person)) = self.try_candidate(game_db, birth_offset, end) {
                if selected.is_some() {
                    // FM27 requires a unique candidate in the window.
                    return None;
                }
                selected = Some((location, person));
            }
            zero_position += 1;
        }
        selected
    }

    fn try_candidate(
        &self,
        game_db: &[u8],
        birth_offset: usize,
        window_end: usize,
    ) -> Option<(PersonLocation, PersonData)> {
        let birth_word = read_u32(game_db, birth_offset)?;
        let packed = (birth_word & 0xFFFF) as u16;
        let year = (birth_word >> 16) as i32;
        if packed >> 9 != 0 {
            return None;
        }
        let day_of_year = packed & 0x1FF;
        if !(1..=366).contains(&day_of_year) {
            return None;
        }
        if !(crate::layout::EARLIEST_YEAR..=self.clock.year()).contains(&year) {
            return None;
        }

        let zero_region = game_db.get(
            birth_offset + PersonLayout::DATE_ZERO_BYTES_OFFSET_FROM_BIRTH
                ..birth_offset
                    + PersonLayout::DATE_ZERO_BYTES_OFFSET_FROM_BIRTH
                    + PersonLayout::DATE_ZERO_BYTES_COUNT,
        )?;
        if zero_region.iter().any(|byte| *byte != 0) {
            return None;
        }

        let legal_name_length = self.legal_name_length(game_db, birth_offset)?;
        let block_start = birth_offset
            .checked_sub(PersonLayout::LEGAL_NAME_OFFSET_FROM_BLOCK_START + legal_name_length)?;
        let name_block_offset = block_start.checked_sub(8)?;
        let trait_bits = read_u64(game_db, name_block_offset)?;
        let first_name_id = read_u32(game_db, block_start)?;
        let first_name_zero = read_u8(game_db, block_start + 4)?;
        let surname_id = read_u32(game_db, block_start + 5)?;
        let surname_zero = read_u8(game_db, block_start + 9)?;
        let common_name_id = read_u32(game_db, block_start + 10)?;
        let common_name_zero = read_u8(game_db, block_start + 14)?;
        if first_name_zero != 0 || surname_zero != 0 || common_name_zero != 0 {
            return None;
        }
        if (first_name_id != MISSING_U32 && first_name_id >= PersonLayout::NAME_ID_LIMIT)
            || (surname_id != MISSING_U32 && surname_id >= PersonLayout::NAME_ID_LIMIT)
            || (common_name_id != MISSING_U32 && common_name_id >= PersonLayout::NAME_ID_LIMIT)
        {
            return None;
        }

        let nation_id = read_u16(
            game_db,
            birth_offset + PersonLayout::NATION_ID_OFFSET_FROM_BIRTH,
        )?;
        let personality_bytes = game_db.get(
            birth_offset + PersonLayout::PERSONALITY_OFFSET_FROM_BIRTH
                ..birth_offset
                    + PersonLayout::PERSONALITY_OFFSET_FROM_BIRTH
                    + PersonLayout::PERSONALITY_COUNT,
        )?;
        let personality = Personality {
            adaptability: personality_bytes[0],
            ambition: personality_bytes[1],
            loyalty: personality_bytes[2],
            pressure: personality_bytes[3],
            professionalism: personality_bytes[4],
            sportsmanship: personality_bytes[5],
            temperament: personality_bytes[6],
            controversy: personality_bytes[7],
        };

        let present = read_u8(
            game_db,
            birth_offset + PersonLayout::RELATION_PRESENT_OFFSET_FROM_BIRTH,
        )?;
        let count = read_u8(
            game_db,
            birth_offset + PersonLayout::RELATION_COUNT_OFFSET_FROM_BIRTH,
        )?;
        let mut second_nation_ids = Vec::new();
        let mut home_grown_nation_ids = Vec::new();
        let mut home_grown_club_uids = Vec::new();
        let mut home_grown_club_names = Vec::new();
        let mut seen_home_grown_club_refs = Vec::new();
        if present != 0 && count != 0 {
            let entries_start = birth_offset + PersonLayout::RELATION_ENTRIES_OFFSET_FROM_BIRTH;
            let entries_end = entries_start + PersonLayout::RELATION_ENTRY_BYTES * count as usize;
            if entries_end > window_end {
                return None;
            }
            for entry in 0..count as usize {
                let at = entries_start + entry * PersonLayout::RELATION_ENTRY_BYTES;
                let referenced = read_u32(game_db, at)?;
                let kind_role = read_u16(game_db, at + 10)?;
                let qualifier = read_u8(game_db, at + 12)?;
                let key = kind_role;
                let second_key = ((PersonLayout::SECOND_NATION_PAIR.1 as u16) << 8)
                    | PersonLayout::SECOND_NATION_PAIR.0 as u16;
                let home_nation_key = ((PersonLayout::HOME_GROWN_NATION_PAIR.1 as u16) << 8)
                    | PersonLayout::HOME_GROWN_NATION_PAIR.0 as u16;
                let home_club_key = ((PersonLayout::HOME_GROWN_CLUB_PAIR.1 as u16) << 8)
                    | PersonLayout::HOME_GROWN_CLUB_PAIR.0 as u16;
                if key == second_key {
                    if PersonLayout::SECOND_NATION_QUALIFIERS.contains(&qualifier)
                        && !second_nation_ids.contains(&referenced)
                    {
                        second_nation_ids.push(referenced);
                    }
                } else if key == home_nation_key {
                    if !home_grown_nation_ids.contains(&referenced) {
                        home_grown_nation_ids.push(referenced);
                    }
                } else if key == home_club_key && !seen_home_grown_club_refs.contains(&referenced) {
                    seen_home_grown_club_refs.push(referenced);
                    let club_uid = self.clubs.uid_by_index.get(&referenced).copied();
                    let club_name =
                        club_uid.and_then(|uid| self.clubs.name_by_uid.get(&uid).cloned());
                    home_grown_club_uids.push(club_uid);
                    home_grown_club_names.push(club_name);
                }
            }
        }

        let first_name = self.names.first_names.name_at(game_db, first_name_id);
        let last_name = self.names.surnames.name_at(game_db, surname_id);
        let common_name = if common_name_id == MISSING_U32 {
            None
        } else {
            self.names.resolve_common(game_db, common_name_id)
        };
        let legal_name = self.decode_legal_name(game_db, block_start, legal_name_length);
        let full_name = match (&first_name, &last_name) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            _ => None,
        };
        let name = if let Some(common) = &common_name {
            Some(common.clone())
        } else if let Some(full) = &full_name {
            Some(full.clone())
        } else if first_name.is_some() {
            first_name.clone()
        } else if last_name.is_some() {
            last_name.clone()
        } else {
            legal_name.clone()
        };

        let birth_date = decode_date(game_db, birth_offset);
        let age = birth_date.map(|date| crate::model::Player::age_at(date, self.clock));
        let mut traits = Vec::new();
        let mut remaining = trait_bits;
        while remaining != 0 {
            let bit = remaining.trailing_zeros() as u8;
            traits.push(Trait {
                bit,
                name: trait_name(bit).map(str::to_owned),
            });
            remaining &= remaining - 1;
        }

        Some((
            PersonLocation {
                birth_offset,
                block_start,
            },
            PersonData {
                name,
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
            },
        ))
    }

    fn legal_name_length(&self, game_db: &[u8], birth_offset: usize) -> Option<usize> {
        let zero_length_word_at = birth_offset.checked_sub(4)?;
        if read_u32(game_db, zero_length_word_at)? == 0 {
            return Some(0);
        }
        for length in PersonLayout::LEGAL_NAME_LENGTH_MIN..=PersonLayout::LEGAL_NAME_LENGTH_MAX {
            let length_word_at = birth_offset.checked_sub(4 + length)?;
            if read_u32(game_db, length_word_at)? as usize == length {
                return Some(length);
            }
        }
        None
    }

    fn decode_legal_name(
        &self,
        game_db: &[u8],
        block_start: usize,
        legal_name_length: usize,
    ) -> Option<String> {
        if legal_name_length == 0 {
            return None;
        }
        let start = block_start + PersonLayout::LEGAL_NAME_OFFSET_FROM_BLOCK_START;
        let bytes = game_db.get(start..start + legal_name_length)?;
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    }
}
