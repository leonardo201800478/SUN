"""P-A causal synchronization differential tests against the R2 oracle."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_causal_sync import (
    ACausalCode,
    ACausalState,
    ADevice,
    ATransition,
    synchronize,
)
from causal_reference import (
    CausalState as R2CausalState,
    Device as R2Device,
    SyncCode as R2SyncCode,
    Transition as R2Transition,
    synchronize as r2_synchronize,
)


def state(*devices):
    return ACausalState(tuple(devices))


def device(name, *, progress=0, transitions=(), applied=()):
    return ADevice(name, Fraction(progress), tuple(transitions), tuple(applied))


def to_r2(state_value: ACausalState) -> R2CausalState:
    return R2CausalState(tuple(
        R2Device(
            item.name,
            item.progress_common,
            tuple(
                R2Transition(t.identity, t.at_common, t.requires, t.commutative)
                for t in item.transitions
            ),
            item.applied,
        )
        for item in state_value.devices
    ))


def canonical(state_value):
    return tuple(
        (
            item.name,
            item.progress_common,
            tuple(
                (t.identity, t.at_common, t.requires, t.commutative)
                for t in item.transitions
            ),
            item.applied,
        )
        for item in state_value.devices
    )


class PACausalSyncTests(unittest.TestCase):
    def compare(self, initial, **kwargs):
        actual = synchronize(initial, **kwargs)
        expected = r2_synchronize(to_r2(initial), **kwargs)
        self.assertEqual(actual.code.value, expected.code.value)
        self.assertEqual(actual.applied_order, expected.applied_order)
        self.assertEqual(canonical(actual.state), canonical(expected.state))
        return actual

    def test_observation_applies_only_target_transitions_at_or_before_sample(self):
        initial = state(device("vdp", transitions=(
            ATransition("line-4", Fraction(4)),
            ATransition("line-6", Fraction(6)),
        )))
        result = self.compare(
            initial, target_device="vdp", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("line-4",))
        self.assertEqual(result.state.devices[0].progress_common, Fraction(5))
        self.assertEqual(result.state.devices[0].applied, ("line-4",))
        self.assertEqual(initial.devices[0].applied, ())

    def test_write_commit_synchronizes_only_the_altered_destination(self):
        initial = state(
            device("cpu", progress=2),
            device("psg", transitions=(ATransition("write-commit", Fraction(3)),)),
            device("vdp", progress=1, transitions=(ATransition("unrelated", Fraction(2)),)),
        )
        result = self.compare(
            initial, target_device="psg", at_common=Fraction(3), alters=("psg",)
        )
        self.assertEqual(result.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(result.state.devices[1].applied, ("write-commit",))
        self.assertEqual(result.state.devices[0], initial.devices[0])
        self.assertEqual(result.state.devices[2], initial.devices[2])

    def test_declared_cross_device_dependency_is_resolved_before_observation(self):
        initial = state(
            device("cpu", transitions=(ATransition("cpu-read", Fraction(5), ("vdp-status",)),)),
            device("vdp", transitions=(ATransition("vdp-status", Fraction(4)),)),
        )
        result = self.compare(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("vdp-status", "cpu-read"))

    def test_unrelated_device_is_not_advanced(self):
        initial = state(
            device("cpu", progress=0),
            device("vdp", progress=0, transitions=(ATransition("vdp-event", Fraction(2)),)),
            device("psg", progress=0, transitions=(ATransition("psg-event", Fraction(1)),)),
        )
        result = self.compare(initial, target_device="vdp", at_common=Fraction(2))
        self.assertEqual(result.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(result.state.devices[2], initial.devices[2])

    def test_missing_dependency_blocks_without_partial_progress(self):
        initial = state(
            device("cpu", transitions=(ATransition("cpu-read", Fraction(5), ("missing",)),))
        )
        result = self.compare(initial, target_device="cpu", at_common=Fraction(5))
        self.assertEqual(result.code, ACausalCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_reciprocal_dependency_blocks_without_speculative_advance(self):
        initial = state(
            device("cpu", transitions=(ATransition("cpu-read", Fraction(5), ("vdp-write",)),)),
            device("vdp", transitions=(ATransition("vdp-write", Fraction(5), ("cpu-read",)),)),
        )
        result = self.compare(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, ACausalCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_required_participant_must_be_declared(self):
        initial = state(
            device("cpu", transitions=(ATransition("cpu-read", Fraction(5), ("vdp-status",)),)),
            device("vdp", transitions=(ATransition("vdp-status", Fraction(4)),)),
        )
        result = self.compare(initial, target_device="cpu", at_common=Fraction(5))
        self.assertEqual(result.code, ACausalCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_noncommutative_simultaneous_transitions_remain_indeterminate(self):
        initial = state(
            device("cpu", transitions=(ATransition("cpu-write", Fraction(5)),)),
            device("vdp", transitions=(ATransition("vdp-latch", Fraction(5)),)),
        )
        result = self.compare(
            initial, target_device="cpu", at_common=Fraction(5), alters=("vdp",)
        )
        self.assertEqual(result.code, ACausalCode.INDETERMINATE)
        self.assertEqual(result.state, initial)

    def test_commutative_simultaneous_transitions_have_canonical_result(self):
        cpu = device("cpu", transitions=(
            ATransition("cpu-sample", Fraction(5), commutative=True),
        ))
        vdp = device("vdp", transitions=(
            ATransition("vdp-sample", Fraction(5), commutative=True),
        ))
        left = self.compare(
            state(cpu, vdp), target_device="cpu",
            at_common=Fraction(5), observes=("vdp",),
        )
        right = self.compare(
            state(vdp, cpu), target_device="cpu",
            at_common=Fraction(5), observes=("vdp",),
        )
        self.assertEqual(left.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(left.applied_order, ("cpu-sample", "vdp-sample"))
        self.assertEqual(left.applied_order, right.applied_order)

    def test_request_behind_participant_progress_is_late_without_mutation(self):
        initial = state(device("vdp", progress=6))
        result = self.compare(initial, target_device="vdp", at_common=Fraction(5))
        self.assertEqual(result.code, ACausalCode.LATE_INPUT)
        self.assertEqual(result.state, initial)

    def test_invalid_descriptor_precedes_dependency_resolution(self):
        initial = state(device("cpu"))
        result = self.compare(initial, target_device="cpu", at_common=5.0)
        self.assertEqual(result.code, ACausalCode.INVALID_INPUT)
        self.assertEqual(result.state, initial)

    def test_same_instant_dependency_declares_order_without_physical_phase(self):
        initial = state(
            device("cpu", transitions=(
                ATransition("cpu-read", Fraction(5), ("vdp-status",)),
            )),
            device("vdp", transitions=(ATransition("vdp-status", Fraction(5)),)),
        )
        result = self.compare(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("vdp-status", "cpu-read"))

    def test_preapplied_causal_ledger_must_match_owner_progress_and_dependencies(self):
        invalid_states = (
            state(
                device("cpu", transitions=(ATransition("cpu-write", Fraction(5)),)),
                device("vdp", applied=("cpu-write",)),
            ),
            state(device(
                "cpu",
                transitions=(ATransition("cpu-write", Fraction(5)),),
                applied=("cpu-write",),
            )),
            state(device(
                "cpu", progress=6,
                transitions=(ATransition("missed-write", Fraction(5)),),
            )),
            state(device(
                "cpu",
                transitions=(
                    ATransition("dependency", Fraction(5)),
                    ATransition("dependent", Fraction(5), ("dependency",)),
                ),
                applied=("dependent",),
            )),
            state(device(
                "cpu",
                transitions=(
                    ATransition("cycle-a", Fraction(5), ("cycle-b",)),
                    ATransition("cycle-b", Fraction(5), ("cycle-a",)),
                ),
                applied=("cycle-a", "cycle-b"),
            )),
        )
        for initial in invalid_states:
            with self.subTest(initial=initial):
                result = self.compare(
                    initial,
                    target_device="cpu",
                    at_common=Fraction(10),
                    observes=("vdp",) if len(initial.devices) > 1 else (),
                )
                self.assertEqual(result.code, ACausalCode.INVALID_INPUT)
                self.assertEqual(result.state, initial)
                self.assertEqual(result.applied_order, ())


if __name__ == "__main__":
    unittest.main(verbosity=2)
