//! Real-save regression test for the staff directory.
//!
//! Set `FM27_TEST_SAVE` to an FM27 career save and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -- --ignored staff
//! ```

use fm27_core::parse_save;
use fm27_core::store::{StaffQuery, StaffRoleFilter, Store};

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn reads_the_reference_career_staff_directory() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");

    assert!(
        parsed.staff.len() > 5_000,
        "expected a full staff section, got {} rows",
        parsed.staff.len()
    );
    let with_attributes = parsed
        .staff
        .iter()
        .filter(|staff| staff.attributes.is_some())
        .count();
    let with_roles = parsed
        .staff
        .iter()
        .filter(|staff| staff.roles.is_some())
        .count();
    assert!(
        with_attributes * 100 >= parsed.staff.len() * 95,
        "expected profile attributes for nearly every staff row, got {with_attributes}"
    );
    assert!(
        with_roles * 100 >= parsed.staff.len() * 80,
        "expected role suitability for most staff rows, got {with_roles}"
    );
    assert!(
        parsed
            .staff
            .iter()
            .all(|staff| staff.club_uid.is_some() && staff.job_code.is_some()),
        "every staff row should hold a club job"
    );
    assert!(parsed.staff.iter().all(|staff| {
        staff
            .attributes
            .as_ref()
            .is_none_or(|attributes| attributes.values.len() == 21)
            && staff
                .roles
                .as_ref()
                .is_none_or(|roles| roles.iter().all(|score| (1..=20).contains(score)))
    }));

    let mut store = Store::open_in_memory().expect("open in-memory store");
    let save_id = store.insert_parsed(&parsed).expect("insert parsed save");

    let page = store
        .query_staff(&StaffQuery {
            save_id,
            page_size: Some(100),
            ..StaffQuery::default()
        })
        .expect("staff page");
    assert_eq!(page.total as usize, parsed.staff.len());
    assert_eq!(page.rows.len(), 100);

    let coaches = store
        .query_staff(&StaffQuery {
            save_id,
            roles: vec![StaffRoleFilter {
                key: "coach".to_owned(),
                min: Some(15),
                max: None,
            }],
            page_size: Some(100),
            ..StaffQuery::default()
        })
        .expect("role filter");
    assert!(!coaches.rows.is_empty(), "expected strong coaches");
    assert!(
        coaches
            .rows
            .iter()
            .all(|row| row.roles.as_ref().is_some_and(|roles| roles[2] >= 15))
    );

    let options = store
        .staff_filter_options(save_id, None)
        .expect("filter options");
    assert!(options.jobs.len() > 5);
    assert!(options.with_contract > 0);
    assert_eq!(options.with_attributes as usize, parsed.staff.len());
}
