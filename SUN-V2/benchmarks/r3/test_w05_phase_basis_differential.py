"""W05 differential: same-instant cancellation phase-basis boundaries.

These tests compare only the synthetic P-A/P-D contract. A structurally valid
phase basis is not evidence that a physical SMS phase relationship is known.
"""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_a_deadline_lifecycle import (
    ADeadlineBook,
    ADeadlineContext,
    ADeadlineStatus,
    AParticipant,
    APhaseBasis,
    arbitrate_cancellations as pa_arbitrate,
    cancel_deadline as pa_cancel,
    publish_deadline as pa_publish,
)
from p_d_discrete_agenda import (
    AgendaContext,
    AgendaParticipant,
    AgendaPhaseBasis,
    AgendaState,
    EventStatus,
    arbitrate_cancellations as pd_arbitrate,
    publish_with_context as pd_publish,
    request_cancel as pd_cancel,
)


def _basis(revision: str = "rev-1"):
    values = (
        "SMS-NTSC", "profile-rev-1", "common-phase", "phase-unit",
        1, 1, "sms-phase-basis", revision, "validation-record",
        "a" * 64,
    )
    return APhaseBasis(*values), AgendaPhaseBasis(*values)


def _pa_state(book):
    return (
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.effect_id, item.status.value, item.microstep,
            item.phase_order, item.phase_evidence,
            None if item.phase_basis is None else (
                item.phase_basis.profile_id,
                item.phase_basis.profile_revision,
                item.phase_basis.coordinate,
                item.phase_basis.unit,
                item.phase_basis.scale_numerator,
                item.phase_basis.scale_denominator,
                item.phase_basis.basis_id,
                item.phase_basis.basis_revision,
                item.phase_basis.validation_record_id,
                item.phase_basis.validation_record_sha256,
            ),
        ) for item in book.deadlines)),
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
            None if item.phase_basis is None else (
                item.phase_basis.profile_id,
                item.phase_basis.profile_revision,
                item.phase_basis.coordinate,
                item.phase_basis.unit,
                item.phase_basis.scale_numerator,
                item.phase_basis.scale_denominator,
                item.phase_basis.basis_id,
                item.phase_basis.basis_revision,
                item.phase_basis.validation_record_id,
                item.phase_basis.validation_record_sha256,
            ),
        ) for item in book.pending_cancellations)),
        book.watermark_common,
    )


def _pd_state(state):
    return (
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.effect_id, item.status.value, item.microstep,
            item.phase_order, item.phase_evidence,
            None if item.phase_basis is None else (
                item.phase_basis.profile_id,
                item.phase_basis.profile_revision,
                item.phase_basis.coordinate,
                item.phase_basis.unit,
                item.phase_basis.scale_numerator,
                item.phase_basis.scale_denominator,
                item.phase_basis.basis_id,
                item.phase_basis.basis_revision,
                item.phase_basis.validation_record_id,
                item.phase_basis.validation_record_sha256,
            ),
        ) for item in state.events)),
        tuple(sorted((
            item.identity, item.generation, item.owner, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
            None if item.phase_basis is None else (
                item.phase_basis.profile_id,
                item.phase_basis.profile_revision,
                item.phase_basis.coordinate,
                item.phase_basis.unit,
                item.phase_basis.scale_numerator,
                item.phase_basis.scale_denominator,
                item.phase_basis.basis_id,
                item.phase_basis.basis_revision,
                item.phase_basis.validation_record_id,
                item.phase_basis.validation_record_sha256,
            ),
        ) for item in state.pending_cancellations)),
        state.watermark_common,
    )


class W05SameInstantPhaseBasisDifferentialTests(unittest.TestCase):
    def setUp(self):
        names = frozenset(("cpu", "vdp"))
        self.pa_context = ADeadlineContext(
            (
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )
        self.pd_context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            names,
        )

    def _run_case(self, *, deadline_order, cancel_order, deadline_basis,
                  cancel_basis, expected):
        pa_deadline_basis, pd_deadline_basis = deadline_basis
        pa_cancel_basis, pd_cancel_basis = cancel_basis
        pa = pa_publish(
            ADeadlineBook(), self.pa_context,
            owner="cpu", identity="same-instant-phase", generation=0,
            at_common=Fraction(10), effect_id="effect",
            phase_order=deadline_order, phase_evidence="deadline-phase",
            phase_basis=pa_deadline_basis,
        )
        pd = pd_publish(
            AgendaState(), self.pd_context,
            owner="cpu", identity="same-instant-phase", generation=0,
            at_common=Fraction(10), effect_id="effect",
            phase_order=deadline_order, phase_evidence="deadline-phase",
            phase_basis=pd_deadline_basis,
        )
        self.assertEqual(pa.code.value, "Accepted")
        self.assertEqual(pd.code.value, pa.code.value)
        self.assertEqual(_pa_state(pa.book), _pd_state(pd.state))

        pa_cancelled = pa_cancel(
            pa.book, self.pa_context,
            owner="cpu", identity="same-instant-phase", generation=0,
            at_common=Fraction(10), phase_order=cancel_order,
            phase_evidence="cancellation-phase", phase_basis=pa_cancel_basis,
        )
        pd_cancelled = pd_cancel(
            pd.state, self.pd_context,
            owner="cpu", identity="same-instant-phase", generation=0,
            at_common=Fraction(10), phase_order=cancel_order,
            phase_evidence="cancellation-phase", phase_basis=pd_cancel_basis,
        )
        self.assertEqual(pa_cancelled.code.value, expected)
        self.assertEqual(pd_cancelled.code.value, pa_cancelled.code.value)
        self.assertEqual(_pa_state(pa_cancelled.book), _pd_state(pd_cancelled.state))

        if expected == "Accepted":
            pa_final = pa_arbitrate(pa_cancelled.book)
            pd_final = pd_arbitrate(pd_cancelled.state)
            self.assertEqual(pa_final.code.value, "Accepted")
            self.assertEqual(pd_final.code.value, pa_final.code.value)
            self.assertEqual(_pa_state(pa_final.book), _pd_state(pd_final.state))
            self.assertEqual(pa_final.book.deadlines[0].status, ADeadlineStatus.CANCELLED)
            self.assertEqual(pd_final.state.events[0].status, EventStatus.CANCELLED)
        else:
            self.assertEqual(_pa_state(pa_cancelled.book), _pa_state(pa.book))
            self.assertEqual(_pd_state(pd_cancelled.state), _pd_state(pd.state))

    def test_earlier_phase_on_identical_basis_admits_cancellation(self):
        pa_basis, pd_basis = _basis()
        self._run_case(
            deadline_order=2, cancel_order=1,
            deadline_basis=(pa_basis, pd_basis),
            cancel_basis=(pa_basis, pd_basis),
            expected="Accepted",
        )

    def test_later_phase_on_identical_basis_rejects_cancellation(self):
        pa_basis, pd_basis = _basis()
        self._run_case(
            deadline_order=2, cancel_order=3,
            deadline_basis=(pa_basis, pd_basis),
            cancel_basis=(pa_basis, pd_basis),
            expected="LateInput",
        )

    def test_different_phase_basis_keeps_same_instant_order_unknown(self):
        pa_deadline, pd_deadline = _basis("rev-1")
        pa_cancel, pd_cancel = _basis("rev-2")
        self._run_case(
            deadline_order=2, cancel_order=1,
            deadline_basis=(pa_deadline, pd_deadline),
            cancel_basis=(pa_cancel, pd_cancel),
            expected="IndeterminateCausalOrder",
        )

    def test_missing_phase_basis_keeps_same_instant_order_unknown(self):
        pa_basis, pd_basis = _basis()
        self._run_case(
            deadline_order=2, cancel_order=1,
            deadline_basis=(pa_basis, pd_basis),
            cancel_basis=(None, None),
            expected="IndeterminateCausalOrder",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
