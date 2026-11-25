"""R2-B executable cases for deadline ownership, admission, races and ties."""
from fractions import Fraction
import unittest

from chronos_reference import ClockDomain, make_state
from deadline_reference import (
    DeadlineBook,
    DeadlineCode,
    DeadlineStatus,
    active_deadlines,
    arbitrate_cancellations,
    cancel_deadline,
    close_watermark,
    dispatch_deadline,
    publish_deadline,
    replace_deadline,
    schedule_microstep,
)


def machine(*, progress=0, watermark=None, participants=("cpu",)):
    domains = [
        ClockDomain(name, Fraction(1), progress_local=Fraction(progress),
                    horizon_local=Fraction(100))
        for name in participants
    ]
    return make_state(
        profile="SMS-NTSC-ABSTRACT",
        domains=domains,
        expected_participants=participants,
        watermark_common=None if watermark is None else Fraction(watermark),
    )


def publish(book, state, *, identity="evt", generation=1, at=10,
            owner="cpu", **kwargs):
    return publish_deadline(
        book, state, owner=owner, identity=identity, generation=generation,
        at_common=Fraction(at), effect_id=f"effect:{identity}:{generation}",
        **kwargs,
    )


class R2BPublicationTests(unittest.TestCase):
    def test_future_deadline_is_owned_and_published(self):
        result = publish(DeadlineBook(), machine(), at=10)
        self.assertEqual(result.code, DeadlineCode.ACCEPTED)
        item = result.book.deadlines[0]
        self.assertEqual((item.owner, item.identity, item.generation),
                         ("cpu", "evt", 1))
        self.assertEqual(item.status, DeadlineStatus.SCHEDULED)

    def test_deadline_before_owner_progress_is_late(self):
        result = publish(DeadlineBook(), machine(progress=5), at=4)
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)

    def test_deadline_at_or_before_closed_watermark_is_late(self):
        for instant in (4, 5):
            with self.subTest(instant=instant):
                result = publish(
                    DeadlineBook(watermark_common=Fraction(5)),
                    machine(progress=0), at=instant,
                )
                self.assertEqual(result.code, DeadlineCode.LATE_INPUT)

    def test_same_instant_as_progress_requires_explicit_microstep(self):
        state = machine(progress=5)
        rejected = publish(DeadlineBook(), state, at=5)
        accepted = publish(DeadlineBook(), state, at=5, microstep=True)
        self.assertEqual(rejected.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(accepted.code, DeadlineCode.ACCEPTED)
        self.assertEqual(accepted.book.deadlines[0].microstep, 1)

    def test_incomplete_participants_block_before_temporal_admission(self):
        state = machine(progress=10, participants=("cpu", "vdp"))
        state = make_state(
            profile=state.profile,
            domains=[state.domains[0]],
            expected_participants=("cpu", "vdp"),
        )
        result = publish(DeadlineBook(), state, at=1)
        self.assertEqual(result.code, DeadlineCode.BLOCKED)

    def test_owner_must_be_a_nonblank_participant(self):
        result = publish(DeadlineBook(), machine(), owner=" ", at=10)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)

    def test_identity_generation_must_increase_after_terminal_generation(self):
        first = publish(DeadlineBook(), machine(), at=10)
        cancelled = cancel_deadline(
            first.book, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(cancelled.code, DeadlineCode.ACCEPTED)
        terminal = arbitrate_cancellations(cancelled.book).book
        second = publish(terminal, machine(), at=12, generation=2)
        stale = publish(terminal, machine(), at=12, generation=1)
        self.assertEqual(second.code, DeadlineCode.ACCEPTED)
        self.assertEqual(stale.code, DeadlineCode.INVALID_INPUT)


class R2BCancellationRaceTests(unittest.TestCase):
    def setUp(self):
        self.initial = publish(DeadlineBook(), machine(), at=10).book

    def test_cancel_frontier_includes_closed_watermark(self):
        state = machine(watermark=3)
        initial = publish(
            DeadlineBook(watermark_common=Fraction(3)), state, at=10
        ).book
        result = cancel_deadline(
            initial, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(2),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, initial)

    def test_cancel_frontier_includes_every_participant_progress(self):
        initial_state = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), initial_state, at=10).book
        current_state = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(1), progress_local=Fraction(0)),
                ClockDomain("vdp", Fraction(1), progress_local=Fraction(5)),
            ],
            expected_participants=("cpu", "vdp"),
        )
        result = cancel_deadline(
            initial, current_state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, initial)

    def test_cancel_frontier_blocks_on_incomplete_participants(self):
        initial_state = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), initial_state, at=10).book
        incomplete = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[ClockDomain("cpu", Fraction(1), horizon_local=Fraction(100))],
            expected_participants=("cpu", "vdp"),
        )
        result = cancel_deadline(
            initial, incomplete, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.BLOCKED)
        self.assertEqual(result.book, initial)

    def test_cancel_frontier_rejects_invalid_clock_conversion_without_mutation(self):
        initial_state = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), initial_state, at=10).book
        invalid = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(1), horizon_local=Fraction(100)),
                ClockDomain("vdp", Fraction(0), horizon_local=Fraction(100)),
            ],
            expected_participants=("cpu", "vdp"),
        )
        result = cancel_deadline(
            initial, invalid, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, initial)

    def test_cancel_frontier_rejects_disagreeing_watermarks_without_mutation(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        inconsistent = machine(watermark=1)
        result = cancel_deadline(
            initial, inconsistent, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, initial)

    def test_cancel_frontier_accepts_exact_rational_participant_conversions(self):
        state = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(2), progress_local=Fraction(1, 2),
                            horizon_local=Fraction(100)),
                ClockDomain("vdp", Fraction(3), progress_local=Fraction(1, 3),
                            horizon_local=Fraction(100)),
            ],
            expected_participants=("cpu", "vdp"),
        )
        initial = publish(DeadlineBook(), state, at=10).book
        result = cancel_deadline(
            initial, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(2),
        )
        self.assertEqual(result.code, DeadlineCode.ACCEPTED)
        self.assertEqual(result.book.pending_cancellations[0].at_common,
                         Fraction(2))

    def test_cancel_at_or_before_admission_frontier_is_late(self):
        result = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(2),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)

    def test_cancel_at_deadline_is_late_even_if_dispatch_is_delayed(self):
        result = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(10),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)

    def test_admitted_cancel_blocks_watermark_until_arbitrated(self):
        admitted = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(admitted.code, DeadlineCode.ACCEPTED)
        blocked = close_watermark(admitted.book, Fraction(10))
        self.assertEqual(blocked.code, DeadlineCode.BLOCKED)
        arbitrated = arbitrate_cancellations(admitted.book)
        closed = close_watermark(arbitrated.book, Fraction(10))
        self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
        self.assertEqual(
            closed.book.deadlines[0].status, DeadlineStatus.CANCELLED
        )
        self.assertEqual(closed.claimed, ())

    def test_cancel_after_watermark_closed_does_not_suppress_claim(self):
        claimed = close_watermark(self.initial, Fraction(10))
        self.assertEqual(claimed.book.deadlines[0].status, DeadlineStatus.CLAIMED)
        late = cancel_deadline(
            claimed.book, machine(progress=2, watermark=10), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(11),
        )
        self.assertEqual(late.code, DeadlineCode.LATE_INPUT)
        dispatched = dispatch_deadline(
            claimed.book, identity="evt", generation=1
        )
        self.assertEqual(dispatched.book.deadlines[0].status,
                         DeadlineStatus.DISPATCHED)
        repeated = dispatch_deadline(
            dispatched.book, identity="evt", generation=1
        )
        self.assertEqual(repeated.code, DeadlineCode.ACCEPTED)
        self.assertEqual(repeated.book, dispatched.book)

    def test_old_generation_cannot_dispatch_after_replacement(self):
        admitted = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        terminal = arbitrate_cancellations(admitted.book).book
        replacement = publish(terminal, machine(), at=12, generation=2)
        claimed = close_watermark(replacement.book, Fraction(12))
        stale = dispatch_deadline(
            claimed.book, identity="evt", generation=1
        )
        current = dispatch_deadline(
            stale.book, identity="evt", generation=2
        )
        self.assertEqual(stale.code, DeadlineCode.STALE_IGNORED)
        self.assertEqual(current.code, DeadlineCode.ACCEPTED)
        self.assertEqual(
            next(d for d in current.book.deadlines if d.generation == 1).status,
            DeadlineStatus.CANCELLED,
        )


class R2BReplacementTests(unittest.TestCase):
    def test_replacement_cancels_old_generation_and_publishes_new_atomically(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        result = replace_deadline(
            initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            cancel_at=Fraction(4),
            new_generation=2, new_at_common=Fraction(12),
            new_effect_id="replacement-effect",
        )
        self.assertEqual(result.code, DeadlineCode.ACCEPTED)
        old = next(d for d in result.book.deadlines if d.generation == 1)
        new = next(d for d in result.book.deadlines if d.generation == 2)
        self.assertEqual(old.status, DeadlineStatus.CANCELLED)
        self.assertEqual(new.status, DeadlineStatus.SCHEDULED)

    def test_rejected_replacement_does_not_cancel_original_generation(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        result = replace_deadline(
            initial, machine(progress=20), owner="cpu", identity="evt",
            generation=1, cancel_at=Fraction(4), new_generation=2,
            new_at_common=Fraction(15), new_effect_id="invalid-replacement",
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, initial)
        self.assertEqual(result.book.deadlines[0].status, DeadlineStatus.SCHEDULED)


class R2BCausalTieTests(unittest.TestCase):
    def test_noncommutative_unknown_tie_does_not_close_watermark(self):
        book = publish(DeadlineBook(), machine(), identity="a", at=10).book
        book = publish(book, machine(), identity="b", at=10).book
        result = close_watermark(book, Fraction(10))
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertIsNone(result.book.watermark_common)
        self.assertTrue(all(d.status is DeadlineStatus.SCHEDULED
                            for d in result.book.deadlines))

    def test_commutative_tie_is_invariant_to_publication_order(self):
        def make(order):
            book = DeadlineBook()
            for identity in order:
                book = publish(
                    book, machine(), identity=identity, at=10,
                    commutative=True,
                ).book
            return close_watermark(book, Fraction(10))
        left = make(("a", "b"))
        right = make(("b", "a"))
        self.assertEqual(left.code, DeadlineCode.ACCEPTED)
        self.assertEqual(left.claimed, right.claimed)
        self.assertEqual(left.claimed, (("a", 1), ("b", 1)))

    def test_explicit_distinct_phase_orders_tie_without_insertion_precedence(self):
        def make(order):
            book = DeadlineBook()
            values = {"later": 2, "earlier": 1}
            for identity in order:
                book = publish(
                    book, machine(), identity=identity, at=10,
                    phase_order=values[identity],
                    phase_evidence="synthetic-test-fixture",
                ).book
            return close_watermark(book, Fraction(10))
        left = make(("later", "earlier"))
        right = make(("earlier", "later"))
        self.assertEqual(left.claimed, right.claimed)
        self.assertEqual(left.claimed, (("earlier", 1), ("later", 1)))

    def test_same_phase_noncommutative_tie_remains_indeterminate(self):
        book = publish(
            DeadlineBook(), machine(), identity="a", at=10,
            phase_order=1, phase_evidence="synthetic-test-fixture",
        ).book
        book = publish(
            book, machine(), identity="b", at=10,
            phase_order=1, phase_evidence="synthetic-test-fixture",
        ).book
        result = close_watermark(book, Fraction(10))
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)


class R2BMicrostepTests(unittest.TestCase):
    def test_same_instant_child_increments_microstep(self):
        first = schedule_microstep(
            DeadlineBook(), owner="cpu", identity="root", generation=1,
            at_common=Fraction(5), effect_id="root-effect",
            current_microstep=0,
        )
        child = schedule_microstep(
            first.book, owner="cpu", identity="child", generation=1,
            at_common=Fraction(5), effect_id="child-effect",
            current_microstep=first.book.deadlines[0].microstep,
        )
        self.assertEqual(first.code, DeadlineCode.ACCEPTED)
        self.assertEqual(child.code, DeadlineCode.ACCEPTED)
        self.assertEqual(child.book.deadlines[-1].microstep, 2)

    def test_distinct_microsteps_order_noncommutative_same_instant_events(self):
        first = schedule_microstep(
            DeadlineBook(), owner="cpu", identity="root", generation=1,
            at_common=Fraction(5), effect_id="root-effect",
            current_microstep=0,
        )
        child = schedule_microstep(
            first.book, owner="cpu", identity="child", generation=1,
            at_common=Fraction(5), effect_id="child-effect",
            current_microstep=first.book.deadlines[0].microstep,
        )
        closed = close_watermark(child.book, Fraction(5))
        self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
        self.assertEqual(closed.claimed, (("root", 1), ("child", 1)))

    def test_microstep_limit_is_explicit_and_does_not_mutate(self):
        book = DeadlineBook(max_microsteps=2)
        first = schedule_microstep(
            book, owner="cpu", identity="one", generation=1,
            at_common=Fraction(5), effect_id="one",
            current_microstep=0,
        )
        second = schedule_microstep(
            first.book, owner="cpu", identity="two", generation=1,
            at_common=Fraction(5), effect_id="two",
            current_microstep=1,
        )
        before = second.book
        exhausted = schedule_microstep(
            second.book, owner="cpu", identity="three", generation=1,
            at_common=Fraction(5), effect_id="three",
            current_microstep=2,
        )
        self.assertEqual(exhausted.code, DeadlineCode.RESOURCE_EXHAUSTED)
        self.assertEqual(exhausted.book, before)

    def test_active_deadline_view_is_stable_and_excludes_cancelled(self):
        book = publish(DeadlineBook(), machine(), identity="z", at=12).book
        book = publish(book, machine(), identity="a", at=10).book
        cancelled = cancel_deadline(
            book, machine(progress=2), owner="cpu", identity="z", generation=1,
            at_common=Fraction(4),
        )
        terminal = arbitrate_cancellations(cancelled.book).book
        self.assertEqual(
            tuple(item.identity for item in active_deadlines(terminal)),
            ("a",),
        )
