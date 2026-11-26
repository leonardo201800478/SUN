"""Discriminating W07 matrix for explicit pairwise causal/commutative relations."""
from __future__ import annotations

import unittest

from w07_pairwise_matrix import PairDecision, PairRelation, adjudicate_pair


class W07PairwiseRelationMatrixTests(unittest.TestCase):
    """Verify all four R1 §16.6 relation classes without changing P-A/R2."""

    def decide(self, relation, requested):
        return adjudicate_pair(
            first_id="b",
            second_id="c",
            relation=relation,
            requested_id=requested,
        )

    def test_b_causes_c_requires_b_before_c(self):
        self.assertEqual(
            self.decide(PairRelation.FIRST_CAUSES_SECOND, "b"),
            PairDecision.ADVANCE,
        )
        self.assertEqual(
            self.decide(PairRelation.FIRST_CAUSES_SECOND, "c"),
            PairDecision.BLOCK,
        )

    def test_c_causes_b_requires_c_before_b(self):
        self.assertEqual(
            self.decide(PairRelation.SECOND_CAUSES_FIRST, "c"),
            PairDecision.ADVANCE,
        )
        self.assertEqual(
            self.decide(PairRelation.SECOND_CAUSES_FIRST, "b"),
            PairDecision.BLOCK,
        )

    def test_proven_commutative_pair_allows_either_order(self):
        self.assertEqual(
            self.decide(PairRelation.COMMUTATIVE, "b"),
            PairDecision.ADVANCE,
        )
        self.assertEqual(
            self.decide(PairRelation.COMMUTATIVE, "c"),
            PairDecision.ADVANCE,
        )

    def test_unknown_pair_is_indeterminate_for_either_order(self):
        self.assertEqual(
            self.decide(PairRelation.UNKNOWN, "b"),
            PairDecision.INDETERMINATE,
        )
        self.assertEqual(
            self.decide(PairRelation.UNKNOWN, "c"),
            PairDecision.INDETERMINATE,
        )

    def test_unary_commutative_flags_do_not_enter_pair_adjudication(self):
        # The API accepts only one explicit pair relation, not two per-item
        # booleans. A and B being individually marked commutative cannot
        # silently establish commutativity(B, C).
        with self.assertRaises(ValueError):
            adjudicate_pair(
                first_id="b",
                second_id="c",
                relation="both_items_commutative",
                requested_id="c",
            )

    def test_invalid_or_ambiguous_pair_descriptors_are_rejected(self):
        invalid = (
            {"first_id": "", "second_id": "c", "requested_id": "c"},
            {"first_id": "b", "second_id": "b", "requested_id": "b"},
            {"first_id": "b", "second_id": "c", "requested_id": "x"},
        )
        for item in invalid:
            with self.subTest(item=item), self.assertRaises(ValueError):
                adjudicate_pair(
                    first_id=item["first_id"],
                    second_id=item["second_id"],
                    relation=PairRelation.UNKNOWN,
                    requested_id=item["requested_id"],
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
