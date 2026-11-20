#!/usr/bin/env python3
"""Compare the production Rust PG increment with pinned Nuked trace rows.

The Nuked adapter captures the exact inputs consumed by OPLL_PhaseCalcIncrement
before each OPLL_Clock call and the resulting pg_inc after that call. This
script feeds those inputs to the real Rust PhaseGenerator implementation.
It compares arithmetic only; it does not map the two cores' slot pipelines.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def load_nuked_trace(path: Path) -> list[dict[str, object]]:
    """Load JSONL trace and validate the fields needed for PG comparison."""
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]

    if len(rows) < 2 or rows[0].get("schema") != "ym2413-nuked-opll-trace-v1":
        raise ValueError("input is not a supported Nuked-OPLL trace")
    if rows[0].get("time_unit") != "opll_clock_call":
        raise ValueError("trace time unit must be opll_clock_call")

    samples = rows[1:]
    required = (
        "call_index",
        "calc_fnum",
        "calc_block",
        "calc_mul",
        "calc_vib",
        "calc_lfo_vib_counter",
        "pg_inc_after",
    )
    for row in samples:
        missing = [field for field in required if field not in row]
        if missing:
            raise ValueError(f"call {row.get('call_index')}: missing {missing}")
    return samples


def main() -> int:
    """Run the Rust PG implementation and compare every traced increment."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()

    samples = load_nuked_trace(args.nuked_trace)
    if not any(int(row["calc_fnum"]) > 0 for row in samples):
        raise ValueError("Nuked trace never applied the programmed nonzero FNUM")
    if not any(int(row["calc_mul"]) > 0 for row in samples):
        raise ValueError("Nuked trace never selected a nonzero instrument multiplier")

    with tempfile.TemporaryDirectory(prefix="ym2413-pg-diff-") as temp_dir:
        input_csv = Path(temp_dir) / "rust_pg_inputs.csv"
        with input_csv.open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            for row in samples:
                writer.writerow(
                    (
                        row["call_index"],
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
            "pg_trace",
            "--",
            str(input_csv),
        ]
        completed = subprocess.run(
            command,
            cwd=args.repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
        rust_rows = [json.loads(line) for line in completed.stdout.splitlines() if line]
        if len(rust_rows) != len(samples):
            raise ValueError(
                f"Rust emitted {len(rust_rows)} rows for {len(samples)} Nuked samples"
            )

        mismatches = []
        for nuked, rust in zip(samples, rust_rows, strict=True):
            if nuked["call_index"] != rust["call_index"]:
                raise ValueError("trace call indices are not aligned")
            expected = int(nuked["pg_inc_after"])
            actual = int(rust["phase_step"])
            if expected != actual:
                mismatches.append(
                    {
                        "call_index": nuked["call_index"],
                        "cycle_before": nuked.get("cycle_before"),
                        "fnum": nuked["calc_fnum"],
                        "block": nuked["calc_block"],
                        "mul": nuked["calc_mul"],
                        "vib": nuked["calc_vib"],
                        "lfo_vib_counter": nuked["calc_lfo_vib_counter"],
                        "nuked_pg_inc": expected,
                        "rust_phase_step": actual,
                    }
                )

    if mismatches:
        print(json.dumps({"status": "FAIL", "mismatch_count": len(mismatches),
                          "first_mismatches": mismatches[:20]}, indent=2))
        return 1

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(samples),
                "comparison": "Nuked OPLL_PhaseCalcIncrement output vs Rust PhaseGenerator::phase_step",
                "scope": "PG integer increment only; no slot/phase accumulator alignment claim",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"PG differential comparison failed: {error}", file=sys.stderr)
        raise SystemExit(2)
