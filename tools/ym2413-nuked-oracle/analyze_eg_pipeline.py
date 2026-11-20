#!/usr/bin/env python3
"""Audit Nuked EG pre-update selectors against captured post-update operator slots."""
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter
from pathlib import Path

NUKED_TO_RUST_STAGE = {0: 1, 1: 2, 2: 3, 3: 0}
EG_STEP_TABLE = (
    (0, 1, 0, 1, 0, 1, 0, 1),
    (0, 1, 0, 1, 1, 1, 0, 1),
    (0, 1, 1, 1, 0, 1, 1, 1),
    (0, 1, 1, 1, 1, 1, 1, 1),
)


def load_trace(path: Path) -> dict[int, dict]:
    """Load the trace while retaining every post-clock state by call index."""
    with path.open(encoding="utf-8") as stream:
        schema = json.loads(next(stream))
        assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
        assert schema["time_unit"] == "opll_clock_call"
        return {
            row["call_index"]: row
            for row in (json.loads(line) for line in stream if line.strip())
        }


def decay_step(pre: dict) -> int:
    """Reconstruct Nuked's two-bit DECAY/SUSTAIN/RELEASE increment."""
    rate_hi = pre["eg_rate_hi"]
    inc_lo = bool(pre["eg_inc_lo"])
    inc_hi = bool(pre["eg_inc_hi"])
    counter_prev = pre["eg_counter_state_prev"]
    i0 = rate_hi == 15 or (rate_hi == 14 and inc_hi)
    i1 = (
        (rate_hi == 14 and not inc_hi)
        or (rate_hi == 13 and inc_hi)
        or (rate_hi == 13 and not inc_hi and (counter_prev & 1) != 0)
        or (rate_hi == 12 and inc_hi and (counter_prev & 1) != 0)
        or (rate_hi == 12 and not inc_hi and (counter_prev & 3) == 3)
        or (inc_lo and (counter_prev & 3) == 3)
    )
    return (int(i0) << 1) | int(i1)


def replay_nuked_update(pre: dict, slot: int) -> tuple[int, int, dict]:
    """Recompute one Nuked EG update from the exact pre-call state."""
    level = pre["eg_level"][slot]
    state = pre["eg_state"][slot]
    zero = level == 0
    key_on = (pre["eg_kon"] & 2) != 0
    dokon = (pre["eg_dokon"] & 2) != 0
    off = (pre["eg_off"] & 2) != 0
    maxrate = bool(pre["eg_maxrate"])
    next_level = level

    if state != 0 and off and not dokon:
        next_level = 127
    if maxrate and dokon:
        next_level = 0

    step = 0
    if state == 0:  # ATTACK
        if not maxrate and key_on and not zero:
            shift = (
                pre["eg_inc_lo"]
                if pre["eg_rate_hi"] < 12
                else pre["eg_rate_hi"] - 11 + pre["eg_inc_hi"]
            )
            if shift > 0:
                step = (~level) >> (5 - shift)
        next_state = 1 if zero else 0
    elif state == 1:  # DECAY
        if not off and not dokon and (level >> 3) != pre["eg_sl"]:
            step = decay_step(pre)
        next_state = 2 if (level >> 3) == pre["eg_sl"] else 1
    else:  # SUSTAIN or RELEASE
        if not off and not dokon:
            step = decay_step(pre)
        next_state = state

    if not key_on:
        next_state = 3  # RELEASE
    if dokon:
        next_state = 0  # ATTACK

    consumed = {
        "level_before": level,
        "state_before": state,
        "key_on_pipeline": key_on,
        "dokon_pipeline": dokon,
        "off_pipeline": off,
        "maxrate": maxrate,
        "rate_hi": pre["eg_rate_hi"],
        "inc_lo": pre["eg_inc_lo"],
        "inc_hi": pre["eg_inc_hi"],
        "timer_shift_lock": pre["eg_timer_shift_lock"],
        "timer_low_lock": pre["eg_timer_low_lock"],
        "timer": pre["eg_timer"],
        "timer_carry": pre["eg_timer_carry"],
        "counter_state": pre["eg_counter_state"],
        "counter_state_prev": pre["eg_counter_state_prev"],
        "signed_step": step,
    }
    return next_level + step, next_state, consumed


def rust_effective_attack_rate(sample: dict) -> int:
    """Calculate the production model's effective AR for this sample."""
    base = int(sample["attack_rate"])
    if base == 0:
        return 0
    key_code = (int(sample["block"]) << 1) | (int(sample["fnum"]) >> 8)
    rks = key_code if int(sample["key_scale_rate"]) else key_code >> 2
    return min(63, 4 * base + rks)


def rust_attack_selector(counter: int, rate: int) -> dict:
    """Expose the production attack table selector for a counter candidate."""
    if not 4 <= rate < 60:
        return {"rate": rate, "enabled": False}
    if rate < 48:
        shift = 13 - rate // 4
        mask = ((1 << shift) - 1) & ~3
        column = (counter >> shift) & 7
        table_bit = EG_STEP_TABLE[rate & 3][column]
        return {
            "rate": rate,
            "shift": shift,
            "mask": mask,
            "counter_masked": counter & mask,
            "column": column,
            "table_bit": table_bit,
            "enabled": (counter & mask) == 0 and table_bit != 0,
        }
    column = (counter & 0x0C) >> 1
    shift = 16 - rate // 4
    if EG_STEP_TABLE[rate & 3][column] != 0:
        shift -= 1
    return {"rate": rate, "shift": shift, "column": column, "enabled": True}


def compress_counter_counts(counts: Counter[int]) -> list[dict]:
    """Compress nonzero mismatch counts into contiguous counter intervals."""
    values = sorted(counter for counter, count in counts.items() if count)
    if not values:
        return []
    ranges = []
    start = previous = values[0]
    total = counts[start]
    for value in values[1:]:
        if value == previous + 1:
            previous = value
            total += counts[value]
        else:
            ranges.append({"start": start, "end": previous, "mismatches": total})
            start = previous = value
            total = counts[value]
    ranges.append({"start": start, "end": previous, "mismatches": total})
    return ranges


def main() -> int:
    """Validate reference update replay and explain the first Rust divergences."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=Path)
    parser.add_argument("--rust-trace", required=True, type=Path)
    args = parser.parse_args()

    trace = load_trace(args.trace)
    with args.samples.open(encoding="utf-8", newline="") as stream:
        samples = list(csv.DictReader(stream))
    with args.rust_trace.open(encoding="utf-8") as stream:
        candidates = {
            (int(row["call_index"]), int(row["slot"])): row
            for row in (json.loads(line) for line in stream if line.strip())
        }

    reference_errors = []
    level_mismatches = 0
    stage_mismatches = 0
    by_slot: Counter[int] = Counter()
    by_counter: Counter[int] = Counter()
    first_level = None
    first_stage = None

    for sample in samples:
        call = int(sample["# call_index"])
        slot = int(sample["slot"])
        counter = int(sample["global_counter"])
        assert call > 0 and call - 1 in trace, f"missing pre-update row for call {call}"
        pre = trace[call - 1]
        post = trace[call]
        expected_level, expected_state, consumed = replay_nuked_update(pre, slot)
        observed_level = post["eg_level"][slot]
        observed_state = post["eg_state"][slot]

        if (expected_level, expected_state) != (observed_level, observed_state):
            reference_errors.append({
                "call_index": call,
                "slot": slot,
                "expected": [expected_level, expected_state],
                "observed": [observed_level, observed_state],
            })

        candidate = candidates[(call, slot)]
        rust_level = int(candidate["candidate_level_initial_127_deferred_unwrapped"])
        rust_stage = int(candidate["candidate_stage_initial_127_deferred_unwrapped"])
        level_bad = rust_level != observed_level
        stage_bad = rust_stage != NUKED_TO_RUST_STAGE[observed_state]
        level_mismatches += int(level_bad)
        stage_mismatches += int(stage_bad)
        by_slot[slot] += int(level_bad)
        by_counter[counter] += int(level_bad)

        if level_bad and first_level is None:
            rate = rust_effective_attack_rate(sample)
            first_level = {
                "call_index": call,
                "slot": slot,
                "global_counter": counter,
                "observed_nuked_level": observed_level,
                "candidate_rust_level": rust_level,
                "rust_attack_selector": rust_attack_selector(counter, rate),
                "nuked_consumed_state": consumed,
            }
        if stage_bad and first_stage is None:
            first_stage = {
                "call_index": call,
                "slot": slot,
                "observed_nuked_state": observed_state,
                "expected_rust_stage": NUKED_TO_RUST_STAGE[observed_state],
                "candidate_rust_stage": rust_stage,
                "observed_nuked_level": observed_level,
                "candidate_rust_level": rust_level,
                "nuked_consumed_state": consumed,
            }

    assert not reference_errors, (
        f"Nuked pre-state replay failed for {len(reference_errors)} samples: "
        f"{reference_errors[:3]}"
    )
    print(json.dumps({
        "sample_count": len(samples),
        "reference_internal_update_replay": "PASS",
        "reference_replay_mismatches": 0,
        "rust_candidate_level_mismatches": level_mismatches,
        "rust_candidate_stage_mismatches": stage_mismatches,
        "rust_candidate_level_mismatches_by_slot": dict(sorted(by_slot.items())),
        "rust_candidate_level_mismatch_counter_ranges": compress_counter_counts(by_counter),
        "first_rust_level_mismatch": first_level,
        "first_rust_stage_mismatch": first_stage,
        "time_alignment": "exact call_index and serial slot; previous OPLL_Clock post-state is current pre-state",
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
