"""Replayable W04 fixture for repeated observation cuts.

Four independent synchronize calls capture a complete state after each
observation. Synthetic P-A/R2 model evidence only; no physical SMS cadence.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_causal_sync import (  # noqa: E402
    ACausalCode,
    ACausalState,
    ADevice,
    ATransition,
    synchronize,
)
from causal_reference import (  # noqa: E402
    CausalState as R2CausalState,
    Device as R2Device,
    Transition as R2Transition,
    synchronize as r2_synchronize,
)
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)


def _fraction(value):
    """Restore exact rational coordinates from the canonical corpus form."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _state_data(state):
    """Serialize every participant, transition descriptor and applied ledger."""
    return {
        "devices": [
            {
                "name": device.name,
                "progress_common": device.progress_common,
                "transitions": [
                    {
                        "identity": transition.identity,
                        "at_common": transition.at_common,
                        "requires": list(transition.requires),
                        "commutative": transition.commutative,
                    }
                    for transition in device.transitions
                ],
                "applied": list(device.applied),
            }
            for device in state.devices
        ]
    }


def _models(data, *, side):
    """Rebuild P-A or R2-owned causal types without sharing model objects."""
    if "state" in data:
        data = data["state"]
    devices = []
    for item in data["devices"]:
        transitions = tuple(
            (
                ATransition(
                    transition["identity"],
                    _fraction(transition["at_common"]),
                    tuple(transition["requires"]),
                    transition["commutative"],
                )
                if side == "left"
                else R2Transition(
                    transition["identity"],
                    _fraction(transition["at_common"]),
                    tuple(transition["requires"]),
                    transition["commutative"],
                )
            )
            for transition in item["transitions"]
        )
        device_type = ADevice if side == "left" else R2Device
        devices.append(device_type(
            item["name"],
            _fraction(item["progress_common"]),
            transitions,
            tuple(item["applied"]),
        ))
    state_type = ACausalState if side == "left" else R2CausalState
    return state_type(tuple(devices))


def _step(data, name, args, *, side):
    """Execute one observation through only the selected independent model."""
    if name != "synchronize":
        raise ValueError(f"unsupported W04 operation: {name}")
    state = _models(data, side=side)
    transition = synchronize if side == "left" else r2_synchronize
    result = transition(
        state,
        target_device=args["target_device"],
        at_common=_fraction(args["at_common"]),
        observes=tuple(args["observes"]),
        alters=tuple(args["alters"]),
    )
    return {
        "code": result.code.value,
        "applied_order": list(result.applied_order),
        "state": _state_data(result.state),
    }


def _p_a_step(data, name, args):
    """P-A adapter for the declarative replay harness."""
    return _step(data, name, args, side="left")


def _r2_step(data, name, args):
    """Independent R2 adapter for the declarative replay harness."""
    return _step(data, name, args, side="right")


class W04ReplayableRepeatedObservationTests(unittest.TestCase):
    def test_w04_repeated_observations_preserve_each_complete_state_cut(self):
        initial = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice(
                "vdp",
                Fraction(0),
                tuple(
                    ATransition(f"vdp-{tick}", Fraction(tick), commutative=True)
                    for tick in (1, 2, 3, 4)
                ),
            ),
            ADevice(
                "psg",
                Fraction(0),
                tuple(
                    ATransition(f"psg-{tick}", Fraction(tick), commutative=True)
                    for tick in (1, 2, 3, 4)
                ),
            ),
        ))
        operations = tuple(
            {
                "sequence": tick - 1,
                "name": "synchronize",
                "args": {
                    "target_device": "cpu",
                    "at_common": Fraction(tick),
                    "observes": ["vdp", "psg"],
                    "alters": [],
                },
            }
            for tick in (1, 2, 3, 4)
        )
        fixture = capture_differential_fixture(
            fixture_id="W04_REPEATED_OBSERVATIONS_COMPLETE_STATE_AT_EACH_CUT",
            provenance={
                "gate": "R3",
                "workload": "W04",
                "classification": "OBSERVATION",
                "scenario": (
                    "Four separate CPU observation calls include VDP and PSG. "
                    "Each call resolves only the transitions due at its exact "
                    "rational cut and records the complete participant state."
                ),
                "scope": "synthetic P-A/R2 repeated-observation comparison",
                "limitations": (
                    "The integer cut labels are synthetic coordinates, not SMS "
                    "clock measurements, observation cadence or physical phases."
                ),
                "contract": "R1 v0.3.18; observation-directed causal synchronization",
            },
            initial_state=_state_data(initial),
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["operations"]), 4)
        self.assertEqual(len(fixture["cuts"]), 4)

        expected_orders = [
            [f"psg-{tick}", f"vdp-{tick}"]
            for tick in (1, 2, 3, 4)
        ]
        for tick, cut in enumerate(fixture["cuts"], start=1):
            observed = cut["left_state"]
            self.assertEqual(observed["code"], ACausalCode.SYNCHRONIZED.value)
            self.assertEqual(observed["applied_order"], expected_orders[tick - 1])
            devices = observed["state"]["devices"]
            self.assertEqual([item["name"] for item in devices], ["cpu", "vdp", "psg"])
            self.assertEqual(
                [item["progress_common"] for item in devices],
                [Fraction(tick), Fraction(tick), Fraction(tick)],
            )
            self.assertEqual(devices[0]["applied"], [])
            self.assertEqual(
                devices[1]["applied"],
                [f"vdp-{index}" for index in range(1, tick + 1)],
            )
            self.assertEqual(
                devices[2]["applied"],
                [f"psg-{index}" for index in range(1, tick + 1)],
            )
            self.assertTrue(cut["equal"])

        p_a_cuts = replay_fixture_side(fixture, step=_p_a_step, side="left")
        r2_cuts = replay_fixture_side(fixture, step=_r2_step, side="right")
        self.assertEqual(len(p_a_cuts), 4)
        self.assertEqual(len(r2_cuts), 4)


if __name__ == "__main__":
    unittest.main(verbosity=2)
