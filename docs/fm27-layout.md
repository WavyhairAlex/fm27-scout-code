# FM27 layout quick reference

Builds `27.0.0+2486695`, `27.0.0+2493426` and `27.0.1+2494687`, `game_db` schema
4000. All three builds share the layout below. Offsets are decimal bytes unless noted.

## Save container

```text
header size            26
magic                  02 01 66 6d 66 2e
trailer pointer u64 @  9
trailer offset         9 + pointer
trailer header         9 bytes
zstd frame             1..n, then directory frame
section header         03 01 + reversed extension, schema u16 @ +6
```

## Player record

Marker `01 00 6c 07` is at record +102. Record extent 122 bytes.

```text
pindex              -19
uid                 -15
uid copy            -11
object kind         -7
home reputation     -6
current reputation  -4
world reputation    -2
CA                  +0   u16
PA                  +2   i16
reputation bucket   +8   u8
team id             +16  u32
positions           +24  15 × u8
attributes          +39  54 × u8 (feet at 24/25)
transfer value      +93  u32
club join date      +98  game date
match sharpness     +106 u16
condition           +110 u16
height cm           +121 u8
```

Display attributes: `max(1, (raw + 2) / 5)`.

## Person block (FM27 shift)

```text
window start        record + 100
personality offset  birth + 22  (FM26: 21)
zero region         birth + 15, 6 bytes (FM26: 14, 7)
nation id           birth + 14
relation present    birth + 38
relation count      birth + 39
relation entries    birth + 40, 16 bytes
legal name length   birth - 4 (u32)
legal name text     block_start + 19
trait bits          block_start - 8
first name id       block_start + 0
surname id          block_start + 5
common name id      block_start + 10
```

FM27 requires exactly one valid candidate.

## Clubs

Counted table rows; FM27 changes:

```text
full_finance[0].lead_bytes = 66   (FM26: 70)
after_finance[5..7].count_bytes = 2 (FM26: 1)
```

Initial club uid is 0 and has no anchor; it comes from the table's first row.
Anchor scan fallback: `FF FF FF FF` at record +17.

### Club finance chain

A club that keeps a monthly series stores a counted run of 49-byte rows inside
its own record. The count is a u32 at `row - 4`; the full chain count range is
3–1000, with a balanced 1–2 row short chain as a fallback.

```text
+0   tag                 01
+1   balance             i32
+5   transfer allocated  i32
+9   transfer remaining  i32
+13  wage budget         u32
+17  wage payroll        u32
+21  income excl. transfers i32
+25  net transfers       i32
+29  wage bill           i32
+33  net                 i32
+37  expenditure excl. transfers i32
+41  total income        i32
+45  total expenditure   i32
```

The corporate-facilities byte sits 50 bytes past the end of the chain, range
1–20. Only clubs in the tracked league nations keep a chain.

### Club facility rows

Training, youth, junior-coaching and youth-recruitment ratings live in a club-info
row outside the club record. A row starts with the stored club index plus one
(u32) and the training-facilities byte at `+4`. The row header is followed by a
variable-length list, then the supporter-share block:

```text
supporter shares       six bytes, sum 100
supporter traits       +15, six bytes (1–20)
zero run               +21, six bytes
tag                    +27  02
youth facilities       +63  u8 (1–20)
junior coaching        +64  u8 (1–20)
youth recruitment      +65  u8 (1–20)
```

Rows are not contiguous and only the tracked clubs (those with a finance chain)
have one. A club's key can also occur inside another record, so locate every
plausible row by its club-index key and share block, then keep the longest
sequence ordered by club index: the real facility table is sorted by that key.
Do not simply take the first byte match or walk the rows sequentially.

English Youth Level is not stored in this row or in the save. It comes from the
game's `eng_youth_grades.edt` file, which maps club Unique IDs to EPPP categories
1–4 for the current season. The parser reads that file from the installed game
and leaves the field empty when the file is unavailable.

### Stadium table

Rows are 181 bytes, unless bit `0x10` of the flags byte at +156 is set: then a
u32 name length at +158 and that many UTF-8 bytes at +162 follow, and the row is
`181 + 4 + name_length` bytes.

```text
+0   ordinal             u32
+4   uid                 u32 (stored uid = uid - 1)
+8   uid copy            u32
+12  zero byte           u8
+13  all-seater capacity u32
+21  expansion capacity  u32
+29  owner club index    u32 (FFFFFFFF = none)
+34  capacity            u32
+38  pitch length        u16 (decimetres)
+40  pitch width         u16
+50  built date          game date
+54  rebuilt date        game date
+67.. pitch min/max      four u16
+156 flags               u8
```

The table is located by twelve consecutive rows carrying ordinals 1, 2, 3 …,
the uid twice and a zero byte. Ownership is stored; home-ground use is not.

## Contract

```text
selector            pindex + 1
chain tag offset    selector offset - 5
tag                 01 00 6c 07
team id             +9
wage                +17
start date          -19
tail base            -66
tail step            29
tail event count     +42
tail end date        +28
tail squad status    +36
tail list flags      +39  bit 0 transfer, bit 1 loan
tail status code     +40  often 5, not a listing indicator
clause step          8
clause entries       base - 4
head gate            base - 35 (value 5)
head type            base - 33
```

Loans: tail-shaped block before a no-terms chain record, marker `FFFFFFFF` at +24,
end date at +28.

## Competitions

The stage table sits in the tail of `game_db`. Rows are 33 bytes:

```text
previous stage id   +0   u32
stage id            +4   u32
stage id copy       +8   u32
zero byte           +12  u8
competition id      +13  u32
```

A row decodes when the copy equals the stage id, the zero byte is zero, the previous id is
`id - 1` or `FFFFFFFF`, and the id is strictly between 0 and 200000. The table is found by a 200-row chain
and walked with byte-level resynchronisation.

Editor database ids come from `FF × 16 + 01` markers. The record starts 30 bytes after the
marker:

```text
entity id           +0   u32 (1..199999)
database id         +4   u32
database id copy    +8   u32 (must equal +4)
constants           -8 7, -6 0, -4 7, -1 FF, +12 0, +15 1,
                    +22..+25 0, +26 1, +32 2, +40 3, +55 6, +60 7
```

The database id is the key for `shared/competition-names.csv`; the save stores no names.

## Season statistics frame

The season-statistics section is not listed in the directory. It is one frame in the
unlisted gap after `non_pl_hist_ls`:

```text
region            unlisted_after_non_pl_hist_ls
section header    03 01 74 61 64 2e 0f 00
```

Record layout:

```text
record marker     01
player key        +1   u32 (pindex + 1)
header            +5   16 bytes
own slots         +21  8 slots
block             +?   20 bytes
other team count  u32 (max 16)
other team row    lead 1 byte, team id u32, 6 slots
rating item count u32 (max 256)
rating items      count × 8 bytes
form entries      count byte (max 16)
form entries      count × 3 bytes
```

A slot is `00` when absent or a 155-byte line starting with `01 07`. The list ends with
`00` followed by exactly 16 footer bytes.

Line fields use the FM26 offsets, with `line_bytes = 155` for FM27. Goalkeepers and outfield
players share offsets 35, 37, 69/71, 83 and 95; natural position GK selects the goalkeeper
set. `distance_km == 0` marks the whole advanced-stat block missing (key passes, progressive
passes, interceptions, headers, clearances, blocks, sprints, possession, saves, xGP).

Rust implementation: `crates/fm27-core/src/season_stats.rs`; container region support:
`crates/fm27-core/src/container.rs`.

## Staff objects

Staff objects use the person-header shape with kind byte `1` at header +12. Verified on
`27.0.0+2486695`, `27.0.0+2493426` and `27.0.1+2494687`:

```text
header             pindex u32
uid                +4   u32 (previous person's Unique ID)
uid copy           +8   u32
kind               +12  01
entry count        +13  u8 (7-byte entries follow)
ability block      header + 21 + 7 * entry_count
  CA               +0   u16 (1..200)
  PA               +2   i16 (1..200)
  r4               +4   u8 (not the displayed job; never used as one)
  sentinel         +13  12
  preferences      +14  26 bytes (1..100)
  block 40         +40  26 bytes (1..100)
```

The closing person header has `pindex + 1` and a doubled uid; its uid is this person's
Unique ID. Job codes are read from the current contract record at `tag + selector + 16`,
or from a no-contract role record at `tag + 32`:

```text
contract tag      01 00 6c 07
selector          tag + 5   u32 (pindex + 1)
team              tag + 9   u32
wage              tag + 17  u32
job               selector + 16
offer marker      selector + 19 (a second tag marks an offer, not a held job)

no-contract tag   01 00 6c 07
lead              tag + 14  05 05
selector          tag + 16  u32 (pindex + 1)
team              tag + 20  u32
job               tag + 32  u8
```

Staff profile attributes, displayed on the 1–20 scale:

```text
preferences +4    authority               1..20
preferences +12   working with youngsters  1..20
preferences +13   determination            1..100 (divide by 5 for display)
block 40 index    1 goalkeeping            2 judging player ability
                  3 judging player potential 4 people management
                  5 motivating             6 physiotherapy
                  7 tactical knowledge     8 attacking
                  9 defending              10 fitness
                  11 possession            12 technical
                  13 tactical              18 data analysis
                  21 sports science        23 negotiating
                  24 judging staff ability 25 set pieces
```

Role suitability is a 15-byte vector found inside the person's object, within 4 KiB of its
header: each byte is 1..20, preceded by `01`, followed by four zero bytes, and at least one
value is 10 or higher. The order is **Manager, Assistant Manager, Coach, Physio, Scout,
Goalkeeping Coach, Fitness Coach, Chairperson, Director of Football, Head of Youth
Development, Loan Manager, Technical Director, Data Analyst, Sports Scientist, Set Piece
Coach**.

Rust implementation: `crates/fm27-core/src/staff.rs`.

## Managed club

`humans` schema 24 names the human managers:

```text
+8   u16 human-manager count
+10  u32 first manager's selector (person id + 1)
```

The selector appears five bytes after a contract record tag in `game_db`. The
`save_game_summary` strings are the independent check: a club short name
followed by its club uid; the adjacent preceding string is the manager name.
Details and conflict rules are in [parser-notes.md](parser-notes.md).

Rust implementation: `crates/fm27-core/src/managed.rs`.

## Section schemas observed on the reference save

`game_db` 4000, `game_info` 47, `save_game_summary` 31, `contract_man` 14,
`person_record_manager` 30, `person_db_changes` 3754, `player_stats_hist_dt` 7,
`regeneration_manager` 47, `training_man` 32, `news` 2957, `humans` 24.
Other sections are not required by the scouting MVP.
