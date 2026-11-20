#!/usr/bin/env python3
"""Extract channel-0 EG update points from a pinned Nuked JSONL trace."""
from __future__ import annotations
import argparse
import csv
import json
from pathlib import Path

def main() -> int:
    """Select the two channel-0 EG array entries and preserve their clock phase."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    with args.trace.open(encoding="utf-8") as stream:
        schema = json.loads(next(stream))
        assert schema["schema"] == "ym2413-nuked-opll-trace-v1"
        assert schema["time_unit"] == "opll_clock_call"
        rows = (json.loads(line) for line in stream if line.strip())
        with args.output.open("w", newline="", encoding="utf-8") as output:
            writer = csv.writer(output)
            writer.writerow(("# call_index", "slot", "global_counter", "key_on",
                             "observed_level", "observed_state"))
            count = 0
            for row in rows:
                slot = (row["cycle_before"] + 16) % 18
                if slot not in (2, 17) or row["kon"][0] != 1:
                    continue
                global_counter = row["call_index"] // 18
                assert row["eg_counter_state"] == (global_counter & 0xff), (
                    f"EG counter phase mismatch at call {row['call_index']}: "
                    f"state={row['eg_counter_state']} expected={global_counter & 0xff}"
                )
                writer.writerow((
                    row["call_index"], slot, global_counter, 1,
                    row["eg_level"][slot], row["eg_state"][slot],
                ))
                count += 1
    assert count > 0, "no channel-0 EG samples found during KEY-ON"
    print(f"extracted {count} active EG samples (carrier slot 2, modulator slot 17)")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
