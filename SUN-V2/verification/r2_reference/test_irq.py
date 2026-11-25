"""R2-T10 IRQ/NMI sampling and acknowledge regressions."""
import unittest
from dataclasses import replace
from fractions import Fraction

from irq_reference import (
    AckEffect, AckProfile, IrqCode, IrqState, Line, LineTransition,
    begin_acknowledge, commit_ack_device_effect, complete_acknowledge,
    execute_ei, publish_transition, retire_instruction, sample_ack_vector,
    sample_instruction_end, sample_int_boundary, snapshot,
)


class R2T10InterruptTests(unittest.TestCase):
    def test_int_transition_before_boundary_is_observed(self):
        state = publish_transition(IrqState(iff1=True), LineTransition("int-high", Line.INT, Fraction(4), True)).state
        result = sample_int_boundary(state, Fraction(5))
        self.assertEqual(result.value, "INT")
        self.assertTrue(result.state.int_level)

    def test_int_transition_after_boundary_is_not_applied_early(self):
        state = publish_transition(IrqState(iff1=True), LineTransition("int-high", Line.INT, Fraction(6), True)).state
        result = sample_int_boundary(state, Fraction(5))
        self.assertIsNone(result.value)
        self.assertFalse(result.state.int_level)
        self.assertEqual(result.state.applied_transitions, ())

    def test_nmi_falling_edge_is_recognized_at_instruction_end(self):
        state = publish_transition(IrqState(), LineTransition("nmi-fall", Line.NMI, Fraction(3), False)).state
        result = sample_instruction_end(state, Fraction(4))
        self.assertEqual(result.value, "NMI")
        self.assertFalse(result.state.nmi_edge_pending)

    def test_nmi_rising_edge_does_not_create_pending_edge(self):
        state = publish_transition(IrqState(nmi_level=False), LineTransition("nmi-rise", Line.NMI, Fraction(3), True)).state
        result = sample_instruction_end(state, Fraction(4))
        self.assertIsNone(result.value)

    def test_nmi_has_priority_over_simultaneously_asserted_int(self):
        state = IrqState(iff1=True)
        state = publish_transition(state, LineTransition("int", Line.INT, Fraction(3), True)).state
        state = publish_transition(state, LineTransition("nmi", Line.NMI, Fraction(3), False)).state
        result = sample_instruction_end(state, Fraction(4))
        self.assertEqual(result.value, "NMI")

    def test_ei_delay_blocks_int_until_one_following_instruction_retires(self):
        state = execute_ei(IrqState()).state
        state = publish_transition(state, LineTransition("int", Line.INT, Fraction(1), True)).state
        blocked = sample_int_boundary(state, Fraction(1))
        self.assertIsNone(blocked.value)
        retired = retire_instruction(blocked.state).state
        accepted = sample_int_boundary(retired, Fraction(2))
        self.assertEqual(accepted.value, "INT")

    def test_iff_disabled_prevents_int_acceptance(self):
        state = publish_transition(IrqState(iff1=False), LineTransition("int", Line.INT, Fraction(1), True)).state
        self.assertIsNone(sample_int_boundary(state, Fraction(1)).value)

    def test_ack_vector_is_sampled_once_and_retry_returns_original(self):
        state = begin_acknowledge(IrqState(accepted="INT")).state
        first = sample_ack_vector(state, "0x38")
        retry = sample_ack_vector(first.state, "0x00")
        self.assertEqual(retry.value, "0x38")
        self.assertEqual(retry.state, first.state)

    def test_none_profile_has_no_device_commit_and_snapshot_is_supported(self):
        profile = AckProfile(AckEffect.NONE)
        state = begin_acknowledge(IrqState(accepted="INT")).state
        sampled = sample_ack_vector(state, "0x38")
        done = complete_acknowledge(sampled.state, profile)
        self.assertEqual(done.code, IrqCode.OK)
        self.assertFalse(done.state.ack_active)
        self.assertEqual(snapshot(done.state, profile).code, IrqCode.OK)

    def test_known_device_commit_has_separate_boundary_and_snapshot(self):
        profile = AckProfile(AckEffect.COMMIT, Fraction(8))
        state = begin_acknowledge(IrqState(accepted="INT")).state
        sampled = sample_ack_vector(state, "0x38")
        early = commit_ack_device_effect(sampled.state, profile, Fraction(7))
        self.assertEqual(early.code, IrqCode.BLOCKED)
        self.assertEqual(snapshot(early.state, profile).code, IrqCode.OK)
        committed = commit_ack_device_effect(early.state, profile, Fraction(8))
        self.assertTrue(committed.state.device_effect_committed)
        self.assertEqual(complete_acknowledge(committed.state, profile).code, IrqCode.OK)

    def test_unknown_device_effect_returns_unsupported_snapshot_without_mutation(self):
        profile = AckProfile(AckEffect.UNKNOWN)
        state = begin_acknowledge(IrqState(accepted="INT")).state
        result = snapshot(state, profile)
        self.assertEqual(result.code, IrqCode.UNSUPPORTED_SNAPSHOT)
        self.assertIsNone(result.snapshot)
        self.assertEqual(result.state, state)

    def test_unknown_commit_boundary_is_unsupported_during_ack_window(self):
        profile = AckProfile(AckEffect.COMMIT, None)
        state = begin_acknowledge(IrqState(accepted="INT")).state
        result = snapshot(state, profile)
        self.assertEqual(result.code, IrqCode.UNSUPPORTED_SNAPSHOT)
        self.assertEqual(result.state, state)

    def test_unknown_profile_before_ack_window_does_not_block_unrelated_snapshot(self):
        result = snapshot(IrqState(), AckProfile(AckEffect.UNKNOWN))
        self.assertEqual(result.code, IrqCode.OK)
        self.assertEqual(result.snapshot, IrqState())

    def test_unknown_device_effect_cannot_be_claimed_committed(self):
        state = begin_acknowledge(IrqState(accepted="INT")).state
        result = commit_ack_device_effect(state, AckProfile(AckEffect.UNKNOWN), Fraction(4))
        self.assertEqual(result.code, IrqCode.INDETERMINATE)
        self.assertEqual(result.state, state)

    def test_conflicting_same_instant_line_transitions_remain_indeterminate(self):
        state = IrqState(iff1=True)
        state = publish_transition(state, LineTransition("int-up", Line.INT, Fraction(2), True)).state
        state = publish_transition(state, LineTransition("int-down", Line.INT, Fraction(2), False)).state
        result = sample_int_boundary(state, Fraction(2))
        self.assertEqual(result.code, IrqCode.INDETERMINATE)
        self.assertEqual(result.state, state)

    def test_transition_behind_sampled_frontier_is_late_without_mutation(self):
        sampled = sample_int_boundary(IrqState(), Fraction(5)).state
        late = publish_transition(sampled, LineTransition("late", Line.INT, Fraction(4), True))
        self.assertEqual(late.code, IrqCode.LATE_INPUT)
        self.assertEqual(late.state, sampled)

    def test_transition_at_sampled_instant_is_late_without_mutation(self):
        sampled = sample_int_boundary(IrqState(), Fraction(5)).state
        late = publish_transition(sampled, LineTransition("same-instant-late", Line.INT, Fraction(5), True))
        self.assertEqual(late.code, IrqCode.LATE_INPUT)
        self.assertEqual(late.state, sampled)


if __name__ == "__main__":
    unittest.main(verbosity=2)
