"""Deterministic P-A disposable prototype tests."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_instruction_catchup import (
    AObligation,
    AParticipant,
    AState,
    advance_budgeted,
    state_digest as p_a_digest,
)
from chronos_reference import (
    ClockDomain,
    Obligation,
    ResultKind,
    advance as r2_advance,
    make_state as r2_make_state,
)


class PATestCase(unittest.TestCase):
    def compare_case(self, state: AState, target: Fraction, chunk: Fraction):
        result = advance_budgeted(state, target=target, chunk=chunk)
        r2_state = r2_make_state(
            profile="R3-SYNTHETIC-ABSTRACT",
            domains=tuple(
                ClockDomain(
                    domain_id=item.name,
                    local_to_common=item.units_per_common,
                    progress_local=item.progress / item.units_per_common,
                    horizon_local=item.safe_horizon / item.units_per_common,
                )
                for item in state.participants
            ),
            expected_participants=(item.name for item in state.participants),
            obligations=tuple(
                Obligation(
                    obligation_id=item.identity,
                    owner=item.owner,
                    at_common=item.at_common,
                    resolved=item.resolved,
                    commutative=item.commutative,
                )
                for item in state.obligations
            ),
        )
        reference = r2_advance(r2_state, target)
        self.assertEqual(result.code, reference.result.kind.value)
        self.assertEqual(result.reached, reference.result.reached_common)
        self.assertEqual(
            self._progress_digest(result.next_state),
            self._reference_digest(reference.state),
        )
        self.assertEqual(
            result.next_state.participants,
            tuple(
                AParticipant(
                    domain.domain_id,
                    domain.progress_common(),
                    domain.horizon_common(),
                    domain.local_to_common,
                )
                for domain in reference.state.domains
            ),
        )
        if result.code == ResultKind.INDETERMINATE_CAUSAL_ORDER.value:
            self.assertEqual(result.next_state, state)
            self.assertEqual(result.reached, self._reference_frontier(reference.state))
        return result, reference

    @staticmethod
    def _progress_digest(state):
        return tuple(
            sorted(
                (item.name, item.progress.numerator, item.progress.denominator)
                for item in state.participants
            )
        )

    @staticmethod
    def _reference_digest(state):
        return tuple(
            sorted(
                (domain.domain_id, domain.progress_common().numerator,
                 domain.progress_common().denominator)
                for domain in state.domains
            )
        )

    @staticmethod
    def _reference_frontier(state):
        return max(domain.progress_common() for domain in state.domains)

    def test_sparse_budgeted_advance_matches_r2(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100), Fraction(1)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(3, 2)),
            )
        )
        result, _ = self.compare_case(state, Fraction(80), Fraction(8))
        self.assertEqual(result.code, ResultKind.REACHED.value)
        self.assertEqual(result.chunks, 10)
        self.assertEqual(result.obligations_inspected, 0)
        self.assertEqual(
            result.participants_inspected,
            result.chunks * len(state.participants),
        )

    def test_pending_obligation_stops_before_boundary(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(0), Fraction(20)),),
            obligations=(AObligation("irq", "cpu", Fraction(7)),),
        )
        result, _ = self.compare_case(state, Fraction(15), Fraction(4))
        self.assertEqual(result.code, ResultKind.STOPPED_AT.value)
        self.assertEqual(result.reached, Fraction(7))
        self.assertEqual(result.reason, "PendingObligation:irq")
        self.assertEqual(result.chunks, 2)
        self.assertEqual(result.obligations_inspected, 2)

    def test_unknown_simultaneous_obligations_do_not_advance(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(0), Fraction(20)),),
            obligations=(
                AObligation("irq", "cpu", Fraction(7), commutative=False),
                AObligation("dma", "cpu", Fraction(7), commutative=False),
            ),
        )
        result, _ = self.compare_case(state, Fraction(15), Fraction(4))
        self.assertEqual(result.code, ResultKind.INDETERMINATE_CAUSAL_ORDER.value)
        self.assertEqual(result.reached, Fraction(0))
        self.assertEqual(result.next_state, state)

    def test_safe_horizon_bounds_budget(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(9)),
                AParticipant("vdp", Fraction(0), Fraction(6)),
            )
        )
        result, _ = self.compare_case(state, Fraction(15), Fraction(4))
        self.assertEqual(result.code, ResultKind.STOPPED_AT.value)
        self.assertEqual(result.reason, "SafeHorizon")
        self.assertEqual(result.reached, Fraction(6))
        self.assertEqual(result.chunks, 2)

    def test_retrograde_target_is_rejected_without_mutation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(5), Fraction(20)),)
        )
        result, _ = self.compare_case(state, Fraction(4), Fraction(2))
        self.assertEqual(result.code, ResultKind.LATE_INPUT.value)
        self.assertEqual(result.next_state, state)

    def test_chunk_partition_invariance(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(2)),
            )
        )
        one, _ = self.compare_case(state, Fraction(40), Fraction(40))
        many, _ = self.compare_case(state, Fraction(40), Fraction(2))
        self.assertEqual(p_a_digest(one.next_state), p_a_digest(many.next_state))
        self.assertEqual(one.reached, many.reached)
        self.assertGreater(many.chunks, one.chunks)

    def test_machine_frontier_is_maximum_participant_progress(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(5), Fraction(20), Fraction(3, 2)),
            )
        )
        result, reference = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.REACHED.value)
        self.assertEqual(result.reached, Fraction(10))
        self.assertEqual(
            result.next_state.participants,
            tuple(
                AParticipant(
                    domain.domain_id,
                    domain.progress_common(),
                    domain.horizon_common(),
                    domain.local_to_common,
                )
                for domain in reference.state.domains
            ),
        )

    def test_target_between_participant_progress_values_is_late(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(5), Fraction(20)),
            )
        )
        result, _ = self.compare_case(state, Fraction(4), Fraction(1))
        self.assertEqual(result.code, ResultKind.LATE_INPUT.value)
        self.assertEqual(result.next_state, state)

    def test_obligation_at_current_frontier_stops_zero_length_advance(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(7), Fraction(20)),),
            obligations=(AObligation("irq", "cpu", Fraction(7)),),
        )
        result, _ = self.compare_case(state, Fraction(7), Fraction(4))
        self.assertEqual(result.code, ResultKind.STOPPED_AT.value)
        self.assertEqual(result.reached, Fraction(7))
        self.assertEqual(result.chunks, 0)
        self.assertEqual(result.next_state, state)

    def test_zero_length_advance_catches_up_lagging_participant(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(5), Fraction(20), Fraction(3, 2)),
            )
        )
        result, reference = self.compare_case(state, Fraction(5), Fraction(4))
        self.assertEqual(result.code, ResultKind.REACHED.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(result.chunks, 0)
        self.assertEqual(
            result.next_state.participants,
            tuple(
                AParticipant(
                    domain.domain_id,
                    domain.progress_common(),
                    domain.horizon_common(),
                    domain.local_to_common,
                )
                for domain in reference.state.domains
            ),
        )

    def test_obligation_at_frontier_catches_up_lagging_participant(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(5), Fraction(20), Fraction(3, 2)),
            ),
            obligations=(AObligation("irq", "cpu", Fraction(5)),),
        )
        result, reference = self.compare_case(state, Fraction(5), Fraction(4))
        self.assertEqual(result.code, ResultKind.STOPPED_AT.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(
            result.next_state.participants,
            tuple(
                AParticipant(
                    domain.domain_id,
                    domain.progress_common(),
                    domain.horizon_common(),
                    domain.local_to_common,
                )
                for domain in reference.state.domains
            ),
        )

    def test_safe_horizon_behind_machine_frontier_is_invariant_violation(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(5), Fraction(20)),
                AParticipant("vdp", Fraction(2), Fraction(4)),
            )
        )
        result, _ = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.INVARIANT_VIOLATION.value)
        self.assertEqual(result.reached, Fraction(0))
        self.assertEqual(result.next_state, state)

    def test_unresolved_obligation_behind_machine_frontier_is_invariant_violation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(5), Fraction(20)),),
            obligations=(AObligation("irq", "cpu", Fraction(4)),),
        )
        result, _ = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.INVARIANT_VIOLATION.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(result.next_state, state)

    def test_float_target_is_invalid_input_without_mutation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(5), Fraction(20)),)
        )
        result, _ = self.compare_case(state, 5.25, Fraction(2))
        self.assertEqual(result.code, ResultKind.INVALID_INPUT.value)
        self.assertEqual(result.reached, Fraction(0))
        self.assertEqual(result.next_state, state)


if __name__ == "__main__":
    unittest.main()
