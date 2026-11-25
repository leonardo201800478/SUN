"""P-A deadline lifecycle differential tests against the R2 oracle."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_instruction_catchup import AObligation, AParticipant
from p_a_deadline_lifecycle import (
    ADeadline,
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    ADeadlineStatus,
    active_deadlines,
    arbitrate_cancellations,
    cancel_deadline,
    close_watermark,
    complete_deadline,
    dispatch_deadline,
    publish_deadline,
    replace_deadline,
)
from chronos_reference import ClockDomain, Obligation, make_state
from deadline_reference import (
    Cancellation as R2Cancellation,
    Deadline as R2Deadline,
    DeadlineBook as R2DeadlineBook,
    DeadlineCode as R2DeadlineCode,
    DeadlineStatus as R2DeadlineStatus,
    active_deadlines as r2_active_deadlines,
    arbitrate_cancellations as r2_arbitrate_cancellations,
    cancel_deadline as r2_cancel_deadline,
    close_watermark as r2_close_watermark,
    complete_deadline as r2_complete_deadline,
    dispatch_deadline as r2_dispatch_deadline,
    publish_deadline as r2_publish_deadline,
    replace_deadline as r2_replace_deadline,
)


def to_r2_context(context: ADeadlineContext):
    return make_state(
        profile="R3-P-A-DEADLINE-ABSTRACT",
        domains=tuple(
            ClockDomain(
                domain_id=item.name,
                local_to_common=item.units_per_common,
                common_offset=item.common_offset,
                progress_local=(item.progress - item.common_offset) / item.units_per_common,
                horizon_local=(item.safe_horizon - item.common_offset) / item.units_per_common,
            )
            for item in context.participants
        ),
        expected_participants=context.expected_participants,
        obligations=tuple(
            Obligation(
                obligation_id=item.identity,
                owner=item.owner,
                at_common=item.at_common,
                resolved=item.resolved,
                commutative=item.commutative,
            )
            for item in context.obligations
        ),
        watermark_common=context.watermark_common,
    )


def to_r2_book(book: ADeadlineBook) -> R2DeadlineBook:
    return R2DeadlineBook(
        deadlines=tuple(
            R2Deadline(
                owner=item.owner,
                identity=item.identity,
                generation=item.generation,
                at_common=item.at_common,
                effect_id=item.effect_id,
                commutative=item.commutative,
                phase_order=item.phase_order,
                phase_evidence=item.phase_evidence,
                microstep=item.microstep,
                status=R2DeadlineStatus(item.status.value),
            )
            for item in book.deadlines
        ),
        pending_cancellations=tuple(
            R2Cancellation(
                owner=item.owner,
                identity=item.identity,
                generation=item.generation,
                at_common=item.at_common,
                microstep_level=item.microstep_level,
                phase_order=item.phase_order,
                phase_evidence=item.phase_evidence,
            )
            for item in book.pending_cancellations
        ),
        watermark_common=book.watermark_common,
        max_microsteps=book.max_microsteps,
    )


def canonical_book(book):
    deadlines = tuple(sorted(
        (
            item.owner, item.identity, item.generation, item.at_common,
            item.effect_id, item.commutative, item.phase_order,
            item.phase_evidence, item.microstep, item.status.value,
        )
        for item in book.deadlines
    ))
    cancellations = tuple(sorted(
        (
            item.owner, item.identity, item.generation, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
        )
        for item in book.pending_cancellations
    ))
    return deadlines, cancellations, book.watermark_common, book.max_microsteps


class PADeadlineLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(3, 2)),
            ),
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        self.empty = ADeadlineBook()

    def compare(self, actual, expected):
        self.assertEqual(actual.code.value, expected.code.value)
        self.assertEqual(actual.claimed, expected.claimed)
        self.assertEqual(canonical_book(actual.book), canonical_book(expected.book))

    def publish(self, book=None, context=None, **kwargs):
        book = self.empty if book is None else book
        context = self.context if context is None else context
        actual = publish_deadline(book, context, **kwargs)
        expected = r2_publish_deadline(
            to_r2_book(book), to_r2_context(context), **kwargs
        )
        self.compare(actual, expected)
        return actual

    def test_future_publication_is_owned_and_exact(self):
        result = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        self.assertEqual(result.code, ADeadlineCode.ACCEPTED)
        self.assertEqual(result.book.deadlines[0].status, ADeadlineStatus.SCHEDULED)

    def test_publication_before_owner_progress_is_late(self):
        context = ADeadlineContext(
            (AParticipant("cpu", Fraction(5), Fraction(20)),),
            frozenset({"cpu"}),
        )
        result = self.publish(
            context=context, owner="cpu", identity="irq", generation=0,
            at_common=Fraction(4), effect_id="irq-line",
        )
        self.assertEqual(result.code, ADeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, self.empty)

    def test_publication_at_owner_progress_requires_microstep(self):
        context = ADeadlineContext(
            (AParticipant("cpu", Fraction(5), Fraction(20)),),
            frozenset({"cpu"}),
        )
        result = self.publish(
            context=context, owner="cpu", identity="irq", generation=0,
            at_common=Fraction(5), effect_id="irq-line",
        )
        self.assertEqual(result.code, ADeadlineCode.INDETERMINATE)

    def test_publication_at_owner_progress_accepts_explicit_microstep(self):
        context = ADeadlineContext(
            (AParticipant("cpu", Fraction(5), Fraction(20)),),
            frozenset({"cpu"}),
        )
        result = self.publish(
            context=context, owner="cpu", identity="irq", generation=0,
            at_common=Fraction(5), effect_id="irq-line",
            microstep=True, microstep_level=2,
        )
        self.assertEqual(result.code, ADeadlineCode.ACCEPTED)
        self.assertEqual(result.book.deadlines[0].microstep, 2)

    def test_publication_at_closed_watermark_is_late(self):
        context = ADeadlineContext(
            (AParticipant("cpu", Fraction(0), Fraction(20)),),
            frozenset({"cpu"}), watermark_common=Fraction(10),
        )
        book = ADeadlineBook(watermark_common=Fraction(10))
        result = self.publish(
            book=book, context=context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(10), effect_id="irq-line",
            microstep=True,
        )
        self.assertEqual(result.code, ADeadlineCode.LATE_INPUT)

    def test_duplicate_generation_is_invalid_without_mutation(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        result = self.publish(
            book=first.book, owner="cpu", identity="irq", generation=0,
            at_common=Fraction(20), effect_id="irq-line-new",
        )
        self.assertEqual(result.code, ADeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, first.book)

    def test_cancellation_c_before_d_is_pending_until_arbitrated(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        actual = cancel_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(5),
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(self.context),
            owner="cpu", identity="irq", generation=0, at_common=Fraction(5),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.ACCEPTED)
        self.assertEqual(actual.book.deadlines[0].status, ADeadlineStatus.CANCEL_PENDING)

        blocked = close_watermark(actual.book, Fraction(10))
        blocked_ref = r2_close_watermark(to_r2_book(actual.book), Fraction(10))
        self.compare(blocked, blocked_ref)
        self.assertEqual(blocked.code, ADeadlineCode.BLOCKED)

        arbitrated = arbitrate_cancellations(actual.book)
        arbitrated_ref = r2_arbitrate_cancellations(to_r2_book(actual.book))
        self.compare(arbitrated, arbitrated_ref)
        self.assertEqual(arbitrated.book.deadlines[0].status, ADeadlineStatus.CANCELLED)

        closed = close_watermark(arbitrated.book, Fraction(10))
        closed_ref = r2_close_watermark(to_r2_book(arbitrated.book), Fraction(10))
        self.compare(closed, closed_ref)
        self.assertEqual(closed.claimed, ())

    def test_cancel_at_d_without_causal_relation_is_indeterminate(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        actual = cancel_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(10),
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(self.context),
            owner="cpu", identity="irq", generation=0, at_common=Fraction(10),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.INDETERMINATE)
        self.assertEqual(actual.book, first.book)

    def test_cancel_at_d_uses_explicit_microstep_order(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
            microstep=True, microstep_level=2,
        )
        actual = cancel_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(10), microstep_level=1,
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(self.context),
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), microstep_level=1,
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.ACCEPTED)

    def test_cancel_at_or_before_global_admission_frontier_is_late(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(5), Fraction(20)),
            ),
            frozenset({"cpu", "vdp"}),
        )
        actual = cancel_deadline(
            first.book, context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(5),
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(context),
            owner="cpu", identity="irq", generation=0, at_common=Fraction(5),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.LATE_INPUT)

    def test_cancel_after_deadline_is_late_even_before_dispatch(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        actual = cancel_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(11),
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(self.context),
            owner="cpu", identity="irq", generation=0, at_common=Fraction(11),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.LATE_INPUT)

    def test_cancel_blocks_when_declared_participant_is_missing(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        incomplete = ADeadlineContext(
            (AParticipant("cpu", Fraction(0), Fraction(20)),),
            frozenset({"cpu", "vdp"}),
        )
        actual = cancel_deadline(
            first.book, incomplete, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(5),
        )
        expected = r2_cancel_deadline(
            to_r2_book(first.book), to_r2_context(incomplete),
            owner="cpu", identity="irq", generation=0, at_common=Fraction(5),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.BLOCKED)

    def test_watermark_blocks_until_admitted_cancel_is_arbitrated(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        pending = cancel_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, at_common=Fraction(5),
        )
        before_d = close_watermark(pending.book, Fraction(9))
        before_d_ref = r2_close_watermark(to_r2_book(pending.book), Fraction(9))
        self.compare(before_d, before_d_ref)
        self.assertEqual(before_d.code, ADeadlineCode.ACCEPTED)
        at_d = close_watermark(before_d.book, Fraction(10))
        at_d_ref = r2_close_watermark(to_r2_book(before_d.book), Fraction(10))
        self.compare(at_d, at_d_ref)
        self.assertEqual(at_d.code, ADeadlineCode.BLOCKED)

    def test_noncommutative_unknown_tie_does_not_close_watermark(self):
        first = self.publish(
            owner="cpu", identity="irq-a", generation=0,
            at_common=Fraction(10), effect_id="a",
        )
        second = self.publish(
            book=first.book, owner="vdp", identity="irq-b", generation=0,
            at_common=Fraction(10), effect_id="b",
        )
        actual = close_watermark(second.book, Fraction(10))
        expected = r2_close_watermark(to_r2_book(second.book), Fraction(10))
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.INDETERMINATE)
        self.assertIsNone(actual.book.watermark_common)

    def test_commutative_tie_claim_order_is_identity_stable(self):
        first = self.publish(
            owner="cpu", identity="irq-z", generation=0,
            at_common=Fraction(10), effect_id="z", commutative=True,
        )
        second = self.publish(
            book=first.book, owner="vdp", identity="irq-a", generation=0,
            at_common=Fraction(10), effect_id="a", commutative=True,
        )
        actual = close_watermark(second.book, Fraction(10))
        expected = r2_close_watermark(to_r2_book(second.book), Fraction(10))
        self.compare(actual, expected)
        self.assertEqual(actual.claimed, (("irq-a", 0), ("irq-z", 0)))

    def test_dispatch_is_exactly_once_and_completion_requires_dispatch(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="irq-line",
        )
        claimed = close_watermark(first.book, Fraction(10))
        claimed_ref = r2_close_watermark(to_r2_book(first.book), Fraction(10))
        self.compare(claimed, claimed_ref)

        dispatched = dispatch_deadline(claimed.book, identity="irq", generation=0)
        dispatched_ref = r2_dispatch_deadline(to_r2_book(claimed.book), identity="irq", generation=0)
        self.compare(dispatched, dispatched_ref)
        repeated = dispatch_deadline(dispatched.book, identity="irq", generation=0)
        repeated_ref = r2_dispatch_deadline(to_r2_book(dispatched.book), identity="irq", generation=0)
        self.compare(repeated, repeated_ref)
        self.assertEqual(repeated.book, dispatched.book)

        completed = complete_deadline(repeated.book, identity="irq", generation=0)
        completed_ref = r2_complete_deadline(to_r2_book(repeated.book), identity="irq", generation=0)
        self.compare(completed, completed_ref)
        self.assertEqual(completed.book.deadlines[0].status, ADeadlineStatus.COMPLETED)

    def test_generation_replacement_is_atomic_and_fences_old_generation(self):
        first = self.publish(
            owner="cpu", identity="irq", generation=0,
            at_common=Fraction(10), effect_id="old",
        )
        actual = replace_deadline(
            first.book, self.context, owner="cpu", identity="irq",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new",
        )
        expected = r2_replace_deadline(
            to_r2_book(first.book), to_r2_context(self.context),
            owner="cpu", identity="irq", generation=0, cancel_at=Fraction(5),
            new_generation=1, new_at_common=Fraction(20), new_effect_id="new",
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, ADeadlineCode.ACCEPTED)
        self.assertEqual(
            tuple(item.status for item in actual.book.deadlines),
            (ADeadlineStatus.CANCELLED, ADeadlineStatus.SCHEDULED),
        )
        stale = dispatch_deadline(actual.book, identity="irq", generation=0)
        stale_ref = r2_dispatch_deadline(to_r2_book(actual.book), identity="irq", generation=0)
        self.compare(stale, stale_ref)
        self.assertEqual(stale.code, ADeadlineCode.STALE_IGNORED)

    def test_microstep_limit_is_finite_and_non_mutating(self):
        book = ADeadlineBook(max_microsteps=1)
        result = self.publish(
            book=book, owner="cpu", identity="irq", generation=0,
            at_common=Fraction(0), effect_id="irq-line",
            microstep=True, microstep_level=2,
        )
        self.assertEqual(result.code, ADeadlineCode.RESOURCE_EXHAUSTED)
        self.assertEqual(result.book, book)

    def test_active_deadline_view_is_stable_and_excludes_terminal(self):
        first = self.publish(
            owner="cpu", identity="irq-z", generation=0,
            at_common=Fraction(10), effect_id="z",
        )
        second = self.publish(
            book=first.book, owner="vdp", identity="irq-a", generation=0,
            at_common=Fraction(12), effect_id="a",
        )
        closed = close_watermark(second.book, Fraction(12))
        dispatched = dispatch_deadline(closed.book, identity="irq-a", generation=0)
        completed = complete_deadline(dispatched.book, identity="irq-a", generation=0)
        actual = active_deadlines(completed.book)
        expected = r2_active_deadlines(to_r2_book(completed.book))
        self.assertEqual(
            tuple((item.identity, item.generation, item.status.value) for item in actual),
            tuple((item.identity, item.generation, item.status.value) for item in expected),
        )


if __name__ == "__main__":
    unittest.main()
