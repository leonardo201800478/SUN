"""P-A W07 microstep-cycle differential tests against the R2 oracle."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from itertools import combinations, permutations
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_microsteps import (
    AStepCode,
    AStepCycle,
    AStepObligation,
    AStepPairConstraint,
    AStepPairRelation,
    close_cycle,
    make_cycle,
    process_obligation,
)
from trace_fixture_support import record_differential
from benchmark_statistics import (
    MAX_CPU_CLOCK_RESOLUTION_NS,
    cpu_clock_is_usable,
    percentile_type7,
)
from microstep_reference import (
    MicrostepCode as R2Code,
    MicrostepCycle as R2Cycle,
    MicrostepObligation as R2Obligation,
    close_cycle as r2_close_cycle,
    make_cycle as r2_make_cycle,
    process_obligation as r2_process_obligation,
)


def to_r2(state: AStepCycle) -> R2Cycle:
    return R2Cycle(
        state.at_common,
        state.max_microsteps,
        state.next_microstep,
        tuple(
            R2Obligation(item.identity, item.owner, item.microstep, item.commutative)
            for item in state.pending
        ),
        state.resolved,
    )


def canonical(state: AStepCycle):
    return (
        state.at_common,
        state.max_microsteps,
        state.next_microstep,
        tuple(
            (item.identity, item.owner, item.microstep, item.commutative)
            for item in state.pending
        ),
        state.resolved,
    )


class BenchmarkStatisticsTests(unittest.TestCase):
    def test_type7_quantile_interpolates_and_preserves_extrema(self):
        values = [10, 0]
        self.assertEqual(percentile_type7(values, 0.0), 0.0)
        self.assertEqual(percentile_type7(values, 0.5), 5.0)
        self.assertEqual(percentile_type7(values, 0.95), 9.5)
        self.assertEqual(percentile_type7(values, 1.0), 10.0)

    def test_type7_rejects_empty_or_invalid_quantile(self):
        with self.assertRaises(ValueError):
            percentile_type7([], 0.5)
        with self.assertRaises(ValueError):
            percentile_type7([1, 2], 1.1)

    def test_cpu_clock_resolution_gate_is_conservative(self):
        self.assertTrue(cpu_clock_is_usable(MAX_CPU_CLOCK_RESOLUTION_NS))
        self.assertFalse(cpu_clock_is_usable(MAX_CPU_CLOCK_RESOLUTION_NS + 1))
        self.assertFalse(cpu_clock_is_usable(None))
        self.assertFalse(cpu_clock_is_usable(0))


class PAMicrostepTests(unittest.TestCase):
    def compare(self, actual, expected):
        expected_state = (
            expected.state.at_common,
            expected.state.max_microsteps,
            expected.state.next_microstep,
            tuple(
                (item.obligation_id, item.owner, item.microstep, item.commutative)
                for item in expected.state.pending
            ),
            expected.state.resolved,
        )
        self.assertEqual(actual.code.value, expected.code.value)
        self.assertEqual(canonical(actual.state), expected_state)
        record_differential(
            self.id(), (actual.code.value, canonical(actual.state)),
            (expected.code.value, expected_state),
        )

    def make_pair(self, at_common, obligations, max_microsteps, *, future=(), relations=()):
        actual = make_cycle(
            at_common, obligations, max_microsteps=max_microsteps,
            future=future, relations=relations,
        )
        expected = r2_make_cycle(
            at_common, obligations, max_microsteps=max_microsteps
        )
        self.compare(actual, expected)
        return actual

    def test_child_is_scheduled_at_next_microstep_and_parent_is_resolved(self):
        actual = self.make_pair(
            Fraction(10), (("root", "cpu", False),), 4
        )
        expected_state = to_r2(actual.state)
        actual = process_obligation(
            actual.state, "root", resolves_obligation=True,
            emitted=(("child", "vdp", False),),
        )
        expected = r2_process_obligation(
            expected_state, "root", resolves_obligation=True,
            emitted=(("child", "vdp", False),),
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, AStepCode.ADVANCED)
        self.assertEqual(actual.state.resolved, ("root",))
        self.assertEqual(actual.state.pending[0].microstep, 1)

        actual = process_obligation(
            actual.state, "child", resolves_obligation=True
        )
        expected = r2_process_obligation(
            expected.state, "child", resolves_obligation=True
        )
        self.compare(actual, expected)
        self.assertEqual(actual.code, AStepCode.COMPLETE)
        self.assertEqual(actual.state.pending, ())
        self.assertEqual(actual.state.resolved, ("child", "root"))
        self.compare(close_cycle(actual.state), r2_close_cycle(expected.state))

    def test_noop_repetition_is_blocked_without_progress(self):
        actual = self.make_pair(
            Fraction(10), (("root", "cpu", False),), 4
        )
        before = actual.state
        expected_before = to_r2(before)
        first = process_obligation(
            before, "root", resolves_obligation=False
        )
        expected_first = r2_process_obligation(
            expected_before, "root", resolves_obligation=False
        )
        self.compare(first, expected_first)
        repeated = process_obligation(
            first.state, "root", resolves_obligation=False
        )
        expected_repeated = r2_process_obligation(
            expected_first.state, "root", resolves_obligation=False
        )
        self.compare(repeated, expected_repeated)
        self.assertEqual(first.code, AStepCode.BLOCKED)
        self.assertEqual(repeated.code, AStepCode.BLOCKED)
        self.assertEqual(first.state, before)
        self.assertEqual(repeated.state, before)

    def test_emission_cycle_exhausts_limit_atomically(self):
        actual = self.make_pair(
            Fraction(10), (("loop-0", "cpu", False),), 2
        )
        expected_state = to_r2(actual.state)
        for index in range(2):
            identity = f"loop-{index}"
            actual = process_obligation(
                actual.state, identity, resolves_obligation=True,
                emitted=((f"loop-{index + 1}", "cpu", False),),
            )
            expected = r2_process_obligation(
                expected_state, identity, resolves_obligation=True,
                emitted=((f"loop-{index + 1}", "cpu", False),),
            )
            self.compare(actual, expected)
            self.assertEqual(actual.code, AStepCode.ADVANCED)
            expected_state = expected.state

        before = actual.state
        exhausted = process_obligation(
            before, "loop-2", resolves_obligation=True,
            emitted=(("loop-3", "cpu", False),),
        )
        expected_exhausted = r2_process_obligation(
            expected_state, "loop-2", resolves_obligation=True,
            emitted=(("loop-3", "cpu", False),),
        )
        self.compare(exhausted, expected_exhausted)
        self.assertEqual(exhausted.code, AStepCode.RESOURCE_EXHAUSTED)
        self.assertEqual(exhausted.state, before)

    def test_independent_commutative_siblings_are_permutation_invariant(self):
        descriptors = tuple(
            (f"event-{index}", f"device-{index}", True) for index in range(5)
        )
        expected_final = None
        relations = tuple(
            AStepPairConstraint(first[0], second[0], AStepPairRelation.COMMUTATIVE)
            for first, second in combinations(descriptors, 2)
        )
        for order in permutations(descriptors):
            actual = self.make_pair(
                Fraction(10), order, 2, relations=relations
            )
            expected_state = to_r2(actual.state)
            for descriptor in order:
                actual = process_obligation(
                    actual.state, descriptor[0], resolves_obligation=True
                )
                expected = r2_process_obligation(
                    expected_state, descriptor[0], resolves_obligation=True
                )
                self.compare(actual, expected)
                expected_state = expected.state
            closed = close_cycle(actual.state)
            closed_ref = r2_close_cycle(expected_state)
            self.compare(closed, closed_ref)
            self.assertEqual(closed.code, AStepCode.COMPLETE)
            if expected_final is None:
                expected_final = canonical(closed.state)
            self.assertEqual(canonical(closed.state), expected_final)

    def test_later_microstep_with_unknown_pair_relation_is_indeterminate(self):
        future = (AStepObligation("c", "psg", 1, True),)
        relations = (
            AStepPairConstraint("a", "b", AStepPairRelation.COMMUTATIVE),
        )
        initial = self.make_pair(
            Fraction(10), (("a", "cpu", True), ("b", "vdp", True)), 4,
            future=future, relations=relations,
        ).state
        r2_initial = to_r2(initial)
        parent = process_obligation(
            initial, "a", resolves_obligation=True,
            emitted=(("c", "psg", True),),
        )
        r2_parent = r2_process_obligation(
            r2_initial, "a", resolves_obligation=True,
            emitted=(("c", "psg", False),),
        )
        self.assertEqual(parent.code, AStepCode.ADVANCED)
        self.assertEqual(tuple((x.identity, x.microstep) for x in parent.state.pending),
                         (("b", 0), ("c", 1)))

        before = parent.state
        unresolved = process_obligation(before, "c", resolves_obligation=True)
        self.assertEqual(unresolved.code, AStepCode.INDETERMINATE)
        self.assertIs(unresolved.state, before)
        self.assertIn("b", tuple(item.identity for item in unresolved.state.pending))
        self.assertNotIn("c", unresolved.state.resolved)

        # R2 accepts the overtaking attempt; preserve the divergence without
        # claiming that either synthetic ordering is physical hardware evidence.
        r2_overtake = r2_process_obligation(
            r2_parent.state, "c", resolves_obligation=True
        )
        self.assertEqual(r2_overtake.code, R2Code.ADVANCED)

    def test_explicit_pairwise_commutativity_allows_later_microstep_to_overtake(self):
        future = (AStepObligation("c", "psg", 1, False),)
        relations = (
            AStepPairConstraint("a", "b", AStepPairRelation.COMMUTATIVE),
            AStepPairConstraint("b", "c", AStepPairRelation.COMMUTATIVE),
        )
        initial = self.make_pair(
            Fraction(10), (("a", "cpu", True), ("b", "vdp", True)), 4,
            future=future, relations=relations,
        ).state
        first = process_obligation(
            initial, "a", resolves_obligation=True,
            emitted=(("c", "psg", False),),
        )
        self.assertEqual(first.code, AStepCode.ADVANCED)
        overtaken = process_obligation(
            first.state, "c", resolves_obligation=True
        )
        self.assertEqual(overtaken.code, AStepCode.ADVANCED)
        self.assertEqual(overtaken.state.resolved, ("a", "c"))
        self.assertEqual(tuple(item.identity for item in overtaken.state.pending), ("b",))
        finished = process_obligation(
            overtaken.state, "b", resolves_obligation=True
        )
        self.assertEqual(finished.code, AStepCode.COMPLETE)

    def test_explicit_pairwise_predecessor_blocks_later_microstep(self):
        future = (AStepObligation("c", "psg", 1, False),)
        relations = (
            AStepPairConstraint("a", "b", AStepPairRelation.COMMUTATIVE),
            AStepPairConstraint("b", "c", AStepPairRelation.FIRST_CAUSES_SECOND),
        )
        initial = self.make_pair(
            Fraction(10), (("a", "cpu", True), ("b", "vdp", True)), 4,
            future=future, relations=relations,
        ).state
        first = process_obligation(
            initial, "a", resolves_obligation=True,
            emitted=(("c", "psg", False),),
        )
        blocked = process_obligation(
            first.state, "c", resolves_obligation=True
        )
        self.assertEqual(blocked.code, AStepCode.BLOCKED)
        self.assertIs(blocked.state, first.state)
        predecessor = process_obligation(
            blocked.state, "b", resolves_obligation=True
        )
        self.assertEqual(predecessor.code, AStepCode.ADVANCED)
        child = process_obligation(
            predecessor.state, "c", resolves_obligation=True
        )
        self.assertEqual(child.code, AStepCode.COMPLETE)

    def test_pairwise_causality_cannot_contradict_microstep_assignment(self):
        result = make_cycle(
            Fraction(10),
            (("b", "vdp", True),),
            future=(AStepObligation("c", "psg", 1, True),),
            relations=(
                AStepPairConstraint(
                    "b", "c", AStepPairRelation.SECOND_CAUSES_FIRST
                ),
            ),
        )
        self.assertEqual(result.code, AStepCode.INVALID_INPUT)
        self.assertIn("contradicts", result.detail)

    def test_unknown_noncommutative_siblings_remain_indeterminate(self):
        actual = self.make_pair(
            Fraction(10),
            (("a", "cpu", False), ("b", "vdp", False)),
            2,
        )
        before = actual.state
        expected_before = to_r2(before)
        processed = process_obligation(
            before, "a", resolves_obligation=True
        )
        processed_ref = r2_process_obligation(
            expected_before, "a", resolves_obligation=True
        )
        closed = close_cycle(before)
        closed_ref = r2_close_cycle(expected_before)
        self.compare(processed, processed_ref)
        self.compare(closed, closed_ref)
        self.assertEqual(processed.code, AStepCode.INDETERMINATE)
        self.assertEqual(closed.code, AStepCode.INDETERMINATE)
        self.assertEqual(processed.state, before)
        self.assertEqual(closed.state, before)

    def test_invalid_emission_descriptors_are_atomic(self):
        actual = self.make_pair(
            Fraction(10), (("root", "cpu", False),), 2
        )
        before = actual.state
        invalid = process_obligation(
            before, "root", resolves_obligation=True,
            emitted=(("root", "vdp", False),),
        )
        self.assertEqual(invalid.code, AStepCode.INVALID_INPUT)
        self.assertEqual(invalid.state, before)
        unresolved = process_obligation(
            before, "root", resolves_obligation=False,
            emitted=(("child", "vdp", False),),
        )
        self.assertEqual(unresolved.code, AStepCode.INVALID_INPUT)
        self.assertEqual(unresolved.state, before)

    def test_depth_matrix_matches_r2_at_every_causal_cut(self):
        for depth in (1, 2, 4, 8, 16, 32):
            with self.subTest(depth=depth):
                actual = self.make_pair(
                    Fraction(10), (("root", "cpu", False),), depth
                )
                expected_state = to_r2(actual.state)
                current = "root"
                for index in range(depth):
                    child = f"child-{index + 1}"
                    actual = process_obligation(
                        actual.state, current, resolves_obligation=True,
                        emitted=((child, "cpu", False),),
                    )
                    expected = r2_process_obligation(
                        expected_state, current, resolves_obligation=True,
                        emitted=((child, "cpu", False),),
                    )
                    self.compare(actual, expected)
                    self.assertEqual(actual.code, AStepCode.ADVANCED)
                    expected_state = expected.state
                    current = child

                actual = process_obligation(
                    actual.state, current, resolves_obligation=True
                )
                expected = r2_process_obligation(
                    expected_state, current, resolves_obligation=True
                )
                self.compare(actual, expected)
                self.assertEqual(actual.code, AStepCode.COMPLETE)
                self.compare(close_cycle(actual.state), r2_close_cycle(expected.state))


if __name__ == "__main__":
    unittest.main(verbosity=2)
