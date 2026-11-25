"""Discriminating tests for R2-T05 causal synchronization."""
from fractions import Fraction
import unittest

from causal_reference import (
    CausalState,
    Device,
    SyncCode,
    Transition,
    synchronize,
)


def state(*devices):
    return CausalState(tuple(devices))


def device(name, *, progress=0, transitions=(), applied=()):
    return Device(name, Fraction(progress), tuple(transitions), tuple(applied))


class R2T05CausalSynchronizationTests(unittest.TestCase):
    def test_observation_applies_only_target_transitions_at_or_before_sample(self):
        initial = state(device("vdp", transitions=(
            Transition("line-4", Fraction(4)),
            Transition("line-6", Fraction(6)),
        )))
        result = synchronize(initial, target_device="vdp", at_common=Fraction(5), observes=("vdp",))
        self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("line-4",))
        self.assertEqual(result.state.devices[0].progress_common, Fraction(5))
        self.assertEqual(result.state.devices[0].applied, ("line-4",))
        self.assertEqual(initial.devices[0].applied, ())

    def test_write_commit_synchronizes_only_the_altered_destination(self):
        initial = state(
            device("cpu", progress=2),
            device("psg", transitions=(Transition("write-commit", Fraction(3)),)),
            device("vdp", progress=1, transitions=(Transition("unrelated", Fraction(2)),)),
        )
        result = synchronize(initial, target_device="psg", at_common=Fraction(3), alters=("psg",))
        self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(result.state.devices[1].applied, ("write-commit",))
        self.assertEqual(result.state.devices[0], initial.devices[0])
        self.assertEqual(result.state.devices[2], initial.devices[2])

    def test_declared_cross_device_dependency_is_resolved_before_observation(self):
        initial = state(
            device("cpu", transitions=(Transition("cpu-read", Fraction(5), ("vdp-status",)),)),
            device("vdp", transitions=(Transition("vdp-status", Fraction(4)),)),
        )
        result = synchronize(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("vdp-status", "cpu-read"))

    def test_unrelated_device_is_not_advanced(self):
        initial = state(
            device("cpu", progress=0),
            device("vdp", progress=0, transitions=(Transition("vdp-event", Fraction(2)),)),
            device("psg", progress=0, transitions=(Transition("psg-event", Fraction(1)),)),
        )
        result = synchronize(initial, target_device="vdp", at_common=Fraction(2))
        self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(result.state.devices[2], initial.devices[2])

    def test_missing_dependency_blocks_without_partial_progress(self):
        initial = state(
            device("cpu", transitions=(Transition("cpu-read", Fraction(5), ("missing",)),))
        )
        result = synchronize(initial, target_device="cpu", at_common=Fraction(5))
        self.assertEqual(result.code, SyncCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_reciprocal_dependency_blocks_without_speculative_advance(self):
        initial = state(
            device("cpu", transitions=(Transition("cpu-read", Fraction(5), ("vdp-write",)),)),
            device("vdp", transitions=(Transition("vdp-write", Fraction(5), ("cpu-read",)),)),
        )
        result = synchronize(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, SyncCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_required_participant_must_be_declared(self):
        initial = state(
            device("cpu", transitions=(Transition("cpu-read", Fraction(5), ("vdp-status",)),)),
            device("vdp", transitions=(Transition("vdp-status", Fraction(4)),)),
        )
        result = synchronize(initial, target_device="cpu", at_common=Fraction(5))
        self.assertEqual(result.code, SyncCode.BLOCKED)
        self.assertEqual(result.state, initial)

    def test_noncommutative_simultaneous_transitions_remain_indeterminate(self):
        initial = state(
            device("cpu", transitions=(Transition("cpu-write", Fraction(5)),)),
            device("vdp", transitions=(Transition("vdp-latch", Fraction(5)),)),
        )
        result = synchronize(
            initial, target_device="cpu", at_common=Fraction(5), alters=("vdp",)
        )
        self.assertEqual(result.code, SyncCode.INDETERMINATE)
        self.assertEqual(result.state, initial)

    def test_commutative_simultaneous_transitions_have_canonical_result(self):
        def run(order):
            return synchronize(
                state(*order), target_device="cpu", at_common=Fraction(5), observes=("vdp",)
            )
        cpu = device("cpu", transitions=(Transition("cpu-sample", Fraction(5), commutative=True),))
        vdp = device("vdp", transitions=(Transition("vdp-sample", Fraction(5), commutative=True),))
        left = run((cpu, vdp))
        right = run((vdp, cpu))
        self.assertEqual(left.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(left.applied_order, ("cpu-sample", "vdp-sample"))
        self.assertEqual(left.applied_order, right.applied_order)

    def test_request_behind_participant_progress_is_late_without_mutation(self):
        initial = state(device("vdp", progress=6))
        result = synchronize(initial, target_device="vdp", at_common=Fraction(5))
        self.assertEqual(result.code, SyncCode.LATE_INPUT)
        self.assertEqual(result.state, initial)

    def test_invalid_descriptor_precedes_dependency_resolution(self):
        initial = state(device("cpu"))
        result = synchronize(initial, target_device="cpu", at_common=5.0)
        self.assertEqual(result.code, SyncCode.INVALID_INPUT)
        self.assertEqual(result.state, initial)


    def test_same_instant_dependency_declares_order_without_physical_phase(self):
        initial = state(
            device("cpu", transitions=(
                Transition("cpu-read", Fraction(5), ("vdp-status",)),
            )),
            device("vdp", transitions=(Transition("vdp-status", Fraction(5)),)),
        )
        result = synchronize(
            initial, target_device="cpu", at_common=Fraction(5), observes=("vdp",)
        )
        self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
        self.assertEqual(result.applied_order, ("vdp-status", "cpu-read"))


    def test_preapplied_causal_ledger_must_match_owner_progress_and_dependencies(self):
        invalid_states = (
            state(
                device("cpu", transitions=(Transition("cpu-write", Fraction(5)),)),
                device("vdp", applied=("cpu-write",)),
            ),
            state(device(
                "cpu",
                transitions=(Transition("cpu-write", Fraction(5)),),
                applied=("cpu-write",),
            )),
            state(device(
                "cpu",
                progress=6,
                transitions=(Transition("missed-write", Fraction(5)),),
            )),
            state(device(
                "cpu",
                transitions=(
                    Transition("dependency", Fraction(5)),
                    Transition("dependent", Fraction(5), ("dependency",)),
                ),
                applied=("dependent",),
            )),
        )
        for initial in invalid_states:
            with self.subTest(initial=initial):
                result = synchronize(
                    initial,
                    target_device="cpu",
                    at_common=Fraction(10),
                    observes=("vdp",) if len(initial.devices) > 1 else (),
                )
                self.assertEqual(result.code, SyncCode.INVALID_INPUT)
                self.assertEqual(result.state, initial)
                self.assertEqual(result.applied_order, ())


if __name__ == "__main__":
    unittest.main(verbosity=2)
