//! Real-save regression test.
//!
//! Fixtures are intentionally not committed. Set `FM27_TEST_SAVE` to a
//! verified-build career save and run:
//!
//! ```text
//! FM27_TEST_SAVE="/path/to/save.fm" cargo test --release -- --ignored
//! ```

use fm27_core::parse_save;

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn parses_the_reference_career() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");
    assert_eq!(parsed.players.len(), 46_273, "player count");
    assert_eq!(parsed.clubs.len(), 26_214, "club count");
    assert_eq!(parsed.report.contracts, 44_341, "contract count");
    assert_eq!(parsed.report.competitions, 1_724, "competition count");
    assert_eq!(parsed.report.competition_stages, 3_459, "stage count");
    assert_eq!(
        parsed.report.competitions_with_database_id, 1_586,
        "competitions with an editor database id"
    );
    assert_eq!(parsed.competitions[1].id, 1);
    assert_eq!(parsed.competitions[1].database_id, Some(2));
    assert_eq!(parsed.competitions[1].stage_ids, vec![187]);
    assert_eq!(parsed.report.players_with_person, 46_272);
    assert_eq!(parsed.report.markerless_players, 485);

    let modric = parsed
        .players
        .iter()
        .find(|player| player.name.as_deref() == Some("Luka Modrić"))
        .expect("Luka Modrić");
    assert_eq!(modric.ca, 158);
    assert_eq!(modric.pa, Some(180));
    assert_eq!(modric.club_name.as_deref(), Some("Casciavit Milano"));
    assert_eq!(modric.attributes.get("passing"), Some(18));
    assert_eq!(modric.attributes.get("important_matches"), Some(18));
    assert_eq!(
        modric.personality.map(|value| value.professionalism),
        Some(20)
    );
    assert_eq!(
        modric.contract.as_ref().and_then(|value| value.wage),
        Some(106_965)
    );
    assert_eq!(
        modric
            .contract
            .as_ref()
            .and_then(|value| value.end.as_deref()),
        Some("2027-06-30")
    );
    assert_eq!(modric.nation_name.as_deref(), Some("克罗地亚"));
    assert_eq!(modric.gender, Some(0));
    assert!(modric.best_role.is_some());

    assert!(
        parsed
            .players
            .iter()
            .filter(|player| player.gender == Some(1))
            .count()
            > 10_000,
        "women's players are present"
    );
    assert!(
        parsed.players.iter().any(|player| player.transfer_listed),
        "transfer-listed players are present"
    );
    assert!(
        parsed.players.iter().any(|player| player.loan_listed),
        "loan-listed players are present"
    );
    assert_eq!(
        parsed
            .players
            .iter()
            .map(|player| player.suspensions.len())
            .sum::<usize>(),
        17
    );
    assert_eq!(
        parsed
            .players
            .iter()
            .map(|player| player.injuries.len())
            .sum::<usize>(),
        783
    );
}

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn resolves_second_nationality_names() {
    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");

    let with_second: Vec<_> = parsed
        .players
        .iter()
        .filter(|player| !player.second_nation_ids.is_empty())
        .collect();
    assert!(!with_second.is_empty(), "players with other nationalities");

    let mut total = 0usize;
    let mut resolved = 0usize;
    for player in &with_second {
        assert_eq!(
            player.second_nation_ids.len(),
            player.second_nation_names.len(),
            "names stay aligned with ids for {}",
            player.uid
        );
        for name in &player.second_nation_names {
            total += 1;
            if name.is_some() {
                resolved += 1;
            }
        }
    }
    assert!(resolved > 0, "at least one name resolves");
    assert!(
        resolved * 100 >= total * 99,
        "at least 99% of other nationalities resolve ({resolved}/{total})"
    );
}

#[test]
#[ignore = "requires a real FM27 save and FM27_TEST_SAVE"]
fn caches_queries_and_exports_the_reference_career() {
    use fm27_core::export::write_export;
    use fm27_core::store::{PlayerQuery, Store};

    let path = std::env::var("FM27_TEST_SAVE").expect("set FM27_TEST_SAVE");
    let parsed = parse_save(&path).expect("parse save");
    let mut store = Store::open_in_memory().expect("open store");
    let save_id = store.insert_parsed(&parsed).expect("insert");

    let page = store
        .query_players(&PlayerQuery {
            save_id,
            search: Some("Modrić".to_owned()),
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("query");
    assert!(page.total >= 1);
    assert!(
        page.rows
            .iter()
            .any(|row| row.name.as_deref() == Some("Luka Modrić"))
    );
    assert!(page.rows.iter().all(|row| row.best_role_score.is_some()));

    let attributes = store
        .query_players(&PlayerQuery {
            save_id,
            age_max: Some(23),
            ca_min: Some(140),
            attributes: vec![fm27_core::store::AttributeFilter {
                key: "passing".to_owned(),
                min: Some(16),
                max: None,
            }],
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("attribute query");
    assert!(
        attributes
            .rows
            .iter()
            .all(|row| row.age.unwrap_or(99) <= 23)
    );

    let competitions = store.competitions(save_id).expect("competitions");
    assert_eq!(competitions.len(), 1_724);
    assert_eq!(competitions[1].database_id, Some(2));
    assert_eq!(competitions[1].stage_ids, vec![187]);

    let overview = store.analysis_overview(save_id, None).expect("overview");
    assert_eq!(overview.total_players, 46_273);
    assert!(!overview.top_clubs.is_empty());
    assert!(overview.median_ca > 0);

    let scoped = store
        .analysis_overview(save_id, Some(1099))
        .expect("scoped overview");
    assert!(scoped.total_players > 0);
    assert!(scoped.total_players < overview.total_players);
    assert!(scoped.top_clubs.iter().any(|club| club.club_uid == 1099));

    let managed = store
        .managed_club(save_id)
        .expect("managed club")
        .expect("managed club present");
    assert_eq!(managed.club_uid, 1099);

    let wonderkids = store
        .query_players(&PlayerQuery {
            save_id,
            age_max: Some(21),
            pa_min: Some(140),
            pa_ca_gap_min: Some(10),
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("wonderkids");
    assert!(wonderkids.total > 0);
    assert!(wonderkids.rows.iter().all(|row| row.age.unwrap_or(99) <= 21
        && row.pa.unwrap_or(0) >= 140
        && row.pa.unwrap_or(0) as i32 - row.ca as i32 >= 10));

    let squad = store.squad_analysis(save_id, 1099).expect("squad");
    assert_eq!(squad.club_name, "Casciavit Milano");
    assert!(squad.player_count > 0);
    assert!(!squad.players.is_empty());

    let options = store.filter_options(save_id).expect("filter options");
    assert!(!options.clubs.is_empty());
    assert!(!options.nations.is_empty());
    assert!(!options.genders.is_empty());
    assert!(!options.availability.is_empty());
    assert_eq!(options.listing.len(), 2);
    assert!(options.listing.iter().all(|option| option.count > 0));
    assert!(
        options
            .nations
            .iter()
            .any(|nation| nation.name == "克罗地亚")
    );
    assert!(options.min_ca.unwrap_or(0) <= options.max_ca.unwrap_or(200));

    let filtered_options = store
        .filter_options_with_query(
            save_id,
            Some(&PlayerQuery {
                save_id,
                genders: vec![1],
                ..PlayerQuery::default()
            }),
        )
        .expect("filtered options");
    let women_count = filtered_options
        .genders
        .iter()
        .find(|gender| gender.id == 1)
        .map(|gender| gender.count)
        .unwrap_or(0);
    assert!(women_count > 10_000);

    let listed = store
        .query_players(&PlayerQuery {
            save_id,
            listings: vec!["sale".to_owned()],
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("transfer-listed query");
    assert!(listed.total > 0);
    assert!(listed.rows.iter().all(|row| row.transfer_listed));

    let available_for_loan = store
        .query_players(&PlayerQuery {
            save_id,
            listings: vec!["loan".to_owned()],
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("loan-listed query");
    assert!(available_for_loan.total > 0);
    assert!(available_for_loan.rows.iter().all(|row| row.loan_listed));

    let women = store
        .query_players(&PlayerQuery {
            save_id,
            genders: vec![1],
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("women's query");
    assert!(women.total > 10_000);
    assert!(women.rows.iter().all(|row| row.gender == Some(1)));

    let multi = store
        .query_players(&PlayerQuery {
            save_id,
            sort: vec![
                fm27_core::store::SortRule {
                    by: "ca".to_owned(),
                    dir: "desc".to_owned(),
                },
                fm27_core::store::SortRule {
                    by: "age".to_owned(),
                    dir: "asc".to_owned(),
                },
            ],
            page_size: Some(20),
            ..PlayerQuery::default()
        })
        .expect("multi sort");
    assert!(multi.rows.windows(2).all(|pair| pair[0].ca >= pair[1].ca));

    let history = store.growth_history(save_id, 652912).expect("growth");
    assert!(!history.is_empty());

    let player = store
        .get_player(save_id, 652912, true)
        .expect("get player")
        .expect("Modrić");
    assert_eq!(player.name.as_deref(), Some("Luka Modrić"));

    let redacted = store
        .get_player(save_id, 652912, false)
        .expect("get redacted")
        .expect("Modrić");
    assert_eq!(redacted.attributes.get("consistency"), Some(0));
    assert!(redacted.personality.is_none());

    let rows = store
        .query_all(&PlayerQuery {
            save_id,
            ..PlayerQuery::default()
        })
        .expect("all rows");
    assert_eq!(rows.len(), 46_273);

    let dir = tempfile::tempdir().expect("temp dir");
    let csv = write_export(&dir.path().join("players.csv"), "csv", &rows).expect("csv");
    assert_eq!(csv.rows, 46_273);
    let json = write_export(&dir.path().join("players.json"), "json", &rows).expect("json");
    assert_eq!(json.rows, 46_273);
}
