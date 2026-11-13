import type {
  Attributes,
  Club,
  ClubDetail,
  ClubFilterOptions,
  ClubPage,
  ClubQuery,
  ClubRow,
  DistributionBand,
  FilterOptions,
  FinanceMonth,
  ManagedClub,
  Player,
  PlayerQuery,
  PlayerRow,
  Personality,
  Positions,
  RoleScore,
  SaveRecord,
  ScoutDeparture,
  ScoutDepartureAction,
  ScoutDepthPlayer,
  ScoutFitting,
  ScoutLine,
  ScoutOverview,
  ScoutPlayer,
  ScoutRequest,
  ScoutRoleInfo,
  ScoutSuggestion,
  Scouted,
  SnapshotPoint,
  SaveSummary,
  SquadAnalysis,
  SquadTeam,
  Stadium,
  StaffFilterOptions,
  StaffAttributes,
  StaffPage,
  StaffQuery,
  StaffRow,
  StatsDriver,
  StatsMoneyballPage,
  StatsMoneyballRequest,
  SimilarStatsPlayer,
  SquadNeed,
  SquadNeeds,
  SquadNeedsRequest,
  SquadUpgrade,
  StatsMoneyballRow,
  Trait,
  TraitOption,
  ValueFinderPage,
  ValueFinderRequest,
  ValueFinderRow,
} from "./types";
import { STAFF_ATTRIBUTE_KEYS, STAFF_ROLE_KEYS, UNKNOWN_GENDER_ID } from "./types";

export const MOCK_SAVE_ID = 1;

const FIRST_NAMES = [
  "Luka", "Mateo", "Andrea", "Nico", "Rafael", "Tommaso", "Elias", "Jonas",
  "Noah", "Liam", "Oscar", "Hugo", "Theo", "Lucas", "Milan", "Ivan",
  "Sami", "Youssef", "Karim", "Amine", "Kenji", "Min-jun", "Diego", "Pablo",
  "Facundo", "Santiago", "Bruno", "Pedro", "João", "Rúben", "Kai", "Finn",
];
const LAST_NAMES = [
  "Modrić", "Barella", "Pavard", "Ramos", "Silva", "Costa", "Nielsen", "Berg",
  "Müller", "Schmidt", "Rossi", "Ricci", "Dubois", "Moreau", "Santos", "Ferreira",
  "Kovač", "Petrović", "Ali", "Hassan", "Tanaka", "Kim", "García", "López",
  "Fernández", "Romero", "Almeida", "Carvalho", "Nakamura", "Sørensen", "Okafor", "Mensah",
];
const CLUBS: [string, number, number][] = [
  ["Casciavit Milano", 1099, 150],
  ["Barcelona", 1708, 173],
  ["Manchester United", 728, 139],
  ["Tottenham Hotspur", 729, 139],
  ["Juventus", 1139, 164],
  ["Inter", 1107, 164],
  ["Bayern München", 1852, 164],
  ["Paris Saint-Germain", 1063, 168],
  ["Rangers", 1570, 181],
  ["RSC Anderlecht", 256, 138],
  ["Ajax", 992, 156],
  ["Benfica", 1610, 177],
  ["River Plate", 1745, 11],
  ["Flamengo", 1820, 23],
];
const NATIONS = [135, 150, 164, 139, 173, 168, 181, 138, 156, 177, 11, 23, 70, 216, 126];
const NATION_NAMES: Record<number, string> = {
  11: "巴西",
  23: "阿根廷",
  70: "克罗地亚",
  126: "意大利",
  135: "英格兰",
  138: "西班牙",
  139: "苏格兰",
  150: "德国",
  156: "法国",
  164: "葡萄牙",
  168: "荷兰",
  173: "比利时",
  177: "丹麦",
  181: "瑞典",
  216: "乌拉圭",
};
const POSITION_SETS: [string, string][] = [
  ["MC", "DM"], ["DC", ""], ["STC", ""], ["AML", "AMC"], ["DR", "WBR"],
  ["DL", "WBL"], ["GK", ""], ["AMC", "MC"], ["MR", "AMR"], ["DM", "MC"],
];
const TRAITS: Trait[] = [
  { bit: 8, name: "Shoots From Distance" },
  { bit: 19, name: "Comes Deep To Get Ball" },
  { bit: 22, name: "Dictates Tempo" },
  { bit: 34, name: "Avoids Using Weaker Foot" },
  { bit: 51, name: "Runs With Ball Often" },
];
export const MOCK_TRAIT_OPTIONS: TraitOption[] = TRAITS.map((trait) => ({
  bit: trait.bit,
  name: trait.name ?? `Trait ${trait.bit}`,
}));

const ATTR_KEYS = Object.keys({
  crossing: 1, dribbling: 1, finishing: 1, heading: 1, long_shots: 1, marking: 1,
  off_the_ball: 1, passing: 1, penalty_taking: 1, tackling: 1, vision: 1, handling: 1,
  aerial_reach: 1, command_of_area: 1, communication: 1, kicking: 1, throwing: 1,
  anticipation: 1, decisions: 1, one_on_ones: 1, positioning: 1, reflexes: 1,
  first_touch: 1, technique: 1, flair: 1, corners: 1, teamwork: 1, work_rate: 1,
  long_throws: 1, eccentricity: 1, rushing_out: 1, punching: 1, acceleration: 1,
  free_kick_taking: 1, strength: 1, stamina: 1, pace: 1, jumping_reach: 1, leadership: 1,
  dirtiness: 1, balance: 1, bravery: 1, consistency: 1, aggression: 1, agility: 1,
  important_matches: 1, injury_proneness: 1, versatility: 1, natural_fitness: 1,
  determination: 1, composure: 1, concentration: 1,
});

function rng(seed: number) {
  let state = seed >>> 0;
  return () => {
    state = (state * 1664525 + 1013904223) >>> 0;
    return state / 0xffffffff;
  };
}

function genderBucket(gender: number | null): number {
  return gender === 0 || gender === 1 ? gender : UNKNOWN_GENDER_ID;
}

function emptyPositions(): Positions {
  return {
    gk: 1, sw: 1, dl: 1, dc: 1, dr: 1, dm: 1, ml: 1, mc: 1, mr: 1,
    aml: 1, amc: 1, amr: 1, stc: 1, wbl: 1, wbr: 1,
  };
}

function buildPlayer(index: number): Player {
  const random = rng(index + 7);
  // Warm the LCG once: the first value from small seeds clusters low, which
  // would leave the mock save with almost no high-CA players.
  random();
  const first = FIRST_NAMES[index % FIRST_NAMES.length];
  const last = LAST_NAMES[(index * 7 + 3) % LAST_NAMES.length];
  const [club, clubUid, nation] = CLUBS[index % CLUBS.length];
  const secondNation = index % 4 === 0 ? NATIONS[(index + 12) % NATIONS.length] : null;
  const [pos1, pos2] = POSITION_SETS[index % POSITION_SETS.length];
  const isKeeper = pos1 === "GK";
  const ca = index === 0 ? 158 : Math.round(70 + random() * 110);
  const pa = index === 0 ? 180 : Math.max(ca, Math.min(200, ca + Math.round(random() * 35)));
  const age = index === 0 ? 40 : Math.round(16 + random() * 23);
  const positions = emptyPositions();
  const key = pos1.toLowerCase() as keyof Positions;
  positions[key] = 20;
  if (pos2) positions[pos2.toLowerCase() as keyof Positions] = 17 + Math.round(random() * 2);
  const attributes: Attributes = {};
  for (const attr of ATTR_KEYS) {
    const base = isKeeper && !["handling", "aerial_reach", "command_of_area", "communication", "kicking", "throwing", "one_on_ones", "reflexes", "rushing_out", "punching", "eccentricity"].includes(attr)
      ? 2 + Math.round(random() * 8)
      : Math.round(5 + random() * 15);
    attributes[attr] = base;
  }
  if (index === 0) {
    Object.assign(attributes, {
      crossing: 15, dribbling: 16, finishing: 10, heading: 7, long_shots: 13, marking: 7,
      off_the_ball: 15, passing: 18, penalty_taking: 14, tackling: 9, vision: 19,
      first_touch: 19, technique: 19, flair: 15, corners: 16, teamwork: 16, work_rate: 14,
      acceleration: 11, strength: 8, stamina: 9, pace: 11, leadership: 18, dirtiness: 12,
      balance: 14, bravery: 14, consistency: 17, important_matches: 18, injury_proneness: 5,
      versatility: 16, natural_fitness: 16, determination: 17, composure: 18, concentration: 18,
    });
  }
  const raw: Attributes = {};
  for (const attr of ATTR_KEYS) raw[attr] = Math.min(100, attributes[attr] * 5 - 2);
  const contractEnd = index === 0 ? "2027-06-30" : `${2027 + (index % 4)}-06-30`;
  const wage = index === 0 ? 106965 : Math.round(500 + random() * 80000);
  const value = index === 0 ? 4408650 : Math.round(10000 + random() * 60000000);
  const generated = index > 0 && index % 11 === 0;
  return {
    uid: 100000 + index,
    unique_id: generated ? 2_002_000_000 + index : 653054 + index,
    generated,
    name: index === 0 ? "Luka Modrić" : `${first} ${last}`,
    first_name: first,
    last_name: last,
    common_name: null,
    full_name: `${first} ${last}`,
    legal_name: null,
    birth_date: index === 0 ? "1985-09-09" : `${2006 - (index % 20)}-0${1 + (index % 9)}-1${index % 9}`,
    age,
    nation_id: nation,
    nation_name: NATION_NAMES[nation] ?? `国家 #${nation}`,
    gender: index % 5 === 0 ? 1 : 0,
    second_nation_ids: secondNation == null ? [] : [secondNation],
    second_nation_names:
      secondNation == null ? [] : [NATION_NAMES[secondNation] ?? `国家 #${secondNation}`],
    home_grown_nation_ids: [nation],
    home_grown_club_uids: [clubUid],
    home_grown_club_names: [club],
    height_cm: index === 0 ? 172 : Math.round(165 + random() * 30),
    ca,
    pa,
    pa_range_code: null,
    reputation: { bucket: 80 + (index % 90), home: 3000, current: 2500, world: 1500 },
    club_uid: clubUid,
    club_name: club,
    club_short_name: club.split(" ")[0],
    club_nation_id: nation,
    club_nation_name: NATION_NAMES[nation] ?? `国家 #${nation}`,
    club_fa_nation_id: nation,
    club_reputation: 7000,
    club_last_league_position: 1 + (index % 18),
    league_competition_id: 22,
    league_name: "Italian Serie A",
    team_id: 800 + index,
    // Three mock squads so the Overview grouping has something to show:
    // first team, then two youth sides by age.
    team_slot: age >= 22 ? 0 : age >= 19 ? 1 : 2,
    club_join_date: "2025-07-14",
    natural_positions: pos2 ? [pos1, pos2] : [pos1],
    accomplished_positions: [],
    personality: {
      adaptability: 10 + (index % 10),
      ambition: 8 + (index % 12),
      loyalty: 9 + (index % 11),
      pressure: 7 + (index % 13),
      professionalism: index === 0 ? 20 : 8 + (index % 12),
      sportsmanship: 7 + (index % 13),
      temperament: 8 + (index % 12),
      controversy: index % 12,
    },
    attributes,
    raw_attributes: raw,
    left_foot: 5 + (index % 15),
    right_foot: 20,
    raw_left_foot: 25,
    raw_right_foot: 100,
    positions,
    transfer_value: value,
    transfer_value_state: "ok",
    transfer_listed: index % 23 === 0,
    loan_listed: index % 29 === 0,
    raw_condition: 7000,
    raw_match_sharpness: 7500,
    traits: index % 4 === 0 ? TRAITS : [],
    trait_bits: null,
    best_role: {
      id: "central_midfielder",
      name: "中场（策应）",
      group: "中场",
      score: Math.max(1, Math.min(20, Math.round(ca / 9))),
      position: pos1,
      position_rating: 20,
    },
    injuries:
      index % 37 === 0
        ? [
            {
              kind: "history",
              date: "2026-03-14",
              expected_return: null,
              type_id: null,
              type_name: null,
              cause: 0,
              severity: 2,
              team_id: 800 + index,
              club_uid: clubUid,
              club_name: club,
            },
          ]
        : [],
    suspensions:
      index % 53 === 0
        ? [
            {
              scope: "nation",
              scope_code: 10,
              competition_id: null,
              competition_name: null,
              nation_id: nation,
              nation_name: NATION_NAMES[nation] ?? null,
              issued_date: "2026-07-13",
            },
          ]
        : [],
    on_loan: index % 17 === 0,
    loan_parent_club_uid: index % 17 === 0 ? 728 : null,
    loan_parent_club_name: index % 17 === 0 ? "Tottenham Hotspur" : null,
    loan_start: index % 17 === 0 ? "2026-07-13" : null,
    loan_end: index % 17 === 0 ? "2027-06-30" : null,
    contract: {
      club_uid: clubUid,
      club_name: club,
      team_id: 800 + index,
      wage,
      start: "2026-06-30",
      end: contractEnd,
      squad_status: 2,
      contract_type: 1,
      transfer_listed: index % 23 === 0,
      loan_listed: index % 29 === 0,
      clauses: [],
      on_loan: index % 17 === 0,
      loan_parent_club_uid: index % 17 === 0 ? 728 : null,
      loan_parent_club_name: index % 17 === 0 ? "Tottenham Hotspur" : null,
      loan_start: index % 17 === 0 ? "2026-07-13" : null,
      loan_end: index % 17 === 0 ? "2027-06-30" : null,
      chain: [
        {
          club_uid: clubUid,
          club_name: club,
          team_id: 800 + index,
          wage,
          start: "2026-06-30",
          end: contractEnd,
          has_terms: true,
        },
      ],
    },
  } as Player;
}

export const MOCK_PLAYERS: Player[] = Array.from({ length: 320 }, (_, index) => buildPlayer(index));

// One free agent exercises the unknown-gender path: players inherit gender
// from their club, so a player without a club has no resolved gender.
Object.assign(MOCK_PLAYERS[MOCK_PLAYERS.length - 1], {
  gender: null,
  club_uid: null,
  club_name: null,
  club_short_name: null,
  club_nation_id: null,
  club_nation_name: null,
  club_fa_nation_id: null,
  club_reputation: null,
  club_last_league_position: null,
  league_competition_id: null,
  league_name: null,
  team_id: null,
  team_slot: null,
  club_join_date: null,
  on_loan: false,
  loan_parent_club_uid: null,
  loan_parent_club_name: null,
  loan_start: null,
  loan_end: null,
  contract: null,
});

export const MOCK_SUMMARY: SaveSummary = {
  path: "/Users/demo/Library/Application Support/Sports Interactive/Football Manager 27/games/BB KING - Casciavit.fm",
  file_name: "BB KING - Casciavit.fm",
  file_size: 49042131,
  mtime_ns: 1793328187036747886,
  fingerprint: "demo",
  save_name: "BB KING - Casciavit",
  game: "FM27",
  build: "27.0.1+2503222",
  game_date: "2026-07-13",
  db_version: "27.0.0+0",
  section_count: 75,
  decompressed_bytes: 192548055,
};

export function mockManagedClub(): ManagedClub {
  return {
    club_uid: 1099,
    club_name: "Casciavit Milano",
    manager_name: "Demo Manager",
  };
}

export const MOCK_SAVE: SaveRecord = {
  id: MOCK_SAVE_ID,
  path: MOCK_SUMMARY.path,
  file_name: MOCK_SUMMARY.file_name,
  game: MOCK_SUMMARY.game,
  build: MOCK_SUMMARY.build,
  game_date: MOCK_SUMMARY.game_date,
  parser_version: "0.1.0",
  parsed_at: new Date().toISOString(),
  players: MOCK_PLAYERS.length,
  staff: 120,
  clubs: CLUBS.length,
  contracts: MOCK_PLAYERS.length,
};

export function mockPlayerRow(player: Player): PlayerRow {
  const positionsText = player.natural_positions.join(" / ");
  return {
    uid: player.uid,
    generated: player.generated,
    name: player.name,
    age: player.age,
    club_uid: player.club_uid,
    club_name: player.club_name,
    team_id: player.team_id,
    team_slot: player.team_slot,
    league_competition_id: player.league_competition_id,
    league_name: player.league_name,
    nation_id: player.nation_id,
    nation_name: player.nation_name,
    gender: player.gender,
    ca: player.ca,
    pa: player.pa,
    pa_range_code: player.pa_range_code,
    positions_text: positionsText,
    positions: player.positions,
    attributes: player.attributes,
    personality: player.personality,
    transfer_value: player.transfer_value,
    transfer_value_state: player.transfer_value_state,
    wage: player.contract?.wage ?? null,
    contract_end: player.contract?.end ?? null,
    height_cm: player.height_cm,
    left_foot: player.left_foot,
    right_foot: player.right_foot,
    on_loan: player.on_loan,
    transfer_listed: player.transfer_listed,
    loan_listed: player.loan_listed,
    traits: player.traits,
    best_role_id: player.best_role?.id ?? null,
    best_role_name: player.best_role?.name ?? null,
    best_role_score: player.best_role?.score ?? null,
    best_role_position: player.best_role?.position ?? null,
    injury_status: player.injuries.some((injury) => injury.kind === "current")
      ? "injured"
      : "fit",
    injury_until: null,
    recent_injury: player.injuries.find((injury) => injury.date)?.date ?? null,
    suspension_count: player.suspensions.length,
  };
}

export function mockPlayerRoles(uid: number): RoleScore[] {
  const player = MOCK_PLAYERS.find((item) => item.uid === uid) ?? MOCK_PLAYERS[0];
  const top = player.best_role?.score ?? 14;
  return [
    ["goalkeeper", "门将（防守）", "门将", "GK"],
    ["central_defender", "中后卫（防守）", "后卫", "DC"],
    ["wing_back", "翼卫（进攻）", "后卫", "WBL"],
    ["defensive_midfielder", "防守型中场（防守）", "中场", "DM"],
    ["deep_lying_playmaker", "后组织核心（策应）", "中场", "MC"],
    ["box_to_box_midfielder", "全能中场（策应）", "中场", "MC"],
    ["advanced_playmaker", "前组织核心（进攻）", "中场", "AMC"],
    ["winger", "边锋（进攻）", "前场", "AML"],
    ["inside_forward", "内锋（进攻）", "前场", "AMR"],
    ["trequartista", "九号半（进攻）", "前场", "AMC"],
    ["advanced_forward", "突前前锋（进攻）", "前锋", "STC"],
    ["poacher", "抢点前锋（进攻）", "前锋", "STC"],
  ].map(([id, name, group, position], index) => ({
    id,
    name,
    group,
    position,
    score: Math.max(1, Math.min(20, top + ((index * 7) % 9) - 4)),
    position_rating: 12 + ((index * 3) % 9),
  }));
}

export function mockGrowthHistory(uid: number): SnapshotPoint[] {
  const player = MOCK_PLAYERS.find((item) => item.uid === uid) ?? MOCK_PLAYERS[0];
  const base = Math.max(60, player.ca - 12);
  return ["2024-07-01", "2025-01-01", "2025-07-01", "2026-01-01", "2026-07-13"].map(
    (game_date, index) => ({
      game_date,
      ca: Math.min(player.ca, base + index * 3),
      pa: player.pa,
      transfer_value: Math.round((player.transfer_value ?? 0) * (0.6 + index * 0.1)),
      club_uid: player.club_uid,
      club_name: player.club_name,
      wage: player.contract?.wage ?? null,
      contract_end: player.contract?.end ?? null,
    }),
  );
}

export function mockFilterOptions(): FilterOptions {
  const clubCounts = new Map<string, { id: number; name: string; count: number }>();
  for (const player of MOCK_PLAYERS) {
    if (player.club_uid == null || !player.club_name) continue;
    const entry = clubCounts.get(player.club_name) ?? {
      id: player.club_uid,
      name: player.club_name,
      count: 0,
    };
    entry.count += 1;
    clubCounts.set(player.club_name, entry);
  }
  const nationCounts = new Map<number, number>();
  for (const player of MOCK_PLAYERS) {
    if (player.nation_id != null) {
      nationCounts.set(player.nation_id, (nationCounts.get(player.nation_id) ?? 0) + 1);
    }
  }
  return {
    clubs: [...clubCounts.values()].sort((a, b) => a.name.localeCompare(b.name)),
    nations: [...nationCounts.entries()].map(([id, count]) => ({
      id,
      name: NATION_NAMES[id] ?? `国家 #${id}`,
      count,
    })),
    genders: [
      { id: 0, name: "男子", count: MOCK_PLAYERS.filter((player) => player.gender === 0).length },
      { id: 1, name: "女子", count: MOCK_PLAYERS.filter((player) => player.gender === 1).length },
      {
        id: UNKNOWN_GENDER_ID,
        name: "未知",
        count: MOCK_PLAYERS.filter((player) => genderBucket(player.gender) === UNKNOWN_GENDER_ID)
          .length,
      },
    ],
    availability: [
      { id: 1, name: "伤病", count: 0 },
      { id: 2, name: "停赛", count: MOCK_PLAYERS.filter((player) => player.suspensions.length > 0).length },
      { id: 3, name: "租借", count: MOCK_PLAYERS.filter((player) => player.on_loan).length },
      { id: 4, name: "自由球员", count: 0 },
      { id: 5, name: "可出战", count: MOCK_PLAYERS.length },
      {
        id: 6,
        name: "伤病史",
        count: MOCK_PLAYERS.filter((player) => player.injuries.length > 0).length,
      },
    ],
    listing: [
      {
        id: 1,
        name: "挂牌出售",
        count: MOCK_PLAYERS.filter((player) => player.transfer_listed).length,
      },
      {
        id: 2,
        name: "挂牌出租",
        count: MOCK_PLAYERS.filter((player) => player.loan_listed).length,
      },
    ],
    min_age: 16,
    max_age: 40,
    min_ca: 70,
    max_ca: 180,
    min_pa: 70,
    max_pa: 200,
    min_value: 10000,
    max_value: 60000000,
    min_wage: 500,
    max_wage: 200000,
  };
}

export function mockQuery(query: PlayerQuery): {
  total: number;
  page: number;
  page_size: number;
  rows: PlayerRow[];
} {
  let rows = MOCK_PLAYERS.map(mockPlayerRow);
  const search = query.search?.trim().toLowerCase();
  if (search) {
    rows = rows.filter(
      (row) =>
        row.name?.toLowerCase().includes(search) ||
        row.club_name?.toLowerCase().includes(search) ||
        row.nation_name?.toLowerCase().includes(search),
    );
  }
  if (query.uids?.length) rows = rows.filter((row) => query.uids!.includes(row.uid));
  if (query.club_uids?.length) rows = rows.filter((row) => query.club_uids!.includes(row.club_uid ?? -1));
  if (query.nation_ids?.length) rows = rows.filter((row) => query.nation_ids!.includes(row.nation_id ?? -1));
  if (query.genders?.length)
    rows = rows.filter((row) => query.genders!.includes(genderBucket(row.gender)));
  if (query.availability === "injured") rows = rows.filter((row) => row.injury_status === "injured");
  if (query.availability === "injury_history") rows = rows.filter((row) => row.recent_injury != null);
  if (query.availability === "suspended") rows = rows.filter((row) => row.suspension_count > 0);
  if (query.availability === "loan") rows = rows.filter((row) => row.on_loan === true);
  if (query.availability === "free") rows = rows.filter((row) => row.club_uid == null);
  if (query.availability === "available") {
    rows = rows.filter((row) => row.injury_status !== "injured" && row.suspension_count === 0);
  }
  if (query.listings?.length) {
    rows = rows.filter((row) =>
      query.listings!.some((listing) =>
        listing === "sale" ? row.transfer_listed : row.loan_listed,
      ),
    );
  }
  if (query.age_min != null) rows = rows.filter((row) => (row.age ?? 0) >= query.age_min!);
  if (query.age_max != null) rows = rows.filter((row) => (row.age ?? 999) <= query.age_max!);
  if (query.ca_min != null) rows = rows.filter((row) => row.ca >= query.ca_min!);
  if (query.ca_max != null) rows = rows.filter((row) => row.ca <= query.ca_max!);
  if (query.pa_min != null) rows = rows.filter((row) => (row.pa ?? 0) >= query.pa_min!);
  if (query.pa_max != null) rows = rows.filter((row) => (row.pa ?? 999) <= query.pa_max!);
  if (query.pa_ca_gap_min != null) {
    rows = rows.filter((row) => row.pa != null && row.pa - row.ca >= query.pa_ca_gap_min!);
  }
  if (query.generated === true) rows = rows.filter((row) => row.generated);
  if (query.generated === false) rows = rows.filter((row) => !row.generated);
  if (query.positions?.length) {
    const threshold = query.position_min ?? 15;
    rows = rows.filter((row) =>
      query.positions!.some((key) => (row.positions as unknown as Record<string, number>)[key] >= threshold),
    );
  }
  for (const filter of query.attributes ?? []) {
    rows = rows.filter((row) => {
      const value =
        filter.key in row.attributes
          ? row.attributes[filter.key]
          : (row.personality?.[filter.key as keyof Personality] ?? 0);
      return (filter.min == null || value >= filter.min) && (filter.max == null || value <= filter.max);
    });
  }
  if (query.value_min != null) rows = rows.filter((row) => (row.transfer_value ?? 0) >= query.value_min!);
  if (query.value_max != null) rows = rows.filter((row) => (row.transfer_value ?? 0) <= query.value_max!);
  if (query.wage_min != null) rows = rows.filter((row) => (row.wage ?? 0) >= query.wage_min!);
  if (query.wage_max != null) rows = rows.filter((row) => (row.wage ?? 0) <= query.wage_max!);
  if (query.on_loan != null) rows = rows.filter((row) => row.on_loan === query.on_loan);
  const rules = query.sort?.length
    ? query.sort
    : [{ by: query.sort_by ?? "ca", dir: query.sort_dir ?? "desc" }];
  rows.sort((a, b) => {
    for (const rule of rules) {
      const direction = rule.dir === "asc" ? 1 : -1;
      const read = (row: PlayerRow): number | string | null => {
        switch (rule.by) {
          case "name": return row.name;
          case "age": return row.age;
          case "club": return row.club_name;
          case "nation_name": return row.nation_name;
          case "gender": return row.gender;
          case "value": return row.transfer_value;
          case "wage": return row.wage;
          case "contract_end": return row.contract_end;
          case "pa": return row.pa;
          case "role": return row.best_role_score;
          case "best_role": return row.best_role_name;
          default: return row.ca;
        }
      };
      const left = read(a);
      const right = read(b);
      if (left == null && right == null) continue;
      if (left == null) return 1;
      if (right == null) return -1;
      const compared = (left > right ? 1 : left < right ? -1 : 0) * direction;
      if (compared !== 0) return compared;
    }
    return 0;
  });
  const total = rows.length;
  const page = query.page ?? 0;
  const pageSize = query.page_size ?? 150;
  return {
    total,
    page,
    page_size: pageSize,
    rows: rows.slice(page * pageSize, (page + 1) * pageSize),
  };
}


const STAFF_JOBS: [number, string][] = [
  [16, "Manager"],
  [20, "Assistant Manager"],
  [2, "Coach"],
  [12, "Physio"],
  [14, "Scout"],
  [34, "Goalkeeping Coach"],
  [26, "Fitness Coach"],
  [4, "Chairperson"],
  [10, "Director of Football"],
  [64, "Head of Youth Development"],
  [86, "Loan Manager"],
  [88, "Technical Director"],
  [58, "Data Analyst"],
  [48, "Sports Scientist"],
  [22, "Set Piece Coach"],
];

function buildStaff(index: number): StaffRow {
  const random = rng(9_001 + index * 37);
  const [jobCode, jobName] = STAFF_JOBS[index % STAFF_JOBS.length];
  const club = CLUBS[Math.floor(index / STAFF_JOBS.length) % CLUBS.length];
  const nationId = NATIONS[index % NATIONS.length];
  const attributes: StaffAttributes = {};
  for (const key of STAFF_ATTRIBUTE_KEYS) {
    attributes[key] = 1 + Math.floor(random() * 20);
  }
  const roleKey = STAFF_ROLE_KEYS[index % STAFF_ROLE_KEYS.length];
  const roles = STAFF_ROLE_KEYS.map((key) =>
    key === roleKey ? 13 + Math.floor(random() * 8) : 1 + Math.floor(random() * 17),
  );
  const bestRoleIndex = roles.indexOf(Math.max(...roles));
  const bestRoleKey = STAFF_ROLE_KEYS[bestRoleIndex];
  const age = 32 + Math.floor(random() * 32);
  const ca = 70 + Math.floor(random() * 90);
  const hasContract = index % 11 !== 0;
  return {
    uid: 500_000 + index,
    unique_id: 700_000 + index,
    name: `${FIRST_NAMES[index % FIRST_NAMES.length]} ${LAST_NAMES[(index * 3) % LAST_NAMES.length]}`,
    age,
    club_uid: club[1],
    club_name: club[0],
    nation_id: nationId,
    nation_name: NATION_NAMES[nationId] ?? `国家 #${nationId}`,
    job_code: jobCode,
    job_name: jobName,
    ca,
    pa: Math.min(200, ca + Math.floor(random() * 40)),
    wage: hasContract ? 800 + Math.floor(random() * 24_000) : null,
    contract_start: hasContract ? "2025-07-01" : null,
    contract_end: hasContract
      ? `${2026 + Math.floor(random() * 4)}-06-30`
      : null,
    has_contract: hasContract,
    attributes,
    roles,
    best_role_index: bestRoleIndex,
    best_role_name: bestRoleKey,
    best_role_score: roles[bestRoleIndex],
    personality: null,
  };
}

export const MOCK_STAFF: StaffRow[] = Array.from({ length: 120 }, (_, index) =>
  buildStaff(index),
);

export function mockQueryStaff(query: StaffQuery): StaffPage {
  let rows = [...MOCK_STAFF];
  const search = query.search?.trim().toLowerCase();
  if (search) {
    rows = rows.filter(
      (row) =>
        row.name?.toLowerCase().includes(search) ||
        row.club_name?.toLowerCase().includes(search) ||
        row.nation_name?.toLowerCase().includes(search) ||
        row.job_name?.toLowerCase().includes(search),
    );
  }
  if (query.uids?.length) rows = rows.filter((row) => query.uids!.includes(row.uid));
  if (query.club_uids?.length) {
    rows = rows.filter((row) => query.club_uids!.includes(row.club_uid ?? -1));
  }
  if (query.nation_ids?.length) {
    rows = rows.filter((row) => query.nation_ids!.includes(row.nation_id ?? -1));
  }
  if (query.job_codes?.length) {
    rows = rows.filter((row) => query.job_codes!.includes(row.job_code ?? -1));
  }
  if (query.age_min != null) rows = rows.filter((row) => (row.age ?? 0) >= query.age_min!);
  if (query.age_max != null) rows = rows.filter((row) => (row.age ?? 999) <= query.age_max!);
  if (query.ca_min != null) rows = rows.filter((row) => (row.ca ?? 0) >= query.ca_min!);
  if (query.ca_max != null) rows = rows.filter((row) => (row.ca ?? 999) <= query.ca_max!);
  if (query.pa_min != null) rows = rows.filter((row) => (row.pa ?? 0) >= query.pa_min!);
  if (query.pa_max != null) rows = rows.filter((row) => (row.pa ?? 999) <= query.pa_max!);
  if (query.wage_min != null) rows = rows.filter((row) => (row.wage ?? 0) >= query.wage_min!);
  if (query.wage_max != null) rows = rows.filter((row) => (row.wage ?? 0) <= query.wage_max!);
  if (query.contract_end_before) {
    rows = rows.filter(
      (row) => row.contract_end != null && row.contract_end <= query.contract_end_before!,
    );
  }
  if (query.contract_end_after) {
    rows = rows.filter(
      (row) => row.contract_end != null && row.contract_end >= query.contract_end_after!,
    );
  }
  if (query.has_contract != null) {
    rows = rows.filter((row) => row.has_contract === query.has_contract);
  }
  for (const rule of query.attributes ?? []) {
    rows = rows.filter((row) => {
      const value = row.attributes?.[rule.key] ?? 0;
      return (
        (rule.min == null || value >= rule.min) &&
        (rule.max == null || value <= rule.max)
      );
    });
  }
  for (const rule of query.roles ?? []) {
    const index = STAFF_ROLE_KEYS.indexOf(rule.key as (typeof STAFF_ROLE_KEYS)[number]);
    if (index < 0) continue;
    rows = rows.filter((row) => {
      const value = row.roles?.[index] ?? 0;
      return (
        (rule.min == null || value >= rule.min) &&
        (rule.max == null || value <= rule.max)
      );
    });
  }
  const rules = query.sort?.length
    ? query.sort
    : [{ by: query.sort_by ?? "ca", dir: query.sort_dir ?? "desc" }];
  rows.sort((a, b) => {
    for (const rule of rules) {
      const direction = rule.dir === "asc" ? 1 : -1;
      const read = (row: StaffRow): number | string | null => {
        switch (rule.by) {
          case "name": return row.name;
          case "age": return row.age;
          case "club": return row.club_name;
          case "nation_name": return row.nation_name;
          case "job": return row.job_code;
          case "job_name": return row.job_name;
          case "best_role": return row.best_role_name;
          case "role": return row.best_role_score;
          case "wage": return row.wage;
          case "contract_end": return row.contract_end;
          case "uid": return row.uid;
          case "pa": return row.pa;
          default: return row.ca;
        }
      };
      const left = read(a);
      const right = read(b);
      if (left == null && right == null) continue;
      if (left == null) return 1;
      if (right == null) return -1;
      const compared = (left > right ? 1 : left < right ? -1 : 0) * direction;
      if (compared !== 0) return compared;
    }
    return a.uid - b.uid;
  });
  const total = rows.length;
  const page = query.page ?? 0;
  const pageSize = query.page_size ?? 100;
  return {
    total,
    page,
    page_size: pageSize,
    rows: rows.slice(page * pageSize, (page + 1) * pageSize),
  };
}

export function mockStaffFilterOptions(): StaffFilterOptions {
  const clubs = new Map<number, { id: number; name: string; count: number }>();
  const nations = new Map<number, { id: number; name: string; count: number }>();
  const jobs = new Map<number, { id: number; name: string; count: number }>();
  for (const row of MOCK_STAFF) {
    if (row.club_uid != null && row.club_name) {
      const entry = clubs.get(row.club_uid) ?? { id: row.club_uid, name: row.club_name, count: 0 };
      entry.count += 1;
      clubs.set(row.club_uid, entry);
    }
    if (row.nation_id != null) {
      const entry = nations.get(row.nation_id) ?? {
        id: row.nation_id,
        name: row.nation_name ?? `国家 #${row.nation_id}`,
        count: 0,
      };
      entry.count += 1;
      nations.set(row.nation_id, entry);
    }
    if (row.job_code != null) {
      const entry = jobs.get(row.job_code) ?? {
        id: row.job_code,
        name: row.job_name ?? `Job #${row.job_code}`,
        count: 0,
      };
      entry.count += 1;
      jobs.set(row.job_code, entry);
    }
  }
  const wages = MOCK_STAFF.map((row) => row.wage).filter((wage): wage is number => wage != null);
  const ages = MOCK_STAFF.map((row) => row.age).filter((age): age is number => age != null);
  const cas = MOCK_STAFF.map((row) => row.ca).filter((ca): ca is number => ca != null);
  const pas = MOCK_STAFF.map((row) => row.pa).filter((pa): pa is number => pa != null);
  return {
    clubs: [...clubs.values()].sort((a, b) => a.name.localeCompare(b.name)),
    nations: [...nations.values()].sort((a, b) => a.name.localeCompare(b.name)),
    jobs: [...jobs.values()].sort((a, b) => a.name.localeCompare(b.name)),
    min_age: Math.min(...ages),
    max_age: Math.max(...ages),
    min_ca: Math.min(...cas),
    max_ca: Math.max(...cas),
    min_pa: Math.min(...pas),
    max_pa: Math.max(...pas),
    min_wage: Math.min(...wages),
    max_wage: Math.max(...wages),
    with_contract: MOCK_STAFF.filter((row) => row.has_contract).length,
    without_contract: MOCK_STAFF.filter((row) => !row.has_contract).length,
    with_attributes: MOCK_STAFF.filter((row) => row.attributes != null).length,
    with_roles: MOCK_STAFF.filter((row) => row.roles != null).length,
  };
}

function mockPositionFilterMatches(filter: string, group: string): boolean {
  if (filter === "MID") return group === "DM" || group === "MC" || group === "WM";
  if (filter === "AM") return group === "WM" || group === "AMC";
  return filter === group;
}

function mockPositionGroup(position: string | null): string {
  switch ((position ?? "").toUpperCase()) {
    case "GK": return "GK";
    case "DC": case "SW": return "DC";
    case "DL": case "DR": case "WBL": case "WBR": return "FB";
    case "DM": return "DM";
    case "MC": return "MC";
    case "ML": case "MR": case "AML": case "AMR": return "WM";
    case "AMC": return "AMC";
    case "STC": case "SC": return "ST";
    default: return "OTHER";
  }
}

export function mockValueFinder(request: ValueFinderRequest): ValueFinderPage {
  const players = MOCK_PLAYERS.map(mockPlayerRow);
  const withCost = players.map((player) => {
    const fee = player.transfer_value_state === "ok" ? player.transfer_value : null;
    const cost = (fee ?? 0) + (player.wage ?? 0) * 52 * 3;
    const rolePct = (player.best_role_score ?? 1) / 20;
    const caPct = player.ca / 200;
    const qualityPct =
      (request.mode ?? "full") === "full" ? 0.6 * rolePct + 0.4 * caPct : rolePct;
    const costPct = cost > 0 ? Math.min(1, Math.log1p(cost / 1000) / 16) : 0;
    const score = 100 * (0.55 * qualityPct + 0.45 * (1 - costPct));
    return { player, fee, cost, qualityPct, costPct, score };
  });
  const filtered = withCost
    .filter(({ player }) => {
      const group = mockPositionGroup(
        player.positions_text?.split(" / ")[0] ?? player.best_role_position,
      );
      if (request.position_group && !mockPositionFilterMatches(request.position_group, group)) return false;
      if (request.league && player.league_name !== request.league) return false;
      if (request.age_min != null && (player.age ?? 0) < request.age_min) return false;
      if (request.age_max != null && (player.age ?? 999) > request.age_max) return false;
      if (request.ca_min != null && player.ca < request.ca_min) return false;
      if (request.ca_max != null && player.ca > request.ca_max) return false;
      if (request.role_min != null && (player.best_role_score ?? 0) < request.role_min) return false;
      if (!request.include_unknown_fee && player.transfer_value_state !== "ok") return false;
      if (request.max_fee != null && player.transfer_value_state === "ok" && (player.transfer_value ?? Infinity) > request.max_fee) return false;
      if (request.max_wage != null && (player.wage ?? Infinity) > request.max_wage) return false;
      if (request.contract_end_before && (player.contract_end ?? "9999") > request.contract_end_before) return false;
      if (request.listings?.length) {
        const matches = request.listings.some((listing) =>
          listing === "sale" ? player.transfer_listed : listing === "loan" ? player.loan_listed : player.club_uid == null,
        );
        if (!matches) return false;
      }
      return true;
    })
    .sort((left, right) => {
      const by = request.sort_by ?? "score";
      const direction = request.sort_dir === "asc" ? 1 : -1;
      const read = (item: typeof left): number | string | null => {
        switch (by) {
          case "ca": return item.player.ca;
          case "age": return item.player.age;
          case "value": case "fee": return item.player.transfer_value;
          case "wage": return item.player.wage;
          case "cost": return item.cost;
          case "quality": return item.qualityPct;
          case "surplus": return item.qualityPct - item.costPct;
          case "name": return item.player.name;
          default: return item.score;
        }
      };
      const a = read(left);
      const b = read(right);
      if (a == null && b == null) return 0;
      if (a == null) return 1;
      if (b == null) return -1;
      return (a > b ? 1 : a < b ? -1 : 0) * direction;
    });
  const page = request.page ?? 0;
  const pageSize = request.page_size ?? 100;
  const rows: ValueFinderRow[] = filtered.slice(page * pageSize, (page + 1) * pageSize).map(({ player, fee, cost, qualityPct, costPct, score }) => ({
    uid: player.uid,
    name: player.name,
    age: player.age,
    club_uid: player.club_uid,
    club_name: player.club_name,
    league_name: player.league_name,
    nation_name: player.nation_name,
    ca: player.ca,
    pa: player.pa,
    role_id: player.best_role_id,
    role_name: player.best_role_name,
    role_score: player.best_role_score,
    role_position: player.best_role_position,
    position_group: mockPositionGroup(player.best_role_position),
    transfer_value: player.transfer_value,
    transfer_value_state: player.transfer_value_state,
    wage: player.wage,
    contract_end: player.contract_end,
    club_reputation: null,
    fee,
    cost,
    fee_known: fee != null,
    wage_known: player.wage != null,
    role_pct: (player.best_role_score ?? 1) / 20,
    ca_pct: player.ca / 200,
    quality_pct: qualityPct,
    cost_pct: costPct,
    surplus: qualityPct - costPct,
    score,
    confidence: fee != null && player.wage != null ? 1 : 0.75,
    league_median_ca: null,
    flags: fee == null ? ["missing_fee"] : [],
    transfer_listed: player.transfer_listed,
    loan_listed: player.loan_listed,
    injury_status: player.injury_status,
    recent_injury: player.recent_injury,
    injury_recent: player.recent_injury != null,
  }));
  return {
    total: filtered.length,
    page,
    page_size: pageSize,
    rows,
    meta: {
      model: "value-finder-v3-mock",
      mode: request.mode ?? "full",
      scored: filtered.length,
      filtered: filtered.length,
      cost_horizon_years: 3,
      leagues: Array.from(new Set(players.map((player) => player.league_name).filter((name): name is string => !!name))).sort(),
    },
  };
}

export function mockSquadNeeds(request: SquadNeedsRequest): SquadNeeds {
  const minMinutes = request.min_minutes ?? 300;
  const rows = mockStatsMoneyball({
    save_id: request.save_id,
    model: request.model ?? "core",
    window: request.window ?? "overall",
    min_minutes: minMinutes,
    page: 0,
    page_size: 500,
  }).rows;
  const groups = ["GK", "DC", "FB", "DM", "MC", "WM", "AMC", "ST"];
  const needs = groups
    .map((group) => {
      const own = rows.filter(
        (row) =>
          row.position_group === group &&
          row.club_uid === request.club_uid &&
          row.score != null &&
          row.minutes >= minMinutes,
      );
      if (own.length === 0) return null;
      const best = own.reduce((left, right) =>
        (right.score ?? 0) > (left.score ?? 0) ? right : left,
      );
      const upgrades = rows
        .filter(
          (row) =>
            row.position_group === group &&
            row.club_uid != null &&
            row.club_uid !== request.club_uid &&
            row.fee_known &&
            row.minutes >= minMinutes &&
            (row.score ?? 0) > (best.score ?? 0),
        )
        .sort(
          (left, right) =>
            (right.bargain_score ?? 0) - (left.bargain_score ?? 0) ||
            (right.score ?? 0) - (left.score ?? 0),
        )
        .slice(0, 3)
        .map<SquadUpgrade>((row) => ({
          uid: row.uid,
          name: row.name,
          age: row.age,
          club_name: row.club_name,
          minutes: row.minutes,
          quality: row.score,
          quality_gain:
            row.score != null && best.score != null ? row.score - best.score : null,
          fee: row.fee,
          fair_price: row.fair_price,
          value_gap_pct: row.value_gap_pct,
          bargain_score: row.bargain_score,
        }));
      return {
        position_group: group,
        best_quality: best.score,
        best_uid: best.uid,
        best_name: best.name,
        upgrades,
      };
    })
    .filter((need): need is NonNullable<typeof need> => need != null)
    .sort((left, right) => (left.best_quality ?? 0) - (right.best_quality ?? 0));
  return {
    club_uid: request.club_uid,
    club_name: rows.find((row) => row.club_uid === request.club_uid)?.club_name ?? null,
    needs,
  };
}

export function mockSimilarStatsPlayers(
  request: StatsMoneyballRequest,
  uid: number,
  limit = 8,
): SimilarStatsPlayer[] {
  const page = mockStatsMoneyball({ ...request, page: 0, page_size: 500 });
  const target = page.rows.find((row) => row.uid === uid);
  if (!target) return [];
  return page.rows
    .filter((row) => row.uid !== uid && row.position_group === target.position_group)
    .map((row) => ({
      uid: row.uid,
      name: row.name,
      age: row.age,
      club_name: row.club_name,
      position_group: row.position_group,
      minutes: row.minutes,
      quality: row.production_tier,
      fee: row.fee,
      fair_price: row.fair_price,
      value_gap_pct: row.value_gap_pct,
      distance:
        Math.abs((row.production_tier ?? 0) - (target.production_tier ?? 0)) / 100,
    }))
    .sort((left, right) => left.distance - right.distance)
    .slice(0, Math.max(1, Math.min(limit, 50)));
}

export function mockStatsMoneyball(request: StatsMoneyballRequest): StatsMoneyballPage {
  const players = MOCK_PLAYERS.map(mockPlayerRow);
  const model = request.model ?? "core";
  const minMinutes = request.min_minutes ?? 900;
  const rows: StatsMoneyballRow[] = players.map((player, index) => {
    const primary = player.positions_text?.split(" / ")[0] ?? player.best_role_position;
    const group = mockPositionGroup(primary);
    const minutes = 900 + ((index * 173) % 1500);
    const seed = ((player.uid % 97) / 97);
    const finishing = (player.attributes.finishing ?? 10) / 20;
    const vision = (player.attributes.vision ?? 10) / 20;
    const passing = (player.attributes.passing ?? 10) / 20;
    const tackling = (player.attributes.tackling ?? 10) / 20;
    const heading = (player.attributes.heading ?? 10) / 20;
    const attack = group === "ST" ? finishing : group === "AM" ? finishing * 0.8 + vision * 0.2 : 0.12;
    const creation = group === "ST" ? vision * 0.5 : group === "AM" || group === "MID" ? vision * 0.8 + passing * 0.2 : passing * 0.25;
    const xg90 = Math.max(0, attack * 0.9 + seed * 0.15 - 0.12);
    const xa90 = Math.max(0, creation * 0.7 + seed * 0.2 - 0.08);
    const shots = Math.round((xg90 * minutes) / 90 * 7);
    const shotsOnTarget = Math.round(shots * (0.35 + seed * 0.25));
    const goals = Math.round((xg90 * minutes) / 90);
    const assists = Math.round((xa90 * minutes) / 90);
    const passesAttempted = Math.round(minutes * (group === "MID" ? 0.62 : 0.38));
    const passCompletion = Math.min(96, 65 + passing * 30 + seed * 3);
    const passesCompleted = Math.round(passesAttempted * passCompletion / 100);
    const advancedAvailable = index % 4 !== 0;
    const tacklesAttempted = Math.round(minutes * (group === "DC" || group === "FB" ? 0.055 : 0.025) + seed * 5);
    const tacklesCompleted = Math.round(tacklesAttempted * (0.6 + tackling * 0.3));
    const interceptions = advancedAvailable ? Math.round(minutes * (group === "DC" || group === "MID" ? 0.035 : 0.015) + seed * 4) : null;
    const keyPasses = advancedAvailable ? Math.round((xa90 * minutes) / 90 * 4 + seed * 4) : null;
    const progressivePasses = advancedAvailable ? Math.round(minutes * (group === "MID" ? 0.045 : 0.02) + seed * 10) : null;
    const headersWon = advancedAvailable ? Math.round(minutes * (group === "DC" ? 0.035 : 0.012) + seed * 5) : null;
    const aerialChallenges = advancedAvailable ? Math.round((headersWon ?? 0) / (0.45 + heading * 0.3)) : null;
    const clearances = advancedAvailable ? Math.round(minutes * (group === "DC" ? 0.02 : 0.004)) : null;
    const blocks = advancedAvailable ? Math.round(minutes * 0.006) : null;
    const crossesCompleted = advancedAvailable ? Math.round(minutes * (group === "FB" ? 0.012 : 0.004)) : null;
    const isGoalkeeper = group === "GK";
    const savesHeld = advancedAvailable && isGoalkeeper ? Math.round(minutes * 0.02) : null;
    const shotsFaced = advancedAvailable && isGoalkeeper ? Math.round(minutes * 0.07) : null;
    const savePercent = savesHeld != null && shotsFaced ? (savesHeld / shotsFaced) * 100 : null;
    const fee = player.transfer_value_state === "ok" ? player.transfer_value : null;
    const cost = (fee ?? 0) + (player.wage ?? 0) * 52 * 3;
    const productionTier = Math.min(99, Math.max(1, 38 + (player.ca - 90) * 0.55 + seed * 18));
    const productionGlobal = Math.min(99, Math.max(1, productionTier + (seed - 0.5) * 12));
    const costPctTier = Math.min(98, Math.max(1, Math.log1p(cost / 1000) * 6));
    const costPctGlobal = Math.min(98, Math.max(1, costPctTier + (seed - 0.5) * 8));
    const surplus = productionTier - costPctTier;
    const score = productionTier;
    const band = 30 / Math.sqrt(minutes / 90);
    const productionLow = Math.max(0, productionTier - band);
    const productionHigh = Math.min(100, productionTier + band);
    const scoreLow = productionLow;
    const gapPct = fee != null && fee > 0 ? Math.round((seed - 0.42) * 36) : null;
    // Free agents and unknown fees still get a market price, like the backend.
    const fairPrice =
      fee != null && fee > 0
        ? Math.round((fee * (1 + (gapPct ?? 0) / 200)) / (1 - (gapPct ?? 0) / 200))
        : Math.round(player.ca * 25_000 + seed * 500_000);
    const gapPercentile = gapPct == null ? null : Math.min(100, Math.max(0, 50 + gapPct * 1.5));
    const bargainScore = gapPercentile == null ? null : 0.55 * productionLow + 0.45 * gapPercentile;
    const confidence = Math.min(1, Math.max(0, 1 - (2 * band) / 100));
    const flags: string[] = [];
    if (fee == null) flags.push("missing_fee");
    if (player.wage == null) flags.push("missing_wage");
    if (!advancedAvailable) flags.push("advanced_missing");
    if (!player.league_name) flags.push("unknown_league");
    if (player.transfer_listed) flags.push("transfer_listed");
    if (player.loan_listed) flags.push("loan_listed");
    if (confidence < 0.7) flags.push("low_confidence");
    const drivers: StatsDriver[] = [
      { key: "npxg90", label: "npxG/90", percentile: Math.round(productionTier), contribution: 30 },
      { key: "xa90", label: "xA/90", percentile: Math.round(productionTier - 4), contribution: 25 },
      { key: "pass_pct", label: "Pass completion %", percentile: Math.round(passCompletion), contribution: 15 },
    ];
    return {
      uid: player.uid,
      name: player.name,
      age: player.age,
      club_uid: player.club_uid,
      club_name: player.club_name,
      league_name: player.league_name,
      nation_name: player.nation_name,
      ca: player.ca,
      pa: player.pa,
      position_group: group,
      minutes,
      starts: Math.floor(minutes / 90),
      substitute_appearances: index % 5,
      average_rating: Math.min(9, 6.2 + productionTier / 80),
      goals,
      assists,
      expected_goals: (xg90 * minutes) / 90,
      non_penalty_expected_goals: (xg90 * minutes) / 90,
      expected_assists: (xa90 * minutes) / 90,
      shots,
      shots_on_target: shotsOnTarget,
      passes_attempted: passesAttempted,
      passes_completed: passesCompleted,
      pass_completion_percent: passCompletion,
      tackles_attempted: tacklesAttempted,
      tackles_completed: tacklesCompleted,
      interceptions,
      key_passes: keyPasses,
      progressive_passes: progressivePasses,
      headers_won: headersWon,
      aerial_challenges_attempted: aerialChallenges,
      clearances,
      blocks,
      crosses_completed: crossesCompleted,
      saves_held: savesHeld,
      shots_on_target_faced: shotsFaced,
      save_percent: savePercent,
      expected_goals_prevented: advancedAvailable && isGoalkeeper ? (seed - 0.5) * 10 : null,
      advanced_available: advancedAvailable,
      transfer_value: player.transfer_value,
      transfer_value_state: player.transfer_value_state,
      wage: player.wage,
      contract_end: player.contract_end,
      club_reputation: null,
      fee,
      cost,
      fee_known: fee != null,
      wage_known: player.wage != null,
      production_global: productionGlobal,
      production_tier: productionTier,
      production_low: productionLow,
      production_high: productionHigh,
      cost_pct_global: costPctGlobal,
      cost_pct_tier: costPctTier,
      surplus,
      score,
      score_low: scoreLow,
      fair_price: fairPrice,
      value_gap_pct: gapPct,
      bargain_score: bargainScore,
      peer_count: 40 + (index % 20),
      confidence: Math.min(1, confidence),
      flags,
      drivers,
      transfer_listed: player.transfer_listed,
      loan_listed: player.loan_listed,
      injury_status: player.injury_status,
      recent_injury: player.recent_injury,
      injury_recent: player.recent_injury != null,
      stats_from_other_club: false,
    };
  });
  const filtered = rows.filter((row) => {
    if (row.minutes < minMinutes) return false;
    if (row.position_group === "OTHER") return false;
    if (model === "advanced" && !row.advanced_available) return false;
    if (row.position_group === "GK" && !(request.include_goalkeepers || request.position_group === "GK")) return false;
    if (request.position_group && !mockPositionFilterMatches(request.position_group, row.position_group)) return false;
    if (request.league && row.league_name !== request.league) return false;
    if (request.age_min != null && (row.age ?? 0) < request.age_min) return false;
    if (request.age_max != null && (row.age ?? 999) > request.age_max) return false;
    if (request.ca_min != null && row.ca < request.ca_min) return false;
    if (request.ca_max != null && row.ca > request.ca_max) return false;
    if (!request.include_unknown_fee && row.fee == null) return false;
    if (request.max_fee != null && row.fee != null && row.fee > request.max_fee) return false;
    if (request.max_wage != null && (row.wage ?? Infinity) > request.max_wage) return false;
    if (request.contract_end_before && (row.contract_end ?? "9999") > request.contract_end_before) return false;
    if (request.listings?.length) {
      const matches = request.listings.some((listing) =>
        listing === "sale" ? row.transfer_listed : listing === "loan" ? row.loan_listed : row.club_uid == null,
      );
      if (!matches) return false;
    }
    if (request.min_confidence != null && row.confidence < request.min_confidence) return false;
    if (request.exclude_injured && row.injury_status === "injured") return false;
    if (request.exclude_injury_history && row.injury_recent) return false;
    return true;
  }).sort((left, right) => {
    const by = request.sort_by ?? "score";
    const direction = request.sort_dir === "asc" ? 1 : -1;
    const read = (row: StatsMoneyballRow): number | string | null => {
      switch (by) {
        case "age": return row.age;
        case "ca": return row.ca;
        case "minutes": return row.minutes;
        case "production": return row.production_tier;
        case "production_low": return row.production_low;
        case "score_low": return row.score_low;
        case "bargain": return row.bargain_score;
        case "value_gap": return row.value_gap_pct;
        case "fair_price": return row.fair_price;
        case "cost": return row.cost;
        case "fee": return row.fee;
        case "wage": return row.wage;
        case "surplus": return row.surplus;
        case "name": return row.name;
        default: return row.score;
      }
    };
    const a = read(left);
    const b = read(right);
    if (a == null && b == null) return 0;
    if (a == null) return 1;
    if (b == null) return -1;
    if (typeof a === "string" || typeof b === "string") {
      return String(a).localeCompare(String(b)) * direction;
    }
    return (a - b) * direction;
  });
  const page = request.page ?? 0;
  const pageSize = Math.min(500, Math.max(1, request.page_size ?? 100));
  const slice = filtered.slice(page * pageSize, (page + 1) * pageSize);
  return {
    total: filtered.length,
    page,
    page_size: pageSize,
    rows: slice,
    meta: {
      model: model === "advanced" ? "stats-advanced-v3-mock" : "stats-core-v3-mock",
      scored: filtered.length,
      filtered: filtered.length,
      min_minutes: minMinutes,
      score_floor_minutes: 300,
      cost_horizon_years: 3,
      leagues: Array.from(new Set(rows.map((row) => row.league_name).filter((name): name is string => !!name))).sort(),
      window: request.window ?? "overall",
      mode: request.mode ?? "full",
      price_model: rows.some((row) => row.fair_price != null) ? "hedonic-ridge-v2-mock" : null,
      priced: rows.filter((row) => row.fair_price != null).length,
    },
  };
}

const MOCK_CLUB_ROWS: ClubRow[] = CLUBS.map(([name, uid, nation], index) => {
  const hasFinance = index < 6;
  const balance = hasFinance ? 80_000_000 - index * 9_000_000 : null;
  return {
    uid,
    unique_id: 700000 + uid,
    name,
    short_name: name,
    nation_id: nation,
    nation_name: NATION_NAMES[nation] ?? null,
    fa_nation_id: nation,
    fa_nation_name: NATION_NAMES[nation] ?? null,
    city_id: 1000 + index,
    gender: 0,
    reputation: 9000 - index * 180,
    last_league_position: 1 + (index % 18),
    league_competition_id: 22,
    league_name: "Italian Serie A",
    parent_club_uid: null,
    parent_club_name: null,
    corporate_facilities: hasFinance ? 6 + (index % 10) : null,
    finance_month: hasFinance ? "2026-09-01" : null,
    balance,
    transfer_budget_allocated: hasFinance ? 20_000_000 - index * 1_000_000 : null,
    transfer_budget_remaining: hasFinance ? 12_000_000 - index * 900_000 : null,
    wage_budget_weekly: hasFinance ? 900_000 - index * 60_000 : null,
    wage_payroll_weekly: hasFinance ? 850_000 - index * 55_000 : null,
    net: hasFinance ? 1_200_000 - index * 300_000 : null,
    player_count: 18 + (index % 12),
    average_age: 24 + (index % 6),
    average_ca: 145 - index * 2.5,
    total_value: hasFinance ? 240_000_000 - index * 12_000_000 : 50_000_000 - index * 1_000_000,
    total_wage: hasFinance ? 44_000_000 - index * 2_000_000 : 12_000_000,
    stadium_count: index % 4 === 0 ? 0 : 1,
    stadium_capacity: index % 4 === 0 ? null : 60_000 - index * 2_000,
  };
});

type MockFacilityField =
  | "training_facilities"
  | "youth_facilities"
  | "junior_coaching"
  | "youth_recruitment";

function mockFacilityValue(row: ClubRow, field: MockFacilityField): number | null {
  if (row.corporate_facilities == null) return null;
  switch (field) {
    case "training_facilities":
      return 12 + (row.uid % 8);
    case "youth_facilities":
      return 10 + (row.uid % 9);
    case "junior_coaching":
      return 9 + (row.uid % 10);
    case "youth_recruitment":
      return 10 + (row.uid % 8);
  }
}

export function mockQueryClubs(query: ClubQuery): ClubPage {
  let rows = [...MOCK_CLUB_ROWS];
  const inRange = (value: number | null, min?: number | null, max?: number | null) =>
    (min == null || (value ?? -Infinity) >= min) && (max == null || (value ?? Infinity) <= max);
  const search = query.search?.trim().toLowerCase();
  if (search) {
    rows = rows.filter(
      (row) =>
        row.name.toLowerCase().includes(search) ||
        row.short_name.toLowerCase().includes(search) ||
        String(row.uid).includes(search),
    );
  }
  if (query.nation_ids?.length) rows = rows.filter((row) => query.nation_ids!.includes(row.nation_id));
  if (query.genders?.length) rows = rows.filter((row) => query.genders!.includes(row.gender ?? -1));
  if (query.reputation_min != null) rows = rows.filter((row) => (row.reputation ?? 0) >= query.reputation_min!);
  if (query.reputation_max != null) rows = rows.filter((row) => (row.reputation ?? 99999) <= query.reputation_max!);
  if (query.has_finances != null) {
    rows = rows.filter((row) => (row.finance_month != null) === query.has_finances);
  }
  if (query.balance_min != null) rows = rows.filter((row) => (row.balance ?? -Infinity) >= query.balance_min!);
  if (query.balance_max != null) rows = rows.filter((row) => (row.balance ?? Infinity) <= query.balance_max!);
  if (query.transfer_budget_min != null) {
    rows = rows.filter((row) => (row.transfer_budget_remaining ?? -Infinity) >= query.transfer_budget_min!);
  }
  if (query.transfer_budget_max != null) {
    rows = rows.filter((row) => (row.transfer_budget_remaining ?? Infinity) <= query.transfer_budget_max!);
  }
  if (query.corporate_facilities_min != null || query.corporate_facilities_max != null) {
    rows = rows.filter((row) =>
      inRange(row.corporate_facilities, query.corporate_facilities_min, query.corporate_facilities_max),
    );
  }
  for (const field of [
    "training_facilities",
    "youth_facilities",
    "junior_coaching",
    "youth_recruitment",
  ] as const) {
    if (query[`${field}_min`] == null && query[`${field}_max`] == null) continue;
    rows = rows.filter((row) =>
      inRange(
        mockFacilityValue(row, field),
        query[`${field}_min`],
        query[`${field}_max`],
      ),
    );
  }
  if (query.player_count_min != null) rows = rows.filter((row) => row.player_count >= query.player_count_min!);
  if (query.has_stadium != null) rows = rows.filter((row) => (row.stadium_count > 0) === query.has_stadium);
  const rules = query.sort?.length
    ? query.sort
    : [{ by: query.sort_by ?? "reputation", dir: query.sort_dir ?? "desc" }];
  rows.sort((left, right) => {
    for (const rule of rules) {
      const direction = rule.dir === "asc" ? 1 : -1;
      const read = (row: ClubRow): number | string | null => {
        switch (rule.by) {
          case "name": return row.name;
          case "nation": return row.nation_name;
          case "balance": return row.balance;
          case "transfer_budget": return row.transfer_budget_remaining;
          case "wage_budget": return row.wage_budget_weekly;
          case "player_count": return row.player_count;
          case "average_ca": return row.average_ca;
          case "total_value": return row.total_value;
          case "stadium_capacity": return row.stadium_capacity;
          default: return row.reputation;
        }
      };
      const a = read(left);
      const b = read(right);
      if (a == null && b == null) continue;
      if (a == null) return 1;
      if (b == null) return -1;
      const compared = (a > b ? 1 : a < b ? -1 : 0) * direction;
      if (compared !== 0) return compared;
    }
    return left.uid - right.uid;
  });
  const page = query.page ?? 0;
  const pageSize = query.page_size ?? 100;
  return {
    total: rows.length,
    page,
    page_size: pageSize,
    rows: rows.slice(page * pageSize, (page + 1) * pageSize),
  };
}

export function mockClubDetail(uid: number): ClubDetail | null {
  const row = MOCK_CLUB_ROWS.find((club) => club.uid === uid);
  if (!row) return null;
  const club: Club = {
    uid: row.uid,
    unique_id: row.unique_id,
    name: row.name,
    short_name: row.short_name,
    nation_id: row.nation_id,
    nation_name: row.nation_name,
    fa_nation_id: row.fa_nation_id,
    fa_nation_name: row.fa_nation_name,
    city_id: row.city_id,
    gender: row.gender,
    reputation: row.reputation,
    last_league_position: row.last_league_position,
    league_competition_id: row.league_competition_id,
    league_name: row.league_name,
    parent_club_uid: row.parent_club_uid,
    parent_club_name: row.parent_club_name,
    corporate_facilities: row.corporate_facilities,
    training_facilities:
      row.corporate_facilities == null ? null : 12 + (row.uid % 8),
    youth_facilities: row.corporate_facilities == null ? null : 10 + (row.uid % 9),
    junior_coaching: row.corporate_facilities == null ? null : 9 + (row.uid % 10),
    youth_recruitment: row.corporate_facilities == null ? null : 10 + (row.uid % 8),
    youth_level: row.corporate_facilities == null ? null : 1 + (row.uid % 4),
    teams: [],
  };
  const financeMonths: FinanceMonth[] =
    row.finance_month == null
      ? []
      : ["2026-06-01", "2026-07-01", "2026-08-01", "2026-09-01"].map((month, index) => ({
          club_uid: row.uid,
          club_name: row.name,
          month,
          balance: (row.balance ?? 0) - (3 - index) * 2_000_000,
          transfer_budget_allocated: row.transfer_budget_allocated ?? 0,
          transfer_budget_remaining: (row.transfer_budget_remaining ?? 0) + (3 - index) * 400_000,
          wage_budget_weekly: row.wage_budget_weekly ?? 0,
          wage_payroll_weekly: row.wage_payroll_weekly ?? 0,
          income_excluding_transfers: 3_000_000 + index * 200_000,
          net_transfers: index % 2 === 0 ? 1_000_000 : -800_000,
          wage_bill: (row.wage_payroll_weekly ?? 0) * 4,
          net: 1_000_000 - index * 200_000,
          expenditure_excluding_transfers: 2_000_000 + index * 100_000,
          total_income: 3_500_000 + index * 200_000,
          total_expenditure: 2_500_000 + index * 300_000,
        }));
  const stadiums: Stadium[] =
    row.stadium_count > 0
      ? [
          {
            uid: 500000 + uid,
            name: `${row.name} Stadium`,
            all_seater_capacity: row.stadium_capacity ?? 20_000,
            expansion_capacity: (row.stadium_capacity ?? 20_000) + 5_000,
            capacity: null,
            owner_club_uid: row.uid,
            owner_club_name: row.name,
            pitch_length_dm: 1050,
            pitch_width_dm: 680,
            pitch_min_length_dm: 900,
            pitch_min_width_dm: 550,
            pitch_max_length_dm: 1200,
            pitch_max_width_dm: 900,
            built_date: "1995-07-01",
            rebuilt_date: "2015-07-01",
          },
        ]
      : [];
  return {
    club,
    finance_months: financeMonths,
    stadiums,
    child_clubs: [],
  };
}

export function mockClubFilterOptions(): ClubFilterOptions {
  const nations = new Map<number, string>();
  for (const row of MOCK_CLUB_ROWS) {
    if (row.nation_name) nations.set(row.nation_id, row.nation_name);
  }
  return {
    nations: [...nations.entries()].map(([id, name]) => ({
      id,
      name,
      count: MOCK_CLUB_ROWS.filter((row) => row.nation_id === id).length,
    })),
    genders: [{ id: 0, name: "Men", count: MOCK_CLUB_ROWS.length }],
    min_reputation: Math.min(...MOCK_CLUB_ROWS.map((row) => row.reputation ?? 0)),
    max_reputation: Math.max(...MOCK_CLUB_ROWS.map((row) => row.reputation ?? 0)),
    min_balance: 10_000_000,
    max_balance: 80_000_000,
    min_transfer_budget: 1_000_000,
    max_transfer_budget: 12_000_000,
    min_wage_budget: 200_000,
    max_wage_budget: 900_000,
    clubs_with_finances: MOCK_CLUB_ROWS.filter((row) => row.finance_month != null).length,
    clubs_with_stadiums: MOCK_CLUB_ROWS.filter((row) => row.stadium_count > 0).length,
  };
}

function bandLabels<T>(values: T[], label: (value: T) => string, order: string[]): DistributionBand[] {
  const counts = new Map<string, number>();
  for (const value of values) {
    const key = label(value);
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return order.map((key) => ({ label: key, count: counts.get(key) ?? 0 }));
}

function mockAgeLabel(age: number): string {
  if (age <= 17) return "≤17";
  if (age <= 21) return "18–21";
  if (age <= 25) return "22–25";
  if (age <= 29) return "26–29";
  if (age <= 33) return "30–33";
  return "34+";
}

function roundOne(value: number): number {
  return Math.round(value * 10) / 10;
}

export function mockSquadAnalysis(clubUid: number): SquadAnalysis {
  const players = MOCK_PLAYERS.filter((player) => player.club_uid === clubUid);
  const rows = players.map(mockPlayerRow);
  const ages = rows.map((row) => row.age).filter((age): age is number => age != null);
  const pa = rows.map((row) => row.pa).filter((value): value is number => value != null);
  const contractYears = new Map<string, number>();
  for (const row of rows) {
    const year = row.contract_end?.slice(0, 4);
    if (year) contractYears.set(year, (contractYears.get(year) ?? 0) + 1);
  }
  const positionGroups = ["门将", "后卫", "中场", "前锋"].map((label) => ({ label, count: 0 }));
  for (const row of rows) {
    if (row.positions.gk >= 15) positionGroups[0].count += 1;
    if ([row.positions.dl, row.positions.dc, row.positions.dr, row.positions.wbl, row.positions.wbr].some((rating) => rating >= 15)) positionGroups[1].count += 1;
    if ([row.positions.dm, row.positions.ml, row.positions.mc, row.positions.mr].some((rating) => rating >= 15)) positionGroups[2].count += 1;
    if ([row.positions.stc, row.positions.aml, row.positions.amc, row.positions.amr].some((rating) => rating >= 15)) positionGroups[3].count += 1;
  }
  const teamInfo: Record<number, Pick<SquadTeam, "competition_id" | "competition_name" | "fixture_count" | "label">> = {
    0: {
      competition_id: 22,
      competition_name: "Italian Serie A",
      fixture_count: 38,
      label: { kind: "firstTeam", age: null },
    },
    1: {
      competition_id: 3560,
      competition_name: "Italian U20 Primavera 1",
      fixture_count: 38,
      label: { kind: "youth", age: 20 },
    },
    2: {
      competition_id: 3572,
      competition_name: "Italian U18 Division 1 Group A",
      fixture_count: 26,
      label: { kind: "youth", age: 18 },
    },
  };
  const teams = [...new Set(rows.map((row) => row.team_slot).filter((slot): slot is number => slot != null))]
    .sort((left, right) => left - right)
    .map((slot): SquadTeam => {
      const info = teamInfo[slot];
      if (info) {
        return { team_id: 800 + slot, slot, inferred: false, ...info };
      }
      return {
        team_id: 800 + slot,
        slot,
        competition_id: null,
        competition_name: null,
        fixture_count: 0,
        label: { kind: "unknown", age: null },
        inferred: true,
      };
    });
  return {
    club_uid: clubUid,
    club_name: players[0]?.club_name ?? `Club ${clubUid}`,
    player_count: rows.length,
    average_age: roundOne(ages.reduce((sum, age) => sum + age, 0) / Math.max(1, ages.length)),
    average_ca: roundOne(rows.reduce((sum, row) => sum + row.ca, 0) / Math.max(1, rows.length)),
    average_pa: roundOne(pa.reduce((sum, value) => sum + value, 0) / Math.max(1, pa.length)),
    total_wage: rows.reduce((sum, row) => sum + (row.wage ?? 0), 0),
    total_value: rows.reduce((sum, row) => sum + (row.transfer_value ?? 0), 0),
    loanees: rows.filter((row) => row.on_loan).length,
    position_groups: positionGroups,
    age_bands: bandLabels(ages, mockAgeLabel, ["≤17", "18–21", "22–25", "26–29", "30–33", "34+"]),
    contract_years: [...contractYears.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([label, count]) => ({ label, count })),
    players: rows.sort((a, b) => b.ca - a.ca),
    teams,
  };
}

const MOCK_SCOUT_POSITIONS = [
  "gk", "dr", "dc", "dl", "wbr", "wbl", "dm", "mc", "mr", "ml", "amr", "amc", "aml", "stc",
];

const MOCK_SCOUT_LINES: { key: string; positions: string[]; target: number }[] = [
  { key: "gk", positions: ["gk"], target: 2 },
  { key: "dc", positions: ["dc"], target: 4 },
  { key: "rightBack", positions: ["dr", "wbr"], target: 2 },
  { key: "leftBack", positions: ["dl", "wbl"], target: 2 },
  { key: "dm", positions: ["dm"], target: 2 },
  { key: "mc", positions: ["mc"], target: 3 },
  { key: "rightWing", positions: ["amr", "mr"], target: 2 },
  { key: "leftWing", positions: ["aml", "ml"], target: 2 },
  { key: "amc", positions: ["amc"], target: 2 },
  { key: "st", positions: ["stc"], target: 3 },
];

const MOCK_SCOUT_ROLES: ScoutRoleInfo[] = [
  ["goalkeeper", "门将（防守）", ["gk"]],
  ["sweeper_keeper", "清道夫门将（策应）", ["gk"]],
  ["central_defender", "中后卫（防守）", ["dc"]],
  ["ball_playing_defender", "出球后卫（防守）", ["dc"]],
  ["no_nonsense_centre_back", "传统中后卫（防守）", ["dc"]],
  ["full_back", "边后卫（防守）", ["dl", "dr"]],
  ["wing_back", "翼卫（进攻）", ["dl", "dr", "wbl", "wbr"]],
  ["inverted_full_back", "内收边后卫（防守）", ["dl", "dr"]],
  ["defensive_midfielder", "防守型中场（防守）", ["dm"]],
  ["anchor", "锚定中场（防守）", ["dm"]],
  ["deep_lying_playmaker", "后组织核心（策应）", ["dm", "mc"]],
  ["central_midfielder", "中场（策应）", ["mc"]],
  ["box_to_box_midfielder", "全能中场（策应）", ["mc"]],
  ["advanced_playmaker", "前组织核心（进攻）", ["amc", "mc"]],
  ["attacking_midfielder", "攻击型中场（进攻）", ["amc"]],
  ["winger", "边锋（进攻）", ["aml", "amr", "ml", "mr"]],
  ["inverted_winger", "内切边锋（进攻）", ["aml", "amr"]],
  ["inside_forward", "内锋（进攻）", ["aml", "amr", "stc"]],
  ["advanced_forward", "突前前锋（进攻）", ["stc"]],
  ["poacher", "抢点前锋（进攻）", ["stc"]],
  ["complete_forward", "全能前锋（进攻）", ["stc"]],
  ["target_forward", "站桩中锋（进攻）", ["stc"]],
  ["deep_lying_forward", "拖后前锋（策应）", ["stc"]],
  ["pressing_forward", "逼抢前锋（防守）", ["stc"]],
].map(([id, name, positions]) => ({
  id: id as string,
  name: name as string,
  positions: positions as string[],
}));

function mockScoutPlayer(player: Player): ScoutPlayer {
  return {
    uid: player.uid,
    name: player.name,
    age: player.age,
    club_uid: player.club_uid,
    club_name: player.club_name,
    ca: player.ca,
    pa: player.pa,
    transfer_value:
      player.transfer_value_state === "ok" ? player.transfer_value : null,
    wage: player.contract?.wage ?? null,
    contract_end: player.contract?.end ?? null,
    transfer_listed: player.transfer_listed,
  };
}

function mockPositionRating(player: Player, position: string): number {
  return (player.positions as unknown as Record<string, number>)[position] ?? 0;
}

function mockScoutLineOf(player: Player): { line: number; rating: number } | null {
  let best: { line: number; rating: number } | null = null;
  MOCK_SCOUT_LINES.forEach((line, index) => {
    const rating = Math.max(
      0,
      ...line.positions.map((position) => mockPositionRating(player, position)),
    );
    if (rating > 0 && rating > (best?.rating ?? 0)) best = { line: index, rating };
  });
  return best;
}

function mockScoutFactor(rating: number): number {
  if (rating >= 18) return 1;
  if (rating >= 15) return 0.97;
  if (rating >= 13) return 0.88;
  if (rating >= 10) return 0.75;
  return 0;
}

function mockScoutFit(
  player: Player,
  position: string,
  roleId: string | null,
): ScoutFitting | null {
  const rating = mockPositionRating(player, position);
  if (rating < 10) return null;
  const roles = MOCK_SCOUT_ROLES.filter((role) => role.positions.includes(position));
  const role = roleId
    ? roles.find((candidate) => candidate.id === roleId) ?? roles[0]
    : roles[0];
  if (!role) return null;
  const raw = Math.min(
    20,
    Math.max(
      1,
      Math.round(player.ca / 14 + (rating - 10) * 0.3) + (player.uid % 3) - 1,
    ),
  );
  return {
    player: mockScoutPlayer(player),
    fit: Math.max(0, Math.min(100, Math.round(((raw - 5) / 12) * 100 * mockScoutFactor(rating)))),
    role_id: role.id,
    role_name: role.name,
    role_score: raw,
    position_rating: rating,
  };
}

function mockScoutFee(player: Player): number | null {
  if (player.transfer_value_state !== "ok") return null;
  const value = player.transfer_value;
  if (value == null || value === 300_000_000) return null;
  return Math.round(player.transfer_listed ? value * 0.8 : value);
}

function mockScoutWage(player: Player): number {
  const wage = player.contract?.wage;
  if (wage != null && wage > 0) return Math.round(wage * 1.25);
  const base = Math.max(0, player.ca - 50);
  return Math.max(250, Math.round((base * base * base) / 17));
}

function mockScoutProbability(player: Player): number {
  let points = 50;
  if (player.transfer_listed) points += 12;
  if (player.transfer_value_state !== "ok") points += 8;
  if ((player.age ?? 27) >= 30) points -= 5;
  if (player.transfer_value != null && player.transfer_value > 30_000_000) points -= 10;
  points += ((player.uid * 7) % 19) - 9;
  return Math.max(0, Math.min(100, points));
}

export function mockScoutOverview(clubUid: number): ScoutOverview {
  const players = MOCK_PLAYERS.filter((player) => player.club_uid === clubUid);
  const people = players.map(mockScoutPlayer);
  const abilities = people.map((player) => player.ca).sort((a, b) => b - a);
  const level = abilities.length
    ? Math.round(abilities.slice(0, Math.min(11, abilities.length)).reduce((sum, ca) => sum + ca, 0) / Math.min(11, abilities.length))
    : 0;
  const ages = people.map((player) => player.age).filter((age): age is number => age != null);
  const averageAge = ages.length
    ? Math.round((ages.reduce((sum, age) => sum + age, 0) / ages.length) * 10) / 10
    : null;

  const lines: ScoutLine[] = MOCK_SCOUT_LINES.map((line, index) => {
    const own = players
      .map((player) => ({ player, place: mockScoutLineOf(player) }))
      .filter((entry) => entry.place?.line === index)
      .map(
        ({ player, place }): ScoutDepthPlayer => ({
          player: mockScoutPlayer(player),
          rating: place?.rating ?? 0,
          on_loan: player.on_loan === true,
        }),
      )
      .sort((a, b) => b.player.ca - a.player.ca);
    const cover = players
      .filter((player) => mockScoutLineOf(player)?.line !== index)
      .map((player) => ({
        player,
        rating: Math.max(
          0,
          ...line.positions.map((position) => mockPositionRating(player, position)),
        ),
      }))
      .filter((entry) => entry.rating >= 15)
      .map(
        ({ player, rating }): ScoutDepthPlayer => ({
          player: mockScoutPlayer(player),
          rating,
          on_loan: player.on_loan === true,
        }),
      )
      .sort((a, b) => b.player.ca - a.player.ca);
    const status =
      own.length === 0 && cover.length === 0
        ? "missing"
        : own.length + Math.floor(cover.length / 2) < line.target
          ? "thin"
          : own.length >= line.target + 2
            ? "crowded"
            : "fine";
    return {
      key: line.key,
      positions: line.positions,
      target: line.target,
      players: own,
      cover,
      status,
      weak: own.length > 0 && own[0].player.ca + 15 < level,
      ageing:
        own.length > 0 && own.every((player) => (player.player.age ?? 0) >= 31),
    };
  });

  const departures: ScoutDeparture[] = [];
  for (const player of players) {
    const place = mockScoutLineOf(player);
    if (!place) continue;
    const line = lines[place.line];
    const rank = line.players.findIndex((entry) => entry.player.uid === player.uid) + 1;
    const reasons: ScoutDeparture["reasons"] = [];
    if (line.status === "crowded" && rank > line.target) {
      reasons.push({ code: "surplus", value: line.players.length });
    }
    if ((player.age ?? 0) >= 32) reasons.push({ code: "ageing", value: player.age });
    if (player.transfer_listed) reasons.push({ code: "listed", value: null });
    if (reasons.length === 0) continue;
    const value = player.transfer_value_state === "ok" ? player.transfer_value : null;
    const action: ScoutDepartureAction =
      value != null && value < 250_000 && (player.age ?? 0) >= 28
        ? "release"
        : "sell";
    departures.push({
      player: mockScoutPlayer(player),
      action,
      reasons,
      priority: reasons.length,
      value,
      not_for_sale: value === 300_000_000,
      wage: player.contract?.wage ?? null,
      line: line.key,
      rank,
    });
  }
  departures.sort((a, b) => b.priority - a.priority || a.player.ca - b.player.ca);

  const wageBill = people.reduce((sum, player) => sum + (player.wage ?? 0), 0);
  const wageBudget = 1_500_000;
  return {
    club_uid: clubUid,
    club_name: players[0]?.club_name ?? `Club ${clubUid}`,
    transfer_budget: 30_000_000,
    wage_budget: wageBudget,
    wage_bill: wageBill,
    wage_room: wageBudget - wageBill,
    squad_size: people.length,
    average_age: averageAge,
    level,
    lines,
    departures,
    positions: MOCK_SCOUT_POSITIONS,
    roles: MOCK_SCOUT_ROLES,
  };
}

export function mockScoutAdvice(request: ScoutRequest): Scouted {
  const own = MOCK_PLAYERS.filter((player) => player.club_uid === request.club_uid);
  const ownFits = own
    .map((player) => mockScoutFit(player, request.position, request.role_id ?? null))
    .filter((fitting): fitting is ScoutFitting => fitting != null)
    .sort((a, b) => b.fit - a.fit);
  const bestOwn = ownFits[0]?.fit ?? 0;
  const secondOwn = ownFits[1]?.fit ?? 0;

  const candidates = MOCK_PLAYERS.filter(
    (player) =>
      player.club_uid != null &&
      player.club_uid !== request.club_uid &&
      player.gender === 0,
  );
  const suggestions: ScoutSuggestion[] = [];
  for (const player of candidates) {
    const fitting = mockScoutFit(player, request.position, request.role_id ?? null);
    if (!fitting) continue;
    const age = player.age ?? 27;
    const room = (player.pa ?? player.ca) - player.ca;
    if (request.min_age != null && age < request.min_age) continue;
    if (request.max_age != null && age > request.max_age) continue;
    if (request.no_free_agents && player.club_uid == null) continue;
    if (request.purpose === "future" && (age > 21 || room < 15)) continue;
    if (request.purpose === "firstTeam" && fitting.fit <= bestOwn) continue;
    if (request.purpose === "rotation" && fitting.fit < Math.max(secondOwn, bestOwn - 10)) continue;
    const probability = mockScoutProbability(player);
    if (request.min_probability != null && probability < request.min_probability) continue;
    const fee = mockScoutFee(player);
    if (request.max_fee != null && (fee == null || fee > request.max_fee)) continue;
    const growth = age <= 23 && room >= 15 ? Math.min(6, Math.floor(room / 5)) : 0;
    suggestions.push({
      ...fitting,
      probability,
      reasons: player.transfer_listed
        ? [{ code: "listed", points: 12, value: null }]
        : player.uid % 3 === 0
          ? [
              { code: "smallerClub", points: 8, value: null },
              { code: "ambitious", points: 4, value: 16 },
            ]
          : player.uid % 3 === 1
            ? [{ code: "fringe", points: 8, value: 4 }]
            : [{ code: "biggerClub", points: -18, value: 170 }],
      fee,
      wage: mockScoutWage(player),
      value:
        player.transfer_value_state === "ok" ? player.transfer_value : null,
      score: Math.max(
        0,
        Math.min(100, Math.round(0.7 * fitting.fit + 0.3 * probability) + growth),
      ),
      owner_club: null,
      owner_club_uid: null,
    });
  }
  suggestions.sort((a, b) => b.score - a.score || b.fit - a.fit);
  const limit = Math.max(1, Math.min(200, request.how_many ?? 50));
  const wageBill = own.reduce((sum, player) => sum + (player.contract?.wage ?? 0), 0);
  return {
    suggestions: suggestions.slice(0, limit),
    squad: ownFits.slice(0, 5),
    transfer_budget: 30_000_000,
    wage_budget: 1_500_000,
    wage_bill: wageBill,
    wage_room: 1_500_000 - wageBill,
    examined: candidates.filter(
      (player) => mockPositionRating(player, request.position) >= 10,
    ).length,
    candidates: candidates.filter(
      (player) => mockScoutFit(player, request.position, request.role_id ?? null) != null,
    ).length,
    eligible: suggestions.length,
    game_date: MOCK_SUMMARY.game_date,
  };
}
