from __future__ import annotations

import unittest

from compare_volume_capture import (
    candidate_bin_from_channel_output,
    candidate_bin_from_output_pin,
    score_alignment,
    select_sample_cycle,
)


class CandidateMappingTests(unittest.TestCase):
    def test_channel_output_mapping_preserves_the_signed_domain(self) -> None:
        self.assertEqual(candidate_bin_from_channel_output(255), 0)
        self.assertEqual(candidate_bin_from_channel_output(0), 255)
        self.assertEqual(candidate_bin_from_channel_output(-1), 256)
        self.assertEqual(candidate_bin_from_channel_output(-256), 511)

    def test_mo_pin_mapping_accounts_for_positive_offset(self) -> None:
        self.assertEqual(candidate_bin_from_output_pin(256), 0)
        self.assertEqual(candidate_bin_from_output_pin(1), 255)
        self.assertEqual(candidate_bin_from_output_pin(-1), 256)
        self.assertEqual(candidate_bin_from_output_pin(-256), 511)

    def test_mo_pin_zero_is_not_a_valid_dual_zero_code(self) -> None:
        with self.assertRaises(ValueError):
            candidate_bin_from_output_pin(0)

    def test_rejects_output_outside_nine_bit_domain(self) -> None:
        with self.assertRaises(ValueError):
            candidate_bin_from_channel_output(256)
        with self.assertRaises(ValueError):
            candidate_bin_from_channel_output(-257)


class SampleCycleTests(unittest.TestCase):
    def test_selects_the_unique_active_mo_slot(self) -> None:
        counts = [0] * 18
        magnitudes = [1] * 18
        counts[7] = 9
        magnitudes[7] = 210
        self.assertEqual(select_sample_cycle(counts, magnitudes), 7)

    def test_rejects_ambiguous_active_slots(self) -> None:
        counts = [0] * 18
        magnitudes = [1] * 18
        counts[3] = counts[8] = 4
        magnitudes[3] = magnitudes[8] = 180
        with self.assertRaisesRegex(ValueError, "could not identify one active MO slot"):
            select_sample_cycle(counts, magnitudes)


class AlignmentTests(unittest.TestCase):
    def test_finds_known_sample_phase_offset(self) -> None:
        measured = [12, 24, 36, 48, 60, 72]
        model = [300, 301, 12, 24, 36, 48, 60, 72, 302]
        result = score_alignment(
            measured, model, phase_search=2, prefix_samples=6
        )
        self.assertEqual(result["best_phase_offset_samples"], 2)
        self.assertEqual(result["full_exact_matches"], 6)
        self.assertEqual(result["full_exact_match_rate"], 1.0)

    def test_reports_error_when_no_exact_alignment_exists(self) -> None:
        measured = [1, 2, 3, 4]
        model = [10, 20, 30, 40, 50]
        result = score_alignment(
            measured, model, phase_search=1, prefix_samples=4
        )
        self.assertEqual(result["full_exact_matches"], 0)
        self.assertGreater(result["mean_absolute_error"], 0)


if __name__ == "__main__":
    unittest.main()
