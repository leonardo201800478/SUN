#!/usr/bin/env python3
"""Extract the shared YM2413 EG timer pipeline from a Nuked-OPLL trace."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

FIELDS = [
    "call_index",
    "slot_index",
    "next_rate",
    "expected_timer",
    "expected_carry",
    "expected_counter_state",
    "expected_counter_state_prev",
    "expected_shift",
    "expected_shift_stop",
    "expected_low_lock",
    "expected_shift_lock",
    "expected_rate_hi",
    "expected_inc_lo",
    "expected_inc_hi",
    "expected_max_rate",
    "expected_zero_rate",
]


def main() -> int:
    """Convert timer fields from the JSONL oracle trace into a compact CSV."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected-calls", type=int, default=30000)
    args = parser.parse_args()

    rows_written = 0
    expected_slot = 0
    with args.trace.open(encoding="utf-8") as source, args.output.open(
        "w", encoding="utf-8", newline=""
    ) as destination:
        first_line = source.readline()
        if not first_line:
            raise ValueError("trace is empty")
        json.loads(first_line)  # Validate the trace schema/header object.
        writer = csv.DictWriter(destination, fieldnames=FIELDS)
        writer.writeheader()

        for line_number, line in enumerate(source, start=2):
            row = json.loads(line)
            slot = int(row["cycle_before"])
            if slot != expected_slot:
                raise ValueError(
                    f"slot sequence mismatch at line {line_number}: "
                    f"expected {expected_slot}, got {slot}"
                )
            expected_slot = (expected_slot + 1) % 18

            rate = int(row["eg_rate"])
            if not 0 <= rate <= 63:
                raise ValueError(f"invalid EG rate at line {line_number}: {rate}")

            writer.writerow({
                "call_index": int(row["call_index"]),
                "slot_index": slot,
                "next_rate": rate,
                "expected_timer": int(row["eg_timer"]),
                "expected_carry": int(row["eg_timer_carry"]),
                "expected_counter_state": int(row["eg_counter_state"]),
                "expected_counter_state_prev": int(row["eg_counter_state_prev"]),
                "expected_shift": int(row["eg_timer_shift"]),
                "expected_shift_stop": int(row["eg_timer_shift_stop"]),
                "expected_low_lock": int(row["eg_timer_low_lock"]),
                "expected_shift_lock": int(row["eg_timer_shift_lock"]),
                "expected_rate_hi": int(row["eg_rate_hi"]),
                "expected_inc_lo": int(row["eg_inc_lo"]),
                "expected_inc_hi": int(row["eg_inc_hi"]),
                "expected_max_rate": int(row["eg_maxrate"]),
                "expected_zero_rate": int(row["eg_zerorate"]),
            })
            rows_written += 1

    if rows_written != args.expected_calls:
        raise ValueError(
            f"expected {args.expected_calls} timer samples, got {rows_written}"
        )
    print(json.dumps({
        "samples": rows_written,
        "first_slot": 0,
        "last_slot": (rows_written - 1) % 18,
        "output": str(args.output),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
