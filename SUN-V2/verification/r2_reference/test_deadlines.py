"""R2-B executable cases for deadline ownership, admission, races and ties."""
from dataclasses import replace
from fractions import Fraction
import unittest

from chronos_reference import ClockDomain, Obligation, make_state
from deadline_reference import (
    Cancellation,
    Deadline,
    DeadlineBook,
    DeadlineCode,
    DeadlineStatus,
    active_deadlines,
    arbitrate_cancellations,
    cancel_deadline,
    close_watermark,
    complete_deadline,
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
                    machine(progress=0, watermark=5), at=instant,
                )
                self.assertEqual(result.code, DeadlineCode.LATE_INPUT)

    def test_publication_rejects_disagreeing_watermarks_without_mutation(self):
        cases = (
            (DeadlineBook(), machine(watermark=5)),
            (DeadlineBook(watermark_common=Fraction(5)), machine()),
        )
        for book, state in cases:
            with self.subTest(book_watermark=book.watermark_common,
                              state_watermark=state.watermark_common):
                result = publish(book, state, at=10)
                self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
                self.assertEqual(result.book, book)

    def test_same_instant_as_progress_requires_explicit_microstep(self):
        state = machine(progress=5)
        rejected = publish(DeadlineBook(), state, at=5)
        accepted = publish(DeadlineBook(), state, at=5, microstep=True)
        self.assertEqual(rejected.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(accepted.code, DeadlineCode.ACCEPTED)
        self.assertEqual(accepted.book.deadlines[0].microstep, 1)

    def test_publication_at_owner_progress_with_open_watermark_requires_microstep(self):
        cases = (
            (DeadlineBook(), machine(progress=5)),
            (
                DeadlineBook(watermark_common=Fraction(4)),
                machine(progress=5, watermark=4),
            ),
        )
        for book, state in cases:
            with self.subTest(watermark=book.watermark_common):
                rejected = publish(book, state, at=5)
                accepted = publish(book, state, at=5, microstep=True)
                self.assertEqual(rejected.code, DeadlineCode.INDETERMINATE)
                self.assertEqual(rejected.book, book)
                self.assertEqual(accepted.code, DeadlineCode.ACCEPTED)
                self.assertEqual(accepted.book.deadlines[0].microstep, 1)

        closed_book = DeadlineBook(watermark_common=Fraction(5))
        closed_state = machine(progress=5, watermark=5)
        at_closed_cut = publish(
            closed_book, closed_state, at=5, microstep=True
        )
        self.assertEqual(at_closed_cut.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(at_closed_cut.book, closed_book)

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

    def test_publication_rejects_duplicate_participant_identity(self):
        state = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=(
                ClockDomain("cpu", Fraction(1), horizon_local=Fraction(100)),
                ClockDomain("cpu", Fraction(2), horizon_local=Fraction(100)),
            ),
            expected_participants=("cpu",),
        )
        book = DeadlineBook()
        result = publish(book, state, at=10)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_publication_rejects_empty_profile_without_mutation(self):
        state = machine()
        invalid = make_state(
            profile="",
            domains=state.domains,
            expected_participants=state.expected_participants,
        )
        book = DeadlineBook()
        result = publish(book, invalid, at=10)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

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

    def test_direct_publication_enforces_finite_microstep_limit(self):
        book = DeadlineBook(max_microsteps=2)

        boundary = publish(
            book, machine(), at=5, microstep=True, microstep_level=2
        )
        self.assertEqual(boundary.code, DeadlineCode.ACCEPTED)

        for level in (3, 999):
            with self.subTest(level=level):
                rejected = publish(
                    book, machine(), at=5,
                    microstep=True, microstep_level=level,
                )
                self.assertEqual(rejected.code, DeadlineCode.RESOURCE_EXHAUSTED)
                self.assertEqual(rejected.book, book)

    def test_publication_order_permutation_preserves_causal_claim_and_dispatch(self):
        def execute(publication_order):
            book = DeadlineBook()
            instants = {"early": Fraction(5), "late": Fraction(7)}
            for identity in publication_order:
                result = publish(
                    book, machine(), identity=identity, at=instants[identity]
                )
                self.assertEqual(result.code, DeadlineCode.ACCEPTED)
                book = result.book

            closed = close_watermark(book, Fraction(7))
            self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
            self.assertEqual(
                closed.claimed, (("early", 1), ("late", 1))
            )
            book = closed.book
            for identity in ("early", "late"):
                dispatched = dispatch_deadline(
                    book, identity=identity, generation=1
                )
                self.assertEqual(dispatched.code, DeadlineCode.ACCEPTED)
                completed = complete_deadline(
                    dispatched.book, identity=identity, generation=1
                )
                self.assertEqual(completed.code, DeadlineCode.ACCEPTED)
                book = completed.book
            return tuple(sorted(
                (item.identity, item.status) for item in book.deadlines
            ))

        self.assertEqual(
            execute(("early", "late")),
            execute(("late", "early")),
        )

    def test_preconstructed_book_enforces_microstep_limit_before_claiming(self):
        for level in (0, 1, 2, 999):
            with self.subTest(level=level):
                deadline = Deadline(
                    owner="cpu",
                    identity="prebuilt",
                    generation=1,
                    at_common=Fraction(5),
                    effect_id="prebuilt-effect",
                    microstep=level,
                )
                book = DeadlineBook(
                    deadlines=(deadline,),
                    max_microsteps=2,
                )

                result = close_watermark(book, Fraction(5))

                if level <= 2:
                    self.assertEqual(result.code, DeadlineCode.ACCEPTED)
                    self.assertEqual(result.claimed, (("prebuilt", 1),))
                    self.assertEqual(
                        result.book.deadlines[0].status, DeadlineStatus.CLAIMED
                    )
                else:
                    self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                    self.assertEqual(result.book, book)
                    self.assertEqual(result.claimed, ())
                    self.assertEqual(
                        result.book.deadlines[0].status, DeadlineStatus.SCHEDULED
                    )

    def test_publication_rejects_non_exact_or_regressed_safe_horizon(self):
        book = DeadlineBook()
        valid_state = machine(progress=2)
        for horizon in (0.5, Fraction(-1), Fraction(1)):
            with self.subTest(horizon=horizon):
                domain = replace(valid_state.domains[0], horizon_local=horizon)
                state = replace(valid_state, domains=(domain,))
                rejected = publish(book, state, at=10)
                expected = (
                    DeadlineCode.INVALID_INPUT
                    if isinstance(horizon, float)
                    else DeadlineCode.INVARIANT_VIOLATION
                )
                self.assertEqual(rejected.code, expected)
                self.assertEqual(rejected.book, book)


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
                ClockDomain(
                    "cpu", Fraction(1), progress_local=Fraction(0),
                    horizon_local=Fraction(100),
                ),
                ClockDomain(
                    "vdp", Fraction(1), progress_local=Fraction(5),
                    horizon_local=Fraction(100),
                ),
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

    def test_cancel_frontier_blocks_when_required_participant_set_is_empty(self):
        empty = make_state(domains=[], expected_participants=[])
        result = cancel_deadline(
            self.initial, empty, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.BLOCKED)
        self.assertEqual(result.book, self.initial)

    def test_cancel_frontier_requires_owner_in_declared_and_present_participants(self):
        states = (
            machine(participants=("vdp",)),
            make_state(
                profile="SMS-NTSC-ABSTRACT",
                domains=[ClockDomain("vdp", Fraction(1), horizon_local=Fraction(100))],
                expected_participants=("cpu", "vdp"),
            ),
        )
        for state in states:
            with self.subTest(expected=state.expected_participants,
                              actual=tuple(d.domain_id for d in state.domains)):
                result = cancel_deadline(
                    self.initial, state, owner="cpu", identity="evt",
                    generation=1, at_common=Fraction(4),
                )
                self.assertEqual(result.code, DeadlineCode.BLOCKED)
                self.assertEqual(result.book, self.initial)

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

    def test_non_exact_clock_coordinates_are_invalid_input_before_comparison(self):
        valid = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), valid, at=10).book
        for field in ("local_to_common", "common_offset", "progress_local"):
            with self.subTest(field=field):
                domains = list(valid.domains)
                domains[1] = replace(domains[1], **{field: 1.5})
                malformed = replace(valid, domains=tuple(domains))

                published = publish_deadline(
                    DeadlineBook(), malformed, owner="cpu", identity="evt",
                    generation=1, at_common=Fraction(10), effect_id="fx",
                )
                cancelled = cancel_deadline(
                    initial, malformed, owner="cpu", identity="evt",
                    generation=1, at_common=Fraction(4),
                )

                self.assertEqual(published.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(published.book, DeadlineBook())
                self.assertEqual(cancelled.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(cancelled.book, initial)

    def test_cancel_frontier_rejects_disagreeing_watermarks_without_mutation(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        inconsistent = machine(watermark=1)
        result = cancel_deadline(
            initial, inconsistent, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, initial)

    def test_cancel_resolves_frontier_before_claimed_lifecycle_status(self):
        claimed = close_watermark(self.initial, Fraction(10)).book
        incomplete = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[ClockDomain("cpu", Fraction(1), horizon_local=Fraction(100))],
            expected_participants=("cpu", "vdp"),
            watermark_common=Fraction(10),
        )
        result = cancel_deadline(
            claimed, incomplete, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(11),
        )
        self.assertEqual(result.code, DeadlineCode.BLOCKED)
        self.assertEqual(result.book, claimed)

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

    def test_cancel_at_deadline_without_causal_relation_is_indeterminate(self):
        state = machine(progress=2)
        result = cancel_deadline(
            self.initial, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(10),
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, self.initial)

    def test_cancel_at_deadline_uses_explicit_microstep_or_phase_order(self):
        earlier_microstep = publish(
            DeadlineBook(), machine(), at=10,
            microstep=True, microstep_level=2,
        ).book
        admitted = cancel_deadline(
            earlier_microstep, machine(progress=2), owner="cpu",
            identity="evt", generation=1, at_common=Fraction(10),
            microstep_level=1,
        )
        self.assertEqual(admitted.code, DeadlineCode.ACCEPTED)
        blocked = close_watermark(admitted.book, Fraction(10))
        self.assertEqual(blocked.code, DeadlineCode.BLOCKED)
        self.assertEqual(blocked.book, admitted.book)
        arbitrated = arbitrate_cancellations(admitted.book)
        self.assertEqual(arbitrated.code, DeadlineCode.ACCEPTED)
        self.assertEqual(
            arbitrated.book.deadlines[0].status, DeadlineStatus.CANCELLED
        )
        closed = close_watermark(arbitrated.book, Fraction(10))
        self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
        self.assertEqual(closed.claimed, ())

        later_microstep = publish(
            DeadlineBook(), machine(), at=10,
            microstep=True, microstep_level=1,
        ).book
        late = cancel_deadline(
            later_microstep, machine(progress=2), owner="cpu",
            identity="evt", generation=1, at_common=Fraction(10),
            microstep_level=2,
        )
        self.assertEqual(late.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(late.book, later_microstep)

        unknown_same_level = cancel_deadline(
            later_microstep, machine(progress=2), owner="cpu",
            identity="evt", generation=1, at_common=Fraction(10),
            microstep_level=1,
        )
        self.assertEqual(unknown_same_level.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(unknown_same_level.book, later_microstep)

        phase_deadline = publish(
            DeadlineBook(), machine(), at=10,
            phase_order=2, phase_evidence="profile-order-fixture",
        ).book
        phase_before = cancel_deadline(
            phase_deadline, machine(progress=2), owner="cpu",
            identity="evt", generation=1, at_common=Fraction(10),
            phase_order=1, phase_evidence="profile-order-fixture",
        )
        self.assertEqual(phase_before.code, DeadlineCode.ACCEPTED)

        phase_after = cancel_deadline(
            phase_deadline, machine(progress=2), owner="cpu",
            identity="evt", generation=1, at_common=Fraction(10),
            phase_order=3, phase_evidence="profile-order-fixture",
        )
        self.assertEqual(phase_after.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(phase_after.book, phase_deadline)

    def test_cancel_admission_frontier_precedes_same_instant_microstep_order(self):
        deadline = publish(
            DeadlineBook(), machine(progress=10), at=10,
            microstep=True, microstep_level=2,
        ).book
        rejected = cancel_deadline(
            deadline, machine(progress=10), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(10), microstep_level=1,
        )
        self.assertEqual(rejected.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(rejected.book, deadline)

    def test_cancellation_microstep_obeys_finite_book_limit(self):
        book = DeadlineBook(max_microsteps=2)
        deadline = publish(
            book, machine(), at=10, microstep=True, microstep_level=2
        ).book

        rejected = cancel_deadline(
            deadline, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(8), microstep_level=3,
        )
        self.assertEqual(rejected.code, DeadlineCode.RESOURCE_EXHAUSTED)
        self.assertEqual(rejected.book, deadline)

        preconstructed = DeadlineBook(
            deadlines=(
                replace(deadline.deadlines[0],
                        status=DeadlineStatus.CANCEL_PENDING),
            ),
            pending_cancellations=(
                Cancellation(
                    "cpu", "evt", 1, Fraction(8), microstep_level=3
                ),
            ),
            max_microsteps=2,
        )
        invalid = arbitrate_cancellations(preconstructed)
        self.assertEqual(invalid.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(invalid.book, preconstructed)

    def test_cancel_at_deadline_before_watermark_without_causal_relation_is_indeterminate(self):
        state = machine(progress=2, watermark=9)
        book = DeadlineBook(
            deadlines=self.initial.deadlines,
            watermark_common=Fraction(9),
        )
        result = cancel_deadline(
            book, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(10),
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, book)

    def test_cancel_at_deadline_when_watermark_is_closed_is_late(self):
        for watermark in (Fraction(10), Fraction(11)):
            with self.subTest(watermark=watermark):
                state = machine(progress=2, watermark=watermark)
                book = DeadlineBook(
                    deadlines=(
                        replace(
                            self.initial.deadlines[0],
                            status=DeadlineStatus.CLAIMED,
                        ),
                    ),
                    watermark_common=watermark,
                )
                result = cancel_deadline(
                    book, state, owner="cpu", identity="evt", generation=1,
                    at_common=Fraction(10),
                )
                self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
                self.assertEqual(result.book, book)

    def test_cancel_before_deadline_but_after_watermark_closed_is_late(self):
        state = machine(progress=2, watermark=10)
        book = DeadlineBook(
            deadlines=(
                replace(
                    self.initial.deadlines[0],
                    status=DeadlineStatus.CLAIMED,
                ),
            ),
            watermark_common=Fraction(10),
        )
        result = cancel_deadline(
            book, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(9),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, book)

    def test_cancel_after_deadline_is_late_even_if_dispatch_is_delayed(self):
        result = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt", generation=1,
            at_common=Fraction(11),
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, self.initial)

    def test_preconstructed_pending_cancellation_must_precede_deadline(self):
        deadline = replace(
            self.initial.deadlines[0], status=DeadlineStatus.CANCEL_PENDING
        )
        for cancel_at, watermark, expected in (
            (Fraction(10), None, DeadlineCode.INDETERMINATE),
            (Fraction(10), Fraction(10), DeadlineCode.LATE_INPUT),
            (Fraction(11), None, DeadlineCode.LATE_INPUT),
            (Fraction(9), Fraction(10), DeadlineCode.INVARIANT_VIOLATION),
            (Fraction(9), Fraction(11), DeadlineCode.INVARIANT_VIOLATION),
        ):
            with self.subTest(cancel_at=cancel_at, watermark=watermark):
                book = DeadlineBook(
                    deadlines=(deadline,),
                    watermark_common=watermark,
                    pending_cancellations=(
                        Cancellation("cpu", "evt", 1, cancel_at),
                    ),
                )
                for result in (
                    arbitrate_cancellations(book),
                    close_watermark(book, Fraction(10)),
                ):
                    self.assertEqual(result.code, expected)
                    self.assertEqual(result.book, book)
                    self.assertEqual(result.claimed, ())

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

    def test_cancellation_rejects_non_exact_or_regressed_safe_horizon(self):
        valid_state = machine(progress=2)
        for horizon in (0.5, Fraction(-1), Fraction(1)):
            with self.subTest(horizon=horizon):
                domain = replace(valid_state.domains[0], horizon_local=horizon)
                state = replace(valid_state, domains=(domain,))
                rejected = cancel_deadline(
                    self.initial, state, owner="cpu", identity="evt",
                    generation=1, at_common=Fraction(4),
                )
                expected = (
                    DeadlineCode.INVALID_INPUT
                    if isinstance(horizon, float)
                    else DeadlineCode.INVARIANT_VIOLATION
                )
                self.assertEqual(rejected.code, expected)
                self.assertEqual(rejected.book, self.initial)

    def test_publication_rejects_global_horizon_behind_watermark(self):
        state = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(1), horizon_local=Fraction(10)),
                ClockDomain("vdp", Fraction(1), horizon_local=Fraction(30)),
            ],
            expected_participants=("cpu", "vdp"),
            watermark_common=Fraction(20),
        )
        book = DeadlineBook(watermark_common=Fraction(20))
        rejected = publish(book, state, at=30)
        self.assertEqual(rejected.code, DeadlineCode.INVARIANT_VIOLATION)
        self.assertEqual(rejected.book, book)

    def test_cancellation_rejects_global_horizon_behind_watermark(self):
        initial_state = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), initial_state, at=30).book
        inconsistent = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(1), horizon_local=Fraction(10)),
                ClockDomain("vdp", Fraction(1), horizon_local=Fraction(30)),
            ],
            expected_participants=("cpu", "vdp"),
            watermark_common=Fraction(20),
        )
        book = replace(initial, watermark_common=Fraction(20))
        rejected = cancel_deadline(
            book, inconsistent, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(25),
        )
        self.assertEqual(rejected.code, DeadlineCode.INVARIANT_VIOLATION)
        self.assertEqual(rejected.book, book)

    def test_cancellation_detects_known_horizon_violation_before_incomplete_block(self):
        initial_state = machine(participants=("cpu", "vdp"))
        initial = publish(DeadlineBook(), initial_state, at=30).book
        incomplete = make_state(
            profile="SMS-NTSC-ABSTRACT",
            domains=[
                ClockDomain("cpu", Fraction(1), horizon_local=Fraction(10)),
            ],
            expected_participants=("cpu", "vdp"),
            watermark_common=Fraction(20),
        )
        book = replace(initial, watermark_common=Fraction(20))
        rejected = cancel_deadline(
            book, incomplete, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(25),
        )
        self.assertEqual(rejected.code, DeadlineCode.INVARIANT_VIOLATION)
        self.assertEqual(rejected.book, book)

    def test_deadline_admission_rejects_obligation_inconsistent_reference_state(self):
        cases = (
            (Fraction(0), Fraction(5), Fraction(5)),
            (Fraction(6), None, Fraction(5)),
        )
        for progress, watermark, obligation_time in cases:
            with self.subTest(progress=progress, watermark=watermark):
                initial_book = DeadlineBook(watermark_common=watermark)
                initial_state = machine(progress=0, watermark=watermark)
                scheduled = publish(initial_book, initial_state, at=10).book
                inconsistent = make_state(
                    profile="SMS-NTSC-ABSTRACT",
                    domains=(
                        ClockDomain(
                            "cpu", Fraction(1), progress_local=progress,
                            horizon_local=Fraction(100),
                        ),
                    ),
                    expected_participants=("cpu",),
                    watermark_common=watermark,
                    obligations=(
                        Obligation(
                            "pending", "cpu", obligation_time, resolved=False
                        ),
                    ),
                )

                publication = publish(initial_book, inconsistent, at=12)
                cancellation = cancel_deadline(
                    scheduled, inconsistent, owner="cpu", identity="evt",
                    generation=1, at_common=Fraction(8),
                )

                self.assertEqual(
                    publication.code, DeadlineCode.INVARIANT_VIOLATION
                )
                self.assertEqual(publication.book, initial_book)
                self.assertEqual(
                    cancellation.code, DeadlineCode.INVARIANT_VIOLATION
                )
                self.assertEqual(cancellation.book, scheduled)


class R2BReplacementTests(unittest.TestCase):
    def test_replacement_validates_new_descriptor_before_temporal_admission(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        invalid_descriptors = (
            {"new_generation": -1, "new_effect_id": "valid-effect"},
            {"new_generation": 2, "new_effect_id": ""},
            {"new_generation": True, "new_effect_id": "valid-effect"},
            {"new_generation": 2, "new_effect_id": "valid-effect", "microstep": 1},
        )
        for descriptor in invalid_descriptors:
            with self.subTest(descriptor=descriptor):
                result = replace_deadline(
                    initial, machine(progress=5), owner="cpu", identity="evt",
                    generation=1, cancel_at=Fraction(4),
                    new_generation=descriptor["new_generation"],
                    new_at_common=Fraction(15),
                    new_effect_id=descriptor["new_effect_id"],
                    microstep=descriptor.get("microstep", False),
                )
                self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(result.book, initial)
                self.assertEqual(
                    result.book.deadlines[0].status, DeadlineStatus.SCHEDULED
                )

    def test_replacement_rejects_new_generation_not_after_cancel_time(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        for new_at, expected in (
            (Fraction(3), DeadlineCode.LATE_INPUT),
            (Fraction(4), DeadlineCode.INDETERMINATE),
        ):
            with self.subTest(new_at=new_at):
                result = replace_deadline(
                    initial, machine(progress=2), owner="cpu", identity="evt",
                    generation=1, cancel_at=Fraction(4), new_generation=2,
                    new_at_common=new_at, new_effect_id="retroactive-replacement",
                )
                self.assertEqual(result.code, expected)
                self.assertEqual(result.book, initial)
                self.assertEqual(result.book.deadlines[0].status,
                                 DeadlineStatus.SCHEDULED)

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
    def setUp(self):
        self.initial = publish(DeadlineBook(), machine(), at=10).book

    def test_close_watermark_rejects_deadline_equal_to_existing_watermark(self):
        scheduled = publish(DeadlineBook(), machine(), at=5).book
        inconsistent = replace(scheduled, watermark_common=Fraction(5))
        result = close_watermark(inconsistent, Fraction(6))
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, inconsistent)
        self.assertEqual(result.claimed, ())

    def test_close_watermark_rejects_scheduled_deadline_behind_existing_watermark(self):
        scheduled = publish(DeadlineBook(), machine(), at=4).book
        inconsistent = replace(scheduled, watermark_common=Fraction(5))

        result = close_watermark(inconsistent, Fraction(6))

        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, inconsistent)
        self.assertEqual(result.claimed, ())

    def test_arbitration_rejects_orphan_and_owner_mismatched_cancellations(self):
        admitted = cancel_deadline(
            self.initial, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(4),
        ).book
        wrong_owner = replace(
            admitted,
            pending_cancellations=(
                replace(admitted.pending_cancellations[0], owner="vdp"),
            ),
        )
        orphan = DeadlineBook(
            pending_cancellations=(
                Cancellation("cpu", "missing", 1, Fraction(4)),
            )
        )
        missing_request = replace(admitted, pending_cancellations=())
        for inconsistent in (wrong_owner, orphan, missing_request):
            with self.subTest(book=inconsistent):
                result = arbitrate_cancellations(inconsistent)
                self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(result.book, inconsistent)
                self.assertEqual(
                    next((d.status for d in inconsistent.deadlines), None),
                    next((d.status for d in result.book.deadlines), None),
                )

    def test_noncommutative_unknown_tie_does_not_close_watermark(self):
        book = publish(DeadlineBook(), machine(), identity="a", at=10).book
        book = publish(book, machine(), identity="b", at=10).book
        result = close_watermark(book, Fraction(10))
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertIsNone(result.book.watermark_common)
        self.assertTrue(all(d.status is DeadlineStatus.SCHEDULED
                            for d in result.book.deadlines))

    def test_commutative_tie_is_invariant_to_publication_and_dispatch_order(self):
        def make(publication_order, dispatch_order):
            book = DeadlineBook()
            for identity in publication_order:
                book = publish(
                    book, machine(), identity=identity, at=10,
                    commutative=True,
                ).book
            closed = close_watermark(book, Fraction(10))
            self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
            book = closed.book
            for identity in dispatch_order:
                dispatched = dispatch_deadline(
                    book, identity=identity, generation=1
                )
                self.assertEqual(dispatched.code, DeadlineCode.ACCEPTED)
                completed = complete_deadline(
                    dispatched.book, identity=identity, generation=1
                )
                self.assertEqual(completed.code, DeadlineCode.ACCEPTED)
                book = completed.book
            final_states = tuple(sorted(
                (item.identity, item.status) for item in book.deadlines
            ))
            return closed.claimed, final_states

        left = make(("a", "b"), ("a", "b"))
        right = make(("b", "a"), ("b", "a"))
        self.assertEqual(left, right)
        self.assertEqual(left[0], (("a", 1), ("b", 1)))

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

    def test_same_instant_child_preserves_an_earlier_closed_watermark(self):
        book = DeadlineBook(watermark_common=Fraction(5))
        result = schedule_microstep(
            book, owner="cpu", identity="child", generation=1,
            at_common=Fraction(10), effect_id="child-effect",
            current_microstep=0,
        )
        self.assertEqual(result.code, DeadlineCode.ACCEPTED)
        self.assertEqual(result.book.watermark_common, Fraction(5))
        self.assertEqual(result.book.deadlines[0].microstep, 1)

    def test_same_instant_child_at_closed_watermark_is_late_without_mutation(self):
        book = DeadlineBook(watermark_common=Fraction(5))
        result = schedule_microstep(
            book, owner="cpu", identity="child", generation=1,
            at_common=Fraction(5), effect_id="child-effect",
            current_microstep=0,
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, book)

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

    def test_invalid_microstep_descriptors_are_invalid_input_without_exception(self):
        book = DeadlineBook()
        cases = (
            {"owner": []},
            {"identity": []},
            {"generation": True},
            {"effect_id": []},
            {"at_common": 5.0},
            {"current_microstep": -1},
            {"current_microstep": 0.5},
            {"commutative": 1},
        )
        for overrides in cases:
            with self.subTest(overrides=overrides):
                arguments = {
                    "owner": "cpu",
                    "identity": "child",
                    "generation": 1,
                    "at_common": Fraction(5),
                    "effect_id": "child-effect",
                    "current_microstep": 0,
                    "commutative": False,
                }
                arguments.update(overrides)
                result = schedule_microstep(book, **arguments)
                self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(result.book, book)

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


class R2BStructuralValidationTests(unittest.TestCase):
    def test_publication_rejects_non_clock_domain_before_reading_identity(self):
        state = replace(machine(), domains=(object(),))
        book = DeadlineBook()
        result = publish(book, state)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_publication_rejects_malformed_expected_participant_container(self):
        state = replace(machine(), expected_participants=[[]])
        book = DeadlineBook()
        result = publish(book, state)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_cancellation_rejects_non_clock_domain_before_reading_identity(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        state = replace(machine(progress=2), domains=(object(),))
        result = cancel_deadline(
            initial, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, initial)

    def test_cancellation_rejects_malformed_expected_participant_container(self):
        initial = publish(DeadlineBook(), machine(), at=10).book
        state = replace(machine(progress=2), expected_participants=[[]])
        result = cancel_deadline(
            initial, state, owner="cpu", identity="evt", generation=1,
            at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, initial)

    def test_publication_rejects_non_deadline_book_without_dereference(self):
        result = publish(None, machine())
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertIsNone(result.book)

    def test_cancellation_rejects_non_deadline_book_before_lookup(self):
        result = cancel_deadline(
            None, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertIsNone(result.book)

    def test_publication_rejects_malformed_deadline_entries(self):
        book = DeadlineBook(deadlines=(object(),))
        result = publish(book, machine())
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_cancellation_rejects_malformed_pending_cancellation_entries(self):
        book = DeadlineBook(pending_cancellations=(object(),))
        result = cancel_deadline(
            book, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(4),
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_dispatch_and_completion_reject_invalid_identity_or_generation(self):
        scheduled = publish(DeadlineBook(), machine(), at=10).book
        claimed = close_watermark(scheduled, Fraction(10)).book
        dispatched = dispatch_deadline(
            claimed, identity="evt", generation=1
        ).book
        invalid_descriptors = (
            {"identity": []},
            {"identity": ""},
            {"identity": "   "},
            {"generation": True},
            {"generation": -1},
            {"generation": 1.0},
            {"generation": 1.5},
        )
        for overrides in invalid_descriptors:
            with self.subTest(overrides=overrides):
                dispatch_args = {"identity": "evt", "generation": 1}
                dispatch_args.update(overrides)
                dispatch_result = dispatch_deadline(claimed, **dispatch_args)
                self.assertEqual(dispatch_result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(dispatch_result.book, claimed)

                complete_args = {"identity": "evt", "generation": 1}
                complete_args.update(overrides)
                complete_result = complete_deadline(dispatched, **complete_args)
                self.assertEqual(complete_result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(complete_result.book, dispatched)

    def test_watermark_close_rejects_malformed_deadline_book(self):
        book = DeadlineBook(deadlines=(object(),))
        result = close_watermark(book, Fraction(10))
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_scheduled_deadline_behind_closed_watermark_is_rejected_by_all_operations(self):
        inconsistent = DeadlineBook(
            deadlines=(
                Deadline("cpu", "evt", 1, Fraction(4), "effect:evt:1"),
            ),
            watermark_common=Fraction(5),
        )
        results = (
            publish_deadline(
                inconsistent, machine(), owner="cpu", identity="new",
                generation=1, at_common=Fraction(10), effect_id="effect:new:1",
            ),
            cancel_deadline(
                inconsistent, machine(), owner="cpu", identity="evt",
                generation=1, at_common=Fraction(6),
            ),
            arbitrate_cancellations(inconsistent),
            close_watermark(inconsistent, Fraction(6)),
            dispatch_deadline(inconsistent, identity="evt", generation=1),
            complete_deadline(inconsistent, identity="evt", generation=1),
            replace_deadline(
                inconsistent, machine(), owner="cpu", identity="evt",
                generation=1, cancel_at=Fraction(6), new_generation=2,
                new_at_common=Fraction(7), new_effect_id="effect:evt:2",
            ),
            schedule_microstep(
                inconsistent, owner="cpu", identity="child", generation=1,
                at_common=Fraction(6), effect_id="effect:child",
                current_microstep=0,
            ),
        )
        for result in results:
            with self.subTest(code=result.code):
                self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                self.assertEqual(result.book, inconsistent)
                self.assertEqual(result.claimed, ())

    def test_claimed_and_processed_deadlines_require_closed_watermark(self):
        statuses = (
            DeadlineStatus.CLAIMED,
            DeadlineStatus.DISPATCHED,
            DeadlineStatus.COMPLETED,
            DeadlineStatus.FAILED,
        )
        watermarks = (None, Fraction(9))
        for status in statuses:
            for watermark in watermarks:
                with self.subTest(status=status, watermark=watermark):
                    inconsistent = DeadlineBook(
                        deadlines=(
                            Deadline(
                                "cpu", "evt", 1, Fraction(10), "effect:evt:1",
                                status=status,
                            ),
                        ),
                        watermark_common=watermark,
                    )
                    dispatched = dispatch_deadline(
                        inconsistent, identity="evt", generation=1
                    )
                    completed = complete_deadline(
                        inconsistent, identity="evt", generation=1
                    )
                    self.assertEqual(
                        dispatched.code, DeadlineCode.INVARIANT_VIOLATION
                    )
                    self.assertEqual(dispatched.book, inconsistent)
                    self.assertEqual(completed.code, DeadlineCode.INVARIANT_VIOLATION)
                    self.assertEqual(completed.book, inconsistent)

    def test_microstep_rejects_non_deadline_book_before_reading_limit(self):
        result = schedule_microstep(
            None, owner="cpu", identity="child", generation=1,
            at_common=Fraction(5), effect_id="child-effect",
            current_microstep=0,
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertIsNone(result.book)

    def test_publication_rejects_unhashable_domain_identity(self):
        state = replace(
            machine(),
            domains=(ClockDomain([], Fraction(1), horizon_local=Fraction(100)),),
        )
        book = DeadlineBook()
        result = publish(book, state)
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, book)

    def test_book_rejects_older_active_generation_after_newer_generation_exists(self):
        cases = (
            (DeadlineStatus.SCHEDULED, None, ()),
            (
                DeadlineStatus.CANCEL_PENDING,
                None,
                (Cancellation("cpu", "evt", 1, Fraction(4)),),
            ),
            (DeadlineStatus.CLAIMED, Fraction(5), ()),
            (DeadlineStatus.DISPATCHED, Fraction(5), ()),
        )
        for old_status, watermark, cancellations in cases:
            with self.subTest(old_status=old_status):
                inconsistent = DeadlineBook(
                    deadlines=(
                        Deadline(
                            "cpu", "evt", 1, Fraction(5), "effect:evt:1",
                            status=old_status,
                        ),
                        Deadline(
                            "cpu", "evt", 2, Fraction(6), "effect:evt:2",
                        ),
                    ),
                    pending_cancellations=cancellations,
                    watermark_common=watermark,
                )
                results = (
                    close_watermark(inconsistent, Fraction(6)),
                    dispatch_deadline(
                        inconsistent, identity="evt", generation=1
                    ),
                )
                for result in results:
                    self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
                    self.assertEqual(result.book, inconsistent)
                    self.assertEqual(result.claimed, ())


class R2T06DispatcherRaceClosureTests(unittest.TestCase):
    def test_admitted_predeadline_cancel_stays_suppressed_when_dispatcher_runs_later(self):
        published = publish(DeadlineBook(), machine(), at=10)
        admitted = cancel_deadline(
            published.book, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(4),
        )
        self.assertEqual(admitted.code, DeadlineCode.ACCEPTED)
        self.assertEqual(close_watermark(admitted.book, Fraction(10)).code, DeadlineCode.BLOCKED)
        arbitrated = arbitrate_cancellations(admitted.book)
        closed = close_watermark(arbitrated.book, Fraction(10))
        self.assertEqual(closed.code, DeadlineCode.ACCEPTED)
        delayed_dispatch = dispatch_deadline(closed.book, identity="evt", generation=1)
        self.assertEqual(delayed_dispatch.code, DeadlineCode.STALE_IGNORED)
        self.assertEqual(delayed_dispatch.book.deadlines[0].status, DeadlineStatus.CANCELLED)
        self.assertEqual(delayed_dispatch.claimed, ())

    def test_postdeadline_cancel_cannot_suppress_claim_before_delayed_dispatch(self):
        published = publish(DeadlineBook(), machine(), at=10)
        claimed = close_watermark(published.book, Fraction(10))
        self.assertEqual(claimed.claimed, (("evt", 1),))
        late_cancel = cancel_deadline(
            claimed.book, machine(progress=2), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(11),
        )
        self.assertEqual(late_cancel.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(late_cancel.book, claimed.book)
        dispatched = dispatch_deadline(claimed.book, identity="evt", generation=1)
        self.assertEqual(dispatched.code, DeadlineCode.ACCEPTED)
        completed = complete_deadline(dispatched.book, identity="evt", generation=1)
        self.assertEqual(completed.code, DeadlineCode.ACCEPTED)
        self.assertEqual(completed.book.deadlines[0].status, DeadlineStatus.COMPLETED)
