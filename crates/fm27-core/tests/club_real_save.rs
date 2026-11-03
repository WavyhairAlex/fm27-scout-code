//! Real-save regression coverage for club search, finance, stadiums and edits.
//!
//! Set `FM27_TEST_SAVE` to a 27.0.0+2486695 career save and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -- --ignored
//! ```

use fm27_core::club_editor::{ClubEditRequest, ClubEditor, ClubFinanceEdit, StadiumCapacityEdit};
use fm27_core::store::{ClubQuery, Store};
use fm27_core::{container::SaveFile, metadata::read_metadata, parse_save};

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn reads_club_finances_stadiums_and_queries() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");
    assert!(parsed.clubs.len() > 20_000, "club count");
    assert!(!parsed.club_finances.is_empty(), "finance rows");
    assert!(!parsed.stadiums.is_empty(), "stadium rows");
    assert!(
        parsed
            .clubs
            .iter()
            .any(|club| club.corporate_facilities.is_some())
    );

    let mut store = Store::open_in_memory().expect("open store");
    let save_id = store.insert_parsed(&parsed).expect("insert");
    let page = store
        .query_clubs(&ClubQuery {
            save_id,
            search: Some("Parma".to_owned()),
            page_size: Some(10),
            ..ClubQuery::default()
        })
        .expect("club query");
    assert!(page.total >= 1);
    let parma = page
        .rows
        .iter()
        .find(|club| club.uid == 1155)
        .expect("Parma");
    assert!(parma.finance_month.is_some());
    assert!(parma.balance.is_some());

    let parma_club = parsed
        .clubs
        .iter()
        .find(|club| club.uid == parma.uid)
        .expect("Parma club row");
    assert_eq!(parma_club.training_facilities, Some(16));
    assert_eq!(parma_club.youth_facilities, Some(13));
    assert_eq!(parma_club.junior_coaching, Some(10));
    assert_eq!(parma_club.youth_recruitment, Some(10));
    assert_eq!(parma_club.youth_level, None);

    if !fm27_core::youth_level::read_english_youth_levels().is_empty() {
        for (uid, expected) in [
            (602, Some(1)), // Arsenal
            (639, Some(2)), // Coventry City
            (677, Some(3)), // Luton Town
            (729, Some(4)), // Tranmere Rovers
        ] {
            let club = parsed
                .clubs
                .iter()
                .find(|club| club.uid == uid)
                .expect("club row");
            assert_eq!(club.youth_level, expected, "youth level for club {uid}");
        }
    }

    for (uid, expected) in [
        (602, (19, 16, 17, 15)),  // Arsenal
        (676, (20, 20, 17, 16)),  // Liverpool
        (679, (20, 20, 19, 20)),  // Manchester City
        (728, (20, 20, 16, 16)),  // Tottenham Hotspur
        (1150, (17, 13, 12, 12)), // Napoli
        (1174, (15, 14, 14, 14)), // Torino
    ] {
        let club = parsed
            .clubs
            .iter()
            .find(|club| club.uid == uid)
            .expect("club row");
        assert_eq!(
            (
                club.training_facilities,
                club.youth_facilities,
                club.junior_coaching,
                club.youth_recruitment,
            ),
            (
                Some(expected.0),
                Some(expected.1),
                Some(expected.2),
                Some(expected.3),
            ),
            "facilities for club {uid}"
        );
    }

    let detail = store
        .get_club(save_id, parma.uid)
        .expect("get club")
        .expect("club detail");
    assert_eq!(detail.club.training_facilities, Some(16));
    assert_eq!(detail.club.youth_facilities, Some(13));
    assert_eq!(detail.club.junior_coaching, Some(10));
    assert_eq!(detail.club.youth_recruitment, Some(10));
    assert!(!detail.finance_months.is_empty());
    assert!(!detail.stadiums.is_empty());
    assert_eq!(
        detail.finance_months.last().map(|row| row.balance),
        Some(parma.balance.unwrap_or_default())
    );

    let parent = store
        .get_club(save_id, 1664)
        .expect("get parent")
        .expect("Athletic Club");
    assert!(
        parent
            .child_clubs
            .iter()
            .any(|child| child.uid == 1665 && child.name == "Bilbao Athletic"),
        "affiliated club link"
    );

    let options = store.club_filter_options(save_id).expect("club options");
    assert!(!options.nations.is_empty());
    assert!(options.clubs_with_finances > 0);
    assert!(options.clubs_with_stadiums > 0);
}

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn edits_low_risk_club_fields_in_a_copy() {
    let source = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&source).expect("parse source");
    let club = parsed
        .clubs
        .iter()
        .find(|club| club.corporate_facilities.is_some() && club.training_facilities.is_some())
        .expect("a club with finance and facility data");
    let stadium = parsed
        .stadiums
        .iter()
        .find(|stadium| stadium.owner_club_uid == Some(club.uid))
        .expect("a stadium owned by that club");
    let finance = parsed
        .club_finances
        .iter()
        .filter(|row| row.club_uid == club.uid)
        .max_by(|left, right| left.month.cmp(&right.month))
        .expect("latest finance row");

    let request = ClubEditRequest {
        reputation: Some(club.reputation.unwrap_or(5000) + 1),
        corporate_facilities: Some(club.corporate_facilities.unwrap_or(10)),
        training_facilities: Some(15),
        youth_facilities: Some(14),
        junior_coaching: Some(13),
        youth_recruitment: Some(12),
        finances: Some(ClubFinanceEdit {
            balance: Some(finance.balance + 1),
            transfer_budget_remaining: Some(finance.transfer_budget_remaining + 1),
            wage_budget_weekly: Some(finance.wage_budget_weekly + 1),
        }),
        stadiums: vec![StadiumCapacityEdit {
            stadium_uid: stadium.uid,
            all_seater_capacity: Some(stadium.all_seater_capacity + 1),
            expansion_capacity: Some(stadium.expansion_capacity + 1),
        }],
    };

    let save = SaveFile::open(&source).expect("open save");
    let metadata = read_metadata(&save).expect("metadata");
    let mut editor = ClubEditor::open(&save, &metadata).expect("open club editor");
    request.apply(&mut editor, club.uid).expect("apply");
    editor.validate().expect("validate");

    let directory = tempfile::tempdir().expect("tempdir");
    let output = directory.path().join("club-edited.fm");
    editor
        .write_as(&save, &output, 1, Some("FM27 Scout Club Test"))
        .expect("write");
    let edited = parse_save(&output).expect("parse edited");
    request.verify(&edited, club.uid).expect("verify");
}
