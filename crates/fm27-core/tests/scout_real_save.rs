//! Real-save regression test for scouting advice.
//!
//! Set `FM27_TEST_SAVE` to an FM27 career save with a human manager and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test -p fm27-core --release \
//!   --test scout_real_save -- --ignored
//! ```

use fm27_core::parse_save;
use fm27_core::scouting::{DepthStatus, ScoutPurpose, ScoutRequest};
use fm27_core::store::Store;

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn recommends_players_for_the_managed_club() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");
    let mut store = Store::open_in_memory().expect("open in-memory store");
    let save_id = store.insert_parsed(&parsed).expect("insert parsed save");
    let managed = store
        .managed_club(save_id)
        .expect("managed club query")
        .expect("the save should name a managed club");

    let overview = store
        .scout_overview(save_id, managed.club_uid)
        .expect("scout overview");
    assert_eq!(overview.club_uid, managed.club_uid);
    assert_eq!(overview.lines.len(), 10);
    assert!(
        overview.squad_size > 0,
        "the managed club should have players"
    );
    assert!(
        overview.level > 0,
        "the best eleven should have an ability level"
    );
    assert!(
        overview
            .lines
            .iter()
            .map(|line| line.players.len())
            .sum::<usize>()
            <= overview.squad_size
    );

    let position = overview
        .lines
        .iter()
        .find(|line| matches!(line.status, DepthStatus::Missing | DepthStatus::Thin))
        .map(|line| line.positions[0])
        .unwrap_or("stc");
    let request = ScoutRequest {
        save_id,
        club_uid: managed.club_uid,
        position: position.to_owned(),
        purpose: ScoutPurpose::Any,
        how_many: Some(20),
        ..ScoutRequest::default()
    };
    let advice = store.scout_advice(&request).expect("scout advice");
    assert!(advice.examined > 0, "candidate scan should look at players");
    assert!(
        !advice.suggestions.is_empty(),
        "an any-purpose request should recommend players"
    );
    assert!(
        advice.suggestions.len() <= 20,
        "the limit should cap the list"
    );
    assert!(
        advice
            .suggestions
            .iter()
            .all(|suggestion| suggestion.fitting.player.club_uid != Some(managed.club_uid)),
        "the managed club's own players must not be suggested"
    );
    assert!(
        advice
            .suggestions
            .iter()
            .all(|suggestion| (0..=100).contains(&suggestion.probability)),
        "probabilities stay in range"
    );
}
