import type { TranslationKey } from "./i18n";

export type TransferValueState = "ok" | "unset" | "placeholder" | "zero";

export interface Attributes {
  [key: string]: number;
}

export interface Positions {
  gk: number;
  sw: number;
  dl: number;
  dc: number;
  dr: number;
  dm: number;
  ml: number;
  mc: number;
  mr: number;
  aml: number;
  amc: number;
  amr: number;
  stc: number;
  wbl: number;
  wbr: number;
}

export interface Personality {
  adaptability: number;
  ambition: number;
  loyalty: number;
  pressure: number;
  professionalism: number;
  sportsmanship: number;
  temperament: number;
  controversy: number;
}

export const PERSONALITY_KEYS = [
  "adaptability",
  "ambition",
  "loyalty",
  "pressure",
  "professionalism",
  "sportsmanship",
  "temperament",
  "controversy",
] as const;

export interface Trait {
  bit: number;
  name: string | null;
}

export interface TraitOption {
  bit: number;
  name: string;
}

export interface AttributeEdit {
  key: string;
  value: number;
}

export interface PlayerEditRequest {
  ca: number | null;
  pa: number | null;
  attributes: AttributeEdit[];
  personality: AttributeEdit[];
  traits: number[];
  untraits: number[];
  wage: number | null;
  contract_end: string | null;
}

export interface EditChange {
  field: string;
  before: number;
  after: number;
  before_display?: string | null;
  after_display?: string | null;
}

export interface EditPlayerResult {
  output: string;
  file_size: number;
  changes: EditChange[];
  warnings: string[];
}

export interface PlayerEditBatch {
  uid: number;
  request: PlayerEditRequest;
}

export interface BatchEditResult {
  output: string;
  file_size: number;
  players: number;
  changes: number;
}

export interface CaEstimateRequest {
  /** Internal 1–100 attribute values keyed by attribute name. */
  attributes: Record<string, number>;
  /** Full 15-position familiarity table (0–20). */
  positions: Positions;
  /** Optional single position; omitted means the best position. */
  position?: string | null;
  /** Internal 1–100 foot values. */
  left_foot: number;
  right_foot: number;
}

export interface CaEstimate {
  position: string;
  estimated_ca: number;
  weighted_average: number;
  excess_average: number;
  total_weight: number;
  position_cost: number;
  initial_ca: number;
  iterations: number;
  left_foot: number;
  right_foot: number;
}

export interface RoleScore {
  id: string;
  name: string;
  group: string;
  score: number;
  position: string;
  position_rating: number;
}

export interface Injury {
  kind: "current" | "history";
  date: string | null;
  expected_return: string | null;
  type_id: number | null;
  type_name: string | null;
  cause: number | null;
  severity: number | null;
  team_id: number | null;
  club_uid: number | null;
  club_name: string | null;
}

export interface Suspension {
  scope: "competition" | "nation" | "unknown";
  scope_code: number;
  competition_id: number | null;
  competition_name: string | null;
  nation_id: number | null;
  nation_name: string | null;
  issued_date: string | null;
}

export interface SnapshotPoint {
  game_date: string;
  ca: number | null;
  pa: number | null;
  transfer_value: number | null;
  club_uid: number | null;
  club_name: string | null;
  wage: number | null;
  contract_end: string | null;
}

export interface Clause {
  kind: number;
  parameter: number | null;
  value: number | null;
}

export interface ContractChainEntry {
  club_uid: number | null;
  club_name: string | null;
  team_id: number;
  wage: number;
  start: string | null;
  end: string | null;
  has_terms: boolean;
}

export interface Contract {
  club_uid: number | null;
  club_name: string | null;
  team_id: number | null;
  wage: number | null;
  start: string | null;
  end: string | null;
  squad_status: number | null;
  contract_type: number | null;
  transfer_listed: boolean;
  loan_listed: boolean;
  clauses: Clause[];
  on_loan: boolean | null;
  loan_parent_club_uid: number | null;
  loan_parent_club_name: string | null;
  loan_start: string | null;
  loan_end: string | null;
  chain: ContractChainEntry[];
}

export interface Reputation {
  bucket: number;
  home: number;
  current: number;
  world: number;
}

export interface Player {
  uid: number;
  unique_id: number | null;
  /** True when the game generated this player after the save started. */
  generated: boolean;
  name: string | null;
  first_name: string | null;
  last_name: string | null;
  common_name: string | null;
  full_name: string | null;
  legal_name: string | null;
  birth_date: string | null;
  age: number | null;
  nation_id: number | null;
  nation_name: string | null;
  gender: number | null;
  second_nation_ids: number[];
  /** Resolved names aligned with second_nation_ids; null when the save-local id is unnamed. */
  second_nation_names: (string | null)[];
  home_grown_nation_ids: number[];
  home_grown_club_uids: (number | null)[];
  home_grown_club_names: (string | null)[];
  height_cm: number;
  ca: number;
  pa: number | null;
  pa_range_code: number | null;
  reputation: Reputation;
  club_uid: number | null;
  club_name: string | null;
  club_short_name: string | null;
  club_nation_id: number | null;
  club_nation_name: string | null;
  club_fa_nation_id: number | null;
  club_reputation: number | null;
  club_last_league_position: number | null;
  league_competition_id: number | null;
  league_name: string | null;
  team_id: number | null;
  team_slot: number | null;
  club_join_date: string | null;
  natural_positions: string[];
  accomplished_positions: string[];
  personality: Personality | null;
  attributes: Attributes;
  raw_attributes: Attributes;
  left_foot: number;
  right_foot: number;
  raw_left_foot: number;
  raw_right_foot: number;
  positions: Positions;
  transfer_value: number | null;
  transfer_value_state: TransferValueState;
  transfer_listed: boolean;
  loan_listed: boolean;
  raw_condition: number;
  raw_match_sharpness: number;
  traits: Trait[];
  trait_bits: number | null;
  best_role: RoleScore | null;
  injuries: Injury[];
  suspensions: Suspension[];
  on_loan: boolean | null;
  loan_parent_club_uid: number | null;
  loan_parent_club_name: string | null;
  loan_start: string | null;
  loan_end: string | null;
  contract: Contract | null;
}

export interface PlayerRow {
  uid: number;
  generated: boolean;
  name: string | null;
  age: number | null;
  club_uid: number | null;
  club_name: string | null;
  league_competition_id: number | null;
  league_name: string | null;
  nation_id: number | null;
  nation_name: string | null;
  gender: number | null;
  ca: number;
  pa: number | null;
  pa_range_code: number | null;
  positions_text: string | null;
  positions: Positions;
  attributes: Attributes;
  personality: Personality | null;
  transfer_value: number | null;
  transfer_value_state: TransferValueState;
  wage: number | null;
  contract_end: string | null;
  height_cm: number | null;
  left_foot: number | null;
  right_foot: number | null;
  on_loan: boolean | null;
  transfer_listed: boolean;
  loan_listed: boolean;
  traits: Trait[];
  best_role_id: string | null;
  best_role_name: string | null;
  best_role_score: number | null;
  best_role_position: string | null;
  injury_status: "injured" | "fit";
  injury_until: string | null;
  recent_injury: string | null;
  suspension_count: number;
}

export interface SortRule {
  by: string;
  dir: "asc" | "desc";
}

export interface PlayerQuery {
  save_id: number;
  search?: string | null;
  uids?: number[];
  club_uids?: number[];
  nation_ids?: number[];
  genders?: number[];
  availability?: string | null;
  listings?: string[];
  age_min?: number | null;
  age_max?: number | null;
  ca_min?: number | null;
  ca_max?: number | null;
  pa_min?: number | null;
  pa_max?: number | null;
  pa_ca_gap_min?: number | null;
  /** `true` keeps only system-generated players, `false` keeps only database players. */
  generated?: boolean | null;
  positions?: string[];
  position_min?: number | null;
  attributes?: { key: string; min?: number | null; max?: number | null }[];
  value_min?: number | null;
  value_max?: number | null;
  wage_min?: number | null;
  wage_max?: number | null;
  contract_end_before?: string | null;
  contract_end_after?: string | null;
  on_loan?: boolean | null;
  include_hidden?: boolean;
  sort?: SortRule[];
  sort_by?: string | null;
  sort_dir?: "asc" | "desc" | null;
  page?: number | null;
  page_size?: number | null;
}

export interface PlayerPage {
  total: number;
  page: number;
  page_size: number;
  rows: PlayerRow[];
}

/** Staff profile attributes, keyed by `STAFF_ATTRIBUTE_KEYS`, on the 1–20 scale. */
export type StaffAttributes = Record<string, number>;

export interface StaffRow {
  uid: number;
  unique_id: number | null;
  name: string | null;
  age: number | null;
  club_uid: number | null;
  club_name: string | null;
  nation_id: number | null;
  nation_name: string | null;
  job_code: number | null;
  job_name: string | null;
  ca: number | null;
  pa: number | null;
  wage: number | null;
  contract_start: string | null;
  contract_end: string | null;
  has_contract: boolean;
  attributes: StaffAttributes | null;
  roles: number[] | null;
  best_role_index: number | null;
  best_role_name: string | null;
  best_role_score: number | null;
  personality: Personality | null;
}

export interface StaffRoleFilter {
  key: string;
  min?: number | null;
  max?: number | null;
}

export interface StaffQuery {
  save_id: number;
  search?: string | null;
  uids?: number[];
  club_uids?: number[];
  nation_ids?: number[];
  job_codes?: number[];
  age_min?: number | null;
  age_max?: number | null;
  ca_min?: number | null;
  ca_max?: number | null;
  pa_min?: number | null;
  pa_max?: number | null;
  wage_min?: number | null;
  wage_max?: number | null;
  contract_end_before?: string | null;
  contract_end_after?: string | null;
  has_contract?: boolean | null;
  attributes?: { key: string; min?: number | null; max?: number | null }[];
  roles?: StaffRoleFilter[];
  sort?: SortRule[];
  sort_by?: string | null;
  sort_dir?: "asc" | "desc" | null;
  page?: number | null;
  page_size?: number | null;
}

export interface StaffPage {
  total: number;
  page: number;
  page_size: number;
  rows: StaffRow[];
}

export interface StaffFilterOptions {
  clubs: FilterOption[];
  nations: FilterOption[];
  jobs: FilterOption[];
  min_age: number | null;
  max_age: number | null;
  min_ca: number | null;
  max_ca: number | null;
  min_pa: number | null;
  max_pa: number | null;
  min_wage: number | null;
  max_wage: number | null;
  with_contract: number;
  without_contract: number;
  with_attributes: number;
  with_roles: number;
}

export interface ValueFinderRequest {
  save_id: number;
  mode?: "full" | "fair_play" | "moneyball" | null;
  position_group?: string | null;
  league?: string | null;
  age_min?: number | null;
  age_max?: number | null;
  ca_min?: number | null;
  ca_max?: number | null;
  role_min?: number | null;
  max_fee?: number | null;
  max_wage?: number | null;
  contract_end_before?: string | null;
  listings?: string[];
  min_confidence?: number | null;
  exclude_injured?: boolean;
  exclude_injury_history?: boolean;
  include_unknown_fee?: boolean;
  sort_by?: string | null;
  sort_dir?: "asc" | "desc" | null;
  page?: number | null;
  page_size?: number | null;
}

export interface ValueFinderRow {
  uid: number;
  name: string | null;
  age: number | null;
  club_uid: number | null;
  club_name: string | null;
  league_name: string | null;
  nation_name: string | null;
  ca: number;
  pa: number | null;
  role_id: string | null;
  role_name: string | null;
  role_score: number | null;
  role_position: string | null;
  position_group: string;
  transfer_value: number | null;
  transfer_value_state: TransferValueState;
  wage: number | null;
  contract_end: string | null;
  club_reputation: number | null;
  fee: number | null;
  cost: number;
  fee_known: boolean;
  wage_known: boolean;
  role_pct: number | null;
  ca_pct: number | null;
  quality_pct: number;
  cost_pct: number | null;
  surplus: number | null;
  score: number;
  confidence: number;
  league_median_ca: number | null;
  flags: string[];
  transfer_listed: boolean;
  loan_listed: boolean;
  injury_status: "injured" | "fit";
  recent_injury: string | null;
  injury_recent: boolean;
}

export interface ValueFinderMeta {
  model: string;
  mode: string;
  scored: number;
  filtered: number;
  cost_horizon_years: number;
  leagues: string[];
}

export interface ValueFinderPage {
  total: number;
  page: number;
  page_size: number;
  rows: ValueFinderRow[];
  meta: ValueFinderMeta;
}

export interface StatsMoneyballRequest {
  save_id: number;
  model?: "core" | "advanced" | null;
  window?: "overall" | "league" | "calendar" | null;
  mode?: "full" | "fair_play" | "moneyball" | null;
  include_goalkeepers?: boolean;
  position_group?: string | null;
  league?: string | null;
  age_min?: number | null;
  age_max?: number | null;
  ca_min?: number | null;
  ca_max?: number | null;
  min_minutes?: number | null;
  max_fee?: number | null;
  max_wage?: number | null;
  contract_end_before?: string | null;
  listings?: string[];
  min_confidence?: number | null;
  exclude_injured?: boolean;
  exclude_injury_history?: boolean;
  include_unknown_fee?: boolean;
  sort_by?: string | null;
  sort_dir?: "asc" | "desc" | null;
  page?: number | null;
  page_size?: number | null;
}

export interface StatsDriver {
  key: string;
  label: string;
  percentile: number;
  contribution: number;
}

export interface StatsMoneyballRow {
  uid: number;
  name: string | null;
  age: number | null;
  club_uid: number | null;
  club_name: string | null;
  league_name: string | null;
  nation_name: string | null;
  ca: number;
  pa: number | null;
  position_group: string;
  minutes: number;
  starts: number;
  substitute_appearances: number;
  average_rating: number | null;
  goals: number;
  assists: number;
  expected_goals: number;
  non_penalty_expected_goals: number;
  expected_assists: number;
  shots: number;
  shots_on_target: number;
  passes_attempted: number;
  passes_completed: number;
  pass_completion_percent: number | null;
  tackles_attempted: number;
  tackles_completed: number;
  interceptions: number | null;
  key_passes: number | null;
  progressive_passes: number | null;
  headers_won: number | null;
  aerial_challenges_attempted: number | null;
  clearances: number | null;
  blocks: number | null;
  crosses_completed: number | null;
  saves_held: number | null;
  shots_on_target_faced: number | null;
  save_percent: number | null;
  expected_goals_prevented: number | null;
  advanced_available: boolean;
  transfer_value: number | null;
  transfer_value_state: TransferValueState;
  wage: number | null;
  contract_end: string | null;
  club_reputation: number | null;
  fee: number | null;
  cost: number;
  fee_known: boolean;
  wage_known: boolean;
  production_global: number | null;
  production_tier: number | null;
  production_low: number | null;
  production_high: number | null;
  cost_pct_global: number | null;
  cost_pct_tier: number | null;
  surplus: number | null;
  score: number | null;
  score_low: number | null;
  fair_price: number | null;
  value_gap_pct: number | null;
  bargain_score: number | null;
  peer_count: number;
  confidence: number;
  flags: string[];
  drivers: StatsDriver[];
  transfer_listed: boolean;
  loan_listed: boolean;
  injury_status: "injured" | "fit";
  recent_injury: string | null;
  injury_recent: boolean;
  stats_from_other_club: boolean;
}

export interface SimilarStatsPlayer {
  uid: number;
  name: string | null;
  age: number | null;
  club_name: string | null;
  position_group: string;
  minutes: number;
  quality: number | null;
  fee: number | null;
  fair_price: number | null;
  value_gap_pct: number | null;
  distance: number;
}

export interface SquadNeedsRequest {
  save_id: number;
  club_uid: number;
  model?: "core" | "advanced" | null;
  window?: "overall" | "league" | "calendar" | null;
  min_minutes?: number | null;
}

export interface SquadUpgrade {
  uid: number;
  name: string | null;
  age: number | null;
  club_name: string | null;
  minutes: number;
  quality: number | null;
  quality_gain: number | null;
  fee: number | null;
  fair_price: number | null;
  value_gap_pct: number | null;
  bargain_score: number | null;
}

export interface SquadNeed {
  position_group: string;
  best_quality: number | null;
  best_uid: number | null;
  best_name: string | null;
  upgrades: SquadUpgrade[];
}

export interface SquadNeeds {
  club_uid: number;
  club_name: string | null;
  needs: SquadNeed[];
}

export interface StatsMoneyballMeta {
  model: string;
  scored: number;
  filtered: number;
  min_minutes: number;
  score_floor_minutes: number;
  cost_horizon_years: number;
  leagues: string[];
  window: string;
  mode: string;
  price_model: string | null;
  priced: number;
}

export interface StatsMoneyballPage {
  total: number;
  page: number;
  page_size: number;
  rows: StatsMoneyballRow[];
  meta: StatsMoneyballMeta;
}

export interface Club {
  uid: number;
  unique_id: number | null;
  name: string;
  short_name: string;
  nation_id: number;
  nation_name: string | null;
  fa_nation_id: number;
  fa_nation_name: string | null;
  city_id: number | null;
  gender: number | null;
  reputation: number | null;
  last_league_position: number | null;
  league_competition_id: number | null;
  league_name: string | null;
  parent_club_uid: number | null;
  parent_club_name: string | null;
  corporate_facilities: number | null;
  training_facilities: number | null;
  youth_facilities: number | null;
  junior_coaching: number | null;
  youth_recruitment: number | null;
  /** Optional byte displayed as Youth Level; not editable. */
  youth_level: number | null;
  teams: number[];
}

export interface FinanceMonth {
  club_uid: number;
  club_name: string;
  month: string;
  balance: number;
  transfer_budget_allocated: number;
  transfer_budget_remaining: number;
  wage_budget_weekly: number;
  wage_payroll_weekly: number;
  income_excluding_transfers: number;
  net_transfers: number;
  wage_bill: number;
  net: number;
  expenditure_excluding_transfers: number;
  total_income: number;
  total_expenditure: number;
}

export interface Stadium {
  uid: number;
  name: string | null;
  all_seater_capacity: number;
  expansion_capacity: number;
  capacity: number | null;
  owner_club_uid: number | null;
  owner_club_name: string | null;
  pitch_length_dm: number;
  pitch_width_dm: number;
  pitch_min_length_dm: number;
  pitch_min_width_dm: number;
  pitch_max_length_dm: number;
  pitch_max_width_dm: number;
  built_date: string | null;
  rebuilt_date: string | null;
}

export interface ClubLink {
  uid: number;
  name: string;
  short_name: string;
}

export interface ClubDetail {
  club: Club;
  finance_months: FinanceMonth[];
  stadiums: Stadium[];
  child_clubs: ClubLink[];
}

export interface ClubRow {
  uid: number;
  unique_id: number | null;
  name: string;
  short_name: string;
  nation_id: number;
  nation_name: string | null;
  fa_nation_id: number;
  fa_nation_name: string | null;
  city_id: number | null;
  gender: number | null;
  reputation: number | null;
  last_league_position: number | null;
  league_competition_id: number | null;
  league_name: string | null;
  parent_club_uid: number | null;
  parent_club_name: string | null;
  corporate_facilities: number | null;
  finance_month: string | null;
  balance: number | null;
  transfer_budget_allocated: number | null;
  transfer_budget_remaining: number | null;
  wage_budget_weekly: number | null;
  wage_payroll_weekly: number | null;
  net: number | null;
  player_count: number;
  average_age: number | null;
  average_ca: number | null;
  total_value: number | null;
  total_wage: number | null;
  stadium_count: number;
  stadium_capacity: number | null;
}

export interface ClubPage {
  total: number;
  page: number;
  page_size: number;
  rows: ClubRow[];
}

export interface ClubQuery {
  save_id: number;
  search?: string | null;
  nation_ids?: number[];
  genders?: number[];
  reputation_min?: number | null;
  reputation_max?: number | null;
  last_position_min?: number | null;
  last_position_max?: number | null;
  has_finances?: boolean | null;
  balance_min?: number | null;
  balance_max?: number | null;
  transfer_budget_min?: number | null;
  transfer_budget_max?: number | null;
  wage_budget_min?: number | null;
  wage_budget_max?: number | null;
  corporate_facilities_min?: number | null;
  corporate_facilities_max?: number | null;
  training_facilities_min?: number | null;
  training_facilities_max?: number | null;
  youth_facilities_min?: number | null;
  youth_facilities_max?: number | null;
  junior_coaching_min?: number | null;
  junior_coaching_max?: number | null;
  youth_recruitment_min?: number | null;
  youth_recruitment_max?: number | null;
  player_count_min?: number | null;
  player_count_max?: number | null;
  average_ca_min?: number | null;
  average_ca_max?: number | null;
  total_value_min?: number | null;
  total_value_max?: number | null;
  has_stadium?: boolean | null;
  stadium_capacity_min?: number | null;
  sort?: SortRule[];
  sort_by?: string | null;
  sort_dir?: "asc" | "desc" | null;
  page?: number | null;
  page_size?: number | null;
}

export interface ClubFilterOptions {
  nations: FilterOption[];
  genders: FilterOption[];
  min_reputation: number | null;
  max_reputation: number | null;
  min_balance: number | null;
  max_balance: number | null;
  min_transfer_budget: number | null;
  max_transfer_budget: number | null;
  min_wage_budget: number | null;
  max_wage_budget: number | null;
  clubs_with_finances: number;
  clubs_with_stadiums: number;
}

export interface ClubFinanceEdit {
  balance: number | null;
  transfer_budget_remaining: number | null;
  wage_budget_weekly: number | null;
}

export interface StadiumCapacityEdit {
  stadium_uid: number;
  all_seater_capacity: number | null;
  expansion_capacity: number | null;
}

export interface ClubEditRequest {
  reputation: number | null;
  corporate_facilities: number | null;
  training_facilities: number | null;
  youth_facilities: number | null;
  junior_coaching: number | null;
  youth_recruitment: number | null;
  finances: ClubFinanceEdit | null;
  stadiums: StadiumCapacityEdit[];
}

export interface EditClubResult {
  output: string;
  file_size: number;
  changes: EditChange[];
  warnings: string[];
}

export interface SaveRecord {
  id: number;
  path: string;
  file_name: string;
  game: string;
  build: string;
  game_date: string | null;
  parser_version: string;
  parsed_at: string;
  players: number;
  staff: number;
  clubs: number;
  contracts: number;
}

export interface ManagedClub {
  club_uid: number;
  club_name: string;
  manager_name: string | null;
}

export interface SaveSummary {
  path: string;
  file_name: string;
  file_size: number;
  mtime_ns: number;
  fingerprint: string;
  save_name: string;
  game: string;
  build: string;
  game_date: string | null;
  db_version: string;
  section_count: number;
  decompressed_bytes: number;
}

export interface Competition {
  id: number;
  database_id: number | null;
  name: string | null;
  stage_ids: number[];
}

export interface ParseReport {
  save: SaveSummary;
  players: number;
  staff: number;
  staff_with_attributes: number;
  staff_with_roles: number;
  clubs: number;
  season_stats: number;
  contracts: number;
  competitions: number;
  competition_stages: number;
  competitions_with_database_id: number;
  competitions_with_name: number;
  club_finance_months: number;
  clubs_with_finance: number;
  stadiums: number;
  clubs_with_corporate_facilities: number;
  markerless_players: number;
  players_with_person: number;
  players_with_club: number;
  players_with_contract: number;
  elapsed_ms: number;
  warnings: string[];
}

/** Matches `store::UNKNOWN_GENDER_ID`; used for players without a club gender. */
export const UNKNOWN_GENDER_ID = 255;

export interface FilterOption {
  id: number;
  name: string;
  count: number;
}

export interface FilterOptions {
  clubs: FilterOption[];
  nations: FilterOption[];
  genders: FilterOption[];
  availability: FilterOption[];
  listing: FilterOption[];
  min_age: number | null;
  max_age: number | null;
  min_ca: number | null;
  max_ca: number | null;
  min_pa: number | null;
  max_pa: number | null;
  min_value: number | null;
  max_value: number | null;
  min_wage: number | null;
  max_wage: number | null;
}

export interface DistributionBand {
  label: string;
  count: number;
}

export interface ClubOverview {
  club_uid: number;
  club_name: string;
  players: number;
  avg_ca: number;
  max_ca: number;
  avg_age: number;
  total_value: number;
}

export interface AnalysisOverview {
  total_players: number;
  total_clubs: number;
  average_age: number;
  median_ca: number;
  total_value: number;
  free_agents: number;
  loanees: number;
  expiring_contracts: number;
  game_date: string | null;
  age_bands: DistributionBand[];
  ca_bands: DistributionBand[];
  contract_years: DistributionBand[];
  top_clubs: ClubOverview[];
}

export interface SquadAnalysis {
  club_uid: number;
  club_name: string;
  player_count: number;
  average_age: number;
  average_ca: number;
  average_pa: number;
  total_wage: number;
  total_value: number;
  loanees: number;
  position_groups: DistributionBand[];
  age_bands: DistributionBand[];
  contract_years: DistributionBand[];
  players: PlayerRow[];
}

export type ScoutPurpose = "firstTeam" | "rotation" | "future" | "any";
export type ScoutDepthStatus = "missing" | "thin" | "fine" | "crowded";
export type ScoutDepartureAction = "sell" | "loan" | "release";

export interface ScoutPlayer {
  uid: number;
  name: string | null;
  age: number | null;
  club_uid: number | null;
  club_name: string | null;
  ca: number;
  pa: number | null;
  transfer_value: number | null;
  wage: number | null;
  contract_end: string | null;
  transfer_listed: boolean;
}

export interface ScoutFitting {
  player: ScoutPlayer;
  fit: number;
  role_id: string;
  role_name: string;
  role_score: number;
  position_rating: number;
}

export interface ScoutReason {
  code: string;
  points: number;
  value: number | null;
}

export interface ScoutSuggestion extends ScoutFitting {
  probability: number;
  reasons: ScoutReason[];
  fee: number | null;
  wage: number | null;
  value: number | null;
  score: number;
  owner_club: string | null;
  owner_club_uid: number | null;
}

export interface Scouted {
  suggestions: ScoutSuggestion[];
  squad: ScoutFitting[];
  transfer_budget: number | null;
  wage_budget: number | null;
  wage_bill: number | null;
  wage_room: number | null;
  examined: number;
  candidates: number;
  eligible: number;
  game_date: string | null;
}

export interface ScoutRoleInfo {
  id: string;
  name: string;
  positions: string[];
}

export interface ScoutDepthPlayer {
  player: ScoutPlayer;
  rating: number;
  on_loan: boolean;
}

export interface ScoutLine {
  key: string;
  positions: string[];
  target: number;
  players: ScoutDepthPlayer[];
  cover: ScoutDepthPlayer[];
  status: ScoutDepthStatus;
  weak: boolean;
  ageing: boolean;
}

export interface ScoutDepartureReason {
  code: string;
  value: number | null;
}

export interface ScoutDeparture {
  player: ScoutPlayer;
  action: ScoutDepartureAction;
  reasons: ScoutDepartureReason[];
  priority: number;
  value: number | null;
  not_for_sale: boolean;
  wage: number | null;
  line: string;
  rank: number;
}

export interface ScoutOverview {
  club_uid: number;
  club_name: string;
  transfer_budget: number | null;
  wage_budget: number | null;
  wage_bill: number | null;
  wage_room: number | null;
  squad_size: number;
  average_age: number | null;
  level: number;
  lines: ScoutLine[];
  departures: ScoutDeparture[];
  positions: string[];
  roles: ScoutRoleInfo[];
}

export interface ScoutRequest {
  save_id: number;
  club_uid: number;
  position: string;
  role_id?: string | null;
  purpose?: ScoutPurpose;
  min_age?: number | null;
  max_age?: number | null;
  max_fee?: number | null;
  min_probability?: number | null;
  no_free_agents?: boolean;
  how_many?: number | null;
}

export interface OpenSaveResult {
  save_id: number;
  cached: boolean;
  summary: SaveSummary;
  report: ParseReport | null;
}

export interface ParseProgress {
  stage: string;
  current: number;
  total: number;
  message: string;
}

export interface ExportReport {
  path: string;
  format: string;
  rows: number;
}

export const POSITION_KEYS = [
  "gk",
  "sw",
  "dl",
  "dc",
  "dr",
  "dm",
  "ml",
  "mc",
  "mr",
  "aml",
  "amc",
  "amr",
  "stc",
  "wbl",
  "wbr",
] as const;

export const POSITION_LABELS: Record<string, string> = {
  gk: "GK",
  sw: "SW",
  dl: "DL",
  dc: "DC",
  dr: "DR",
  dm: "DM",
  ml: "ML",
  mc: "MC",
  mr: "MR",
  aml: "AML",
  amc: "AMC",
  amr: "AMR",
  stc: "ST",
  wbl: "WBL",
  wbr: "WBR",
};

export type AttributeGroupKey =
  | "technical"
  | "mental"
  | "physical"
  | "goalkeeping"
  | "hidden";

export const ATTRIBUTE_GROUPS: Record<string, { labelKey: AttributeGroupKey; keys: string[] }[]> = {
  technical: [
    {
      labelKey: "technical",
      keys: [
        "corners",
        "crossing",
        "dribbling",
        "finishing",
        "first_touch",
        "free_kick_taking",
        "heading",
        "long_shots",
        "long_throws",
        "marking",
        "passing",
        "penalty_taking",
        "tackling",
        "technique",
      ],
    },
    {
      labelKey: "mental",
      keys: [
        "aggression",
        "anticipation",
        "bravery",
        "composure",
        "concentration",
        "decisions",
        "determination",
        "flair",
        "leadership",
        "off_the_ball",
        "positioning",
        "teamwork",
        "vision",
        "work_rate",
      ],
    },
    {
      labelKey: "physical",
      keys: [
        "acceleration",
        "agility",
        "balance",
        "jumping_reach",
        "natural_fitness",
        "pace",
        "stamina",
        "strength",
      ],
    },
  ],
  goalkeeping: [
    {
      labelKey: "goalkeeping",
      keys: [
        "aerial_reach",
        "command_of_area",
        "communication",
        "eccentricity",
        "handling",
        "kicking",
        "one_on_ones",
        "punching",
        "reflexes",
        "rushing_out",
        "throwing",
      ],
    },
  ],
  hidden: [
    {
      labelKey: "hidden",
      keys: [
        "consistency",
        "dirtiness",
        "important_matches",
        "injury_proneness",
        "versatility",
      ],
    },
  ],
};

export const HIDDEN_ATTRIBUTE_KEYS = ATTRIBUTE_GROUPS.hidden[0].keys;
export const CA_EXEMPT_ATTRIBUTE_KEYS: ReadonlySet<string> = new Set([
  ...HIDDEN_ATTRIBUTE_KEYS,
  "determination",
  "natural_fitness",
  "flair",
  "aggression",
]);
export const HIDDEN_FILTER_KEYS = [...HIDDEN_ATTRIBUTE_KEYS, ...PERSONALITY_KEYS];
export const HIDDEN_FILTERS: ReadonlySet<string> = new Set(HIDDEN_FILTER_KEYS);

function filterGroupLabelKey(key: AttributeGroupKey): TranslationKey {
  return `attribute.group.${key}`;
}

export const ATTRIBUTE_FILTER_GROUPS: {
  labelKey: TranslationKey;
  keys: readonly string[];
}[] = [
  ...Object.values(ATTRIBUTE_GROUPS)
    .flat()
    .map((group) => ({
      labelKey: filterGroupLabelKey(group.labelKey),
      keys: group.keys,
    })),
  { labelKey: "attribute.group.personality", keys: [...PERSONALITY_KEYS] },
];

export const ATTRIBUTE_LABELS: Record<string, string> = {
  crossing: "传中",
  dribbling: "盘带",
  finishing: "射门",
  heading: "头球",
  long_shots: "远射",
  marking: "盯人",
  off_the_ball: "无球跑动",
  passing: "传球",
  penalty_taking: "点球",
  tackling: "抢断",
  vision: "视野",
  handling: "手控球",
  aerial_reach: "制空",
  command_of_area: "指挥防守",
  communication: "交流",
  kicking: "开球",
  throwing: "手抛球",
  anticipation: "预判",
  decisions: "决断",
  one_on_ones: "一对一",
  positioning: "位置感",
  reflexes: "反应",
  first_touch: "停球",
  technique: "技术",
  flair: "才华",
  corners: "角球",
  teamwork: "团队合作",
  work_rate: "工作投入",
  long_throws: "长界外球",
  eccentricity: "神经刀",
  rushing_out: "出击",
  punching: "击球",
  acceleration: "加速",
  free_kick_taking: "任意球",
  strength: "力量",
  stamina: "耐力",
  pace: "速度",
  jumping_reach: "弹跳",
  leadership: "领导力",
  dirtiness: "肮脏",
  balance: "平衡",
  bravery: "勇敢",
  consistency: "稳定",
  aggression: "侵略性",
  agility: "敏捷",
  important_matches: "大赛",
  injury_proneness: "受伤倾向",
  versatility: "多面性",
  natural_fitness: "体质",
  determination: "意志力",
  composure: "冷静",
  concentration: "专注",
};

export const ATTRIBUTE_FILTER_KEYS = ATTRIBUTE_FILTER_GROUPS.flatMap((group) => group.keys);

/** Staff profile attributes, in the order the parser stores them. */
export const STAFF_ATTRIBUTE_KEYS = [
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
] as const;

export const STAFF_ATTRIBUTE_GROUPS: { labelKey: TranslationKey; keys: readonly string[] }[] = [
  {
    labelKey: "staff.group.profile",
    keys: [
      "authority",
      "determination",
      "working_with_youngsters",
      "motivating",
      "people_management",
    ],
  },
  {
    labelKey: "staff.group.judging",
    keys: [
      "judging_player_ability",
      "judging_player_potential",
      "judging_staff_ability",
    ],
  },
  {
    labelKey: "staff.group.coaching",
    keys: [
      "attacking",
      "defending",
      "possession",
      "technical",
      "tactical",
      "set_pieces",
      "tactical_knowledge",
      "fitness",
      "goalkeeping",
    ],
  },
  {
    labelKey: "staff.group.support",
    keys: ["data_analysis", "sports_science", "negotiating", "physiotherapy"],
  },
];

/** Role-suitability vector order used by the save and the cache columns. */
export const STAFF_ROLE_KEYS = [
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
] as const;

/** A role score at or above this value is a realistic appointment. */
export const STAFF_ROLE_GREEN_THRESHOLD = 10;
