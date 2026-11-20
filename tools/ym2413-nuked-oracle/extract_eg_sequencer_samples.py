#!/usr/bin/env python3
"""Extract one complete serial-cycle input and post-state row from Nuked."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

# Nuked ch_offset[cycle] for the pinned reference revision.
CH_OFFSET = (1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8, 0)


def main() -> int:
    """Pair selected current-cycle register latches with all post-cycle EG fields."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    calls = 0
    with args.trace.open(encoding="utf-8") as source, args.output.open(
        "w", encoding="utf-8", newline=""
    ) as destination:
        schema = json.loads(next(source))
        assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
        assert schema["time_unit"] == "opll_clock_call"
        writer = csv.writer(destination)
        writer.writerow((
            "call_index", "cycle", "attack_rate", "decay_rate", "release_rate",
            "sustain_level", "key_scale_rate", "ksr_frequency", "sustain_on",
            "sustained_tone", "key_on", "expected_operator_slot",
            "expected_level", "expected_state", "expected_timer",
            "expected_carry", "expected_counter_state", "expected_counter_state_prev",
            "expected_shift", "expected_shift_stop", "expected_low_lock",
            "expected_shift_lock", "expected_rate_hi", "expected_inc_lo",
            "expected_inc_hi", "expected_max_rate", "expected_zero_rate",
            "expected_rate", "expected_eg_kon", "expected_eg_off",
            "expected_eg_dokon", "expected_eg_sl",
        ))

        previous_c_sl = 0
        for line in source:
            if not line.strip():
                continue
            row = json.loads(line)
            cycle = int(row["cycle_before"])
            channel = CH_OFFSET[cycle]
            operator_slot = (cycle + 16) % 18
            writer.writerow((
                row["call_index"],
                cycle,
                row["c_adrr"][0],
                row["c_adrr"][1],
                row["c_adrr"][2],
                previous_c_sl,
                row["c_ksr"],
                row["c_ksr_freq"],
                row["son"][channel],
                row["c_et"],
                row["kon"][channel],
                operator_slot,
                row["eg_level"][operator_slot],
                row["eg_state"][operator_slot],
                row["eg_timer"],
                row["eg_timer_carry"],
                row["eg_counter_state"],
                row["eg_counter_state_prev"],
                row["eg_timer_shift"],
                row["eg_timer_shift_stop"],
                row["eg_timer_low_lock"],
                row["eg_timer_shift_lock"],
                row["eg_rate_hi"],
                row["eg_inc_lo"],
                row["eg_inc_hi"],
                row["eg_maxrate"],
                row["eg_zerorate"],
                row["eg_rate"],
                row["eg_kon"],
                row["eg_off"],
                row["eg_dokon"],
                row["eg_sl"],
            ))
            previous_c_sl = row["c_sl"]
            calls += 1

    if calls == 0:
        raise ValueError("trace contained no OPLL_Clock samples")
    print(json.dumps({"calls": calls, "output": str(args.output)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
