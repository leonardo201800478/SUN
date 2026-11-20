#!/usr/bin/env python3
"""Compare pinned external PSG traces with the Sega Emulator core trace."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any


def read_jsonl(path: Path, expected_schema: str) -> tuple[dict[str, Any], list[dict[str, Any]], bytes]:
    raw = path.read_bytes()
    lines = raw.decode("utf-8").splitlines()
    if not lines:
        raise SystemExit(f"{path}: empty trace")
    header = json.loads(lines[0])
    if header.get("schema") != expected_schema:
        raise SystemExit(
            f"{path}: expected schema {expected_schema!r}, got {header.get('schema')!r}"
        )
    rows = [json.loads(line) for line in lines[1:] if line.strip()]
    ticks = [row.get("tick") for row in rows]
    if ticks != list(range(len(rows))):
        raise SystemExit(f"{path}: tick sequence is not contiguous from zero")
    return header, rows, raw


def first_index(values: list[bool]) -> int | None:
    return next((index for index, value in enumerate(values) if value), None)


def main() -> int:
    if len(sys.argv) not in (3, 4):
        raise SystemExit(
            "usage: compare.py crabemu-reference.jsonl core.jsonl [gearsystem-reference.jsonl]"
        )

    ref_header, ref_rows, ref_raw = read_jsonl(
        Path(sys.argv[1]), "smsplus-crabemu-psg-trace-v1"
    )
    core_header, core_rows, core_raw = read_jsonl(
        Path(sys.argv[2]), "sega-sn76489-core-state-trace-v1"
    )

    if len(ref_rows) != len(core_rows):
        raise SystemExit(
            f"CrabEmu/core trace lengths differ: reference={len(ref_rows)} core={len(core_rows)}"
        )
    if ref_header.get("source_commit") != "3844b46caa926b6494987b97da63092818c4ddef":
        raise SystemExit("CrabEmu reference commit pin mismatch")
    if ref_header.get("noise_bits") != 16 or ref_header.get("noise_tapped") != 9:
        raise SystemExit("CrabEmu SMS noise configuration mismatch")

    state_mismatch = []
    sample_vs_core_lfsr = []
    sample_vs_core_latch = []
    ref_shift_ticks = []
    core_shift_ticks = []
    sample_bits = []

    for tick, (reference, core) in enumerate(zip(ref_rows, core_rows, strict=True)):
        ref_before = int(reference["noise_shift_before"])
        ref_after = int(reference["noise_shift_after"])
        core_before = int(core["lfsr_before"])
        core_after = int(core["lfsr_after"])
        sample_bit = int(reference["sample_l"] != 0)

        # In this trace sequence the three tone channels remain attenuated.
        # CrabEmu's raw noise sample must equal its pre-update LFSR bit 0.
        if sample_bit != int(reference["noise_bit_before"]):
            raise SystemExit(
                f"CrabEmu sample does not match its pre-update LFSR bit at tick {tick}"
            )

        if ref_before != ref_after:
            ref_shift_ticks.append(tick)
        if core_before != core_after:
            core_shift_ticks.append(tick)
        if ref_after != core_after:
            state_mismatch.append(tick)
        if sample_bit != int(core["lfsr_bit_before"]):
            sample_vs_core_lfsr.append(tick)
        if sample_bit != int(core["noise_output_after"]):
            sample_vs_core_latch.append(tick)
        sample_bits.append(sample_bit)

    summary: dict[str, Any] = {
        "schema": "sega-psg-differential-summary-v1",
        "comparison_policy": "same tick index; no waveform alignment or correlation-based shifting",
        "reference": {
            "name": "SMS Plus GX CrabEmu SN76489",
            "commit": ref_header["source_commit"],
            "license": ref_header["license"],
            "trace_sha256": hashlib.sha256(ref_raw).hexdigest(),
        },
        "core": {
            "profile": core_header["profile"],
            "trace_sha256": hashlib.sha256(core_raw).hexdigest(),
        },
        "ticks_compared": len(ref_rows),
        "reference_lfsr_shift_count": len(ref_shift_ticks),
        "core_lfsr_shift_count": len(core_shift_ticks),
        "reference_first_lfsr_shift_tick": first_index(
            [tick in ref_shift_ticks for tick in range(len(ref_rows))]
        ),
        "core_first_lfsr_shift_tick": first_index(
            [tick in core_shift_ticks for tick in range(len(core_rows))]
        ),
        "reference_lfsr_shift_ticks_first_20": ref_shift_ticks[:20],
        "core_lfsr_shift_ticks_first_20": core_shift_ticks[:20],
        "lfsr_state_mismatch_count": len(state_mismatch),
        "lfsr_state_first_mismatch_tick": first_index(
            [tick in state_mismatch for tick in range(len(ref_rows))]
        ),
        "reference_sample_vs_core_lfsr_bit_mismatch_count": len(sample_vs_core_lfsr),
        "reference_sample_vs_core_lfsr_bit_first_mismatch_tick": first_index(
            [tick in sample_vs_core_lfsr for tick in range(len(ref_rows))]
        ),
        "reference_sample_vs_core_noise_output_latch_mismatch_count": len(
            sample_vs_core_latch
        ),
        "reference_sample_vs_core_noise_output_latch_first_mismatch_tick": first_index(
            [tick in sample_vs_core_latch for tick in range(len(ref_rows))]
        ),
        "reference_sample_bit_ones": sum(sample_bits),
        "note": (
            "Differences are diagnostic, not an automatic hardware pass/fail. "
            "The CrabEmu sample is captured before its LFSR update; the Rust "
            "core exposes both current LFSR state and its latched noise_output."
        ),
    }

    if len(sys.argv) == 4:
        gear_header, gear_rows, gear_raw = read_jsonl(
            Path(sys.argv[3]), "gearsystem-sms-psg-state-trace-v1"
        )
        if len(gear_rows) != len(core_rows):
            raise SystemExit(
                f"Gearsystem/core trace lengths differ: reference={len(gear_rows)} core={len(core_rows)}"
            )
        if gear_header.get("source_commit") != "1be9a12b82cae84ddd6f9b7006d546abe2d58bd4":
            raise SystemExit("Gearsystem reference commit pin mismatch")
        if gear_header.get("row_duration_tstates") != 16:
            raise SystemExit("Gearsystem trace must advance exactly 16 Z80 T-states per row")

        gear_shift_ticks = []
        gear_bit_vs_core_bit = []
        gear_bit_vs_core_latch = []
        for tick, (gear, core) in enumerate(zip(gear_rows, core_rows, strict=True)):
            if int(gear["noise_shifter_before"]) != int(gear["noise_shifter_after"]):
                gear_shift_ticks.append(tick)
            gear_bit = int(gear["noise_bit_after"])
            if gear_bit != int(core["lfsr_bit_after"]):
                gear_bit_vs_core_bit.append(tick)
            if gear_bit != int(core["noise_output_after"]):
                gear_bit_vs_core_latch.append(tick)

        summary["gearsystem"] = {
            "name": "Gearsystem Sms_Apu",
            "commit": gear_header["source_commit"],
            "license": gear_header["license"],
            "trace_sha256": hashlib.sha256(gear_raw).hexdigest(),
            "lfsr_shift_count": len(gear_shift_ticks),
            "first_lfsr_shift_tick": first_index(
                [tick in gear_shift_ticks for tick in range(len(gear_rows))]
            ),
            "lfsr_shift_ticks_first_20": gear_shift_ticks[:20],
            "galois_state_raw_values_compared": False,
            "observable_bit0_vs_core_lfsr_bit_mismatch_count": len(gear_bit_vs_core_bit),
            "observable_bit0_vs_core_lfsr_bit_first_mismatch_tick": first_index(
                [tick in gear_bit_vs_core_bit for tick in range(len(gear_rows))]
            ),
            "observable_bit0_vs_core_noise_output_latch_mismatch_count": len(
                gear_bit_vs_core_latch
            ),
            "observable_bit0_vs_core_noise_output_latch_first_mismatch_tick": first_index(
                [tick in gear_bit_vs_core_latch for tick in range(len(gear_rows))]
            ),
            "note": (
                "Gearsystem uses a Galois-form LFSR. Raw state integers are not "
                "compared to the core's Fibonacci-form register; only observable "
                "bit-0 state and timestamped shift events are compared."
            ),
        }

    print(json.dumps(summary, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
