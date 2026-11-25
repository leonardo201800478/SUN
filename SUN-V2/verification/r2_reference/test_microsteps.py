"""R2-T14 executable properties for finite same-instant obligation cycles."""
import unittest
from fractions import Fraction

from microstep_reference import (
    MicrostepCode,
    close_cycle,
    make_cycle,
    process_obligation,
)


class R2T14MicrostepCycleTests(unittest.TestCase):
    def test_repeated_microstep_without_resolving_obligation_is_blocked(self):
        initial = make_cycle(
            Fraction(10), (("root", "cpu", False),), max_microsteps=4
        )
        before = initial.state

        first = process_obligation(
            before, "root", resolves_obligation=False
        )
        repeated = process_obligation(
            first.state, "root", resolves_obligation=False
        )
        close = close_cycle(repeated.state)

        self.assertEqual(first.code, MicrostepCode.BLOCKED)
        self.assertEqual(repeated.code, MicrostepCode.BLOCKED)
        self.assertEqual(close.code, MicrostepCode.BLOCKED)
        self.assertEqual(first.state, before)
        self.assertEqual(repeated.state, before)
        self.assertEqual(close.state, before)

    def test_resolving_obligation_and_emitting_child_uses_next_microstep(self):
        initial = make_cycle(
            Fraction(10), (("root", "cpu", False),), max_microsteps=4
        )
        child = process_obligation(
            initial.state,
            "root",
            resolves_obligation=True,
            emitted=(("child", "vdp", False),),
        )
        child_event = next(
            item for item in child.state.pending
            if item.obligation_id == "child"
        )
        completed = process_obligation(
            child.state, "child", resolves_obligation=True
        )

        self.assertEqual(child.code, MicrostepCode.ADVANCED)
        self.assertEqual(child_event.microstep, 1)
        self.assertEqual(completed.code, MicrostepCode.COMPLETE)
        self.assertEqual(completed.state.pending, ())
        self.assertEqual(completed.state.resolved, ("child", "root"))
        self.assertEqual(close_cycle(completed.state).code, MicrostepCode.COMPLETE)

    def test_same_instant_emission_cycle_hits_finite_limit_atomically(self):
        initial = make_cycle(
            Fraction(10), (("loop-0", "cpu", False),), max_microsteps=2
        )
        first = process_obligation(
            initial.state, "loop-0", resolves_obligation=True,
            emitted=(("loop-1", "cpu", False),),
        )
        second = process_obligation(
            first.state, "loop-1", resolves_obligation=True,
            emitted=(("loop-2", "cpu", False),),
        )
        before_exhaustion = second.state
        exhausted = process_obligation(
            before_exhaustion, "loop-2", resolves_obligation=True,
            emitted=(("loop-3", "cpu", False),),
        )

        self.assertEqual(first.code, MicrostepCode.ADVANCED)
        self.assertEqual(second.code, MicrostepCode.ADVANCED)
        self.assertEqual(exhausted.code, MicrostepCode.RESOURCE_EXHAUSTED)
        self.assertEqual(exhausted.state, before_exhaustion)
        self.assertEqual(
            next(item for item in before_exhaustion.pending
                 if item.obligation_id == "loop-2").microstep,
            2,
        )

    def test_permuting_independent_commutative_obligations_is_invariant(self):
        initial = make_cycle(
            Fraction(10),
            (("a", "cpu", True), ("b", "vdp", True)),
            max_microsteps=2,
        )

        def finish(order):
            state = initial.state
            for obligation_id in order:
                result = process_obligation(
                    state, obligation_id, resolves_obligation=True
                )
                self.assertEqual(result.code, MicrostepCode.COMPLETE
                                 if obligation_id == order[-1]
                                 else MicrostepCode.ADVANCED)
                state = result.state
            return close_cycle(state)

        left = finish(("a", "b"))
        right = finish(("b", "a"))
        self.assertEqual(left.code, MicrostepCode.COMPLETE)
        self.assertEqual(right.code, MicrostepCode.COMPLETE)
        self.assertEqual(left.state, right.state)
        self.assertEqual(left.state.resolved, ("a", "b"))

    def test_unknown_noncommutative_same_microstep_cannot_be_ordered(self):
        initial = make_cycle(
            Fraction(10),
            (("a", "cpu", False), ("b", "vdp", False)),
            max_microsteps=2,
        )
        before = initial.state

        processed = process_obligation(
            before, "a", resolves_obligation=True
        )
        closed = close_cycle(before)

        self.assertEqual(processed.code, MicrostepCode.INDETERMINATE)
        self.assertEqual(closed.code, MicrostepCode.INDETERMINATE)
        self.assertEqual(processed.state, before)
        self.assertEqual(closed.state, before)


if __name__ == "__main__":
    unittest.main()
