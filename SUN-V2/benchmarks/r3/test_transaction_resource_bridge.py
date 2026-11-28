"""W08 end-to-end candidate composition tests against the independent R2 resource oracle."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "verification" / "r2_reference"))

from resource_reference import (  # noqa: E402
    ResourceLedger as R2Ledger,
    commit_effect as r2_commit,
    fail as r2_fail,
    reserve as r2_reserve,
)
from r3_resource_ledger import ResourceLedger
from r3_transaction_core import (
    MarkerKind, TransactionBook, TxMarker, TxSpec, TxState,
    recover as recover_tx,
)
from transaction_resource_bridge import (
    BridgeCode, TransactionResourceState, audit_recovery_boundary,
    commit_marker_with_resource, fail_transaction_with_resource,
    reach_transaction_marker, reserve_and_submit, start_transaction,
)


def spec():
    return TxSpec(
        identity="tx-bridge",
        generation=0,
        origin="cpu",
        destination="vdp",
        operation="write",
        profile="SMS-NTSC",
        payload="payload",
        admitted_at=Fraction(0),
        start_at=Fraction(0),
        duration_base=Fraction(0),
        automatic_tws=0,
        external_tws=0,
        markers=(
            TxMarker("commit-1", MarkerKind.COMMIT, Fraction(1), True),
            TxMarker("commit-2", MarkerKind.COMMIT, Fraction(2), True),
        ),
    )


def initial(capacity=10):
    return TransactionResourceState(
        book=TransactionBook(),
        resources=ResourceLedger(capacity),
    )


def admit(state=None, amount=7):
    return reserve_and_submit(
        state or initial(),
        spec(),
        amount=amount,
        marker_effects=(("commit-1", "effect-1"), ("commit-2", "effect-2")),
    )


def assert_resource_matches(test, candidate, reference):
    test.assertEqual(candidate.capacity, reference.capacity)
    test.assertEqual(candidate.used, reference.used)
    test.assertEqual(candidate.reservations, reference.reservations)
    test.assertEqual(candidate.pending_effects, reference.pending_effects)
    test.assertEqual(candidate.committed_effects, reference.committed_effects)
    test.assertEqual(candidate.failed, reference.failed)
    test.assertEqual(candidate.completed, reference.completed)
    test.assertEqual(candidate.error, reference.error)


class TransactionResourceBridgeTests(unittest.TestCase):
    def test_reservation_and_transaction_admission_are_atomic_at_capacity(self):
        candidate = admit(initial(capacity=5), amount=7)
        reference = r2_reserve(R2Ledger(5), "tx-bridge", 7, ("effect-1", "effect-2"))
        self.assertEqual(candidate.code.value, reference.code.value)
        self.assertEqual(candidate.code, BridgeCode.RESOURCE_EXHAUSTED)
        self.assertEqual(candidate.state.book, TransactionBook())
        self.assertEqual(candidate.state.resources, ResourceLedger(5))
        assert_resource_matches(self, candidate.state.resources, reference.ledger)

        accepted = admit(initial(capacity=7), amount=7)
        accepted_reference = r2_reserve(
            R2Ledger(7), "tx-bridge", 7, ("effect-1", "effect-2")
        )
        self.assertEqual(accepted.code, BridgeCode.OK)
        assert_resource_matches(self, accepted.state.resources, accepted_reference.ledger)
        self.assertEqual(len(accepted.state.book.transactions), 1)

    def test_identical_retry_is_idempotent_and_divergent_budget_is_atomic(self):
        first = admit(initial(capacity=10), amount=7)
        self.assertEqual(first.code, BridgeCode.OK)
        retry = admit(first.state, amount=7)
        self.assertEqual(retry.code, BridgeCode.OK)
        self.assertEqual(retry.state, first.state)

        divergent = admit(first.state, amount=6)
        self.assertEqual(divergent.code, BridgeCode.INVALID_INPUT)
        self.assertEqual(divergent.state, first.state)

    def test_commit_marker_and_resource_effect_commit_exactly_once(self):
        first = admit(initial(capacity=10), amount=7)
        reference = r2_reserve(
            R2Ledger(10), "tx-bridge", 7, ("effect-1", "effect-2")
        )
        started = start_transaction(first.state)
        self.assertEqual(started.code, BridgeCode.OK)
        reached = reach_transaction_marker(started.state, Fraction(1))
        self.assertEqual(reached.code, BridgeCode.OK)

        committed = commit_marker_with_resource(reached.state, "commit-1")
        reference = r2_commit(reference.ledger, "effect-1")
        self.assertEqual(committed.code, BridgeCode.OK)
        assert_resource_matches(self, committed.state.resources, reference.ledger)
        self.assertEqual(
            committed.state.book.transactions[0].committed, ("commit-1",)
        )
        retried = commit_marker_with_resource(committed.state, "commit-1")
        self.assertEqual(retried.code, BridgeCode.OK)
        self.assertEqual(retried.state, committed.state)

    def test_partial_failure_preserves_matching_committed_prefix_and_pending_suffix(self):
        first = admit(initial(capacity=10), amount=7)
        reference = r2_reserve(
            R2Ledger(10), "tx-bridge", 7, ("effect-1", "effect-2")
        )
        started = start_transaction(first.state)
        reached_first = reach_transaction_marker(started.state, Fraction(1))
        committed_first = commit_marker_with_resource(reached_first.state, "commit-1")
        reference = r2_commit(reference.ledger, "effect-1")
        reached_second = reach_transaction_marker(committed_first.state, Fraction(2))
        self.assertEqual(reached_second.code, BridgeCode.OK)

        failed = fail_transaction_with_resource(reached_second.state, "device fault")
        reference = r2_fail(reference.ledger, "device fault")
        self.assertEqual(failed.code, BridgeCode.OK)
        assert_resource_matches(self, failed.state.resources, reference.ledger)
        tx = failed.state.book.transactions[0]
        self.assertEqual(tx.state, TxState.FAILED)
        self.assertEqual(tx.committed, ("commit-1",))
        self.assertEqual(failed.state.resources.committed_effects, ("effect-1",))
        self.assertEqual(failed.state.resources.pending_effects, ("effect-2",))

    def test_transaction_and_resource_recovery_are_explicitly_incomparable(self):
        first = admit(initial(capacity=10), amount=7)
        started = start_transaction(first.state)
        reached_first = reach_transaction_marker(started.state, Fraction(1))
        committed_first = commit_marker_with_resource(reached_first.state, "commit-1")
        reached_second = reach_transaction_marker(committed_first.state, Fraction(2))
        failed = fail_transaction_with_resource(reached_second.state, "device fault")
        before = failed.state

        boundary = audit_recovery_boundary(before)
        self.assertEqual(boundary.code, BridgeCode.INCOMPARABLE_RECOVERY)
        self.assertEqual(boundary.state, before)
        self.assertEqual(len(boundary.recovery_audit.not_equivalent), 3)

        transaction_recovery = recover_tx(before.book, "tx-bridge")
        from r3_resource_ledger import recover as recover_resource
        resource_recovery = recover_resource(before.resources)
        self.assertEqual(transaction_recovery.code.value, "Ok")
        self.assertEqual(
            transaction_recovery.book.transactions[0].state, TxState.IN_PROGRESS
        )
        self.assertEqual(resource_recovery.code.value, "Ok")
        self.assertTrue(resource_recovery.ledger.completed)
        self.assertEqual(resource_recovery.ledger.pending_effects, ())
        self.assertEqual(
            resource_recovery.ledger.committed_effects,
            ("effect-1", "effect-2"),
        )
        self.assertNotEqual(
            transaction_recovery.book.transactions[0].committed,
            ("commit-1", "commit-2"),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
