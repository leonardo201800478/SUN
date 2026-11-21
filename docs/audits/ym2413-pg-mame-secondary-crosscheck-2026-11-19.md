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

The script tests every possible cyclic PM phase-origin offset. Offset 6 maps MAME's raw PM scale to `[0, 4, 8, 4, 0, -4, -8, -4]`. Expressing the correction in Rust's corrected-FNUM units as `trunc_toward_zero(raw_pm × FNUM[8:6] / 8)` reproduces every entry of the Rust 8×8 PM table exactly.

Even with that phase-origin mapping, the final increments do not compare exactly because MAME preserves fractional PM corrections and performs its own intermediate shifts:

- Floor-normalized PM increment mismatches at offset 6: **135,136 / 524,288**.
- Exact common-unit PM increment mismatches at offset 6: **140,625 / 524,288**.
- The largest mismatch groups are odd FNUM[8:6] rows, where MAME's fractional half-step information is discarded by the Rust integer PM table.

## Interpretation and decision

The base/no-PM arithmetic agrees after phase-unit normalization. A more detailed PM analysis shows that MAME's raw LFO waveform is compatible with the Rust PM table after a cyclic phase-origin offset of 6 and truncation toward zero. The remaining phase-increment mismatches come from MAME retaining fractional PM adjustments and applying integer shifts at different points, not from a different quantized 8×8 PM table.

This is a **recorded secondary-reference arithmetic difference**, not proof that either implementation is physically correct or that MAME is defective. The quantized PM table shape agrees after phase-origin normalization; the increment results differ because the two implementations retain and truncate fractional PM information differently. The comparison is static source-level arithmetic; it does not run a MAME executable, capture its internal state, establish full PM clock alignment, or measure silicon.

Project decision remains unchanged:

- Nuked-OPLL is the primary software oracle for YM2413 PG convergence.
- MAME/ymfm remains a secondary independent source for triangulation.
- Do not modify the Rust PM table: after phase-origin offset 6 and truncation toward zero, the MAME raw PM waveform produces the same 8×8 table.
- Investigate the fractional PM arithmetic and intermediate rounding differences before any production change.
- Preserve the exact Nuked differential result: 147,456 calls, all eight PM positions, zero PG increment mismatches.

## Validation boundary

The script is deterministic and exhaustive for the formulas and tables listed above. It does not establish full PG phase-accumulator equivalence, slot/subphase alignment, EG, feedback, PCM, or physical chip fidelity. No MAME or Nuked implementation source is copied into the Rust backend.


## Re-execution in the current-main PG workflow (2026-11-20)

The static source-level audit was rerun by the PG differential workflow on PR #1119, run `44840972144`, job `136648936949` — SUCCESS.

The regenerated report retained the same classifications:

- Base/no-PM: 65,536 tuples; zero floor-normalized mismatches; 256 exact common-unit differences, all half-Rust-LSB cases.
- PM: 524,288 tuples; 404,965 same-position floor-normalized mismatches before phase-origin normalization.
- Best cyclic PM phase-origin rotation: 6.
- At rotation 6, the quantized MAME PM table matches the Rust table after truncation toward zero.
- Final PM increment comparison still differs: 135,136 floor-normalized mismatches and 140,625 exact common-unit mismatches at the best rotation.

This remains a source-level arithmetic comparison. No MAME executable or runtime state was captured. The result is not a MAME PASS for the production PG increment and does not supersede Nuked as the primary software compatibility oracle.
