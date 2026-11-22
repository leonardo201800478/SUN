from __future__ import annotations

import unittest

from audit_volume_pairs import (
    evaluate_alignment,
    infer_alignment_offset,
)


def output_bin(adc_code: int) -> int:
    return (adc_code - 100) // 100


class AlignmentTests(unittest.TestCase):
    def setUp(self) -> None:
        self.raw = [
            100, 200, 300, 400, 500, 600, 700, 800,
            100, 200, 300, 400, 500, 600, 700, 800,
        ]

    def test_infers_negative_two_sample_offset(self) -> None:
        output = [output_bin(value) for value in self.raw[2:]]
        offset, paired, conflicts, violations = infer_alignment_offset(
            self.raw, output
        )
        self.assertEqual(offset, -2)
        self.assertEqual(paired, len(output))
        self.assertEqual(conflicts, 0)
        self.assertEqual(violations, 0)

    def test_infers_positive_one_sample_offset(self) -> None:
        output = [511, *(output_bin(value) for value in self.raw)]
        offset, paired, conflicts, violations = infer_alignment_offset(
            self.raw, output
        )
        self.assertEqual(offset, 1)
        self.assertEqual(paired, len(self.raw))
        self.assertEqual(conflicts, 0)
        self.assertEqual(violations, 0)

    def test_prefers_zero_offset_when_full_length_is_clean(self) -> None:
        output = [output_bin(value) for value in self.raw]
        offset, paired, conflicts, violations = infer_alignment_offset(
            self.raw, output
        )
        self.assertEqual(offset, 0)
        self.assertEqual(paired, len(self.raw))
        self.assertEqual(conflicts, 0)
        self.assertEqual(violations, 0)

    def test_reports_mapping_conflicts_and_monotonicity(self) -> None:
        raw = [100, 200, 100, 200, 300, 400]
        output = [1, 2, 3, 4, 6, 5]
        conflicts, violations, paired = evaluate_alignment(raw, output, 0)
        self.assertGreater(conflicts, 0)
        self.assertGreater(violations, 0)
        self.assertEqual(paired, len(raw))

    def test_rejects_alignment_without_a_clean_candidate(self) -> None:
        raw = [100, 200, 100, 200, 100, 200, 100, 200]
        output = [7, 1, 6, 2, 5, 3, 4, 0]
        with self.assertRaisesRegex(ValueError, "no clean raw/output alignment"):
            infer_alignment_offset(raw, output, max_offset=0)


if __name__ == "__main__":
    unittest.main()
