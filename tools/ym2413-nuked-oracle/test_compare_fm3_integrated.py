"""Tests for the integrated YM2413 MO/RO output-pipeline comparator."""

from __future__ import annotations

import unittest

from compare_fm3_integrated import (
    compare_measured_adapter_frame,
    compare_output_pipeline_row,
    measured_code_from_operator_output,
)


def make_pair(call_index: int) -> tuple[dict[str, object], dict[str, object]]:
    """Build matching Nuked/Rust output rows at a selected pipeline warm-up point."""
    cycle_before = call_index % 18
    cycle_after = (cycle_before + 1) % 18
    ch_out_before = 0 if call_index == 0 else 10 + call_index
    ch_out_after = -30 + call_index
    nuked = {
        "cycle_before": cycle_before,
        "cycle_after": cycle_after,
        "output_pipe_ch_out_before": ch_out_before,
        "output_pipe_rm_enable_before": -64,
        "ch_out": ch_out_after,
        "rm_enable": 192,
        "output_pins_m": 100 + call_index,
        "output_pins_r": -100 - call_index,
        "output_m": 20 + call_index,
        "output_r": -20 - call_index,
    }
    frame_valid = True
    pins_valid = True
    rust = {
        "output_frame_cycle": cycle_before,
        "output_frame_m": 20 + call_index if frame_valid else None,
        "output_frame_m_valid": frame_valid,
        "output_frame_r": -20 - call_index if frame_valid else None,
        "output_frame_r_valid": frame_valid,
        "output_pins_m": 100 + call_index,
        "output_pins_m_valid": pins_valid,
        "output_pins_r": -100 - call_index,
        "output_pins_r_valid": pins_valid,
        "output_pipe_ch_out_before": ch_out_before,
        "output_pipe_ch_out_before_valid": True,
        "output_pipe_rm_enable_before": -64,
        "output_pipe_ch_out_after": ch_out_after,
        "output_pipe_ch_out_after_valid": True,
        "output_pipe_rm_enable_after": 192,
        "output_pipeline_next_cycle": cycle_after,
    }
    return nuked, rust


class IntegratedOutputPipelineTests(unittest.TestCase):
    """Validate the one-call MO/RO output-buffer delay and pin-stage comparison."""

    def test_first_call_compares_nuked_software_reset_output(self) -> None:
        """The software pipeline starts at Nuked's explicit zero reset state."""
        nuked, rust = make_pair(0)
        compare_output_pipeline_row(0, nuked, rust)

    def test_second_call_compares_pin_latches(self) -> None:
        """MO/RO pin latches remain comparable after the first serial call."""
        nuked, rust = make_pair(1)
        compare_output_pipeline_row(1, nuked, rust)

    def test_third_call_compares_delayed_mo_ro_buffer(self) -> None:
        """The frame matches the buffer value from the previous OPLL call."""
        nuked, rust = make_pair(2)
        compare_output_pipeline_row(2, nuked, rust)

    def test_rejects_mo_pin_mismatch(self) -> None:
        """A wrong MO pin value is a hard differential failure."""
        nuked, rust = make_pair(1)
        rust["output_pins_m"] = int(rust["output_pins_m"]) + 1
        with self.assertRaisesRegex(ValueError, "output_pins_m mismatch"):
            compare_output_pipeline_row(1, nuked, rust)

    def test_rejects_ro_buffer_mismatch(self) -> None:
        """A wrong delayed RO buffer value is a hard differential failure."""
        nuked, rust = make_pair(2)
        rust["output_frame_r"] = int(rust["output_frame_r"]) - 1
        with self.assertRaisesRegex(ValueError, "output_frame_r mismatch"):
            compare_output_pipeline_row(2, nuked, rust)

    def test_rejects_unknown_output_buffer_from_software_reset(self) -> None:
        """The Nuked-matched software pipeline must retain known MO/RO buffers."""
        nuked, rust = make_pair(0)
        rust["output_frame_m_valid"] = False
        with self.assertRaisesRegex(ValueError, "output_frame_m_valid"):
            compare_output_pipeline_row(0, nuked, rust)

    def test_rejects_unknown_pin_after_pipeline_warmup(self) -> None:
        """A melodic output may not silently become unknown after warm-up."""
        nuked, rust = make_pair(1)
        rust["output_pins_r_valid"] = False
        with self.assertRaisesRegex(ValueError, "output_pins_r_valid"):
            compare_output_pipeline_row(1, nuked, rust)


class MeasuredAdapterDifferentialTests(unittest.TestCase):
    """Validate the delayed measured-domain MO adapter against prior Nuked latches."""

    def make_rows(
        self,
        call_index: int,
        previous_ch_out: int = 12,
    ) -> tuple[dict[str, object], dict[str, object], dict[str, object]]:
        """Build matching current/previous Nuked rows and one emitted Rust frame."""
        previous_cycle = (call_index - 1) % 18
        current_cycle = call_index % 18
        slot_index = (previous_cycle + 3) % 18
        active = slot_index in {3, 4, 5, 9, 10, 11, 15, 16, 17}
        known = not active or call_index > 1
        level_valid = active and call_index > 1
        value, negative_zero = measured_code_from_operator_output(previous_ch_out)
        previous_nuked = {
            "cycle_before": previous_cycle,
            "output_pipe_ch_out_before": previous_ch_out,
        }
        nuked = {"cycle_before": current_cycle}
        rust = {
            "measured_frame_present": True,
            "measured_frame_emitted_cycle": current_cycle,
            "measured_frame_source_cycle": previous_cycle,
            "measured_frame_source_mo_slot_index": slot_index,
            "measured_frame_source_mo_known": known,
            "measured_frame_source_mo_level_valid": level_valid,
            "measured_frame_source_mo_level": value if level_valid else None,
            "measured_frame_source_mo_negative_zero": negative_zero if level_valid else False,
            "measured_frame_source_ro_mode_inactive": True,
        }
        return nuked, rust, previous_nuked

    def test_first_call_has_no_emitted_measured_frame(self) -> None:
        """The adapter cannot emit a frame before the first serial input."""
        nuked = {"cycle_before": 0}
        rust = {
            "measured_frame_present": False,
            "measured_frame_source_mo_level": None,
        }
        compare_measured_adapter_frame(0, nuked, rust, None)

    def test_active_mo_slot_preserves_negative_zero(self) -> None:
        """A -1 operator latch is preserved as the distinct negative-zero code."""
        nuked, rust, previous = self.make_rows(2, previous_ch_out=-1)
        self.assertEqual(rust["measured_frame_source_mo_level"], 0)
        self.assertTrue(rust["measured_frame_source_mo_negative_zero"])
        compare_measured_adapter_frame(2, nuked, rust, previous)

    def test_negative_full_scale_maps_to_negative_255(self) -> None:
        """The minimum operator code maps to the measured -255 endpoint."""
        nuked, rust, previous = self.make_rows(3, previous_ch_out=-256)
        self.assertEqual(rust["measured_frame_source_mo_level"], -255)
        self.assertFalse(rust["measured_frame_source_mo_negative_zero"])
        compare_measured_adapter_frame(3, nuked, rust, previous)

    def test_inactive_mo_slot_is_known_without_a_pulse(self) -> None:
        """An inactive carrier window is known and must not carry a level."""
        nuked, rust, previous = self.make_rows(4, previous_ch_out=33)
        self.assertFalse(rust["measured_frame_source_mo_level_valid"])
        self.assertTrue(rust["measured_frame_source_mo_known"])
        compare_measured_adapter_frame(4, nuked, rust, previous)

    def test_cycle_zero_after_wrap_keeps_measured_output_valid(self) -> None:
        """The first-call unknown state must not recur at the 18-slot wrap."""
        nuked, rust, previous = self.make_rows(19, previous_ch_out=-1)
        self.assertEqual(rust["measured_frame_source_cycle"], 0)
        self.assertTrue(rust["measured_frame_source_mo_known"])
        self.assertTrue(rust["measured_frame_source_mo_level_valid"])
        compare_measured_adapter_frame(19, nuked, rust, previous)

    def test_rejects_wrong_previous_cycle(self) -> None:
        """A frame emitted one cycle late or early is a differential failure."""
        nuked, rust, previous = self.make_rows(2, previous_ch_out=12)
        rust["measured_frame_source_cycle"] = 0
        with self.assertRaisesRegex(ValueError, "source_cycle mismatch"):
            compare_measured_adapter_frame(2, nuked, rust, previous)

    def test_rejects_wrong_negative_zero(self) -> None:
        """The comparator distinguishes +0 from -0."""
        nuked, rust, previous = self.make_rows(2, previous_ch_out=-1)
        rust["measured_frame_source_mo_negative_zero"] = False
        with self.assertRaisesRegex(ValueError, "measured MO level mismatch"):
            compare_measured_adapter_frame(2, nuked, rust, previous)


if __name__ == "__main__":
    unittest.main()
