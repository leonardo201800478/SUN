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


def main() -> None:
    """Exhaustively compare base PG and PM arithmetic across legal inputs."""
    base_total = 0
    base_mismatches = 0
    pm_total = 0
    same_phase_mismatches = 0
    rotation_mismatches = [0] * 8

    for fnum in range(512):
        for block in range(8):
            for mul in range(16):
                rust_base = rust_phase_step(fnum, block, mul, 0)
                mame_base_normalized = (
                    mame_phase_step_10_10(fnum, block, mul, 0, False) >> 1
                )
                base_total += 1
                base_mismatches += mame_base_normalized != rust_base

                for rust_position in range(8):
                    rust_step = rust_phase_step(fnum, block, mul, rust_position)
                    mame_same_position = (
                        mame_phase_step_10_10(
                            fnum, block, mul, rust_position, True
                        )
                        >> 1
                    )
                    pm_total += 1
                    same_phase_mismatches += mame_same_position != rust_step

                    for rotation in range(8):
                        mame_position = (rust_position + rotation) & 7
                        mame_step = (
                            mame_phase_step_10_10(
                                fnum, block, mul, mame_position, True
                            )
                            >> 1
                        )
                        rotation_mismatches[rotation] += mame_step != rust_step

    best_rotation = min(range(8), key=rotation_mismatches.__getitem__)
    result = {
        "classification": "SOURCE_LEVEL_STATIC_ARITHMETIC_COMPARISON",
        "base": {
            "tuples": base_total,
            "mismatches_after_10_10_to_9_fractional_bit_normalization": base_mismatches,
        },
        "pm": {
            "tuples": pm_total,
            "mismatches_same_position": same_phase_mismatches,
            "best_cyclic_rotation": best_rotation,
            "mismatches_at_best_cyclic_rotation": rotation_mismatches[best_rotation],
            "mismatches_by_rotation": rotation_mismatches,
        },
        "limitations": [
            "No MAME executable or runtime state was captured.",
            "The PM phase origin is not assumed; all eight cyclic rotations are counted.",
            "This does not prove which implementation is physically correct.",
            "No Rust production code is changed by this diagnostic.",
        ],
    }
    print(json.dumps(result, indent=2))

    if base_mismatches != 0 or rotation_mismatches[best_rotation] == 0:
        raise SystemExit("unexpected source-level comparison result")


if __name__ == "__main__":
    main()
