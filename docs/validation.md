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
budget, weekly wage budget, corporate facilities, training, youth,
junior-coaching and youth-recruitment ratings, all-seater capacity and expansion
capacity. Re-parsing the output and fmsave both read the new values; the club's
last league position and all other finance fields remained unchanged.

The ignored `club_real_save.rs` tests cover:

- finance, facilities and stadium counts,
- training/youth/junior-coaching/youth-recruitment ratings against the in-game
  facilities screen for Parma, Arsenal, Liverpool, Manchester City, Tottenham,
  Napoli and Torino, plus English Youth Level against `eng_youth_grades.edt`,
- club search and detail retrieval through SQLite,
- owned-stadium capacity edits,
- re-parsing and field-by-field verification of an edited save copy.

## Staff validation

Staff rows were compared with the fmsave staff reader on both supported builds. The
comparison joins on the person object's `uid` and checks name, club, job code, CA, PA,
wage, contract end and contract presence.

```text
Parma (27.0.0+2486695): oracle=14225 rust=14173 matched=14173
  missing=52 extra=0  ca/pa/wage/contract_end/has_contract/club/job mismatches=0
Milan (27.0.0+2493426): oracle=13809 rust=13770 matched=13770
  missing=39 extra=0  ca/pa/wage/contract_end/has_contract/club/job mismatches=0
```

The unmatched oracle rows are people a club lists without any held or no-contract role
record plus the human manager; national-team appointments are excluded because their
team id has no club mapping. Profile attributes decode for every staff row on both saves.
Role suitability decodes for 12,977 of 14,173 Parma rows and 12,484 of 13,770 Milan rows;
the rest are mostly no-contract board roles whose objects carry no readable vector.
The attribute block is validated against the save's preference slots and the 26-byte
ability block; role vectors require a unique signature and at least one score of 10.

The ignored `staff_real_save.rs` test covers the directory count, attribute and role
coverage, the club-job invariant, SQLite insertion, role filtering and staff facet counts.

## Build compatibility

Build `27.0.1+2494687` was verified against the previous build by parsing the same
career at the same in-game date (2027-12-03) from a save written by `27.0.0+2493426`
and one written by `27.0.1+2494687`:

- 52,858 players, 26,127 clubs, 27,082 stadiums, 2,014 monthly finance rows and
  12,432 staff rows matched by identity and count.
- Every static field matched, including names, attributes, positions, personality,
  traits, contract structure, club reputation, facilities and stadium data.
- The only differences were mutable in-game values (transfer value, condition,
  match sharpness and six contract-clause amounts), which is consistent with the
  game recalculating values after the patch rather than a layout shift.
- `editor_real_save` (CA/PA, attributes, personality, traits, wage and contract
  end) and the club-edit real-save test passed against the new build, with the
  editor re-parsing and field-verifying each output.

The managed club resolves to the same club and manager on the new build. Reading
other `27.` builds remains degraded; editing accepts only the exact verified
build strings.

Build `27.0.1+2503222` was verified the same way against `27.0.1+2494687` by
parsing the same career at the same in-game date (2028-03-03), one save written
by each build:

- 52,147 players, 26,127 clubs, 27,088 stadiums, 2,374 monthly finance rows,
  12,256 staff rows, 2,005 competitions and 168,338 season-stat rows matched by
  identity and count.
- Every static field matched, including names, attributes, positions,
  personality, traits, contract structure, club reputation, facilities, stadium
  data, staff profile attributes and role ratings.
- The only differences were mutable in-game values: 18,030 transfer values, 70
  transfer-value states, 59 condition values, 37 match-sharpness values and 13
  contract-clause amounts.
- `editor_real_save` (CA/PA, attributes, personality, traits, wage and contract
  end) and the club-edit real-save test passed against the new build, with the
  editor re-parsing and field-verifying each output.
- The managed club resolves to A.C. Milan with manager `AP King` on the new
  build.
