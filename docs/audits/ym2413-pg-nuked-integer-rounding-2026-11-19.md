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

The Rust implementation now preserves both truncation points. The regression test pins positive and negative odd-frequency PM boundaries across all 16 MUL encodings, plus a BLOCK=1 control case. The exhaustive 524,288-tuple comparison was performed as a static arithmetic analysis. It is now complemented by an executable Nuked differential run described below.

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
- Targeted PM/MUL rounding regression vectors: CI PASS on HEAD `15278ea4fcb19669a7f3b3c10e8a89ab3cebb379`.
- Exhaustive 524,288-tuple static arithmetic comparison: completed in the audit; not a runtime oracle comparison.
- Rust workspace regression, Rust quality/unit regression and documentation patch hygiene: CI PASS on the recorded HEAD.
- Executable Nuked differential: **147,456 OPLL_Clock calls**, all increments compared exactly against the production Rust `PhaseGenerator::phase_step`; 16,373 calls had PM enabled, and all eight `lfo_vib_counter` positions (0..7) were covered. CI run `44652587533`: SUCCESS.
- MAME/ymfm output vectors: not yet generated; exact independent cross-check remains open.
- PCM-level impact and cycle/slot alignment: still open.

This is a targeted convergence correction, not a claim that the complete YM2413 pipeline is cycle-accurate or validated.


## Executable differential result — 2026-11-19

The external Nuked adapter captures the exact inputs consumed by
`OPLL_PhaseCalcIncrement` immediately before each `OPLL_Clock` call and the
calculated `pg_inc` immediately after that call's calculation. The Rust
comparator feeds those inputs into the production `PhaseGenerator::phase_step`
implementation; it does not reimplement the formula in Python.

- Nuked revision: `1269cf5a783b65583b50fa2464d08be75830aaa0`.
- Rust correction branch: PR #1075.
- Differential harness: PR #1077.
- Calls compared: **147,456**.
- PM-enabled calls: **16,373**.
- PM positions covered with nonzero FNUM and PM enabled: **0, 1, 2, 3, 4, 5, 6, 7**.
- Mismatches: **0**.
- Workflow: run `44652587533`, job `136010866086` — SUCCESS.

**What this proves:** exact equality of the integer PG increment for the
captured legal register/patch inputs, including every PM LFO position exercised
by the pinned reference.

**What this does not prove:** equality of accumulated phase at a shared
physical instant, correspondence between Nuked's 18 serial steps and the
Rust 18-slot/four-input-clock representation, reset alignment, PG service
subphase, EG timing, feedback timing, PCM output, or physical silicon fidelity.
MAME/ymfm triangulation remains pending.
