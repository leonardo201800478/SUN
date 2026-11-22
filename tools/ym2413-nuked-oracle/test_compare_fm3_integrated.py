"""Tests for the integrated YM2413 MO/RO output-pipeline comparator."""

from __future__ import annotations

import unittest

from compare_fm3_integrated import compare_output_pipeline_row


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


if __name__ == "__main__":
    unittest.main()
