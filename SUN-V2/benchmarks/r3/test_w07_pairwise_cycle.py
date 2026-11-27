"""Discriminating W07 cycle tests for explicit pairwise causal relations."""
from __future__ import annotations

import unittest

from w07_pairwise_cycle import (
    Obligation, PairConstraint, PairRelation, PairwiseCycle, StepCode, process,
)


class W07PairwiseCycleTests(unittest.TestCase):
    def initial(self, bc_relation=PairRelation.UNKNOWN):
        return PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),
             PairConstraint("b", "c", bc_relation)),
            future=(Obligation("c", "psg", 1),),
        )

    def after_a(self, relation=PairRelation.UNKNOWN):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),
             PairConstraint("b", "c", relation)),
            future=(Obligation("c", "psg", 1),),
        )
        result = process(state, "a", emitted=(Obligation("c", "psg", 1),))
        self.assertEqual(result.code, StepCode.ADVANCED)
        return result.state

    def test_unknown_b_c_makes_both_choices_indeterminate_without_mutation(self):
        state = self.after_a()
        for requested in ("b", "c"):
            with self.subTest(requested=requested):
                result = process(state, requested)
                self.assertEqual(result.code, StepCode.INDETERMINATE)
                self.assertIs(result.state, state)
                self.assertEqual(result.state.pending, (
                    Obligation("b", "vdp", 0), Obligation("c", "psg", 1)
                ))

    def test_b_causes_c_requires_b_first(self):
        state = self.after_a(PairRelation.FIRST_CAUSES_SECOND)
        blocked = process(state, "c")
        self.assertEqual(blocked.code, StepCode.BLOCKED)
        self.assertIs(blocked.state, state)
        advanced = process(state, "b")
        self.assertEqual(advanced.code, StepCode.ADVANCED)
        self.assertEqual([item.identity for item in advanced.state.pending], ["c"])

    def test_c_causes_b_conflicting_microstep_assignment_is_invalid(self):
        state = PairwiseCycle(
            10,
            (Obligation("b", "vdp", 0),),
            (PairConstraint("b", "c", PairRelation.SECOND_CAUSES_FIRST),),
            future=(Obligation("c", "psg", 1),),
        )
        result = process(state, "b")
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_c_causes_b_when_microstep_assignment_is_consistent(self):
        state = PairwiseCycle(
            10,
            (Obligation("b", "vdp", 1), Obligation("c", "psg", 0)),
            (PairConstraint("b", "c", PairRelation.SECOND_CAUSES_FIRST),),
        )
        advanced = process(state, "c")
        self.assertEqual(advanced.code, StepCode.ADVANCED)
        self.assertEqual([item.identity for item in advanced.state.pending], ["b"])

    def test_commutative_pair_respects_declared_microstep_sequence(self):
        state = self.after_a(PairRelation.COMMUTATIVE)
        later = process(state, "c")
        self.assertEqual(later.code, StepCode.BLOCKED)
        self.assertIs(later.state, state)
        earlier = process(state, "b")
        self.assertEqual(earlier.code, StepCode.ADVANCED)

    def test_same_microstep_commutative_pair_allows_either_order(self):
        state = PairwiseCycle(
            10,
            (Obligation("b", "vdp", 0), Obligation("c", "psg", 0)),
            (PairConstraint("b", "c", PairRelation.COMMUTATIVE),),
        )
        for requested in ("b", "c"):
            with self.subTest(requested=requested):
                result = process(state, requested)
                self.assertEqual(result.code, StepCode.ADVANCED)

    def test_initial_siblings_require_explicit_commutativity(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (),
        )
        result = process(state, "a")
        self.assertEqual(result.code, StepCode.INDETERMINATE)
        self.assertIs(result.state, state)


    def test_unknown_relation_endpoint_is_invalid(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("b", "typo-c", PairRelation.UNKNOWN),),
        )
        result = process(state, "a")
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_emitted_obligation_must_be_declared_future(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
        )
        result = process(state, "a", emitted=(Obligation("c", "psg", 1),))
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_emitted_obligation_must_match_future_declaration(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 1),),
        )
        result = process(state, "a", emitted=(Obligation("c", "cpu", 2),))
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_emitted_child_cannot_be_inserted_at_or_before_current_microstep(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 0),),
        )
        result = process(state, "a", emitted=(Obligation("c", "psg", 0),))
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_future_declaration_identity_cannot_alias_pending(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (),
            future=(Obligation("a", "psg", 1),),
        )
        result = process(state, "a")
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_invalid_duplicate_pair_is_rejected_without_mutation(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),
             PairConstraint("b", "a", PairRelation.COMMUTATIVE)),
        )
        result = process(state, "a")
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)


    def test_malformed_emitted_descriptors_return_invalid_without_exception(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 1),),
        )
        malformed = (
            Obligation([], "psg", 1),
            Obligation({}, "psg", 1),
            Obligation(None, "psg", 1),
            Obligation("   ", "psg", 1),
            Obligation("c", None, 1),
            Obligation("c", "   ", 1),
            Obligation("c", "psg", True),
            Obligation("c", "psg", -1),
            object(),
        )
        for child in malformed:
            with self.subTest(child=child):
                result = process(state, "a", emitted=(child,))
                self.assertEqual(result.code, StepCode.INVALID_INPUT)
                self.assertIs(result.state, state)

    def test_duplicate_emitted_identities_return_invalid_without_mutation(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 1),),
        )
        child = Obligation("c", "psg", 1)
        result = process(state, "a", emitted=(child, child))
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)

if __name__ == "__main__":
    unittest.main(verbosity=2)
