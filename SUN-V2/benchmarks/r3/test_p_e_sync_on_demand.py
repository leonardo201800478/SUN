"""Tests for the disposable P-E sync-on-demand candidate."""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_e_sync_on_demand import (
    PairRelation,
    SyncCode,
    SyncConstraint,
    SyncDomain,
    SyncState,
    SyncTransition,
    synchronize,
)


class SyncOnDemandTests(unittest.TestCase):
    def test_unrelated_domains_are_not_advanced(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(20)),
                SyncDomain("vdp", Fraction(0), Fraction(20)),
                SyncDomain("psg", Fraction(0), Fraction(20)),
            ),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=Fraction(10)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        progress = {item.identity: item.progress for item in result.state.domains}
        self.assertEqual(progress["cpu"], Fraction(10))
        self.assertEqual(progress["vdp"], Fraction(0))
        self.assertEqual(progress["psg"], Fraction(0))
        self.assertEqual(result.synchronized_domains, ("cpu",))

    def test_transitive_dependency_closure_adds_only_required_domains(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(20)),
                SyncDomain("vdp", Fraction(0), Fraction(20)),
                SyncDomain("psg", Fraction(0), Fraction(20)),
            ),
            transitions=(
                SyncTransition("cpu-write", "cpu", Fraction(3)),
                SyncTransition("vdp-observe", "vdp", Fraction(4), ("cpu-write",)),
            ),
        )
        result = synchronize(
            state, requested_domains=("vdp",), target_common=Fraction(8)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        self.assertEqual(result.state.observed, ("cpu-write", "vdp-observe"))
        progress = {item.identity: item.progress for item in result.state.domains}
        self.assertEqual(progress["cpu"], Fraction(8))
        self.assertEqual(progress["vdp"], Fraction(8))
        self.assertEqual(progress["psg"], Fraction(0))
        self.assertEqual(result.synchronized_domains, ("cpu", "vdp"))

    def test_unknown_same_instant_pair_is_indeterminate_without_mutation(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("a", "cpu", Fraction(5)),
                SyncTransition("b", "vdp", Fraction(5)),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.INDETERMINATE)
        self.assertIs(result.state, state)

    def test_commutative_same_instant_pair_uses_stable_order(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("b", "vdp", Fraction(5)),
                SyncTransition("a", "cpu", Fraction(5)),
            ),
            constraints=(
                SyncConstraint("a", "b", PairRelation.COMMUTATIVE),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        self.assertEqual(result.state.observed, ("a", "b"))

    def test_explicit_same_instant_causality_orders_transitions(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("consumer", "vdp", Fraction(5)),
                SyncTransition("producer", "cpu", Fraction(5)),
            ),
            constraints=(
                SyncConstraint("producer", "consumer", PairRelation.FIRST_BEFORE_SECOND),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        self.assertEqual(result.state.observed, ("producer", "consumer"))

    def test_safe_horizon_stops_only_selected_causal_closure(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(4)),
                SyncDomain("vdp", Fraction(0), Fraction(7)),
                SyncDomain("psg", Fraction(0), Fraction(100)),
            ),
            transitions=(
                SyncTransition("cpu-event", "cpu", Fraction(3)),
                SyncTransition("vdp-event", "vdp", Fraction(6)),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(10)
        )
        self.assertEqual(result.code, SyncCode.STOPPED_AT)
        self.assertEqual(result.reached_common, Fraction(4))
        self.assertEqual(result.state.observed, ("cpu-event",))
        progress = {item.identity: item.progress for item in result.state.domains}
        self.assertEqual(progress["cpu"], Fraction(4))
        self.assertEqual(progress["vdp"], Fraction(4))
        self.assertEqual(progress["psg"], Fraction(0))

    def test_missing_dependency_is_invalid_before_any_progress(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(10)),),
            transitions=(
                SyncTransition("event", "cpu", Fraction(2), ("missing",)),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_unknown_requested_participant_blocks_without_mutation(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(10)),),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.BLOCKED)
        self.assertIs(result.state, state)

    def test_retrograde_target_is_late_input(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(5), Fraction(10)),),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=Fraction(4)
        )
        self.assertEqual(result.code, SyncCode.LATE_INPUT)
        self.assertIs(result.state, state)

    def test_fractional_target_is_exact(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(2)),),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=Fraction(1, 3)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        self.assertEqual(result.state.domains[0].progress, Fraction(1, 3))

    def test_float_target_is_rejected(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(2)),),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=0.5
        )
        self.assertEqual(result.code, SyncCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_same_instant_dependency_cycle_blocks_without_mutation(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("a", "cpu", Fraction(5), ("b",)),
                SyncTransition("b", "vdp", Fraction(5), ("a",)),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.BLOCKED)
        self.assertIs(result.state, state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
