//! Real-save editor regression test.
//!
//! Fixtures are intentionally not committed. Set `FM27_TEST_SAVE` to a
//! 27.0.0+2486695 career save and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -- --ignored
//! ```

use fm27_core::container::SaveFile;
use fm27_core::editor::{AttributeEdit, PlayerEditRequest, PlayerEditor};
use fm27_core::layout::trait_name;
use fm27_core::metadata::read_metadata;
use fm27_core::parse_save;

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn edits_ca_pa_attributes_personality_and_traits_in_a_copy() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse source save");
    let source = parsed
        .players
        .iter()
        .find(|player| {
            player.name.is_some()
                && player.personality.is_some()
                && player.ca < 200
                && player.pa.is_some()
        })
        .expect("a player with CA, PA and personality");
    let untouched = parsed
        .players
        .iter()
        .find(|player| player.uid != source.uid)
        .expect("a second player");

    let save = SaveFile::open(&path).expect("open container");
    let metadata = read_metadata(&save).expect("metadata");
    let mut editor = PlayerEditor::open(&save, &metadata).expect("open editor");

    let new_ca = source.ca + 1;
    let new_pa = source
        .pa
        .unwrap_or(200)
        .max(i16::try_from(new_ca + 1).unwrap())
        .min(200);
    let trait_bit = (0..64u8)
        .find(|bit| {
            trait_name(*bit).is_some()
                && source
                    .trait_bits
                    .is_none_or(|bits| bits & (1u64 << *bit) == 0)
        })
        .expect("an unset named trait");
    let request = PlayerEditRequest {
        ca: Some(new_ca),
        pa: Some(new_pa),
        attributes: vec![
            AttributeEdit {
                key: "crossing".to_owned(),
                value: 100,
            },
            AttributeEdit {
                key: "consistency".to_owned(),
                value: 100,
            },
        ],
        personality: vec![AttributeEdit {
            key: "professionalism".to_owned(),
            value: 20,
        }],
        traits: vec![trait_bit],
        untraits: vec![],
    };
    request.apply(&mut editor, source.uid).expect("apply edits");
    editor.validate().expect("validate edits");

    let directory = tempfile::tempdir().expect("tempdir");
    let output = directory.path().join("edited.fm");
    let report = editor.write(&save, &output, 3).expect("write edited save");
    assert_eq!(report.sections_replaced, vec!["game_db"]);

    let edited = parse_save(&output).expect("parse edited save");
    assert_eq!(edited.players.len(), parsed.players.len());
    assert_eq!(edited.clubs.len(), parsed.clubs.len());
    assert_eq!(edited.report.contracts, parsed.report.contracts);

    let changed = edited
        .players
        .iter()
        .find(|player| player.uid == source.uid)
        .expect("edited player");
    assert_eq!(changed.ca, new_ca);
    assert_eq!(changed.pa, Some(new_pa));
    assert_eq!(changed.raw_attributes.get("crossing"), Some(100));
    assert_eq!(changed.raw_attributes.get("consistency"), Some(100));
    assert_eq!(
        changed.personality.map(|value| value.professionalism),
        Some(20)
    );
    assert!(
        changed
            .trait_bits
            .is_some_and(|bits| bits & (1u64 << trait_bit) != 0)
    );
    request
        .verify(changed)
        .expect("verify the shared edit request");

    let unchanged = edited
        .players
        .iter()
        .find(|player| player.uid == untouched.uid)
        .expect("untouched player");
    assert_eq!(unchanged.ca, untouched.ca);
    assert_eq!(unchanged.pa, untouched.pa);
    assert_eq!(
        unchanged.name.as_deref(),
        untouched.name.as_deref(),
        "an unrelated player must not change"
    );
}
