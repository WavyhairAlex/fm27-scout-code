//! FM27 27.0.0+2486695 layout constants, ported from the verified fmsave reference
//! implementation and checked against a real career save.

use chrono::NaiveDate;

pub const GAME: &str = "FM27";
pub const BUILD: &str = "27.0.0+2486695";
pub const GAME_DB_SCHEMA: u32 = 4000;
/// Cache/parser revision. The suffix invalidates caches written before the
/// complete player-trait and nation-name tables landed, and before the current
/// injury windows were read from the stride-14 arrays.
pub const PARSER_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+nation-names+listing+injury-windows"
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

/// Attributes that do not consume the CA budget. Determination and natural
/// fitness stayed unchanged in controlled CA rebalancing tests; hidden attributes
/// are exempt too.
pub fn is_ca_exempt_attribute(key: &str) -> bool {
    is_hidden_attribute_key(key) || matches!(key, "determination" | "natural_fitness")
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
    /// `5` when the club has placed the player on the transfer list.
    pub const TAIL_LIST_CODE_OFFSET: usize = 40;
    pub const LIST_CODE_TRANSFER: u8 = 5;
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
