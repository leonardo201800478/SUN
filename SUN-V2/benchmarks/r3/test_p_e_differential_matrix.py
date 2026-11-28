"""Expanded P-E/P-A W03/W04 differential cases.

P-A remains the serial full-scan comparator. P-E remains independent and
uses explicit pairwise constraints for commutative simultaneous transitions.
These are synthetic contract tests, not physical SMS timing evidence.
"""
from __future__ import annotations

import sys
import unittest
from itertools import combinations, permutations
from pathlib import Path
from fractions import Fraction

from p_a_causal_sync import (
    ACausalCode,
    ACausalState,
    ADevice,
    ATransition,
    synchronize as p_a_synchronize,
)
R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from causal_reference import (
    CausalState as R2CausalState,
    Device as R2Device,
    SyncCode as R2SyncCode,
    Transition as R2Transition,
    synchronize as r2_synchronize,
)

from p_e_sync_on_demand import (
    PairRelation,
    SyncCode,
    SyncConstraint,
    SyncDomain,
    SyncState,
    SyncTransition,
    synchronize as p_e_synchronize,
)


def _pa_progress(state):
    return tuple((item.name, item.progress_common) for item in state.devices)


def _pe_progress(state):
    return tuple((item.identity, item.progress) for item in state.domains)


class PEADifferentialMatrixTests(unittest.TestCase):
    def test_noncommutative_same_instant_tie_is_indeterminate_without_mutation(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice("d0", Fraction(0), (
                ATransition("write-d0", Fraction(5)),
            )),
            ADevice("d1", Fraction(0), (
                ATransition("write-d1", Fraction(5)),
            )),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(0), Fraction(100)),
                SyncDomain("d0", Fraction(0), Fraction(100)),
                SyncDomain("d1", Fraction(0), Fraction(100)),
            ),
            (
                SyncTransition("write-d0", "d0", Fraction(5)),
                SyncTransition("write-d1", "d1", Fraction(5)),
            ),
        )

        reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("d0", "d1"),
        )
        candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "d0", "d1"),
            target_common=Fraction(5),
        )
        self.assertEqual(reference.code, ACausalCode.INDETERMINATE)
        self.assertEqual(candidate.code, SyncCode.INDETERMINATE)
        self.assertEqual(reference.state, pa_state)
        self.assertEqual(candidate.state, pe_state)
        self.assertEqual(_pa_progress(reference.state), _pe_progress(candidate.state))

    def test_pairwise_commutative_same_instant_tie_matches_canonical_p_a_order(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice("d0", Fraction(0), (
                ATransition("write-z", Fraction(5), commutative=True),
            )),
            ADevice("d1", Fraction(0), (
                ATransition("write-a", Fraction(5), commutative=True),
            )),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(0), Fraction(100)),
                SyncDomain("d0", Fraction(0), Fraction(100)),
                SyncDomain("d1", Fraction(0), Fraction(100)),
            ),
            (
                SyncTransition("write-z", "d0", Fraction(5)),
                SyncTransition("write-a", "d1", Fraction(5)),
            ),
            (
                SyncConstraint("write-z", "write-a", PairRelation.COMMUTATIVE),
            ),
        )

        reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("d0", "d1"),
        )
        candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "d0", "d1"),
            target_common=Fraction(5),
        )
        self.assertEqual(reference.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(candidate.code, SyncCode.REACHED)
        self.assertEqual(candidate.state.observed, reference.applied_order)
        self.assertEqual(reference.applied_order, ("write-a", "write-z"))
        self.assertEqual(_pa_progress(reference.state), _pe_progress(candidate.state))
        self.assertTrue(all(value == Fraction(5) for _, value in _pa_progress(reference.state)))

    def test_unknown_requested_participant_blocks_without_partial_progress(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice("d0", Fraction(0)),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(0), Fraction(100)),
                SyncDomain("d0", Fraction(0), Fraction(100)),
            ),
            (),
        )

        reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("missing",),
        )
        candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "missing"),
            target_common=Fraction(5),
        )
        self.assertEqual(reference.code, ACausalCode.BLOCKED)
        self.assertEqual(candidate.code, SyncCode.BLOCKED)
        self.assertEqual(reference.state, pa_state)
        self.assertEqual(candidate.state, pe_state)
        self.assertEqual(_pa_progress(reference.state), _pe_progress(candidate.state))

    def test_retrograde_observation_is_late_without_mutation(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(10)),
            ADevice("d0", Fraction(10)),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(10), Fraction(100)),
                SyncDomain("d0", Fraction(10), Fraction(100)),
            ),
            (),
        )

        reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("d0",),
        )
        candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "d0"),
            target_common=Fraction(5),
        )
        self.assertEqual(reference.code, ACausalCode.LATE_INPUT)
        self.assertEqual(candidate.code, SyncCode.LATE_INPUT)
        self.assertEqual(reference.state, pa_state)
        self.assertEqual(candidate.state, pe_state)
        self.assertEqual(_pa_progress(reference.state), _pe_progress(candidate.state))


    def test_explicit_same_instant_dependency_orders_noncommutative_transitions(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice("d0", Fraction(0), (
                ATransition("first-write", Fraction(5)),
            )),
            ADevice("d1", Fraction(0), (
                ATransition("second-write", Fraction(5), requires=("first-write",)),
            )),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(0), Fraction(100)),
                SyncDomain("d0", Fraction(0), Fraction(100)),
                SyncDomain("d1", Fraction(0), Fraction(100)),
            ),
            (
                SyncTransition("first-write", "d0", Fraction(5)),
                SyncTransition("second-write", "d1", Fraction(5),
                               depends_on=("first-write",)),
            ),
        )
        reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("d0", "d1"),
        )
        candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "d0", "d1"),
            target_common=Fraction(5),
        )
        self.assertEqual(reference.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(candidate.code, SyncCode.REACHED)
        self.assertEqual(reference.applied_order, ("first-write", "second-write"))
        self.assertEqual(candidate.state.observed, reference.applied_order)
        self.assertEqual(_pa_progress(reference.state), _pe_progress(candidate.state))

    def test_repeated_observation_is_idempotent_and_unrelated_domain_is_unchanged(self):
        pa_state = ACausalState((
            ADevice("cpu", Fraction(0)),
            ADevice("d0", Fraction(0), (
                ATransition("d0-observation", Fraction(5)),
            )),
            ADevice("unrelated", Fraction(0), (
                ATransition("unrelated-event", Fraction(4)),
            )),
        ))
        pe_state = SyncState(
            (
                SyncDomain("cpu", Fraction(0), Fraction(100)),
                SyncDomain("d0", Fraction(0), Fraction(100)),
                SyncDomain("unrelated", Fraction(0), Fraction(100)),
            ),
            (
                SyncTransition("d0-observation", "d0", Fraction(5)),
                SyncTransition("unrelated-event", "unrelated", Fraction(4)),
            ),
        )
        first_reference = p_a_synchronize(
            pa_state, target_device="cpu", at_common=Fraction(5),
            observes=("d0",),
        )
        first_candidate = p_e_synchronize(
            pe_state, requested_domains=("cpu", "d0"),
            target_common=Fraction(5),
        )
        self.assertEqual(first_reference.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(first_candidate.code, SyncCode.REACHED)
        self.assertEqual(_pa_progress(first_reference.state), _pe_progress(first_candidate.state))
        self.assertEqual(
            next(item.progress_common for item in first_reference.state.devices
                 if item.name == "unrelated"),
            Fraction(0),
        )
        self.assertEqual(
            next(item.progress for item in first_candidate.state.domains
                 if item.identity == "unrelated"),
            Fraction(0),
        )

        repeated_reference = p_a_synchronize(
            first_reference.state, target_device="cpu", at_common=Fraction(5),
            observes=("d0",),
        )
        repeated_candidate = p_e_synchronize(
            first_candidate.state, requested_domains=("cpu", "d0"),
            target_common=Fraction(5),
        )
        self.assertEqual(repeated_reference.code, ACausalCode.SYNCHRONIZED)
        self.assertEqual(repeated_candidate.code, SyncCode.REACHED)
        self.assertEqual(repeated_reference.state, first_reference.state)
        self.assertEqual(repeated_candidate.state, first_candidate.state)
        self.assertEqual(_pa_progress(repeated_reference.state), _pe_progress(repeated_candidate.state))


    def _compare_all_three_models_for_permutation(
        self, events, *, constraints=(), expected_order=(), indeterminate=False
    ):
        pa_state = ACausalState(tuple(
            ADevice(
                owner, Fraction(0),
                (ATransition(identity, Fraction(5), dependencies, commutative),),
            )
            for identity, owner, dependencies, commutative in events
        ))
        pe_state = SyncState(
            tuple(
                SyncDomain(f"d{index}", Fraction(0), Fraction(100))
                for index in range(5)
            ),
            tuple(
                SyncTransition(
                    identity, owner, Fraction(5), depends_on=dependencies
                )
                for identity, owner, dependencies, _ in events
            ),
            tuple(constraints),
        )
        r2_state = R2CausalState(tuple(
            R2Device(
                owner, Fraction(0),
                (R2Transition(identity, Fraction(5), dependencies, commutative),),
            )
            for identity, owner, dependencies, commutative in events
        ))
        participants = ("d0", "d1", "d2", "d3", "d4")
        pa = p_a_synchronize(
            pa_state, target_device="d0", at_common=Fraction(5),
            observes=("d1", "d2", "d3", "d4"),
        )
        pe = p_e_synchronize(
            pe_state, requested_domains=participants,
            target_common=Fraction(5),
        )
        r2 = r2_synchronize(
            r2_state, target_device="d0", at_common=Fraction(5),
            observes=("d1", "d2", "d3", "d4"),
        )
        if indeterminate:
            self.assertEqual(pa.code, ACausalCode.INDETERMINATE)
            self.assertEqual(pe.code, SyncCode.INDETERMINATE)
            self.assertEqual(r2.code, R2SyncCode.INDETERMINATE)
            self.assertEqual(pa.state, pa_state)
            self.assertIs(pe.state, pe_state)
            self.assertEqual(r2.state, r2_state)
            self.assertEqual(pa.applied_order, ())
            self.assertEqual(pe.state.observed, ())
            self.assertEqual(r2.applied_order, ())
        else:
            self.assertEqual(pa.code, ACausalCode.SYNCHRONIZED)
            self.assertEqual(pe.code, SyncCode.REACHED)
            self.assertEqual(r2.code, R2SyncCode.SYNCHRONIZED)
            self.assertEqual(pa.applied_order, expected_order)
            self.assertEqual(pe.state.observed, expected_order)
            self.assertEqual(r2.applied_order, expected_order)
        pa_progress = dict(_pa_progress(pa.state))
        pe_progress = dict(_pe_progress(pe.state))
        r2_progress = {item.name: item.progress_common for item in r2.state.devices}
        self.assertEqual(pa_progress, pe_progress)
        self.assertEqual(pa_progress, r2_progress)

    def test_120_commutative_five_event_permutations_match_p_a_and_r2(self):
        events = tuple(
            (f"e{index}", f"d{index}", (), True)
            for index in range(5)
        )
        constraints = tuple(
            SyncConstraint(first[0], second[0], PairRelation.COMMUTATIVE)
            for first, second in combinations(events, 2)
        )
        expected = tuple(sorted(item[0] for item in events))
        for ordered in permutations(events):
            self._compare_all_three_models_for_permutation(
                ordered, constraints=constraints, expected_order=expected
            )

    def test_120_noncommutative_unknown_five_event_permutations_match_p_a_and_r2(self):
        events = tuple(
            (f"e{index}", f"d{index}", (), False)
            for index in range(5)
        )
        for ordered in permutations(events):
            self._compare_all_three_models_for_permutation(
                ordered, indeterminate=True
            )

    def test_120_explicit_dependency_five_event_permutations_match_p_a_and_r2(self):
        events = tuple(
            (
                f"e{index}", f"d{index}",
                () if index == 0 else (f"e{index - 1}",),
                False,
            )
            for index in range(5)
        )
        expected = tuple(item[0] for item in events)
        for ordered in permutations(events):
            self._compare_all_three_models_for_permutation(
                ordered, expected_order=expected
            )

    def test_120_partial_order_five_event_permutations_match_p_a_and_r2(self):
        events = (
            ("e0", "d0", (), False),
            ("e1", "d1", ("e0",), True),
            ("e2", "d2", ("e0",), True),
            ("e3", "d3", ("e1", "e2"), False),
            ("e4", "d4", ("e3",), False),
        )
        constraints = (
            SyncConstraint("e1", "e2", PairRelation.COMMUTATIVE),
        )
        expected = ("e0", "e1", "e2", "e3", "e4")
        for ordered in permutations(events):
            self._compare_all_three_models_for_permutation(
                ordered, constraints=constraints, expected_order=expected
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
