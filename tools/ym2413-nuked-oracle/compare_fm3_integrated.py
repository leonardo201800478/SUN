#!/usr/bin/env python3
"""Compare the complete Rust serial FM-3 pipeline with a full Nuked OPLL trace."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


SCALAR_FIELDS = (
    "cycle_before",
    "cycle_after",
    "pg_inc",
    "pg_phase_next",
    "pg_out",
    "op_calc_eg_out",
    "op_calc_eg_silent",
    "op_calc_c_fb",
    "op_calc_c_dc",
    "op_calc_c_dm",
    "op_calc_pg_out",
    "op_calc_op_mod",
    "op_calc_op_fbsum",
    "lfo_counter",
    "lfo_vib_counter",
    "lfo_am_counter",
    "lfo_am_dir",
    "lfo_am_car",
    "lfo_am_step",
    "lfo_am_out",
    "c_instr",
    "c_op",
    "c_tl",
    "c_fb",
    "c_am",
    "c_vib",
    "c_ksr",
    "c_ksl",
    "c_multi",
    "c_fnum",
    "c_block",
    "c_et",
    "c_sl",
    "c_ksr_freq",
    "c_dc",
    "c_dm",
    "c_ksl_freq",
    "c_ksl_block",
    "eg_counter_state",
    "eg_counter_state_prev",
    "eg_timer",
    "eg_timer_carry",
    "eg_timer_low_lock",
    "eg_timer_shift",
    "eg_timer_shift_lock",
    "eg_timer_shift_stop",
    "eg_kon",
    "eg_dokon",
    "eg_off",
    "eg_rate",
    "eg_rate_hi",
    "eg_maxrate",
    "eg_zerorate",
    "eg_inc_lo",
    "eg_inc_hi",
    "eg_sl",
    "eg_silent",
    "eg_out",
    "eg_ksltl",
    "op_fbsum",
    "op_mod",
    "op_neg",
    "op_logsin",
    "op_exp_m",
    "op_exp_s",
    "op_phase_index",
    "op_output",
    "ch_out",
)
ARRAY_FIELDS = (
    "pg_phase",
    "c_adrr",
    "eg_state",
    "eg_level",
    "op_fb1",
    "op_fb2",
)


def load_jsonl(path: Path, label: str) -> list[dict[str, object]]:
    """Load a JSONL trace and validate its rows."""
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows:
        raise ValueError(f"{label} trace is empty")
    return rows


def compare_output_pipeline_row(
    call_index: int,
    nuked: dict[str, object],
    rust: dict[str, object],
) -> None:
    """Compare integrated MO/RO buffer and pin latches at their exact call boundaries."""
    required = (
        "output_pipe_ch_out_before",
        "output_pipe_ch_out_before_valid",
        "output_pipe_rm_enable_before",
        "output_pipe_ch_out_after",
        "output_pipe_ch_out_after_valid",
        "output_pipe_rm_enable_after",
        "output_pipeline_next_cycle",
        "output_frame_cycle",
        "output_frame_m",
        "output_frame_m_valid",
        "output_frame_r",
        "output_frame_r_valid",
        "output_pins_m",
        "output_pins_m_valid",
        "output_pins_r",
        "output_pins_r_valid",
    )
    missing = [field for field in required if field not in rust]
    missing.extend(
        field for field in (
            "output_pipe_ch_out_before",
            "output_pipe_rm_enable_before",
            "ch_out",
            "rm_enable",
            "cycle_before",
            "cycle_after",
            "output_m",
            "output_r",
            "output_pins_m",
            "output_pins_r",
        ) if field not in nuked
    )
    if missing:
        raise ValueError(f"call {call_index}: missing MO/RO pipeline fields {missing}")

    pairs = (
        ("output_pipe_ch_out_before", "output_pipe_ch_out_before"),
        ("output_pipe_rm_enable_before", "output_pipe_rm_enable_before"),
        ("output_pipe_ch_out_after", "ch_out"),
        ("output_pipe_rm_enable_after", "rm_enable"),
        ("output_pipeline_next_cycle", "cycle_after"),
        ("output_frame_cycle", "cycle_before"),
    )
    for rust_field, nuked_field in pairs:
        expected = int(nuked[nuked_field])
        actual = int(rust[rust_field])
        if expected != actual:
            raise ValueError(
                f"call {call_index}: {rust_field} mismatch "
                f"Nuked={expected} Rust={actual}"
            )

    if not bool(rust["output_pipe_ch_out_before_valid"]):
        raise ValueError(f"call {call_index}: previous ch_out unexpectedly became unknown")
    if not bool(rust["output_pipe_ch_out_after_valid"]):
        raise ValueError(f"call {call_index}: integrated operator output became unknown")

    for field in ("output_pins_m_valid", "output_pins_r_valid"):
        if not bool(rust[field]):
            raise ValueError(f"call {call_index}: {field} unexpectedly became unknown")
    for field in ("output_pins_m", "output_pins_r"):
        expected = int(nuked[field])
        actual = int(rust[field])
        if expected != actual:
            raise ValueError(
                f"call {call_index}: {field} mismatch "
                f"Nuked={expected} Rust={actual}"
            )

    for field in ("output_frame_m_valid", "output_frame_r_valid"):
        if not bool(rust[field]):
            raise ValueError(f"call {call_index}: {field} unexpectedly became unknown")
    for rust_field, nuked_field in (
        ("output_frame_m", "output_m"),
        ("output_frame_r", "output_r"),
    ):
        if rust[rust_field] is None:
            raise ValueError(f"call {call_index}: {rust_field} is null after software reset")
        expected = int(nuked[nuked_field])
        actual = int(rust[rust_field])
        if actual != expected:
            raise ValueError(
                f"call {call_index}: {rust_field} mismatch "
                f"Nuked={expected} Rust={actual}"
            )


MEASURED_MO_ACTIVE_SLOTS = {3, 4, 5, 9, 10, 11, 15, 16, 17}


def measured_code_from_operator_output(value: int) -> tuple[int, bool]:
    """Decode the 9-bit one's-complement measured amplitude, preserving negative zero."""
    if value >= 0:
        return value, False
    magnitude = ~value
    return -magnitude, magnitude == 0


def compare_measured_adapter_frame(
    call_index: int,
    nuked: dict[str, object],
    rust: dict[str, object],
    previous_nuked: dict[str, object] | None,
) -> None:
    """Compare the delayed measured MO frame with the prior Nuked ch_out latch."""
    required = (
        "measured_frame_present",
        "measured_frame_emitted_cycle",
        "measured_frame_source_cycle",
        "measured_frame_source_mo_slot_index",
        "measured_frame_source_mo_known",
        "measured_frame_source_mo_level_valid",
        "measured_frame_source_mo_level",
        "measured_frame_source_mo_negative_zero",
        "measured_frame_source_ro_mode_inactive",
    )
    missing = [field for field in required if field not in rust]
    if missing:
        raise ValueError(f"call {call_index}: missing measured-adapter fields {missing}")

    if call_index == 0:
        if bool(rust["measured_frame_present"]):
            raise ValueError("call 0: measured adapter emitted a frame before its first input")
        if rust["measured_frame_source_mo_level"] is not None:
            raise ValueError("call 0: measured adapter exposed a level without a source frame")
        return

    if previous_nuked is None:
        raise ValueError(f"call {call_index}: missing previous Nuked row for measured frame")
    if not bool(rust["measured_frame_present"]):
        raise ValueError(f"call {call_index}: measured adapter failed to emit its delayed frame")

    source_cycle = int(previous_nuked["cycle_before"])
    emitted_cycle = int(nuked["cycle_before"])
    expected_slot = (source_cycle + 3) % 18
    actual_fields = (
        ("measured_frame_emitted_cycle", emitted_cycle),
        ("measured_frame_source_cycle", source_cycle),
        ("measured_frame_source_mo_slot_index", expected_slot),
    )
    for field, expected in actual_fields:
        actual = int(rust[field])
        if actual != expected:
            raise ValueError(
                f"call {call_index}: {field} mismatch expected={expected} Rust={actual}"
            )

    active_mo = expected_slot in MEASURED_MO_ACTIVE_SLOTS
    expected_mo_known = not active_mo or call_index > 1
    expected_level_valid = active_mo and call_index > 1
    if bool(rust["measured_frame_source_mo_known"]) != expected_mo_known:
        raise ValueError(f"call {call_index}: measured MO known-state mismatch")
    if bool(rust["measured_frame_source_mo_level_valid"]) != expected_level_valid:
        raise ValueError(f"call {call_index}: measured MO pulse-presence mismatch")
    if not bool(rust["measured_frame_source_ro_mode_inactive"]):
        raise ValueError(f"call {call_index}: RO must remain inactive in the melodic fixture")

    if expected_level_valid:
        expected_value, expected_negative_zero = measured_code_from_operator_output(
            int(previous_nuked["output_pipe_ch_out_before"])
        )
        actual_value = int(rust["measured_frame_source_mo_level"])
        actual_negative_zero = bool(rust["measured_frame_source_mo_negative_zero"])
        if actual_value != expected_value or actual_negative_zero != expected_negative_zero:
            raise ValueError(
                f"call {call_index}: measured MO level mismatch "
                f"Nuked ch_out={previous_nuked['output_pipe_ch_out_before']} "
                f"expected=({expected_value}, negative_zero={expected_negative_zero}) "
                f"Rust=({actual_value}, negative_zero={actual_negative_zero})"
            )
    elif rust["measured_frame_source_mo_level"] is not None or bool(
        rust["measured_frame_source_mo_negative_zero"]
    ):
        raise ValueError(f"call {call_index}: unknown/inactive MO slot carried a level")


def compare_row(
    call_index: int,
    nuked: dict[str, object],
    rust: dict[str, object],
    previous_nuked: dict[str, object] | None = None,
) -> None:
    """Compare all integrated serial state fields for one OPLL_Clock call."""
    required_fields = (
        *SCALAR_FIELDS,
        *ARRAY_FIELDS,
        "op_calc_op_fb1",
        "op_calc_op_fb2",
    )
    missing = [
        field
        for field in required_fields
        if field not in nuked or field not in rust
    ]
    if missing:
        raise ValueError(f"call {call_index}: missing integrated fields {missing}")

    for field in ("op_calc_op_fb1", "op_calc_op_fb2"):
        expected_values = [int(value) for value in nuked[field]]
        actual_values = [int(value) for value in rust[field]]
        if expected_values != actual_values:
            for index, (expected, actual) in enumerate(
                zip(expected_values, actual_values, strict=True)
            ):
                if expected != actual:
                    raise ValueError(
                        f"call {call_index}: {field}[{index}] mismatch "
                        f"Nuked={expected} Rust={actual}"
                    )

    for field in SCALAR_FIELDS:
        expected = int(nuked[field])
        actual = int(rust[field])
        if field == "lfo_counter":
            expected &= 0x1FFF
            actual &= 0x1FFF
        if expected != actual:
            raise ValueError(
                f"call {call_index}: {field} mismatch Nuked={expected} Rust={actual}"
            )

    for field in ARRAY_FIELDS:
        expected_values = [int(value) for value in nuked[field]]
        actual_values = [int(value) for value in rust[field]]
        if expected_values != actual_values:
            for index, (expected, actual) in enumerate(
                zip(expected_values, actual_values, strict=True)
            ):
                if expected != actual:
                    raise ValueError(
                        f"call {call_index}: {field}[{index}] mismatch "
                        f"Nuked={expected} Rust={actual}"
                    )
            raise ValueError(f"call {call_index}: {field} length mismatch")

    expected_ksl = int(nuked["eg_ksltl"]) - (int(nuked["c_tl"]) << 1)
    actual_ksl = int(rust["c_ksl_attenuation"])
    if expected_ksl != actual_ksl:
        raise ValueError(
            f"call {call_index}: c_ksl_attenuation mismatch "
            f"Nuked-derived={expected_ksl} Rust={actual_ksl}"
        )

    if int(rust["op_calc_pg_out"]) != int(nuked["op_calc_pg_out"]):
        raise ValueError(f"call {call_index}: pre-operator PG output latch mismatch")
    if int(rust["pg_increment"]) != int(nuked["pg_inc"]):
        raise ValueError(f"call {call_index}: pg_increment alias mismatch")
    if int(rust["pg_output"]) != int(nuked["pg_out"]):
        raise ValueError(f"call {call_index}: pg_output alias mismatch")

    compare_output_pipeline_row(call_index, nuked, rust)
    compare_measured_adapter_frame(call_index, nuked, rust, previous_nuked)


def main() -> int:
    """Compare the Rust PG/EG/AM/operator state against Nuked on every call."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--nuked-trace", required=True, type=Path)
    parser.add_argument("--rust-trace", required=True, type=Path)
    args = parser.parse_args()

    nuked_rows = load_jsonl(args.nuked_trace, "Nuked")
    rust_rows = load_jsonl(args.rust_trace, "Rust")
    if nuked_rows[0].get("schema") != "ym2413-nuked-opll-trace-v1":
        raise ValueError("Nuked input must be a full ym2413-nuked-opll-trace-v1 trace")
    nuked_samples = nuked_rows[1:]
    if len(nuked_samples) != len(rust_rows):
        raise ValueError(
            f"trace length mismatch: Nuked={len(nuked_samples)} Rust={len(rust_rows)}"
        )
    if len(nuked_samples) != 147456:
        raise ValueError(f"expected 147456 calls, got {len(nuked_samples)}")

    cycles = {int(row["cycle_before"]) for row in nuked_samples}
    if cycles != set(range(18)):
        raise ValueError("Nuked trace does not cover all eighteen serial cycles")

    # The full arithmetic differential uses the mutable user patch (code 0)
    # on every channel. Preset-ROM candidates are checked by the separate
    # candidate-aware latch characterization so their data uncertainty is not
    # mistaken for a PG/EG/operator timing regression.
    observed_instruments = {int(row["c_instr"]) for row in nuked_samples}
    if observed_instruments != {0}:
        raise ValueError(
            f"full integrated differential must use only user patch code 0, "
            f"observed {sorted(observed_instruments)}"
        )

    # The fixture changes channel FNUM/BLOCK while playback is active. Assert
    # that the changed values reached Nuked's retained parameter latches.
    observed_fnums = {int(row["c_fnum"]) for row in nuked_samples}
    required_fnums = {0x155, 0x0A3, 0x1C1}
    if not required_fnums.issubset(observed_fnums):
        missing = sorted(required_fnums - observed_fnums)
        raise ValueError(f"live FNUM updates did not reach serial latches: {missing}")

    observed_blocks = {int(row["c_block"]) for row in nuked_samples}
    required_blocks = {2, 5, 6}
    if not required_blocks.issubset(observed_blocks):
        missing = sorted(required_blocks - observed_blocks)
        raise ValueError(f"live BLOCK updates did not reach serial latches: {missing}")

    previous_nuked = None
    for index, (nuked, rust) in enumerate(
        zip(nuked_samples, rust_rows, strict=True)
    ):
        if int(nuked["call_index"]) != index or int(rust["call_index"]) != index:
            raise ValueError(f"call sequence discontinuity at row {index}")
        compare_row(index, nuked, rust, previous_nuked)
        previous_nuked = nuked

    # The fixed all-channel vector deliberately crosses bit 19 in the
    # Nuked uint32_t PG ring. Keep this assertion so a 19-bit mask cannot
    # silently return while the comparator is otherwise fed low phases.
    if not any(
        any(int(value) > 0x7FFFF for value in row["pg_phase"])
        for row in nuked_samples
    ):
        raise ValueError("Nuked vector did not exercise full-width PG phase state")
    if not any(
        any(int(value) > 0x7FFFF for value in row["pg_phase"])
        for row in rust_rows
    ):
        raise ValueError("Rust PG ring did not preserve phase bits above bit 18")

    print(
        json.dumps(
            {
                "status": "PASS",
                "compared_calls": len(nuked_samples),
                "serial_cycles": sorted(cycles),
                "observed_instrument_codes": sorted(observed_instruments),
                "observed_live_fnums": sorted(required_fnums),
                "observed_live_blocks": sorted(required_blocks),
                "compared_scalar_fields": list(SCALAR_FIELDS),
                "compared_array_fields": list(ARRAY_FIELDS),
                "derived_checks": ["c_ksl_attenuation"],
                "compared_output_pipeline_fields": [
                    "previous ch_out before/after",
                    "rm_enable before/after",
                    "MO/RO output pins after channel stage",
                    "MO/RO output buffer one-call delayed",
                    "serial output cursor and validity warm-up",
                ],
                "output_pin_samples_compared": len(nuked_samples),
                "output_buffer_samples_compared": len(nuked_samples),
                "measured_adapter_frames_compared": len(nuked_samples) - 1,
                "measured_mo_levels_compared": sum(
                    1
                    for row in nuked_samples[:-1]
                    if (int(row["cycle_before"]) + 3) % 18 in MEASURED_MO_ACTIVE_SLOTS
                    and int(row["call_index"]) > 0
                ),
                "measured_adapter_checks": [
                    "previous ch_out latch converted to signed measured-domain code",
                    "negative-zero preservation",
                    "one-call output-buffer delay",
                    "MO carrier-slot mapping",
                    "RO remains inactive in melodic fixture",
                ],
                "scope": "integrated Rust PG/EG/AM/operator, digital MO/RO pipeline and measured-domain MO adapter against Nuked",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"YM2413 integrated FM-3 differential failed: {error}", file=sys.stderr)
        raise SystemExit(2)
