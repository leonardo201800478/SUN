#!/usr/bin/env python3
"""Compare Rust slot-rate rhythm noise with the pinned Nuked OPLL_Clock trace."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load_trace(path: Path, label: str) -> list[dict[str, int]]:
    """Load JSONL trace rows, skipping the Nuked schema header when present."""
    rows: list[dict[str, int]] = []
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            if not line.strip():
                continue
            row = json.loads(line)
            if "schema" in row:
                continue
            rows.append(row)
    if not rows:
        raise ValueError(f"{label} trace is empty")
    return rows


def main() -> int:
    """Require exact per-call equality of the 23-bit DoRhythm register."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--rust-trace", required=True, type=Path)
    parser.add_argument("--expected-calls", type=int, default=144)
    args = parser.parse_args()

    nuked = load_trace(args.nuked_trace, "Nuked")
    rust = load_trace(args.rust_trace, "Rust")
    if len(nuked) != args.expected_calls or len(rust) != args.expected_calls:
        raise ValueError(
            f"trace length mismatch: Nuked={len(nuked)} Rust={len(rust)} "
            f"expected={args.expected_calls}"
        )

    for index, (left, right) in enumerate(zip(nuked, rust, strict=True)):
        if int(left["call_index"]) != index or int(right["call_index"]) != index:
            raise ValueError(f"call sequence discontinuity at row {index}")
        if int(left["rm_noise"]) != int(right["rm_noise"]):
            raise ValueError(
                f"call {index}: rm_noise mismatch Nuked={left['rm_noise']:#08x} "
                f"Rust={right['rm_noise']:#08x}"
            )

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(nuked),
                "first_register_after_reset": int(nuked[0]["rm_noise"]),
                "last_register": int(nuked[-1]["rm_noise"]),
                "compared_fields": ["rm_noise"],
                "scope": "pinned Nuked software DoRhythm recurrence from its zeroed OPLL_Reset state",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
