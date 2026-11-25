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
from trace_fixture_support import record_differential
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
                    common_offset=item.common_offset,
                    progress_local=(item.progress - item.common_offset) / item.units_per_common,
                    horizon_local=(item.safe_horizon - item.common_offset) / item.units_per_common,
                )
                for item in state.participants
            ),
            expected_participants=(
                state.expected_participants
                if state.expected_participants is not None
                else (item.name for item in state.participants)
            ),
            watermark_common=state.watermark_common,
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
        self.assertEqual(result.requested, reference.result.requested_common)
        self.assertEqual(result.reached, reference.result.reached_common)
        self.assertEqual(result.next_state.watermark_common, reference.state.watermark_common)
        self.assertEqual(
            result.next_state.expected_participants
            if result.next_state.expected_participants is not None
            else frozenset(item.name for item in result.next_state.participants),
            reference.state.expected_participants,
        )
        self.assertEqual(
            tuple(sorted(
                (item.identity, item.owner, item.at_common, item.resolved, item.commutative)
                for item in result.next_state.obligations
            )),
            tuple(sorted(
                (item.obligation_id, item.owner, item.at_common, item.resolved, item.commutative)
                for item in reference.state.obligations
            )),
        )
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
                    domain.common_offset,
                )
                for domain in reference.state.domains
            ),
        )
        if result.code == ResultKind.INDETERMINATE_CAUSAL_ORDER.value:
            self.assertEqual(result.next_state, state)
            self.assertEqual(result.reached, self._reference_frontier(reference.state))
        record_differential(
            self.id(),
            (
                result.code, result.requested, result.reached,
                self._progress_digest(result.next_state),
            ),
            (
                reference.result.kind.value, reference.result.requested_common,
                reference.result.reached_common,
                self._reference_digest(reference.state),
            ),
        )
        return result, reference

    @staticmethod
    def _progress_digest(state):
        return tuple(
            sorted(
                (item.name, item.progress.numerator, item.progress.denominator,
                 item.common_offset.numerator, item.common_offset.denominator)
                for item in state.participants
            )
        )

    @staticmethod
    def _reference_digest(state):
        return tuple(
            sorted(
                (domain.domain_id, domain.progress_common().numerator,
                 domain.progress_common().denominator,
                 domain.common_offset.numerator, domain.common_offset.denominator)
                for domain in state.domains
            )
        )

    @staticmethod
    def _reference_frontier(state):
        values = [domain.progress_common() for domain in state.domains]
        if state.watermark_common is not None:
            values.append(state.watermark_common)
        return max(values, default=state.watermark_common or Fraction(0))

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
        self.assertEqual(result.reached, Fraction(0))
        self.assertEqual(result.next_state, state)

    def test_float_target_is_invalid_input_without_mutation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(5), Fraction(20)),)
        )
        result, _ = self.compare_case(state, 5.25, Fraction(2))
        self.assertEqual(result.code, ResultKind.INVALID_INPUT.value)
        self.assertEqual(result.reached, Fraction(0))
        self.assertEqual(result.next_state, state)

    def test_deterministic_boundary_matrix_matches_r2(self):
        for domain_count in range(1, 5):
            frontier = Fraction(domain_count + 1)
            participants = tuple(
                AParticipant(
                    f"d{i}",
                    frontier - Fraction((i + domain_count) % 3),
                    frontier + 12 + (i % 2),
                    Fraction(1 + (i % 3), 2),
                )
                for i in range(domain_count)
            )
            # All horizons must be at/after the maximum participant progress.
            horizon = min(item.safe_horizon for item in participants)
            obligations = tuple(
                AObligation(
                    f"e{j}",
                    f"d{j % domain_count}",
                    frontier + Fraction((j * 3 + domain_count) % 9),
                    resolved=(j % 4 == 0),
                    commutative=(j % 2 == 0),
                )
                for j in range(domain_count + 2)
            )
            state = AState(participants, obligations)
            for target in (
                frontier,
                frontier + Fraction(2),
                horizon,
                horizon + Fraction(3),
            ):
                for chunk in (Fraction(1), Fraction(3), Fraction(7)):
                    with self.subTest(
                        domains=domain_count, target=target, chunk=chunk
                    ):
                        self.compare_case(state, target, chunk)

    def test_commutative_simultaneous_obligations_match_r2(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(0), Fraction(20)),),
            obligations=(
                AObligation("irq-z", "cpu", Fraction(5), commutative=True),
                AObligation("irq-a", "cpu", Fraction(5), commutative=True),
            ),
        )
        result, reference = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.STOPPED_AT.value)
        self.assertEqual(result.reason, "PendingObligation:irq-a")
        self.assertEqual(reference.result.obligation_id, "irq-a")

    def test_resolved_obligation_does_not_stop_advance(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(0), Fraction(20)),),
            obligations=(
                AObligation("already-done", "cpu", Fraction(4), resolved=True),
            ),
        )
        result, _ = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.REACHED.value)
        self.assertEqual(result.reached, Fraction(10))

    def test_empty_participant_set_is_blocked(self):
        state = AState(participants=())
        result, reference = self.compare_case(state, Fraction(10), Fraction(2))
        self.assertEqual(result.code, ResultKind.BLOCKED.value)
        self.assertEqual(result.next_state, state)
        self.assertEqual(reference.state.domains, ())

    def test_closed_watermark_participates_in_machine_frontier(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(2), Fraction(20)),
                AParticipant("vdp", Fraction(3), Fraction(20)),
            ),
            watermark_common=Fraction(5),
        )
        late, _ = self.compare_case(state, Fraction(4), Fraction(2))
        self.assertEqual(late.code, ResultKind.LATE_INPUT.value)
        self.assertEqual(late.reached, Fraction(5))
        advanced, _ = self.compare_case(state, Fraction(8), Fraction(2))
        self.assertEqual(advanced.code, ResultKind.REACHED.value)
        self.assertEqual(advanced.reached, Fraction(8))

    def test_incomplete_declared_participant_set_blocks_advance(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(2), Fraction(20)),),
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        result, _ = self.compare_case(state, Fraction(8), Fraction(2))
        self.assertEqual(result.code, ResultKind.BLOCKED.value)
        self.assertEqual(result.next_state, state)

    def test_watermark_beyond_safe_horizon_is_invariant_violation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(2), Fraction(4)),),
            watermark_common=Fraction(5),
        )
        result, _ = self.compare_case(state, Fraction(8), Fraction(2))
        self.assertEqual(result.code, ResultKind.INVARIANT_VIOLATION.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(result.next_state, state)

    def test_unresolved_obligation_at_watermark_is_invariant_violation(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(2), Fraction(20)),),
            obligations=(AObligation("irq", "cpu", Fraction(5)),),
            watermark_common=Fraction(5),
        )
        result, _ = self.compare_case(state, Fraction(8), Fraction(2))
        self.assertEqual(result.code, ResultKind.INVARIANT_VIOLATION.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(result.next_state, state)

    def test_rational_common_offset_conversion_matches_r2(self):
        state = AState(
            participants=(
                AParticipant("cpu", Fraction(5), Fraction(20), Fraction(3, 2), Fraction(2)),
                AParticipant("vdp", Fraction(6), Fraction(30), Fraction(2), Fraction(3)),
            )
        )
        result, _ = self.compare_case(state, Fraction(10), Fraction(3))
        self.assertEqual(result.code, ResultKind.REACHED.value)
        self.assertEqual(result.reached, Fraction(10))

    def test_incomplete_participant_set_still_checks_known_horizon_invariant(self):
        state = AState(
            participants=(AParticipant("cpu", Fraction(2), Fraction(4)),),
            expected_participants=frozenset({"cpu", "vdp"}),
            watermark_common=Fraction(5),
        )
        result, _ = self.compare_case(state, Fraction(8), Fraction(2))
        self.assertEqual(result.code, ResultKind.INVARIANT_VIOLATION.value)
        self.assertEqual(result.reached, Fraction(5))
        self.assertEqual(result.next_state, state)


if __name__ == "__main__":
    unittest.main()
