#!/usr/bin/env python3
"""Validate the pinned Nuked-OPLL single-channel EG characterization trace."""
from __future__ import annotations
import argparse
import json
from pathlib import Path

def load_trace(path: Path) -> list[dict]:
    """Load JSONL trace records, including the schema descriptor."""
    with path.open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream if line.strip()]

def main() -> int:
    """Validate trace shape, channel events, and serial EG update behavior."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--expected-calls", required=True, type=int)
    args = parser.parse_args()
    rows = load_trace(args.trace)
    assert len(rows) == args.expected_calls + 1, (
        f"expected {args.expected_calls + 1} JSONL rows, got {len(rows)}"
    )
    schema, samples = rows[0], rows[1:]
    assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
    assert schema["time_unit"] == "opll_clock_call"
    assert schema["sample_point"] == "after_OPLL_Clock"
    required = {
        "call_index", "cycle_before", "cycle_after", "c_adrr", "c_et",
        "c_sl", "c_ksr_freq", "eg_counter_state", "eg_counter_state_prev",
        "eg_timer", "eg_timer_carry", "eg_timer_low_lock", "eg_timer_shift",
        "eg_timer_shift_lock", "eg_timer_shift_stop", "eg_state", "eg_level",
        "eg_kon", "eg_dokon", "eg_off", "eg_rate", "eg_rate_hi",
        "eg_maxrate", "eg_zerorate", "eg_inc_lo", "eg_inc_hi", "eg_sl",
        "eg_silent", "kon",
    }
    for index, sample in enumerate(samples):
        missing = required.difference(sample)
        assert not missing, f"call {index}: missing fields {sorted(missing)}"
        assert sample["call_index"] == index
        assert sample["cycle_before"] == index % 18
        assert sample["cycle_after"] == (index + 1) % 18
        assert len(sample["eg_state"]) == 18
        assert len(sample["eg_level"]) == 18
        assert len(sample["kon"]) == 9
        assert all(0 <= state <= 3 for state in sample["eg_state"])
        assert all(0 <= level <= 127 for level in sample["eg_level"])
    active_calls = [s["call_index"] for s in samples if s["kon"][0] == 1]
    inactive_after_keyoff = [
        s for s in samples if s["call_index"] >= 16_100 and s["kon"][0] == 0
    ]
    assert active_calls, "channel 0 KEY-ON was not observed by Nuked"
    assert inactive_after_keyoff, "channel 0 KEY-OFF was not observed by Nuked"
    patch_rows = [
        s for s in samples
        if s["c_adrr"] == [10, 5, 7] and s["c_et"] == 1 and s["c_sl"] == 4
    ]
    if not patch_rows:
        observed = sorted({(tuple(s["c_adrr"]), s["c_et"], s["c_sl"], s["c_instr"], s["c_op"]) for s in samples})
        raise AssertionError(f"configured user-patch EG parameters were not observed; observed tuples={observed[:24]}")
    changed_level_slots = [0] * 18
    changed_state_slots = [0] * 18
    eg_dokon_nonzero = 0
    timer_changes = 0
    counter_changes = 0
    previous = None
    for sample in samples:
        if sample["eg_dokon"]:
            eg_dokon_nonzero += 1
        if previous is not None:
            changed_levels = [
                i for i, pair in enumerate(zip(previous["eg_level"], sample["eg_level"], strict=True))
                if pair[0] != pair[1]
            ]
            changed_states = [
                i for i, pair in enumerate(zip(previous["eg_state"], sample["eg_state"], strict=True))
                if pair[0] != pair[1]
            ]
            assert len(changed_levels) <= 1, (
                f"multiple EG level slots changed at call {sample['call_index']}: {changed_levels}"
            )
            assert len(changed_states) <= 1, (
                f"multiple EG state slots changed at call {sample['call_index']}: {changed_states}"
            )
            for slot in changed_levels:
                changed_level_slots[slot] += 1
            for slot in changed_states:
                changed_state_slots[slot] += 1
            timer_changes += previous["eg_timer"] != sample["eg_timer"]
            counter_changes += previous["eg_counter_state"] != sample["eg_counter_state"]
        previous = sample
    # In the pinned source, cycle c updates array index (c + 16) % 18.
    # ch_offset[2]/mcsel(2) selects channel 0 carrier => EG index 2;
    # ch_offset[17]/mcsel(17) selects channel 0 modulator => EG index 17.
    assert changed_level_slots[2] > 0, "channel 0 carrier EG slot 2 never changed"
    assert changed_level_slots[17] > 0, "channel 0 modulator EG slot 17 never changed"
    assert timer_changes > 0, "Nuked EG timer did not advance"
    assert counter_changes > 0, "Nuked EG counter state did not advance"
    assert eg_dokon_nonzero > 0, "no EG KEY-ON transition was observed"
    summary = {
        "calls": len(samples),
        "channel_0_keyon_first_call": active_calls[0],
        "channel_0_keyoff_observed": True,
        "user_patch_rows": len(patch_rows),
        "eg_dokon_nonzero_rows": eg_dokon_nonzero,
        "eg_timer_changes": timer_changes,
        "eg_counter_state_changes": counter_changes,
        "eg_level_changes_by_slot": changed_level_slots,
        "eg_state_changes_by_slot": changed_state_slots,
        "channel_0_carrier_eg_slot": 2,
        "channel_0_modulator_eg_slot": 17,
        "timebase": "Nuked OPLL_Clock calls; not XIN clocks",
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
