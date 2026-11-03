//! Optional competition name table.
//!
//! A save stores competition database ids but no competition names; the game renders those
//! from its installed database. `shared/competition-names.csv` is the editable two-column table
//! (`database_id,name`) that names the competition index and suspension scopes. It is populated
//! from the installed FM27 database for the local save; add rows for other databases and rebuild
//! to extend it.

use std::collections::HashMap;
use std::sync::OnceLock;

pub fn competition_name(database_id: u32) -> Option<&'static str> {
    names().get(&database_id).map(String::as_str)
}

pub fn name_count() -> usize {
    names().len()
}

fn names() -> &'static HashMap<u32, String> {
    static NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut map = HashMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(include_str!("../../../shared/competition-names.csv").as_bytes());
        for row in reader.records().flatten() {
            let Some(id) = row
                .get(0)
                .and_then(|value| value.trim().parse::<u32>().ok())
            else {
                continue;
            };
            let name = row.get(1).unwrap_or("").trim();
            if !name.is_empty() {
                map.insert(id, name.to_owned());
            }
        }
        map
    })
}
