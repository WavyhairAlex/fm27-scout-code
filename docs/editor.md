# Experimental save editor

The desktop app remains read-only. The `edit` CLI subcommand edits a **copy** of a
save and writes a new `.fm` file; it refuses to overwrite the source file.

> Use this only on your own single-player saves. Keep a backup, close Football
> Manager before replacing a save, and do not use edited saves in online games.

## Supported saves

- FM27 build `27.0.0+2486695`
- `game_db` schema 4000

The editor refuses other builds and schemas instead of guessing field offsets.

## Usage

```bash
# Preview the changes without writing a file
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --personality professionalism=20 \
  --dry-run

# Write a new save next to the source
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --attr consistency=100 \
  --personality professionalism=20 --trait 11 --output edited.fm
```

By default the output is `<save stem>.edited.fm` in the same directory. Use
`--save-name "FM27 Scout Editor Test"` to change the internal save name so a test
copy is easy to find in the game's load menu. The command refuses an existing
output unless `--force` is passed, and it verifies the result by parsing the new
save and checking every requested field.

## Fields

| Option | Range | Notes |
|---|---|---|
| `--ca` | 1–200 | Current ability |
| `--pa` | 1–200 | Potential ability; negative random-range codes are not supported |
| `--attr KEY=VALUE` | 1–100 raw | Any visible attribute, or a hidden attribute |
| `--personality KEY=VALUE` | 1–20 | Adaptability, ambition, loyalty, pressure, professionalism, sportsmanship, temperament, controversy |
| `--trait BIT` / `--untrait BIT` | 0–63 | Adds or removes a known trait bit |

`--ca` and `--pa` must leave CA at or below PA. The editor rejects the
combination before writing.

### Hidden attributes

These five attributes live in the same 54-byte array as the visible attributes and
can be set with `--attr`:

```text
consistency, important_matches, injury_proneness, versatility, dirtiness
```

The eight personality bytes are stored in a separate person block and use
`--personality`.

### Raw values and display values

The game stores attributes as raw bytes from 1 to 100. The UI display value is:

```text
display = max(1, (raw + 2) / 5)     // 1..20
```

The CLI takes raw values. To set a displayed value `d`, use a raw value around
`5 * d - 2`; the exact raw byte matters if you later compute CA.

## CA/PA behaviour

PA is a ceiling. Raising PA alone only helps players who can still develop, and it
does not change current attributes immediately.

CA and attributes are linked. In a measured FM27 test, a player was edited to
CA 150 / PA 200 with two visible attributes at raw 100. After 27 in-game days:

- CA rose from 150 to 152.
- PA 200, personality, and the added trait stayed unchanged.
- The two deliberately raised attributes and the hidden attributes stayed at
  their edited values.
- The other 29 visible attributes rose by roughly 5–6 raw points.

In other words, **CA drives the visible attributes**, and FM spent about a month
redistributing them to match the new CA. Moving CA alone is therefore a valid
workflow: set `--ca` and `--pa`, and let the game fill in the attributes.

The CLI prints a warning when:

- `--ca` is changed, because visible attributes will be rebalanced; or
- a visible attribute is changed without `--ca`, because that change may be
  temporary.

Hidden attributes (`consistency`, `important_matches`, `injury_proneness`,
`versatility`, `dirtiness`), personality, and traits do not participate in this
rebalancing and are the most stable edits.

For a specific visible-attribute boost, set `--ca` high enough for the desired
attribute budget and set the key attributes too, then verify over at least a month.
Editing only visible attributes while leaving CA low is likely to be reverted.

## Safety

1. Close Football Manager before relying on an edited save. FM rotates/autosaves
   save files while the game is running.
2. Keep the original save; the editor writes a new file by default.
3. Load the edited copy from the in-game load menu and check the player after a few
   in-game days/weeks.
4. If anything looks wrong, delete the edited file and restore the backup.

The editor does not read game memory or inject into the game process.
