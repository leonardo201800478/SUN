#!/usr/bin/env python3
"""Characterize the Rust/Nuked YM2413 operator-to-measured-output candidate.

This is an offline diagnostic. It does not identify the physical DAC input bus.
The candidate code is 255 - channel_output, while the signed measured-domain
interpretation preserves the one's-complement negative-zero token separately.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import TextIO

TRACE_SCHEMA = "ym2413-nuked-opll-trace-v1"
CHANNEL_OUTPUT_MIN = -256
CHANNEL_OUTPUT_MAX = 255
PHASE_COUNT = 1024
VOLUME_COUNT = 16


def candidate_output_code(channel_output: int) -> int:
    """Return the candidate 9-bit measured-output bin for a channel latch."""
    if not CHANNEL_OUTPUT_MIN <= channel_output <= CHANNEL_OUTPUT_MAX:
        raise ValueError(
            f"channel_output outside {CHANNEL_OUTPUT_MIN}..={CHANNEL_OUTPUT_MAX}: "
            f"{channel_output}"
        )
    return 255 - channel_output


def signed_measurement(channel_output: int) -> tuple[int, bool]:
    """Decode one's-complement channel output into signed amplitude and -0 flag."""
    candidate_output_code(channel_output)
    if channel_output >= 0:
        return channel_output, False
    magnitude = ~channel_output
    if magnitude == 0:
        return 0, True
    return -magnitude, False


def load_predicted_sine(path: Path) -> list[list[int]]:
    """Load the published 1024-row, 16-volume signed prediction artifact."""
    rows: list[list[int]] = []
    try:
        lines = path.read_text(encoding="ascii").splitlines()
    except OSError as error:
        raise ValueError(f"cannot read predicted-sine artifact {path}: {error}") from error

    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            values = [int(token, 10) for token in line.split()]
        except ValueError as error:
            raise ValueError(
                f"predicted-sine line {line_number} contains a non-integer"
            ) from error
        if len(values) != VOLUME_COUNT:
            raise ValueError(
                f"predicted-sine line {line_number}: expected {VOLUME_COUNT} "
                f"columns, got {len(values)}"
            )
        if any(value < -256 or value > 255 for value in values):
            raise ValueError(f"predicted-sine line {line_number}: value outside -256..255")
        rows.append(values)

    if len(rows) != PHASE_COUNT:
        raise ValueError(
            f"predicted-sine artifact must contain {PHASE_COUNT} rows, got {len(rows)}"
        )
    return rows


def iter_trace(stream: TextIO) -> tuple[dict[str, object], list[dict[str, object]]]:
    """Read and validate the full Nuked operator JSONL trace."""
    try:
        header = json.loads(stream.readline())
    except json.JSONDecodeError as error:
        raise ValueError("trace must start with a JSON schema header") from error
    if header.get("schema") != TRACE_SCHEMA:
        raise ValueError(f"unexpected trace schema: {header.get('schema')!r}")
    if header.get("time_unit") != "opll_clock_call":
        raise ValueError("trace time_unit must be opll_clock_call")

    rows: list[dict[str, object]] = []
    required = ("call_index", "cycle_before", "op_phase_index", "op_output", "ch_out")
    for line_number, line in enumerate(stream, 2):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"trace line {line_number} is invalid JSON") from error
        missing = [field for field in required if field not in row]
        if missing:
            raise ValueError(f"trace line {line_number} missing fields: {missing}")
        for field in required:
            if not isinstance(row[field], int) or isinstance(row[field], bool):
                raise ValueError(f"trace line {line_number}: {field} must be an integer")
        if not 0 <= int(row["op_phase_index"]) < PHASE_COUNT:
            raise ValueError(
                f"trace line {line_number}: op_phase_index outside 0..1023"
            )
        channel = int(row["ch_out"])
        candidate_output_code(channel)
        rows.append(row)

    if not rows:
        raise ValueError("trace contains no operator samples")
    return header, rows


def characterize(
    rows: list[dict[str, object]],
    predicted: list[list[int]] | None,
    volume_column: int | None,
) -> dict[str, object]:
    """Build per-event candidate records and deterministic aggregate metrics."""
    events: list[dict[str, object]] = []
    exact_prediction_matches = 0
    prediction_count = 0
    for row in rows:
        operator_output = int(row["op_output"])
        channel_output = int(row["ch_out"])
        phase = int(row["op_phase_index"])
        code = candidate_output_code(channel_output)
        amplitude, negative_zero = signed_measurement(channel_output)
        event: dict[str, object] = {
            "call_index": int(row["call_index"]),
            "serial_cycle": int(row["cycle_before"]),
            "phase_index": phase,
            "operator_output": operator_output,
            "channel_output": channel_output,
            "candidate_output_code": code,
            "signed_measurement_value": amplitude,
            "negative_zero": negative_zero,
            "operator_shift_matches_channel": (operator_output >> 3) == channel_output,
            "nuked_ch_out": channel_output,
        }
        if predicted is not None and volume_column is not None:
            expected = predicted[phase][volume_column]
            event["published_predicted_sine_value"] = expected
            event["prediction_delta"] = amplitude - expected
            exact_prediction_matches += amplitude == expected
            prediction_count += 1
        events.append(event)

    shift_mismatches = sum(not bool(event["operator_shift_matches_channel"]) for event in events)
    report: dict[str, object] = {
        "status": "PASS" if shift_mismatches == 0 else "DIVERGENCE",
        "schema": "ym2413-output-code-candidate-report-v1",
        "events": len(events),
        "candidate_code_range": [min(int(e["candidate_output_code"]) for e in events),
                                 max(int(e["candidate_output_code"]) for e in events)],
        "distinct_candidate_codes": len({int(e["candidate_output_code"]) for e in events}),
        "negative_zero_events": sum(bool(e["negative_zero"]) for e in events),
        "operator_shift_mismatches": shift_mismatches,
        "published_prediction": None if predicted is None else {
            "volume_column": volume_column,
            "compared_events": prediction_count,
            "exact_matches": exact_prediction_matches,
            "exact_match_rate": exact_prediction_matches / prediction_count,
            "provenance": "published predicted-sine.txt artifact; not regenerated from analyze/exp.cc",
        },
        "evidence_boundary": (
            "candidate measured-output mapping only; not a confirmed physical DAC input word"
        ),
        "records": events,
    }
    return report


def main() -> int:
    """Validate a Nuked trace and emit a JSON candidate-mapping report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--predicted-sine", type=Path)
    parser.add_argument("--volume-column", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    if args.volume_column is not None and not 0 <= args.volume_column < VOLUME_COUNT:
        raise ValueError(f"volume column must be in 0..{VOLUME_COUNT - 1}")
    if args.predicted_sine and args.volume_column is None:
        raise ValueError("--volume-column is required with --predicted-sine")
    if args.volume_column is not None and args.predicted_sine is None:
        raise ValueError("--predicted-sine is required with --volume-column")

    with args.nuked_trace.open(encoding="utf-8") as stream:
        _, rows = iter_trace(stream)
    predicted = load_predicted_sine(args.predicted_sine) if args.predicted_sine else None
    report = characterize(rows, predicted, args.volume_column)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"YM2413 output-code candidate comparison failed: {error}", file=sys.stderr)
        raise SystemExit(2)
