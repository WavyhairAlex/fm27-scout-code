//! Save-local nation ids and the editor-database ids used by the bundled name table.
//!
//! A save stores no nation names. Nation fields in player and club records hold an index into
//! the save's nation table, whose closing header pairs that index with the editor database id.
//! The bundled `shared/nation-names.csv` maps the database id to a Chinese/English name.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::{read_u8, read_u16, read_u32};

const NATION_ROW_COUNT: u32 = 251;
const NATION_TABLE_HEAD: [u8; 5] = [0xfb, 0x00, 0x00, 0x00, 0x00];
const MAX_STRING_BYTES: u32 = 1024;
const MAX_ENTRIES: u32 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nation {
    pub nation_id: u16,
    pub database_id: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NationIndex {
    pub nations: Vec<Nation>,
    pub name_by_id: HashMap<u16, String>,
    pub database_id_by_id: HashMap<u16, u32>,
}

impl NationIndex {
    pub fn read(game_db: &[u8]) -> Result<Self> {
        let mut candidates: Vec<Vec<Nation>> = Vec::new();
        let mut search_from = 0usize;
        while let Some(relative) = find_bytes(&game_db[search_from..], &NATION_TABLE_HEAD) {
            let head = search_from + relative;
            search_from = head + 1;
            if let Some(nations) = walk_table(game_db, head) {
                candidates.push(nations);
            }
        }
        if candidates.is_empty() {
            return Err(Error::parse("the nation table was not found"));
        }
        candidates.sort_by_key(|left| left.len());
        let nations = candidates.pop().unwrap_or_default();
        let mut name_by_id = HashMap::new();
        let mut database_id_by_id = HashMap::new();
        for nation in &nations {
            database_id_by_id.insert(nation.nation_id, nation.database_id);
            if let Some(name) = &nation.name {
                name_by_id.insert(nation.nation_id, name.clone());
            }
        }
        Ok(Self {
            nations,
            name_by_id,
            database_id_by_id,
        })
    }

    pub fn name_for(&self, nation_id: u16) -> Option<&str> {
        self.name_by_id.get(&nation_id).map(String::as_str)
    }

    pub fn database_id_for(&self, nation_id: u16) -> Option<u32> {
        self.database_id_by_id.get(&nation_id).copied()
    }
}

fn walk_table(game_db: &[u8], head: usize) -> Option<Vec<Nation>> {
    if head + 5 > game_db.len() {
        return None;
    }
    let count = read_u32(game_db, head)?;
    if count != NATION_ROW_COUNT {
        return None;
    }
    let mut cursor = head + 5;
    let mut nations = Vec::with_capacity(count as usize);
    for index in 0..count {
        let record_start = cursor;
        if read_u8(game_db, cursor)? != 0 {
            return None;
        }
        cursor += 1;
        cursor = walk_record(game_db, cursor)?;
        let stored_index = read_u32(game_db, cursor)?;
        let database_id = read_u32(game_db, cursor + 4)?;
        let database_copy = read_u32(game_db, cursor + 8)?;
        if stored_index != index || database_id != database_copy || database_id == 0 {
            return None;
        }
        cursor += 12;
        if record_start == cursor {
            return None;
        }
        let name = nation_name(database_id);
        nations.push(Nation {
            nation_id: index as u16,
            database_id,
            name,
        });
    }
    Some(nations)
}

fn walk_record(game_db: &[u8], mut cursor: usize) -> Option<usize> {
    // 1: four bytes.
    cursor = cursor.checked_add(4)?;
    // 2: u16 count, 4 bytes per entry.
    cursor = skip_count(game_db, cursor, 2, 4)?;
    // 3: u32 count, 4 bytes per entry.
    cursor = skip_count(game_db, cursor, 4, 4)?;
    // 4: u16 count of records with a lead byte, four strings and twenty tail bytes.
    let count = read_u16(game_db, cursor)? as usize;
    if count as u32 > MAX_ENTRIES {
        return None;
    }
    cursor += 2;
    for _ in 0..count {
        cursor = cursor.checked_add(1)?;
        for _ in 0..4 {
            let length = read_u32(game_db, cursor)?;
            if length > MAX_STRING_BYTES {
                return None;
            }
            cursor = cursor.checked_add(4 + length as usize)?;
            if cursor > game_db.len() {
                return None;
            }
        }
        cursor = cursor.checked_add(20)?;
    }
    // 5: 48 bytes.
    cursor = cursor.checked_add(48)?;
    // 6-8: counted arrays.
    cursor = skip_count(game_db, cursor, 4, 25)?;
    cursor = skip_count(game_db, cursor, 4, 12)?;
    cursor = skip_count(game_db, cursor, 4, 14)?;
    // 9: 24 bytes.
    cursor = cursor.checked_add(24)?;
    // 10: u8 count, 5 bytes per entry.
    cursor = skip_count(game_db, cursor, 1, 5)?;
    // 11-13: counted arrays.
    cursor = skip_count(game_db, cursor, 4, 28)?;
    cursor = skip_count(game_db, cursor, 4, 28)?;
    cursor = skip_count(game_db, cursor, 4, 12)?;
    // 14: 54 bytes.
    cursor = cursor.checked_add(54)?;
    (cursor <= game_db.len()).then_some(cursor)
}

fn skip_count(
    game_db: &[u8],
    cursor: usize,
    count_bytes: usize,
    entry_bytes: usize,
) -> Option<usize> {
    let count = match count_bytes {
        1 => read_u8(game_db, cursor)? as usize,
        2 => read_u16(game_db, cursor)? as usize,
        4 => read_u32(game_db, cursor)? as usize,
        _ => return None,
    };
    if count as u32 > MAX_ENTRIES {
        return None;
    }
    let end = cursor.checked_add(count_bytes + count.checked_mul(entry_bytes)?)?;
    (end <= game_db.len()).then_some(end)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn names() -> &'static HashMap<u32, String> {
    static NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut map = HashMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(include_str!("../../../shared/nation-names.csv").as_bytes());
        for row in reader.records().flatten() {
            let Some(id) = row.get(0).and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };
            let zh = row.get(1).unwrap_or("").trim();
            let en = row.get(2).unwrap_or("").trim();
            let name = if !zh.is_empty() {
                zh
            } else if !en.is_empty() {
                en
            } else {
                continue;
            };
            map.insert(id, name.to_owned());
        }
        map
    })
}

pub fn nation_name(database_id: u32) -> Option<String> {
    names().get(&database_id).cloned()
}

pub fn name_count() -> usize {
    names().len()
}

#[cfg(test)]
mod tests {
    use super::nation_name;

    #[test]
    fn non_fifa_nations_have_names() {
        for (database_id, expected) in [
            (214394, "直布罗陀"),
            (217945, "科索沃"),
            (917496, "法属圭亚那"),
            (917498, "瓜德罗普"),
            (917502, "马提尼克"),
            (917506, "荷属圣马丁"),
            (917508, "法属圣马丁"),
            (918740, "马约特"),
        ] {
            assert_eq!(nation_name(database_id).as_deref(), Some(expected));
        }
    }
}
