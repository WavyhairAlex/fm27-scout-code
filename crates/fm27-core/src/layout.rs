//! FM27 layout constants, ported from the verified fmsave reference implementation
//! and checked against real career saves.

use chrono::{Datelike, NaiveDate};

pub const GAME: &str = "FM27";
/// Latest FM27 build whose save layout has been verified.
pub const BUILD: &str = "27.0.1+2503222";
/// Builds that share the verified FM27 save layout. Editing is limited to these
/// exact builds so a layout change fails closed instead of guessing offsets.
pub const EDITABLE_BUILDS: &[&str] = &["27.0.0+2486695", "27.0.0+2493426", "27.0.1+2494687", BUILD];
pub const GAME_DB_SCHEMA: u32 = 4000;

pub fn is_editable_build(build: &str) -> bool {
    EDITABLE_BUILDS.contains(&build)
}

pub fn editable_builds() -> String {
    EDITABLE_BUILDS.join(", ")
}
/// Cache/parser revision. This is intentionally independent of the app version:
/// bump the leading version and/or the suffix only when parsing behavior or the
/// cached data shape changes. The suffix invalidates caches written before the
/// complete player-trait and nation-name tables landed, before the current
/// injury windows were read from the stride-14 arrays, before the bundled
/// competition name table was populated, before club leagues were resolved,
/// before the closing person header accepted descending Unique IDs, before
/// staff objects were decoded into their own cached table, before players
/// carried resolved names for their other nationalities, before the managed
/// club was read from the first human manager, before build 27.0.1+2494687
/// was verified and the managed club was read for it, and before build
/// 27.0.1+2503222 was verified against 27.0.1+2494687 and the managed club was
/// read for it, before players carried their registered team slot, and before
/// the fixture calendar was joined to competitions to label reserve and youth
/// squads.
pub const PARSER_VERSION: &str = concat!(
    "1.1.7",
    "+nation-names+listing+listing-flags+injury-windows+club-finance+stadiums",
    "+club-facilities+youth-level-edt+competition-names+club-leagues+season-stats+unique-ids",
    "+staff+second-nations+managed-club+build-2494687+build-2503222+squad-slots",
    "+squad-fixtures"
);
pub const EARLIEST_YEAR: i32 = 1901;
pub const LATEST_YEAR: i32 = 2200;
pub const MISSING_U32: u32 = 0xFFFF_FFFF;

pub const POSITION_NAMES: [&str; 15] = [
    "GK", "SW", "DL", "DC", "DR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "STC", "WBL", "WBR",
];

pub const POSITION_KEYS: [&str; 15] = [
    "gk", "sw", "dl", "dc", "dr", "dm", "ml", "mc", "mr", "aml", "amc", "amr", "stc", "wbl", "wbr",
];

pub const ATTRIBUTE_KEYS: [&str; 52] = [
    "crossing",
    "dribbling",
    "finishing",
    "heading",
    "long_shots",
    "marking",
    "off_the_ball",
    "passing",
    "penalty_taking",
    "tackling",
    "vision",
    "handling",
    "aerial_reach",
    "command_of_area",
    "communication",
    "kicking",
    "throwing",
    "anticipation",
    "decisions",
    "one_on_ones",
    "positioning",
    "reflexes",
    "first_touch",
    "technique",
    "flair",
    "corners",
    "teamwork",
    "work_rate",
    "long_throws",
    "eccentricity",
    "rushing_out",
    "punching",
    "acceleration",
    "free_kick_taking",
    "strength",
    "stamina",
    "pace",
    "jumping_reach",
    "leadership",
    "dirtiness",
    "balance",
    "bravery",
    "consistency",
    "aggression",
    "agility",
    "important_matches",
    "injury_proneness",
    "versatility",
    "natural_fitness",
    "determination",
    "composure",
    "concentration",
];

pub const HIDDEN_ATTRIBUTE_KEYS: [&str; 5] = [
    "dirtiness",
    "consistency",
    "important_matches",
    "injury_proneness",
    "versatility",
];

pub fn is_hidden_attribute_key(key: &str) -> bool {
    HIDDEN_ATTRIBUTE_KEYS.contains(&key)
}

/// Attributes that do not consume the CA budget. Determination, natural fitness,
/// flair and aggression stayed unchanged in controlled CA rebalancing tests;
/// hidden attributes are exempt too.
pub fn is_ca_exempt_attribute(key: &str) -> bool {
    is_hidden_attribute_key(key)
        || matches!(
            key,
            "determination" | "natural_fitness" | "flair" | "aggression"
        )
}

pub const PERSONALITY_KEYS: [&str; 8] = [
    "adaptability",
    "ambition",
    "loyalty",
    "pressure",
    "professionalism",
    "sportsmanship",
    "temperament",
    "controversy",
];

/// Bit index -> the player trait the game stores in that bit.
///
/// The first 48 entries follow the game's own trait order. The remaining entries
/// were matched against the save's player data (positions and attributes) and
/// the FM27 localisation table. Bit 53 currently has no trait name.
pub const TRAIT_NAMES: [&str; 64] = [
    "Runs With Ball Down Left",
    "Runs With Ball Down Right",
    "Runs With Ball Through Centre",
    "Gets Into Opposition Area",
    "Moves Into Channels",
    "Gets Forward Whenever Possible",
    "Plays Short Simple Passes",
    "Tries Killer Balls Often",
    "Shoots From Distance",
    "Shoots With Power",
    "Places Shots",
    "Curls Ball",
    "Likes To Round Keeper",
    "Likes To Try To Beat Offside Trap",
    "Uses Outside Of Foot",
    "Marks Opponent Tightly",
    "Winds Up Opponents",
    "Argues With Officials",
    "Plays With Back To Goal",
    "Comes Deep To Get Ball",
    "Plays One-Twos",
    "Likes To Lob Keeper",
    "Dictates Tempo",
    "Attempts Overhead Kicks",
    "Looks For Pass Rather Than Attempting To Score",
    "Plays No Through Balls",
    "Stops Play",
    "Knocks Ball Past Opponent",
    "Moves Ball To Right Foot Before Dribble Attempt",
    "Moves Ball To Left Foot Before Dribble Attempt",
    "Dwells On Ball",
    "Arrives Late In Opponents' Area",
    "Tries To Play Way Out Of Trouble",
    "Stays Back At All Times",
    "Avoids Using Weaker Foot",
    "Tries Tricks",
    "Tries Long Range Free Kicks",
    "Dives Into Tackles",
    "Does Not Dive Into Tackles",
    "Cuts Inside From Both Wings",
    "Hugs Line",
    "Gets Crowd Going",
    "Tries First Time Shots",
    "Tries Long Range Passes",
    "Likes Ball Played Into Feet",
    "Hits Free Kicks With Power",
    "Likes To Beat Opponent Repeatedly",
    "Likes To Switch Ball To Wide Areas",
    "Uses Long Throw To Start Counter Attacks",
    "Play Football For as Long as Possible",
    "Possesses Long Flat Throw",
    "Runs With Ball Often",
    "Runs With Ball Rarely",
    "",
    "Does not Move Into Channels",
    "Plays Ball With Feet",
    "Refrains From Taking Long Shots",
    "Cuts Inside From Left Wing",
    "Cuts Inside From Right Wing",
    "Crosses Early",
    "Brings Ball Out of Defence",
    "Demands to Take Free Kicks",
    "Demands to Take Penalties",
    "Will Retire at Top",
];

pub fn trait_name(bit: u8) -> Option<&'static str> {
    TRAIT_NAMES
        .get(usize::from(bit))
        .copied()
        .filter(|name| !name.is_empty())
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerLayout;

impl PlayerLayout {
    pub const MARKER: &'static [u8] = &[0x01, 0x00, 0x6c, 0x07];
    pub const MARKER_OFFSET: usize = 102;
    pub const PINDEX_OFFSET: i32 = -19;
    pub const UID_OFFSET: i32 = -15;
    pub const UID_COPY_OFFSET: i32 = -11;
    pub const OBJECT_KIND_OFFSET: i32 = -7;
    pub const HOME_REPUTATION_OFFSET: i32 = -6;
    pub const CURRENT_REPUTATION_OFFSET: i32 = -4;
    pub const WORLD_REPUTATION_OFFSET: i32 = -2;
    pub const CA_OFFSET: usize = 0;
    pub const PA_OFFSET: usize = 2;
    pub const REPUTATION_BUCKET_OFFSET: usize = 8;
    pub const TEAM_ID_OFFSET: usize = 16;
    pub const RATINGS_OFFSET: usize = 24;
    pub const RATINGS_COUNT: usize = 15;
    pub const ATTRIBUTES_OFFSET: usize = 39;
    pub const ATTRIBUTE_COUNT: usize = 54;
    pub const LEFT_FOOT_INDEX: usize = 24;
    pub const RIGHT_FOOT_INDEX: usize = 25;
    pub const TRANSFER_VALUE_OFFSET: usize = 93;
    pub const TRANSFER_VALUE_PLACEHOLDER: u32 = 300_000_000;
    pub const CLUB_JOIN_DATE_OFFSET: usize = 98;
    pub const MATCH_SHARPNESS_OFFSET: usize = 106;
    pub const CONDITION_OFFSET: usize = 110;
    pub const HEIGHT_OFFSET: usize = 121;
    pub const DECODE_EXTENT: usize = 122;
    pub const ALLOCATED_UID_MINIMUM: u32 = 2_000_000_000;
}

#[derive(Debug, Clone, Copy)]
pub struct PersonLayout;

impl PersonLayout {
    pub const WINDOW_START_OFFSET: usize = 100;
    /// FM27 moved the person block one byte later than FM26.
    pub const PERSONALITY_OFFSET_FROM_BIRTH: usize = 22;
    pub const PERSONALITY_COUNT: usize = 8;
    pub const DATE_ZERO_BYTES_OFFSET_FROM_BIRTH: usize = 15;
    pub const DATE_ZERO_BYTES_COUNT: usize = 6;
    pub const NATION_ID_OFFSET_FROM_BIRTH: usize = 14;
    pub const RELATION_PRESENT_OFFSET_FROM_BIRTH: usize = 38;
    pub const RELATION_COUNT_OFFSET_FROM_BIRTH: usize = 39;
    pub const RELATION_ENTRIES_OFFSET_FROM_BIRTH: usize = 40;
    pub const RELATION_ENTRY_BYTES: usize = 16;
    pub const LEGAL_NAME_LENGTH_MIN: usize = 2;
    pub const LEGAL_NAME_LENGTH_MAX: usize = 80;
    pub const LEGAL_NAME_OFFSET_FROM_BLOCK_START: usize = 19;
    pub const TRAIT_BITS_OFFSET_FROM_BLOCK_START: i32 = -8;
    pub const NAME_ID_LIMIT: u32 = 1 << 23;
    pub const SECOND_NATION_PAIR: (u8, u8) = (8, 9);
    pub const HOME_GROWN_NATION_PAIR: (u8, u8) = (8, 70);
    pub const HOME_GROWN_CLUB_PAIR: (u8, u8) = (1, 72);
    pub const SECOND_NATION_QUALIFIERS: [u8; 2] = [15, 100];
    pub const RELATION_SENTINEL_VALUE: u8 = 0xFF;
}

/// Fixed-size stage rows in the tail of `game_db`, ported from the verified FM26
/// layout that the FM27 preview build still uses. A row opens with the previous
/// stage id, stores the stage id twice, and carries the competition id at +13.
#[derive(Debug, Clone, Copy)]
pub struct StageTableLayout;

impl StageTableLayout {
    pub const ROW_BYTES: usize = 33;
    pub const PREVIOUS_STAGE_ID_OFFSET: usize = 0;
    pub const STAGE_ID_OFFSET: usize = 4;
    pub const STAGE_ID_COPY_OFFSET: usize = 8;
    pub const ZERO_BYTE_OFFSET: usize = 12;
    pub const COMPETITION_ID_OFFSET: usize = 13;
    pub const STAGE_ID_EXCLUSIVE_RANGE: (u32, u32) = (0, 200_000);
    pub const COMPETITION_ID_LIMIT: u32 = 1_000_000;
    pub const SEARCH_BYTES: usize = 2_000_000;
    pub const CHAIN_ROWS: usize = 200;
    pub const RESYNCHRONISATION_BYTES: usize = 4_096;
}

/// The `FF..FF 01` records that pair a stage-space entity id with the game's
/// editor database id. Every offset is relative to the record start, which sits
/// 30 bytes after the marker.
#[derive(Debug, Clone, Copy)]
pub struct CompetitionIdPairLayout;

impl CompetitionIdPairLayout {
    pub const MARKER: &'static [u8] = &[
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0x01,
    ];
    pub const RECORD_OFFSET_FROM_MARKER: usize = 30;
    pub const ENTITY_ID_OFFSET: usize = 0;
    pub const DATABASE_ID_OFFSET: usize = 4;
    pub const DATABASE_ID_COPY_OFFSET: usize = 8;
    pub const ENTITY_ID_RANGE: (u32, u32) = (1, 199_999);
    pub const DATABASE_ID_RANGE: (u32, u32) = (1, i32::MAX as u32);
    pub const CONSTANT_BYTES: [(isize, u8); 15] = [
        (-8, 7),
        (-6, 0),
        (-4, 7),
        (-1, 255),
        (12, 0),
        (15, 1),
        (22, 0),
        (23, 0),
        (24, 0),
        (25, 0),
        (26, 1),
        (32, 2),
        (40, 3),
        (55, 6),
        (60, 7),
    ];
}

/// A fixture-calendar record in the unlisted span after `non_pl_hist_ls`.
///
/// The record's named offset origin is the home team id, so the locator marker
/// and stage id sit before it. FM27 uses the FM26 shape with a two-byte
/// shorter record: 66 bytes, marker `0x1F`, stage-relative fields unchanged.
/// The kick-off date is two packed u16 words at +13; the year is the high word
/// at +15. The season start year at +22 is the field used to keep only the
/// current season for squad labels.
#[derive(Debug, Clone, Copy)]
pub struct FixtureCalendarLayout;

impl FixtureCalendarLayout {
    pub const RECORD_BYTES: usize = 66;
    pub const MARKER_OFFSET: isize = -12;
    pub const MARKER: u8 = 0x1F;
    pub const SENTINEL_OFFSETS: [(usize, u8); 2] = [(4, 0xFF), (11, 0xFF)];
    pub const STAGE_ID_OFFSET: isize = -11;
    pub const HOME_TEAM_ID_OFFSET: usize = 0;
    pub const AWAY_TEAM_ID_OFFSET: usize = 7;
    pub const KICK_OFF_YEAR_OFFSET: usize = 15;
    pub const SEASON_START_YEAR_OFFSET: usize = 22;
    pub const STAGE_ID_LIMIT: u32 = 100_000_000;
    pub const TEAM_ID_LIMIT: u32 = 3_000_000;
    pub const YEARS_BEFORE_CLOCK: i32 = 7;
    pub const YEARS_AFTER_CLOCK: i32 = 8;
    pub const CLUSTER_GAP_BYTES: usize = 1_048_576;
    pub const HEAD_BYTES: usize = 16 * 1024;
    pub const MIN_HEAD_RECORDS: usize = 2;
    pub const MIN_HEAD_FALLBACK: usize = 10;
    pub const LARGE_FRAME_FALLBACK: usize = 4;
    pub const MIN_SEASON_RECORDS: usize = 20;
    pub const MIN_CALENDAR_RECORDS: usize = 100;
}

#[derive(Debug, Clone, Copy)]
pub struct ClubLayout;

impl ClubLayout {
    pub const ANCHOR: &'static [u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
    pub const ANCHOR_OFFSET: usize = 17;
    pub const CLUB_INDEX_OFFSET: usize = 0;
    pub const UID_OFFSET: usize = 4;
    pub const UID_COPY_OFFSET: usize = 8;
    pub const ZERO_BYTE_OFFSET: usize = 12;
    pub const NATION_OFFSET: usize = 13;
    pub const FA_NATION_OFFSET: usize = 21;
    pub const NATION_COPY_OFFSET: usize = 25;
    pub const CITY_OFFSET: usize = 29;
    pub const LONG_NAME_OFFSET: usize = 39;
    pub const NATION_RANGE: (u32, u32) = (0, 999);
    pub const MAX_NAME_BYTES: usize = 255;
    pub const STOP_GAP_BYTES: usize = 64 * 1024;
    pub const NULL_DATE_TRIPLE: [u8; 12] = [1, 0, 0x6c, 0x07, 1, 0, 0x6c, 0x07, 1, 0, 0x6c, 0x07];
    pub const TEAM_COUNTS_OFFSET: usize = 37;
    pub const FIRST_ENTRY_BYTES: usize = 25;
    pub const FIRST_ENTRY_LEAD: u8 = 0x02;
    pub const SECOND_ENTRY_BYTES: usize = 12;
    pub const SECOND_ENTRY_LEAD: u8 = 0x01;
    pub const FILLER_BYTES: usize = 9;
    pub const TEAM_COUNT_RANGE: (u8, u8) = (1, 8);
    pub const TEAM_ID_RANGE: (u32, u32) = (1, 3_000_000);
    pub const AFFILIATE_COUNT_RANGE: (u8, u8) = (0, 8);
    pub const FLOAT_ANCHOR: [u8; 4] = 1.0f32.to_le_bytes();
    pub const FLOAT_ANCHOR_OFFSET: usize = 33;
    pub const MINIMUM_FLOAT_ANCHOR_RECORD_OFFSET: usize = 40;
    pub const GENDER_OFFSET_FROM_TEAM_COUNT: i32 = -1;
}

/// Monthly finance snapshots and the corporate-facilities byte live inside each
/// club record. The row layout is shared with FM26; FM27 changes only the club
/// record framing that surrounds it.
#[derive(Debug, Clone, Copy)]
pub struct FinanceLayout;

impl FinanceLayout {
    pub const ROW_BYTES: usize = 49;
    pub const TAG: u8 = 0x01;
    pub const COUNT_OFFSET: i32 = -4;
    pub const COUNT_RANGE: (u32, u32) = (3, 1_000);
    pub const SHORT_COUNT_RANGE: (u32, u32) = (1, 2);
    pub const BALANCE_RANGE: (i32, i32) = (-400_000_000, 2_000_000_000);
    pub const WEEKLY_MAXIMUM: u32 = 20_000_000;
    pub const BALANCE_OFFSET: usize = 1;
    pub const TRANSFER_ALLOCATED_OFFSET: usize = 5;
    pub const TRANSFER_REMAINING_OFFSET: usize = 9;
    pub const WAGE_BUDGET_OFFSET: usize = 13;
    pub const WAGE_PAYROLL_OFFSET: usize = 17;
    pub const INCOME_EXCLUDING_TRANSFERS_OFFSET: usize = 21;
    pub const NET_TRANSFERS_OFFSET: usize = 25;
    pub const WAGE_BILL_OFFSET: usize = 29;
    pub const NET_OFFSET: usize = 33;
    pub const EXPENDITURE_EXCLUDING_TRANSFERS_OFFSET: usize = 37;
    pub const TOTAL_INCOME_OFFSET: usize = 41;
    pub const TOTAL_EXPENDITURE_OFFSET: usize = 45;
    /// Records shorter than this never hold a finance chain on the saves
    /// measured, so skipping them keeps the per-club search cheap.
    pub const MINIMUM_RECORD_BYTES: usize = 1_500;
    /// Corporate facilities sit this many bytes past the end of the chain.
    pub const FACILITY_OFFSET_AFTER_CHAIN: usize = 50;
    pub const FACILITY_RANGE: (u8, u8) = (1, 20);
}

/// The stadium table in `game_db`. The table is found by its own first rows;
/// rows are 181 bytes unless an inline-name flag adds a counted UTF-8 name.
#[derive(Debug, Clone, Copy)]
pub struct StadiumLayout;

impl StadiumLayout {
    pub const ROW_BYTES: usize = 181;
    pub const ORDINAL_OFFSET: usize = 0;
    pub const UID_OFFSET: usize = 4;
    pub const UID_COPY_OFFSET: usize = 8;
    pub const ZERO_BYTE_OFFSET: usize = 12;
    pub const ALL_SEATER_OFFSET: usize = 13;
    pub const EXPANSION_OFFSET: usize = 21;
    pub const OWNER_OFFSET: usize = 29;
    pub const CAPACITY_OFFSET: usize = 34;
    pub const PITCH_LENGTH_OFFSET: usize = 38;
    pub const PITCH_WIDTH_OFFSET: usize = 40;
    pub const BUILT_OFFSET: usize = 50;
    pub const REBUILT_OFFSET: usize = 54;
    pub const PITCH_MIN_LENGTH_OFFSET: usize = 67;
    pub const PITCH_MIN_WIDTH_OFFSET: usize = 69;
    pub const PITCH_MAX_LENGTH_OFFSET: usize = 71;
    pub const PITCH_MAX_WIDTH_OFFSET: usize = 73;
    pub const FLAGS_OFFSET: usize = 156;
    pub const INLINE_NAME_FLAG: u8 = 0x10;
    pub const NAME_LENGTH_OFFSET: usize = 158;
    pub const NAME_OFFSET: usize = 162;
    pub const NAMED_ROW_EXTRA_BYTES: usize = 4;
    pub const NAME_LENGTH_RANGE: (u32, u32) = (1, 256);
    pub const LOCATOR_ROWS: usize = 12;
    pub const PITCH_LENGTH_RANGE: (u16, u16) = (900, 1_300);
    pub const CLOSING_MARKER_OFFSET: usize = 12;
    pub const CLOSING_MARKER: [u8; 5] = [0, 0, 0, 0, 1];
    pub const CLOSING_UID_OFFSET: usize = 17;
}

#[derive(Debug, Clone, Copy)]
pub struct ContractLayout;

impl ContractLayout {
    pub const TAG: &'static [u8] = &[0x01, 0x00, 0x6c, 0x07];
    pub const CHAIN_WINDOW_START_OFFSET: i32 = -30;
    pub const CHAIN_WINDOW_END_OFFSET: i32 = -30;
    pub const SELECTOR_OFFSET: usize = 5;
    pub const START_OFFSET: i32 = -19;
    pub const TEAM_ID_OFFSET: usize = 9;
    pub const WAGE_OFFSET: usize = 17;
    pub const TAIL_BASE_OFFSET: usize = 66;
    pub const TAIL_STEP_BYTES: usize = 29;
    pub const TAIL_MAX_EVENT_COUNT: usize = 200;
    pub const TAIL_SENTINEL_BYTE_OFFSET: usize = 3;
    pub const TAIL_SENTINEL_BYTE_VALUE: u8 = 3;
    pub const TAIL_SENTINEL_WORD_OFFSET: usize = 4;
    pub const TAIL_SENTINEL_WORD_VALUE: u32 = 4;
    pub const TAIL_END_OFFSET: usize = 28;
    pub const TAIL_PRINTED_START_OFFSET: usize = 32;
    pub const TAIL_SQUAD_STATUS_OFFSET: usize = 36;
    /// Bit 0 is transfer listed, bit 1 is loan listed. Other bits are game flags
    /// (for example 0x40 on players the club will not sell).
    pub const TAIL_LIST_FLAGS_OFFSET: usize = 39;
    /// A status byte that is often `5`, but also on players who are not listed,
    /// including key players. It is not a transfer-listing indicator.
    pub const TAIL_STATUS_CODE_OFFSET: usize = 40;
    pub const TAIL_E24_OFFSET: usize = 24;
    pub const TAIL_EVENT_COUNT_OFFSET: usize = 42;
    pub const TAIL_STRUCT_END: usize = 46;
    pub const LOAN_BLOCK_MAX_EVENT_COUNT: usize = 60;
    pub const CLAUSE_STEP_BYTES: usize = 8;
    pub const CLAUSE_MAX_COUNT: usize = 23;
    pub const CLAUSE_FF_OFFSET: i32 = -16;
    pub const CLAUSE_FF_COUNT: usize = 8;
    pub const CLAUSE_ZERO_OFFSET: i32 = -8;
    pub const CLAUSE_ZERO_COUNT: usize = 3;
    pub const CLAUSE_COUNT_OFFSET: i32 = -5;
    pub const CLAUSE_ENTRIES_OFFSET: i32 = -4;
    pub const CLAUSE_ENTRY_BYTES: usize = 8;
    pub const CLAUSE_COMPETITION_COUNT_BYTES: usize = 1;
    pub const CLAUSE_COMPETITION_ITEM_BYTES: usize = 31;
    pub const CLAUSE_COMPETITION_ITEM_PREFIX: &'static [u8] = &[0x01, 0x00, 0x01, 0x00, 0x6c, 0x07];
    pub const CLAUSE_COMPETITION_MAX_COUNT: usize = 32;
    pub const CLAUSE_AWARD_FLAG_BYTES: usize = 1;
    pub const CLAUSE_AWARD_COUNT_BYTES: usize = 4;
    pub const CLAUSE_AWARD_ITEM_BYTES: usize = 15;
    pub const CLAUSE_AWARD_ITEM_PREFIX: &'static [u8] = &[0x01, 0x00];
    pub const CLAUSE_AWARD_MAX_COUNT: usize = 32;
    pub const CLAUSE_TRAILER_BYTES: usize = 2;
    pub const CLAUSE_TEAM_MARKER_ID_BYTES: usize = 4;
    pub const HEAD_GATE_OFFSET: i32 = -35;
    pub const HEAD_GATE_VALUE: u16 = 5;
    pub const HEAD_TYPE_OFFSET: i32 = -33;
    pub const FALLBACK_START_FROM_RECORD: usize = 110;
    pub const FALLBACK_END_MARGIN: usize = 90;
    pub const FALLBACK_NONZERO_OFFSET: usize = 8;
    pub const FALLBACK_NONZERO_LENGTH: usize = 8;
    pub const FALLBACK_GATE_LENGTH: usize = 16;
}

/// The `humans` section names the save's human managers. The first selector is the
/// manager's person id plus one; the matching person header in `game_db` stores that
/// person id followed by the doubled uid.
#[derive(Debug, Clone, Copy)]
pub struct HumansLayout;

impl HumansLayout {
    pub const HUMAN_COUNT_OFFSET: usize = 8;
    pub const FIRST_SELECTOR_OFFSET: usize = 10;
    pub const PERSON_UID_OFFSET: usize = 4;
    pub const PERSON_UID_COPY_OFFSET: usize = 8;
}

/// The readable strings in `save_game_summary`, used as the independent check for
/// the managed club. A string is a u32 byte length followed by that many UTF-8 bytes;
/// every code point is at least a space.
#[derive(Debug, Clone, Copy)]
pub struct SummaryStringsLayout;

impl SummaryStringsLayout {
    pub const STRINGS_START_OFFSET: usize = 8;
    pub const MIN_LENGTH: usize = 2;
    pub const MAX_LENGTH: usize = 120;
    pub const LOWEST_CODE_POINT: char = '\u{20}';
}

/// FM27 staff objects. The header is a person id followed by the doubled uid, the
/// object-kind byte and the entry count; the ability block sits after the entries and
/// three reputation words. Profile attributes are split between the preference area
/// and the 26-byte block at the end of the ability block.
#[derive(Debug, Clone, Copy)]
pub struct StaffLayout;

impl StaffLayout {
    pub const KIND_OFFSET: usize = 12;
    pub const STAFF_KIND: u8 = 1;
    pub const HUMAN_KIND: u8 = 9;
    pub const UID_OFFSET: usize = 4;
    pub const UID_COPY_OFFSET: usize = 8;
    pub const ENTRY_COUNT_OFFSET: usize = 13;
    pub const ENTRY_BYTES: usize = 7;
    pub const ABILITY_BASE_OFFSET: usize = 21;
    pub const CURRENT_ABILITY_OFFSET: usize = 0;
    pub const POTENTIAL_ABILITY_OFFSET: usize = 2;
    /// The ability block's r4 byte. It is not the displayed job.
    pub const ABILITY_R4_OFFSET: usize = 4;
    pub const SENTINEL_OFFSET: usize = 13;
    pub const SENTINEL_VALUE: u8 = 12;
    pub const PREFERENCES_OFFSET: usize = 14;
    pub const PREFERENCE_COUNT: usize = 26;
    pub const PREFERENCE_AUTHORITY_OFFSET: usize = 4;
    pub const PREFERENCE_YOUNGSTERS_OFFSET: usize = 12;
    pub const PREFERENCE_DETERMINATION_OFFSET: usize = 13;
    pub const BLOCK_40_OFFSET: usize = 40;
    pub const BLOCK_40_COUNT: usize = 26;
    /// (attribute key index, block-40 slot) for the attributes outside the
    /// preference area. Determination, authority and working with youngsters are
    /// read from the preference area instead.
    pub const BLOCK_40_ATTRIBUTE_SLOTS: [(usize, usize); 18] = [
        (3, 1),
        (4, 2),
        (5, 3),
        (6, 4),
        (7, 5),
        (8, 6),
        (9, 7),
        (10, 8),
        (11, 9),
        (12, 10),
        (13, 11),
        (14, 12),
        (15, 13),
        (16, 18),
        (17, 21),
        (18, 23),
        (19, 24),
        (20, 25),
    ];
    pub const ABILITY_RANGE: (u16, u16) = (1, 200);
    pub const BLOCK_40_RANGE: (u8, u8) = (1, 100);
    pub const ALLOCATED_UID_MINIMUM: u32 = 2_000_000_000;
    /// FM27 reads the job code this many bytes past a contract record's selector.
    pub const JOB_OFFSET_FROM_SELECTOR: usize = 16;
    /// A contract offer's second tag follows the selector by this many bytes.
    pub const OFFER_MARKER_OFFSET: usize = 19;
    pub const CONTRACT_SEARCH_BYTES: usize = 32_768;
    /// A contracted person's name block starts this many bytes past his latest tag.
    pub const PERSON_BLOCK_OFFSET_AFTER_TAG: usize = 60;
    pub const PERSON_SEARCH_BYTES: usize = 4_096;
    pub const LIST_PERSON_SEARCH_BYTES: usize = 20_000;
    /// A person who holds a club role with no contract has a tag, a 05 05 lead,
    /// his selector, his team id and then his job code.
    pub const NO_CONTRACT_SELECTOR_OFFSET: usize = 16;
    pub const NO_CONTRACT_LEAD: [u8; 2] = [0x05, 0x05];
    pub const NO_CONTRACT_TEAM_OFFSET: usize = 20;
    pub const NO_CONTRACT_JOB_OFFSET: usize = 32;
    /// Fifteen role-suitability bytes follow a 0x01 byte and precede four zero bytes.
    pub const ROLE_VECTOR_BYTES: usize = 15;
    pub const ROLE_VECTOR_PRECEDING: u8 = 1;
    pub const ROLE_SCORE_MIN: u8 = 1;
    pub const ROLE_SCORE_MAX: u8 = 20;
    pub const ROLE_GREEN_THRESHOLD: u8 = 10;
}

pub const STAFF_ATTRIBUTE_KEYS: [&str; 21] = [
    "authority",
    "working_with_youngsters",
    "determination",
    "goalkeeping",
    "judging_player_ability",
    "judging_player_potential",
    "people_management",
    "motivating",
    "physiotherapy",
    "tactical_knowledge",
    "attacking",
    "defending",
    "fitness",
    "possession",
    "technical",
    "tactical",
    "data_analysis",
    "sports_science",
    "negotiating",
    "judging_staff_ability",
    "set_pieces",
];

pub const STAFF_ROLE_KEYS: [&str; 15] = [
    "manager",
    "assistant_manager",
    "coach",
    "physio",
    "scout",
    "gk_coach",
    "fitness_coach",
    "chairperson",
    "director_of_football",
    "head_of_youth_development",
    "loan_manager",
    "technical_director",
    "data_analyst",
    "sports_scientist",
    "set_piece_coach",
];

pub const STAFF_ROLE_NAMES: [&str; 15] = [
    "Manager",
    "Assistant Manager",
    "Coach",
    "Physio",
    "Scout",
    "Goalkeeping Coach",
    "Fitness Coach",
    "Chairperson",
    "Director of Football",
    "Head of Youth Development",
    "Loan Manager",
    "Technical Director",
    "Data Analyst",
    "Sports Scientist",
    "Set Piece Coach",
];

/// The job codes FM27 labels in the staff screen. Unconfirmed codes return `None`
/// instead of inventing a title.
pub fn staff_job_name(code: u8) -> Option<&'static str> {
    Some(match code {
        2 => "Coach",
        4 => "Chairperson",
        6 => "Director",
        8 => "Managing Director",
        10 => "Director of Football",
        12 => "Physio",
        14 => "Scout",
        16 => "Manager",
        20 => "Assistant Manager",
        22 => "Set Piece Coach",
        26 => "Fitness Coach",
        34 => "Goalkeeping Coach",
        38 => "Chief Doctor",
        40 => "Head of Sports Science",
        48 => "Sports Scientist",
        50 => "Head Physio",
        58 => "Data Analyst",
        60 => "Performance Analyst",
        62 => "Data Analyst",
        64 => "Head of Youth Development",
        70 => "President",
        86 => "Loan Manager",
        88 => "Technical Director",
        _ => return None,
    })
}

pub fn staff_role_name(index: u8) -> Option<&'static str> {
    STAFF_ROLE_NAMES.get(usize::from(index)).copied()
}

pub const NAME_POOL_SIGNATURE: [u8; 24] = {
    let mut sig = [0u8; 24];
    let words: [u32; 6] = [46421, 0, 1024, 256, 2048, 0];
    let mut i = 0;
    while i < 6 {
        let bytes = words[i].to_le_bytes();
        sig[i * 4] = bytes[0];
        sig[i * 4 + 1] = bytes[1];
        sig[i * 4 + 2] = bytes[2];
        sig[i * 4 + 3] = bytes[3];
        i += 1;
    }
    sig
};
pub const NAME_POOL_PREFIX: [u8; 5] = [0x55, 0xb5, 0x00, 0x00, 0x00]; // 46421 = 0xB555

/// Decode a stored game date. `offset` points at the low 16-bit day/slot word.
pub fn decode_date(bytes: &[u8], offset: usize) -> Option<NaiveDate> {
    let packed = u16::from_le_bytes(*read_slice::<2>(bytes, offset)?);
    let year = u16::from_le_bytes(*read_slice::<2>(bytes, offset + 2)?) as i32;
    let day_of_year = (packed & 0x1FF) as u32;
    if !(EARLIEST_YEAR..=LATEST_YEAR).contains(&year) {
        return None;
    }
    NaiveDate::from_yo_opt(year, day_of_year)
}

/// Encode a game date, preserving any flags stored in the high bits of the
/// existing low word. The low 9 bits are the day of the year (1..=366); the
/// next word is the year.
pub fn encode_date(date: NaiveDate, existing_packed: u16) -> Option<[u8; 4]> {
    let year = u16::try_from(date.year()).ok()?;
    if !(EARLIEST_YEAR..=LATEST_YEAR).contains(&(year as i32)) {
        return None;
    }
    let packed = (existing_packed & !0x1FF) | date.ordinal() as u16;
    let mut bytes = [0u8; 4];
    bytes[..2].copy_from_slice(&packed.to_le_bytes());
    bytes[2..].copy_from_slice(&year.to_le_bytes());
    Some(bytes)
}

pub fn fmt_date(date: Option<NaiveDate>) -> Option<String> {
    date.map(|value| value.format("%Y-%m-%d").to_string())
}

pub fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(*read_slice::<2>(bytes, offset)?))
}

pub fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(*read_slice::<4>(bytes, offset)?))
}

pub fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_le_bytes(*read_slice::<2>(bytes, offset)?))
}

pub fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(*read_slice::<4>(bytes, offset)?))
}

pub fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(*read_slice::<8>(bytes, offset)?))
}

pub fn read_u8(bytes: &[u8], offset: usize) -> Option<u8> {
    bytes.get(offset).copied()
}

fn read_slice<const N: usize>(bytes: &[u8], offset: usize) -> Option<&[u8; N]> {
    bytes.get(offset..offset + N)?.try_into().ok()
}

pub fn scale_attribute(raw: u8) -> u8 {
    std::cmp::max(1, (raw as u16 + 2) / 5) as u8
}

/// Map an attribute key to its byte index inside the stored 54-byte player slice.
/// The two foot bytes sit at indices 24 and 25, between `technique` and `flair`.
pub fn stored_attribute_index(key: &str) -> Option<usize> {
    ATTRIBUTE_KEYS
        .iter()
        .position(|name| *name == key)
        .map(|index| {
            if index < PlayerLayout::LEFT_FOOT_INDEX {
                index
            } else {
                index + 2
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_accepts_only_verified_builds() {
        assert!(is_editable_build(BUILD));
        assert!(is_editable_build("27.0.0+2486695"));
        assert!(is_editable_build("27.0.0+2493426"));
        assert!(is_editable_build("27.0.1+2494687"));
        assert!(is_editable_build("27.0.1+2503222"));
        assert!(!is_editable_build("27.0.0+9999999"));
        assert!(!is_editable_build("27.0.1+2493426"));
        assert!(!is_editable_build("27.0.1+2494688"));
        assert!(!is_editable_build("27.0.1+2503223"));
        assert_eq!(
            editable_builds(),
            "27.0.0+2486695, 27.0.0+2493426, 27.0.1+2494687, 27.0.1+2503222"
        );
    }

    #[test]
    fn encodes_and_decodes_game_dates() {
        for date in [
            NaiveDate::from_ymd_opt(2027, 6, 30).unwrap(),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        ] {
            let bytes = encode_date(date, 0).expect("date in range");
            assert_eq!(decode_date(&bytes, 0), Some(date));
        }
    }

    #[test]
    fn encoding_preserves_high_date_bits() {
        let date = NaiveDate::from_ymd_opt(2029, 6, 30).unwrap();
        let bytes = encode_date(date, 0xFE00).expect("date in range");
        let packed = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(packed & !0x1FF, 0xFE00);
        assert_eq!(packed & 0x1FF, date.ordinal() as u16);
        assert_eq!(decode_date(&bytes, 0), Some(date));
    }

    #[test]
    fn rejects_dates_outside_the_game_range() {
        assert!(encode_date(NaiveDate::from_ymd_opt(1900, 1, 2).unwrap(), 0).is_none());
        assert!(encode_date(NaiveDate::from_ymd_opt(2201, 1, 1).unwrap(), 0).is_none());
    }
}
