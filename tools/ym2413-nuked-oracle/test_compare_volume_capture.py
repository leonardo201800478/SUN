from __future__ import annotations

import unittest
from io import StringIO

from compare_volume_capture import (
    candidate_bin_from_channel_output,
    candidate_bin_from_output_pin,
    local_phase_alignment,
    residual_profile,
    score_alignment,
    select_sample_cycle,
    read_output_trace,
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

    def test_residual_profile_detects_a_monotonic_offset_mapping(self) -> None:
        measured = [101, 102, 104, 105]
        model = [100, 101, 103, 104]
        result = residual_profile(measured, model, 0)
        self.assertEqual(result["median_signed_error"], 1)
        self.assertEqual(result["empirical_mapping_monotonicity_violations"], 0)

    def test_local_phase_alignment_finds_window_offset(self) -> None:
        measured = [2, 3, 4, 5, 6, 7, 8, 9]
        model = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        windows = local_phase_alignment(
            measured, model, 0, window_size=4, search_radius=2
        )
        self.assertEqual(
            [window["phase_delta_from_global"] for window in windows],
            [2, 2],
        )


class OutputTraceValidationTests(unittest.TestCase):
    @staticmethod
    def make_trace(skip_call=None, wrong_cycle=False):
        rows = ['{"schema":"ym2413-nuked-opll-output-trace-v2",'
                '"time_unit":"opll_clock_call","sample_point":"after_OPLL_Clock"}']
        for call_index in range(18):
            if call_index == skip_call:
                continue
            cycle = (call_index + 1) % 18 if wrong_cycle and call_index == 4 else call_index % 18
            active = cycle == 6
            channel_output = 100 if active else 255
            output_pin = 101 if active else 1
            rows.append(
                '{"call_index":%d,"cycle_before":%d,"ch_out_before":%d,"output_pins_m":%d,"output_pins_r":-1}'
                % (call_index, cycle, channel_output, output_pin)
            )
        return StringIO("\\n".join(rows) + "\\n")

    def test_accepts_contiguous_full_slot_scan(self):
        bins, diagnostics = read_output_trace(self.make_trace(), scan_calls=18, capture_start_call=0)
        self.assertEqual(diagnostics["sample_cycle"], 6)
        self.assertEqual(len(bins), 1)
        self.assertEqual(bins[0], 155)

    def test_rejects_missing_call_index(self):
        with self.assertRaisesRegex(ValueError, "non-contiguous trace call index"):
            read_output_trace(self.make_trace(skip_call=4), scan_calls=18, capture_start_call=0)

    def test_rejects_cycle_index_inconsistent_with_call_index(self):
        with self.assertRaisesRegex(ValueError, "trace cycle mismatch"):
            read_output_trace(self.make_trace(wrong_cycle=True), scan_calls=18, capture_start_call=0)

    def test_rejects_invalid_sample_point(self):
        trace = self.make_trace().getvalue().replace('"sample_point":"after_OPLL_Clock"', '"sample_point":"before_OPLL_Clock"')
        with self.assertRaisesRegex(ValueError, "sample_point"):
            read_output_trace(StringIO(trace), scan_calls=18, capture_start_call=0)

if __name__ == "__main__":
    unittest.main()
