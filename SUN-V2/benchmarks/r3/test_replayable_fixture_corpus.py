"""Tests for the experimental replayable differential fixture harness."""
from __future__ import annotations

import unittest
from fractions import Fraction

from replayable_fixture_corpus import (
    SCHEMA,
    capture_differential_fixture,
    replay_fixture_side,
)


def step(state, operation, args):
    """Apply one declarative integer-counter operation."""
    updated = dict(state)
    if operation == "add":
        updated["value"] += args["amount"]
    elif operation == "multiply":
        updated["value"] *= args["factor"]
    else:
        raise ValueError(f"unknown operation: {operation}")
    return updated


def divergent_step(state, operation, args):
    """Independent comparison side with a deliberate second-operation defect."""
    updated = step(state, operation, args)
    if operation == "multiply":
        updated["value"] += 1
    return updated


class ReplayableFixtureCorpusTests(unittest.TestCase):
    """Exercise complete inputs, ordered operations, state cuts and divergence."""

    def fixture(self, right_step=step):
        return capture_differential_fixture(
            fixture_id="synthetic-counter-w07-shape",
            provenance={
                "classification": "OBSERVATION",
                "source": "synthetic harness test; not SMS hardware evidence",
                "revision": "fixture-v1",
            },
            initial_state={"value": 2},
            operations=(
                {"sequence": 0, "name": "add", "args": {"amount": 3}},
                {"sequence": 1, "name": "multiply", "args": {"factor": 4}},
            ),
            left_step=step,
            right_step=right_step,
        )

    def test_equal_fixture_contains_replay_inputs_and_full_state_cuts(self):
        fixture = self.fixture()
        self.assertEqual(fixture["schema"], SCHEMA)
        self.assertEqual(fixture["initial_state"], {"value": 2})
        self.assertEqual(
            [item["name"] for item in fixture["operations"]],
            ["add", "multiply"],
        )
        self.assertEqual(
            [item["left_state"] for item in fixture["cuts"]],
            [{"value": 5}, {"value": 20}],
        )
        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(
            replay_fixture_side(fixture, step=step, side="left"),
            ({"value": 5}, {"value": 20}),
        )
        self.assertEqual(
            replay_fixture_side(fixture, step=step, side="right"),
            ({"value": 5}, {"value": 20}),
        )

    def test_intentional_divergence_is_preserved_without_aborting_capture(self):
        fixture = self.fixture(divergent_step)
        self.assertFalse(fixture["all_equal"])
        self.assertEqual(fixture["first_divergence"]["sequence"], 1)
        self.assertEqual(fixture["first_divergence"]["operation"], "multiply")
        self.assertEqual(fixture["first_divergence"]["left_state"], {"value": 20})
        self.assertEqual(fixture["first_divergence"]["right_state"], {"value": 21})
        self.assertEqual(len(fixture["cuts"]), 2)
        self.assertTrue(fixture["cuts"][0]["equal"])
        self.assertFalse(fixture["cuts"][1]["equal"])
        self.assertEqual(
            replay_fixture_side(fixture, step=step, side="left"),
            ({"value": 5}, {"value": 20}),
        )
        self.assertEqual(
            replay_fixture_side(fixture, step=divergent_step, side="right"),
            ({"value": 5}, {"value": 21}),
        )

    def test_fraction_inputs_are_preserved_exactly(self):
        fixture = capture_differential_fixture(
            fixture_id="fraction-input",
            provenance={"classification": "OBSERVATION"},
            initial_state={"ticks": 0},
            operations=(
                {"sequence": 0, "name": "add", "args": {"amount": Fraction(1, 3)}},
            ),
            left_step=lambda state, _name, args: {
                "ticks": Fraction(state["ticks"]) + Fraction(*args["amount"]["$fraction"])
            },
            right_step=lambda state, _name, args: {
                "ticks": Fraction(state["ticks"]) + Fraction(*args["amount"]["$fraction"])
            },
        )
        self.assertEqual(
            fixture["operations"][0]["args"]["amount"],
            {"$fraction": [1, 3]},
        )

    def test_non_contiguous_operation_sequence_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "contiguous"):
            capture_differential_fixture(
                fixture_id="invalid-sequence",
                provenance={},
                initial_state={"value": 0},
                operations=(
                    {"sequence": 1, "name": "add", "args": {"amount": 1}},
                ),
                left_step=step,
                right_step=step,
            )

    def test_mutating_left_side_cannot_change_right_side_initial_state(self):
        def mutating(state, _name, _args):
            state["value"] += 1
            return state

        fixture = capture_differential_fixture(
            fixture_id="mutation-isolation",
            provenance={},
            initial_state={"value": 0},
            operations=(
                {"sequence": 0, "name": "mutate", "args": {}},
            ),
            left_step=mutating,
            right_step=mutating,
        )
        self.assertEqual(fixture["cuts"][0]["left_state"], {"value": 1})
        self.assertEqual(fixture["cuts"][0]["right_state"], {"value": 1})


if __name__ == "__main__":
    unittest.main(verbosity=2)
