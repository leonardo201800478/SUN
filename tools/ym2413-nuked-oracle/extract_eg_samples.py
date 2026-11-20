#!/usr/bin/env python3
"""Extract per-operator EG update points from a pinned Nuked JSONL trace."""
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter
from pathlib import Path

# Logical EG array index -> channel in the pinned Nuked-OPLL serial schedule.
CH_OFFSET = (1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8, 0)


def main() -> int:
    """Select active channel/operator EG slots and preserve per-row parameters."""
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
    with args.trace.open(encoding="utf-8") as stream:
        schema = json.loads(next(stream))
        assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
        assert schema["time_unit"] == "opll_clock_call"
        rows = (json.loads(line) for line in stream if line.strip())
        with args.output.open("w", newline="", encoding="utf-8") as output:
            writer = csv.writer(output)
            writer.writerow((
                "# call_index", "slot", "channel", "is_carrier",
                "global_counter", "key_on", "observed_level", "observed_state",
                "attack_rate", "decay_rate", "release_rate", "sustain_level",
                "key_scale_rate", "fnum", "block", "sustain_on", "sustained_tone",
            ))
            for row in rows:
                # This is the array entry updated during this OPLL_Clock call.
                slot = (row["cycle_before"] + 16) % 18
                channel = CH_OFFSET[slot]
                # eg_kon is shifted once inside OPLL_EnvelopeGenerate.
                # Post-call bit 2 is the bit-1 key-on value consumed by this
                # slot's EG update; raw channel KON is earlier in the pipeline.
                if channel not in selected_channels or (row["eg_kon"] & 0x04) == 0:
                    continue

                global_counter = row["call_index"] // 18
                assert row["eg_counter_state"] == (global_counter & 0xff), (
                    f"EG counter phase mismatch at call {row['call_index']}: "
                    f"state={row['eg_counter_state']} expected={global_counter & 0xff}"
                )
                # The EG array index is (cycle_before + 16) % 18, but
                # PreparePatch1 selects modulator/carrier from the live chip
                # cycle. Do not derive mcsel from the delayed EG array index.
                mcsel = ((row["cycle_before"] + 1) // 3) & 1
                is_carrier = mcsel == 1
                writer.writerow((
                    row["call_index"], slot, channel, int(is_carrier),
                    global_counter, 1, row["eg_level"][slot], row["eg_state"][slot],
                    row["c_adrr"][0], row["c_adrr"][1], row["c_adrr"][2],
                    row["c_sl"], row["c_ksr"], row["fnum"][channel], row["block"][channel],
                    row["son"][channel], row["c_et"],
                ))
                counts_by_channel[channel] += 1
                counts_by_slot[slot] += 1

    assert counts_by_channel, "no selected channel EG samples found during KEY-ON"
    print(json.dumps({
        "selected_channels": sorted(selected_channels),
        "samples_by_channel": dict(sorted(counts_by_channel.items())),
        "samples_by_eg_slot": dict(sorted(counts_by_slot.items())),
        "total_samples": sum(counts_by_channel.values()),
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
