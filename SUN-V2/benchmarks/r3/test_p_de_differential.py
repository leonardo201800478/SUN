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
    complete_deadline as p_a_complete_deadline,
    dispatch_deadline as p_a_dispatch_deadline,
    publish_deadline as p_a_publish_deadline,
    replace_deadline as p_a_replace_deadline,
)
from p_a_instruction_catchup import AObligation, AParticipant

from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaContext,
    AgendaEvent,
    AgendaObligation,
    AgendaParticipant,
    AgendaState,
    EventStatus,
    arbitrate_cancellations as p_d_arbitrate_cancellations,
    PairRelation as AgendaPairRelation,
    claim_next as p_d_claim_next,
    close_watermark as p_d_close_watermark,
    complete as p_d_complete,
    dispatch_event as p_d_dispatch_event,
    publish as p_d_publish,
    publish_with_context as p_d_publish_with_context,
    request_cancel as p_d_request_cancel,
    replace_with_context as p_d_replace_with_context,
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

    def test_p_d_w05_generation_replacement_is_atomic_and_fences_old_generation(self):
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

        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-replace",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-replace",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        self.assertEqual(pa_first.code.value, "Accepted")
        self.assertEqual(pd_first.code.value, pa_first.code.value)
        self.assertEqual(_p_a_w05_state(pa_first.book), _p_d_w05_state(pd_first.state))

        pa = p_a_replace_deadline(
            pa_first.book, pa_context, owner="cpu", identity="w05-replace",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pd = p_d_replace_with_context(
            pd_first.state, pd_context, owner="cpu", identity="w05-replace",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))
        self.assertEqual(
            tuple(item.status.value for item in pa.book.deadlines),
            ("Cancelled", "Scheduled"),
        )
        self.assertEqual(
            tuple(item.status.value for item in pd.state.events),
            ("Cancelled", "Scheduled"),
        )
        self.assertEqual(pd.state.heap, ((Fraction(20), "w05-replace", 1),))

        pa_closed = p_a_close_watermark(pa.book, Fraction(20))
        pd_closed = p_d_close_watermark(pd.state, Fraction(20), pd_context)
        self.assertEqual(pa_closed.code.value, "Accepted")
        self.assertEqual(pd_closed.code.value, pa_closed.code.value)
        self.assertEqual(_p_a_w05_state(pa_closed.book), _p_d_w05_state(pd_closed.state))
        self.assertEqual(
            tuple(item.status.value for item in pa_closed.book.deadlines),
            ("Cancelled", "Claimed"),
        )
        self.assertEqual(
            tuple(item.status.value for item in pd_closed.state.events),
            ("Cancelled", "Claimed"),
        )

        for cancel_at, new_at, expected_code in (
            (Fraction(5), Fraction(4), "LateInput"),
            (Fraction(5), Fraction(5), "IndeterminateCausalOrder"),
        ):
            with self.subTest(cancel_at=cancel_at, new_at=new_at):
                pa_result = p_a_replace_deadline(
                    pa_first.book, pa_context, owner="cpu", identity="w05-replace",
                    generation=0, cancel_at=cancel_at, new_generation=1,
                    new_at_common=new_at, new_effect_id="new-effect",
                )
                pd_result = p_d_replace_with_context(
                    pd_first.state, pd_context, owner="cpu", identity="w05-replace",
                    generation=0, cancel_at=cancel_at, new_generation=1,
                    new_at_common=new_at, new_effect_id="new-effect",
                )
                self.assertEqual(pa_result.code.value, expected_code)
                self.assertEqual(pd_result.code.value, pa_result.code.value)
                self.assertEqual(_p_a_w05_state(pa_result.book), _p_d_w05_state(pd_result.state))
                self.assertEqual(pa_result.book, pa_first.book)
                self.assertEqual(pd_result.state, pd_first.state)

        pa_invalid = p_a_replace_deadline(
            pa_first.book, pa_context, owner="cpu", identity="w05-replace",
            generation=0, cancel_at=Fraction(5), new_generation=0,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pd_invalid = p_d_replace_with_context(
            pd_first.state, pd_context, owner="cpu", identity="w05-replace",
            generation=0, cancel_at=Fraction(5), new_generation=0,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        self.assertEqual(pa_invalid.code.value, "InvalidInput")
        self.assertEqual(pd_invalid.code.value, pa_invalid.code.value)
        self.assertEqual(_p_a_w05_state(pa_invalid.book), _p_d_w05_state(pd_invalid.state))
        self.assertEqual(pa_invalid.book, pa_first.book)
        self.assertEqual(pd_invalid.state, pd_first.state)


    def test_p_d_w05_rejects_older_active_generation_after_newer_terminal_generation(self):
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
        identity = "w05-invalid-generation-history"
        pa_book = ADeadlineBook(deadlines=(
            ADeadline("cpu", identity, 0, Fraction(10), "old-effect"),
            ADeadline(
                "cpu", identity, 1, Fraction(20), "new-effect",
                status=ADeadlineStatus.CANCELLED,
            ),
        ))
        pd_state = AgendaState(
            heap=((Fraction(10), identity, 0),),
            events=(
                AgendaEvent(identity, 0, "cpu", Fraction(10), "old-effect"),
                AgendaEvent(
                    identity, 1, "cpu", Fraction(20), "new-effect",
                    EventStatus.CANCELLED,
                ),
            ),
        )
        self.assertEqual(_p_a_w05_state(pa_book), _p_d_w05_state(pd_state))

        pa = p_a_cancel_deadline(
            pa_book, pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(5),
        )
        pd = p_d_request_cancel(
            pd_state, pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(5),
        )
        self.assertEqual(pa.code.value, "InvalidInput")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(pa.book, pa_book)
        self.assertEqual(pd.state, pd_state)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))


    def test_p_d_w05_publish_rejects_successor_while_prior_generation_is_scheduled(self):
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
        identity = "w05-publish-scheduled-predecessor"
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        self.assertEqual(pa_first.code.value, "Accepted")
        self.assertEqual(pd_first.code.value, pa_first.code.value)
        self.assertEqual(_p_a_w05_state(pa_first.book), _p_d_w05_state(pd_first.state))

        pa_rejected = p_a_publish_deadline(
            pa_first.book, pa_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        pd_rejected = p_d_publish_with_context(
            pd_first.state, pd_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        self.assertEqual(pa_rejected.code.value, "InvalidInput")
        self.assertEqual(pd_rejected.code.value, pa_rejected.code.value)
        self.assertEqual(pa_rejected.book, pa_first.book)
        self.assertEqual(pd_rejected.state, pd_first.state)
        self.assertEqual(_p_a_w05_state(pa_rejected.book), _p_d_w05_state(pd_rejected.state))


    def test_p_d_w05_publish_rejects_successor_while_prior_cancel_is_pending(self):
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
        identity = "w05-publish-pending-cancellation"
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        self.assertEqual(pa_first.code.value, "Accepted")
        self.assertEqual(pd_first.code.value, pa_first.code.value)
        self.assertEqual(_p_a_w05_state(pa_first.book), _p_d_w05_state(pd_first.state))

        pa_pending = p_a_cancel_deadline(
            pa_first.book, pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        pd_pending = p_d_request_cancel(
            pd_first.state, pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        self.assertEqual(pa_pending.code.value, "Accepted")
        self.assertEqual(pd_pending.code.value, pa_pending.code.value)
        self.assertEqual(_p_a_w05_state(pa_pending.book), _p_d_w05_state(pd_pending.state))

        pa_rejected = p_a_publish_deadline(
            pa_pending.book, pa_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        pd_rejected = p_d_publish_with_context(
            pd_pending.state, pd_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        self.assertEqual(pa_rejected.code.value, "InvalidInput")
        self.assertEqual(pd_rejected.code.value, pa_rejected.code.value)
        self.assertEqual(pa_rejected.book, pa_pending.book)
        self.assertEqual(pd_rejected.state, pd_pending.state)
        self.assertEqual(_p_a_w05_state(pa_rejected.book), _p_d_w05_state(pd_rejected.state))


    def test_p_d_w05_publication_rejects_blank_owner_and_duplicate_participants(self):
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
        duplicate_pa = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
                AParticipant("cpu", Fraction(0), Fraction(100)),
            ),
            names,
        )
        duplicate_pd = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
            ),
            names,
        )
        cases = (
            ("blank-owner", complete_pa, complete_pd, ""),
            ("duplicate-participant", duplicate_pa, duplicate_pd, "cpu"),
        )
        for label, pa_context, pd_context, owner in cases:
            with self.subTest(case=label):
                pa = p_a_publish_deadline(
                    ADeadlineBook(), pa_context, owner=owner,
                    identity=f"w05-structural-{label}", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                pd = p_d_publish_with_context(
                    AgendaState(), pd_context, owner=owner,
                    identity=f"w05-structural-{label}", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                self.assertEqual(pa.code.value, "InvalidInput")
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(pa.book, ADeadlineBook())
                self.assertEqual(pd.state, AgendaState())
                self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))


    def test_p_d_w05_generation_must_increase_after_terminal_cancellation(self):
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
        identity = "w05-terminal-generation-monotonicity"
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pa_cancel = p_a_cancel_deadline(
            pa_first.book, pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        pd_cancel = p_d_request_cancel(
            pd_first.state, pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        pa_terminal = p_a_arbitrate_cancellations(pa_cancel.book)
        pd_terminal = p_d_arbitrate_cancellations(pd_cancel.state)
        self.assertEqual(pa_terminal.code.value, "Accepted")
        self.assertEqual(pd_terminal.code.value, pa_terminal.code.value)
        self.assertEqual(_p_a_w05_state(pa_terminal.book), _p_d_w05_state(pd_terminal.state))

        pa_stale = p_a_publish_deadline(
            pa_terminal.book, pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(20), effect_id="stale-effect",
        )
        pd_stale = p_d_publish_with_context(
            pd_terminal.state, pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(20), effect_id="stale-effect",
        )
        self.assertEqual(pa_stale.code.value, "InvalidInput")
        self.assertEqual(pd_stale.code.value, pa_stale.code.value)
        self.assertEqual(pa_stale.book, pa_terminal.book)
        self.assertEqual(pd_stale.state, pd_terminal.state)

        pa_successor = p_a_publish_deadline(
            pa_terminal.book, pa_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        pd_successor = p_d_publish_with_context(
            pd_terminal.state, pd_context, owner="cpu", identity=identity,
            generation=1, at_common=Fraction(20), effect_id="new-effect",
        )
        self.assertEqual(pa_successor.code.value, "Accepted")
        self.assertEqual(pd_successor.code.value, pa_successor.code.value)
        self.assertEqual(_p_a_w05_state(pa_successor.book), _p_d_w05_state(pd_successor.state))


    def test_p_d_w05_preconstructed_microstep_over_budget_is_rejected_atomically(self):
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
        pa_first = p_a_publish_deadline(
            ADeadlineBook(max_microsteps=1), pa_context, owner="cpu",
            identity="w05-preconstructed-budget", generation=0,
            at_common=Fraction(10), effect_id="effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(max_microsteps=1), pd_context, owner="cpu",
            identity="w05-preconstructed-budget", generation=0,
            at_common=Fraction(10), effect_id="effect",
        )
        pa_book = replace(
            pa_first.book,
            deadlines=(replace(pa_first.book.deadlines[0], microstep=2),),
        )
        pd_state = replace(
            pd_first.state,
            events=(replace(pd_first.state.events[0], microstep=2),),
        )

        pa_rejected = p_a_close_watermark(pa_book, Fraction(10))
        pd_rejected = p_d_close_watermark(pd_state, Fraction(10), pd_context)

        self.assertEqual(pa_rejected.code.value, "InvalidInput")
        self.assertEqual(pd_rejected.code.value, pa_rejected.code.value)
        self.assertEqual(pa_rejected.book, pa_book)
        self.assertEqual(pd_rejected.state, pd_state)


    def test_p_d_w05_arbitration_rejects_orphan_and_owner_mismatched_cancellations(self):
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
        identity = "w05-preconstructed-cancellation-integrity"
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pa_pending = p_a_cancel_deadline(
            pa_first.book, pa_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        pd_pending = p_d_request_cancel(
            pd_first.state, pd_context, owner="cpu", identity=identity,
            generation=0, at_common=Fraction(4),
        )
        self.assertEqual(pa_pending.code.value, "Accepted")
        self.assertEqual(pd_pending.code.value, pa_pending.code.value)

        orphan_pa = replace(pa_pending.book, deadlines=())
        orphan_pd = replace(pd_pending.state, events=(), heap=())
        owner_mismatch_pa = replace(
            pa_pending.book,
            pending_cancellations=(
                replace(pa_pending.book.pending_cancellations[0], owner="vdp"),
            ),
        )
        owner_mismatch_pd = replace(
            pd_pending.state,
            pending_cancellations=(
                replace(pd_pending.state.pending_cancellations[0], owner="vdp"),
            ),
        )
        cases = (
            ("orphan cancellation", orphan_pa, orphan_pd),
            ("owner mismatch", owner_mismatch_pa, owner_mismatch_pd),
        )
        for label, pa_book, pd_state in cases:
            with self.subTest(case=label):
                pa_before = pa_book
                pd_before = pd_state
                pa = p_a_arbitrate_cancellations(pa_book)
                pd = p_d_arbitrate_cancellations(pd_state)
                self.assertEqual(pa.code.value, "InvalidInput")
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(pa.book, pa_before)
                self.assertEqual(pd.state, pd_before)


    def test_p_d_w05_malformed_expected_participant_container_is_rejected_atomically(self):
        pa_context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            ["cpu", "vdp"],
        )
        pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            ["cpu", "vdp"],
        )
        pa_before = ADeadlineBook()
        pd_before = AgendaState()
        pa = p_a_publish_deadline(
            pa_before, pa_context, owner="cpu", identity="w05-invalid-expected",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pd = p_d_publish_with_context(
            pd_before, pd_context, owner="cpu", identity="w05-invalid-expected",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        self.assertEqual(pa.code.value, "InvalidInput")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(pa.book, pa_before)
        self.assertEqual(pd.state, pd_before)


    def test_p_d_w05_malformed_deadline_entries_are_rejected_atomically(self):
        names = frozenset(("cpu", "vdp"))
        pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pa_before = ADeadlineBook(deadlines=(object(),))
        pd_before = AgendaState(events=(object(),))
        pa = p_a_close_watermark(pa_before, Fraction(10))
        pd = p_d_close_watermark(pd_before, Fraction(10), pd_context)
        self.assertEqual(pa.code.value, "InvalidInput")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(pa.book, pa_before)
        self.assertEqual(pd.state, pd_before)


    def test_p_d_w05_malformed_pending_cancellation_entries_are_rejected_atomically(self):
        pa_before = ADeadlineBook(pending_cancellations=(object(),))
        pd_before = AgendaState(pending_cancellations=(object(),))
        pa = p_a_arbitrate_cancellations(pa_before)
        pd = p_d_arbitrate_cancellations(pd_before)
        self.assertEqual(pa.code.value, "InvalidInput")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(pa.book, pa_before)
        self.assertEqual(pd.state, pd_before)


    def test_p_d_w05_claim_dispatch_complete_lifecycle_matches_p_a(self):
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
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-lifecycle",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pd = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-lifecycle",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))

        early_pa = p_a_dispatch_deadline(
            pa.book, identity="w05-lifecycle", generation=0,
        )
        early_pd = p_d_dispatch_event(
            pd.state, "w05-lifecycle", 0,
        )
        self.assertEqual(early_pa.code.value, "StaleIgnored")
        self.assertEqual(early_pd.code.value, early_pa.code.value)
        self.assertEqual(early_pa.book, pa.book)
        self.assertEqual(early_pd.state, pd.state)

        pa_closed = p_a_close_watermark(pa.book, Fraction(10))
        pd_closed = p_d_close_watermark(pd.state, Fraction(10), pd_context)
        self.assertEqual(pa_closed.code.value, "Accepted")
        self.assertEqual(pd_closed.code.value, pa_closed.code.value)
        self.assertEqual(pa_closed.claimed, pd_closed.claimed)
        self.assertEqual(_p_a_w05_state(pa_closed.book), _p_d_w05_state(pd_closed.state))

        premature_pa = p_a_complete_deadline(
            pa_closed.book, identity="w05-lifecycle", generation=0,
        )
        premature_pd = p_d_complete(
            pd_closed.state, "w05-lifecycle", 0,
        )
        self.assertEqual(premature_pa.code.value, "InvalidInput")
        self.assertEqual(premature_pd.code.value, premature_pa.code.value)
        self.assertEqual(premature_pa.book, pa_closed.book)
        self.assertEqual(premature_pd.state, pd_closed.state)

        dispatched_pa = p_a_dispatch_deadline(
            pa_closed.book, identity="w05-lifecycle", generation=0,
        )
        dispatched_pd = p_d_dispatch_event(
            pd_closed.state, "w05-lifecycle", 0,
        )
        self.assertEqual(dispatched_pa.code.value, "Accepted")
        self.assertEqual(dispatched_pd.code.value, dispatched_pa.code.value)
        self.assertEqual(_p_a_w05_state(dispatched_pa.book), _p_d_w05_state(dispatched_pd.state))
        self.assertEqual(
            next(item for item in dispatched_pa.book.deadlines).status.value,
            "Dispatched",
        )
        self.assertEqual(
            next(item for item in dispatched_pd.state.events).status.value,
            "Dispatched",
        )

        repeated_pa = p_a_dispatch_deadline(
            dispatched_pa.book, identity="w05-lifecycle", generation=0,
        )
        repeated_pd = p_d_dispatch_event(
            dispatched_pd.state, "w05-lifecycle", 0,
        )
        self.assertEqual(repeated_pa.code.value, "Accepted")
        self.assertEqual(repeated_pd.code.value, repeated_pa.code.value)
        self.assertEqual(repeated_pa.book, dispatched_pa.book)
        self.assertEqual(repeated_pd.state, dispatched_pd.state)

        completed_pa = p_a_complete_deadline(
            dispatched_pa.book, identity="w05-lifecycle", generation=0,
        )
        completed_pd = p_d_complete(
            dispatched_pd.state, "w05-lifecycle", 0,
        )
        self.assertEqual(completed_pa.code.value, "Accepted")
        self.assertEqual(completed_pd.code.value, completed_pa.code.value)
        self.assertEqual(_p_a_w05_state(completed_pa.book), _p_d_w05_state(completed_pd.state))

        repeated_complete_pa = p_a_complete_deadline(
            completed_pa.book, identity="w05-lifecycle", generation=0,
        )
        repeated_complete_pd = p_d_complete(
            completed_pd.state, "w05-lifecycle", 0,
        )
        self.assertEqual(repeated_complete_pa.code.value, "InvalidInput")
        self.assertEqual(repeated_complete_pd.code.value, repeated_complete_pa.code.value)
        self.assertEqual(repeated_complete_pa.book, completed_pa.book)
        self.assertEqual(repeated_complete_pd.state, completed_pd.state)

        dispatch_after_complete_pa = p_a_dispatch_deadline(
            completed_pa.book, identity="w05-lifecycle", generation=0,
        )
        dispatch_after_complete_pd = p_d_dispatch_event(
            completed_pd.state, "w05-lifecycle", 0,
        )
        self.assertEqual(dispatch_after_complete_pa.code.value, "Accepted")
        self.assertEqual(dispatch_after_complete_pd.code.value, dispatch_after_complete_pa.code.value)
        self.assertEqual(dispatch_after_complete_pa.book, completed_pa.book)
        self.assertEqual(dispatch_after_complete_pd.state, completed_pd.state)


    def test_p_d_w05_unwatermarked_claim_cannot_dispatch_or_complete(self):
        state = p_d_publish(
            AgendaState(), identity="w05-local-claim", generation=0,
            owner="cpu", at_common=Fraction(5), effect_id="effect",
        ).state
        claimed = p_d_claim_next(state, Fraction(5))
        self.assertEqual(claimed.code, AgendaCode.CLAIMED)
        before = claimed.state
        dispatched = p_d_dispatch_event(before, "w05-local-claim", 0)
        completed = p_d_complete(before, "w05-local-claim", 0)
        self.assertEqual(dispatched.code, AgendaCode.INVARIANT_VIOLATION)
        self.assertEqual(dispatched.state, before)
        self.assertEqual(completed.code, AgendaCode.INVARIANT_VIOLATION)
        self.assertEqual(completed.state, before)


    def test_p_d_w05_dispatched_generation_cannot_be_replaced_until_completed(self):
        names = frozenset(("cpu", "vdp"))
        open_pa_context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        open_pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        pa = p_a_publish_deadline(
            ADeadlineBook(), open_pa_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=0,
            at_common=Fraction(10), effect_id="old",
        )
        pd = p_d_publish_with_context(
            AgendaState(), open_pd_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=0,
            at_common=Fraction(10), effect_id="old",
        )
        pa_closed = p_a_close_watermark(pa.book, Fraction(10))
        pd_closed = p_d_close_watermark(pd.state, Fraction(10), open_pd_context)
        pa_dispatched = p_a_dispatch_deadline(
            pa_closed.book, identity="w05-dispatched-replacement", generation=0,
        )
        pd_dispatched = p_d_dispatch_event(
            pd_closed.state, "w05-dispatched-replacement", 0,
        )
        self.assertEqual(pa_dispatched.code.value, "Accepted")
        self.assertEqual(pd_dispatched.code.value, pa_dispatched.code.value)
        self.assertEqual(_p_a_w05_state(pa_dispatched.book), _p_d_w05_state(pd_dispatched.state))

        closed_pa_context = replace(open_pa_context, watermark_common=Fraction(10))
        closed_pd_context = replace(open_pd_context, watermark_common=Fraction(10))
        rejected_pa = p_a_publish_deadline(
            pa_dispatched.book, closed_pa_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        rejected_pd = p_d_publish_with_context(
            pd_dispatched.state, closed_pd_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        self.assertEqual(rejected_pa.code.value, "InvalidInput")
        self.assertEqual(rejected_pd.code.value, rejected_pa.code.value)
        self.assertEqual(rejected_pa.book, pa_dispatched.book)
        self.assertEqual(rejected_pd.state, pd_dispatched.state)

        completed_pa = p_a_complete_deadline(
            pa_dispatched.book, identity="w05-dispatched-replacement", generation=0,
        )
        completed_pd = p_d_complete(
            pd_dispatched.state, "w05-dispatched-replacement", 0,
        )
        self.assertEqual(completed_pa.code.value, "Accepted")
        self.assertEqual(completed_pd.code.value, completed_pa.code.value)
        allowed_pa = p_a_publish_deadline(
            completed_pa.book, closed_pa_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        allowed_pd = p_d_publish_with_context(
            completed_pd.state, closed_pd_context, owner="cpu",
            identity="w05-dispatched-replacement", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        self.assertEqual(allowed_pa.code.value, "Accepted")
        self.assertEqual(allowed_pd.code.value, allowed_pa.code.value)
        self.assertEqual(_p_a_w05_state(allowed_pa.book), _p_d_w05_state(allowed_pd.state))


    def test_p_d_w05_replaced_old_generation_is_stale_at_dispatch(self):
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
            identity="w05-stale-dispatch", generation=0,
            at_common=Fraction(10), effect_id="old",
        )
        pd = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu",
            identity="w05-stale-dispatch", generation=0,
            at_common=Fraction(10), effect_id="old",
        )
        pa_cancel = p_a_cancel_deadline(
            pa.book, pa_context, owner="cpu", identity="w05-stale-dispatch",
            generation=0, at_common=Fraction(4),
        )
        pd_cancel = p_d_request_cancel(
            pd.state, pd_context, owner="cpu", identity="w05-stale-dispatch",
            generation=0, at_common=Fraction(4),
        )
        pa_arbitrated = p_a_arbitrate_cancellations(pa_cancel.book)
        pd_arbitrated = p_d_arbitrate_cancellations(pd_cancel.state)
        self.assertEqual(pa_arbitrated.code.value, "Accepted")
        self.assertEqual(pd_arbitrated.code.value, pa_arbitrated.code.value)
        pa_next = p_a_publish_deadline(
            pa_arbitrated.book, pa_context, owner="cpu",
            identity="w05-stale-dispatch", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        pd_next = p_d_publish_with_context(
            pd_arbitrated.state, pd_context, owner="cpu",
            identity="w05-stale-dispatch", generation=1,
            at_common=Fraction(12), effect_id="new",
        )
        self.assertEqual(pa_next.code.value, "Accepted")
        self.assertEqual(pd_next.code.value, pa_next.code.value)
        pa_closed = p_a_close_watermark(pa_next.book, Fraction(12))
        pd_closed = p_d_close_watermark(pd_next.state, Fraction(12), pd_context)
        self.assertEqual(pa_closed.claimed, pd_closed.claimed)

        stale_pa = p_a_dispatch_deadline(
            pa_closed.book, identity="w05-stale-dispatch", generation=0,
        )
        stale_pd = p_d_dispatch_event(
            pd_closed.state, "w05-stale-dispatch", 0,
        )
        self.assertEqual(stale_pa.code.value, "StaleIgnored")
        self.assertEqual(stale_pd.code.value, stale_pa.code.value)
        self.assertEqual(stale_pa.book, pa_closed.book)
        self.assertEqual(stale_pd.state, pd_closed.state)

        current_pa = p_a_dispatch_deadline(
            stale_pa.book, identity="w05-stale-dispatch", generation=1,
        )
        current_pd = p_d_dispatch_event(
            stale_pd.state, "w05-stale-dispatch", 1,
        )
        self.assertEqual(current_pa.code.value, "Accepted")
        self.assertEqual(current_pd.code.value, current_pa.code.value)
        self.assertEqual(_p_a_w05_state(current_pa.book), _p_d_w05_state(current_pd.state))


    def test_p_d_w05_dispatch_and_completion_reject_invalid_descriptors_atomically(self):
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
            identity="w05-invalid-lifecycle-descriptor", generation=0,
            at_common=Fraction(10), effect_id="effect",
        )
        pd = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu",
            identity="w05-invalid-lifecycle-descriptor", generation=0,
            at_common=Fraction(10), effect_id="effect",
        )
        cases = (
            ("blank identity", {"identity": " ", "generation": 0}),
            ("non-string identity", {"identity": None, "generation": 0}),
            ("boolean generation", {"identity": "w05-invalid-lifecycle-descriptor", "generation": True}),
            ("negative generation", {"identity": "w05-invalid-lifecycle-descriptor", "generation": -1}),
        )
        for label, descriptor in cases:
            with self.subTest(case=label):
                pa_dispatch = p_a_dispatch_deadline(pa.book, **descriptor)
                pd_dispatch = p_d_dispatch_event(pd.state, **descriptor)
                self.assertEqual(pa_dispatch.code.value, "InvalidInput")
                self.assertEqual(pd_dispatch.code.value, pa_dispatch.code.value)
                self.assertEqual(pa_dispatch.book, pa.book)
                self.assertEqual(pd_dispatch.state, pd.state)

                pa_complete = p_a_complete_deadline(pa.book, **descriptor)
                pd_complete = p_d_complete(pd.state, **descriptor)
                self.assertEqual(pa_complete.code.value, "InvalidInput")
                self.assertEqual(pd_complete.code.value, pa_complete.code.value)
                self.assertEqual(pa_complete.book, pa.book)
                self.assertEqual(pd_complete.state, pd.state)


    def test_p_d_w05_generation_replacement_preserves_successor_microstep(self):
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
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-replace-microstep",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-replace-microstep",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        self.assertEqual(_p_a_w05_state(pa_first.book), _p_d_w05_state(pd_first.state))

        pa_replaced = p_a_replace_deadline(
            pa_first.book, pa_context, owner="cpu", identity="w05-replace-microstep",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect", microstep=True,
        )
        pd_replaced = p_d_replace_with_context(
            pd_first.state, pd_context, owner="cpu", identity="w05-replace-microstep",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect", microstep=True,
        )
        self.assertEqual(pa_replaced.code.value, "Accepted")
        self.assertEqual(pd_replaced.code.value, pa_replaced.code.value)
        self.assertEqual(_p_a_w05_state(pa_replaced.book), _p_d_w05_state(pd_replaced.state))
        self.assertEqual(pa_replaced.book.deadlines[-1].microstep, 1)
        self.assertEqual(pd_replaced.state.events[-1].microstep, 1)

        pa_cancel = p_a_cancel_deadline(
            pa_replaced.book, pa_context, owner="cpu",
            identity="w05-replace-microstep", generation=1,
            at_common=Fraction(20), microstep_level=0,
        )
        pd_cancel = p_d_request_cancel(
            pd_replaced.state, pd_context, owner="cpu",
            identity="w05-replace-microstep", generation=1,
            at_common=Fraction(20), microstep_level=0,
        )
        self.assertEqual(pa_cancel.code.value, "Accepted")
        self.assertEqual(pd_cancel.code.value, pa_cancel.code.value)
        self.assertEqual(_p_a_w05_state(pa_cancel.book), _p_d_w05_state(pd_cancel.state))
        self.assertEqual(
            next(item for item in pa_cancel.book.deadlines if item.generation == 1).status.value,
            "CancelPending",
        )
        self.assertEqual(
            next(item for item in pd_cancel.state.events if item.generation == 1).status.value,
            "CancelPending",
        )


    def test_p_d_w05_generation_replacement_rejects_invalid_cancellation_frontiers_atomically(self):
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
        pa_first = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        pd_first = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, at_common=Fraction(10), effect_id="old-effect",
        )
        self.assertEqual(_p_a_w05_state(pa_first.book), _p_d_w05_state(pd_first.state))

        for cancel_at, expected_code in (
            (Fraction(0), "LateInput"),
            (Fraction(10), "IndeterminateCausalOrder"),
            (Fraction(11), "LateInput"),
        ):
            with self.subTest(cancel_at=cancel_at):
                pa = p_a_replace_deadline(
                    pa_first.book, pa_context, owner="cpu", identity="w05-replace-frontier",
                    generation=0, cancel_at=cancel_at, new_generation=1,
                    new_at_common=Fraction(20), new_effect_id="new-effect",
                )
                pd = p_d_replace_with_context(
                    pd_first.state, pd_context, owner="cpu", identity="w05-replace-frontier",
                    generation=0, cancel_at=cancel_at, new_generation=1,
                    new_at_common=Fraction(20), new_effect_id="new-effect",
                )
                self.assertEqual(pa.code.value, expected_code)
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(_p_a_w05_state(pa.book), _p_d_w05_state(pd.state))
                self.assertEqual(pa.book, pa_first.book)
                self.assertEqual(pd.state, pd_first.state)

        pa_owner = p_a_replace_deadline(
            pa_first.book, pa_context, owner="vdp", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pd_owner = p_d_replace_with_context(
            pd_first.state, pd_context, owner="vdp", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        self.assertEqual(pa_owner.code.value, "InvalidInput")
        self.assertEqual(pd_owner.code.value, pa_owner.code.value)
        self.assertEqual(_p_a_w05_state(pa_owner.book), _p_d_w05_state(pd_owner.state))
        self.assertEqual(pa_owner.book, pa_first.book)
        self.assertEqual(pd_owner.state, pd_first.state)

        incomplete_pa_context = ADeadlineContext(
            (AParticipant("cpu", Fraction(0), Fraction(100)),), names
        )
        incomplete_pd_context = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(100)),), names
        )
        pa_incomplete = p_a_replace_deadline(
            pa_first.book, incomplete_pa_context, owner="cpu",
            identity="w05-replace-frontier", generation=0,
            cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pd_incomplete = p_d_replace_with_context(
            pd_first.state, incomplete_pd_context, owner="cpu",
            identity="w05-replace-frontier", generation=0,
            cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        self.assertEqual(pa_incomplete.code.value, "Blocked")
        self.assertEqual(pd_incomplete.code.value, pa_incomplete.code.value)
        self.assertEqual(_p_a_w05_state(pa_incomplete.book), _p_d_w05_state(pd_incomplete.state))
        self.assertEqual(pa_incomplete.book, pa_first.book)
        self.assertEqual(pd_incomplete.state, pd_first.state)

        pa_success = p_a_replace_deadline(
            pa_first.book, pa_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pd_success = p_d_replace_with_context(
            pd_first.state, pd_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(20), new_effect_id="new-effect",
        )
        pa_non_monotonic = p_a_replace_deadline(
            pa_success.book, pa_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(30), new_effect_id="third-effect",
        )
        pd_non_monotonic = p_d_replace_with_context(
            pd_success.state, pd_context, owner="cpu", identity="w05-replace-frontier",
            generation=0, cancel_at=Fraction(5), new_generation=1,
            new_at_common=Fraction(30), new_effect_id="third-effect",
        )
        self.assertEqual(pa_non_monotonic.code.value, "InvalidInput")
        self.assertEqual(pd_non_monotonic.code.value, pa_non_monotonic.code.value)
        self.assertEqual(
            _p_a_w05_state(pa_non_monotonic.book),
            _p_d_w05_state(pd_non_monotonic.state),
        )
        self.assertEqual(pa_non_monotonic.book, pa_success.book)
        self.assertEqual(pd_non_monotonic.state, pd_success.state)


    def test_p_d_w05_admitted_cancel_must_be_arbitrated_before_watermark_matches_p_a(self):
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
        pa = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-cancel-watermark",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pd = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-cancel-watermark",
            generation=0, at_common=Fraction(10), effect_id="effect",
        )
        pa_cancel = p_a_cancel_deadline(
            pa.book, pa_context, owner="cpu", identity="w05-cancel-watermark",
            generation=0, at_common=Fraction(8),
        )
        pd_cancel = p_d_request_cancel(
            pd.state, pd_context, owner="cpu", identity="w05-cancel-watermark",
            generation=0, at_common=Fraction(8),
        )
        self.assertEqual(pa_cancel.code.value, "Accepted")
        self.assertEqual(pd_cancel.code.value, pa_cancel.code.value)
        self.assertEqual(_p_a_w05_state(pa_cancel.book), _p_d_w05_state(pd_cancel.state))

        pa_close_early = p_a_close_watermark(pa_cancel.book, Fraction(10))
        pd_close_early = p_d_close_watermark(pd_cancel.state, Fraction(10), pd_context)
        self.assertEqual(pa_close_early.code.value, "Blocked")
        self.assertEqual(pd_close_early.code.value, pa_close_early.code.value)
        self.assertEqual(_p_a_w05_state(pa_close_early.book), _p_d_w05_state(pd_close_early.state))
        self.assertEqual(pa_close_early.book, pa_cancel.book)
        self.assertEqual(pd_close_early.state, pd_cancel.state)

        pa_arbitrated = p_a_arbitrate_cancellations(pa_cancel.book)
        pd_arbitrated = p_d_arbitrate_cancellations(pd_cancel.state)
        self.assertEqual(pa_arbitrated.code.value, "Accepted")
        self.assertEqual(pd_arbitrated.code.value, pa_arbitrated.code.value)
        self.assertEqual(_p_a_w05_state(pa_arbitrated.book), _p_d_w05_state(pd_arbitrated.state))

        pa_closed = p_a_close_watermark(pa_arbitrated.book, Fraction(10))
        pd_closed = p_d_close_watermark(pd_arbitrated.state, Fraction(10), pd_context)
        self.assertEqual(pa_closed.code.value, "Accepted")
        self.assertEqual(pd_closed.code.value, pa_closed.code.value)
        self.assertEqual(_p_a_w05_state(pa_closed.book), _p_d_w05_state(pd_closed.state))
        self.assertEqual(pa_closed.book.watermark_common, Fraction(10))
        self.assertEqual(pd_closed.state.watermark_common, Fraction(10))

        pa_live = p_a_publish_deadline(
            ADeadlineBook(), pa_context, owner="cpu", identity="w05-cancel-late",
            generation=0, at_common=Fraction(10), effect_id="late-effect",
        )
        pd_live = p_d_publish_with_context(
            AgendaState(), pd_context, owner="cpu", identity="w05-cancel-late",
            generation=0, at_common=Fraction(10), effect_id="late-effect",
        )
        pa_live_closed = p_a_close_watermark(pa_live.book, Fraction(10))
        pd_live_closed = p_d_close_watermark(pd_live.state, Fraction(10), pd_context)
        self.assertEqual(_p_a_w05_state(pa_live_closed.book), _p_d_w05_state(pd_live_closed.state))
        pa_closed_context = replace(
            pa_context, watermark_common=Fraction(10)
        )
        pd_closed_context = replace(
            pd_context, watermark_common=Fraction(10)
        )
        pa_late = p_a_cancel_deadline(
            pa_live_closed.book, pa_closed_context, owner="cpu", identity="w05-cancel-late",
            generation=0, at_common=Fraction(11),
        )
        pd_late = p_d_request_cancel(
            pd_live_closed.state, pd_closed_context, owner="cpu", identity="w05-cancel-late",
            generation=0, at_common=Fraction(11),
        )
        self.assertEqual(pa_late.code.value, "LateInput")
        self.assertEqual(pd_late.code.value, pa_late.code.value)
        self.assertEqual(_p_a_w05_state(pa_late.book), _p_d_w05_state(pd_late.state))
        self.assertEqual(pa_late.book, pa_live_closed.book)
        self.assertEqual(pd_late.state, pd_live_closed.state)


    def test_p_d_w05_invalid_cancellation_descriptors_match_p_a_without_mutation(self):
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
        cases = (
            ("owner mismatch", {"owner": "vdp"}),
            ("unknown identity", {"identity": "missing"}),
            ("boolean generation", {"generation": True}),
            ("non-exact coordinate", {"at_common": 8.5}),
            ("negative microstep", {"microstep_level": -1}),
            ("phase order without evidence", {"phase_order": 1}),
            ("evidence without phase order", {"phase_evidence": "phase"}),
        )
        for label, overrides in cases:
            with self.subTest(case=label):
                pa_published = p_a_publish_deadline(
                    ADeadlineBook(), pa_context, owner="cpu",
                    identity="w05-invalid-descriptor", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                pd_published = p_d_publish_with_context(
                    AgendaState(), pd_context, owner="cpu",
                    identity="w05-invalid-descriptor", generation=0,
                    at_common=Fraction(10), effect_id="effect",
                )
                self.assertEqual(pa_published.code.value, "Accepted")
                self.assertEqual(pd_published.code.value, "Accepted")
                self.assertEqual(
                    _p_a_w05_state(pa_published.book),
                    _p_d_w05_state(pd_published.state),
                )
                arguments = {
                    "owner": "cpu",
                    "identity": "w05-invalid-descriptor",
                    "generation": 0,
                    "at_common": Fraction(8),
                }
                arguments.update(overrides)
                pa = p_a_cancel_deadline(
                    pa_published.book, pa_context, **arguments
                )
                pd = p_d_request_cancel(
                    pd_published.state, pd_context, **arguments
                )
                self.assertEqual(pa.code.value, "InvalidInput")
                self.assertEqual(pd.code.value, pa.code.value)
                self.assertEqual(
                    _p_a_w05_state(pa.book),
                    _p_d_w05_state(pd.state),
                )
                self.assertEqual(pa.book, pa_published.book)
                self.assertEqual(pd.state, pd_published.state)


    def test_p_d_w05_claimed_and_processed_states_require_closed_watermark_on_every_operation(self):
        names = frozenset(("cpu",))
        base_context = AgendaContext(
            (AgendaParticipant("cpu", Fraction(0), Fraction(100)),),
            names,
        )
        for status in (
            EventStatus.CLAIMED,
            EventStatus.DISPATCHED,
            EventStatus.COMPLETED,
        ):
            for watermark in (None, Fraction(9)):
                with self.subTest(status=status, watermark=watermark):
                    event = AgendaEvent(
                        "w05-prebuilt-lifecycle", 1, "cpu", Fraction(10),
                        "effect", status,
                    )
                    state = AgendaState(
                        heap=(), events=(event,), watermark_common=watermark,
                    )
                    context = replace(
                        base_context, watermark_common=watermark,
                    )
                    operations = (
                        p_d_publish_with_context(
                            state, context, owner="cpu", identity="new",
                            generation=1, at_common=Fraction(20),
                            effect_id="new-effect",
                        ),
                        p_d_request_cancel(
                            state, context, owner="cpu",
                            identity="w05-prebuilt-lifecycle", generation=1,
                            at_common=Fraction(5),
                        ),
                        p_d_arbitrate_cancellations(state),
                        p_d_close_watermark(state, Fraction(20), context),
                        p_d_dispatch_event(
                            state, "w05-prebuilt-lifecycle", 1,
                        ),
                        p_d_complete(
                            state, "w05-prebuilt-lifecycle", 1,
                        ),
                        p_d_replace_with_context(
                            state, context, owner="cpu",
                            identity="w05-prebuilt-lifecycle", generation=1,
                            cancel_at=Fraction(5), new_generation=2,
                            new_at_common=Fraction(20),
                            new_effect_id="replacement-effect",
                        ),
                    )
                    for result in operations:
                        self.assertEqual(
                            result.code, AgendaCode.INVARIANT_VIOLATION,
                        )
                        self.assertEqual(result.state, state)



if __name__ == "__main__":
    unittest.main(verbosity=2)
