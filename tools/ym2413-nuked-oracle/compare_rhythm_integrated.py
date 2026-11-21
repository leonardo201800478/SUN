#!/usr/bin/env python3
"""Compare the seeded Rust Rhythm Mode PG/EG/operator service with Nuked."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from compare_fm3_integrated import ARRAY_FIELDS, SCALAR_FIELDS, compare_row, load_jsonl


RHYTHM_OUTPUT_FIELDS = ("ch_out_hh", "ch_out_tm", "ch_out_bd", "ch_out_sd", "ch_out_tc")


def compare_rhythm_row(call_index: int, nuked: dict[str, object], rust: dict[str, object]) -> None:
    """Compare the full shared FM pipeline and Rhythm Mode-specific state."""
    compare_row(call_index, nuked, rust)

    pairs = (
        ("rm_enable", "rm_enable"),
        ("op_calc_rm_enable", "op_calc_rm_enable"),
        ("rm_select", "rm_select"),
        ("rm_noise", "rhythm_noise_register"),
        ("rhythm", "rhythm_control"),
    )
    for expected_field, actual_field in pairs:
        expected = int(nuked[expected_field])
        actual = int(rust[actual_field])
        if expected != actual:
            raise ValueError(
                f"call {call_index}: {expected_field}/{actual_field} mismatch "
                f"Nuked={expected} Rust={actual}"
            )

    expected_rhythm_enabled = int(bool(int(nuked["rhythm"]) & 0x20))
    actual_rhythm_enabled = int(rust["rhythm_mode_enabled"])
    if expected_rhythm_enabled != actual_rhythm_enabled:
        raise ValueError(
            f"call {call_index}: rhythm mode bit mismatch "
            f"Nuked={expected_rhythm_enabled} Rust={actual_rhythm_enabled}"
        )

    for field in RHYTHM_OUTPUT_FIELDS:
        expected = int(nuked[field])
        actual = int(rust[field])
        if expected != actual:
            raise ValueError(
                f"call {call_index}: {field} mismatch Nuked={expected} Rust={actual}"
            )


def main() -> int:
    """Validate 147456 seeded Rhythm Mode slots against the pinned software oracle."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--rust-trace", required=True, type=Path)
    args = parser.parse_args()

    nuked_rows = load_jsonl(args.nuked_trace, "Nuked")
    rust_rows = load_jsonl(args.rust_trace, "Rust")
    if nuked_rows[0].get("schema") != "ym2413-nuked-opll-trace-v1":
        raise ValueError("Nuked input must be a full ym2413-nuked-opll-trace-v1 trace")
    nuked_samples = nuked_rows[1:]
    if len(nuked_samples) != len(rust_rows):
        raise ValueError(
            f"trace length mismatch: Nuked={len(nuked_samples)} Rust={len(rust_rows)}"
        )
    if len(nuked_samples) != 147456:
        raise ValueError(f"expected 147456 calls, got {len(nuked_samples)}")

    for index, (nuked, rust) in enumerate(zip(nuked_samples, rust_rows, strict=True)):
        if int(nuked["call_index"]) != index or int(rust["call_index"]) != index:
            raise ValueError(f"call sequence discontinuity at row {index}")
        required = (
            "rm_enable",
            "op_calc_rm_enable",
            "rm_select",
            "rm_noise",
            "rhythm",
            *RHYTHM_OUTPUT_FIELDS,
        )
        missing = [field for field in required if field not in nuked]
        rust_required = (
            "rm_enable",
            "op_calc_rm_enable",
            "rm_select",
            "rhythm_noise_register",
            "rhythm_control",
            "rhythm_mode_enabled",
            *RHYTHM_OUTPUT_FIELDS,
        )
        missing.extend(field for field in rust_required if field not in rust)
        if missing:
            raise ValueError(f"call {index}: missing Rhythm Mode fields {missing}")
        compare_rhythm_row(index, nuked, rust)

    modes = {int(row["op_calc_rm_enable"]) for row in nuked_samples}
    for bit in (0x40, 0x80):
        if not any(mode & bit for mode in modes):
            raise ValueError(f"trace never exercises effective rm_enable bit {bit:#04x}")

    for field in RHYTHM_OUTPUT_FIELDS:
        if not any(int(row[field]) != 0 for row in nuked_samples):
            raise ValueError(f"trace never produces nonzero {field}")

    if not all(int(row["rhythm_noise_register"]) >= 0 for row in rust_rows):
        raise ValueError("Rust trace did not retain the explicit zero-seed LFSR candidate")

    cycles = {int(row["cycle_before"]) for row in nuked_samples}
    if cycles != set(range(18)):
        raise ValueError("Nuked trace does not cover all eighteen serial cycles")

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(nuked_samples),
                "serial_cycles": sorted(cycles),
                "noise_seed": 0,
                "noise_seed_interpretation": "Nuked software reset candidate only; not physical silicon seed",
                "compared_scalar_fields": list(SCALAR_FIELDS),
                "compared_array_fields": list(ARRAY_FIELDS),
                "rhythm_fields": [
                    "rm_enable",
                    "op_calc_rm_enable",
                    "rm_select",
                    "rm_noise",
                    *RHYTHM_OUTPUT_FIELDS,
                ],
                "scope": "integrated Rust seeded Rhythm Mode PG/EG/AM/operator/LFSR state against pinned Nuked",
                "limitations": [
                    "software differential, not physical chip certification",
                    "does not validate operator-to-raw-DAC conversion or analog MO/RO voltage",
                ],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"YM2413 integrated Rhythm Mode differential failed: {error}", file=sys.stderr)
        raise SystemExit(2)
