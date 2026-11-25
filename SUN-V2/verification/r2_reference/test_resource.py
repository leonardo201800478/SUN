"""R2-T15 finite-capacity, partial-effect and atomicity regressions."""
import unittest

from resource_reference import (
    ResourceCode, ResourceLedger, commit_effect, fail, recover, reserve,
)


class R2T15ResourceTests(unittest.TestCase):
    def test_reservation_exactly_at_capacity_is_admitted(self):
        result = reserve(ResourceLedger(4), "r1", 4, ("a", "b"))
        self.assertEqual(result.code, ResourceCode.OK)
        self.assertEqual(result.ledger.used, 4)

    def test_capacity_exhaustion_is_atomic(self):
        first = reserve(ResourceLedger(4), "r1", 3, ("a",)).ledger
        second = reserve(first, "r2", 2, ("b",))
        self.assertEqual(second.code, ResourceCode.RESOURCE_EXHAUSTED)
        self.assertEqual(second.ledger, first)

    def test_invalid_descriptor_precedes_capacity_evaluation(self):
        ledger = ResourceLedger(0)
        result = reserve(ledger, "", 99, ("a",))
        self.assertEqual(result.code, ResourceCode.INVALID_INPUT)
        self.assertEqual(result.ledger, ledger)

    def test_identical_reservation_retry_is_idempotent(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a",)).ledger
        retry = reserve(first, "r1", 2, ("a",))
        self.assertEqual(retry.code, ResourceCode.OK)
        self.assertEqual(retry.ledger, first)

    def test_divergent_reservation_retry_is_invalid_without_mutation(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a",)).ledger
        retry = reserve(first, "r1", 2, ("b",))
        self.assertEqual(retry.code, ResourceCode.INVALID_INPUT)
        self.assertEqual(retry.ledger, first)

    def test_effect_commit_is_exactly_once(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a",)).ledger
        committed = commit_effect(first, "a")
        retry = commit_effect(committed.ledger, "a")
        self.assertEqual(retry.ledger, committed.ledger)
        self.assertEqual(retry.ledger.committed_effects, ("a",))

    def test_failure_before_first_effect_preserves_all_pending(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a", "b")).ledger
        failed = fail(first, "fault")
        self.assertEqual(failed.ledger.pending_effects, ("a", "b"))
        self.assertEqual(failed.ledger.committed_effects, ())

    def test_failure_after_partial_effect_preserves_exact_prefix_and_pending_suffix(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a", "b", "c")).ledger
        one = commit_effect(first, "a").ledger
        two = commit_effect(one, "b").ledger
        failed = fail(two, "fault")
        self.assertEqual(failed.ledger.committed_effects, ("a", "b"))
        self.assertEqual(failed.ledger.pending_effects, ("c",))

    def test_recovery_commits_only_pending_effects_and_is_terminal(self):
        first = reserve(ResourceLedger(4), "r1", 2, ("a", "b")).ledger
        one = commit_effect(first, "a").ledger
        failed = fail(one, "fault").ledger
        recovered = recover(failed)
        self.assertEqual(recovered.ledger.committed_effects, ("a", "b"))
        self.assertEqual(recovered.ledger.pending_effects, ())
        self.assertTrue(recovered.ledger.completed)
        self.assertEqual(recover(recovered.ledger).code, ResourceCode.INVARIANT_VIOLATION)

    def test_invalid_ledger_total_is_rejected_before_any_effect(self):
        invalid = ResourceLedger(4, used=3, reservations=(("r", 2, ("a",)),), pending_effects=("a",))
        result = commit_effect(invalid, "a")
        self.assertEqual(result.code, ResourceCode.INVALID_INPUT)
        self.assertEqual(result.ledger, invalid)


if __name__ == "__main__":
    unittest.main(verbosity=2)
