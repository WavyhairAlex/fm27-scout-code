//! FM27 Scout core: a read-only Football Manager 27 save parser.
//!
//! The core is platform independent and shared by the CLI and the Tauri app.

pub mod ability;
mod bytes;
pub mod club_editor;
pub mod clubs;
pub mod competitions;
pub mod container;
pub mod contracts;
pub mod editor;
pub mod error;
pub mod export;
pub mod facilities;
pub mod finances;
pub mod injuries;
pub mod layout;
mod layout_tests;
pub mod leagues;
pub mod metadata;
pub mod model;
pub mod moneyball;
pub mod names;
pub mod nations;
pub mod parse;
pub mod persons;
pub mod player_scan;
pub mod reference;
pub mod roles;
pub mod season_stats;
pub mod stadiums;
pub mod staff;
pub mod stats_moneyball;
pub mod store;
pub mod suspensions;
pub mod youth_level;

pub use club_editor::{
    ClubEditRequest, ClubEditor, ClubFacilityEdit, ClubFinanceEdit, StadiumCapacityEdit,
};
pub use editor::{EditChange, PlayerEditor};
pub use error::{Error, Result};
pub use model::{
    Club, ClubDetail, ClubLink, Contract, FinanceMonth, ParseReport, Player, SaveSummary, Stadium,
    Staff,
};
pub use parse::{
    ParseProgress, ParsedSave, parse_save, parse_save_with_progress, parser_display_version,
    parser_version,
};

/// Open a save and return its container metadata without decompressing `game_db`.
pub fn inspect_save(path: impl AsRef<std::path::Path>) -> Result<SaveSummary> {
    let save = container::SaveFile::open(path.as_ref())?;
    let metadata = metadata::read_metadata(&save)?;
    let path = path.as_ref();
    Ok(SaveSummary {
        path: path.display().to_string(),
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        file_size: save.fingerprint.file_size,
        mtime_ns: save.fingerprint.mtime_ns,
        fingerprint: save.fingerprint.digest.clone(),
        save_name: save.save_name.clone(),
        game: metadata.game,
        build: metadata.build,
        game_date: metadata.game_date,
        db_version: metadata.db_version,
        section_count: save
            .directory
            .top_entries
            .iter()
            .filter(|entry| entry.is_section())
            .count(),
        decompressed_bytes: save.total_declared_bytes(),
    })
}
