#!/usr/bin/env python3
"""Compare integrated Rust serial PG state against the pinned Nuked OPLL trace."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

PHASE_MASK = (1 << 19) - 1


def load_trace(path: Path) -> list[dict[str, object]]:
    """Load and validate the external Nuked serial PG trace."""
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if len(rows) < 2 or rows[0].get("schema") != "ym2413-nuked-opll-pg-phase-trace-v1":
        raise ValueError("input is not a supported Nuked PG trace")
    if rows[0].get("time_unit") != "opll_clock_call":
        raise ValueError("trace time unit must be opll_clock_call")

    samples = rows[1:]
    required = (
        "call_index",
        "cycle_before",
        "cycle_after",
        "calc_lfo_vib_counter",
        "phase_next_reset",
        "calc_eg_dokon",
        "pg_inc_after",
        "pg_phase_next",
        "pg_phase",
        "pg_out",
    )
    for row in samples:
        missing = [field for field in required if field not in row]
        if missing:
            raise ValueError(f"call {row.get('call_index')}: missing {missing}")
        if len(row["pg_phase"]) != 18:
            raise ValueError(f"call {row['call_index']}: expected 18 Nuked phase entries")
    return samples


def main() -> int:
    """Run the Rust serial PG service and compare all observable PG pipeline state."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--events", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()

    samples = load_trace(args.nuked_trace)
    reset_events = sum(bool(row["phase_next_reset"]) for row in samples)
    eg_key_events = sum(int(row["calc_eg_dokon"]) != 0 for row in samples)
    if reset_events == 0 or eg_key_events == 0:
        raise ValueError(
            f"serial trace did not exercise a KEY-ON phase reset: resets={reset_events}, "
            f"nonzero EG DOKON calls={eg_key_events}"
        )

    command = [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "--manifest-path",
        str(args.repo_root / "native/ym2413/Cargo.toml"),
        "--example",
        "pg_serial_trace",
        "--",
        str(args.events),
        str(len(samples)),
    ]
    completed = subprocess.run(
        command,
        cwd=args.repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise ValueError("Rust serial PG example failed:\n" + completed.stderr[-8000:])

    rust_rows = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    if len(rust_rows) != len(samples):
        raise ValueError(f"Rust emitted {len(rust_rows)} rows for {len(samples)} Nuked calls")

    mismatches: list[dict[str, object]] = []
    wrap_events = 0
    active_slots: set[int] = set()
    previous_phases = [0] * 18

    for index, (nuked, rust) in enumerate(zip(samples, rust_rows, strict=True)):
        if int(nuked["call_index"]) != int(rust["call_index"]):
            raise ValueError("Nuked and Rust call indices are not aligned")

        scalar_pairs = (
            ("cycle_before", "cycle_before", None),
            ("cycle_after", "cycle_after", None),
            ("calc_lfo_vib_counter", "lfo_vib_counter_before", None),
            ("pg_inc_after", "pg_increment", None),
            ("pg_phase_next", "pg_phase_next", PHASE_MASK),
            ("pg_out", "pg_output", 0x03FF),
        )
        for expected_key, actual_key, mask in scalar_pairs:
            expected = int(nuked[expected_key])
            if mask is not None:
                expected &= mask
            actual = int(rust[actual_key])
            if expected != actual and len(mismatches) < 20:
                mismatches.append(
                    {
                        "call_index": nuked["call_index"],
                        "field": expected_key,
                        "nuked": expected,
                        "rust": actual,
                    }
                )

        nuked_phases = nuked["pg_phase"]
        rust_phases = rust["phases"]
        if len(rust_phases) != 18:
            raise ValueError(f"call {rust['call_index']}: expected 18 Rust phase entries")
        for slot, (expected_raw, actual_raw) in enumerate(
            zip(nuked_phases, rust_phases, strict=True)
        ):
            expected = int(expected_raw) & PHASE_MASK
            actual = int(actual_raw)
            if actual != 0:
                active_slots.add(slot)
            if actual < previous_phases[slot]:
                wrap_events += 1
            if expected != actual and len(mismatches) < 20:
                mismatches.append(
                    {
                        "call_index": nuked["call_index"],
                        "field": "pg_phase",
                        "slot": slot,
                        "nuked_low19": expected,
                        "rust": actual,
                    }
                )
        previous_phases = [int(value) for value in rust_phases]

        if index + 1 < len(samples):
            next_nuked_vib = int(samples[index + 1]["calc_lfo_vib_counter"])
            rust_vib_after = int(rust["lfo_vib_counter_after"])
            if next_nuked_vib != rust_vib_after and len(mismatches) < 20:
                mismatches.append(
                    {
                        "call_index": nuked["call_index"],
                        "field": "lfo_vib_counter_after",
                        "nuked_next_call": next_nuked_vib,
                        "rust": rust_vib_after,
                    }
                )

    if active_slots != set(range(18)):
        raise ValueError(f"serial PG trace did not activate all 18 slots: {sorted(active_slots)}")
    if wrap_events == 0:
        raise ValueError("serial PG trace did not exercise any 19-bit accumulator wrap")

    if mismatches:
        print(
            json.dumps(
                {
                    "status": "FAIL",
                    "mismatch_count_at_least": len(mismatches),
                    "first_mismatches": mismatches,
                },
                indent=2,
            )
        )
        return 1

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(samples),
                "phase_values_compared": len(samples) * 18,
                "active_phase_slots": sorted(active_slots),
                "rust_19_bit_wrap_events": wrap_events,
                "phase_reset_events": reset_events,
                "eg_dokon_nonzero_calls": eg_key_events,
                "comparison": (
                    "integrated Rust core-owned serial PG service vs Nuked: "
                    "18 accumulators, pg_inc, pg_phase_next, pg_out, cycle and PM counter"
                ),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Serial PG differential failed: {error}", file=sys.stderr)
        raise SystemExit(2)
