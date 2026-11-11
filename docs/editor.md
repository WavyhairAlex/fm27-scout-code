# Experimental save editor

The desktop app's scout view is read-only. The player **Edit** dialog and the
`edit` CLI subcommand edit a **copy** of a save and write a new `.fm` file; they
refuse to overwrite the source file.

> Use this only on your own single-player saves. Keep a backup, close Football
> Manager before replacing a save, and do not use edited saves in online games.

## Supported saves

- FM27 builds `27.0.0+2486695`, `27.0.0+2493426` and `27.0.1+2494687`
- `game_db` schema 4000

All three builds share the same save layout. The editor refuses other builds and
schemas instead of guessing field offsets.

Build support is keyed on the exact number after `+`, not on the game version:
`27.0.1` is the game version string, while `2494687` identifies the build that
wrote the save. A patch release can change record layout, so the game version
alone does not prove that fixed offsets are safe. To support a new build, compare
a save of the same career and game date written by the previous build against one
written by the new build, run the ignored `editor_real_save` and
`club_real_save` tests against it, then add its exact build string to
`EDITABLE_BUILDS` and bump `PARSER_VERSION`. Reading accepts any `27.` build in
degraded mode; editing is limited to the verified list.

## Desktop UI

Open a save, select a player, and click **Edit** in the player profile header.
The dialog groups:

- ability (CA/PA),
- contract (current permanent contract only: weekly wage and end date),
- CA-weighted attributes plus a separate CA-exempt group
  (Determination, Natural Fitness, Flair, Aggression),
- hidden attributes and personality,
- traits.

**Add to pending edits** stages the changes instead of writing a save. Repeat for
as many players as needed; pending players show a count in the toolbar. Open
**Pending edits** to review/remove staged players, then **Save all to a copy…**
writes every staged player in one batch through a native Save dialog. The batch is
verified by re-parsing the new save, and the source save is never overwritten.

## Club edits

The club profile's **Edit club** dialog exposes a small, low-risk set of fields:

- club reputation (1–10000),
- latest-month balance, transfer budget remaining and weekly wage budget,
- corporate facilities (1–20; only for clubs with a monthly finance chain),
- training facilities, youth facilities, junior coaching and youth recruitment
  (1–20; only for clubs with a monthly finance chain),
- all-seater and expansion capacity for each stadium the club owns.

The dialog writes one new save copy, re-parses it, and verifies every changed
field. Attribution and stadium ownership are not editable; the source save is
never modified.

Facility ratings live in the club-info row that also holds supporter shares.
Youth Level is read-only: English clubs get their EPPP category from the
installed game's `eng_youth_grades.edt`, and it cannot be edited.

## Contract edits

The player **Contract** tab edits the current permanent contract only:

- weekly wage (the save's base-currency value, not a converted display currency),
- contract end date.

The editor uses the same in-effect contract selection as the parser, so it
writes the record the profile displays. It refuses players with no current
contract terms (free agents, loan-only records, unobserved structures) and
refuses contracts that have already expired. Loans keep their existing loan end
date; only the parent-club contract is changed. Future pre-contracts are left
untouched.

The wage must be 0–20,000,000. The end date must be after the contract start and
not before the in-game date. FM may recalculate the club's wage bill, budgets and
financial-rule totals after the game advances, so check the club finances screen
after loading an edited copy.

## CLI usage

```bash
# Preview the changes without writing a file
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --personality professionalism=20 \
  --dry-run

# Write a new save next to the source
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --attr consistency=100 \
  --personality professionalism=20 --trait 11 --output edited.fm

# Edit the current permanent contract
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --wage 50000 --contract-end 2030-06-30 --output contract-edited.fm

# Low-risk club edit
./target/release/fm27scout edit-club "/path/to/save.fm" --club-uid 1155 \
  --reputation 6500 --corporate-facilities 8 \
  --training-facilities 18 --youth-facilities 16 \
  --junior-coaching 15 --youth-recruitment 14 \
  --balance 30000000 --transfer-budget 6000000 --wage-budget 600000 \
  --stadium-uid 43156154 --all-seater 1300 --expansion 6000 \
  --output club-edited.fm
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
| `--wage VALUE` | 0–20,000,000 | Weekly wage in the save's base currency; current permanent contract only |
| `--contract-end DATE` | YYYY-MM-DD | Contract end date; must be after the start and not before the in-game date |

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

### CA-exempt attributes

**Determination, Natural Fitness, Flair and Aggression** are visible but do not
consume the CA budget. In the controlled tests they stayed at their original
values while CA-weighted attributes moved:

- Determination: `65 → 65` in all A/B/C runs.
- Natural Fitness: `51 → 52` in the CA-only run (+1 training drift), and
  `51 → 51` in both the low-CA and CA-plus-attributes runs.
- Flair and Aggression: `100 → 100` in the direct test, while stamina and
  strength fell from `100 → 94` under the same CA 110 budget.

They can be edited with `--attr determination=...`, `--attr natural_fitness=...`,
`--attr flair=...` or `--attr aggression=...` without changing CA. The public FM
attribute-weight table also marks these four as zero-weight for every position.

Other position-irrelevant attributes (for example, goalkeeper attributes on an
outfield player) also stayed unchanged in the test, but the editor does not yet
model position-specific CA weights. Its warnings therefore remain conservative.

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
- Hidden attributes, determination, natural fitness, personality, and traits are
  independent of CA and are the most stable edits.

The CLI prints a warning when `--ca` is changed, when a CA-weighted visible
attribute is changed without `--ca`, or when both CA and CA-weighted attributes
are changed. Determination and hidden attributes do not trigger the attribute
warning.

## CA budget estimate

The desktop editor's **Ability** tab estimates the game's recommended current
ability (RCA) from the edited attributes. It is a direct implementation of the
community reverse-engineering of FM's CA calculation:
<https://www.playgm.cc/thread-986285-1-1.html>.

- It uses the corrected community position-weight table (GK, DL/DR, DC, WBL/WBR,
  DM, ML/MR, MC, AML/AMR, AMC, ST).
- Attributes are the internal 1–100 values (the game displays 1–20). The feet
  follow the game's rule: when one foot is 100, only the other foot is counted;
  otherwise both feet are counted, each with the position-specific foot weight.
- The calculation starts from a weighted average, then runs the game's
  distribution-correction loop. Attributes that deviate far enough from the
  candidate's baseline contribute a signed excess, which is folded back into the
  average. The loop follows the same truncation and per-round limit rules as the
  reconstructed code.
- Multi-position players pay a position-combination cost derived from the full
  15-position familiarity table. The cost is deducted from the CA budget before
  the baseline is rebuilt.
- The result is scaled by the position-familiarity retention table (100% at
  18–20 down to 1% at 2–3).
- **Auto** evaluates every position with familiarity at least 2 and keeps the
  highest RCA. A specific position can still be selected; the position cost
  still uses the whole familiarity table.
- Zero-weight attributes (aggression, determination, flair, natural fitness and
  hidden attributes) do not change the estimate.
- The estimate updates live while CA or attributes are edited.
- The card shows the selected basis, the counted foot values and the
  position-combination cost.

The card shows the estimate, a marker for the current/edited CA, and one of:

- **over budget**: attributes imply more RCA than the stored value; FM may trim
  attributes, so raise CA or lower attributes;
- **under budget**: CA is above the attribute RCA; FM may raise attributes over
  time;
- **balanced**: within roughly 3 CA points.

The stored CA is authoritative. On a reference save of over 48,000 players the
mean absolute RCA–CA error was **0.8 CA**; 75–80% of players were within 1 point
and 100% were within 5. Developed players can still differ because FM stores CA
independently from the recommended value. A disagreement above about 8 CA is
flagged in the dialog; custom/edited players can also cause this. This is a
community reconstruction, not an official formula.

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
