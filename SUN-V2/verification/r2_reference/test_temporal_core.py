"""Executable R2-A properties for the independent CHRONOS oracle."""
from fractions import Fraction
import unittest

from chronos_reference import (
    ClockDomain,
    Obligation,
    ResultKind,
    StopReason,
    advance,
    canonical_state,
    make_state,
    resolve_obligation,
)


def one_domain(*, ratio=Fraction(1), horizon=Fraction(100), progress=Fraction(0)):
    return ClockDomain(
        domain_id="cpu",
        local_to_common=ratio,
        progress_local=progress,
        horizon_local=horizon,
    )


class R2T01MonotonicityTests(unittest.TestCase):
    def test_retrograde_external_advance_is_rejected_without_mutation(self):
        state = make_state(domains=[one_domain(progress=Fraction(10))])
        before = canonical_state(state)

        evaluated = advance(state, Fraction(9))

        self.assertEqual(evaluated.result.kind, ResultKind.LATE_INPUT)
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_repeating_the_current_target_is_idempotent_without_pending_work(self):
        state = make_state(domains=[one_domain(progress=Fraction(10))])

        evaluated = advance(state, Fraction(10))

        self.assertEqual(evaluated.result.kind, ResultKind.REACHED)
        self.assertEqual(canonical_state(evaluated.state), canonical_state(state))

    def test_internal_safe_horizon_regression_is_invariant_violation(self):
        domain = ClockDomain(
            domain_id="cpu",
            local_to_common=Fraction(1),
            progress_local=Fraction(5),
            horizon_local=Fraction(4),
        )
        state = make_state(domains=[domain])

        evaluated = advance(state, Fraction(6))

        self.assertEqual(evaluated.result.kind, ResultKind.INVARIANT_VIOLATION)
        self.assertEqual(canonical_state(evaluated.state), canonical_state(state))

    def test_incomplete_participant_set_is_not_used_for_a_partial_frontier(self):
        state = make_state(
            domains=[one_domain()],
            expected_participants=["cpu", "vdp"],
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(5))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(canonical_state(evaluated.state), before)


class R2T02ChunkInvarianceTests(unittest.TestCase):
    def test_one_advance_equals_permitted_partitioned_advances(self):
        initial = make_state(
            domains=[
                one_domain(ratio=Fraction(2, 3), horizon=Fraction(30)),
                ClockDomain(
                    domain_id="vdp",
                    local_to_common=Fraction(5, 7),
                    horizon_local=Fraction(42),
                ),
            ]
        )

        whole = advance(initial, Fraction(12))
        self.assertEqual(whole.result.kind, ResultKind.REACHED)

        first = advance(initial, Fraction(3))
        second = advance(first.state, Fraction(7))
        third = advance(second.state, Fraction(12))

        self.assertEqual(first.result.kind, ResultKind.REACHED)
        self.assertEqual(second.result.kind, ResultKind.REACHED)
        self.assertEqual(third.result.kind, ResultKind.REACHED)
        self.assertEqual(canonical_state(whole.state), canonical_state(third.state))


class R2T03ExactConversionTests(unittest.TestCase):
    def test_non_integer_ratio_round_trips_without_drift(self):
        domain = one_domain(ratio=Fraction(5, 7), horizon=Fraction(700))
        target_common = Fraction(123456789, 11)
        local = domain.local_at_common(target_common)
        round_trip = domain.common_offset + local * domain.local_to_common

        self.assertEqual(round_trip, target_common)

    def test_composed_exact_ratios_equal_direct_ratio(self):
        local = Fraction(987654321, 13)
        a_to_b = Fraction(7, 5)
        b_to_common = Fraction(11, 9)
        direct = a_to_b * b_to_common

        self.assertEqual(local * a_to_b * b_to_common, local * direct)


class R2T04SafeHorizonTests(unittest.TestCase):
    def test_pending_obligation_at_target_horizon_returns_endpoint_stop(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(10))],
            obligations=[
                Obligation("irq-10", "vdp", Fraction(10), resolved=False)
            ],
        )

        evaluated = advance(state, Fraction(10))

        self.assertEqual(evaluated.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(evaluated.result.reached_common, Fraction(10))
        self.assertEqual(evaluated.result.requested_common, Fraction(10))
        self.assertEqual(evaluated.result.reason, StopReason.PENDING_OBLIGATION)
        self.assertEqual(evaluated.result.obligation_id, "irq-10")
        self.assertEqual(evaluated.state.domains[0].progress_common(), Fraction(10))

    def test_pending_obligation_at_horizon_stops_before_later_target(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(10))],
            obligations=[
                Obligation("irq-10", "vdp", Fraction(10), resolved=False)
            ],
        )

        evaluated = advance(state, Fraction(15))

        self.assertEqual(evaluated.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(evaluated.result.reached_common, Fraction(10))
        self.assertLess(evaluated.result.reached_common, evaluated.result.requested_common)

    def test_horizon_endpoint_is_reached_when_all_obligations_are_resolved(self):
        state = make_state(domains=[one_domain(horizon=Fraction(10))])

        evaluated = advance(state, Fraction(10))

        self.assertEqual(evaluated.result.kind, ResultKind.REACHED)
        self.assertEqual(evaluated.result.reached_common, Fraction(10))

    def test_earlier_pending_obligation_prevents_crossing_its_boundary(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(20))],
            obligations=[
                Obligation("bus-7", "bus", Fraction(7), resolved=False)
            ],
        )

        evaluated = advance(state, Fraction(15))

        self.assertEqual(evaluated.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(evaluated.result.reached_common, Fraction(7))
        self.assertEqual(evaluated.result.reason, StopReason.PENDING_OBLIGATION)

    def test_resolving_obligation_allows_subsequent_advance(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(20))],
            obligations=[
                Obligation("bus-7", "bus", Fraction(7), resolved=False)
            ],
        )
        stopped = advance(state, Fraction(15))
        resolved = resolve_obligation(stopped.state, "bus-7")
        continued = advance(resolved, Fraction(15))

        self.assertEqual(stopped.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(continued.result.kind, ResultKind.REACHED)
        self.assertEqual(continued.result.reached_common, Fraction(15))


if __name__ == "__main__":
    unittest.main(verbosity=2)
