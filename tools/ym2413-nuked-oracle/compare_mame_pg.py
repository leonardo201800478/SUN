#!/usr/bin/env python3
"""Static source-level arithmetic comparison for MAME/ymfm OPLL PG.

Sources are pinned in the accompanying audit. This reproduces the expressions
in MAME's opl_compute_phase_step/opll_registers::compute_phase_step and
compares them with the production Rust PhaseGenerator arithmetic. It is not a
MAME executable trace and does not assume LFO phase alignment.
"""

from __future__ import annotations

import json

ML_TABLE = (1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20, 24, 24, 30, 30)
RUST_PM_TABLE = (
    (0, 0, 0, 0, 0, 0, 0, 0),
    (0, 0, 1, 0, 0, 0, -1, 0),
    (0, 1, 2, 1, 0, -1, -2, -1),
    (0, 1, 3, 1, 0, -1, -3, -1),
    (0, 2, 4, 2, 0, -2, -4, -2),
    (0, 2, 5, 2, 0, -2, -5, -2),
    (0, 3, 6, 3, 0, -3, -6, -3),
    (0, 3, 7, 3, 0, -3, -7, -3),
)
# MAME ymfm_opl.cpp, opl_clock_noise_and_lfo(), fixed OPLL PM depth=1.
MAME_PM_SCALE = (8, 4, 0, -4, -8, -4, 0, 4)


def rust_phase_step(fnum: int, block: int, mul: int, pm_position: int) -> int:
    """Evaluate the production Rust PG integer expression."""
    corrected = 2 * fnum + RUST_PM_TABLE[fnum >> 6][pm_position]
    block_scaled = (corrected << block) >> 1
    return (block_scaled * ML_TABLE[mul]) >> 1


def mame_phase_step_10_10(
    fnum: int, block: int, mul: int, lfo_position: int, pm_enabled: bool
) -> int:
    """Reproduce MAME's OPLL phase helper in its native 10.10 phase units."""
    # opll_registers::ch_block_freq(): BLOCK[2:0] + FNUM[8:0].
    block_freq = (block << 9) | fnum

    # opll_registers::compute_phase_step() shifts block_freq left by one
    # before calling opl_compute_phase_step().
    shifted = block_freq << 1
    adjusted_fnum = (shifted & 0x3FF) << 2
    if pm_enabled:
        fnum_high = (shifted >> 7) & 0x07
        adjusted_fnum += (MAME_PM_SCALE[lfo_position] * fnum_high) >> 1
    adjusted_fnum &= 0xFFF

    adjusted_block = (shifted >> 10) & 0x07
    phase_step = (adjusted_fnum << adjusted_block) >> 2

    # MAME's OPLL multiple cache maps MUL to x.1 units; zero maps to 1/2.
    multiple = ML_TABLE[mul]
    return (phase_step * multiple) >> 1


def trunc_toward_zero(numerator: int, denominator: int) -> int:
    """Divide integers while discarding the fractional part toward zero."""
    magnitude = abs(numerator) // denominator
    return magnitude if numerator >= 0 else -magnitude


def main() -> None:
    """Exhaustively compare base PG and PM arithmetic across legal inputs."""
    base_total = 0
    base_floor_mismatches = 0
    base_exact_mismatches = 0
    base_half_unit_cases = 0
    pm_total = 0
    pm_floor_mismatches = 0
    pm_exact_mismatches = 0
    rotation_floor_mismatches = [0] * 8
    rotation_exact_mismatches = [0] * 8
    floor_mismatches_by_fnum_high = [0] * 8
    exact_mismatches_by_fnum_high = [0] * 8

    # MAME's PM counter origin differs by a cyclic offset. At offset 6, its
    # raw 1.3 PM values quantize toward zero to the Rust/Nuked 8x8 PM table.
    quantized_mame_pm_table = []
    for fnum_high in range(8):
        row = []
        for rust_position in range(8):
            raw_pm = MAME_PM_SCALE[(rust_position + 6) & 7]
            row.append(trunc_toward_zero(raw_pm * fnum_high, 8))
        quantized_mame_pm_table.append(row)
    pm_table_matches_after_quantization = all(
        tuple(quantized_mame_pm_table[row]) == RUST_PM_TABLE[row]
        for row in range(8)
    )

    for fnum in range(512):
        for block in range(8):
            for mul in range(16):
                rust_base = rust_phase_step(fnum, block, mul, 0)
                mame_base_native = mame_phase_step_10_10(
                    fnum, block, mul, 0, False
                )
                base_total += 1
                base_floor_mismatches += (mame_base_native >> 1) != rust_base
                base_exact_mismatches += mame_base_native != (2 * rust_base)
                base_half_unit_cases += mame_base_native & 1

                for rust_position in range(8):
                    rust_step = rust_phase_step(fnum, block, mul, rust_position)
                    mame_native_same_position = mame_phase_step_10_10(
                        fnum, block, mul, rust_position, True
                    )
                    pm_total += 1
                    pm_floor_mismatches += (mame_native_same_position >> 1) != rust_step
                    pm_exact_mismatches += mame_native_same_position != (2 * rust_step)

                    for rotation in range(8):
                        mame_position = (rust_position + rotation) & 7
                        mame_native = mame_phase_step_10_10(
                            fnum, block, mul, mame_position, True
                        )
                        floor_mismatch = (mame_native >> 1) != rust_step
                        exact_mismatch = mame_native != (2 * rust_step)
                        rotation_floor_mismatches[rotation] += floor_mismatch
                        rotation_exact_mismatches[rotation] += exact_mismatch
                        if rotation == 6:
                            floor_mismatches_by_fnum_high[fnum >> 6] += floor_mismatch
                            exact_mismatches_by_fnum_high[fnum >> 6] += exact_mismatch

    best_rotation = min(range(8), key=rotation_floor_mismatches.__getitem__)
    result = {
        "classification": "SOURCE_LEVEL_STATIC_ARITHMETIC_COMPARISON",
        "phase_unit_normalization": (
            "MAME operator phase is 10.10; Rust PG is 19-bit with 9 fractional bits. "
            "Exact common-unit equality is MAME_step == 2 * Rust_step; floor normalization "
            "uses MAME_step >> 1 and separately reports discarded half-unit cases."
        ),
        "base": {
            "tuples": base_total,
            "floor_normalized_mismatches": base_floor_mismatches,
            "exact_common_unit_mismatches": base_exact_mismatches,
            "half_rust_lsb_cases": base_half_unit_cases,
        },
        "pm": {
            "tuples": pm_total,
            "floor_normalized_same_position_mismatches": pm_floor_mismatches,
            "exact_common_unit_same_position_mismatches": pm_exact_mismatches,
            "best_cyclic_rotation": best_rotation,
            "pm_table_matches_rust_after_offset_6_and_truncation_toward_zero": pm_table_matches_after_quantization,
            "quantized_mame_pm_table": quantized_mame_pm_table,
            "floor_normalized_mismatches_by_fnum_high_at_offset_6": floor_mismatches_by_fnum_high,
            "exact_common_unit_mismatches_by_fnum_high_at_offset_6": exact_mismatches_by_fnum_high,
            "floor_normalized_mismatches_at_best_rotation": rotation_floor_mismatches[best_rotation],
            "exact_common_unit_mismatches_at_best_rotation": rotation_exact_mismatches[best_rotation],
            "floor_normalized_mismatches_by_rotation": rotation_floor_mismatches,
            "exact_common_unit_mismatches_by_rotation": rotation_exact_mismatches,
        },
        "limitations": [
            "No MAME executable or runtime state was captured.",
            "The PM phase origin is not assumed; all eight cyclic rotations are counted.",
            "This does not prove which implementation is physically correct.",
            "No Rust production code is changed by this diagnostic.",
        ],
    }
    print(json.dumps(result, indent=2))

    if (
        base_floor_mismatches != 0
        or not pm_table_matches_after_quantization
        or rotation_floor_mismatches[best_rotation] == 0
    ):
        raise SystemExit("unexpected source-level comparison result")


if __name__ == "__main__":
    main()
