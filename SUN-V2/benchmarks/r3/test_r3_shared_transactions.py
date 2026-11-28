"""W08 differential tests for the strategy-neutral transaction book carried by P-D/P-E."""
from __future__ import annotations

import sys
import unittest
from dataclasses import replace
from fractions import Fraction
from pathlib import Path

REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(REFERENCE_PATH))

from p_a_transactions import (
    PAMarker,
    PAMarkerKind,
    PATransactionBook,
    PATransactionSpec,
    commit_effect as pa_commit_effect,
    complete as pa_complete,
    consume_base_time as pa_consume_base_time,
    consume_tw as pa_consume_tw,
    enter_waiting as pa_enter_waiting,
    fail as pa_fail,
    reach_next_marker as pa_reach_next_marker,
    recover as pa_recover,
    release_waiting as pa_release_waiting,
    sample_marker as pa_sample_marker,
    start as pa_start,
    submit as pa_submit,
    cancel as pa_cancel,
)
from p_d_discrete_agenda import AgendaState, apply_transaction as pd_apply_transaction
from p_e_sync_on_demand import (
    SyncDomain,
    SyncState,
    apply_transaction as pe_apply_transaction,
)
from transaction_reference import (
    Marker as R2Marker,
    MarkerKind as R2MarkerKind,
    TransactionBook as R2TransactionBook,
    TransactionSpec as R2TransactionSpec,
    submit as r2_submit,
)

from r3_transaction_core import (
    MarkerKind,
    TxMarker,
    TxSpec,
    cancel,
    commit_effect,
    complete,
    consume_base_time,
    consume_tw,
    enter_waiting,
    fail,
    reach_next_marker,
    recover,
    release_waiting,
    sample_marker,
    start,
    submit,
)


def make_pair_spec(identity="tx-main", *, payload="payload", markers=None,
                   duration=2, automatic=2, external=1):
    marker_items = markers or (
        PAMarker("sample", PAMarkerKind.SAMPLE, Fraction(1)),
        PAMarker("commit", PAMarkerKind.COMMIT, Fraction(2)),
    )
    pa = PATransactionSpec(
        identity, 0, "cpu", "vdp", "write", "SMS-NTSC", payload,
        Fraction(0), Fraction(0), Fraction(duration), automatic, external,
        tuple(marker_items),
    )
    r3 = TxSpec(
        identity, 0, "cpu", "vdp", "write", "SMS-NTSC", payload,
        Fraction(0), Fraction(0), Fraction(duration), automatic, external,
        tuple(
            TxMarker(item.identity, MarkerKind(item.kind.value), item.at_common, item.irreversible)
            for item in marker_items
        ),
    )
    return pa, r3


def canonical(tx):
    if tx is None:
        return None
    spec = tx.spec
    return (
        spec.identity, spec.generation, spec.origin, spec.destination,
        spec.operation, spec.profile, spec.payload, spec.admitted_at,
        spec.start_at, spec.duration_base, spec.automatic_tws,
        spec.external_tws,
        tuple((m.identity, m.kind.value, m.at_common, m.irreversible) for m in spec.markers),
        tx.state.value, tx.progress_common, tx.base_consumed,
        tx.automatic_tw_consumed, tx.external_tw_consumed,
        tx.ready_marker, tx.wait_source, tx.sampled, tx.committed,
        tx.failure, tx.recoveries,
    )


class SharedTransactionCoreTests(unittest.TestCase):
    def setUp(self):
        self.pa_book = PATransactionBook()
        self.pd_state = AgendaState()
        self.pe_state = SyncState(
            domains=(SyncDomain("cpu", Fraction(0), Fraction(100)),),
            transitions=(),
        )

    def apply(self, pa_operation, r3_operation, *args, r3_args=None, **kwargs):
        pa_result = pa_operation(self.pa_book, *args, **kwargs)
        candidate_args = args if r3_args is None else r3_args
        self.pd_state, pd_result = pd_apply_transaction(
            self.pd_state, r3_operation, *candidate_args, **kwargs
        )
        self.pe_state, pe_result = pe_apply_transaction(
            self.pe_state, r3_operation, *candidate_args, **kwargs
        )
        self.assertEqual(pa_result.code.value, pd_result.code.value)
        self.assertEqual(pa_result.code.value, pe_result.code.value)
        self.assertEqual(
            tuple(canonical(item) for item in pa_result.book.transactions),
            tuple(canonical(item) for item in pd_result.book.transactions),
        )
        self.assertEqual(
            tuple(canonical(item) for item in pa_result.book.transactions),
            tuple(canonical(item) for item in pe_result.book.transactions),
        )
        self.assertEqual(canonical(pa_result.transaction), canonical(pd_result.transaction))
        self.assertEqual(canonical(pa_result.transaction), canonical(pe_result.transaction))
        self.assertEqual(pa_result.value, pd_result.value)
        self.assertEqual(pa_result.value, pe_result.value)
        self.pa_book = pa_result.book
        self.assertEqual(self.pd_state.transactions, pd_result.book)
        self.assertEqual(self.pe_state.transactions, pe_result.book)
        return pa_result

    def test_w08_full_sample_commit_wait_tw_and_completion_lifecycle(self):
        pa_spec, r3_spec = make_pair_spec()
        pa_result = pa_submit(self.pa_book, pa_spec)
        pd_state, pd_result = pd_apply_transaction(self.pd_state, submit, r3_spec)
        pe_state, pe_result = pe_apply_transaction(self.pe_state, submit, r3_spec)
        self.assertEqual(pa_result.code.value, pd_result.code.value)
        self.assertEqual(pa_result.code.value, pe_result.code.value)
        self.pa_book = pa_result.book
        self.pd_state = pd_state
        self.pe_state = pe_state
        self.assertEqual(
            tuple(canonical(item) for item in self.pa_book.transactions),
            tuple(canonical(item) for item in self.pd_state.transactions.transactions),
        )
        self.assertEqual(self.pd_state.transactions, self.pe_state.transactions)

        # Identical retry is idempotent; divergent descriptor reuse is invalid.
        self.apply(pa_submit, submit, pa_spec, r3_args=(r3_spec,))
        divergent_pa, divergent_r3 = make_pair_spec(payload="different")
        self.apply(pa_submit, submit, divergent_pa, r3_args=(divergent_r3,))

        self.apply(pa_start, start, "tx-main")
        self.apply(pa_enter_waiting, enter_waiting, "tx-main", "dependency")
        self.apply(pa_release_waiting, release_waiting, "tx-main", "dependency")
        self.apply(pa_consume_base_time, consume_base_time, "tx-main", Fraction(1))
        self.apply(pa_consume_tw, consume_tw, "tx-main", automatic=True)
        self.apply(pa_consume_tw, consume_tw, "tx-main", automatic=False)
        self.apply(pa_reach_next_marker, reach_next_marker, "tx-main", Fraction(1))
        self.apply(pa_sample_marker, sample_marker, "tx-main", "sample", "sampled-bus-value")
        retry = self.apply(
            pa_sample_marker, sample_marker, "tx-main", "sample", "different-retry-value"
        )
        self.assertEqual(retry.value, "sampled-bus-value")
        self.apply(pa_reach_next_marker, reach_next_marker, "tx-main", Fraction(2))
        self.apply(pa_commit_effect, commit_effect, "tx-main", "commit")
        self.apply(pa_commit_effect, commit_effect, "tx-main", "commit")
        self.apply(pa_consume_base_time, consume_base_time, "tx-main", Fraction(1))
        self.apply(pa_consume_tw, consume_tw, "tx-main", automatic=True)
        completed = self.apply(pa_complete, complete, "tx-main")
        self.assertEqual(completed.transaction.state.value, "Completed")

    def test_w08_generation_is_part_of_stable_identity_retry_descriptor_against_r2(self):
        pa_spec, r3_spec = make_pair_spec("tx-generation")
        r2_spec = R2TransactionSpec(
            pa_spec.identity, pa_spec.generation, pa_spec.origin,
            pa_spec.destination, pa_spec.operation, pa_spec.profile,
            pa_spec.payload, pa_spec.admitted_at, pa_spec.start_at,
            pa_spec.duration_base, pa_spec.automatic_tws,
            pa_spec.external_tws,
            tuple(
                R2Marker(
                    item.identity, R2MarkerKind(item.kind.value),
                    item.at_common, item.irreversible,
                )
                for item in pa_spec.markers
            ),
        )

        pa_first = pa_submit(PATransactionBook(), pa_spec)
        r2_first = r2_submit(R2TransactionBook(), r2_spec)
        pd_state, pd_first = pd_apply_transaction(AgendaState(), submit, r3_spec)
        pe_state, pe_first = pe_apply_transaction(self.pe_state, submit, r3_spec)
        self.assertEqual(pa_first.code.value, "OK")
        self.assertEqual(r2_first.code.value, pa_first.code.value)
        self.assertEqual(pd_first.code.value, pa_first.code.value)
        self.assertEqual(pe_first.code.value, pa_first.code.value)
        expected = canonical(pa_first.transaction)
        self.assertEqual(canonical(r2_first.transaction), expected)
        self.assertEqual(canonical(pd_first.transaction), expected)
        self.assertEqual(canonical(pe_first.transaction), expected)

        # A changed generation under the same stable identity is a divergent
        # retry descriptor, not an implicit generation replacement.
        pa_retry = pa_submit(pa_first.book, replace(pa_spec, generation=1))
        r2_retry = r2_submit(r2_first.book, replace(r2_spec, generation=1))
        pd_retry_state, pd_retry = pd_apply_transaction(
            pd_state, submit, replace(r3_spec, generation=1)
        )
        pe_retry_state, pe_retry = pe_apply_transaction(
            pe_state, submit, replace(r3_spec, generation=1)
        )
        for result in (pa_retry, r2_retry, pd_retry, pe_retry):
            self.assertEqual(result.code.value, "InvalidInput")
        self.assertEqual(pa_retry.book, pa_first.book)
        self.assertEqual(r2_retry.book, r2_first.book)
        self.assertEqual(pd_retry_state, pd_state)
        self.assertEqual(pe_retry_state, pe_state)
        self.assertEqual(canonical(pa_retry.transaction), expected)
        self.assertEqual(canonical(r2_retry.transaction), expected)
        self.assertEqual(canonical(pd_retry.transaction), expected)
        self.assertEqual(canonical(pe_retry.transaction), expected)


    def test_w08_all_wait_sources_are_differentially_distinct(self):
        for source in ("external", "fixture", "dependency"):
            with self.subTest(source=source):
                identity = f"tx-wait-{source}"
                pa_spec, r3_spec = make_pair_spec(
                    identity, duration=0, automatic=0, external=0
                )
                self.apply(pa_submit, submit, pa_spec, r3_args=(r3_spec,))
                self.apply(pa_start, start, identity)
                waiting = self.apply(
                    pa_enter_waiting, enter_waiting, identity, source
                )
                self.assertEqual(waiting.transaction.wait_source, source)

                wrong_source = next(
                    candidate
                    for candidate in ("external", "fixture", "dependency")
                    if candidate != source
                )
                rejected = self.apply(
                    pa_release_waiting, release_waiting,
                    identity, wrong_source,
                )
                self.assertEqual(rejected.code.value, "InvariantViolation")
                self.assertEqual(rejected.transaction.wait_source, source)

                resumed = self.apply(
                    pa_release_waiting, release_waiting, identity, source
                )
                self.assertEqual(resumed.transaction.state.value, "InProgress")
                self.assertIsNone(resumed.transaction.wait_source)

    def test_w08_failure_recovery_and_irreversible_cancellation(self):
        pa_spec, r3_spec = make_pair_spec(
            "tx-fail",
            markers=(PAMarker("commit", PAMarkerKind.COMMIT, Fraction(1)),),
            duration=0, automatic=0, external=0,
        )
        self.apply(pa_submit, submit, pa_spec, r3_args=(r3_spec,))
        self.apply(pa_start, start, "tx-fail")
        self.apply(pa_reach_next_marker, reach_next_marker, "tx-fail", Fraction(1))
        self.apply(pa_commit_effect, commit_effect, "tx-fail", "commit")
        failed = self.apply(pa_fail, fail, "tx-fail", "failure after partial commit")
        self.assertEqual(failed.transaction.committed, ("commit",))
        self.apply(pa_recover, recover, "tx-fail")
        self.apply(pa_commit_effect, commit_effect, "tx-fail", "commit")
        self.apply(pa_complete, complete, "tx-fail")

        cancel_markers = (PAMarker("sample", PAMarkerKind.SAMPLE, Fraction(1), True),)
        pa_cancel_spec, r3_cancel_spec = make_pair_spec(
            "tx-cancel", markers=cancel_markers, duration=0, automatic=0, external=0
        )
        self.apply(pa_submit, submit, pa_cancel_spec, r3_args=(r3_cancel_spec,))
        self.apply(pa_start, start, "tx-cancel")
        self.apply(pa_cancel, cancel, "tx-cancel")

        pa_late_spec, r3_late_spec = make_pair_spec(
            "tx-late", markers=cancel_markers, duration=0, automatic=0, external=0
        )
        self.apply(pa_submit, submit, pa_late_spec, r3_args=(r3_late_spec,))
        self.apply(pa_start, start, "tx-late")
        self.apply(pa_reach_next_marker, reach_next_marker, "tx-late", Fraction(1))
        self.apply(pa_sample_marker, sample_marker, "tx-late", "sample", "irreversible-read")
        late = self.apply(pa_cancel, cancel, "tx-late")
        self.assertEqual(late.code.value, "LateInput")


if __name__ == "__main__":
    unittest.main()
