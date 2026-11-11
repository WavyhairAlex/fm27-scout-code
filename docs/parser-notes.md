# Parser notes

This document records the FM27 facts that the Rust parser relies on, and the places
where the implementation intentionally stops instead of guessing.

## Reference

- Save: `BB KING - Casciavit.fm`
- Build: `27.0.0+2486695`
- `game_db` schema: 4000
- Game date: 2026-07-13
- Containers: 26-byte header + zstd frames + trailer directory
- Decompressed `game_db`: about 137 MB

The Python reference implementation is the `experiment/fm27-bounded` branch of
[fmsave](https://github.com/rhiever/fmsave). It is used only for development-time
comparison and is not shipped.

## Container

```text
0..26          header, magic 02 01 "fmf."
+9             u64 trailer pointer; trailer offset = 9 + value
trailer        starts with magic, 9-byte header, one zstd frame
directory      save name, reserved u32, entries
entry          name u32-length + name, ext u32-length + ext,
               5 × u64: relative offset, compressed size, decompressed size,
               two trailing values
frame offset   26 + relative offset
```

The parser validates the declared sizes, caps decompression at 1 GiB per frame and
4 GiB total, and never assumes one frame contains the whole section.

## Snapshot policy

The desktop app copies the selected save to `~/Library/Application Support/com.fm27scout.desktop/snapshots/`
before parsing, compares the source size and mtime before/after the copy, and removes the
snapshot after parsing. This keeps a game that is still writing from being read half-way.
Stale snapshots are removed on the next launch.

## Metadata

- `save_game_summary` holds the length-prefixed version string. The structured clock is
  after the setup string, version string, counted division names, manager header,
  manager name, club name and club uid.
- `game_info` schema 47 has a changed filename/migration area. In this save its clock is
  unset, so the summary clock is authoritative (2026-07-13).
- `db_version` is read from `game_info` at offset 8.

## Managed club

- `humans` carries the human-manager count at offset 8 (u16) and the first
  selector at offset 10 (u32). The selector is the manager's person id plus one.
- Route 1 scans `game_db` for the selector five bytes after a contract record tag
  (`01 00 6c 07`), decodes the chain record, and keeps records whose team
  resolves to a club and whose end date is not before the in-game date. A record
  with a parsed tail beats one without, then the later end wins.
- Route 2 scans the length-prefixed strings of `save_game_summary` from offset 8
  (length 2..120, every code point at least 0x20). A club short name immediately
  followed by that club's uid is a link; the string ending exactly where the club
  name starts is the manager name.
- The contract record is authoritative. When the summary links another club, the
  parser keeps the contract club and records a warning. With no contract record,
  exactly one summary link supplies the club; multiple links leave it unknown.
- The reader runs only on the verified builds; an unverified build records a
  warning and leaves the managed club unknown rather than applying these offsets.

## Name pools

The pools are preceded by the signature `55 b5 00 00 00 00 00 00 00 04 00 00 00 01 00 00 00 08 00 00 00 00 00 00`
(6 little-endian u32 words: 46421, 0, 1024, 256, 2048, 0). The parser accepts the
observed scaled form and validates every entry id and length. Entries are
`u32 id + u32 byte length + UTF-8 text`.

## Player records

Marker: `01 00 6c 07`, at `record_start + 102`.

```text
-19   pindex
-15   uid
-11   uid copy
-7    object kind (2 or 3)
-6    home reputation
-4    current reputation
-2    world reputation
+0    CA (u16)
+2    PA (i16; negative is a range code)
+8    reputation bucket
+16   team id
+24   15 position ratings
+39   54 attribute bytes (52 attributes + left/right foot at indices 24,25)
+93   transfer value
+98   club join date
+106  match sharpness
+110  condition
+121  height cm
+122  decode extent
```

Acceptance requires non-zero/doubled uid, valid CA/PA/bucket, the 15+54 shape, and
edited-value bounds. Allocated uid headers may differ from their copy only when the
object kind is 2 or 3. The completeness scan is an O(n) pass that finds 15 consecutive
bytes in `1..=20` and then validates 54 bytes in `1..=125`; it found 485 markerless
records in the reference save.

Attribute display values use `max(1, (raw + 2) / 5)`. Natural positions are ratings
≥18; accomplished are 15–17, both sorted by rating descending.

The player's game Unique ID comes from the closing person header: the next object
header carrying `person_id + 1` and the same Unique ID twice. The game allocates IDs
from `2,002,000,000` upward to people it creates after the save starts, so players at
or above that threshold are system-generated newgens. FM27 database people stay below
it even when their IDs are in the `2,000,000,000` range.

The header is identified by its person id, not by an ascending Unique ID. Long careers
can store batches of generated people in descending ID order, and non-player objects
can sit between two player records, so a floor or ceiling taken from the neighbouring
player record rejects valid headers. The closing header also bounds the person-block
window; without it, a window that spans later people presents several valid person
candidates and the player is left unnamed.

## Person block

The block is found by a zero byte followed by eight personality bytes in `1..=20`.
FM27 moved the block one byte later than FM26:

```text
search start        record + 100
personality offset  birth + 22   (FM26: 21)
zero region         birth + 15, 6 bytes (FM26: 14, 7)
nation id           birth + 14
relation present    birth + 38
relation count      birth + 39
relation entries    birth + 40, 16 bytes each
legal name          block_start + 19, preceded by u32 length at birth - 4
trait bits          block_start - 8 (u64)
name ids            block_start + 0/5/10, each followed by a zero byte
```

FM27 requires exactly one valid candidate in the window; ambiguous windows return no
person data. The window ends at the player's closing person header when one is found,
so later people stored before the next player record do not create ambiguity. Relation
keys are `(role << 8) | kind`; the observed pairs are
`(8,9)` second nation, `(8,70)` home-grown nation, `(1,72)` home-grown club.

## Clubs

The counted club table supplies exact record bounds and the initial club (uid 0).
The FM27 table differs from FM26 in two places:

- the full-finance leading step is 66 bytes instead of 70;
- `after_finance` steps 5, 6 and 7 use 2-byte counts instead of 1-byte counts.

Each row has a kind byte (0 ordinary, 12 extended), two length-prefixed names,
variable finance steps, team counts, affiliate team counts and a 12-byte footer.
A team controlled by another club is fielded by that parent for player joins, while
the registration club remains available in the contract chain.

The anchor scan remains as a fragment fallback. On full saves the counted table is
required; a framing failure is reported rather than guessed around.

## Nations and club gender

The nation table is a counted table of 251 rows. Each row opens with `0x00`, is walked
through its fixed and counted sections, and closes with `row_index, database_id, database_id`.
The database id is the key for `shared/nation-names.csv`; player and club nation fields are
save-local row indexes.

A club's team list stores its gender byte immediately before the team count. `0` is men,
`1` is women, and other values are kept raw. Players inherit their club gender; free agents
stay unknown.

## Competitions

The save has no competition table. Competitions exist because the stage table names them: one
record per distinct competition id, in ascending id. Stages are 33-byte rows in the tail of
`game_db`; each stores the previous stage id at +0, the stage id twice at +4/+8, a zero byte at
+12 and the competition id at +13. The table is found by a chain of 200 decodable rows and is
walked forward with byte-level resynchronisation over gaps.

The editor database id comes from separate `FF × 16 + 01` records. A record starts 30 bytes
after the marker, stores the stage-space entity id at +0 and the editor database id twice at
+4/+8, and must match 15 constant bytes. An entity whose accepted records disagree is left
without a database id. The resulting map covers about three times as many entities as the
stage table calls competitions, so it is restricted to competition ids before joining.

The reference save has 3,459 stage rows, 1,724 competitions and 1,586 competitions with a
database id. A database id claimed by two competitions names neither. Names are not stored in
the save; `shared/competition-names.csv` maps the editor database id to a name, and suspension
scope ids (code 1) are named through the same index.

## Injuries and suspensions

`injury_manager` is a four-array, three-list section ending in
`01 00 00 00 00 00 ff ff`. Arrays 0 and 1 (stride 14) hold the current injury windows: an
earlier recovery milestone at `+1`, the expected return the game displays at `+5`, and the
person selector at `+9`. Array 2 (stride 13) holds typed expected-return rows; array 3
(stride 15) holds injury history. Person selectors are `pindex + 1`. The reference save has
no array-2 rows and 783 history rows.

Suspensions live in `game_db` as 18-byte entries with signature bytes at
`+4 ff, +5 ff, +13 05, +15 ff, +18 ff`. The issued date is a 4-byte game date at `+9`, the
scope code at `+14`, and the scope id (competition or nation) at `+16`. An entry belongs to
the player whose record starts within 30 bytes before it. The reference save has 17 unserved
bans.

## Snapshots and role scores

Every successful parse writes one `player_snapshots` row per player, keyed by
`career_key + game_date + player_uid`. `career_key` is the save name, so opening a later state
of the same career appends points instead of creating an unrelated timeline. Growth queries
only return points at or before the opened save's game date.

Role scores are computed from display attributes with per-role weights plus a position-fit
factor; the best role is stored on the player row for the table, while the full role list is
computed on demand in the inspector.

## Contracts

The player's selector is `pindex + 1`, searched in a window starting at `record - 30`.
A chain record starts at the selector offset minus 5; its first four bytes are the tag
`01 00 6c 07` or a date-marked equivalent. Team id and wage are at +9/+17; the start
date is at -19.

The tail locator starts at `tag - 66`, checks two zero bytes, the sentinel `03 00 00 00 04`
at +3, and `event_count = 0` at +42. Event-counted tails step backwards by 29 bytes.
End date is at tail +28, squad status at +36. Clause tables are checked directly at
`tail - 8*n`; the fallback walk finds tables with bonus lists that end exactly at the
tail. Contract type is read from the head at clause base -35 (gate 5).

Loans use the tail-shaped block before a no-terms chain record: marker `FFFFFFFF` at
+24 and an end date at +28.

The in-effect contract tail also carries transfer status. At +39 the flags byte uses
bit 0 for listed for transfer and bit 1 for listed for loan; other bits (for example
`0x40`) appear on many players and are unrelated to the listing result. The status
byte at +40 is often `5`, but it is also set on players who are not listed, including
key players, so it is not read as a listing indicator. On a save where Trent
Alexander-Arnold (status byte `5`, flags `0x00`) is not listed in game, the flag-only
rule reports 454 transfer-listed and 1,495 loan-listed players; the old `+40 = 5` rule
reported 756 transfer-listed players.

## Staff

Staff records are person objects whose header kind byte is `1`. A header is accepted only
when the pindex and uid are non-zero, the uid copy matches (or the uid is an allocated
`>= 2,000,000,000` id), CA/PA fall in 1..200 and the ability block carries the sentinel
`12`. The 7-byte entry count shifts the ability block, so the header, not a fixed offset,
locates CA/PA.

The current job comes from the held contract record or the no-contract role record. A
second contract tag 19 bytes after the selector marks an offer and is rejected. When several
held records exist, the latest start date wins.

Profile attributes use the preference area and the 26-byte block at ability +40. Authority
and working with youngsters are already 1..20; determination is stored 1..100 and divided
by five. The other 18 attributes use the same 1..100 encoding. A row keeps raw values too,
so the UI can switch scales without re-parsing.

Role suitability is a 15-byte vector of 1..20 scores found within 4 KiB of the person
header. The search requires a unique candidate; two candidates leave the field `None`
rather than picking one. Missing attributes or roles are `None`, never zero.

On the reference saves the decoder finds 14,173 of fmsave's 14,225 staff rows for Parma and
13,770 of 13,809 for Milan, with **0 differences** on CA, PA, wage, contract dates,
contract presence, club and job across every matched row. The remaining rows are people a
club lists without any contract or no-contract role record, plus the human manager, whose
object has no standard ability block; those are documented limitations.

## Validation

The Rust output is normalized to the same JSON keys as the oracle and compared for all
46,273 players. The final run reported **0 differences** across:

- identity, names, birth date, age, nation, home-grown relations
- CA/PA, height, reputation, club/team/slot, join date
- all 52 attributes and raw values, feet, 15 position ratings
- personality, trait bits, transfer value/state, condition/sharpness
- contract club, wage, dates, status, type, clauses, loans and chain
- club set, names, nations, reputation, league position and team lists

See [validation.md](validation.md) for the commands and counts.
