import { translate, type Locale } from "./i18n";
import { SAVE_UNITS, type CurrencyCode } from "./currency";
import { ATTRIBUTE_LABELS, type Clause } from "./types";
import { formatValue, formatWage } from "./utils";

/**
 * Locale-aware labels for values that come from the parser or from the fixed
 * domain tables. Database values themselves are never rewritten.
 */

const ATTRIBUTE_LABELS_EN: Record<string, string> = {
  crossing: "Crossing",
  dribbling: "Dribbling",
  finishing: "Finishing",
  heading: "Heading",
  long_shots: "Long Shots",
  marking: "Marking",
  off_the_ball: "Off the Ball",
  passing: "Passing",
  penalty_taking: "Penalty Taking",
  tackling: "Tackling",
  vision: "Vision",
  handling: "Handling",
  aerial_reach: "Aerial Reach",
  command_of_area: "Command of Area",
  communication: "Communication",
  kicking: "Kicking",
  throwing: "Throwing",
  anticipation: "Anticipation",
  decisions: "Decisions",
  one_on_ones: "One on Ones",
  positioning: "Positioning",
  reflexes: "Reflexes",
  first_touch: "First Touch",
  technique: "Technique",
  flair: "Flair",
  corners: "Corners",
  teamwork: "Teamwork",
  work_rate: "Work Rate",
  long_throws: "Long Throws",
  eccentricity: "Eccentricity",
  rushing_out: "Rushing Out",
  punching: "Punching",
  acceleration: "Acceleration",
  free_kick_taking: "Free Kick Taking",
  strength: "Strength",
  stamina: "Stamina",
  pace: "Pace",
  jumping_reach: "Jumping Reach",
  leadership: "Leadership",
  dirtiness: "Dirtiness",
  balance: "Balance",
  bravery: "Bravery",
  consistency: "Consistency",
  aggression: "Aggression",
  agility: "Agility",
  important_matches: "Important Matches",
  injury_proneness: "Injury Proneness",
  versatility: "Versatility",
  natural_fitness: "Natural Fitness",
  determination: "Determination",
  composure: "Composure",
  concentration: "Concentration",
};

const PERSONALITY_LABELS_EN: Record<string, string> = {
  adaptability: "Adaptability",
  ambition: "Ambition",
  loyalty: "Loyalty",
  pressure: "Pressure",
  professionalism: "Professionalism",
  sportsmanship: "Sportsmanship",
  temperament: "Temperament",
  controversy: "Controversy",
};

const PERSONALITY_LABELS_ZH: Record<string, string> = {
  adaptability: "适应",
  ambition: "雄心",
  loyalty: "忠诚",
  pressure: "抗压",
  professionalism: "职业",
  sportsmanship: "体育精神",
  temperament: "气质",
  controversy: "争议",
};

/**
 * Facility ratings run from 1 to 20. The game groups them into ten named tiers,
 * best first. The scale is the same for every facility line.
 */
const FACILITY_TIER_LABELS: Record<Locale, string[]> = {
  en: [
    "State of the Art",
    "Superb",
    "Excellent",
    "Great",
    "Good",
    "Average",
    "Adequate",
    "Below Average / Fairly Basic",
    "Basic",
    "Poor / Minimal",
  ],
  "zh-CN": [
    "顶级",
    "卓越",
    "优秀",
    "出色",
    "良好",
    "一般",
    "合格",
    "中下 / 较基础",
    "基础",
    "较差 / 最低",
  ],
};

/** Index into the ten-tier scale for a 1–20 rating, or null when unset. */
export function facilityTierIndex(value: number | null | undefined): number | null {
  if (value == null) return null;
  const rating = Math.max(1, Math.min(20, Math.round(value)));
  if (rating === 20) return 0;
  if (rating >= 18) return 1;
  if (rating >= 16) return 2;
  if (rating >= 14) return 3;
  if (rating >= 12) return 4;
  if (rating >= 10) return 5;
  if (rating >= 8) return 6;
  if (rating >= 6) return 7;
  if (rating >= 4) return 8;
  return 9;
}

/** The word the game displays for a facility rating; unnamed values stay numeric. */
export function facilityRatingLabel(
  value: number | null | undefined,
  locale: Locale,
): string | null {
  const index = facilityTierIndex(value);
  if (index == null) return null;
  return FACILITY_TIER_LABELS[locale][index];
}

const SEVERITY_LABELS: Record<Locale, Record<number, string>> = {
  en: { 0: "Minor", 1: "Moderate", 2: "Severe", 3: "Major" },
  "zh-CN": { 0: "轻微", 1: "轻度", 2: "中度", 3: "重度" },
};

/**
 * Contract clause kinds confirmed by the FM27 data and the in-game contract
 * screens. Codes are the raw values stored in the save; every code seen in the
 * current save is covered. The parser keeps any other code as a raw number, so
 * the UI falls back to showing that number rather than guessing a meaning.
 */
const CLAUSE_KIND_LABELS: Record<Locale, Record<number, string>> = {
  en: {
    0: "Minimum Fee Release Clause",
    1: "Relegation Release Clause",
    2: "Non-Promotion Release Clause",
    3: "Yearly Wage Rise",
    4: "Promotion Wage Rise",
    5: "Relegation Wage Drop",
    6: "Non-Playing Job Offer Release Clause",
    7: "Release At Any Time Clause",
    8: "Sell-On Fee Percentage",
    9: "Sell-On Fee Profit Percentage",
    10: "Seasonal Landmark Goal Bonus",
    11: "One-Year Extension After League Games (Final Season)",
    12: "Match Highest Earner Clause",
    13: "Wage After Reaching Club League Games",
    14: "Top Division Promotion Wage Rise",
    15: "Top Division Relegation Wage Drop",
    16: "Minimum Fee Release Clause (Foreign Clubs)",
    17: "Minimum Fee Release Clause (Domestic Clubs in Higher Division)",
    18: "Minimum Fee Release Clause (Domestic Clubs)",
    19: "Wage After Reaching International Appearances",
    20: "Percentage of Wages Paid by Sponsor",
    21: "Percentage of Gate Receipts Paid to Player",
    22: "Optional Contract Extension by Club",
    23: "Waive Club Compensation for Managerial Role",
    24: "Percentage of Club Compensation Required for Managerial Role",
    25: "One-Year Extension After League Games (Promoted Final Season)",
    26: "One-Year Extension After League Games (Avoid Relegation Final Season)",
    27: "Minimum Fee Release Clause (Clubs in a Major Continental Competition)",
    28: "MLS Guaranteed Contract Clause",
    29: "Contract Extension After Promotion",
    30: "Injury Release Clause",
    31: "Minimum Fee Release Clause (Clubs in a Continental Competition)",
    32: "Appearance Fee",
    33: "Goal Bonus",
    34: "Clean Sheet Bonus",
    35: "Team of the Year Bonus (Division)",
    36: "Top Goalscorer Bonus (Division)",
    37: "International Cap Bonus",
    38: "Unused Substitute Fee",
    39: "Assist Bonus",
    40: "Seasonal Landmark Assist Bonus",
    41: "Seasonal Landmark Combined Goal and Assist Bonus",
    54: "Will Leave at End of Contract",
    55: "Active Relegation Release Clause",
    56: "Active Non-Promotion Release Clause",
    58: "Committee Assigned Minimum Fee Release Clause",
    63: "Will Explore Options at End of Contract",
  },
  "zh-CN": {
    0: "最低解约金条款",
    1: "降级解约金条款",
    2: "未升级解约金条款",
    3: "年度涨薪条款",
    4: "升级涨薪条款",
    5: "降级减薪条款",
    6: "非球员岗位解约条款",
    7: "随时解约条款",
    8: "转会费分成条款",
    9: "转会利润分成条款",
    10: "赛季进球里程碑奖金",
    11: "一年续约选项（联赛出场，合同最后一年）",
    12: "顶薪匹配条款",
    13: "联赛出场达标后涨薪",
    14: "顶级联赛升级涨薪",
    15: "顶级联赛降级减薪",
    16: "最低解约金条款（国外俱乐部）",
    17: "最低解约金条款（国内更高级别俱乐部）",
    18: "最低解约金条款（国内俱乐部）",
    19: "国家队出场达标后涨薪",
    20: "赞助商承担工资比例",
    21: "门票收入分成比例",
    22: "俱乐部可选续约条款",
    23: "放弃主教练岗位补偿金",
    24: "主教练岗位补偿金比例",
    25: "一年续约选项（升级，合同最后一年）",
    26: "一年续约选项（保级，合同最后一年）",
    27: "最低解约金条款（主要洲际赛事俱乐部）",
    28: "MLS 保障合同条款",
    29: "升级后自动续约",
    30: "伤病解约条款",
    31: "最低解约金条款（洲际赛事俱乐部）",
    32: "出场费",
    33: "进球奖金",
    34: "零封奖金",
    35: "赛季最佳阵容奖金（联赛）",
    36: "联赛最佳射手奖金",
    37: "国家队出场奖金",
    38: "未上场替补费",
    39: "助攻奖金",
    40: "赛季助攻里程碑奖金",
    41: "赛季进球助攻里程碑奖金",
    54: "合同到期离队",
    55: "已激活的降级解约金条款",
    56: "已激活的未升级解约金条款",
    58: "委员会设定最低解约金条款",
    63: "合同到期后探索去向",
  },
};

/** Clauses that only record that they are present, with no amount. */
const FLAG_CLAUSE_KINDS = new Set([6, 7, 12, 23, 28, 30, 54, 55, 56, 63]);

/** Clauses whose parameter is a percentage. */
const PERCENT_CLAUSE_KINDS = new Set([3, 4, 5, 8, 14, 15, 20, 21, 24]);

const ROLE_NAMES_EN: Record<string, string> = {
  goalkeeper: "Goalkeeper (Defend)",
  sweeper_keeper: "Sweeper Keeper (Support)",
  ball_playing_keeper: "Ball-Playing Keeper (Defend)",
  central_defender: "Central Defender (Defend)",
  ball_playing_defender: "Ball-Playing Defender (Defend)",
  no_nonsense_centre_back: "No-Nonsense Centre-Back",
  full_back: "Full-Back (Defend)",
  wing_back: "Wing-Back (Attack)",
  inverted_full_back: "Inverted Full-Back (Support)",
  defensive_midfielder: "Defensive Midfielder (Defend)",
  anchor: "Anchor (Defend)",
  ball_winning_midfielder: "Ball-Winning Midfielder (Defend)",
  deep_lying_playmaker: "Deep-Lying Playmaker (Support)",
  central_midfielder: "Central Midfielder (Support)",
  box_to_box_midfielder: "Box-to-Box Midfielder (Support)",
  mezzala: "Mezzala (Attack)",
  advanced_playmaker: "Advanced Playmaker (Attack)",
  attacking_midfielder: "Attacking Midfielder (Attack)",
  trequartista: "Trequartista (Attack)",
  shadow_striker: "Shadow Striker (Attack)",
  winger: "Winger (Attack)",
  inverted_winger: "Inverted Winger (Attack)",
  inside_forward: "Inside Forward (Attack)",
  advanced_forward: "Advanced Forward (Attack)",
  poacher: "Poacher (Attack)",
  complete_forward: "Complete Forward (Attack)",
  target_forward: "Target Forward (Attack)",
  deep_lying_forward: "Deep-Lying Forward (Support)",
  pressing_forward: "Pressing Forward (Attack)",
};

const ROLE_GROUPS_EN: Record<string, string> = {
  门将: "Goalkeepers",
  后卫: "Defenders",
  中场: "Midfielders",
  前场: "Attackers",
  前锋: "Forwards",
};

const STAFF_ATTRIBUTE_LABELS: Record<Locale, Record<string, string>> = {
  en: {
    authority: "Authority",
    working_with_youngsters: "Working With Youngsters",
    determination: "Determination",
    goalkeeping: "Goalkeeping",
    judging_player_ability: "Judging Player Ability",
    judging_player_potential: "Judging Player Potential",
    people_management: "People Management",
    motivating: "Motivating",
    physiotherapy: "Physiotherapy",
    tactical_knowledge: "Tactical Knowledge",
    attacking: "Attacking",
    defending: "Defending",
    fitness: "Fitness",
    possession: "Possession",
    technical: "Technical",
    tactical: "Tactical",
    data_analysis: "Data Analysis",
    sports_science: "Sports Science",
    negotiating: "Negotiating",
    judging_staff_ability: "Judging Staff Ability",
    set_pieces: "Set Pieces",
  },
  "zh-CN": {
    authority: "权威",
    working_with_youngsters: "与年轻球员共事",
    determination: "意志力",
    goalkeeping: "门将训练",
    judging_player_ability: "判断球员能力",
    judging_player_potential: "判断球员潜力",
    people_management: "人员管理",
    motivating: "激励",
    physiotherapy: "理疗",
    tactical_knowledge: "战术知识",
    attacking: "进攻训练",
    defending: "防守训练",
    fitness: "体能训练",
    possession: "控球训练",
    technical: "技术训练",
    tactical: "战术训练",
    data_analysis: "数据分析",
    sports_science: "运动科学",
    negotiating: "谈判",
    judging_staff_ability: "判断职员能力",
    set_pieces: "定位球训练",
  },
};

const STAFF_ROLE_LABELS: Record<Locale, Record<string, string>> = {
  en: {
    manager: "Manager",
    assistant_manager: "Assistant Manager",
    coach: "Coach",
    physio: "Physio",
    scout: "Scout",
    gk_coach: "Goalkeeping Coach",
    fitness_coach: "Fitness Coach",
    chairperson: "Chairperson",
    director_of_football: "Director of Football",
    head_of_youth_development: "Head of Youth Development",
    loan_manager: "Loan Manager",
    technical_director: "Technical Director",
    data_analyst: "Data Analyst",
    sports_scientist: "Sports Scientist",
    set_piece_coach: "Set Piece Coach",
  },
  "zh-CN": {
    manager: "主教练",
    assistant_manager: "助理教练",
    coach: "教练",
    physio: "理疗师",
    scout: "球探",
    gk_coach: "门将教练",
    fitness_coach: "体能教练",
    chairperson: "主席",
    director_of_football: "足球总监",
    head_of_youth_development: "青训主管",
    loan_manager: "租借经理",
    technical_director: "技术总监",
    data_analyst: "数据分析师",
    sports_scientist: "运动科学家",
    set_piece_coach: "定位球教练",
  },
};

const STAFF_JOB_LABELS_ZH: Record<number, string> = {
  2: "教练",
  4: "主席",
  6: "董事",
  8: "总经理",
  10: "足球总监",
  12: "理疗师",
  14: "球探",
  16: "主教练",
  20: "助理教练",
  22: "定位球教练",
  26: "体能教练",
  34: "门将教练",
  38: "首席队医",
  40: "运动科学主管",
  48: "运动科学家",
  50: "首席理疗师",
  58: "数据分析师",
  60: "表现分析师",
  62: "数据分析师",
  64: "青训主管",
  70: "主席",
  86: "租借经理",
  88: "技术总监",
};

/** Staff profile attribute name; unknown keys stay as stored. */
export function staffAttributeLabel(key: string, locale: Locale): string {
  return STAFF_ATTRIBUTE_LABELS[locale][key] ?? key;
}

/** Role-suitability name from the save's role order, or its stored English name. */
export function staffRoleLabel(
  key: string | null | undefined,
  fallback: string | null | undefined,
  locale: Locale,
): string | null {
  if (key) {
    const label = STAFF_ROLE_LABELS[locale][key];
    if (label) return label;
  }
  return fallback ?? key ?? null;
}

/** The in-game job title; unconfirmed codes keep their English fallback or raw code. */
export function staffJobLabel(
  code: number | null | undefined,
  fallback: string | null | undefined,
  locale: Locale,
): string | null {
  if (code != null && locale === "zh-CN" && STAFF_JOB_LABELS_ZH[code]) {
    return STAFF_JOB_LABELS_ZH[code];
  }
  return fallback ?? (code != null ? `Job #${code}` : null);
}

const TRAIT_LABELS_ZH: Record<number, string> = {
  0: "左路带球",
  1: "右路带球",
  2: "中路带球",
  3: "插入禁区",
  4: "拉边接应",
  5: "积极前插",
  6: "简单短传",
  7: "常传威胁球",
  8: "远射",
  9: "大力射门",
  10: "巧射",
  11: "弧线球",
  12: "喜欢盘过门将",
  13: "喜欢反越位",
  14: "使用外脚背",
  15: "紧逼盯人",
  16: "挑衅对手",
  17: "与裁判争论",
  18: "背身拿球",
  19: "回撤拿球",
  20: "踢墙配合",
  21: "喜欢吊射",
  22: "控制节奏",
  23: "尝试倒钩",
  24: "优先传球而非射门",
  25: "不传直塞球",
  26: "拖慢节奏",
  27: "人球分过",
  28: "盘带前调整到右脚",
  29: "盘带前调整到左脚",
  30: "粘球",
  31: "后插上进入禁区",
  32: "尝试摆脱困境",
  33: "始终留在后场",
  34: "避免使用逆足",
  35: "尝试花活",
  36: "尝试远距离任意球",
  37: "倒地铲抢",
  38: "不倒地铲抢",
  39: "两翼内切",
  40: "抱边",
  41: "煽动观众",
  42: "尝试直接射门",
  43: "尝试长传",
  44: "喜欢脚下球",
  45: "大力任意球",
  46: "反复突破对手",
  47: "喜欢转移球到边路",
  48: "用手抛球发动反击",
  49: "尽可能延长职业生涯",
  50: "擅长大力平抛球",
  51: "经常带球",
  52: "很少带球",
  54: "不拉边",
  55: "喜欢用脚传球",
  56: "避免远射",
  57: "左翼内切",
  58: "右翼内切",
  59: "提前传中",
  60: "带球出防线",
  61: "要求主罚任意球",
  62: "要求主罚点球",
  63: "巅峰退役",
};

const BAND_LABELS_EN: Record<string, string> = {
  门将: "Goalkeepers",
  后卫: "Defenders",
  中场: "Midfielders",
  前锋: "Forwards",
};

/** Chinese country name -> English name, generated from shared/nation-names.csv. */
const NATION_NAMES_EN: Record<string, string> = {
  阿尔及利亚: "Algeria",
  安哥拉: "Angola",
  贝宁: "Benin",
  博茨瓦纳: "Botswana",
  布基纳法索: "Burkina Faso",
  布隆迪: "Burundi",
  喀麦隆: "Cameroon",
  佛得角: "Cape Verde",
  中非共和国: "Central African Republic",
  乍得: "Chad",
  吉布提: "Djibouti",
  埃及: "Egypt",
  赤道几内亚: "Equatorial Guinea",
  埃塞俄比亚: "Ethiopia",
  加蓬: "Gabon",
  冈比亚: "Gambia",
  加纳: "Ghana",
  几内亚: "Guinea",
  几内亚比绍: "Guinea-Bissau",
  科特迪瓦: "Ivory Coast",
  肯尼亚: "Kenya",
  莱索托: "Lesotho",
  利比里亚: "Liberia",
  利比亚: "Libya",
  马达加斯加: "Madagascar",
  马拉维: "Malawi",
  马里: "Mali",
  毛里塔尼亚: "Mauritania",
  毛里求斯: "Mauritius",
  摩洛哥: "Morocco",
  摩纳哥: "Monaco",
  莫桑比克: "Mozambique",
  纳米比亚: "Namibia",
  尼日尔: "Niger",
  尼日利亚: "Nigeria",
  卢旺达: "Rwanda",
  圣多美和普林西比: "Sao Tome & Principe",
  塞内加尔: "Senegal",
  塞舌尔: "Seychelles",
  塞拉利昂: "Sierra Leone",
  索马里: "Somalia",
  南非: "South Africa",
  苏丹: "Sudan",
  斯威士兰: "Eswatini",
  坦桑尼亚: "Tanzania",
  刚果: "Congo",
  多哥: "Togo",
  突尼斯: "Tunisia",
  乌干达: "Uganda",
  刚果共和国: "Congo",
  赞比亚: "Zambia",
  津巴布韦: "Zimbabwe",
  阿富汗: "Afghanistan",
  巴林: "Bahrain",
  孟加拉国: "Bangladesh",
  文莱: "Brunei",
  中国: "China",
  中国香港: "Hong Kong",
  印度: "India",
  印度尼西亚: "Indonesia",
  伊朗: "Iran",
  伊拉克: "Iraq",
  日本: "Japan",
  约旦: "Jordan",
  柬埔寨: "Cambodia",
  哈萨克斯坦: "Kazakhstan",
  科威特: "Kuwait",
  吉尔吉斯斯坦: "Kyrgyzstan",
  老挝: "Laos",
  黎巴嫩: "Lebanon",
  中国澳门: "Macau",
  马来西亚: "Malaysia",
  马尔代夫: "Maldives",
  缅甸: "Myanmar",
  尼泊尔: "Nepal",
  朝鲜: "North Korea",
  阿曼: "Oman",
  巴基斯坦: "Pakistan",
  卡塔尔: "Qatar",
  沙特阿拉伯: "Saudi Arabia",
  新加坡: "Singapore",
  韩国: "South Korea",
  斯里兰卡: "Sri Lanka",
  叙利亚: "Syria",
  中国台北: "Chinese Taipei",
  塔吉克斯坦: "Tajikistan",
  泰国: "Thailand",
  菲律宾: "Philippines",
  土库曼斯坦: "Turkmenistan",
  阿联酋: "United Arab Emirates",
  乌兹别克斯坦: "Uzbekistan",
  越南: "Vietnam",
  也门: "Yemen",
  安提瓜和巴布达: "Antigua & Barbuda",
  阿鲁巴: "Aruba",
  巴巴多斯: "Barbados",
  伯利兹: "Belize",
  百慕大: "Bermuda",
  加拿大: "Canada",
  开曼群岛: "Cayman Islands",
  哥斯达黎加: "Costa Rica",
  古巴: "Cuba",
  多米尼克: "Dominica",
  萨尔瓦多: "El Salvador",
  格林纳达: "Grenada",
  危地马拉: "Guatemala",
  圭亚那: "Guyana",
  海地: "Haiti",
  洪都拉斯: "Honduras",
  牙买加: "Jamaica",
  墨西哥: "Mexico",
  荷属安的列斯: "Netherlands Antilles",
  尼加拉瓜: "Nicaragua",
  巴拿马: "Panama",
  波多黎各: "Puerto Rico",
  圣卢西亚: "St Lucia",
  圣基茨和尼维斯: "St Kitts & Nevis",
  圣文森特和格林纳丁斯: "St Vincent",
  苏里南: "Surinam",
  巴哈马: "Bahamas",
  特立尼达和多巴哥: "Trinidad & Tobago",
  美国: "United States",
  阿尔巴尼亚: "Albania",
  安道尔: "Andorra",
  亚美尼亚: "Armenia",
  奥地利: "Austria",
  阿塞拜疆: "Azerbaijan",
  比利时: "Belgium",
  白俄罗斯: "Belarus",
  波黑: "Bosnia",
  保加利亚: "Bulgaria",
  克罗地亚: "Croatia",
  塞浦路斯: "Cyprus",
  捷克: "Czech Republic",
  丹麦: "Denmark",
  英格兰: "England",
  爱沙尼亚: "Estonia",
  法罗群岛: "Faroe Islands",
  芬兰: "Finland",
  法国: "France",
  格鲁吉亚: "Georgia",
  德国: "Germany",
  希腊: "Greece",
  匈牙利: "Hungary",
  冰岛: "Iceland",
  以色列: "Israel",
  意大利: "Italy",
  拉脱维亚: "Latvia",
  列支敦士登: "Liechtenstein",
  立陶宛: "Lithuania",
  卢森堡: "Luxembourg",
  北马其顿: "North Macedonia",
  马耳他: "Malta",
  摩尔多瓦: "Moldova",
  荷兰: "Netherlands",
  北爱尔兰: "Northern Ireland",
  挪威: "Norway",
  波兰: "Poland",
  葡萄牙: "Portugal",
  爱尔兰: "Ireland",
  罗马尼亚: "Romania",
  俄罗斯: "Russia",
  圣马力诺: "San Marino",
  苏格兰: "Scotland",
  斯洛伐克: "Slovakia",
  斯洛文尼亚: "Slovenia",
  巴斯克地区: "Basque Country",
  西班牙: "Spain",
  瑞典: "Sweden",
  瑞士: "Switzerland",
  土耳其: "Turkey",
  乌克兰: "Ukraine",
  威尔士: "Wales",
  塞尔维亚: "Serbia",
  澳大利亚: "Australia",
  库克群岛: "Cook Islands",
  斐济: "Fiji",
  新西兰: "New Zealand",
  巴布亚新几内亚: "Papua New Guinea",
  所罗门群岛: "Solomon Islands",
  塔希提: "Tahiti",
  汤加: "Tonga",
  瓦努阿图: "Vanuatu",
  萨摩亚: "Samoa",
  阿根廷: "Argentina",
  玻利维亚: "Bolivia",
  巴西: "Brazil",
  智利: "Chile",
  哥伦比亚: "Colombia",
  厄瓜多尔: "Ecuador",
  巴拉圭: "Paraguay",
  秘鲁: "Peru",
  乌拉圭: "Uruguay",
  委内瑞拉: "Venezuela",
  巴勒斯坦: "Palestine",
  美属萨摩亚: "American Samoa",
  蒙古: "Mongolia",
  关岛: "Guam",
  厄立特里亚: "Eritrea",
  安圭拉: "Anguilla",
  英属维尔京群岛: "British Virgin Islands",
  蒙特塞拉特: "Montserrat",
  美属维尔京群岛: "US Virgin Islands",
  特克斯和凯科斯群岛: "Turks and Caicos Islands",
  新喀里多尼亚: "New Caledonia",
  不丹: "Bhutan",
  多米尼加: "Dominican Republic",
  基里巴斯: "Kiribati",
  直布罗陀: "Gibraltar",
  博奈尔: "Bonaire",
  克里米亚: "Crimea",
  科索沃: "Kosovo",
  法属圭亚那: "French Guiana",
  瓜德罗普: "Guadeloupe",
  马提尼克: "Martinique",
  荷属圣马丁: "Sint Maarten",
  法属圣马丁: "Saint-Martin",
  留尼汪: "Reunion",
  马约特: "Mayotte",
  瓦利斯和富图纳: "Wallis & Futuna",
  圣皮埃尔和密克隆: "St Pierre & Miquelon",
  桑给巴尔: "Zanzibar",
  南苏丹: "South Sudan",
  密克罗尼西亚: "Micronesia",
  北马里亚纳群岛: "Northern Mariana Islands",
  圣巴泰勒米: "St Barthélemy",
  科摩罗: "Comoros",
  东帝汶: "East Timor",
  黑山: "Montenegro",
};

export function attributeLabel(key: string, locale: Locale): string {
  if (locale === "en") {
    return ATTRIBUTE_LABELS_EN[key] ?? PERSONALITY_LABELS_EN[key] ?? key;
  }
  return ATTRIBUTE_LABELS[key] ?? PERSONALITY_LABELS_ZH[key] ?? key;
}

export function personalityLabel(key: string, locale: Locale): string {
  return locale === "en"
    ? (PERSONALITY_LABELS_EN[key] ?? key)
    : (PERSONALITY_LABELS_ZH[key] ?? key);
}

export function severityLabel(value: number, locale: Locale): string | null {
  return SEVERITY_LABELS[locale][value] ?? null;
}

export function clauseKindLabel(kind: number, locale: Locale): string | null {
  return CLAUSE_KIND_LABELS[locale][kind] ?? null;
}

/**
 * A clause's value means different things by kind: money for fees and bonuses,
 * a percentage for wage changes and sell-on clauses, years for extension
 * options, and an amount plus a league threshold for landmark bonuses.
 */
export function clauseValueLabel(
  clause: Clause,
  locale: Locale,
  currency: CurrencyCode = SAVE_UNITS,
): string {
  const { kind, parameter, value } = clause;
  if (FLAG_CLAUSE_KINDS.has(kind)) {
    return translate(locale, "inspector.clauseIncluded");
  }
  if (PERCENT_CLAUSE_KINDS.has(kind)) {
    return parameter != null ? `${parameter}%` : "—";
  }
  if (kind === 9) {
    if (parameter != null && value != null) {
      return translate(locale, "inspector.clauseProfitOver", {
        percent: parameter,
        amount: formatValue(value, currency),
      });
    }
    return parameter != null
      ? `${parameter}%`
      : value != null
        ? formatValue(value, currency)
        : "—";
  }
  if (kind === 10 || kind === 40 || kind === 41) {
    const reward = value != null ? formatValue(value, currency) : null;
    const thresholdKey =
      kind === 10
        ? "inspector.clauseGoals"
        : kind === 40
          ? "inspector.clauseAssists"
          : "inspector.clauseGoalAssists";
    const threshold =
      parameter != null
        ? translate(locale, thresholdKey, { count: parameter })
        : null;
    return [reward, threshold].filter(Boolean).join(" · ") || "—";
  }
  if (kind === 11 || kind === 25 || kind === 26) {
    return parameter != null
      ? translate(locale, "inspector.clauseLeagueGames", { count: parameter })
      : "—";
  }
  if (kind === 13) {
    const wage = value != null ? formatWage(value, locale, currency) : null;
    const games =
      parameter != null
        ? translate(locale, "inspector.clauseLeagueGames", { count: parameter })
        : null;
    return [wage, games].filter(Boolean).join(" · ") || "—";
  }
  if (kind === 19) {
    const wage = value != null ? formatWage(value, locale, currency) : null;
    const caps =
      parameter != null
        ? translate(locale, "inspector.clauseCaps", { count: parameter })
        : null;
    return [wage, caps].filter(Boolean).join(" · ") || "—";
  }
  if (kind === 22 || kind === 29) {
    return parameter != null
      ? translate(locale, "inspector.clauseYears", { count: parameter })
      : "—";
  }
  if (kind === 58 && value == null) {
    return translate(locale, "inspector.clauseIncluded");
  }
  if (value != null) return formatValue(value, currency);
  return parameter != null ? String(parameter) : "—";
}

export function roleName(
  id: string | null | undefined,
  fallback: string | null | undefined,
  locale: Locale,
): string | null {
  if (locale === "en" && id) return ROLE_NAMES_EN[id] ?? fallback ?? id;
  return fallback ?? (id ? (ROLE_NAMES_EN[id] ?? id) : null);
}

export function roleGroup(group: string, locale: Locale): string {
  if (locale === "en") return ROLE_GROUPS_EN[group] ?? group;
  return group;
}

export function traitLabel(
  bit: number,
  fallback: string | null | undefined,
  locale: Locale,
): string {
  if (locale === "zh-CN") return TRAIT_LABELS_ZH[bit] ?? fallback ?? "未知习惯";
  return fallback ?? "Unknown trait";
}

export function nationLabel(name: string | null | undefined, locale: Locale): string | null {
  if (name == null) return null;
  if (locale === "en") return NATION_NAMES_EN[name] ?? name;
  return name;
}

/** Position groups and other domain band labels returned by the Rust query layer. */
export function bandLabel(label: string, locale: Locale): string {
  if (locale === "en") return BAND_LABELS_EN[label] ?? label;
  return label;
}

export function genderLabel(gender: number | null | undefined, locale: Locale): string {
  if (gender === 1) return locale === "en" ? "Women" : "女子";
  if (gender === 0) return locale === "en" ? "Men" : "男子";
  return locale === "en" ? "Unknown" : "未知";
}

export function footLabel(left: number, right: number, _locale: Locale): string {
  return `L: ${left} / R: ${right}`;
}

export function ageLabel(age: number | null | undefined, locale: Locale): string {
  const value = age ?? "?";
  return locale === "en" ? `${value} yrs` : `${value} 岁`;
}

export function playersLabel(count: number, locale: Locale): string {
  return locale === "en" ? `${count} players` : `${count} 名球员`;
}
