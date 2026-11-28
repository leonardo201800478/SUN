"""Expanded P-E/P-A W03/W04 differential cases.

P-A remains the serial full-scan comparator. P-E remains independent and
uses explicit pairwise constraints for commutative simultaneous transitions.
These are synthetic contract tests, not physical SMS timing evidence.
"""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_a_causal_sync import (
    ACausalCode,
    ACausalState,
    ADevice,
    ATransition,
    synchronize as p_a_synchronize,
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
