# FM27 layout quick reference

Build `27.0.0+2486695`, `game_db` schema 4000. Offsets are decimal bytes unless noted.

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
tail list code       +40  5 = transfer listed by club
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

## Section schemas observed on the reference save

`game_db` 4000, `game_info` 47, `save_game_summary` 31, `contract_man` 14,
`person_record_manager` 30, `person_db_changes` 3754, `player_stats_hist_dt` 7,
`regeneration_manager` 47, `training_man` 32, `news` 2957, `humans` 24.
Other sections are not required by the scouting MVP.
