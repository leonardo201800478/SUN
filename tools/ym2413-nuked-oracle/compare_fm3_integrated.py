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


def compare_row(
    call_index: int,
    nuked: dict[str, object],
    rust: dict[str, object],
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

    for index, (nuked, rust) in enumerate(
        zip(nuked_samples, rust_rows, strict=True)
    ):
        if int(nuked["call_index"]) != index or int(rust["call_index"]) != index:
            raise ValueError(f"call sequence discontinuity at row {index}")
        compare_row(index, nuked, rust)

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
                "scope": "integrated Rust PG/EG/AM/OPLL_Operator pipeline against Nuked",
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
