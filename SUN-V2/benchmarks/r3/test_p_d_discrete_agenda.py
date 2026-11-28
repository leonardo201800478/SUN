"""Tests for the disposable P-D central discrete agenda."""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaContext,
    AgendaEvent,
    AgendaParticipant,
    AgendaPhaseBasis,
    AgendaState,
    EventStatus,
    PairRelation,
    cancel,
    claim_next,
    close_watermark,
    complete,
    publish,
    publish_with_context,
    request_cancel,
    arbitrate_cancellations,
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

    def test_malformed_heap_entry_is_rejected_without_exception(self):
        state = AgendaState(heap=((Fraction(1),),))
        result = claim_next(state, Fraction(2))
        self.assertEqual(result.code, AgendaCode.INVALID_INPUT)
        self.assertIs(result.state, state)


    @staticmethod
    def phase_basis(basis_id: str) -> AgendaPhaseBasis:
        return AgendaPhaseBasis(
            profile_id="SMS-NTSC-SYNTHETIC",
            profile_revision="fixture-v1",
            coordinate="master-clock",
            unit="mclk",
            scale_numerator=1,
            scale_denominator=1,
            basis_id=basis_id,
            basis_revision="rev-1",
            validation_record_id="synthetic-record-" + basis_id,
            validation_record_sha256="a" * 64,
        )

    def test_w06_different_phase_bases_remain_unknown(self):
        state = AgendaState()
        state = publish(
            state, identity="phase-a", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="ea",
            phase_order=1, phase_evidence="shared-synthetic-label",
            phase_basis=self.phase_basis("basis-a"),
        ).state
        state = publish(
            state, identity="phase-b", generation=0, owner="vdp",
            at_common=Fraction(5), effect_id="eb",
            phase_order=2, phase_evidence="shared-synthetic-label",
            phase_basis=self.phase_basis("basis-b"),
        ).state
        result = claim_next(state, Fraction(5))
        self.assertEqual(result.code, AgendaCode.INDETERMINATE)
        self.assertIs(result.state, state)

    def test_w06_common_phase_basis_is_a_synthetic_positive_control(self):
        basis = self.phase_basis("shared-basis")
        state = AgendaState()
        state = publish(
            state, identity="phase-a", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="ea",
            phase_order=1, phase_evidence="capture-a", phase_basis=basis,
        ).state
        state = publish(
            state, identity="phase-b", generation=0, owner="vdp",
            at_common=Fraction(5), effect_id="eb",
            phase_order=2, phase_evidence="capture-b", phase_basis=basis,
        ).state
        result = claim_next(state, Fraction(5))
        self.assertEqual(result.code, AgendaCode.CLAIMED)
        self.assertEqual(result.event.identity, "phase-a")

    def test_w07_unknown_b_c_relation_is_not_inferred_from_microstep(self):
        state = AgendaState()
        state = publish(
            state, identity="a", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="ea", microstep=0,
        ).state
        state = publish(
            state, identity="b", generation=0, owner="vdp",
            at_common=Fraction(5), effect_id="eb", microstep=0,
        ).state
        state = AgendaState(
            state.heap, state.events,
            (AgendaConstraint("a", "b", PairRelation.COMMUTATIVE),),
        )
        first = claim_next(state, Fraction(5))
        self.assertEqual(first.event.identity, "a")
        done = complete(first.state, "a", 0)
        with_child = publish(
            done.state, identity="c", generation=0, owner="psg",
            at_common=Fraction(5), effect_id="ec", microstep=1,
        ).state
        result = claim_next(with_child, Fraction(5))
        self.assertEqual(result.code, AgendaCode.INDETERMINATE)
        self.assertIs(result.state, with_child)


    def test_watermark_close_claims_all_due_events_atomically(self):
        state = AgendaState()
        for identity, at in (("b", 5), ("a", 5), ("later", 8)):
            state = publish(
                state, identity=identity, generation=0, owner="cpu",
                at_common=Fraction(at), effect_id="e-" + identity,
            ).state
        state = AgendaState(
            state.heap,
            state.events,
            (AgendaConstraint("a", "b", PairRelation.COMMUTATIVE),),
            state.max_events,
        )
        result = close_watermark(state, Fraction(6))
        self.assertEqual(result.code, AgendaCode.ACCEPTED)
        self.assertEqual(result.claimed, (("a", 0), ("b", 0)))
        self.assertEqual(result.state.watermark_common, Fraction(6))
        self.assertEqual(
            next(item for item in result.state.events if item.identity == "later").status,
            EventStatus.SCHEDULED,
        )

    def test_watermark_close_unknown_tie_is_atomic(self):
        state = AgendaState()
        for identity in ("a", "b"):
            state = publish(
                state, identity=identity, generation=0, owner="cpu",
                at_common=Fraction(5), effect_id="e-" + identity,
            ).state
        result = close_watermark(state, Fraction(5))
        self.assertEqual(result.code, AgendaCode.INDETERMINATE)
        self.assertIs(result.state, state)
        self.assertIsNone(result.state.watermark_common)
        self.assertEqual(result.claimed, ())

    def test_watermark_is_monotonic_and_rejects_retrograde_publication(self):
        state = close_watermark(AgendaState(), Fraction(5)).state
        retrograde = publish(
            state, identity="late", generation=0, owner="cpu",
            at_common=Fraction(5), effect_id="late",
        )
        self.assertEqual(retrograde.code, AgendaCode.LATE_INPUT)
        self.assertIs(retrograde.state, state)
        regression = close_watermark(state, Fraction(4))
        self.assertEqual(regression.code, AgendaCode.LATE_INPUT)
        self.assertIs(regression.state, state)


    def test_context_publication_requires_complete_progress_and_explicit_microstep(self):
        incomplete = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(20)),),
            frozenset(("cpu", "vdp")),
            None,
        )
        blocked = publish_with_context(
            AgendaState(), incomplete, identity="d", generation=0,
            owner="cpu", at_common=Fraction(5), effect_id="e",
        )
        self.assertEqual(blocked.code, AgendaCode.BLOCKED)

        context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(5), Fraction(20)),
                AgendaParticipant("vdp", Fraction(0), Fraction(20)),
            ),
            frozenset(("cpu", "vdp")),
            None,
        )
        indeterminate = publish_with_context(
            AgendaState(), context, identity="d", generation=0,
            owner="cpu", at_common=Fraction(5), effect_id="e",
        )
        self.assertEqual(indeterminate.code, AgendaCode.INDETERMINATE)
        accepted = publish_with_context(
            AgendaState(), context, identity="d", generation=0,
            owner="cpu", at_common=Fraction(5), effect_id="e", microstep=1,
        )
        self.assertEqual(accepted.code, AgendaCode.ACCEPTED)

    def test_predeadline_cancellation_blocks_watermark_until_arbitrated(self):
        context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            frozenset(("cpu", "vdp")),
            None,
        )
        state = publish_with_context(
            AgendaState(), context, identity="deadline", generation=0,
            owner="cpu", at_common=Fraction(10), effect_id="write",
        ).state
        pending = request_cancel(
            state, context, owner="cpu", identity="deadline", generation=0,
            at_common=Fraction(8),
        )
        self.assertEqual(pending.code, AgendaCode.ACCEPTED)
        self.assertEqual(
            next(item for item in pending.state.events if item.identity == "deadline").status,
            EventStatus.CANCEL_PENDING,
        )
        blocked = close_watermark(pending.state, Fraction(10), context)
        self.assertEqual(blocked.code, AgendaCode.BLOCKED)
        self.assertIs(blocked.state, pending.state)

        arbitrated = arbitrate_cancellations(pending.state)
        self.assertEqual(arbitrated.code, AgendaCode.ACCEPTED)
        self.assertEqual(
            next(item for item in arbitrated.state.events if item.identity == "deadline").status,
            EventStatus.CANCELLED,
        )
        closed = close_watermark(arbitrated.state, Fraction(10), context)
        self.assertEqual(closed.code, AgendaCode.ACCEPTED)
        self.assertEqual(closed.state.watermark_common, Fraction(10))

    def test_same_deadline_cancellation_without_phase_proof_is_unknown(self):
        context = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(20)),),
            frozenset(("cpu",)),
            None,
        )
        state = publish_with_context(
            AgendaState(), context, identity="deadline", generation=0,
            owner="cpu", at_common=Fraction(10), effect_id="write",
        ).state
        result = request_cancel(
            state, context, owner="cpu", identity="deadline", generation=0,
            at_common=Fraction(10),
        )
        self.assertEqual(result.code, AgendaCode.INDETERMINATE)
        self.assertIs(result.state, state)

    def test_same_deadline_cancellation_uses_matching_phase_basis(self):
        basis = AgendaPhaseBasis(
            "sms-ntsc", "rev-a", "common", "mclk", 1, 1,
            "logic-capture", "rev-1", "capture-001", "a" * 64,
        )
        context = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(20)),),
            frozenset(("cpu",)),
            None,
        )
        state = publish_with_context(
            AgendaState(), context, identity="deadline", generation=0,
            owner="cpu", at_common=Fraction(10), effect_id="write",
            phase_order=2, phase_evidence="deadline edge", phase_basis=basis,
        ).state
        pending = request_cancel(
            state, context, owner="cpu", identity="deadline", generation=0,
            at_common=Fraction(10), phase_order=1,
            phase_evidence="cancel sample", phase_basis=basis,
        )
        self.assertEqual(pending.code, AgendaCode.ACCEPTED)
        arbitrated = arbitrate_cancellations(pending.state)
        self.assertEqual(arbitrated.code, AgendaCode.ACCEPTED)
        self.assertEqual(
            next(item for item in arbitrated.state.events if item.identity == "deadline").status,
            EventStatus.CANCELLED,
        )

    def test_cancellation_after_deadline_or_at_admission_frontier_is_late(self):
        context = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(30)),),
            frozenset(("cpu",)),
            None,
        )
        state = publish_with_context(
            AgendaState(), context, identity="deadline", generation=0,
            owner="cpu", at_common=Fraction(10), effect_id="write",
        ).state
        at_frontier = request_cancel(
            state, context, owner="cpu", identity="deadline", generation=0,
            at_common=Fraction(0),
        )
        after_deadline = request_cancel(
            state, context, owner="cpu", identity="deadline", generation=0,
            at_common=Fraction(11),
        )
        self.assertEqual(at_frontier.code, AgendaCode.LATE_INPUT)
        self.assertEqual(after_deadline.code, AgendaCode.LATE_INPUT)
        self.assertIs(at_frontier.state, state)
        self.assertIs(after_deadline.state, state)

    def test_contextual_watermark_never_crosses_global_safe_horizon(self):
        context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(5)),
                AgendaParticipant("vdp", Fraction(0), Fraction(8)),
            ),
            frozenset(("cpu", "vdp")),
            None,
        )
        state = AgendaState()
        result = close_watermark(state, Fraction(6), context)
        self.assertEqual(result.code, AgendaCode.BLOCKED)
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
