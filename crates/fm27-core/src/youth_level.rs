//! English Youth Level (EPPP category) from the game's `eng_youth_grades.edt`.
//!
//! FM27 keeps the starting EPPP category in the game installation rather than in
//! the save. The EDT maps club Unique IDs to categories 1–4, where 1 is the
//! highest. Only English clubs have a value; missing or unreadable files leave
//! the field empty instead of guessing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Read the starting English EPPP categories from the installed game.
///
/// The file is optional: a missing game install, a different platform layout, or
/// an unreadable file yields an empty map.
pub fn read_english_youth_levels() -> HashMap<u32, u8> {
    let Some(path) = locate_youth_grades_file() else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    parse_youth_grades(&text)
}

/// Parse `STARTING_EPPP_CATEGORY` lines from an EDT document.
fn parse_youth_grades(text: &str) -> HashMap<u32, u8> {
    let mut levels = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("\"STARTING_EPPP_CATEGORY\"") {
            continue;
        }
        // The EDT format is: "COMMAND" "UID-Name" "CATEGORY"
        let parts: Vec<&str> = line.split('"').collect();
        if parts.len() < 6 {
            continue;
        }
        let Some((uid, _name)) = parts[3].split_once('-') else {
            continue;
        };
        let Ok(uid) = uid.parse::<u32>() else {
            continue;
        };
        let Ok(level) = parts[5].parse::<u8>() else {
            continue;
        };
        if (1..=4).contains(&level) {
            levels.insert(uid, level);
        }
    }
    levels
}

fn locate_youth_grades_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("FM27_YOUTH_GRADES_FILE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let home = std::env::var_os("HOME")?;
    let steam = PathBuf::from(home)
        .join("Library/Application Support/Steam/steamapps/common/Football Manager 27");
    let database_dir = steam.join(
        "fm.app/Contents/PlugIns/game_plugin.bundle/Contents/Resources/shared/data/database/db",
    );
    find_in_database_dir(&database_dir)
}

fn find_in_database_dir(database_dir: &Path) -> Option<PathBuf> {
    let mut versions: Vec<PathBuf> = std::fs::read_dir(database_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    // Prefer the newest database shipped with the game (FM27 uses 2700).
    versions.sort_by_key(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse::<u32>().ok())
            .unwrap_or(0)
    });
    versions.reverse();
    for version in versions {
        let path = version.join("edt/permanent/eng_youth_grades.edt");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_youth_grades;

    #[test]
    fn parses_eppp_categories_from_the_edt_format() {
        let text = r#"
## comment
"STARTING_EPPP_CATEGORY" "602-Arsenal" "1"
"STARTING_EPPP_CATEGORY" "639-Coventry City" "2"
"STARTING_EPPP_CATEGORY" "677-Luton Town" "3"
"STARTING_EPPP_CATEGORY" "729-Tranmere Rovers" "4"
"STARTING_EPPP_CATEGORY" "999-Not a category" "9"
"#;
        let levels = parse_youth_grades(text);
        assert_eq!(levels.get(&602), Some(&1));
        assert_eq!(levels.get(&639), Some(&2));
        assert_eq!(levels.get(&677), Some(&3));
        assert_eq!(levels.get(&729), Some(&4));
        assert_eq!(levels.get(&999), None);
    }
}
