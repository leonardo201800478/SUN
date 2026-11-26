"""Declarative replay fixture for the W07 microstep-order oracle gap.

This fixture intentionally captures a P-A/R2 divergence. It is not a passing
compatibility assertion: it preserves the counterexample and its replay inputs.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from microstep_reference import (  # noqa: E402
    MicrostepCycle,
    MicrostepObligation,
    process_obligation as r2_process_obligation,
)
from p_a_microsteps import (  # noqa: E402
    AStepCycle,
    AStepObligation,
    process_obligation as p_a_process_obligation,
)
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
)


def _state_data(state):
    """Encode either model's immutable cycle into one shared neutral shape."""
    if isinstance(state, AStepCycle):
        pending = state.pending
        identity_name = "identity"
    elif isinstance(state, MicrostepCycle):
        pending = state.pending
        identity_name = "obligation_id"
    else:
        raise TypeError(f"unsupported W07 state: {type(state).__name__}")
    return {
        "at_common": state.at_common,
        "max_microsteps": state.max_microsteps,
        "next_microstep": state.next_microstep,
        "pending": [
            {
                "identity": getattr(item, identity_name),
                "owner": item.owner,
                "microstep": item.microstep,
                "commutative": item.commutative,
            }
            for item in pending
        ],
        "resolved": list(state.resolved),
    }


def _unwrap_state(data):
    """Extract the cycle payload after the first operation result."""
    if set(data) == {"state", "code"}:
        return data["state"]
    return data


def _fraction(value):
    """Restore the exact rational representation emitted by normalize()."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _p_a_from_data(data):
    return AStepCycle(
        _fraction(data["at_common"]),
        data["max_microsteps"],
        data["next_microstep"],
        tuple(
            AStepObligation(
                item["identity"], item["owner"], item["microstep"],
                item["commutative"],
            )
            for item in data["pending"]
        ),
        tuple(data["resolved"]),
    )


def _r2_from_data(data):
    return MicrostepCycle(
        _fraction(data["at_common"]),
        data["max_microsteps"],
        data["next_microstep"],
        tuple(
            MicrostepObligation(
                item["identity"], item["owner"], item["microstep"],
                item["commutative"],
            )
            for item in data["pending"]
        ),
        tuple(data["resolved"]),
    )


def _arguments(args):
    return {
        "obligation_id": args["obligation_id"],
        "resolves_obligation": args["resolves_obligation"],
        "emitted": tuple(tuple(item) for item in args["emitted"]),
    }


def _p_a_step(data, name, args):
    if name != "process_obligation":
        raise ValueError(f"unsupported W07 operation: {name}")
    result = p_a_process_obligation(
        _p_a_from_data(_unwrap_state(data)), **_arguments(args)
    )
    return {"state": _state_data(result.state), "code": result.code.value}


def _r2_step(data, name, args):
    if name != "process_obligation":
        raise ValueError(f"unsupported W07 operation: {name}")
    result = r2_process_obligation(
        _r2_from_data(_unwrap_state(data)), **_arguments(args)
    )
    return {"state": _state_data(result.state), "code": result.code.value}


class W07ReplayableOracleGapTests(unittest.TestCase):
    def test_w07_counterexample_is_fully_captured_and_replayable(self):
        initial = AStepCycle(
            Fraction(10),
            4,
            1,
            (
                AStepObligation("a", "cpu", 0, True),
                AStepObligation("b", "vdp", 0, True),
            ),
            (),
        )
        operations = (
            {
                "sequence": 0,
                "name": "process_obligation",
                "args": {
                    "obligation_id": "a",
                    "resolves_obligation": True,
                    "emitted": (("c", "psg", False),),
                },
            },
            {
                "sequence": 1,
                "name": "process_obligation",
                "args": {
                    "obligation_id": "c",
                    "resolves_obligation": True,
                    "emitted": (),
                },
            },
            {
                "sequence": 2,
                "name": "process_obligation",
                "args": {
                    "obligation_id": "b",
                    "resolves_obligation": True,
                    "emitted": (),
                },
            },
            {
                "sequence": 3,
                "name": "process_obligation",
                "args": {
                    "obligation_id": "c",
                    "resolves_obligation": True,
                    "emitted": (),
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W07_PENDING_COMMUTATIVE_SIBLING_OVERTAKE",
            provenance={
                "gate": "R3",
                "workload": "P-A",
                "classification": "OBSERVATION",
                "question": (
                    "Does the P-A earlier-microstep barrier add an ordering "
                    "constraint not present in the independent R2 model?"
                ),
                "interpretation": (
                    "Counterexample to equivalence; not proof that either "
                    "ordering is physically correct. Pairwise B/C causality "
                    "and commutativity remain unestablished."
                ),
                "source_contract": "R1 v0.3.18 sections 6.1, 16.6.2, 16.6.3",
            },
            initial_state=_state_data(initial),
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )

        self.assertFalse(fixture["all_equal"])
        self.assertEqual(fixture["first_divergence"]["sequence"], 1)
        self.assertEqual(
            fixture["first_divergence"]["left_state"]["code"], "Blocked"
        )
        self.assertEqual(
            fixture["first_divergence"]["right_state"]["code"], "Advanced"
        )
        self.assertEqual(len(fixture["cuts"]), len(operations))
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
