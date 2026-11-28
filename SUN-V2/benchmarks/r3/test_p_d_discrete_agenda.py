"""Tests for the disposable P-D central discrete agenda."""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaEvent,
    AgendaState,
    EventStatus,
    PairRelation,
    cancel,
    claim_next,
    complete,
    publish,
)


class DiscreteAgendaTests(unittest.TestCase):
    def test_heap_selects_earliest_temporal_event_not_insertion_order(self):
        state = AgendaState()
        state = publish(
            state, identity="later", generation=0, owner="vdp",
            at_common=Fraction(20), effect_id="e-later",
        ).state
        state = publish(
            state, identity="earlier", generation=0, owner="cpu",
            at_common=Fraction(10), effect_id="e-earlier",
        ).state
        result = claim_next(state, Fraction(20))
        self.assertEqual(result.code, AgendaCode.CLAIMED)
        self.assertEqual(result.event.identity, "earlier")
        self.assertEqual(result.event.at_common, Fraction(10))

    def test_target_before_next_event_blocks_without_mutation(self):
        state = publish(
            AgendaState(), identity="event", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="effect",
        ).state
        result = claim_next(state, Fraction(4))
        self.assertEqual(result.code, AgendaCode.BLOCKED)
        self.assertIs(result.state, state)

    def test_unknown_same_instant_pair_is_indeterminate_without_mutation(self):
        state = AgendaState()
        state = publish(
            state, identity="a", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="ea",
        ).state
        state = publish(
            state, identity="b", generation=0, owner="vdp",
            at_common=Fraction(5), effect_id="eb",
        ).state
        result = claim_next(state, Fraction(5))
        self.assertEqual(result.code, AgendaCode.INDETERMINATE)
        self.assertIs(result.state, state)

    def test_commutative_tie_uses_stable_software_order(self):
        state = AgendaState()
        state = publish(
            state, identity="b", generation=0, owner="vdp",
            at_common=Fraction(5), effect_id="eb",
        ).state
        state = publish(
            state, identity="a", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="ea",
        ).state
        state = AgendaState(
            state.heap, state.events,
            (AgendaConstraint("a", "b", PairRelation.COMMUTATIVE),),
        )
        result = claim_next(state, Fraction(5))
        self.assertEqual(result.code, AgendaCode.CLAIMED)
        self.assertEqual(result.event.identity, "a")

    def test_causal_pair_claims_predecessor_first(self):
        state = AgendaState()
        for identity, owner in (("a", "cpu"), ("b", "vdp")):
            state = publish(
                state, identity=identity, generation=0, owner=owner,
                at_common=Fraction(5), effect_id="e-" + identity,
            ).state
        state = AgendaState(
            state.heap, state.events,
            (AgendaConstraint("a", "b", PairRelation.FIRST_BEFORE_SECOND),),
        )
        first = claim_next(state, Fraction(5))
        self.assertEqual(first.event.identity, "a")
        second = claim_next(first.state, Fraction(5))
        self.assertEqual(second.event.identity, "b")

    def test_cancel_removes_event_from_live_heap(self):
        state = publish(
            AgendaState(), identity="event", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="effect",
        ).state
        result = cancel(state, "event", 0)
        self.assertEqual(result.code, AgendaCode.CANCELLED)
        self.assertEqual(result.state.heap, ())
        blocked = claim_next(result.state, Fraction(5))
        self.assertEqual(blocked.code, AgendaCode.BLOCKED)

    def test_new_generation_supersedes_old_scheduled_generation(self):
        state = publish(
            AgendaState(), identity="timer", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="old",
        ).state
        result = publish(
            state, identity="timer", generation=1, owner="cpu",
            at_common=Fraction(8), effect_id="new",
        )
        self.assertEqual(result.code, AgendaCode.ACCEPTED)
        old = next(item for item in result.state.events if item.generation == 0)
        self.assertEqual(old.status, EventStatus.SUPERSEDED)
        claimed = claim_next(result.state, Fraction(8))
        self.assertEqual(claimed.event.generation, 1)

    def test_claimed_event_cannot_be_cancelled_or_replaced(self):
        state = publish(
            AgendaState(), identity="event", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="effect",
        ).state
        claimed = claim_next(state, Fraction(5))
        cancelled = cancel(claimed.state, "event", 0)
        self.assertEqual(cancelled.code, AgendaCode.LATE_INPUT)
        replacement = publish(
            claimed.state, identity="event", generation=1, owner="cpu",
            at_common=Fraction(6), effect_id="next",
        )
        self.assertEqual(replacement.code, AgendaCode.BLOCKED)
        self.assertIs(replacement.state, claimed.state)

    def test_completion_is_exactly_once(self):
        state = publish(
            AgendaState(), identity="event", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="effect",
        ).state
        claimed = claim_next(state, Fraction(5))
        done = complete(claimed.state, "event", 0)
        self.assertEqual(done.code, AgendaCode.COMPLETED)
        retry = complete(done.state, "event", 0)
        self.assertEqual(retry.code, AgendaCode.BLOCKED)
        self.assertIs(retry.state, done.state)

    def test_capacity_exhaustion_is_atomic(self):
        state = AgendaState(max_events=1)
        state = publish(
            state, identity="a", generation=0, owner="cpu",
            at_common=Fraction(1), effect_id="a",
        ).state
        result = publish(
            state, identity="b", generation=0, owner="vdp",
            at_common=Fraction(2), effect_id="b",
        )
        self.assertEqual(result.code, AgendaCode.RESOURCE_EXHAUSTED)
        self.assertIs(result.state, state)

    def test_float_coordinate_is_rejected(self):
        result = publish(
            AgendaState(), identity="event", generation=0, owner="cpu",
            at_common=1.5, effect_id="effect",
        )
        self.assertEqual(result.code, AgendaCode.INVALID_INPUT)
        self.assertEqual(result.state.events, ())


if __name__ == "__main__":
    unittest.main(verbosity=2)
