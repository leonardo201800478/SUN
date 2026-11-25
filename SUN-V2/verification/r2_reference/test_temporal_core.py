"""Executable R2-A properties for the independent CHRONOS oracle."""
from fractions import Fraction
import random
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

class R2T01InvalidStateTests(unittest.TestCase):
    def test_invalid_obligation_is_invalid_input_before_incomplete_participant_block(self):
        state = make_state(
            domains=[one_domain()],
            expected_participants=["cpu", "vdp"],
            obligations=[Obligation("irq-1", "cpu", 1.5)],
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(8))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_invalid_domain_progress_is_invalid_input_before_incomplete_participant_block(self):
        state = make_state(
            domains=[
                ClockDomain(
                    domain_id="cpu",
                    local_to_common=Fraction(1),
                    progress_local=1.5,
                    horizon_local=Fraction(10),
                )
            ],
            expected_participants=["cpu", "vdp"],
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(8))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(canonical_state(evaluated.state), before)
    def test_invalid_clock_ratio_returns_error_without_converting_frontier(self):
        state = make_state(
            domains=[
                ClockDomain(
                    domain_id="cpu",
                    local_to_common=None,
                    progress_local=Fraction(4),
                    horizon_local=Fraction(10),
                )
            ],
            watermark_common=Fraction(3),
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(8))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(evaluated.result.reached_common, Fraction(3))
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_float_progress_returns_error_without_leaking_float_frontier(self):
        state = make_state(
            domains=[
                ClockDomain(
                    domain_id="cpu",
                    local_to_common=Fraction(1),
                    progress_local=1.5,
                    horizon_local=Fraction(10),
                )
            ]
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(8))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(evaluated.result.reached_common, Fraction(0))
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_invalid_watermark_is_rejected_before_incomplete_participant_block(self):
        state = make_state(
            domains=[],
            expected_participants=["cpu"],
            watermark_common=1.5,
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(8))

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(evaluated.result.reached_common, Fraction(0))
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_obligation_without_nonblank_owner_is_rejected_without_mutation(self):
        for owner in ("", "   "):
            with self.subTest(owner=owner):
                state = make_state(
                    domains=[one_domain()],
                    obligations=[Obligation("irq-5", owner, Fraction(5))],
                )
                before = canonical_state(state)

                evaluated = advance(state, Fraction(10))

                self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
                self.assertEqual(canonical_state(evaluated.state), before)





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

        self.assertEqual(evaluated.result.kind, ResultKind.BLOCKED)
        self.assertEqual(canonical_state(evaluated.state), before)
        self.assertIn("no partial frontier", evaluated.result.detail)

    def test_known_watermark_horizon_violation_precedes_incomplete_participant_block(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(10))],
            expected_participants=["cpu", "vdp"],
            watermark_common=Fraction(20),
        )
        before = canonical_state(state)
        evaluated = advance(state, Fraction(25))
        self.assertEqual(
            evaluated.result.kind, ResultKind.INVARIANT_VIOLATION
        )
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_float_target_is_invalid_and_does_not_mutate_state(self):
        state = make_state(domains=[one_domain()])
        before = canonical_state(state)

        evaluated = advance(state, 1.25)

        self.assertEqual(evaluated.result.kind, ResultKind.INVALID_INPUT)
        self.assertEqual(canonical_state(evaluated.state), before)

    def test_seeded_monotonic_sequence_and_retrograde_rejections(self):
        rng = random.Random(0xC0A05)
        state = make_state(domains=[one_domain(horizon=Fraction(1000))])
        previous = Fraction(0)

        for _ in range(100):
            target = previous + Fraction(rng.randint(0, 9), rng.randint(1, 7))
            accepted = advance(state, target)
            self.assertEqual(accepted.result.kind, ResultKind.REACHED)
            state = accepted.state
            previous = target

            before = canonical_state(state)
            rejected = advance(state, target - Fraction(1, 101))
            self.assertEqual(rejected.result.kind, ResultKind.LATE_INPUT)
            self.assertEqual(canonical_state(rejected.state), before)


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

    def test_safe_horizon_stop_matches_partition_at_the_boundary(self):
        initial = make_state(domains=[one_domain(horizon=Fraction(10))])

        whole = advance(initial, Fraction(15))
        at_boundary = advance(initial, Fraction(10))
        partitioned = advance(at_boundary.state, Fraction(15))

        self.assertEqual(whole.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(at_boundary.result.kind, ResultKind.REACHED)
        self.assertEqual(partitioned.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(whole.result.reached_common, Fraction(10))
        self.assertEqual(partitioned.result.reached_common, Fraction(10))
        self.assertEqual(canonical_state(whole.state), canonical_state(partitioned.state))

    def test_pending_obligation_stop_matches_partition_and_continuation(self):
        initial = make_state(
            domains=[one_domain(horizon=Fraction(20))],
            obligations=[Obligation("irq-7", "vdp", Fraction(7))],
        )

        whole_stop = advance(initial, Fraction(15))
        first_partition = advance(initial, Fraction(7))
        second_partition = advance(first_partition.state, Fraction(15))

        self.assertEqual(whole_stop.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(first_partition.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(second_partition.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(whole_stop.result.reached_common, Fraction(7))
        self.assertEqual(second_partition.result.reached_common, Fraction(7))
        self.assertEqual(canonical_state(whole_stop.state), canonical_state(second_partition.state))

        whole_continuation = advance(
            resolve_obligation(whole_stop.state, "irq-7"), Fraction(15)
        )
        partitioned_continuation = advance(
            resolve_obligation(second_partition.state, "irq-7"), Fraction(15)
        )

        self.assertEqual(whole_continuation.result.kind, ResultKind.REACHED)
        self.assertEqual(partitioned_continuation.result.kind, ResultKind.REACHED)
        self.assertEqual(
            canonical_state(whole_continuation.state),
            canonical_state(partitioned_continuation.state),
        )

    def test_seeded_random_partitions_match_single_advance(self):
        for seed in range(100):
            rng = random.Random(seed)
            target = rng.randint(10, 1000)
            cut_count = rng.randint(1, min(12, target - 1))
            cuts = sorted(rng.sample(range(1, target), cut_count))
            initial = make_state(
                domains=[
                    one_domain(ratio=Fraction(2, 3), horizon=Fraction(2000)),
                    ClockDomain(
                        domain_id="vdp",
                        local_to_common=Fraction(5, 7),
                        horizon_local=Fraction(2800),
                    ),
                ]
            )

            whole = advance(initial, Fraction(target))
            partitioned_state = initial
            for cut in cuts + [target]:
                part = advance(partitioned_state, Fraction(cut))
                self.assertEqual(part.result.kind, ResultKind.REACHED)
                partitioned_state = part.state

            self.assertEqual(whole.result.kind, ResultKind.REACHED)
            self.assertEqual(canonical_state(whole.state), canonical_state(partitioned_state))


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

    def test_long_exact_conversion_session_has_no_accumulated_drift(self):
        ratios = (Fraction(2, 3), Fraction(5, 7), Fraction(1234567, 1000003))
        for ratio in ratios:
            domain = one_domain(ratio=ratio, horizon=Fraction(20000))
            for tick in range(1, 10001):
                common = domain.common_offset + Fraction(tick) * ratio
                self.assertEqual(domain.local_at_common(common), Fraction(tick))


class R2T04WatermarkHorizonConsistencyTests(unittest.TestCase):
    def test_watermark_beyond_global_safe_horizon_is_invariant_violation(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(10))],
            watermark_common=Fraction(20),
        )
        before = canonical_state(state)

        evaluated = advance(state, Fraction(20))

        self.assertEqual(evaluated.result.kind, ResultKind.INVARIANT_VIOLATION)
        self.assertEqual(evaluated.result.reached_common, Fraction(20))
        self.assertEqual(canonical_state(evaluated.state), before)


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

    def test_target_before_pending_horizon_obligation_is_reached(self):
        state = make_state(
            domains=[one_domain(horizon=Fraction(10))],
            obligations=[Obligation("irq-10", "vdp", Fraction(10))],
        )

        evaluated = advance(state, Fraction(9))

        self.assertEqual(evaluated.result.kind, ResultKind.REACHED)
        self.assertEqual(evaluated.result.reached_common, Fraction(9))

    def test_target_beyond_horizon_without_pending_obligation_stops_at_horizon(self):
        state = make_state(domains=[one_domain(horizon=Fraction(10))])

        evaluated = advance(state, Fraction(15))

        self.assertEqual(evaluated.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(evaluated.result.reached_common, Fraction(10))
        self.assertEqual(evaluated.result.reason, StopReason.SAFE_HORIZON)

    def test_global_safe_horizon_is_minimum_of_participant_horizons(self):
        state = make_state(
            domains=[
                one_domain(horizon=Fraction(20)),
                ClockDomain("vdp", Fraction(2, 1), horizon_local=Fraction(4)),
            ]
        )

        evaluated = advance(state, Fraction(15))

        self.assertEqual(evaluated.result.kind, ResultKind.STOPPED_AT)
        self.assertEqual(evaluated.result.reached_common, Fraction(8))
        self.assertEqual(evaluated.result.reason, StopReason.SAFE_HORIZON)


if __name__ == "__main__":
    unittest.main(verbosity=2)
