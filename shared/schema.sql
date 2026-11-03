-- FM27 Scout local cache schema.
-- Only the Rust parser writes here. All parsing happens from a read-only save copy.

CREATE TABLE IF NOT EXISTS saves (
  id              INTEGER PRIMARY KEY,
  path            TEXT NOT NULL,
  file_name       TEXT NOT NULL,
  file_size       INTEGER NOT NULL,
  mtime_ns        INTEGER NOT NULL,
  fingerprint     TEXT NOT NULL,
  game            TEXT NOT NULL,
  build           TEXT NOT NULL,
  save_name       TEXT NOT NULL DEFAULT '',
  career_key      TEXT NOT NULL DEFAULT '',
  game_date       TEXT,
  db_version      TEXT,
  parser_version  TEXT NOT NULL,
  parsed_at       TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS saves_fingerprint
  ON saves(fingerprint, parser_version);

CREATE TABLE IF NOT EXISTS players (
  save_id         INTEGER NOT NULL,
  uid             INTEGER NOT NULL,
  unique_id       INTEGER,
  name            TEXT,
  first_name      TEXT,
  last_name       TEXT,
  common_name     TEXT,
  birth_date      TEXT,
  age             INTEGER,
  nation_id       INTEGER,
  nation_name     TEXT,
  gender          INTEGER,
  club_uid        INTEGER,
  club_name       TEXT,
  league_competition_id INTEGER,
  league_name     TEXT,
  ca              INTEGER,
  pa              INTEGER,
  pa_range_code   INTEGER,
  height_cm       INTEGER,
  left_foot       INTEGER,
  right_foot      INTEGER,
  transfer_value  INTEGER,
  transfer_value_state TEXT,
  positions_json  TEXT NOT NULL,
  positions_text  TEXT,
  personality_json TEXT,
  traits_json     TEXT,
  attributes_json TEXT NOT NULL,
  raw_attributes_json TEXT NOT NULL,
  best_role_id    TEXT,
  best_role_name  TEXT,
  best_role_score INTEGER,
  best_role_position TEXT,
  injury_status   TEXT NOT NULL DEFAULT 'fit',
  injury_until    TEXT,
  recent_injury   TEXT,
  suspension_count INTEGER NOT NULL DEFAULT 0,
  injuries_json   TEXT NOT NULL DEFAULT '[]',
  suspensions_json TEXT NOT NULL DEFAULT '[]',
  contract_end    TEXT,
  wage            INTEGER,
  on_loan         INTEGER,
  transfer_listed INTEGER NOT NULL DEFAULT 0,
  loan_listed     INTEGER NOT NULL DEFAULT 0,
  data_json       TEXT NOT NULL,
  PRIMARY KEY (save_id, uid),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS players_name ON players(save_id, name);
CREATE INDEX IF NOT EXISTS players_club ON players(save_id, club_uid);
CREATE INDEX IF NOT EXISTS players_nation ON players(save_id, nation_id);
CREATE INDEX IF NOT EXISTS players_age ON players(save_id, age);
CREATE INDEX IF NOT EXISTS players_ca ON players(save_id, ca);
CREATE INDEX IF NOT EXISTS players_pa ON players(save_id, pa);
CREATE INDEX IF NOT EXISTS players_value ON players(save_id, transfer_value);
CREATE INDEX IF NOT EXISTS players_contract_end ON players(save_id, contract_end);
CREATE INDEX IF NOT EXISTS players_gender ON players(save_id, gender);
CREATE INDEX IF NOT EXISTS players_injury ON players(save_id, injury_status);
CREATE INDEX IF NOT EXISTS players_role_score ON players(save_id, best_role_score);
CREATE INDEX IF NOT EXISTS players_transfer_listed ON players(save_id, transfer_listed);
CREATE INDEX IF NOT EXISTS players_loan_listed ON players(save_id, loan_listed);

CREATE TABLE IF NOT EXISTS contracts (
  save_id         INTEGER NOT NULL,
  player_uid      INTEGER NOT NULL,
  club_uid        INTEGER,
  club_name       TEXT,
  team_id         INTEGER,
  wage            INTEGER,
  start_date      TEXT,
  end_date        TEXT,
  squad_status    INTEGER,
  contract_type   INTEGER,
  on_loan         INTEGER,
  loan_parent_club_uid INTEGER,
  loan_parent_club_name TEXT,
  loan_start      TEXT,
  loan_end        TEXT,
  clauses_json    TEXT,
  chain_json      TEXT,
  PRIMARY KEY (save_id, player_uid),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS contracts_wage ON contracts(save_id, wage);
CREATE INDEX IF NOT EXISTS contracts_end ON contracts(save_id, end_date);

CREATE TABLE IF NOT EXISTS clubs (
  save_id         INTEGER NOT NULL,
  uid             INTEGER NOT NULL,
  unique_id       INTEGER,
  name            TEXT,
  short_name      TEXT,
  nation_id       INTEGER,
  nation_name     TEXT,
  fa_nation_id    INTEGER,
  fa_nation_name  TEXT,
  city_id         INTEGER,
  gender          INTEGER,
  reputation      INTEGER,
  last_league_position INTEGER,
  league_competition_id INTEGER,
  league_name     TEXT,
  parent_club_uid INTEGER,
  parent_club_name TEXT,
  corporate_facilities INTEGER,
  training_facilities INTEGER,
  youth_facilities INTEGER,
  junior_coaching INTEGER,
  youth_recruitment INTEGER,
  youth_level    INTEGER,
  finance_month   TEXT,
  balance         INTEGER,
  transfer_budget_allocated INTEGER,
  transfer_budget_remaining INTEGER,
  wage_budget_weekly INTEGER,
  wage_payroll_weekly INTEGER,
  net             INTEGER,
  teams_json      TEXT,
  PRIMARY KEY (save_id, uid),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS clubs_name ON clubs(save_id, name);
CREATE INDEX IF NOT EXISTS clubs_nation ON clubs(save_id, nation_id);
CREATE INDEX IF NOT EXISTS clubs_reputation ON clubs(save_id, reputation);
CREATE INDEX IF NOT EXISTS clubs_finance ON clubs(save_id, finance_month);

CREATE TABLE IF NOT EXISTS club_finances (
  save_id         INTEGER NOT NULL,
  club_uid        INTEGER NOT NULL,
  month           TEXT NOT NULL,
  balance         INTEGER,
  transfer_budget_allocated INTEGER,
  transfer_budget_remaining INTEGER,
  wage_budget_weekly INTEGER,
  wage_payroll_weekly INTEGER,
  income_excluding_transfers INTEGER,
  net_transfers   INTEGER,
  wage_bill       INTEGER,
  net             INTEGER,
  expenditure_excluding_transfers INTEGER,
  total_income    INTEGER,
  total_expenditure INTEGER,
  PRIMARY KEY (save_id, club_uid, month),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS club_finances_club
  ON club_finances(save_id, club_uid, month);

CREATE TABLE IF NOT EXISTS stadiums (
  save_id         INTEGER NOT NULL,
  uid             INTEGER NOT NULL,
  name            TEXT,
  all_seater_capacity INTEGER NOT NULL DEFAULT 0,
  expansion_capacity INTEGER NOT NULL DEFAULT 0,
  capacity        INTEGER,
  owner_club_uid  INTEGER,
  owner_club_name TEXT,
  pitch_length_dm INTEGER NOT NULL DEFAULT 0,
  pitch_width_dm  INTEGER NOT NULL DEFAULT 0,
  pitch_min_length_dm INTEGER NOT NULL DEFAULT 0,
  pitch_min_width_dm INTEGER NOT NULL DEFAULT 0,
  pitch_max_length_dm INTEGER NOT NULL DEFAULT 0,
  pitch_max_width_dm INTEGER NOT NULL DEFAULT 0,
  built_date      TEXT,
  rebuilt_date    TEXT,
  PRIMARY KEY (save_id, uid),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS stadiums_owner
  ON stadiums(save_id, owner_club_uid);

-- Competitions are collected from the stage table. `database_id` is the key for
-- the optional shared/competition-names.csv table; the save itself stores no name.
CREATE TABLE IF NOT EXISTS competitions (
  save_id         INTEGER NOT NULL,
  id              INTEGER NOT NULL,
  database_id     INTEGER,
  name            TEXT,
  stage_count     INTEGER NOT NULL DEFAULT 0,
  stage_ids_json  TEXT NOT NULL DEFAULT '[]',
  PRIMARY KEY (save_id, id),
  FOREIGN KEY (save_id) REFERENCES saves(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS competitions_database_id
  ON competitions(save_id, database_id);

-- Career-wide growth history. `career_key` is the save name, so opening the same career at a
-- later game date adds a point instead of creating an unrelated timeline.
CREATE TABLE IF NOT EXISTS player_snapshots (
  career_key      TEXT NOT NULL,
  game_date       TEXT NOT NULL,
  player_uid      INTEGER NOT NULL,
  save_id         INTEGER NOT NULL,
  ca              INTEGER,
  pa              INTEGER,
  transfer_value  INTEGER,
  club_uid        INTEGER,
  club_name       TEXT,
  wage            INTEGER,
  contract_end    TEXT,
  recorded_at     TEXT NOT NULL,
  PRIMARY KEY (career_key, game_date, player_uid)
);

CREATE INDEX IF NOT EXISTS player_snapshots_player
  ON player_snapshots(career_key, player_uid, game_date);

CREATE TABLE IF NOT EXISTS snapshots (
  save_id         INTEGER NOT NULL,
  game_date       TEXT NOT NULL,
  player_uid      INTEGER NOT NULL,
  ca              INTEGER,
  pa              INTEGER,
  transfer_value  INTEGER,
  club_uid        INTEGER,
  wage            INTEGER,
  contract_end    TEXT,
  PRIMARY KEY (save_id, game_date, player_uid)
);
