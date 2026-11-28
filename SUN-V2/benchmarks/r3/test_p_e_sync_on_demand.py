"""Tests for the disposable P-E sync-on-demand candidate."""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_e_sync_on_demand import (
    PairRelation,
    SyncCode,
    SyncConstraint,
    SyncDomain,
    SyncPhaseBasis,
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

    def test_regressing_safe_horizon_is_rejected_without_mutation(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(5), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(3)),
            ),
            transitions=(),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(8)
        )
        self.assertEqual(result.code, SyncCode.INVALID_INPUT)
        self.assertIs(result.state, state)

    def test_unknown_observed_identity_is_rejected_without_exception(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(10)),),
            transitions=(SyncTransition("known", "cpu", Fraction(2)),),
            observed=("unknown",),
        )
        result = synchronize(
            state, requested_domains=("cpu",), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.INVALID_INPUT)
        self.assertIs(result.state, state)


    @staticmethod
    def phase_basis(basis_id: str) -> SyncPhaseBasis:
        return SyncPhaseBasis(
            profile_id="SMS-NTSC-SYNTHETIC",
            profile_revision="fixture-v1",
            coordinate="master-clock",
            unit="mclk",
            scale_numerator=1,
            scale_denominator=1,
            basis_id=basis_id,
            basis_revision="rev-1",
            validation_record_id="synthetic-record-" + basis_id,
            validation_record_sha256="b" * 64,
        )

    def test_w06_different_phase_bases_remain_unknown(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition(
                    "phase-a", "cpu", Fraction(5), phase_order=1,
                    phase_evidence="shared-synthetic-label",
                    phase_basis=self.phase_basis("basis-a"),
                ),
                SyncTransition(
                    "phase-b", "vdp", Fraction(5), phase_order=2,
                    phase_evidence="shared-synthetic-label",
                    phase_basis=self.phase_basis("basis-b"),
                ),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.INDETERMINATE)
        self.assertIs(result.state, state)

    def test_w06_common_phase_basis_is_a_synthetic_positive_control(self):
        basis = self.phase_basis("shared-basis")
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition(
                    "phase-a", "cpu", Fraction(5), phase_order=1,
                    phase_evidence="capture-a", phase_basis=basis,
                ),
                SyncTransition(
                    "phase-b", "vdp", Fraction(5), phase_order=2,
                    phase_evidence="capture-b", phase_basis=basis,
                ),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.REACHED)
        self.assertEqual(result.state.observed, ("phase-a", "phase-b"))

    def test_w07_unknown_b_c_relation_is_not_inferred_from_microstep(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
                SyncDomain("psg", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("a", "cpu", Fraction(5), microstep=0),
                SyncTransition("b", "vdp", Fraction(5), microstep=0),
                SyncTransition("c", "psg", Fraction(5), ("a",), microstep=1),
            ),
            constraints=(
                SyncConstraint("a", "b", PairRelation.COMMUTATIVE),
            ),
        )
        result = synchronize(
            state, requested_domains=("cpu", "vdp", "psg"), target_common=Fraction(5)
        )
        self.assertEqual(result.code, SyncCode.INDETERMINATE)
        self.assertIs(result.state, state)


    def test_cached_indexes_are_reused_across_observation_cuts(self):
        state = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("write", "cpu", Fraction(1)),
                SyncTransition("observe", "vdp", Fraction(2), ("write",)),
            ),
        )
        first = synchronize(
            state, requested_domains=("cpu", "vdp"), target_common=Fraction(1)
        )
        self.assertEqual(first.code, SyncCode.REACHED)
        self.assertTrue(first.state._validated)
        self.assertIs(first.state._transition_by_id, state._transition_by_id)
        self.assertIs(first.state._transitions_by_owner, state._transitions_by_owner)
        self.assertIs(first.state._transition_times_by_owner, state._transition_times_by_owner)
        second = synchronize(
            first.state, requested_domains=("cpu", "vdp"), target_common=Fraction(2)
        )
        self.assertEqual(second.code, SyncCode.REACHED)
        self.assertIs(second.state._transition_by_id, first.state._transition_by_id)
        self.assertIs(second.state._transition_times_by_owner, first.state._transition_times_by_owner)
        self.assertEqual(second.state.observed, ("write", "observe"))

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
