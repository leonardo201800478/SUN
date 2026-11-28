"""Adversarial JSON canonicality tests for candidate-local W09 replay."""
from __future__ import annotations

import unittest

from p_d_discrete_agenda import AgendaState
from r3_candidate_snapshots import (
    ReplayEvent,
    SnapshotCode,
    capture,
    make_replay_event,
    replay_from_snapshot,
)


class SnapshotJsonCanonicalityTests(unittest.TestCase):
    def test_replay_event_rejects_non_finite_json_numbers(self):
        for value in (float("nan"), float("inf"), float("-inf")):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    make_replay_event(1, "noop", {"value": value})

    def test_replay_rejects_non_standard_nan_and_infinity_tokens_atomically(self):
        state = AgendaState()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=0,
        ).artifact
        self.assertIsNotNone(artifact)

        for raw in (b'{"value":NaN}', b'{"value":Infinity}', b'{"value":-Infinity}'):
            with self.subTest(raw=raw):
                result = replay_from_snapshot(
                    artifact,
                    (ReplayEvent(1, "noop", raw),),
                    expected_candidate="P-D",
                    expected_profile="SMS-NTSC",
                    expected_version="P-D-R3-W09-1",
                    apply=lambda current, operation, arguments: current,
                )
                self.assertEqual(result.code, SnapshotCode.INVALID_INPUT)
                self.assertEqual(result.state, state)
                self.assertEqual(result.input_cursor, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
