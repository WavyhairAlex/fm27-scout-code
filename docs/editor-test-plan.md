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

## Results (2026-08-27)

All three saves were loaded from 2026-07-25 and advanced to 2026-08-27. PA,
personality and traits stayed unchanged in every run.

| Save | CA | Six key attributes | Raw attribute sum |
|---|---|---:|---:|
| A - CA Only | 150 → 152 | baseline → roughly +11 to +13 | 2378 → 2736 |
| B - Attributes Only | 110 → 110 | 100 → 85 | 2618 → 2181 |
| C - CA Plus Attributes | 150 → 150 | 100 → 96 | 2618 → 2504 |

Detailed movement:

- **A**: almost every visible attribute rose by 11–13 raw points; flair and
  natural fitness rose by 1, leadership/bravery by 5. The game filled in the
  attributes to match the new CA.
- **B**: every attribute fell by 13–15 raw points, including the six attributes
  set to 100 (they landed at 85). CA stayed at 110. Attribute-only editing at a
  low CA is not stable.
- **C**: almost every attribute was trimmed by 3–4 raw points; the six key
  attributes stayed at 96. CA stayed at 150. The game preserved the key
  attributes surprisingly well and trimmed the marginal ones to fit the budget.

The raw sum is **not** CA. FM weights attributes by position, so C can have a
lower raw sum than A while both are around CA 150: C's six high-weight attributes
consume more of the budget. Compare the direction of movement and key-attribute
retention, not the raw sum.

## Conclusion

**Use CA as the primary control.**

- For a general boost, editing CA/PA alone is sufficient; FM fills in the
  attributes over roughly a month.
- For a controlled build, set CA/PA plus a small number of key attributes. The
  key attributes are largely preserved, while marginal attributes are trimmed.
- Avoid editing visible attributes alone at a low CA; FM will pull them back
  toward the CA budget.
- Hidden attributes, personality and traits are independent and stay stable.

The earlier warning is confirmed: **CA is the budget; visible attributes are the
allocation.**
