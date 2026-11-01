# Editor comparison test plan

The first test (`FM27 Scout Editor Test - Simone`) mixed CA, visible attributes,
hidden attributes, personality and a trait. It showed that FM rebalances visible
attributes toward CA, while PA, hidden attributes, personality and traits stayed
put. This plan isolates the two remaining questions:

1. Is a CA-only edit enough to raise the player's attributes?
2. Are visible-attribute edits without a matching CA reverted?

## Fixture

- Source save: `BB KING - Parma Calcio (v03).fm`, build `27.0.0+2486695`,
  game date 2026-07-25. This is a backup copy, not the live save.
- Player: **Simone Lontani**, uid `2000395878`, age 18, Parma, STC.
- Baseline: CA 110, PA 160, raw attribute sum 2378, no traits.

Baseline values used below:

```text
finishing 64, pace 64, acceleration 65, dribbling 59, passing 59, technique 49
consistency 65, important_matches 70, injury_proneness 20, versatility 75, dirtiness 55
personality: adaptability 13, ambition 14, loyalty 15, pressure 12,
             professionalism 14, sportsmanship 10, temperament 12, controversy 4
```

## Test saves

All three are generated from the same source and have PA 200, so PA is constant.
Only CA and the six visible attributes differ.

| Save | CA | PA | Six visible attributes | Hidden/personality/trait |
|---|---:|---:|---|---|
| A - CA Only | 150 | 200 | unchanged | unchanged |
| B - Attributes Only | 110 | 200 | set to raw 100 | unchanged |
| C - CA Plus Attributes | 150 | 200 | set to raw 100 | unchanged |

The six visible attributes are `finishing`, `pace`, `acceleration`, `dribbling`,
`passing`, `technique`.

The start saves are in the FM games directory and in `target/editor-test/`:

```text
FM27 Scout Test A - CA Only.fm
FM27 Scout Test B - Attributes Only.fm
FM27 Scout Test C - CA Plus Attributes.fm
```

## Running the test

1. Quit and restart FM so the new saves appear in the load menu.
2. Load **A**, advance to **2026-08-28** (the same end date as the first test),
   then use **Save As** to create a separate file such as `A - month.fm`.
   Do not overwrite the original A file.
3. Repeat for **B** and **C**, using the same end date.
4. Keep the start files; they are the baselines for the comparison script.

## Comparing a result

```bash
cargo build --release -p fm27-cli

python3 scripts/compare_editor_test.py \
  --baseline "target/editor-test/FM27 Scout Test A - CA Only.fm" \
  --after "/path/to/A - month.fm" \
  --uid 2000395878
```

The script prints CA/PA, value, personality, traits and every changed raw
attribute.

## How to read the result

| Observation | Interpretation |
|---|---|
| A's visible attributes rise toward CA 150 | CA-only editing works; FM fills in the attributes |
| B's six 100s fall back | visible attributes without CA are not stable |
| B's six 100s stay near 100 | attribute-only editing is viable, but check CA/PA growth |
| C's six 100s stay and the other attributes rise | CA + key attributes is the best controlled workflow |
| C's six 100s fall while A's attributes rise | CA distribution wins; recommend CA-only by default |
| Personality/traits/hidden attributes change | report it; the hidden/personality edit path needs investigation |

The expected result from the first test is A and C filling in the remaining
attributes, while B's six 100s may be trimmed toward CA 110. That would confirm
the current warning: **CA is the budget; visible attributes are the allocation.**
