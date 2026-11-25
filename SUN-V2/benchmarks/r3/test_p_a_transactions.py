"""P-A W08 transaction lifecycle differential tests against R2."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_transactions import (
    PAMarker, PAMarkerKind, PATransactionBook, PATransactionSpec,
    PATxCode, PATxState, cancel, commit_effect, complete, consume_base_time,
    consume_tw, enter_waiting, fail, reach_next_marker, recover,
    release_waiting, sample_marker, start, submit,
)
from transaction_reference import (
    Marker as R2Marker, MarkerKind as R2MarkerKind,
    TransactionBook as R2Book, TransactionSpec as R2Spec,
    TxCode as R2Code, TxState as R2State,
    cancel as r2_cancel, commit_effect as r2_commit_effect,
    complete as r2_complete, consume_base_time as r2_consume_base_time,
    consume_tw as r2_consume_tw, enter_waiting as r2_enter_waiting,
    fail as r2_fail, reach_next_marker as r2_reach_next_marker,
    recover as r2_recover, release_waiting as r2_release_waiting,
    sample_marker as r2_sample_marker, start as r2_start, submit as r2_submit,
)


def to_r2_spec(spec: PATransactionSpec) -> R2Spec:
    return R2Spec(
        spec.identity, spec.generation, spec.origin, spec.destination,
        spec.operation, spec.profile, spec.payload, spec.admitted_at,
        spec.start_at, spec.duration_base, spec.automatic_tws,
        spec.external_tws,
        tuple(
            R2Marker(
                item.identity, R2MarkerKind(item.kind.value),
                item.at_common, item.irreversible,
            )
            for item in spec.markers
        ),
    )


def to_r2_book(book: PATransactionBook) -> R2Book:
    from transaction_reference import Transaction as R2Transaction
    from transaction_reference import TxState as R2TxState
    from dataclasses import replace

    transactions = []
    for tx in book.transactions:
        spec = to_r2_spec(tx.spec)
        transactions.append(R2Transaction(
            spec=spec,
            state=R2TxState(tx.state.value),
            progress_common=tx.progress_common,
            base_consumed=tx.base_consumed,
            automatic_tw_consumed=tx.automatic_tw_consumed,
            external_tw_consumed=tx.external_tw_consumed,
            ready_marker=tx.ready_marker,
            wait_source=tx.wait_source,
            sampled=tx.sampled,
            committed=tx.committed,
            failure=tx.failure,
            recoveries=tx.recoveries,
        ))
    return R2Book(tuple(transactions))


def canonical_tx(tx):
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


def make_spec(*, markers=(), duration=0, automatic=0, external=0, payload="data",
              identity="tx-1", generation=0):
    return PATransactionSpec(
        identity, generation, "cpu", "vdp", "write", "SMS-NTSC", payload,
        Fraction(0), Fraction(0), Fraction(duration), automatic, external,
        tuple(markers),
    )


def marker(identity, kind, at, irreversible=False):
    return PAMarker(identity, kind, Fraction(at), irreversible)


class PATransactionTests(unittest.TestCase):
    def compare(self, actual, expected):
        self.assertEqual(actual.code.value, expected.code.value)
        self.assertEqual(
            tuple(canonical_tx(tx) for tx in actual.book.transactions),
            tuple(canonical_tx(tx) for tx in expected.book.transactions),
        )
        self.assertEqual(canonical_tx(actual.transaction), canonical_tx(expected.transaction))
        self.assertEqual(actual.value, expected.value)

    def pair(self, operation, reference, book, *args, **kwargs):
        actual = operation(book, *args, **kwargs)
        expected = reference(to_r2_book(book), *args, **kwargs)
        self.compare(actual, expected)
        return actual

    def admitted_pair(self, spec=None):
        descriptor = spec or make_spec()
        actual = submit(PATransactionBook(), descriptor)
        expected = r2_submit(R2Book(), to_r2_spec(descriptor))
        self.compare(actual, expected)
        return actual, expected

    def test_identical_retry_returns_existing_pending_state(self):
        spec = make_spec()
        actual, expected = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        expected = r2_start(expected.book, "tx-1")
        retry = submit(actual.book, spec)
        retry_ref = r2_submit(expected.book, to_r2_spec(spec))
        self.compare(retry, retry_ref)
        self.assertEqual(retry.book, actual.book)

    def test_duplicate_id_with_divergent_descriptor_is_invalid_without_mutation(self):
        actual, expected = self.admitted_pair()
        divergent = make_spec(payload="different")
        result = submit(actual.book, divergent)
        result_ref = r2_submit(expected.book, to_r2_spec(divergent))
        self.compare(result, result_ref)
        self.assertEqual(result.code, PATxCode.INVALID_INPUT)
        self.assertEqual(result.book, actual.book)

    def test_start_is_idempotent_only_while_in_progress(self):
        actual, expected = self.admitted_pair()
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        expected = r2_start(expected.book, "tx-1")
        repeated = self.pair(start, r2_start, actual.book, "tx-1")
        self.assertEqual(repeated.book, actual.book)
        cancelled = self.pair(cancel, r2_cancel, actual.book, "tx-1")
        failed_start = self.pair(start, r2_start, cancelled.book, "tx-1")
        self.assertEqual(failed_start.code, PATxCode.INVARIANT_VIOLATION)

    def test_waiting_and_resume_preserve_identity_and_progress(self):
        actual, _ = self.admitted_pair()
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            enter_waiting, r2_enter_waiting, actual.book, "tx-1", "dependency"
        )
        waiting = actual.transaction
        resumed = self.pair(
            release_waiting, r2_release_waiting, actual.book, "tx-1", "dependency"
        )
        self.assertEqual(waiting.state, PATxState.WAITING)
        self.assertEqual(resumed.transaction.state, PATxState.IN_PROGRESS)
        self.assertEqual(
            resumed.transaction.progress_common, waiting.progress_common
        )
        wrong_source = self.pair(
            release_waiting, r2_release_waiting, actual.book, "tx-1", "external"
        )
        self.assertEqual(wrong_source.code, PATxCode.INVARIANT_VIOLATION)

    def test_sample_marker_is_captured_once_and_retry_returns_original_value(self):
        spec = make_spec(markers=(marker("read", PAMarkerKind.SAMPLE, 2),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(2)
        )
        first = self.pair(
            sample_marker, r2_sample_marker, actual.book, "tx-1", "read", "0x44"
        )
        retry = self.pair(
            sample_marker, r2_sample_marker, first.book, "tx-1", "read", "0x99"
        )
        self.assertEqual(first.value, "0x44")
        self.assertEqual(retry.value, "0x44")
        self.assertEqual(retry.book, first.book)

    def test_commit_effect_is_applied_exactly_once(self):
        spec = make_spec(markers=(marker("write", PAMarkerKind.COMMIT, 3),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(3)
        )
        committed = self.pair(
            commit_effect, r2_commit_effect, actual.book, "tx-1", "write"
        )
        retry = self.pair(
            commit_effect, r2_commit_effect, committed.book, "tx-1", "write"
        )
        self.assertEqual(retry.book, committed.book)
        self.assertEqual(retry.transaction.committed, ("write",))

    def test_cancel_before_first_irreversible_effect_is_allowed(self):
        actual, _ = self.admitted_pair()
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        cancelled = self.pair(cancel, r2_cancel, actual.book, "tx-1")
        self.assertEqual(cancelled.code, PATxCode.OK)
        self.assertEqual(cancelled.transaction.state, PATxState.CANCELLED)

    def test_cancel_after_commit_is_late_and_preserves_partial_ledger(self):
        spec = make_spec(markers=(marker("write", PAMarkerKind.COMMIT, 1),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(1)
        )
        actual = self.pair(
            commit_effect, r2_commit_effect, actual.book, "tx-1", "write"
        )
        result = self.pair(cancel, r2_cancel, actual.book, "tx-1")
        self.assertEqual(result.code, PATxCode.LATE_INPUT)
        self.assertEqual(result.transaction.committed, ("write",))

    def test_irreversible_sample_closes_cancel_window_only_when_declared(self):
        for irreversible, expected_code in (
            (False, PATxCode.OK), (True, PATxCode.LATE_INPUT)
        ):
            with self.subTest(irreversible=irreversible):
                spec = make_spec(
                    markers=(marker("read", PAMarkerKind.SAMPLE, 1, irreversible),)
                )
                actual, _ = self.admitted_pair(spec)
                actual = self.pair(start, r2_start, actual.book, "tx-1")
                actual = self.pair(
                    reach_next_marker, r2_reach_next_marker,
                    actual.book, "tx-1", Fraction(1),
                )
                actual = self.pair(
                    sample_marker, r2_sample_marker,
                    actual.book, "tx-1", "read", "0x81",
                )
                cancelled = self.pair(cancel, r2_cancel, actual.book, "tx-1")
                self.assertEqual(cancelled.code, expected_code)

    def test_failure_before_effect_preserves_pending_and_requires_explicit_recovery(self):
        spec = make_spec(markers=(marker("write", PAMarkerKind.COMMIT, 1),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        failed = self.pair(fail, r2_fail, actual.book, "tx-1", "bus fault")
        retry = submit(failed.book, spec)
        retry_ref = r2_submit(to_r2_book(failed.book), to_r2_spec(spec))
        self.compare(retry, retry_ref)
        self.assertEqual(retry.transaction.state, PATxState.FAILED)
        self.assertEqual(retry.transaction.committed, ())
        recovered = self.pair(recover, r2_recover, failed.book, "tx-1")
        self.assertEqual(recovered.transaction.state, PATxState.IN_PROGRESS)
        self.assertEqual(recovered.transaction.recoveries, 1)

    def test_failure_after_partial_commit_preserves_exact_ledger(self):
        spec = make_spec(markers=(
            marker("write-1", PAMarkerKind.COMMIT, 1),
            marker("write-2", PAMarkerKind.COMMIT, 2),
        ))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(1)
        )
        actual = self.pair(
            commit_effect, r2_commit_effect, actual.book, "tx-1", "write-1"
        )
        failed = self.pair(fail, r2_fail, actual.book, "tx-1", "partial bus fault")
        self.assertEqual(failed.transaction.committed, ("write-1",))
        self.assertEqual(failed.transaction.failure, "partial bus fault")
        self.assertEqual(failed.transaction.state, PATxState.FAILED)

    def test_commit_before_its_reached_frontier_is_invariant_violation(self):
        spec = make_spec(markers=(marker("write", PAMarkerKind.COMMIT, 2),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        early = self.pair(
            commit_effect, r2_commit_effect, actual.book, "tx-1", "write"
        )
        self.assertEqual(early.code, PATxCode.INVARIANT_VIOLATION)
        self.assertEqual(early.book, actual.book)

    def test_markers_are_processed_in_declared_order(self):
        spec = make_spec(markers=(
            marker("sample", PAMarkerKind.SAMPLE, 1),
            marker("commit", PAMarkerKind.COMMIT, 2),
        ))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        first = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(2)
        )
        self.assertEqual(first.transaction.ready_marker, "sample")
        first = self.pair(
            sample_marker, r2_sample_marker, first.book, "tx-1", "sample", "0x12"
        )
        second = self.pair(
            reach_next_marker, r2_reach_next_marker, first.book, "tx-1", Fraction(2)
        )
        self.assertEqual(second.transaction.ready_marker, "commit")

    def test_completion_requires_all_markers_duration_and_tw_budgets(self):
        spec = make_spec(
            markers=(marker("write", PAMarkerKind.COMMIT, 1),),
            duration=2, automatic=1, external=1,
        )
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        blocked = self.pair(complete, r2_complete, actual.book, "tx-1")
        self.assertEqual(blocked.code, PATxCode.BLOCKED)
        actual = self.pair(
            reach_next_marker, r2_reach_next_marker, actual.book, "tx-1", Fraction(1)
        )
        actual = self.pair(
            commit_effect, r2_commit_effect, actual.book, "tx-1", "write"
        )
        actual = self.pair(
            consume_base_time, r2_consume_base_time, actual.book, "tx-1", Fraction(2)
        )
        actual = self.pair(
            consume_tw, r2_consume_tw, actual.book, "tx-1", automatic=True
        )
        blocked = self.pair(complete, r2_complete, actual.book, "tx-1")
        self.assertEqual(blocked.code, PATxCode.BLOCKED)
        actual = self.pair(
            consume_tw, r2_consume_tw, actual.book, "tx-1", automatic=False
        )
        completed = self.pair(complete, r2_complete, actual.book, "tx-1")
        self.assertEqual(completed.transaction.state, PATxState.COMPLETED)

    def test_terminal_cancelled_transaction_is_not_reopened_by_retry(self):
        spec = make_spec()
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(cancel, r2_cancel, actual.book, "tx-1")
        retry = submit(actual.book, spec)
        retry_ref = r2_submit(to_r2_book(actual.book), to_r2_spec(spec))
        self.compare(retry, retry_ref)
        self.assertEqual(retry.transaction.state, PATxState.CANCELLED)

    def test_recovery_is_distinct_from_original_retry(self):
        spec = make_spec()
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(fail, r2_fail, actual.book, "tx-1", "fault")
        retry = submit(actual.book, spec)
        retry_ref = r2_submit(to_r2_book(actual.book), to_r2_spec(spec))
        self.compare(retry, retry_ref)
        self.assertEqual(retry.transaction.state, PATxState.FAILED)
        recovered = self.pair(recover, r2_recover, retry.book, "tx-1")
        self.assertEqual(recovered.transaction.recoveries, 1)

    def test_out_of_order_marker_descriptor_is_invalid_before_admission(self):
        spec = make_spec(markers=(
            marker("later", PAMarkerKind.COMMIT, 3),
            marker("earlier", PAMarkerKind.SAMPLE, 2),
        ))
        actual = submit(PATransactionBook(), spec)
        expected = r2_submit(R2Book(), to_r2_spec(spec))
        self.compare(actual, expected)
        self.assertEqual(actual.code, PATxCode.INVALID_INPUT)
        self.assertEqual(actual.book, PATransactionBook())

    def test_identical_request_retry_preserves_every_control_state(self):
        spec = make_spec(markers=(marker("read", PAMarkerKind.SAMPLE, 1),))
        actual, _ = self.admitted_pair(spec)
        actual = self.pair(start, r2_start, actual.book, "tx-1")
        actual = self.pair(
            enter_waiting, r2_enter_waiting, actual.book, "tx-1", "fixture"
        )
        retry = submit(actual.book, spec)
        retry_ref = r2_submit(to_r2_book(actual.book), to_r2_spec(spec))
        self.compare(retry, retry_ref)
        self.assertEqual(retry.book, actual.book)
        self.assertEqual(retry.transaction.state, PATxState.WAITING)
        self.assertEqual(retry.transaction.wait_source, "fixture")


if __name__ == "__main__":
    unittest.main(verbosity=2)
