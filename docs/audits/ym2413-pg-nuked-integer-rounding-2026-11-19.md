# YM2413 PG integer rounding — Nuked-OPLL convergence

**Date:** 2026-11-19
**Scope:** phase-increment integer arithmetic only.
**Primary reference:** Nuked-OPLL commit `1269cf5a783b65583b50fa2464d08be75830aaa0`.
**Secondary reference:** MAME/ymfm commit `66deabe1768df8e0ee3767688a8bfec32df27f72`.

## Finding

The previous Rust expression combined the block shift, multiplier and final division:

```rust
((corrected_fnum * ml) << block) >> 2
```

Nuked-OPLL applies two distinct integer truncation points in `OPLL_PhaseCalcIncrement`:

1. `freq = (freq << block) >> 1`
2. `pg_inc = (freq * pg_multi[mul]) >> 1`

The expressions are algebraically equivalent over real numbers, but not for integer arithmetic when the PM-adjusted frequency is odd. Combining the operations removed the intermediate truncation.

## Exhaustive arithmetic comparison

The comparison covered every legal tuple:

- FNUM: 0..511
- BLOCK: 0..7
- MUL: 0..15
- PM position: 0..7

Total: **524,288 combinations**.

Before the change, 21,504 combinations differed from the staged Nuked expression (4.10%). Rust produced increments 1..7 units higher in those cases. The first counterexample was:

| Input | Previous Rust | Nuked staged arithmetic |
|---|---:|---:|
| FNUM=64, BLOCK=0, MUL=2, PM=2 | 129 | 128 |

The Rust implementation now preserves both truncation points. The regression test pins positive and negative odd-frequency PM boundaries across all 16 MUL encodings, plus a BLOCK=1 control case. The exhaustive 524,288-tuple comparison was performed as a static arithmetic analysis; it is not represented as an executable Nuked differential test.

## Secondary MAME comparison

MAME/ymfm uses a 10.10 phase representation and routes OPLL phase calculation through its OPL phase-step helper. Its OPLL-specific frequency packing, raw-PM scale, LFO initialization and phase-unit conversion must all be normalized before an exact integer comparison is valid. The source-level inspection performed for this change did not establish that complete mapping, so this audit does **not** claim that MAME agrees with either expression.

The project decision for this narrowly scoped internal PG operation is to follow the explicit staged arithmetic in the selected primary Nuked reference. MAME remains a secondary independent cross-check; its exact PG vector comparison is still open.

## Change boundary

This correction changes only the integer rounding order in `PhaseGenerator::phase_step`.

It does **not** change:

- the 19-bit accumulator width or wrap;
- the PM table values or PM LFO cadence;
- the point at which PG is serviced;
- slot ordering or the four-input-clock slot counter;
- reset alignment or key-on phase reset;
- feedback, EG, operator evaluation, MO/RO, Scheduler or PCM integration.

## Validation status

- Static source comparison against the pinned Nuked formula: completed.
- Targeted PM/MUL rounding regression vectors: pending CI execution on this branch.
- Exhaustive 524,288-tuple static arithmetic comparison: completed in the audit; not a runtime oracle comparison.
- Nuked executable/oracle vectors and MAME output vectors: not yet generated in this environment.
- PCM-level impact and cycle/slot alignment: still open.

This is a targeted convergence correction, not a claim that the complete YM2413 pipeline is cycle-accurate or validated.
