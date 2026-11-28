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


    def test_validate_rejects_malformed_collection_shapes_without_mutation(self):
        valid = self.initial()
        malformed_states = (
            PairwiseCycle(10, list(valid.pending), valid.constraints),
            PairwiseCycle(10, valid.pending, list(valid.constraints)),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=["done"]),
            PairwiseCycle(10, valid.pending, valid.constraints, future=[Obligation("c", "psg", 1)]),
            PairwiseCycle(10, (object(),), valid.constraints),
            PairwiseCycle(10, valid.pending, (object(),)),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=([],)),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=("   ",)),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=("done", "done")),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=("a",)),
            PairwiseCycle(10, valid.pending, valid.constraints, resolved=("c",), future=valid.future),
            PairwiseCycle(10, valid.pending, valid.constraints,
                          future=(Obligation("c", "psg", 1), Obligation("c", "psg", 1))),
            PairwiseCycle(10, valid.pending, valid.constraints,
                          future=(Obligation("c", "psg", True),)),
            PairwiseCycle(10, valid.pending, valid.constraints,
                          future=(Obligation("c", "psg", -1),)),
            PairwiseCycle(10, valid.pending,
                          (PairConstraint("a", "b", "commutative"),)),
            PairwiseCycle(10, valid.pending,
                          (PairConstraint("a", "a", PairRelation.UNKNOWN),)),
            PairwiseCycle(10, valid.pending,
                          (PairConstraint([], "b", PairRelation.UNKNOWN),)),
            PairwiseCycle(10, valid.pending,
                          (PairConstraint("a", "missing", PairRelation.UNKNOWN),)),
        )
        for malformed in malformed_states:
            with self.subTest(malformed=malformed):
                result = process(malformed, "a")
                self.assertEqual(result.code, StepCode.INVALID_INPUT)
                self.assertIs(result.state, malformed)

    def test_validate_rejects_invalid_operation_collection_shapes_without_mutation(self):
        state = self.initial()
        for emitted in ([], {}, None, "not-a-tuple"):
            with self.subTest(emitted=emitted):
                result = process(state, "a", emitted=emitted)
                self.assertEqual(result.code, StepCode.INVALID_INPUT)
                self.assertIs(result.state, state)
        result = process(state, [], emitted=())
        self.assertEqual(result.code, StepCode.INVALID_INPUT)
        self.assertIs(result.state, state)


    def test_validate_rejects_malformed_cycle_pending_and_future_descriptors(self):
        valid = self.initial()
        malformed_states = (
            None,
            PairwiseCycle(True, valid.pending, valid.constraints),
            PairwiseCycle(10, (Obligation([], "cpu", 0),), ()),
            PairwiseCycle(10, (Obligation(" ", "cpu", 0),), ()),
            PairwiseCycle(10, (Obligation("a", [], 0),), ()),
            PairwiseCycle(10, (Obligation("a", " ", 0),), ()),
            PairwiseCycle(10, (Obligation("a", "cpu", True),), ()),
            PairwiseCycle(10, (Obligation("a", "cpu", -1),), ()),
            PairwiseCycle(10, (Obligation("a", "cpu", 0), Obligation("a", "vdp", 0)), ()),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation([], "psg", 1),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation(" ", "psg", 1),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation("x", [], 1),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation("x", " ", 1),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation("x", "psg", True),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(Obligation("x", "psg", -1),)),
            PairwiseCycle(10, valid.pending, (),
                          future=(object(),)),
            PairwiseCycle(10, valid.pending,
                          (PairConstraint("a", [], PairRelation.UNKNOWN),)),
        )
        for malformed in malformed_states:
            with self.subTest(malformed=malformed):
                result = process(malformed, "a")
                self.assertEqual(result.code, StepCode.INVALID_INPUT)
                self.assertIs(result.state, malformed)

    def test_finite_microstep_limit_accepts_boundary(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 32),),
            max_microsteps=32,
        )
        result = process(state, "a", emitted=(Obligation("c", "psg", 32),))
        self.assertEqual(result.code, StepCode.ADVANCED)
        self.assertIn(Obligation("c", "psg", 32), result.state.pending)

    def test_finite_microstep_limit_exhaustion_is_atomic(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 33),),
            max_microsteps=32,
        )
        result = process(state, "a", emitted=(Obligation("c", "psg", 33),))
        self.assertEqual(result.code, StepCode.RESOURCE_EXHAUSTED)
        self.assertIs(result.state, state)

    def test_transition_preserves_non_default_microstep_limit_and_enforces_it_later(self):
        state = PairwiseCycle(
            10,
            (Obligation("a", "cpu", 0), Obligation("b", "vdp", 0)),
            (PairConstraint("a", "b", PairRelation.COMMUTATIVE),),
            future=(Obligation("c", "psg", 1), Obligation("d", "ym2413", 2)),
            max_microsteps=2,
        )
        advanced = process(state, "a", emitted=(Obligation("c", "psg", 1),))
        self.assertEqual(advanced.code, StepCode.ADVANCED)
        self.assertEqual(advanced.state.max_microsteps, 2)
        self.assertEqual(advanced.state.future, (Obligation("d", "ym2413", 2),))

        # The next valid transition reaches the configured boundary, not default 32.
        boundary_state = PairwiseCycle(
            10,
            (Obligation("b", "vdp", 0),),
            (),
            future=(Obligation("d", "ym2413", 2),),
            max_microsteps=2,
        )
        boundary = process(
            boundary_state, "b", emitted=(Obligation("d", "ym2413", 2),)
        )
        self.assertEqual(boundary.code, StepCode.ADVANCED)
        self.assertEqual(boundary.state.max_microsteps, 2)

        over_limit = PairwiseCycle(
            10,
            (Obligation("b", "vdp", 0),),
            (),
            future=(Obligation("e", "ym2413", 3),),
            max_microsteps=2,
        )
        exhausted = process(
            over_limit, "b", emitted=(Obligation("e", "ym2413", 3),)
        )
        self.assertEqual(exhausted.code, StepCode.RESOURCE_EXHAUSTED)
        self.assertIs(exhausted.state, over_limit)

    def test_invalid_finite_microstep_limits_are_rejected_without_mutation(self):
        valid = self.initial()
        for limit in (0, -1, True):
            malformed = PairwiseCycle(
                valid.at_common, valid.pending, valid.constraints,
                valid.resolved, valid.future, limit,
            )
            result = process(malformed, "a")
            self.assertEqual(result.code, StepCode.INVALID_INPUT)
            self.assertIs(result.state, malformed)

if __name__ == "__main__":
    unittest.main(verbosity=2)
