"""R2-T08 state-machine and exactly-once transaction regressions."""
from fractions import Fraction
import unittest

from transaction_reference import (
    Marker, MarkerKind, TransactionBook, TransactionSpec, TxCode, TxState,
    cancel, commit_effect, complete, consume_base_time, consume_tw, enter_waiting,
    fail, reach_next_marker, recover, release_waiting, sample_marker, start, submit,
)


def make_spec(*, markers=(), duration=0, automatic=0, external=0, payload="data"):
    return TransactionSpec(
        "tx-1", 0, "cpu", "vdp", "write", "SMS-NTSC", payload,
        Fraction(0), Fraction(0), Fraction(duration), automatic, external, tuple(markers),
    )


def admitted(spec=None):
    return submit(TransactionBook(), spec or make_spec())


class R2T08TransactionTests(unittest.TestCase):
    def test_identical_retry_returns_existing_pending_state(self):
        descriptor = make_spec()
        first = start(admitted(descriptor).book, "tx-1")
        retry = submit(first.book, descriptor)
        self.assertEqual(retry.code, TxCode.OK)
        self.assertEqual(retry.transaction, first.transaction)
        self.assertEqual(retry.book, first.book)

    def test_duplicate_id_with_divergent_descriptor_is_invalid_without_mutation(self):
        first = admitted()
        retry = submit(first.book, make_spec(payload="different"))
        self.assertEqual(retry.code, TxCode.INVALID_INPUT)
        self.assertEqual(retry.book, first.book)

    def test_start_is_idempotent_only_while_in_progress(self):
        first = start(admitted().book, "tx-1")
        second = start(first.book, "tx-1")
        self.assertEqual(second.code, TxCode.OK)
        self.assertEqual(second.book, first.book)

    def test_waiting_and_resume_preserve_transaction_identity_and_progress(self):
        started = start(admitted().book, "tx-1")
        waiting = enter_waiting(started.book, "tx-1", "dependency")
        resumed = release_waiting(waiting.book, "tx-1", "dependency")
        self.assertEqual(waiting.transaction.state, TxState.WAITING)
        self.assertEqual(resumed.transaction.state, TxState.IN_PROGRESS)
        self.assertEqual(resumed.transaction.progress_common, started.transaction.progress_common)

    def test_sample_marker_is_captured_once_and_retry_returns_original_value(self):
        descriptor = make_spec(markers=(Marker("read", MarkerKind.SAMPLE, Fraction(2)),))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(2))
        sampled = sample_marker(ready.book, "tx-1", "read", "0x42")
        retry = sample_marker(sampled.book, "tx-1", "read", "0x99")
        self.assertEqual(retry.value, "0x42")
        self.assertEqual(retry.book, sampled.book)

    def test_commit_effect_is_applied_exactly_once(self):
        descriptor = make_spec(markers=(Marker("write", MarkerKind.COMMIT, Fraction(3)),))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(3))
        applied = commit_effect(ready.book, "tx-1", "write")
        retry = commit_effect(applied.book, "tx-1", "write")
        self.assertEqual(retry.book, applied.book)
        self.assertEqual(retry.transaction.committed, ("write",))

    def test_cancel_before_first_irreversible_effect_is_allowed(self):
        descriptor = make_spec(markers=(Marker("write", MarkerKind.COMMIT, Fraction(3)),))
        started = start(admitted(descriptor).book, "tx-1")
        result = cancel(started.book, "tx-1")
        self.assertEqual(result.transaction.state, TxState.CANCELLED)

    def test_cancel_after_commit_is_late_and_preserves_partial_ledger(self):
        descriptor = make_spec(markers=(Marker("write", MarkerKind.COMMIT, Fraction(3)),))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(3))
        applied = commit_effect(ready.book, "tx-1", "write")
        result = cancel(applied.book, "tx-1")
        self.assertEqual(result.code, TxCode.LATE_INPUT)
        self.assertEqual(result.book, applied.book)
        self.assertEqual(result.transaction.committed, ("write",))

    def test_irreversible_sample_closes_cancel_window_only_when_declared(self):
        descriptor = make_spec(markers=(Marker("ack", MarkerKind.SAMPLE, Fraction(2), True),))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(2))
        sampled = sample_marker(ready.book, "tx-1", "ack", "vector")
        result = cancel(sampled.book, "tx-1")
        self.assertEqual(result.code, TxCode.LATE_INPUT)
        self.assertEqual(result.book, sampled.book)

    def test_failure_before_effect_preserves_pending_and_requires_explicit_recovery(self):
        descriptor = make_spec(markers=(Marker("write", MarkerKind.COMMIT, Fraction(1)),))
        started = start(admitted(descriptor).book, "tx-1")
        failed = fail(started.book, "tx-1", "device fault")
        retry = submit(failed.book, descriptor)
        self.assertEqual(retry.transaction.state, TxState.FAILED)
        recovered = recover(failed.book, "tx-1")
        self.assertEqual(recovered.transaction.committed, ())
        self.assertEqual(recovered.transaction.recoveries, 1)

    def test_failure_after_partial_commit_preserves_exact_ledger(self):
        descriptor = make_spec(markers=(
            Marker("write-1", MarkerKind.COMMIT, Fraction(1)),
            Marker("write-2", MarkerKind.COMMIT, Fraction(2)),
        ))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(1))
        applied = commit_effect(ready.book, "tx-1", "write-1")
        failed = fail(applied.book, "tx-1", "device fault")
        recovered = recover(failed.book, "tx-1")
        self.assertEqual(failed.transaction.committed, ("write-1",))
        self.assertEqual(recovered.transaction.committed, ("write-1",))
        self.assertEqual(recovered.transaction.state, TxState.IN_PROGRESS)

    def test_commit_before_its_reached_frontier_is_invariant_violation(self):
        descriptor = make_spec(markers=(Marker("write", MarkerKind.COMMIT, Fraction(1)),))
        started = start(admitted(descriptor).book, "tx-1")
        result = commit_effect(started.book, "tx-1", "write")
        self.assertEqual(result.code, TxCode.INVARIANT_VIOLATION)
        self.assertEqual(result.book, started.book)

    def test_markers_are_processed_in_declared_order(self):
        descriptor = make_spec(markers=(
            Marker("sample", MarkerKind.SAMPLE, Fraction(1)),
            Marker("commit", MarkerKind.COMMIT, Fraction(2)),
        ))
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(2))
        self.assertEqual(ready.transaction.ready_marker, "sample")

    def test_completion_requires_all_markers_duration_and_tw_budgets(self):
        descriptor = make_spec(
            markers=(Marker("sample", MarkerKind.SAMPLE, Fraction(1)),),
            duration=2, automatic=1, external=1,
        )
        started = start(admitted(descriptor).book, "tx-1")
        ready = reach_next_marker(started.book, "tx-1", Fraction(1))
        sampled = sample_marker(ready.book, "tx-1", "sample", "value")
        self.assertEqual(complete(sampled.book, "tx-1").code, TxCode.BLOCKED)
        timed = consume_base_time(sampled.book, "tx-1", Fraction(2))
        automatic = consume_tw(timed.book, "tx-1", automatic=True)
        external = consume_tw(automatic.book, "tx-1", automatic=False)
        self.assertEqual(complete(external.book, "tx-1").transaction.state, TxState.COMPLETED)

    def test_terminal_cancelled_transaction_is_not_reopened_by_retry(self):
        cancelled = cancel(admitted().book, "tx-1")
        retry = submit(cancelled.book, cancelled.transaction.spec)
        self.assertEqual(retry.transaction.state, TxState.CANCELLED)
        self.assertEqual(retry.book, cancelled.book)

    def test_recovery_is_distinct_from_original_retry(self):
        started = start(admitted().book, "tx-1")
        failed = fail(started.book, "tx-1", "fault")
        self.assertEqual(recover(started.book, "tx-1").code, TxCode.INVARIANT_VIOLATION)
        self.assertEqual(recover(failed.book, "tx-1").transaction.state, TxState.IN_PROGRESS)


if __name__ == "__main__":
    unittest.main(verbosity=2)
