//! Best-effort current-league resolution for clubs and their players.
//!
//! A club record carries competition entries for the seasons the save keeps. The first-team
//! league is the same competition in three nearby season lists, so a league id is accepted only
//! when it appears in all three windows below. The windows are relative to the club record and
//! were checked against the top divisions of the major leagues in the reference save.
//!
//! The save stores no competition name, so the league id is looked up through the optional
//! `shared/competition-names.csv` table. Clubs whose nation has no supported top-league name,
//! or whose record is a short stub, keep no league rather than a guessed one.

use std::collections::HashMap;

use crate::clubs::{ClubIndex, ClubRecordSpan};
use crate::competitions::CompetitionIndex;
use crate::layout::read_u32;
use crate::nations::NationIndex;

/// Three competition lists kept in a full club record: last, current and next season.
const LEAGUE_WINDOWS: [(usize, usize); 3] = [(350, 500), (600, 750), (1100, 1250)];

/// Resolve the current league competition id for every club we can prove.
pub fn resolve_club_leagues(
    game_db: &[u8],
    clubs: &ClubIndex,
    competitions: &CompetitionIndex,
    nations: &NationIndex,
) -> HashMap<u32, u32> {
    let mut leagues = HashMap::new();
    for club in &clubs.clubs {
        let Some(span) = clubs.record_span(club.uid) else {
            continue;
        };
        let Some(nation_database_id) = nations.database_id_for(club.nation_id as u16) else {
            continue;
        };
        let Some(competition_id) = top_league_competition_id(nation_database_id, competitions)
        else {
            continue;
        };
        if appears_in_all_windows(game_db, span, competition_id) {
            leagues.insert(club.uid, competition_id);
        }
    }
    leagues
}

fn top_league_competition_id(
    nation_database_id: u32,
    competitions: &CompetitionIndex,
) -> Option<u32> {
    competitions
        .competitions
        .iter()
        .filter(|competition| {
            competition
                .name
                .as_deref()
                .is_some_and(|name| is_top_league(nation_database_id, name))
        })
        .map(|competition| competition.id)
        .min()
}

fn is_top_league(nation_database_id: u32, name: &str) -> bool {
    match nation_database_id {
        765 => name == "Premier League",         // England
        776 => name == "Italian Serie A",        // Italy
        796 => name == "Spanish First Division", // Spain
        771 => name == "Bundesliga",             // Germany
        769 => name.starts_with("Ligue 1"),      // France
        784 => name == "VriendenLoterij Eredivisie" || name == "Eredivisie", // Netherlands
        788 => name == "Portuguese Premier League", // Portugal
        _ => false,
    }
}

fn appears_in_all_windows(game_db: &[u8], span: ClubRecordSpan, competition_id: u32) -> bool {
    let record_len = span.record_end.saturating_sub(span.record_start);
    let mut hits = 0;
    for (window_start, window_end) in LEAGUE_WINDOWS {
        let start = window_start.min(record_len);
        let end = window_end.min(record_len);
        if start >= end {
            continue;
        }
        let found = (start..end.saturating_sub(3))
            .any(|offset| read_u32(game_db, span.record_start + offset) == Some(competition_id));
        if found {
            hits += 1;
        }
    }
    hits == LEAGUE_WINDOWS.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Competition;

    fn competition(id: u32, name: &str) -> Competition {
        Competition {
            id,
            database_id: None,
            name: Some(name.to_owned()),
            stage_ids: Vec::new(),
        }
    }

    #[test]
    fn top_leagues_are_named_by_their_nation() {
        let index = CompetitionIndex {
            competitions: vec![
                competition(7, "Premier League"),
                competition(22, "Italian Serie A"),
                competition(51, "Spanish First Division"),
                competition(16, "Bundesliga"),
                competition(11, "Ligue 1 McDonald's"),
                competition(8, "Sky Bet Championship"),
            ],
            ..CompetitionIndex::default()
        };
        assert_eq!(top_league_competition_id(765, &index), Some(7));
        assert_eq!(top_league_competition_id(776, &index), Some(22));
        assert_eq!(top_league_competition_id(796, &index), Some(51));
        assert_eq!(top_league_competition_id(771, &index), Some(16));
        assert_eq!(top_league_competition_id(769, &index), Some(11));
        assert_eq!(top_league_competition_id(999, &index), None);
    }
}
