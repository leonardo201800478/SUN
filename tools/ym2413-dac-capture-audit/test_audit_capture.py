from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from audit_capture import (
    ADC_LEVELS,
    audit_capture,
    detect_peaks,
    insert_missing_peaks,
    integer_bin_boundaries,
    load_capture,
    quadratic_peak,
    triangular_filter,
)


class CaptureLoaderTests(unittest.TestCase):
    def test_loads_unsigned_offset_binary_samples(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.raw"
            path.write_text("0 32768 65535\n", encoding="ascii")
            self.assertEqual(load_capture(path), [0, 32768, 65535])

    def test_rejects_signed_or_out_of_range_samples(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.raw"
            path.write_text("-1 32768", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "outside unsigned 16-bit"):
                load_capture(path)

    def test_rejects_empty_capture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.raw"
            path.write_text("  \n", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "no ADC samples"):
                load_capture(path)


class CalibrationAlgorithmTests(unittest.TestCase):
    def test_triangular_filter_matches_weighted_kernel_and_zero_padding(self) -> None:
        histogram = [0] * ADC_LEVELS
        histogram[0] = 4
        histogram[1] = 2
        filtered = triangular_filter(histogram, 1)
        self.assertEqual(filtered[0], (2 * 4 + 1 * 2) / 4)
        self.assertEqual(filtered[1], (1 * 4 + 2 * 2) / 4)

    def test_quadratic_peak_returns_sub_bin_vertex(self) -> None:
        self.assertAlmostEqual(quadratic_peak(1.0, 4.0, 2.0), 1.0 / 10.0)

    def test_peak_detector_finds_separated_local_maxima(self) -> None:
        signal = [0.0] * 300
        for center in (20, 220):
            signal[center - 1 : center + 2] = [2.0, 10.0, 3.0]
        peaks = detect_peaks(signal)
        self.assertEqual(len(peaks), 2)
        self.assertAlmostEqual(peaks[0], 20.0 + quadratic_peak(2.0, 10.0, 3.0))
        self.assertAlmostEqual(peaks[1], 220.0 + quadratic_peak(2.0, 10.0, 3.0))

    def test_missing_peak_insertion_is_explicit_and_strictly_greater_than_160(self) -> None:
        reconstructed, inserted = insert_missing_peaks([10.0, 170.0, 371.0])
        self.assertEqual(inserted, [270.5])
        self.assertEqual(reconstructed, [10.0, 170.0, 270.5, 371.0])

    def test_bin_boundaries_end_at_unsigned_adc_limit(self) -> None:
        self.assertEqual(integer_bin_boundaries([10.0, 20.0, 40.0]), [15, 30, 65536])

    def test_bin_boundaries_reject_non_monotonic_peaks(self) -> None:
        with self.assertRaisesRegex(ValueError, "strictly increasing"):
            integer_bin_boundaries([20.0, 10.0])

    def test_end_to_end_synthetic_capture_reports_measured_and_inserted_peaks(self) -> None:
        samples: list[int] = []
        for center in (8_000, 8_240, 8_600):
            samples.extend([center] * 100)
            samples.extend([center - 1] * 20)
            samples.extend([center + 1] * 20)
        audit = audit_capture(samples, filter_width=2)
        self.assertEqual(audit.sample_count, len(samples))
        self.assertEqual(audit.minimum_adc_code, 7_999)
        self.assertEqual(audit.maximum_adc_code, 8_601)
        self.assertGreaterEqual(audit.detected_peak_count, 2)
        self.assertGreaterEqual(audit.bin_count, audit.detected_peak_count)
        self.assertEqual(
            audit.inserted_peak_count,
            audit.bin_count - audit.detected_peak_count,
        )


if __name__ == "__main__":
    unittest.main()
