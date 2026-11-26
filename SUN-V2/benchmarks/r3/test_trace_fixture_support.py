"""Canonical trace encoding contract tests."""
from __future__ import annotations

import unittest
from fractions import Fraction

from trace_fixture_support import canonical_bytes, normalize


class CanonicalTraceNormalizationTests(unittest.TestCase):
    """Verify exact-time encoding and reject lossy numeric representations."""

    def test_fraction_is_encoded_as_exact_integer_pair(self):
        self.assertEqual(
            normalize(Fraction(3, 7)),
            {"$fraction": [3, 7]},
        )

    def test_integer_and_boolean_primitives_remain_supported(self):
        self.assertEqual(
            normalize({"count": 4, "enabled": True, "label": "fixture"}),
            {"count": 4, "enabled": True, "label": "fixture"},
        )

    def test_float_values_are_rejected(self):
        for value in (0.0, 1.25, float("nan"), float("inf"), float("-inf")):
            with self.subTest(value=repr(value)):
                with self.assertRaisesRegex(TypeError, "floating-point"):
                    normalize(value)

    def test_nested_float_values_are_rejected_before_serialization(self):
        with self.assertRaisesRegex(TypeError, "floating-point"):
            canonical_bytes({"state": {"elapsed": 0.5}})


if __name__ == "__main__":
    unittest.main(verbosity=2)
