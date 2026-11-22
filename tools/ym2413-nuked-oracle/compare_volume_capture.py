#!/usr/bin/env python3
"""Characterize Nuked-OPLL MO output against the historical volume0 ADC capture.

The candidate mapping under test is measured_bin = 255 - output_pins_m.
This is an offline level/time-series comparison, not a hardware or game-audio
acceptance test.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import TextIO

TRACE_SCHEMA = "ym2413-nuked-opll-output-trace-v1"
SLOT_COUNT = 18
OUTPUT_MIN = -256
OUTPUT_MAX = 255


def load_capture_bins(path: Path) -> list[int]:
    """Load the calibrated ADC-bin indices from one upstream .out capture."""
    try:
        tokens = path.read_text(encoding="ascii").split()
    except OSError as error:
        raise ValueError(f"cannot read capture {path}: {error}") from error
    if not tokens:
        raise ValueError(f"capture {path} is empty")

    bins: list[int] = []
    for index, token in enumerate(tokens):
        try:
            value = int(token, 10)
        except ValueError as error:
            raise ValueError(
                f"invalid capture bin at sample {index}: {token!r}"
            ) from error
        if not 0 <= value <= 511:
            raise ValueError(f"capture bin {index} is outside 0..511: {value}")
        bins.append(value)
    return bins


def candidate_bin_from_output(output_level: int) -> int:
    """Apply the candidate inverse mapping from signed output to ADC-bin code."""
    if not OUTPUT_MIN <= output_level <= OUTPUT_MAX:
        raise ValueError(
            f"output level is outside {OUTPUT_MIN}..{OUTPUT_MAX}: {output_level}"
        )
    return 255 - output_level


def select_sample_cycle(
    nonzero_counts: list[int], maximum_magnitudes: list[int]
) -> int:
    """Select the unique MO slot with the strongest non-muted output activity."""
    if len(nonzero_counts) != SLOT_COUNT or len(maximum_magnitudes) != SLOT_COUNT:
        raise ValueError(f"sample-slot metrics must contain {SLOT_COUNT} entries")

    scores = [
        (nonzero_counts[index], maximum_magnitudes[index])
        for index in range(SLOT_COUNT)
    ]
    best = max(scores)
    winners = [index for index, score in enumerate(scores) if score == best]
    if best[0] == 0 or best[1] <= 1 or len(winners) != 1:
        raise ValueError(f"could not identify one active MO slot; scores={scores}")
    return winners[0]


def score_alignment(
    measured: list[int],
    model: list[int],
    *,
    phase_search: int,
    prefix_samples: int,
) -> dict[str, object]:
    """Find the best cyclic phase offset and score the complete capture."""
    if not measured or len(model) < len(measured):
        raise ValueError("model trace does not contain enough samples")
    if phase_search < 0 or prefix_samples <= 0:
        raise ValueError("phase_search must be non-negative and prefix_samples positive")

    maximum_offset = min(phase_search, len(model) - len(measured))
    prefix_length = min(prefix_samples, len(measured))
    best_key: tuple[int, int, int] | None = None
    best_offset = 0

    for offset in range(maximum_offset + 1):
        errors = [
            abs(measured[index] - model[offset + index])
            for index in range(prefix_length)
        ]
        key = (sum(error == 0 for error in errors), -sum(errors), -offset)
        if best_key is None or key > best_key:
            best_key = key
            best_offset = offset

    errors = [
        abs(measured[index] - model[best_offset + index])
        for index in range(len(measured))
    ]
    ordered_errors = sorted(errors)
    window_size = 2048
    windows = []
    for start in range(0, len(errors), window_size):
        window = errors[start : start + window_size]
        windows.append(
            {
                "start_sample": start,
                "sample_count": len(window),
                "exact_match_rate": sum(error == 0 for error in window)
                / len(window),
            }
        )

    return {
        "best_phase_offset_samples": best_offset,
        "prefix_samples_scored": prefix_length,
        "prefix_exact_matches": best_key[0] if best_key else 0,
        "prefix_exact_match_rate": (
            (best_key[0] / prefix_length) if prefix_length else 0.0
        ),
        "full_exact_matches": sum(error == 0 for error in errors),
        "full_exact_match_rate": sum(error == 0 for error in errors) / len(errors),
        "mean_absolute_error": sum(errors) / len(errors),
        "p95_absolute_error": ordered_errors[int((len(ordered_errors) - 1) * 0.95)],
        "maximum_absolute_error": max(errors),
        "windowed_exact_match_rates": windows,
    }


def read_output_trace(
    stream: TextIO,
    *,
    initial_keyon_call: int,
    scan_calls: int,
    capture_start_call: int,
) -> tuple[int, list[int], list[int], dict[str, object]]:
    """Read compact JSONL and extract one MO sample per 18-slot rotation."""
    header_line = stream.readline()
    try:
        header = json.loads(header_line)
    except json.JSONDecodeError as error:
        raise ValueError("trace does not begin with a valid JSON schema line") from error
    if header.get("schema") != TRACE_SCHEMA:
        raise ValueError(
            f"unexpected trace schema: {header.get('schema')!r}; expected {TRACE_SCHEMA!r}"
        )

    nonzero_counts = [0] * SLOT_COUNT
    maximum_magnitudes = [0] * SLOT_COUNT
    model_by_cycle: list[list[int]] = [[] for _ in range(SLOT_COUNT)]
    rhythm_by_cycle: list[list[int]] = [[] for _ in range(SLOT_COUNT)]
    scan_end = initial_keyon_call + scan_calls

    for line_number, line in enumerate(stream, start=2):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
            call_index = int(row["call_index"])
            cycle = int(row["cycle_before"])
            output_m = int(row["output_pins_m"])
            output_r = int(row["output_pins_r"])
        except (json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
            raise ValueError(f"invalid trace record on line {line_number}: {error}") from error

        if not 0 <= cycle < SLOT_COUNT:
            raise ValueError(
                f"trace cycle on line {line_number} is outside 0..17: {cycle}"
            )
        if not OUTPUT_MIN <= output_m <= OUTPUT_MAX:
            raise ValueError(
                f"MO output on line {line_number} is outside "
                f"{OUTPUT_MIN}..{OUTPUT_MAX}: {output_m}"
            )

        if initial_keyon_call <= call_index < scan_end:
            magnitude = abs(output_m)
            if magnitude > 1:
                nonzero_counts[cycle] += 1
            maximum_magnitudes[cycle] = max(maximum_magnitudes[cycle], magnitude)

        if call_index >= capture_start_call:
            model_by_cycle[cycle].append(output_m)
            rhythm_by_cycle[cycle].append(output_r)

    sample_cycle = select_sample_cycle(nonzero_counts, maximum_magnitudes)
    model_output = model_by_cycle[sample_cycle]
    rhythm_output = rhythm_by_cycle[sample_cycle]
    model_bins = [candidate_bin_from_output(value) for value in model_output]
    rhythm_nonzero_count = sum(abs(value) > 1 for value in rhythm_output)
    rhythm_maximum_magnitude = max(
        (abs(value) for value in rhythm_output), default=0
    )

    diagnostics: dict[str, object] = {
        "sample_cycle": sample_cycle,
        "slot_nonzero_counts": nonzero_counts,
        "slot_maximum_magnitudes": maximum_magnitudes,
        "model_samples_at_sample_cycle": len(model_bins),
        "rhythm_pin_nonzero_count_at_sample_cycle": rhythm_nonzero_count,
        "rhythm_pin_maximum_magnitude_at_sample_cycle": rhythm_maximum_magnitude,
    }
    return sample_cycle, model_bins, rhythm_output, diagnostics


def compare_capture(
    trace_path: Path,
    capture_path: Path,
    *,
    capture_start_call: int,
    initial_keyon_call: int,
    scan_calls: int,
    phase_search: int,
    prefix_samples: int,
) -> dict[str, object]:
    """Compare Nuked's sampled MO output with the measured volume0 bin sequence."""
    measured = load_capture_bins(capture_path)
    try:
        with trace_path.open("r", encoding="utf-8") as stream:
            sample_cycle, model_bins, rhythm_output, diagnostics = read_output_trace(
                stream,
                initial_keyon_call=initial_keyon_call,
                scan_calls=scan_calls,
                capture_start_call=capture_start_call,
            )
    except OSError as error:
        raise ValueError(f"cannot read trace {trace_path}: {error}") from error

    direct = score_alignment(
        measured,
        model_bins,
        phase_search=phase_search,
        prefix_samples=prefix_samples,
    )
    inverted_model = [511 - value for value in model_bins]
    inverted = score_alignment(
        measured,
        inverted_model,
        phase_search=phase_search,
        prefix_samples=prefix_samples,
    )
    return {
        "schema": "ym2413-volume-capture-nuked-differential-v1",
        "capture_file": capture_path.name,
        "capture_samples": len(measured),
        "capture_bin_range": [min(measured), max(measured)],
        "capture_start_call": capture_start_call,
        "initial_keyon_call": initial_keyon_call,
        "sample_interval_opll_calls": SLOT_COUNT,
        "candidate_mapping": "measured_bin = 255 - output_pins_m",
        "direct_candidate": direct,
        "inverted_polarity_control": inverted,
        "trace_diagnostics": diagnostics,
        "interpretation": (
            "characterization only; phase alignment and level agreement do not "
            "certify DAC electrical transfer, MO/RO loading, or game playback"
        ),
    }


def main() -> int:
    """Run the volume0 Nuked-versus-measurement characterization."""
    parser = argparse.ArgumentParser(
        description="Compare Nuked-OPLL MO output with the historical volume0 capture."
    )
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--capture", type=Path, required=True)
    parser.add_argument("--capture-start-call", type=int, default=601)
    parser.add_argument("--initial-keyon-call", type=int, default=296)
    parser.add_argument("--scan-calls", type=int, default=18 * 64)
    parser.add_argument("--phase-search", type=int, default=2048)
    parser.add_argument("--prefix-samples", type=int, default=512)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    try:
        report = compare_capture(
            args.trace,
            args.capture,
            capture_start_call=args.capture_start_call,
            initial_keyon_call=args.initial_keyon_call,
            scan_calls=args.scan_calls,
            phase_search=args.phase_search,
            prefix_samples=args.prefix_samples,
        )
    except ValueError as error:
        parser.error(str(error))

    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        try:
            args.output.write_text(rendered, encoding="utf-8")
        except OSError as error:
            parser.error(f"cannot write report {args.output}: {error}")
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
