# YM2413 PG — MAME/ymfm secondary source-level cross-check

**Date:** 2026-11-19  
**Primary reference:** Nuked-OPLL `1269cf5a783b65583b50fa2464d08be75830aaa0`  
**Secondary source:** MAME/ymfm `66deabe1768df8e0ee3767688a8bfec32df27f72`  
**Scope:** OPLL PG integer arithmetic and PM transfer function, source-level only.

## Source path audited

The comparison script `tools/ym2413-nuked-oracle/compare_mame_pg.py` reproduces these MAME/ymfm source operations:

1. `opll_registers::ch_block_freq()` packs BLOCK and FNUM from registers 0x20/0x10.
2. `opll_registers::compute_phase_step()` shifts the packed frequency left by one and calls `opl_compute_phase_step()`.
3. `opl_compute_phase_step()` extracts the 10-bit fractional FNUM, applies raw PM, masks the adjusted FNUM to 12 bits, shifts by BLOCK, applies the cached MUL factor and performs the final shift.
4. `opll_registers::clock_noise_and_lfo()` calls the shared LFO routine with fixed AM/PM depth arguments `1, 1`.
5. `opl_clock_noise_and_lfo()` uses the PM scale sequence `[8, 4, 0, -4, -8, -4, 0, 4]`.
6. MAME's operator phase accumulator is documented in the source as 10.10; the Rust YM2413 PG is a 19-bit accumulator with 9 fractional bits. The comparison therefore reports both exact common-unit equality (`MAME_step == 2 * Rust_step`) and floor normalization (`MAME_step >> 1`).

Relevant pinned source files:

- `3rdparty/ymfm/src/ymfm_opl.h`
- `3rdparty/ymfm/src/ymfm_opl.cpp`
- `3rdparty/ymfm/src/ymfm_fm.h`
- `3rdparty/ymfm/src/ymfm_fm.ipp`

MAME's OPLL default prescale is 4 and its sample-rate calculation divides by prescale × 18 operators (72 input clocks per generated sample). This provides a useful timing-scale reference, but is not by itself proof that all internal update boundaries map one-to-one to Nuked's serial calls or the Rust slot model.

## Exhaustive static arithmetic result

The source-level script evaluates:

- 512 FNUM values;
- 8 BLOCK values;
- 16 MUL encodings;
- 8 PM/LFO positions.

| Comparison | Tuples | Mismatches |
|---|---:|---:|
| Base PG, floor-normalized from MAME 10.10 to Rust 9 fractional bits | 65,536 | **0** |
| Base PG, exact common-unit equality | 65,536 | **256** |
| PM, same numeric LFO position, floor-normalized | 524,288 | **404,965** |
| PM, same numeric LFO position, exact common-unit equality | 524,288 | **407,425** |

The 256 base exact-unit differences are half-Rust-LSB values retained by MAME's additional fractional precision; flooring MAME's 10.10 increment to the Rust 9-fractional-bit unit yields equality for every base tuple.

The script also tests every possible cyclic PM phase-origin offset. The best cyclic rotation is offset 6:

- Floor-normalized PM mismatches at best offset: **135,136 / 524,288**.
- Exact common-unit PM mismatches at best offset: **140,625 / 524,288**.

No cyclic phase rotation makes the PM transfer functions equal.

## Interpretation and decision

The base/no-PM arithmetic agrees after phase-unit normalization. The PM arithmetic does not: MAME/ymfm's source-level LFO/PM transfer function differs materially from the pinned Nuked-OPLL/Rust PM table, even after trying all eight cyclic phase-origin offsets.

This is a **recorded secondary-reference divergence**, not proof that either implementation is physically correct or that MAME is defective. The comparison is static source-level arithmetic; it does not run a MAME executable, capture its internal state, establish PM clock alignment, or measure silicon.

Project decision remains unchanged:

- Nuked-OPLL is the primary software oracle for YM2413 PG convergence.
- MAME/ymfm remains a secondary independent source for triangulation.
- Do not modify the Rust PM table to match MAME solely because this source-level comparison differs.
- Investigate the PM depth/shape and clock-domain interpretation before any production change.
- Preserve the exact Nuked differential result: 147,456 calls, all eight PM positions, zero PG increment mismatches.

## Validation boundary

The script is deterministic and exhaustive for the formulas and tables listed above. It does not establish full PG phase-accumulator equivalence, slot/subphase alignment, EG, feedback, PCM, or physical chip fidelity. No MAME or Nuked implementation source is copied into the Rust backend.
