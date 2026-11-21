#!/usr/bin/env python3
"""Characterize preset-ROM latch differences without mislabeling candidates as hardware truth."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

EXACT_FIELDS = (
    "cycle_before",
    "cycle_after",
    "c_instr",
    "c_op",
    "c_fnum",
    "c_block",
)
PATCH_FIELDS = (
    "c_tl",
    "c_fb",
    "c_am",
    "c_vib",
    "c_ksr",
    "c_ksl",
    "c_multi",
    "c_adrr",
    "c_et",
    "c_sl",
    "c_dc",
    "c_dm",
)


def load_rows(path: Path, label: str) -> list[dict[str, object]]:
    """Load a non-empty JSONL trace."""
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows:
        raise ValueError(f"{label} trace is empty")
    return rows


def normalized(value: object) -> object:
    """Normalize JSON arrays and scalar values for stable equality."""
    if isinstance(value, list):
        return tuple(int(item) for item in value)
    return int(value)


def main() -> int:
    """Verify serial selection/timing and report candidate preset-latch deltas."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--rust-trace", required=True, type=Path)
    args = parser.parse_args()

    nuked_rows = load_rows(args.nuked_trace, "Nuked")
    rust_rows = load_rows(args.rust_trace, "Rust")
    if nuked_rows[0].get("schema") != "ym2413-nuked-opll-trace-v1":
        raise ValueError("Nuked input must be a full ym2413-nuked-opll-trace-v1 trace")
    nuked = nuked_rows[1:]
    if len(nuked) != len(rust_rows) or len(nuked) != 147456:
        raise ValueError(
            f"trace length mismatch or incomplete duration: Nuked={len(nuked)} "
            f"Rust={len(rust_rows)} expected=147456"
        )

    observed_instruments: set[int] = set()
    cycles: set[int] = set()
    user_patch_signatures: set[tuple[object, ...]] = set()
    candidate_differences: dict[str, dict[str, object]] = {}

    for index, (expected, actual) in enumerate(zip(nuked, rust_rows, strict=True)):
        if int(expected["call_index"]) != index or int(actual["call_index"]) != index:
            raise ValueError(f"call sequence discontinuity at row {index}")

        for field in EXACT_FIELDS:
            left, right = normalized(expected[field]), normalized(actual[field])
            if left != right:
                raise ValueError(
                    f"call {index}: {field} mismatch Nuked={left} Rust={right}"
                )

        instrument = int(expected["c_instr"])
        operator = int(expected["c_op"])
        observed_instruments.add(instrument)
        cycles.add(int(expected["cycle_before"]))

        signature = tuple(normalized(expected[field]) for field in PATCH_FIELDS)
        if instrument == 0:
            rust_signature = tuple(normalized(actual[field]) for field in PATCH_FIELDS)
            if signature != rust_signature:
                raise ValueError(
                    f"call {index}: user-patch latch mismatch "
                    f"Nuked={signature} Rust={rust_signature}"
                )
            user_patch_signatures.add(signature)
            continue

        for field in PATCH_FIELDS:
            left, right = normalized(expected[field]), normalized(actual[field])
            if left == right:
                continue
            # Carrier TL is channel volume, not a preset-ROM field.
            if field == "c_tl" and operator == 1:
                raise ValueError(
                    f"call {index}: carrier volume latch mismatch "
                    f"Nuked={left} Rust={right}"
                )
            key = f"instrument_{instrument}_operator_{operator}_{field}"
            entry = candidate_differences.setdefault(
                key,
                {
                    "instrument": instrument,
                    "operator": operator,
                    "field": field,
                    "mismatch_calls": 0,
                    "first_call": index,
                    "nuked_value": left,
                    "rust_candidate_value": right,
                },
            )
            entry["mismatch_calls"] = int(entry["mismatch_calls"]) + 1

    expected_instruments = set(range(16))
    if observed_instruments != expected_instruments:
        raise ValueError(
            f"expected instrument codes 0..15, observed {sorted(observed_instruments)}"
        )
    if cycles != set(range(18)):
        raise ValueError(f"serial cycle coverage incomplete: {sorted(cycles)}")
    if len(user_patch_signatures) < 2:
        raise ValueError("live user-patch update was not observed in the instrument-0 latches")

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(nuked),
                "serial_cycles": sorted(cycles),
                "observed_instrument_codes": sorted(observed_instruments),
                "exact_fields": list(EXACT_FIELDS),
                "user_patch_fields": list(PATCH_FIELDS),
                "user_patch_signatures": len(user_patch_signatures),
                "preset_candidate_differences": list(candidate_differences.values()),
                "preset_candidate_difference_count": sum(
                    int(item["mismatch_calls"])
                    for item in candidate_differences.values()
                ),
                "scope": (
                    "exact serial selection/timing and user-patch latches; "
                    "preset-ROM parameter differences are explicitly reported "
                    "as candidate data, not treated as hardware-certified values"
                ),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"YM2413 preset latch characterization failed: {error}", file=sys.stderr)
        raise SystemExit(2)
