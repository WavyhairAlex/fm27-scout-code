# Validation

## Reference save

- File: `BB KING - Casciavit.fm`
- Size: 49,042,131 bytes
- Build: `27.0.0+2486695`
- Game date: 2026-07-13
- `game_db`: about 137 MB decompressed
- All sections: about 192.5 MB decompressed

## Oracle

The oracle is the experimental FM27 branch of fmsave, installed into a local virtual
environment only for development:

```bash
.venv/bin/python scripts/oracle_export.py --output /tmp/oracle_players.json
```

The export contains 46,273 players with the fields compared by
`scripts/compare_outputs.py`. Fields the oracle does not export are skipped by the
comparison: derived names, club gender, role scores, injuries and suspensions are covered
by the real-save regression test instead.

## Rust output

```bash
cargo build --release -p fm27-cli
./target/release/fm27scout players "/path/to/save.fm" --limit 50000 \
  --format json --output /tmp/rust_players.json

.venv/bin/python scripts/compare_outputs.py /tmp/oracle_players.json /tmp/rust_players.json
```

## Final result

```text
oracle=46273 rust=46273 missing=0 extra=0
players with differences: 0
```

Compared fields include identity, names, legal/common names, birth date, age, nation,
second/home-grown relations, CA/PA, reputation, height, club/team/slot, join date,
52 attributes and raw values, feet, 15 position ratings, personality, trait bits,
transfer value/state, condition/sharpness, contract club/wage/dates/status/type,
clauses, loans, chain, and club set/names/nations/reputation/league position/teams.

Club comparison:

```text
clubs oracle=26214 rust=26214 missing=0 extra=0
club mismatches: 0
```

Competition comparison:

```bash
.venv/bin/python scripts/oracle_competitions.py --output /tmp/oracle_competitions.json
./target/release/fm27scout competitions "/path/to/save.fm" --output /tmp/rust_competitions.json
.venv/bin/python scripts/compare_competitions.py /tmp/oracle_competitions.json /tmp/rust_competitions.json
```

```text
oracle=1724 rust=1724 missing=0 extra=0
competitions with differences: 0
```

## Counts

| Metric | Value |
|---|---:|
| Players | 46,273 |
| Clubs | 26,214 |
| Competitions | 1,724 |
| Competition stages | 3,459 |
| Competitions with editor database id | 1,586 |
| Contracts | 44,341 |
| Markerless player records | 485 |
| Players with a person block | 46,272 |
| Players with a resolved club | 39,333 |
| Parse time (release) | ~1.3 s |
| Parse + SQLite insert | ~4.5 s |
| Men / women / unknown gender | 27,544 / 11,789 / 6,940 |
| Unserved suspensions | 17 |
| Injury history rows | 783 |
| Countries resolved from the nation table | 251 save-local ids |

## Regression test

```bash
FM27_TEST_SAVE="/path/to/save.fm" \
  cargo test -p fm27-core --release -- --ignored
```

The ignored tests check the counts, Modrić's fields, nation names, gender split, role scores,
suspensions, injury history, SQLite query/filter behavior, facet counts, multi-column sorting,
growth snapshots, analysis overview, squad depth, wonderkid filtering, full 46,273-row export,
CSV and JSON export.

## Club finance, stadium and edit validation

The club pass was checked against the experimental FM27 fmsave reader on
`BB KING - Parma Calcio.fm` (game date 2026-10-23):

```text
clubs                           26,138
clubs with monthly finance         122
finance rows                       488
corporate facilities rows          122
training/youth facility rows       121
stadium rows                    27,042
parse time (release)             2.10 s
```

The development oracle comparison uses:

```bash
.venv/bin/python scripts/oracle_clubs.py "/path/to/save.fm" --output /tmp/oracle_clubs.json
./target/release/fm27scout clubs "/path/to/save.fm" --details --output /tmp/rust_clubs.json
.venv/bin/python scripts/compare_clubs.py /tmp/oracle_clubs.json /tmp/rust_clubs.json
```

```text
clubs: oracle=26138 rust=26138 missing=0 extra=0
club_finances: oracle=488 rust=488 missing=0 extra=0
club_facilities: oracle=122 rust=122
stadiums: oracle=27042 rust=27042 missing=0 extra=0
mismatches: 0
```

A controlled club edit on Parma (uid 1155) changed reputation, balance, transfer
budget, weekly wage budget, corporate facilities, all-seater capacity and
expansion capacity. Re-parsing the output and fmsave both read the new values;
the club's last league position and all other finance fields remained unchanged.

The ignored `club_real_save.rs` tests cover:

- finance, facilities and stadium counts,
- training/youth/junior-coaching/youth-recruitment ratings against the in-game
  facilities screen for Parma, Arsenal, Liverpool, Manchester City, Tottenham,
  Napoli and Torino,
- club search and detail retrieval through SQLite,
- owned-stadium capacity edits,
- re-parsing and field-by-field verification of an edited save copy.
