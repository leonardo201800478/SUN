# YM2413 PG — 18-slot phase accumulator differential

**Date:** 2026-11-19  
**Primary oracle:** Nuked-OPLL `1269cf5a783b65583b50fa2464d08be75830aaa0`  
**Rust base:** PR #1075, PG integer-rounding correction  
**Harness/comparator:** PRs #1076 and #1077  
**CI:** run `44654562391`, job `136017489127` — SUCCESS

## What was compared

The external Nuked adapter emits, after every `OPLL_Clock` call:

- serial cycle before/after the call;
- the `c_fnum`, `c_block`, `c_multi`, `c_vib`, and `lfo_vib_counter` values consumed by the PG calculation during that call;
- `pg_inc` after `OPLL_PhaseCalcIncrement`;
- the complete `pg_phase[18]` array after `OPLL_PhaseGenerate`.

The Rust-side example uses the production `PhaseGenerator::advance` method for 18 independent slot accumulators. It applies the previous call's PG inputs to slot `(current_cycle + 17) % 18`, matching Nuked's one-call pipeline.

## Slot mapping and pipeline derivation

The mapping was derived from the pinned source rather than inferred from matching output:

1. Nuked's `ch_offset[18]` is `[1,2,0,1,2,3,4,5,3,4,5,6,7,8,6,7,8,0]`.
2. `OPLL_PreparePatch2` selects `mcsel = ((cycles + 1) / 3) & 1` and loads channel/operator parameters for the next serial cycle.
3. For the pre-call inputs captured at cycle `n`, `ch_offset[n-1]` and `mcsel(n-1)` correspond to Rust `YM2413_SLOT_MAP[n]`.
4. `OPLL_PhaseGenerate` updates `pg_phase[(cycles + 17) % 18]` with `pg_phase_next + pg_inc`, then reads `pg_phase[cycles]`. Thus each call updates the previous slot while selecting the current slot.
5. `OPLL_Operator` forms its effective phase with `(op_mod + pg_out) & 0x3ff`. Since `pg_out = phase >> 9`, the effective accumulator state is the low 19 bits, matching Rust's `YM2413_PHASE_MASK`.

This establishes the index relationship for the tested melodic, no-KEY-ON sequence. It does not claim that all other pipeline subphases or reset/key-on paths have been validated.

## Controlled register coverage

The harness programs all nine melodic channels with instrument 1 and nonzero FNUM values spanning every FNUM[8:6] row:

`[32, 64, 128, 192, 256, 320, 384, 448, 511]`

BLOCK values 0..7 are covered. Instrument 1 has PM enabled for both operators in the pinned Nuked patch data. The sequence contains no KEY-ON writes and does not enable Rhythm Mode.

## Differential results

| Measurement | Result |
|---|---:|
| `OPLL_Clock` calls | **147,456** |
| PM-enabled calls | **146,885** |
| FNUM[8:6] rows covered | **0..7** |
| BLOCK values covered | **0..7** |
| PM LFO positions covered | **0..7** |
| PG increments compared | **147,456** |
| Increment mismatches | **0** |
| Phase accumulator values compared | **2,654,208** (18 per call) |
| Low-19-bit phase mismatches | **0** |
| Rust 19-bit wrap events observed | **5,076** |
| Active phase slots | **all 18** |

The comparator does not duplicate the Rust arithmetic in Python. It generates input rows from the Nuked trace, runs the production Rust example, and compares every phase value against `pg_phase[slot] & 0x7ffff`.

## Interpretation

For the controlled melodic sequence, the following are now demonstrated against the pinned software oracle:

- exact PG integer increments across all eight PM LFO positions, all eight FNUM high-bit rows, and all BLOCK values;
- exact low-19-bit phase accumulator state for all 18 operator slots at every sampled call;
- correct 18-slot update order and previous-slot increment pipeline for this no-KEY-ON path;
- correct 19-bit wrap behavior across 5,076 observed wraps.

This is materially stronger than comparing only static increments. It is still a software differential result, not a physical measurement or proof of complete YM2413 cycle accuracy.

## Explicitly not covered

- KEY-ON phase reset and `eg_dokon` alignment;
- test-mode phase reset;
- Rhythm Mode PG overrides and percussion phase taps;
- four-XIN subphase correspondence and absolute physical clock alignment;
- EG, feedback, modulator-to-carrier pipeline, waveform/EXP output, PCM and mixer;
- every ROM patch's MUL/VIB/AM combinations;
- executable MAME/ymfm phase-state differential;
- physical silicon fidelity.

No production YM2413 runtime code was changed by this phase-array comparison. The new Rust example and Python comparator are validation tooling only.
