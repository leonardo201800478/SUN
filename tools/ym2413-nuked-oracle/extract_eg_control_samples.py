#!/usr/bin/env python3
"""Extract every selected operator update with the pre-call EG control latches."""
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter
from pathlib import Path

# Logical EG array index -> channel in the pinned Nuked-OPLL serial schedule.
CH_OFFSET = (1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8, 0)


def main() -> int:
    """Pair each serial EG update's pre-call state with its post-call result."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--channels", default="0")
    args = parser.parse_args()
    selected_channels = {int(value) for value in args.channels.split(",")}
    if not selected_channels or any(channel < 0 or channel > 8 for channel in selected_channels):
        raise ValueError("--channels must contain channel indices 0 through 8")

    counts_by_channel: Counter[int] = Counter()
    counts_by_slot: Counter[int] = Counter()
    counts_by_key_state: Counter[str] = Counter()

    with args.trace.open(encoding="utf-8") as stream:
        schema = json.loads(next(stream))
        assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
        assert schema["time_unit"] == "opll_clock_call"
        previous = None
        with args.output.open("w", newline="", encoding="utf-8") as output:
            writer = csv.writer(output)
            writer.writerow((
                "call_index", "slot", "channel", "level_before", "state_before",
                "key_on", "off", "dokon", "sustain_level", "rate_hi",
                "inc_lo", "inc_hi", "max_rate", "counter_state_prev",
                "expected_level", "expected_state",
            ))
            for line in stream:
                if not line.strip():
                    continue
                current = json.loads(line)
                if previous is None:
                    previous = current
                    continue

                call_index = current["call_index"]
                slot = (current["cycle_before"] + 16) % 18
                channel = CH_OFFSET[slot]
                if channel in selected_channels:
                    key_on = (previous["eg_kon"] & 0x02) != 0
                    off = (previous["eg_off"] & 0x02) != 0
                    dokon = (previous["eg_dokon"] & 0x02) != 0
                    writer.writerow((
                        call_index,
                        slot,
                        channel,
                        previous["eg_level"][slot],
                        previous["eg_state"][slot],
                        int(key_on),
                        int(off),
                        int(dokon),
                        previous["eg_sl"],
                        previous["eg_rate_hi"],
                        previous["eg_inc_lo"],
                        previous["eg_inc_hi"],
                        previous["eg_maxrate"],
                        previous["eg_counter_state_prev"],
                        current["eg_level"][slot],
                        current["eg_state"][slot],
                    ))
                    counts_by_channel[channel] += 1
                    counts_by_slot[slot] += 1
                    key_state = "key_on" if key_on else "key_off"
                    counts_by_key_state[key_state] += 1
                previous = current

    assert counts_by_channel, "no selected channel EG updates found"
    print(json.dumps({
        "selected_channels": sorted(selected_channels),
        "samples_by_channel": dict(sorted(counts_by_channel.items())),
        "samples_by_eg_slot": dict(sorted(counts_by_slot.items())),
        "samples_by_key_pipeline": dict(sorted(counts_by_key_state.items())),
        "total_samples": sum(counts_by_channel.values()),
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
