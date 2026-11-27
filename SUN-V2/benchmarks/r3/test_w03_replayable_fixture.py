"""Replayable W03 fixture for observation-directed causal synchronization.

One observation explicitly includes a cross-device dependency; a second
observation targets a participant whose dependency is absent and must block
atomically. Synthetic P-A/R2 model evidence only; no physical SMS phase/order.
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
    """Restore exact rational coordinates from the corpus representation."""
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
    """Execute one synchronization only through the selected independent model."""
    if name != "synchronize":
        raise ValueError(f"unsupported W03 operation: {name}")
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


class W03ReplayableObservationDirectedSyncTests(unittest.TestCase):
    def test_w03_declared_dependency_synchronizes_and_missing_dependency_blocks(self):
        initial = ACausalState((
            ADevice(
                "cpu",
                Fraction(0),
                (ATransition("cpu-read", Fraction(5), ("vdp-status",)),),
            ),
            ADevice(
                "vdp",
                Fraction(0),
                (ATransition("vdp-status", Fraction(4)),),
            ),
            ADevice(
                "psg",
                Fraction(0),
                (ATransition("psg-read", Fraction(5), ("missing-status",)),),
            ),
        ))
        initial_state = _state_data(initial)
        operations = (
            {
                "sequence": 0,
                "name": "synchronize",
                "args": {
                    "target_device": "cpu",
                    "at_common": Fraction(5),
                    "observes": ["vdp"],
                    "alters": [],
                },
            },
            {
                "sequence": 1,
                "name": "synchronize",
                "args": {
                    "target_device": "psg",
                    "at_common": Fraction(5),
                    "observes": [],
                    "alters": [],
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W03_OBSERVATION_DIRECTED_CROSS_DEVICE_DEPENDENCY_AND_MISSING_BLOCK",
            provenance={
                "gate": "R3",
                "workload": "W03",
                "classification": "OBSERVATION",
                "scenario": (
                    "The CPU observation declares VDP as a participant and "
                    "resolves vdp-status before cpu-read. A subsequent PSG "
                    "observation has an absent dependency and blocks without "
                    "partial progress or ledger mutation."
                ),
                "scope": "synthetic P-A/R2 causal synchronization comparison",
                "limitations": (
                    "No physical SMS bus ordering, VDP status sampling phase, "
                    "CPU observation point or PSG hardware timing is established."
                ),
                "contract": "R1 v0.3.18; observation-directed causal synchronization",
            },
            initial_state=initial_state,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), 2)

        synchronized = fixture["cuts"][0]["left_state"]
        self.assertEqual(synchronized["code"], ACausalCode.SYNCHRONIZED.value)
        self.assertEqual(synchronized["applied_order"], ["vdp-status", "cpu-read"])
        self.assertEqual(
            synchronized["state"]["devices"][0]["applied"], ["cpu-read"]
        )
        self.assertEqual(
            synchronized["state"]["devices"][1]["applied"], ["vdp-status"]
        )
        # A declared but unrelated participant is not advanced.
        self.assertEqual(
            synchronized["state"]["devices"][2],
            initial_state["devices"][2],
        )

        blocked = fixture["cuts"][1]["left_state"]
        self.assertEqual(blocked["code"], ACausalCode.BLOCKED.value)
        self.assertEqual(blocked["applied_order"], [])
        self.assertEqual(
            blocked["state"],
            synchronized["state"],
        )

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
