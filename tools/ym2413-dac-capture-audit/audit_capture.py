#!/usr/bin/env python3
"""Reproduce the pinned Andete YM2413 ADC calibration pipeline.

Input values are unsigned offset-binary samples written by analyze/common.cc
after adding 0x8000 to the signed ADC result.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path

ADC_LEVELS = 1 << 16
DEFAULT_FILTER_WIDTH = 30
DEFAULT_EXPECTED_BINS = 512


@dataclass(frozen=True)
class CaptureAudit:
    sample_count: int
    unique_adc_codes: int
    minimum_adc_code: int
    maximum_adc_code: int
    detected_peaks: tuple[float, ...]
    inserted_peak_midpoints: tuple[float, ...]
    bin_boundaries: tuple[int, ...]

    @property
    def detected_peak_count(self) -> int:
        return len(self.detected_peaks)

    @property
    def inserted_peak_count(self) -> int:
        return len(self.inserted_peak_midpoints)

    @property
    def bin_count(self) -> int:
        return len(self.bin_boundaries)


def load_capture(path: Path) -> list[int]:
    """Load whitespace-separated unsigned offset-binary ADC samples."""
    try:
        tokens = path.read_text(encoding="ascii").split()
    except OSError as error:
        raise ValueError(f"cannot read capture {path}: {error}") from error

    if not tokens:
        raise ValueError("capture contains no ADC samples")

    samples: list[int] = []
    for index, token in enumerate(tokens):
        try:
            value = int(token, 10)
        except ValueError as error:
            raise ValueError(f"invalid integer at sample {index}: {token!r}") from error
        if not 0 <= value < ADC_LEVELS:
            raise ValueError(
                f"sample {index} is outside unsigned 16-bit range: {value}"
            )
        samples.append(value)
    return samples


def build_histogram(samples: list[int]) -> list[int]:
    """Build the 65,536-entry ADC histogram used by calibrate.cc."""
    histogram = [0] * ADC_LEVELS
    for value in samples:
        histogram[value] += 1
    return histogram


def triangular_filter(histogram: list[int], width: int) -> list[float]:
    """Apply the exact triangular smoothing kernel from calibrate.cc."""
    if width < 1:
        raise ValueError("filter width must be at least 1")
    if len(histogram) != ADC_LEVELS:
        raise ValueError(f"histogram must contain exactly {ADC_LEVELS} entries")

    denominator = sum(width + 1 - abs(offset) for offset in range(-width, width + 1))
    filtered = [0.0] * len(histogram)
    for index in range(len(histogram)):
        weighted_sum = 0
        for offset in range(-width, width + 1):
            source_index = index + offset
            if 0 <= source_index < len(histogram):
                weight = width + 1 - abs(offset)
                weighted_sum += weight * histogram[source_index]
        filtered[index] = weighted_sum / denominator
    return filtered


def quadratic_peak(left: float, middle: float, right: float) -> float:
    """Return the parabolic peak offset used by the upstream calibration."""
    denominator = 2.0 * left + 2.0 * right - 4.0 * middle
    if denominator == 0.0:
        return 0.0
    return (left - right) / denominator


def detect_peaks(filtered: list[float]) -> list[float]:
    """Reproduce detectPeaks(), including its 32-bin post-peak skip."""
    peaks: list[float] = []
    index = 0
    size = len(filtered)

    while index < size:
        if filtered[index] < 1.0:
            index += 1
            continue

        while index + 1 < size and filtered[index + 1] >= filtered[index]:
            index += 1
        if index + 1 >= size:
            break

        left = filtered[index - 1] if index > 0 else 0.0
        middle = filtered[index]
        right = filtered[index + 1]
        peaks.append(index + quadratic_peak(left, middle, right))

        index += 2 * 16
        while index + 1 < size and filtered[index + 1] <= filtered[index]:
            index += 1
        index += 1  # The upstream for-loop increments after the valley scan.

    return peaks


def insert_missing_peaks(peaks: list[float]) -> tuple[list[float], list[float]]:
    """Apply upstream's single-midpoint insertion for gaps greater than 160."""
    if not peaks:
        return [], []

    result = [peaks[0]]
    inserted: list[float] = []
    for previous, current in zip(peaks, peaks[1:]):
        if current - previous > 10.0 * 16:
            midpoint = (previous + current) / 2.0
            result.append(midpoint)
            inserted.append(midpoint)
        result.append(current)
    return result, inserted


def integer_bin_boundaries(peaks: list[float]) -> list[int]:
    """Build the same integer midpoint boundaries and terminal 65,536 edge."""
    if not peaks:
        raise ValueError("no peaks detected; cannot construct calibration bins")
    if any(right <= left for left, right in zip(peaks, peaks[1:])):
        raise ValueError("peak positions must be strictly increasing")

    boundaries = [
        int((left + right) / 2.0) for left, right in zip(peaks, peaks[1:])
    ]
    boundaries.append(ADC_LEVELS)
    return boundaries


def audit_capture(samples: list[int], filter_width: int = DEFAULT_FILTER_WIDTH) -> CaptureAudit:
    """Run the reproducible ADC histogram, peak and bin reconstruction."""
    if not samples:
        raise ValueError("capture contains no ADC samples")
    if any(not 0 <= value < ADC_LEVELS for value in samples):
        raise ValueError("ADC samples must be unsigned 16-bit values")

    histogram = build_histogram(samples)
    filtered = triangular_filter(histogram, filter_width)
    detected = detect_peaks(filtered)
    reconstructed, inserted = insert_missing_peaks(detected)
    boundaries = integer_bin_boundaries(reconstructed)

    return CaptureAudit(
        sample_count=len(samples),
        unique_adc_codes=len(set(samples)),
        minimum_adc_code=min(samples),
        maximum_adc_code=max(samples),
        detected_peaks=tuple(detected),
        inserted_peak_midpoints=tuple(inserted),
        bin_boundaries=tuple(boundaries),
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Reproduce the pinned Andete YM2413 ADC calibration algorithm."
    )
    parser.add_argument("capture", type=Path, help="whitespace-separated .raw ADC capture")
    parser.add_argument("--filter-width", type=int, default=DEFAULT_FILTER_WIDTH)
    parser.add_argument("--expected-bins", type=int, default=DEFAULT_EXPECTED_BINS)
    parser.add_argument(
        "--require-expected-bins",
        action="store_true",
        help="return exit status 2 unless the reconstructed bin count matches --expected-bins",
    )
    parser.add_argument("--json", type=Path, help="write the complete audit as JSON")
    args = parser.parse_args()

    try:
        audit = audit_capture(load_capture(args.capture), args.filter_width)
    except ValueError as error:
        parser.error(str(error))

    report = asdict(audit)
    report.update(
        detected_peak_count=audit.detected_peak_count,
        inserted_peak_count=audit.inserted_peak_count,
        bin_count=audit.bin_count,
        expected_bin_count=args.expected_bins,
        expected_bin_count_matches=audit.bin_count == args.expected_bins,
        adc_encoding="unsigned offset-binary (signed ADC sample + 0x8000, modulo 65536)",
        calibration_note=(
            "Bin indices are calibrated ADC voltage ranks, not raw YM2413 DAC words."
        ),
    )
    rendered = json.dumps(report, indent=2)
    print(rendered)
    if args.json:
        args.json.write_text(rendered + "\n", encoding="utf-8")

    if args.require_expected_bins and audit.bin_count != args.expected_bins:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
