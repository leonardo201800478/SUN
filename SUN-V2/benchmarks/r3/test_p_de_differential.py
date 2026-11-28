"""Differential adapters for the first P-D/P-E candidate slices.

P-A is imported only as a test comparator. Candidate modules do not import P-A
or R2, and the measured paths remain independent.
"""
from __future__ import annotations

import unittest
from fractions import Fraction
from itertools import combinations

from p_a_causal_sync import (
    ACausalState,
    ADevice,
    ATransition,
    synchronize as p_a_synchronize,
)
from p_a_deadline_lifecycle import (
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    close_watermark as p_a_close_watermark,
    publish_deadline as p_a_publish_deadline,
)
from p_a_instruction_catchup import AParticipant

from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaState,
    PairRelation as AgendaPairRelation,
    claim_next as p_d_claim_next,
    publish as p_d_publish,
)
from p_e_sync_on_demand import (
    PairRelation as SyncPairRelation,
    SyncDomain,
    SyncState,
    SyncTransition,
    synchronize as p_e_synchronize,
)


class CandidateDifferentialTests(unittest.TestCase):
    def test_p_d_heap_claim_order_matches_p_a_deadline_close_for_commutative_groups(self):
        for density in ("sparse", "dense"):
            with self.subTest(density=density):
                names = ("d0", "d1")
                participants = tuple(
                    AParticipant(name, Fraction(0), Fraction(1000), Fraction(1))
                    for name in names
                )
                context = ADeadlineContext(participants, frozenset(names))
                p_a_book = ADeadlineBook()
                p_d_state = AgendaState()
                ids_by_time: dict[Fraction, list[str]] = {}

                for index in range(12):
                    at = (
                        Fraction(10 + index * 5)
                        if density == "sparse"
                        else Fraction(10 + index % 3)
                    )
                    identity = f"event-{index:02d}"
                    owner = names[index % len(names)]
                    pa = p_a_publish_deadline(
                        p_a_book, context,
                        owner=owner,
                        identity=identity,
                        generation=0,
                        at_common=at,
                        effect_id=f"effect-{index:02d}",
                        commutative=True,
                    )
                    self.assertEqual(pa.code, ADeadlineCode.ACCEPTED)
                    p_a_book = pa.book
                    pd = p_d_publish(
                        p_d_state,
                        identity=identity,
                        generation=0,
                        owner=owner,
                        at_common=at,
                        effect_id=f"effect-{index:02d}",
                    )
                    self.assertEqual(pd.code, AgendaCode.ACCEPTED)
                    p_d_state = pd.state
                    ids_by_time.setdefault(at, []).append(identity)

                constraints = tuple(
                    AgendaConstraint(first, second, AgendaPairRelation.COMMUTATIVE)
                    for group in ids_by_time.values()
                    for first, second in combinations(group, 2)
                )
                p_d_state = AgendaState(
                    p_d_state.heap, p_d_state.events, constraints,
                    p_d_state.max_events,
                )

                reference = p_a_close_watermark(p_a_book, Fraction(100))
                self.assertEqual(reference.code, ADeadlineCode.ACCEPTED)
                candidate_claims = []
                while True:
                    result = p_d_claim_next(p_d_state, Fraction(100))
                    if result.code is AgendaCode.BLOCKED:
                        break
                    self.assertEqual(result.code, AgendaCode.CLAIMED)
                    candidate_claims.append((result.event.identity, result.event.generation))
                    p_d_state = result.state
                self.assertEqual(tuple(candidate_claims), reference.claimed)
                self.assertTrue(all(item.status.value == "Claimed" for item in p_d_state.events))

    def test_p_e_dependency_chain_matches_p_a_w03_causal_sync(self):
        names = ("d0", "d1", "d2")
        depth = 9
        previous = None
        p_a_transitions = {name: [] for name in names}
        p_e_transitions = []
        for index in range(depth):
            owner = names[index % len(names)]
            identity = f"chain-{index:04d}"
            requires = () if previous is None else (previous,)
            at = Fraction(index + 1)
            p_a_transitions[owner].append(
                ATransition(identity, at, requires, False)
            )
            p_e_transitions.append(
                SyncTransition(identity, owner, at, requires)
            )
            previous = identity

        cpu_transition = ATransition(
            "cpu-observation", Fraction(depth + 1), (previous,), False
        )
        p_a_state = ACausalState(tuple(
            [ADevice("cpu", Fraction(0), (cpu_transition,), ())]
            + [
                ADevice(name, Fraction(0), tuple(p_a_transitions[name]), ())
                for name in names
            ]
        ))
        target = Fraction(depth + 1)
        reference = p_a_synchronize(
            p_a_state, target_device="cpu", at_common=target, observes=names
        )
        self.assertEqual(reference.code.value, "Synchronized")

        p_e_domains = tuple(
            SyncDomain(name, Fraction(0), target + 1)
            for name in ("cpu", *names)
        )
        p_e_transitions.append(
            SyncTransition("cpu-observation", "cpu", target, (previous,))
        )
        candidate = p_e_synchronize(
            SyncState(p_e_domains, tuple(p_e_transitions)),
            requested_domains=("cpu", *names),
            target_common=target,
        )
        self.assertEqual(candidate.code.value, "Reached")
        self.assertEqual(candidate.state.observed, reference.applied_order)
        self.assertTrue(all(item.progress == target for item in candidate.state.domains))

    def test_p_e_frequent_observations_match_p_a_w04_causal_sync(self):
        names = ("d0", "d1", "d2")
        observations = 6
        p_a_devices = [ADevice("cpu", Fraction(0), (), ())]
        p_e_transitions = []
        constraints = []
        for name in names:
            transitions = tuple(
                ATransition(
                    f"{name}-observation-{tick:04d}",
                    Fraction(tick),
                    (),
                    True,
                )
                for tick in range(1, observations + 1)
            )
            p_a_devices.append(ADevice(name, Fraction(0), transitions, ()))
            p_e_transitions.extend(
                SyncTransition(item.identity, name, item.at_common)
                for item in transitions
            )
        for tick in range(1, observations + 1):
            same_instant = tuple(
                f"{name}-observation-{tick:04d}" for name in names
            )
            constraints.extend(
                SyncConstraint(first, second, SyncPairRelation.COMMUTATIVE)
                for first, second in combinations(same_instant, 2)
            )

        p_a_state = ACausalState(tuple(p_a_devices))
        reference_order = []
        for tick in range(1, observations + 1):
            reference = p_a_synchronize(
                p_a_state,
                target_device="cpu",
                at_common=Fraction(tick),
                observes=names,
            )
            self.assertEqual(reference.code.value, "Synchronized")
            p_a_state = reference.state
            reference_order.extend(reference.applied_order)

        target = Fraction(observations)
        p_e_domains = tuple(
            SyncDomain(name, Fraction(0), target + 1)
            for name in ("cpu", *names)
        )
        candidate = p_e_synchronize(
            SyncState(
                p_e_domains,
                tuple(p_e_transitions),
                tuple(constraints),
            ),
            requested_domains=("cpu", *names),
            target_common=target,
        )
        self.assertEqual(candidate.code.value, "Reached")
        self.assertEqual(candidate.state.observed, tuple(reference_order))
        self.assertTrue(all(item.progress == target for item in candidate.state.domains))


if __name__ == "__main__":
    unittest.main(verbosity=2)
