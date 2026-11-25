"""R2-T12 deterministic input admission and replay regressions."""
import unittest
from dataclasses import replace
from fractions import Fraction

from replay_reference import (
    ExternalInput, InputContract, Participant, ReplayCode, ReplayState,
    accept_batch, accept_input, advance_to, recalculate_horizon, replay,
)


def initial(*, progress=1, horizon=10, watermark=2, pending=()):
    return ReplayState(
        "SMS-NTSC",
        (
            Participant("cpu", Fraction(progress), Fraction(horizon)),
            Participant("vdp", Fraction(progress + 1), Fraction(horizon)),
        ),
        (InputContract("port-write", ("cpu", "vdp")),),
        Fraction(watermark),
        pending_obligations=tuple(pending),
    )


def event(identity="e1", time=5, *, payload="x", order=None, commutative=False, participants=("cpu", "vdp")):
    return ExternalInput(identity, "port-write", "cpu", Fraction(time), payload, participants, order, commutative)


class R2T12ReplayTests(unittest.TestCase):
    def test_admission_frontier_uses_watermark_and_all_participants(self):
        result = accept_input(initial(progress=3, watermark=4), event(time=5))
        self.assertEqual(result.code, ReplayCode.OK)
        self.assertEqual(result.state.accepted[0].sequence, 1)

    def test_incomplete_declared_participant_contract_blocks_without_partial_frontier(self):
        state = initial()
        result = accept_input(state, event(participants=("cpu",)))
        self.assertEqual(result.code, ReplayCode.BLOCKED)
        self.assertEqual(result.state, state)

    def test_missing_participant_progress_blocks_without_using_partial_maximum(self):
        state = initial()
        state = replace(state, participants=(state.participants[0], replace(state.participants[1], progress_common=None)))
        result = accept_input(state, event(time=5))
        self.assertEqual(result.code, ReplayCode.BLOCKED)
        self.assertEqual(result.state, state)

    def test_invalid_conversion_is_indeterminate_not_partial_late_input(self):
        state = initial(progress=9, watermark=0)
        state = replace(state, participants=(state.participants[0], replace(state.participants[1], conversion_valid=False)))
        result = accept_input(state, event(time=1))
        self.assertEqual(result.code, ReplayCode.INDETERMINATE)
        self.assertEqual(result.state, state)

    def test_input_at_or_behind_admission_frontier_is_late(self):
        state = initial(progress=3, watermark=4)
        result = accept_input(state, event(time=4))
        self.assertEqual(result.code, ReplayCode.LATE_INPUT)
        self.assertEqual(result.state, state)

    def test_identical_event_retry_is_idempotent_and_divergent_id_is_invalid(self):
        state = initial()
        descriptor = event(time=5)
        accepted = accept_input(state, descriptor)
        retry = accept_input(accepted.state, descriptor)
        divergent = accept_input(accepted.state, event(time=6))
        self.assertEqual(retry.state, accepted.state)
        self.assertEqual(divergent.code, ReplayCode.INVALID_INPUT)
        self.assertEqual(divergent.state, accepted.state)

    def test_admitted_input_invalidates_every_declared_participant_horizon(self):
        accepted = accept_input(initial(), event(time=5))
        self.assertEqual(accepted.state.recalculate_required, ("cpu", "vdp"))

    def test_advance_blocks_until_all_invalidated_horizons_are_republished(self):
        accepted = accept_input(initial(), event(time=5))
        blocked = advance_to(accepted.state, Fraction(6))
        self.assertEqual(blocked.code, ReplayCode.BLOCKED)
        self.assertEqual(blocked.state, accepted.state)

    def test_advance_resumes_after_all_participants_recalculate_horizon(self):
        state = accept_input(initial(), event(time=5)).state
        state = recalculate_horizon(state, "cpu", Fraction(9)).state
        state = recalculate_horizon(state, "vdp", Fraction(9)).state
        result = advance_to(state, Fraction(6))
        self.assertEqual(result.code, ReplayCode.OK)
        self.assertEqual(result.state.applied_ids, ("e1",))

    def test_same_frontier_advance_is_idempotent_only_without_pending_obligation(self):
        state = initial(progress=3, watermark=4)
        same = advance_to(state, Fraction(4))
        self.assertEqual(same.code, ReplayCode.OK)
        self.assertEqual(same.state, state)
        pending = initial(progress=3, watermark=4, pending=(4,))
        blocked = advance_to(pending, Fraction(4))
        self.assertEqual(blocked.code, ReplayCode.BLOCKED)

    def test_new_input_at_same_frontier_is_not_an_idempotent_advance(self):
        state = initial(progress=3, watermark=4)
        result = accept_input(state, event(time=4))
        self.assertEqual(result.code, ReplayCode.LATE_INPUT)

    def test_concurrent_noncommutative_tie_without_logical_order_is_indeterminate(self):
        state = initial()
        result = accept_batch(state, (event("a", 5, payload="A"), event("b", 5, payload="B")))
        self.assertEqual(result.code, ReplayCode.INDETERMINATE)
        self.assertEqual(result.state, state)

    def test_concurrent_inputs_use_logical_order_not_batch_arrival_order(self):
        first = event("a", 5, payload="A", order=1)
        second = event("b", 5, payload="B", order=2)
        left = accept_batch(initial(), (first, second))
        right = accept_batch(initial(), (second, first))
        self.assertEqual(left.state.accepted, right.state.accepted)
        self.assertEqual(tuple(item.event.identity for item in left.state.accepted), ("a", "b"))

    def test_commutative_tie_without_logical_order_uses_canonical_identity_order(self):
        first = event("z", 5, commutative=True)
        second = event("a", 5, commutative=True)
        result = accept_batch(initial(), (first, second))
        self.assertEqual(result.code, ReplayCode.OK)
        self.assertEqual(tuple(item.event.identity for item in result.state.accepted), ("a", "z"))

    def test_replay_from_same_initial_and_ordered_inputs_is_deterministic(self):
        state = initial()
        inputs = (event("a", 5, order=1), event("b", 6, order=2))
        left = replay(state, inputs, target=Fraction(7))
        right = replay(state, inputs, target=Fraction(7))
        self.assertEqual(left.state, right.state)
        self.assertEqual(left.state.virtual_effects, (("a", "x"), ("b", "x")))

    def test_replay_preserves_accepted_order_after_snapshot_restore(self):
        state = initial()
        accepted = accept_batch(state, (event("a", 5, order=1), event("b", 6, order=2))).state
        replayed = replay(accepted, (), target=Fraction(7))
        self.assertEqual(replayed.state.applied_ids, ("a", "b"))
        self.assertEqual(replayed.state.accepted, accepted.accepted)

    def test_same_time_noncommutative_duplicate_logical_order_is_indeterminate(self):
        state = initial()
        result = accept_batch(state, (event("a", 5, order=1), event("b", 5, order=1)))
        self.assertEqual(result.code, ReplayCode.INDETERMINATE)
        self.assertEqual(result.state, state)

    def test_advance_does_not_cross_any_unresolved_obligation(self):
        state = replace(initial(), pending_obligations=(Fraction(6),))
        blocked = advance_to(state, Fraction(7))
        self.assertEqual(blocked.code, ReplayCode.BLOCKED)
        self.assertEqual(blocked.state, state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
