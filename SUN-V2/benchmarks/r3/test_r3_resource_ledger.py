"""Differential W08 resource-ledger tests: independent R3 candidate vs R2 oracle."""
from __future__ import annotations

import unittest

from resource_reference import (
    ResourceLedger as R2Ledger,
    commit_effect as r2_commit,
    fail as r2_fail,
    recover as r2_recover,
    reserve as r2_reserve,
)
from r3_resource_ledger import (
    ResourceCode,
    ResourceLedger,
    commit_effect,
    fail,
    recover,
    reserve,
)


def pair(ledger, r2_ledger):
    """Assert exact result-code and immutable-ledger agreement."""
    return tuple(getattr(ledger, name) for name in ledger.__dataclass_fields__) == tuple(
        getattr(r2_ledger, name) for name in r2_ledger.__dataclass_fields__
    )


class R3ResourceLedgerDifferentialTests(unittest.TestCase):
    def assert_pair(self, candidate, reference):
        self.assertEqual(candidate.code.value, reference.code.value)
        self.assertTrue(pair(candidate.ledger, reference.ledger))

    def test_capacity_exact_boundary_and_exhaustion_are_atomic(self):
        candidate = ResourceLedger(4)
        reference = R2Ledger(4)
        c1 = reserve(candidate, "r1", 3, ("a",))
        r1 = r2_reserve(reference, "r1", 3, ("a",))
        self.assert_pair(c1, r1)
        c2 = reserve(c1.ledger, "r2", 2, ("b",))
        r2 = r2_reserve(r1.ledger, "r2", 2, ("b",))
        self.assert_pair(c2, r2)
        self.assertEqual(c2.code, ResourceCode.RESOURCE_EXHAUSTED)
        self.assertEqual(c2.ledger, c1.ledger)

        exact_c = reserve(ResourceLedger(4), "exact", 4, ("x", "y"))
        exact_r = r2_reserve(R2Ledger(4), "exact", 4, ("x", "y"))
        self.assert_pair(exact_c, exact_r)
        self.assertEqual(exact_c.ledger.used, 4)

    def test_invalid_descriptors_and_divergent_retry_do_not_mutate(self):
        c = reserve(ResourceLedger(4), "r1", 2, ("a",))
        r = r2_reserve(R2Ledger(4), "r1", 2, ("a",))
        self.assert_pair(c, r)

        cases = (
            ("", 1, ("b",)),
            ("r2", 0, ("b",)),
            ("r2", 1, ()),
            ("r2", 1, ("b", "b")),
        )
        for identity, amount, effects in cases:
            with self.subTest(identity=identity, amount=amount, effects=effects):
                candidate = reserve(c.ledger, identity, amount, effects)
                reference = r2_reserve(r.ledger, identity, amount, effects)
                self.assert_pair(candidate, reference)
                self.assertEqual(candidate.code, ResourceCode.INVALID_INPUT)
                self.assertEqual(candidate.ledger, c.ledger)

        candidate_retry = reserve(c.ledger, "r1", 2, ("b",))
        reference_retry = r2_reserve(r.ledger, "r1", 2, ("b",))
        self.assert_pair(candidate_retry, reference_retry)
        self.assertEqual(candidate_retry.code, ResourceCode.INVALID_INPUT)
        self.assertEqual(candidate_retry.ledger, c.ledger)

    def test_identical_retry_and_effect_commit_are_idempotent(self):
        first_c = reserve(ResourceLedger(4), "r1", 2, ("a", "b"))
        first_r = r2_reserve(R2Ledger(4), "r1", 2, ("a", "b"))
        self.assert_pair(first_c, first_r)
        retry_c = reserve(first_c.ledger, "r1", 2, ("a", "b"))
        retry_r = r2_reserve(first_r.ledger, "r1", 2, ("a", "b"))
        self.assert_pair(retry_c, retry_r)

        committed_c = commit_effect(retry_c.ledger, "a")
        committed_r = r2_commit(retry_r.ledger, "a")
        self.assert_pair(committed_c, committed_r)
        duplicate_c = commit_effect(committed_c.ledger, "a")
        duplicate_r = r2_commit(committed_r.ledger, "a")
        self.assert_pair(duplicate_c, duplicate_r)

    def test_partial_failure_and_recovery_preserve_committed_prefix(self):
        first_c = reserve(ResourceLedger(4), "r1", 2, ("a", "b", "c"))
        first_r = r2_reserve(R2Ledger(4), "r1", 2, ("a", "b", "c"))
        self.assert_pair(first_c, first_r)
        one_c = commit_effect(first_c.ledger, "a")
        one_r = r2_commit(first_r.ledger, "a")
        self.assert_pair(one_c, one_r)
        failed_c = fail(one_c.ledger, "fault")
        failed_r = r2_fail(one_r.ledger, "fault")
        self.assert_pair(failed_c, failed_r)
        self.assertEqual(failed_c.ledger.committed_effects, ("a",))
        self.assertEqual(failed_c.ledger.pending_effects, ("b", "c"))

        recovered_c = recover(failed_c.ledger)
        recovered_r = r2_recover(failed_r.ledger)
        self.assert_pair(recovered_c, recovered_r)
        self.assertTrue(recovered_c.ledger.completed)
        self.assertEqual(recovered_c.ledger.pending_effects, ())
        self.assertEqual(recovered_c.ledger.committed_effects, ("a", "b", "c"))

    def test_invalid_ledger_is_rejected_without_mutation(self):
        candidate = ResourceLedger(
            4, used=3, reservations=(("r", 2, ("a",)),),
            pending_effects=("a",),
        )
        reference = R2Ledger(
            4, used=3, reservations=(("r", 2, ("a",)),),
            pending_effects=("a",),
        )
        c = commit_effect(candidate, "a")
        r = r2_commit(reference, "a")
        self.assert_pair(c, r)
        self.assertEqual(c.code, ResourceCode.INVALID_INPUT)
        self.assertEqual(c.ledger, candidate)


if __name__ == "__main__":
    unittest.main(verbosity=2)
