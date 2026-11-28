"""Differential adapters for the first P-D/P-E candidate slices.

P-A is imported only as a test comparator. Candidate modules do not import P-A
or R2, and the measured paths remain independent.
"""
from __future__ import annotations

import unittest
from dataclasses import replace
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
    ADeadline,
    ADeadlineStatus,
    arbitrate_cancellations as p_a_arbitrate_cancellations,
    cancel_deadline as p_a_cancel_deadline,
    close_watermark as p_a_close_watermark,
    publish_deadline as p_a_publish_deadline,
)
from p_a_instruction_catchup import AObligation, AParticipant

from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaContext,
    AgendaObligation,
    AgendaParticipant,
    AgendaState,
    EventStatus,
    arbitrate_cancellations as p_d_arbitrate_cancellations,
    PairRelation as AgendaPairRelation,
    close_watermark as p_d_close_watermark,
    publish as p_d_publish,
    publish_with_context as p_d_publish_with_context,
    request_cancel as p_d_request_cancel,
)
from p_e_sync_on_demand import (
    PairRelation as SyncPairRelation,
    SyncConstraint,
    SyncDomain,
    SyncState,
    SyncTransition,
    synchronize as p_e_synchronize,
)


def _p_a_w05_state(book):
    return (
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.effect_id, item.status.value, item.microstep,
            item.phase_order, item.phase_evidence,
        ) for item in book.deadlines)),
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
        ) for item in book.pending_cancellations)),
        book.watermark_common,
        book.max_microsteps,
    )


def _p_d_w05_state(state):
    scheduled = {
        (item.at_common, item.identity, item.generation)
        for item in state.events if item.status is EventStatus.SCHEDULED
    }
    if set(state.heap) != scheduled:
        raise AssertionError("P-D heap does not match scheduled W05 events")
    return (
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.effect_id, item.status.value, item.microstep,
            item.phase_order, item.phase_evidence,
        ) for item in state.events)),
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
        ) for item in state.pending_cancellations)),
        state.watermark_common,
        state.max_microsteps,
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
                candidate = p_d_close_watermark(p_d_state, Fraction(100))
                self.assertEqual(candidate.code, AgendaCode.ACCEPTED)
                self.assertEqual(candidate.claimed, reference.claimed)
                self.assertEqual(candidate.state.watermark_common, Fraction(100))
                self.assertTrue(all(
                    item.status.value == "Claimed"
                    for item in candidate.state.events
                ))

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


    def test_p_d_w05_pending_cancel_may_cross_c_but_not_deadline_d(self):
        names = frozenset(("cpu", "vdp"))
        pa_context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
            ),
            expected_participants=names,
        )
        pd_context = AgendaContext(
            participants=(
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            expected_participants=names,
        )
        pa_book, pd_state = ADeadlineBook(), AgendaState()

        pa = p_a_publish_deadline(
            pa_book, pa_context, owner="cpu", identity="w05-c-before-d",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pd = p_d_publish_with_context(
            pd_state, pd_context, owner="cpu", identity="w05-c-before-d",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        self.assertEqual(pa.code.value, pd.code.value)
        pa_book, pd_state = pa.book, pd.state
        self.assertEqual(_p_a_w05_state(pa_book), _p_d_w05_state(pd_state))

        pa = p_a_cancel_deadline(
            pa_book, pa_context, owner="cpu", identity="w05-c-before-d",
            generation=0, at_common=Fraction(8),
        )
        pd = p_d_request_cancel(
            pd_state, pd_context, owner="cpu", identity="w05-c-before-d",
            generation=0, at_common=Fraction(8),
        )
        self.assertEqual(pa.code.value, pd.code.value)
        pa_book, pd_state = pa.book, pd.state
        self.assertEqual(_p_a_w05_state(pa_book), _p_d_w05_state(pd_state))

        # W may pass C while remaining strictly before D. The admitted
        # cancellation remains pending and arbitrable at that causal cut.
        pa = p_a_close_watermark(pa_book, Fraction(9))
        pd = p_d_close_watermark(pd_state, Fraction(9), pd_context)
        self.assertEqual(pa.code.value, pd.code.value)
        self.assertEqual(pa.code.value, "Accepted")
        pa_book, pd_state = pa.book, pd.state
        self.assertEqual(_p_a_w05_state(pa_book), _p_d_w05_state(pd_state))
        pa_context = replace(pa_context, watermark_common=Fraction(9))
        pd_context = replace(pd_context, watermark_common=Fraction(9))

        # W must not cross D until the admitted C<D request is arbitrated.
        pa = p_a_close_watermark(pa_book, Fraction(10))
        pd = p_d_close_watermark(pd_state, Fraction(10), pd_context)
        self.assertEqual(pa.code.value, pd.code.value)
        self.assertEqual(pa.code.value, "Blocked")
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

        pa = p_a_arbitrate_cancellations(pa_book)
        pd = p_d_arbitrate_cancellations(pd_state)
        self.assertEqual(pa.code.value, pd.code.value)
        self.assertEqual(pa.code.value, "Accepted")
        pa_book, pd_state = pa.book, pd.state
        self.assertEqual(_p_a_w05_state(pa_book), _p_d_w05_state(pd_state))
        self.assertEqual(pa_book.deadlines[0].status, ADeadlineStatus.CANCELLED)

        pa = p_a_close_watermark(pa_book, Fraction(10))
        pd = p_d_close_watermark(pd_state, Fraction(10), pd_context)
        self.assertEqual(pa.code.value, pd.code.value)
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

    def test_p_d_w05_safe_horizon_regression_matches_p_a_taxonomy(self):
        names = frozenset(("cpu", "vdp"))
        cases = (
            (
                (AParticipant("cpu", Fraction(5), Fraction(10), Fraction(1), Fraction(0)),
                 AParticipant("vdp", Fraction(0), Fraction(4), Fraction(1), Fraction(0))),
                (AgendaParticipant("cpu", Fraction(5), Fraction(10)),
                 AgendaParticipant("vdp", Fraction(0), Fraction(4))),
                None,
            ),
            (
                (AParticipant("cpu", Fraction(0), Fraction(5), Fraction(1), Fraction(0)),
                 AParticipant("vdp", Fraction(0), Fraction(8), Fraction(1), Fraction(0))),
                (AgendaParticipant("cpu", Fraction(0), Fraction(5)),
                 AgendaParticipant("vdp", Fraction(0), Fraction(8))),
                Fraction(6),
            ),
        )
        for index, (pa_participants, pd_participants, watermark) in enumerate(cases):
            with self.subTest(case=index):
                pa_context = ADeadlineContext(
                    pa_participants, names, watermark_common=watermark
                )
                pd_context = AgendaContext(pd_participants, names, watermark)
                pa = p_a_publish_deadline(
                    ADeadlineBook(watermark_common=watermark), pa_context, owner="cpu",
                    identity=f"w05-safe-{index}", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                pd = p_d_publish_with_context(
                    AgendaState(watermark_common=watermark), pd_context, owner="cpu",
                    identity=f"w05-safe-{index}", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                self.assertEqual(pa.code.value, "InvariantViolation")
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

    def test_p_d_w05_incomplete_participants_and_watermark_mismatch_match_p_a(self):
        names = frozenset(("cpu", "vdp"))
        complete_pa = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        complete_pd = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pa_book = ADeadlineBook(deadlines=(
            ADeadline("cpu", "incomplete-participants", 0, Fraction(10), "effect"),
        ))
        pd_state = p_d_publish(
            AgendaState(), identity="incomplete-participants", generation=0,
            owner="cpu", at_common=Fraction(10), effect_id="effect",
        ).state
        incomplete_pa = ADeadlineContext(
            (AParticipant("cpu", Fraction(0), Fraction(100)),), names
        )
        incomplete_pd = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(100)),), names
        )
        pa = p_a_cancel_deadline(
            pa_book, incomplete_pa, owner="cpu",
            identity="incomplete-participants", generation=0,
            at_common=Fraction(8),
        )
        pd = p_d_request_cancel(
            pd_state, incomplete_pd, owner="cpu",
            identity="incomplete-participants", generation=0,
            at_common=Fraction(8),
        )
        self.assertEqual(pa.code.value, "Blocked")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

        pa_book = ADeadlineBook(
            deadlines=(
                ADeadline("cpu", "watermark-mismatch", 0, Fraction(10), "effect"),
            ),
            watermark_common=Fraction(9),
        )
        pd_state = replace(
            p_d_publish(
                AgendaState(), identity="watermark-mismatch", generation=0,
                owner="cpu", at_common=Fraction(10), effect_id="effect",
            ).state,
            watermark_common=Fraction(9),
        )
        mismatched_pa = ADeadlineContext(
            complete_pa.participants, names, watermark_common=Fraction(8)
        )
        mismatched_pd = AgendaContext(
            complete_pd.participants, names, Fraction(8)
        )
        pa = p_a_publish_deadline(
            pa_book, mismatched_pa, owner="cpu",
            identity="new-deadline", generation=0,
            at_common=Fraction(11), effect_id="new-effect",
        )
        pd = p_d_publish_with_context(
            pd_state, mismatched_pd, owner="cpu",
            identity="new-deadline", generation=0,
            at_common=Fraction(11), effect_id="new-effect",
        )
        self.assertEqual(pa.code.value, "IndeterminateCausalOrder")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

    def test_p_d_w05_same_instant_microstep_order_and_budget_match_p_a(self):
        names = frozenset(("cpu", "vdp"))
        pa_context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pa = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu",
            identity="w05-microstep-order", generation=0,
            at_common=Fraction(10), effect_id="microstep-effect",
            microstep=True, microstep_level=1,
        )
        pd = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu",
            identity="w05-microstep-order", generation=0,
            at_common=Fraction(10), effect_id="microstep-effect",
            microstep=1,
        )
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

        pa = p_a_cancel_deadline(
            pa.book, pa_context, owner="cpu",
            identity="w05-microstep-order", generation=0,
            at_common=Fraction(10), microstep_level=0,
        )
        pd = p_d_request_cancel(
            pd.state, pd_context, owner="cpu",
            identity="w05-microstep-order", generation=0,
            at_common=Fraction(10), microstep_level=0,
        )
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

        pa = p_a_arbitrate_cancellations(pa.book)
        pd = p_d_arbitrate_cancellations(pd.state)
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))
        self.assertEqual(pa.book.deadlines[0].status, ADeadlineStatus.CANCELLED)

        pa_limit = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu",
            identity="w05-microstep-limit", generation=0,
            at_common=Fraction(11), effect_id="limit-effect",
            microstep=True, microstep_level=33,
        )
        pd_limit = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu",
            identity="w05-microstep-limit", generation=0,
            at_common=Fraction(11), effect_id="limit-effect",
            microstep=33,
        )
        self.assertEqual(pa_limit.code.value, "ResourceExhausted")
        self.assertEqual(pd_limit.code.value, pa_limit.code.value)
        self.assertEqual(_p_a_w05_state(pa_limit.book), _p_d_w05_state(pd_limit.state))

    def test_p_d_w05_unresolved_obligations_constrain_admission_frontier(self):
        names = frozenset(("cpu", "vdp"))
        cases = (
            {
                "pa_participants": (
                    AParticipant("cpu", Fraction(5), Fraction(10)),
                    AParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pd_participants": (
                    AgendaParticipant("cpu", Fraction(5), Fraction(10)),
                    AgendaParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pa_obligations": (AObligation("behind-frontier", "cpu", Fraction(4)),),
                "pd_obligations": (AgendaObligation("behind-frontier", "cpu", Fraction(4)),),
                "watermark": None,
                "expected": "InvariantViolation",
            },
            {
                "pa_participants": (
                    AParticipant("cpu", Fraction(0), Fraction(10)),
                    AParticipant("vdp", Fraction(0), Fraction(10)),
                ),
                "pd_participants": (
                    AgendaParticipant("cpu", Fraction(0), Fraction(10)),
                    AgendaParticipant("vdp", Fraction(0), Fraction(10)),
                ),
                "pa_obligations": (AObligation("at-watermark", "cpu", Fraction(6)),),
                "pd_obligations": (AgendaObligation("at-watermark", "cpu", Fraction(6)),),
                "watermark": Fraction(6),
                "expected": "InvariantViolation",
            },
            {
                "pa_participants": (
                    AParticipant("cpu", Fraction(5), Fraction(10)),
                    AParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pd_participants": (
                    AgendaParticipant("cpu", Fraction(5), Fraction(10)),
                    AgendaParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pa_obligations": (AObligation("future", "cpu", Fraction(7)),),
                "pd_obligations": (AgendaObligation("future", "cpu", Fraction(7)),),
                "watermark": None,
                "expected": "Accepted",
            },
            {
                "pa_participants": (
                    AParticipant("cpu", Fraction(5), Fraction(10)),
                    AParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pd_participants": (
                    AgendaParticipant("cpu", Fraction(5), Fraction(10)),
                    AgendaParticipant("vdp", Fraction(0), Fraction(20)),
                ),
                "pa_obligations": (
                    AObligation("resolved-behind", "cpu", Fraction(4), resolved=True),
                ),
                "pd_obligations": (
                    AgendaObligation("resolved-behind", "cpu", Fraction(4), resolved=True),
                ),
                "watermark": None,
                "expected": "Accepted",
            },
        )
        for index, case in enumerate(cases):
            with self.subTest(case=index):
                pa_context = ADeadlineContext(
                    case["pa_participants"], names, case["pa_obligations"],
                    case["watermark"],
                )
                pd_context = AgendaContext(
                    case["pd_participants"], names, case["watermark"],
                    case["pd_obligations"],
                )
                pa = p_a_publish_deadline(
                    ADeadlineBook(watermark_common=case["watermark"]),
                    pa_context, owner="cpu", identity=f"w05-obligation-{index}",
                    generation=0, at_common=Fraction(10), effect_id="effect",
                )
                pd = p_d_publish_with_context(
                    AgendaState(watermark_common=case["watermark"]),
                    pd_context, owner="cpu", identity=f"w05-obligation-{index}",
                    generation=0, at_common=Fraction(10), effect_id="effect",
                )
                self.assertEqual(pa.code.value, case["expected"])
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

    def test_p_d_w05_cancel_owner_outside_participant_contract_is_blocked(self):
        names = frozenset(("cpu", "vdp"))
        pa_context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
            ),
            names,
        )
        pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pa_book = ADeadlineBook(deadlines=(
            ADeadline("apu", "orphan", 0, Fraction(10), "effect"),
        ))
        pd_state = p_d_publish(
            AgendaState(), identity="orphan", generation=0,
            owner="apu", at_common=Fraction(10), effect_id="effect",
        ).state
        pa = p_a_cancel_deadline(
            pa_book, pa_context, owner="apu", identity="orphan",
            generation=0, at_common=Fraction(8),
        )
        pd = p_d_request_cancel(
            pd_state, pd_context, owner="apu", identity="orphan",
            generation=0, at_common=Fraction(8),
        )
        self.assertEqual(pa.code.value, "Blocked")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

    def test_p_d_w05_cancel_admission_and_same_instant_order_match_p_a(self):
        names = frozenset(("cpu", "vdp"))
        pa_context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
                AParticipant("vdp", Fraction(0), Fraction(100), Fraction(1), Fraction(0)),
            ),
            expected_participants=names,
        )
        pd_context = AgendaContext(
            participants=(
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            expected_participants=names,
        )
        for at, expected_code in (
            (Fraction(0), "LateInput"),
            (Fraction(10), "IndeterminateCausalOrder"),
            (Fraction(11), "LateInput"),
        ):
            with self.subTest(cancel_at=at):
                pa = p_a_publish_deadline(
                    ADeadlineBook(), pa_context, owner="cpu",
                    identity="w05-boundary", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                pd = p_d_publish_with_context(
                    AgendaState(), pd_context, owner="cpu",
                    identity="w05-boundary", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                pa_cancel = p_a_cancel_deadline(
                    pa.book, pa_context, owner="cpu",
                    identity="w05-boundary", generation=0, at_common=at,
                )
                pd_cancel = p_d_request_cancel(
                    pd.state, pd_context, owner="cpu",
                    identity="w05-boundary", generation=0, at_common=at,
                )
                self.assertEqual(pa_cancel.code.value, expected_code)
                self.assertEqual(pd_cancel.code.value, expected_code)
                self.assertEqual(_p_a_w05_state(pa_cancel.book), _p_d_w05_state(pd_cancel.state))

if __name__ == "__main__":
    unittest.main(verbosity=2)
