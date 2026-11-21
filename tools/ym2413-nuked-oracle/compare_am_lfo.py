#!/usr/bin/env python3
"""Differentially validate the serial AM LFO against pinned Nuked OPLL_DoLFO."""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> int:
    """Replay every serial AM shift/carry call and compare all state latches."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()

    with args.nuked_trace.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if len(rows) < 2 or rows[0].get("schema") != "ym2413-nuked-opll-trace-v1":
        raise ValueError("input must be a full ym2413-nuked-opll-trace-v1 trace")
    samples = rows[1:]
    required = (
        "call_index",
        "cycle_before",
        "am_calc_lfo_counter",
        "am_calc_testmode",
        "lfo_am_counter",
        "lfo_am_dir",
        "lfo_am_car",
        "lfo_am_step",
        "lfo_am_out",
    )
    for row in samples:
        missing = [field for field in required if field not in row]
        if missing:
            raise ValueError(f"call {row.get('call_index')}: missing {missing}")
    cycles = {int(row["cycle_before"]) for row in samples}
    if cycles != set(range(18)):
        raise ValueError("trace must cover all eighteen serial cycles")
    testmodes = {int(row["am_calc_testmode"]) for row in samples}
    for bit in (0x02, 0x08):
        if not any(mode & bit for mode in testmodes):
            raise ValueError(f"trace does not exercise Test Mode bit {bit:#04x}")

    with tempfile.TemporaryDirectory(prefix="ym2413-am-lfo-diff-") as temp_dir:
        input_csv = Path(temp_dir) / "serial_am_lfo_inputs.csv"
        with input_csv.open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            for row in samples:
                writer.writerow(
                    (
                        row["call_index"],
                        row["cycle_before"],
                        row["am_calc_lfo_counter"],
                        row["am_calc_testmode"],
                        row["lfo_am_counter"],
                        row["lfo_am_dir"],
                        row["lfo_am_car"],
                        row["lfo_am_step"],
                        row["lfo_am_out"],
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
            "serial_am_lfo_trace",
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
            raise ValueError("Rust serial AM LFO replay failed:\n" + completed.stderr[-8000:])
        result = json.loads(completed.stdout.strip().splitlines()[-1])

    if result.get("status") != "PASS" or int(result.get("compared_calls", -1)) != len(samples):
        raise ValueError(f"unexpected serial AM LFO comparator result: {result}")

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(samples),
                "serial_cycles": sorted(cycles),
                "testmode_values": sorted(testmodes),
                "compared_fields": [
                    "lfo_am_counter",
                    "lfo_am_dir",
                    "lfo_am_car",
                    "lfo_am_step",
                    "lfo_am_out",
                ],
                "scope": "serial Nuked OPLL_DoLFO AM shift/carry state",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"YM2413 serial AM LFO differential failed: {error}", file=sys.stderr)
        raise SystemExit(2)
