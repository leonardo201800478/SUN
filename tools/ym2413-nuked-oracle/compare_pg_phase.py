#!/usr/bin/env python3
"""Compare all 18 Nuked PG accumulators against production Rust state."""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
import tempfile
from pathlib import Path

PHASE_MASK = (1 << 19) - 1


def main() -> int:
    """Replay the 18-slot PG pipeline and compare every accumulator per call."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()

    with args.nuked_trace.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]

    if len(rows) < 2 or rows[0].get("schema") != "ym2413-nuked-opll-pg-phase-trace-v1":
        raise ValueError("input is not a supported Nuked PG phase trace")
    if rows[0].get("time_unit") != "opll_clock_call":
        raise ValueError("trace time unit must be opll_clock_call")

    samples = rows[1:]
    required = (
        "call_index",
        "cycle_before",
        "calc_fnum",
        "calc_block",
        "calc_mul",
        "calc_vib",
        "calc_lfo_vib_counter",
        "pg_phase",
    )
    for row in samples:
        missing = [field for field in required if field not in row]
        if missing:
            raise ValueError(f"call {row.get('call_index')}: missing {missing}")
        if len(row["pg_phase"]) != 18:
            raise ValueError(f"call {row['call_index']}: expected 18 Nuked phase entries")

    if not any(int(row["calc_fnum"]) > 0 for row in samples):
        raise ValueError("trace never applied nonzero FNUM")
    if not any(int(row["calc_block"]) == 7 and int(row["calc_fnum"]) > 0 for row in samples):
        raise ValueError("trace never exercised BLOCK=7")

    with tempfile.TemporaryDirectory(prefix="ym2413-pg-phase-diff-") as temp_dir:
        input_csv = Path(temp_dir) / "rust_pg_phase_inputs.csv"
        with input_csv.open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            for row in samples:
                writer.writerow(
                    (
                        row["call_index"],
                        row["cycle_before"],
                        row["calc_fnum"],
                        row["calc_block"],
                        row["calc_mul"],
                        row["calc_vib"],
                        row["calc_lfo_vib_counter"],
                    )
                )

        command = [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "--manifest-path",
            str(args.repo_root / "native/ym2413/Cargo.toml"),
            "--example",
            "pg_phase_trace",
            "--",
            str(input_csv),
        ]
        completed = subprocess.run(
            command,
            cwd=args.repo_root,
            check=False,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            raise ValueError(
                "Rust PG phase example failed:\\n" + completed.stderr[-6000:]
            )

        rust_rows = [json.loads(line) for line in completed.stdout.splitlines() if line]
        if len(rust_rows) != len(samples):
            raise ValueError(
                f"Rust emitted {len(rust_rows)} phase rows for {len(samples)} Nuked samples"
            )

        mismatches = []
        wrap_events = 0
        previous_rust_phases = [0] * 18
        for nuked, rust in zip(samples, rust_rows, strict=True):
            if nuked["call_index"] != rust["call_index"]:
                raise ValueError("phase trace call indices are not aligned")
            if len(rust["phases"]) != 18:
                raise ValueError(f"call {rust['call_index']}: expected 18 Rust phase entries")

            for slot, (nuked_phase, rust_phase) in enumerate(
                zip(nuked["pg_phase"], rust["phases"], strict=True)
            ):
                expected = int(nuked_phase) & PHASE_MASK
                actual = int(rust_phase)
                if actual < previous_rust_phases[slot]:
                    wrap_events += 1
                if expected != actual and len(mismatches) < 20:
                    mismatches.append(
                        {
                            "call_index": nuked["call_index"],
                            "cycle_before": nuked["cycle_before"],
                            "slot": slot,
                            "nuked_phase_low19": expected,
                            "rust_phase": actual,
                            "nuked_phase_raw": int(nuked_phase),
                        }
                    )
                if expected != actual and len(mismatches) == 20:
                    # Keep scanning all rows but bound the diagnostic payload.
                    pass
            previous_rust_phases = [int(value) for value in rust["phases"]]

    if mismatches:
        print(json.dumps({"status": "FAIL", "first_mismatches": mismatches}, indent=2))
        return 1

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(samples),
                "phase_slots_per_call": 18,
                "phase_values_compared": len(samples) * 18,
                "rust_19_bit_wrap_events": wrap_events,
                "comparison": "Nuked pg_phase[18] low 19 bits vs Rust PhaseGenerator[18]",
                "scope": "18-slot accumulator pipeline with no KEY-ON/reset events",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"PG phase differential comparison failed: {error}", file=sys.stderr)
        raise SystemExit(2)
