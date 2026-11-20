#!/usr/bin/env python3
"""Report candidate Rust EG divergence from the pinned Nuked trace."""
from __future__ import annotations
import argparse
import json
from pathlib import Path

# Nuked: ATTACK=0, DECAY=1, SUSTAIN=2, RELEASE=3.
# Rust candidate: DAMP=0, ATTACK=1, DECAY=2, SUSTAIN=3.
NUKED_TO_RUST_STAGE = {0: 1, 1: 2, 2: 3, 3: 0}

def load_jsonl(path: Path) -> list[dict]:
    """Read a JSONL file into ordered records."""
    with path.open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream if line.strip()]

def compare(rows: list[dict], level_key: str, stage_key: str) -> dict:
    """Count exact level/stage mismatches without shifting or auto-aligning time."""
    level_mismatches = 0
    stage_mismatches = 0
    max_level_error = 0
    first_level_mismatch = None
    first_stage_mismatch = None
    for row in rows:
        level_error = abs(row[level_key] - row["observed_level"])
        max_level_error = max(max_level_error, level_error)
        if level_error:
            level_mismatches += 1
            if first_level_mismatch is None:
                first_level_mismatch = {
                    "call_index": row["call_index"],
                    "slot": row["slot"],
                    "global_counter": row["global_counter"],
                    "observed": row["observed_level"],
                    "candidate": row[level_key],
                }
        expected_stage = NUKED_TO_RUST_STAGE[row["observed_state"]]
        if row[stage_key] != expected_stage:
            stage_mismatches += 1
            if first_stage_mismatch is None:
                first_stage_mismatch = {
                    "call_index": row["call_index"],
                    "slot": row["slot"],
                    "observed_nuked_state": row["observed_state"],
                    "expected_rust_stage": expected_stage,
                    "candidate_stage": row[stage_key],
                }
    return {
        "level_mismatches": level_mismatches,
        "stage_mismatches": stage_mismatches,
        "max_absolute_level_error": max_level_error,
        "first_level_mismatch": first_level_mismatch,
        "first_stage_mismatch": first_stage_mismatch,
    }

def main() -> int:
    """Compare both explicit reset-level candidates and print diagnostic metrics."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-trace", required=True, type=Path)
    args = parser.parse_args()
    rows = load_jsonl(args.rust_trace)
    assert rows, "Rust EG trace is empty"
    calls = [row["call_index"] for row in rows]
    assert len(calls) == len(set(calls)), "duplicate call indices in Rust trace"
    summary = {
        "sample_count": len(rows),
        "time_alignment": "exact call_index and per-slot global_counter; no auto-alignment",
        "candidate_initial_level_0": compare(
            rows, "candidate_level_initial_0", "candidate_stage_initial_0"
        ),
        "candidate_initial_level_127_unwrapped": compare(
            rows, "candidate_level_initial_127", "candidate_stage_initial_127"
        ),
        "candidate_initial_level_127_wrapped": compare(
            rows, "candidate_level_initial_127_wrapped",
            "candidate_stage_initial_127_wrapped"
        ),
        "candidate_initial_level_127_wrapped_deferred_attack": compare(
            rows, "candidate_level_initial_127_wrapped_deferred",
            "candidate_stage_initial_127_wrapped_deferred"
        ),
        "candidate_initial_level_127_deferred_attack_unwrapped": compare(
            rows, "candidate_level_initial_127_deferred_unwrapped",
            "candidate_stage_initial_127_deferred_unwrapped"
        ),
        "interpretation": (
            "diagnostic only; mismatches do not select hardware truth or fail this tool"
        ),
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
