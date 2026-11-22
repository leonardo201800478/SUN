#!/usr/bin/env python3
"""Audit paired YM2413 ADC volume captures and reconstruct bin-boundary constraints.

The upstream .out files contain calibrated ADC-bin indices. They are not raw
YM2413 DAC words. This tool uses their sample-index relationship with the
matching .raw ADC streams to constrain the original ADC-bin thresholds.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ADC_LEVELS = 1 << 16
BIN_COUNT = 512
DEFAULT_MAX_ALIGNMENT_OFFSET = 8
UPSTREAM_REPOSITORY = "andete/ym2413"
UPSTREAM_REVISION = "4893d1c09faa0450b5d3355f19b7682d72822501"

ALIGNMENT_DEFINITION = (
    "paired raw[i] with out[i + alignment_offset]; offsets inferred by minimum "
    "repeated-ADC mapping conflicts and monotonicity violations within [-8,+8]"
)
ALIGNMENT_NOTE = (
    "Offsets are data-sequence alignment corrections inferred by testing offsets "
    "-8..+8 and selecting the zero-conflict, zero-monotonicity-violation alignment. "
    "They are not hardware-clock or YM2413 subclock offsets."
)


def load_integer_stream(path: Path, maximum: int, label: str) -> list[int]:
    """Load and range-check a whitespace-separated integer capture."""
    try:
        tokens = path.read_text(encoding="ascii").split()
    except OSError as error:
        raise ValueError(f"cannot read {label} file {path}: {error}") from error

    if not tokens:
        raise ValueError(f"{label} file {path} contains no samples")

    values: list[int] = []
    for index, token in enumerate(tokens):
        try:
            value = int(token, 10)
        except ValueError as error:
            raise ValueError(
                f"invalid {label} integer at sample {index}: {token!r}"
            ) from error
        if not 0 <= value <= maximum:
            raise ValueError(
                f"{label} sample {index} is outside 0..{maximum}: {value}"
            )
        values.append(value)
    return values


def evaluate_alignment(
    raw_samples: list[int], output_bins: list[int], offset: int
) -> tuple[int, int, int]:
    """Count mapping conflicts, monotonicity violations and paired samples."""
    mapping = [-1] * ADC_LEVELS
    conflicts = 0
    paired_samples = 0

    for raw_index, adc_code in enumerate(raw_samples):
        output_index = raw_index + offset
        if not 0 <= output_index < len(output_bins):
            continue

        bin_index = output_bins[output_index]
        previous = mapping[adc_code]
        if previous == -1:
            mapping[adc_code] = bin_index
        elif previous != bin_index:
            conflicts += 1
        paired_samples += 1

    violations = 0
    previous_bin = -1
    for bin_index in mapping:
        if bin_index < 0:
            continue
        if bin_index < previous_bin:
            violations += 1
        elif bin_index > previous_bin:
            previous_bin = bin_index

    return conflicts, violations, paired_samples


def infer_alignment_offset(
    raw_samples: list[int],
    output_bins: list[int],
    max_offset: int = DEFAULT_MAX_ALIGNMENT_OFFSET,
) -> tuple[int, int, int, int]:
    """Find the unique clean raw[i] to out[i+offset] sequence alignment."""
    candidates: list[tuple[int, int, int, int, int]] = []
    for offset in range(-max_offset, max_offset + 1):
        conflicts, violations, paired_samples = evaluate_alignment(
            raw_samples, output_bins, offset
        )
        candidates.append(
            (conflicts, violations, -paired_samples, abs(offset), offset)
        )

    candidates.sort()
    best = candidates[0]
    if best[0] != 0 or best[1] != 0:
        raise ValueError(
            "no clean raw/output alignment found within "
            f"[-{max_offset},+{max_offset}]; best candidate has "
            f"{best[0]} mapping conflicts and {best[1]} monotonicity violations"
        )

    best_rank = best[:4]
    tied = [candidate for candidate in candidates if candidate[:4] == best_rank]
    if len(tied) != 1:
        raise ValueError(
            "ambiguous raw/output alignment: multiple offsets have the same "
            "conflict, monotonicity, pair-count and absolute-offset score"
        )

    return best[4], -best[2], best[0], best[1]


def percentile(values: list[int], fraction: float) -> int:
    """Return the deterministic floor-index percentile used by this audit."""
    if not values:
        raise ValueError("cannot calculate a percentile from an empty sequence")
    ordered = sorted(values)
    return ordered[int((len(ordered) - 1) * fraction)]


def audit_volume_pairs(
    capture_dir: Path,
    *,
    max_offset: int = DEFAULT_MAX_ALIGNMENT_OFFSET,
    source_repository: str = UPSTREAM_REPOSITORY,
    source_revision: str = UPSTREAM_REVISION,
) -> tuple[dict[str, object], dict[str, object]]:
    """Align all 16 raw/output pairs and constrain the 512 ADC-bin boundaries."""
    bins: list[dict[str, int | None]] = [
        {"min_adc": None, "max_adc": None, "sample_count": 0}
        for _ in range(BIN_COUNT)
    ]
    volumes: list[dict[str, int]] = []
    quality: list[dict[str, int]] = []
    global_mapping = [-1] * ADC_LEVELS
    global_conflicts = 0
    paired_samples = 0

    for volume in range(16):
        raw_path = capture_dir / f"volume{volume}.raw"
        output_path = capture_dir / f"volume{volume}.out"
        raw_samples = load_integer_stream(raw_path, ADC_LEVELS - 1, "ADC")
        output_bins = load_integer_stream(output_path, BIN_COUNT - 1, "output-bin")

        offset, pair_count, conflicts, violations = infer_alignment_offset(
            raw_samples, output_bins, max_offset
        )
        if conflicts or violations:
            raise ValueError(
                f"volume{volume}: selected alignment is not clean "
                f"(conflicts={conflicts}, violations={violations})"
            )

        for raw_index, adc_code in enumerate(raw_samples):
            output_index = raw_index + offset
            if not 0 <= output_index < len(output_bins):
                continue

            bin_index = output_bins[output_index]
            entry = bins[bin_index]
            minimum = entry["min_adc"]
            maximum = entry["max_adc"]
            entry["min_adc"] = adc_code if minimum is None else min(minimum, adc_code)
            entry["max_adc"] = adc_code if maximum is None else max(maximum, adc_code)
            entry["sample_count"] = int(entry["sample_count"]) + 1

            previous = global_mapping[adc_code]
            if previous == -1:
                global_mapping[adc_code] = bin_index
            elif previous != bin_index:
                global_conflicts += 1

        paired_samples += pair_count
        volumes.append(
            {
                "volume": volume,
                "raw_samples": len(raw_samples),
                "out_samples": len(output_bins),
                "alignment_offset": offset,
                "paired_samples": pair_count,
            }
        )
        quality.append(
            {
                "volume": volume,
                "alignment_offset": offset,
                "paired_samples": pair_count,
                "mapping_conflicts": conflicts,
                "monotonicity_violations": violations,
            }
        )

    global_monotonicity_violations = 0
    previous_bin = -1
    for bin_index in global_mapping:
        if bin_index < 0:
            continue
        if bin_index < previous_bin:
            global_monotonicity_violations += 1
        elif bin_index > previous_bin:
            previous_bin = bin_index

    observed = [
        index for index, entry in enumerate(bins) if int(entry["sample_count"]) > 0
    ]
    missing = [
        index for index, entry in enumerate(bins) if int(entry["sample_count"]) == 0
    ]

    boundary_intervals: list[dict[str, int]] = []
    for index in range(BIN_COUNT - 1):
        left = bins[index]
        right = bins[index + 1]
        if int(left["sample_count"]) == 0 or int(right["sample_count"]) == 0:
            continue
        minimum_boundary = int(left["max_adc"])
        maximum_boundary = int(right["min_adc"]) - 1
        boundary_intervals.append(
            {
                "after_bin": index,
                "min_adc_boundary": minimum_boundary,
                "max_adc_boundary": maximum_boundary,
                "width": maximum_boundary - minimum_boundary,
            }
        )

    widths = [item["width"] for item in boundary_intervals if item["width"] >= 0]
    impossible_boundary_count = sum(
        item["width"] < 0 for item in boundary_intervals
    )
    exact_boundary_count = sum(item["width"] == 0 for item in boundary_intervals)

    report: dict[str, object] = {
        "source_repository": source_repository,
        "source_revision": source_revision,
        "alignment_definition": (
            ALIGNMENT_DEFINITION.replace("[-8,+8]", f"[-{max_offset},+{max_offset}]")
        ),
        "volumes": volumes,
        "bins": bins,
        "alignment_offsets_note": ALIGNMENT_NOTE.replace(
            "-8..+8", f"-{max_offset}..+{max_offset}"
        ),
        "reconstruction": {
            "paired_samples": paired_samples,
            "observed_bin_count": len(observed),
            "observed_bin_range": [observed[0], observed[-1]] if observed else None,
            "missing_bins": missing,
            "boundary_intervals": boundary_intervals,
            "impossible_boundary_count": impossible_boundary_count,
            "exact_boundary_count": exact_boundary_count,
            "boundary_width_percentiles": {
                "p50": percentile(widths, 0.50),
                "p90": percentile(widths, 0.90),
                "p99": percentile(widths, 0.99),
                "max": max(widths),
            },
        },
    }
    quality_report: dict[str, object] = {
        "volumes": quality,
        "global_mapping_conflicts": global_conflicts,
        "global_monotonicity_violations": global_monotonicity_violations,
        "impossible_boundary_count": impossible_boundary_count,
    }
    return report, quality_report


def main() -> int:
    """Run the paired-capture audit and emit the reproducible JSON report."""
    parser = argparse.ArgumentParser(
        description="Reconstruct YM2413 ADC-bin boundary constraints from volume pairs."
    )
    parser.add_argument(
        "--capture-dir",
        type=Path,
        required=True,
        help="directory containing volume0.raw/.out through volume15.raw/.out",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="write the complete JSON report to this path",
    )
    parser.add_argument(
        "--max-offset",
        type=int,
        default=DEFAULT_MAX_ALIGNMENT_OFFSET,
        help="maximum absolute file-sequence alignment offset (default: 8)",
    )
    parser.add_argument(
        "--require-global-consistency",
        action="store_true",
        help="fail unless all aligned captures share one monotonic ADC-to-bin mapping",
    )
    args = parser.parse_args()

    if args.max_offset < 0:
        parser.error("--max-offset must be non-negative")

    try:
        report, quality = audit_volume_pairs(
            args.capture_dir,
            max_offset=args.max_offset,
        )
    except ValueError as error:
        parser.error(str(error))

    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        try:
            args.output.write_text(rendered, encoding="utf-8")
        except OSError as error:
            parser.error(f"cannot write JSON report {args.output}: {error}")
        print(f"wrote {args.output}")

    print(json.dumps(quality, sort_keys=True))
    if args.require_global_consistency and (
        int(quality["global_mapping_conflicts"]) != 0
        or int(quality["global_monotonicity_violations"]) != 0
        or int(quality["impossible_boundary_count"]) != 0
    ):
        return 2

    if not args.output:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
