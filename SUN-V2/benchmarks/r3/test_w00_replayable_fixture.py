"""Replayable W00 fixture for autonomous advance without obligations.

The fixture compares independent P-A and R2 abstract models using exact
rational clock ratios. It is synthetic model evidence, not an SMS timing trace.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_instruction_catchup import (  # noqa: E402
    AParticipant,
    AState,
    advance_budgeted,
)
from chronos_reference import (  # noqa: E402
    ClockDomain,
    ResultKind,
    advance as r2_advance,
    make_state as r2_make_state,
)
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)


def _fraction(value):
    """Restore exact rational values encoded by normalize()."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _state_data(state):
    """Serialize all W00 participant coordinates and global advance state."""
    return {
        "participants": [
            {
                "name": item.name,
                "progress": item.progress,
                "safe_horizon": item.safe_horizon,
                "units_per_common": item.units_per_common,
                "common_offset": item.common_offset,
            }
            for item in state.participants
        ],
        "expected_participants": (
            None if state.expected_participants is None
            else sorted(state.expected_participants)
        ),
        "watermark_common": state.watermark_common,
        "obligations": [
            {
                "identity": item.identity,
                "owner": item.owner,
                "at_common": item.at_common,
                "resolved": item.resolved,
                "commutative": item.commutative,
            }
            for item in state.obligations
        ],
    }


def _p_a_state(data):
    """Reconstruct W00 state using only P-A-owned model types."""
    return AState(
        participants=tuple(
            AParticipant(
                item["name"],
                _fraction(item["progress"]),
                _fraction(item["safe_horizon"]),
                _fraction(item["units_per_common"]),
                _fraction(item["common_offset"]),
            )
            for item in data["participants"]
        ),
        obligations=tuple(),
        expected_participants=(
            None if data["expected_participants"] is None
            else frozenset(data["expected_participants"])
        ),
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
    )


def _r2_state(data):
    """Reconstruct W00 state using independent R2 reference types."""
    domains = tuple(
        ClockDomain(
            domain_id=item["name"],
            local_to_common=_fraction(item["units_per_common"]),
            common_offset=_fraction(item["common_offset"]),
            progress_local=(
                _fraction(item["progress"]) - _fraction(item["common_offset"])
            ) / _fraction(item["units_per_common"]),
            horizon_local=(
                _fraction(item["safe_horizon"]) - _fraction(item["common_offset"])
            ) / _fraction(item["units_per_common"]),
        )
        for item in data["participants"]
    )
    expected = data["expected_participants"]
    return r2_make_state(
        profile="R3-SYNTHETIC-ABSTRACT",
        domains=domains,
        expected_participants=(
            expected if expected is not None else (item["name"] for item in data["participants"])
        ),
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
        obligations=tuple(),
    )


def _p_a_step(data, name, args):
    """Apply one declared advance through P-A only and return its full state."""
    if name != "advance":
        raise ValueError(f"unsupported W00 operation: {name}")
    result = advance_budgeted(
        _p_a_state(data),
        target=_fraction(args["target"]),
        chunk=_fraction(args["chunk"]),
    )
    return {
        "code": result.code,
        "requested": result.requested,
        "reached": result.reached,
        "state": _state_data(result.next_state),
    }


def _r2_step(data, name, args):
    """Apply one declared advance through the independent R2 oracle only."""
    if name != "advance":
        raise ValueError(f"unsupported W00 operation: {name}")
    result = r2_advance(_r2_state(data), _fraction(args["target"]))
    state = result.state
    return {
        "code": result.result.kind.value,
        "requested": result.result.requested_common,
        "reached": result.result.reached_common,
        "state": {
            "participants": [
                {
                    "name": item.domain_id,
                    "progress": item.progress_common(),
                    "safe_horizon": item.horizon_common(),
                    "units_per_common": item.local_to_common,
                    "common_offset": item.common_offset,
                }
                for item in state.domains
            ],
            "expected_participants": sorted(state.expected_participants),
            "watermark_common": state.watermark_common,
            "obligations": [],
        },
    }


class W00ReplayableAdvanceTests(unittest.TestCase):
    def test_w00_empty_advance_is_captured_and_replayable(self):
        initial = {
            "participants": [
                {
                    "name": "cpu",
                    "progress": Fraction(0),
                    "safe_horizon": Fraction(100),
                    "units_per_common": Fraction(1),
                    "common_offset": Fraction(0),
                },
                {
                    "name": "vdp",
                    "progress": Fraction(0),
                    "safe_horizon": Fraction(100),
                    "units_per_common": Fraction(3, 2),
                    "common_offset": Fraction(0),
                },
            ],
            "expected_participants": ["cpu", "vdp"],
            "watermark_common": None,
            "obligations": [],
        }
        operations = (
            {
                "sequence": 0,
                "name": "advance",
                "args": {"target": Fraction(40), "chunk": Fraction(8)},
            },
            {
                "sequence": 1,
                "name": "advance",
                "args": {"target": Fraction(80), "chunk": Fraction(8)},
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W00_EMPTY_ADVANCE_RATIONAL_CPU_VDP_CLOCKS",
            provenance={
                "gate": "R3",
                "workload": "W00",
                "classification": "OBSERVATION",
                "source": "synthetic P-A/R2 replay fixture; no SMS hardware evidence",
                "question": (
                    "Do P-A and independent R2 agree on autonomous advance "
                    "without events or obligations across rational clock domains?"
                ),
                "limits": (
                    "Abstract state-transition comparison only; does not establish "
                    "physical SMS clock ratios, frame timing, or hardware performance."
                ),
                "contract": "R1 v0.3.18; W00 no-event/no-obligation advance",
            },
            initial_state=initial,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        self.assertEqual(fixture["cuts"][-1]["left_state"]["reached"], Fraction(80))
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
