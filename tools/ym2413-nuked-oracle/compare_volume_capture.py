#!/usr/bin/env python3
"""Characterize Nuked-OPLL MO output against the historical volume0 ADC capture.

The candidate mapping is measured_bin = 255 - ch_out_before. Nuked's MO
output-pin encoding increments non-negative channel outputs by one, preserving
the two zero codes as +1 and -1; the comparator checks that pin encoding too.
This is an offline characterization, not a game-audio acceptance test.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from statistics import median
from pathlib import Path
from typing import TextIO

TRACE_SCHEMA = "ym2413-nuked-opll-output-trace-v2"
SLOT_COUNT = 18
CHANNEL_OUTPUT_MIN = -256
CHANNEL_OUTPUT_MAX = 255
OUTPUT_PIN_MIN = -256
OUTPUT_PIN_MAX = 256


def load_capture_bins(path: Path) -> list[int]:
    """Load calibrated ADC-bin indices from one upstream .out capture."""
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


def candidate_bin_from_channel_output(channel_output: int) -> int:
    """Map the pre-pin signed channel output to its candidate calibrated bin."""
    if not CHANNEL_OUTPUT_MIN <= channel_output <= CHANNEL_OUTPUT_MAX:
        raise ValueError(
            "channel output is outside "
            f"{CHANNEL_OUTPUT_MIN}..{CHANNEL_OUTPUT_MAX}: {channel_output}"
        )
    return 255 - channel_output


def candidate_bin_from_output_pin(output_pin: int) -> int:
    """Invert Nuked's MO pin encoding, including its two distinct zero codes."""
    if not OUTPUT_PIN_MIN <= output_pin <= OUTPUT_PIN_MAX or output_pin == 0:
        raise ValueError(
            f"MO output pin must be in {OUTPUT_PIN_MIN}..{OUTPUT_PIN_MAX}, "
            f"excluding 0: {output_pin}"
        )
    if output_pin > 0:
        return 256 - output_pin
    return 255 - output_pin


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
    """Find the best sample-phase offset and score the complete capture."""
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



def percentile(values: list[float | int], fraction: float) -> float | int:
    """Return a deterministic nearest-rank percentile for a non-empty list."""
    ordered = sorted(values)
    return ordered[int((len(ordered) - 1) * fraction)]


def residual_profile(
    measured: list[int], model: list[int], offset: int
) -> dict[str, object]:
    """Summarize signed bin residuals and the monotonicity of the empirical map."""
    errors = [
        measured[index] - model[offset + index]
        for index in range(len(measured))
    ]
    grouped: dict[int, list[int]] = defaultdict(list)
    for index, error in enumerate(errors):
        grouped[model[offset + index]].append(error)

    per_code = [
        (code, median(code_errors), len(code_errors))
        for code, code_errors in sorted(grouped.items())
    ]
    mapped_bins = [code + residual for code, residual, _ in per_code]
    monotonicity_violations = sum(
        mapped_bins[index] < mapped_bins[index - 1]
        for index in range(1, len(mapped_bins))
    )
    median_residuals = [abs(residual) for _, residual, _ in per_code]
    histogram = Counter(errors)

    return {
        "mean_signed_error": sum(errors) / len(errors),
        "median_signed_error": median(errors),
        "minimum_signed_error": min(errors),
        "maximum_signed_error": max(errors),
        "signed_error_histogram": {
            str(error): count for error, count in sorted(histogram.items())
        },
        "observed_model_codes": len(per_code),
        "empirical_mapping_monotonicity_violations": monotonicity_violations,
        "per_code_median_absolute_residual_p50": percentile(median_residuals, 0.50),
        "per_code_median_absolute_residual_p90": percentile(median_residuals, 0.90),
        "per_code_median_absolute_residual_p99": percentile(median_residuals, 0.99),
        "per_code_median_absolute_residual_max": max(median_residuals),
    }


def local_phase_alignment(
    measured: list[int],
    model: list[int],
    global_offset: int,
    *,
    window_size: int = 2048,
    search_radius: int = 32,
) -> list[dict[str, object]]:
    """Search a small local phase neighborhood to expose drift across the capture."""
    windows: list[dict[str, object]] = []
    for start in range(0, len(measured), window_size):
        length = min(window_size, len(measured) - start)
        minimum_offset = max(0, global_offset - search_radius)
        maximum_offset = min(
            len(model) - start - length, global_offset + search_radius
        )
        best_key: tuple[int, int, int] | None = None
        best_offset = minimum_offset

        for offset in range(minimum_offset, maximum_offset + 1):
            errors = [
                abs(measured[start + index] - model[offset + start + index])
                for index in range(length)
            ]
            key = (
                sum(error == 0 for error in errors),
                -sum(errors),
                -abs(offset - global_offset),
            )
            if best_key is None or key > best_key:
                best_key = key
                best_offset = offset

        errors = [
            abs(measured[start + index] - model[best_offset + start + index])
            for index in range(length)
        ]
        windows.append(
            {
                "start_sample": start,
                "sample_count": length,
                "best_phase_offset_samples": best_offset,
                "phase_delta_from_global": best_offset - global_offset,
                "exact_match_rate": sum(error == 0 for error in errors) / length,
                "mean_absolute_error": sum(errors) / length,
            }
        )
    return windows


def read_output_trace(
    stream: TextIO,
    *,
    initial_keyon_call: int,
    scan_calls: int,
    capture_start_call: int,
) -> tuple[list[int], dict[str, object]]:
    """Read compact JSONL and extract pre-pin output once per 18-slot rotation."""
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
    channel_output_by_cycle: list[list[int]] = [[] for _ in range(SLOT_COUNT)]
    pin_output_by_cycle: list[list[int]] = [[] for _ in range(SLOT_COUNT)]
    rhythm_by_cycle: list[list[int]] = [[] for _ in range(SLOT_COUNT)]
    scan_end = initial_keyon_call + scan_calls

    for line_number, line in enumerate(stream, start=2):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
            call_index = int(row["call_index"])
            cycle = int(row["cycle_before"])
            channel_output = int(row["ch_out_before"])
            output_m = int(row["output_pins_m"])
            output_r = int(row["output_pins_r"])
        except (json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
            raise ValueError(
                f"invalid trace record on line {line_number}: {error}"
            ) from error

        if not 0 <= cycle < SLOT_COUNT:
            raise ValueError(
                f"trace cycle on line {line_number} is outside 0..17: {cycle}"
            )
        if not CHANNEL_OUTPUT_MIN <= channel_output <= CHANNEL_OUTPUT_MAX:
            raise ValueError(
                f"channel output on line {line_number} is outside "
                f"{CHANNEL_OUTPUT_MIN}..{CHANNEL_OUTPUT_MAX}: {channel_output}"
            )
        if not OUTPUT_PIN_MIN <= output_m <= OUTPUT_PIN_MAX:
            raise ValueError(
                f"MO output pin on line {line_number} is outside "
                f"{OUTPUT_PIN_MIN}..{OUTPUT_PIN_MAX}: {output_m}"
            )

        if initial_keyon_call <= call_index < scan_end:
            magnitude = abs(output_m)
            if magnitude > 1:
                nonzero_counts[cycle] += 1
            maximum_magnitudes[cycle] = max(maximum_magnitudes[cycle], magnitude)

        if call_index >= capture_start_call:
            channel_output_by_cycle[cycle].append(channel_output)
            pin_output_by_cycle[cycle].append(output_m)
            rhythm_by_cycle[cycle].append(output_r)

    sample_cycle = select_sample_cycle(nonzero_counts, maximum_magnitudes)
    channel_outputs = channel_output_by_cycle[sample_cycle]
    pin_outputs = pin_output_by_cycle[sample_cycle]
    rhythm_outputs = rhythm_by_cycle[sample_cycle]
    model_bins = [
        candidate_bin_from_channel_output(value) for value in channel_outputs
    ]
    pin_bins = [candidate_bin_from_output_pin(value) for value in pin_outputs]
    pin_encoding_mismatches = sum(
        model_bin != pin_bin for model_bin, pin_bin in zip(model_bins, pin_bins)
    )
    rhythm_nonzero_count = sum(abs(value) > 1 for value in rhythm_outputs)
    rhythm_maximum_magnitude = max(
        (abs(value) for value in rhythm_outputs), default=0
    )

    diagnostics: dict[str, object] = {
        "sample_cycle": sample_cycle,
        "slot_nonzero_counts": nonzero_counts,
        "slot_maximum_magnitudes": maximum_magnitudes,
        "model_samples_at_sample_cycle": len(model_bins),
        "output_pin_encoding_mismatches": pin_encoding_mismatches,
        "rhythm_pin_nonzero_count_at_sample_cycle": rhythm_nonzero_count,
        "rhythm_pin_maximum_magnitude_at_sample_cycle": rhythm_maximum_magnitude,
        "channel_output_range": [min(channel_outputs), max(channel_outputs)],
        "mo_output_pin_range": [min(pin_outputs), max(pin_outputs)],
    }
    return model_bins, diagnostics


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
    """Compare Nuked's sampled MO output with measured volume0 bin indices."""
    measured = load_capture_bins(capture_path)
    try:
        with trace_path.open("r", encoding="utf-8") as stream:
            model_bins, diagnostics = read_output_trace(
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
    direct["residual_profile"] = residual_profile(
        measured,
        model_bins,
        int(direct["best_phase_offset_samples"]),
    )
    direct["local_phase_alignment"] = local_phase_alignment(
        measured,
        model_bins,
        int(direct["best_phase_offset_samples"]),
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
        "sample_interval_ym_clocks": 72,
        "candidate_mapping": "measured_bin = 255 - ch_out_before",
        "mo_output_pin_inverse": (
            "256 - output_pins_m when positive; "
            "255 - output_pins_m when negative"
        ),
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
