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

CA and attributes are linked. Controlled FM27 tests produced these results after
roughly one in-game month:

| Edit | CA | Key attributes | Other attributes |
|---|---:|---|---|
| CA only, 110 → 150 | 150 → 152 | not set | rose by 11–13 raw points |
| Attributes only, CA 110 | 110 → 110 | 100 → 85 | fell by 13–15 raw points |
| CA 150 + six key attributes | 150 → 150 | 100 → 96 | trimmed by 3–4 raw points |

In other words, **CA is the attribute budget**, and FM reallocates visible
attributes to fit it. Raw attribute sum is not CA: FM weights attributes by
position, so a top-heavy distribution can have a lower raw sum for the same CA.

Practical workflow:

- **General boost**: set `--ca` and `--pa`; FM fills in the attributes over
  roughly a month.
- **Controlled build**: set `--ca` high enough and set a small number of key
  attributes. Key attributes are largely preserved; marginal attributes are
  trimmed.
- **Visible attributes alone** at a low CA are unstable; FM pulls them back
  toward the CA budget.
- Hidden attributes, personality, and traits are independent of CA and are the
  most stable edits.

The CLI prints a warning when `--ca` is changed, when a visible attribute is
changed without `--ca`, or when both CA and visible attributes are changed.

See [editor-test-plan.md](editor-test-plan.md) for the controlled A/B/C comparison
of CA-only, attributes-only and CA-plus-attributes edits.

## Safety

1. Close Football Manager before relying on an edited save. FM rotates/autosaves
   save files while the game is running.
2. Keep the original save; the editor writes a new file by default.
3. Load the edited copy from the in-game load menu and check the player after a few
   in-game days/weeks.
4. If anything looks wrong, delete the edited file and restore the backup.

The editor does not read game memory or inject into the game process.
