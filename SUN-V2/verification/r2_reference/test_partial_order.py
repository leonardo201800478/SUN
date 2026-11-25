"""R2-T07 UNKNOWN-order and explicit causal partial-order regressions."""
import unittest
from itertools import permutations
from fractions import Fraction

from chronos_reference import ClockDomain, make_state
from deadline_reference import (
    DeadlineBook, DeadlineCode, cancel_deadline, close_watermark,
    publish_deadline,
)


def machine():
    return make_state(
        profile="SMS-NTSC-ABSTRACT",
        domains=(ClockDomain("cpu", Fraction(1), progress_local=Fraction(2), horizon_local=Fraction(100)),),
        expected_participants=("cpu",),
    )


class R2T07PartialOrderTests(unittest.TestCase):
    def test_phase_order_without_matching_evidence_does_not_create_precedence(self):
        published = publish_deadline(
            DeadlineBook(), machine(), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(10), effect_id="effect:evt:1",
            phase_order=2, phase_evidence="capture-A",
        )
        self.assertEqual(published.code, DeadlineCode.ACCEPTED)
        cancellation = cancel_deadline(
            published.book, machine(), owner="cpu", identity="evt",
            generation=1, at_common=Fraction(10), phase_order=1,
            phase_evidence="capture-B",
        )
        self.assertEqual(cancellation.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(cancellation.book, published.book)

    def test_unknown_noncommutative_tie_never_closes_watermark_under_insertion_permutation(self):
        def attempt(order):
            book = DeadlineBook()
            for identity in order:
                result = publish_deadline(
                    book, machine(), owner="cpu", identity=identity,
                    generation=1, at_common=Fraction(10),
                    effect_id=f"effect:{identity}:1", commutative=False,
                )
                self.assertEqual(result.code, DeadlineCode.ACCEPTED)
                book = result.book
            before = book
            result = close_watermark(book, Fraction(10))
            self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
            self.assertEqual(result.book, before)
            self.assertIsNone(result.book.watermark_common)
            self.assertEqual(result.claimed, ())
            return tuple(sorted((item.identity, item.status) for item in result.book.deadlines))

        self.assertEqual(attempt(("a", "b")), attempt(("b", "a")))


    def test_all_120_permutations_of_five_commutative_events_are_equivalent(self):
        events = tuple(
            device(
                f"d{index}",
                transitions=(Transition(f"e{index}", Fraction(5), commutative=True),),
            )
            for index in range(5)
        )
        expected = tuple(f"e{index}" for index in range(5))
        for ordering in permutations(events):
            result = synchronize(
                state(*ordering),
                target_device="d0",
                at_common=Fraction(5),
                observes=("d1", "d2", "d3", "d4"),
            )
            self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
            self.assertEqual(result.applied_order, expected)

    def test_all_120_permutations_of_five_noncommutative_unknown_events_stay_indeterminate(self):
        events = tuple(
            device(
                f"d{index}",
                transitions=(Transition(f"e{index}", Fraction(5)),),
            )
            for index in range(5)
        )
        for ordering in permutations(events):
            initial = state(*ordering)
            result = synchronize(
                initial,
                target_device="d0",
                at_common=Fraction(5),
                observes=("d1", "d2", "d3", "d4"),
            )
            self.assertEqual(result.code, SyncCode.INDETERMINATE)
            self.assertEqual(result.state, initial)
            self.assertEqual(result.applied_order, ())

    def test_all_120_permutations_preserve_five_event_explicit_dependency_order(self):
        events = tuple(
            device(
                f"d{index}",
                transitions=(Transition(
                    f"e{index}",
                    Fraction(5),
                    () if index == 0 else (f"e{index - 1}",),
                ),),
            )
            for index in range(5)
        )
        expected = ("e0", "e1", "e2", "e3", "e4")
        for ordering in permutations(events):
            result = synchronize(
                state(*ordering),
                target_device="d0",
                at_common=Fraction(5),
                observes=("d1", "d2", "d3", "d4"),
            )
            self.assertEqual(result.code, SyncCode.SYNCHRONIZED)
            self.assertEqual(result.applied_order, expected)


if __name__ == "__main__":
    unittest.main(verbosity=2)
