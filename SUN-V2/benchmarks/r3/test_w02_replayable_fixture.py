"""Replayable W02 fixture for dense and simultaneous deadlines.

The fixture captures one commutative simultaneous group and one UNKNOWN group.
The UNKNOWN close must preserve the deadline book and watermark without
inventing an insertion-order tie-break. This is synthetic P-A/R2 evidence only.
"""
from __future__ import annotations

import unittest
from fractions import Fraction

from p_a_deadline_lifecycle import (
    ADeadlineCode,
    ADeadlineStatus,
    AParticipant,
    active_deadlines,
)
from replayable_fixture_corpus import (
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)
from test_w01_replayable_fixture import (
    _p_a_step,
    _r2_step,
    _state_data,
)
from p_a_deadline_lifecycle import ADeadlineBook, ADeadlineContext


class W02ReplayableSimultaneousDeadlineTests(unittest.TestCase):
    def test_w02_commutative_claims_and_unknown_group_are_captured_and_replayable(self):
        names = ("device-00", "device-01", "device-02", "device-03")
        context = ADeadlineContext(
            participants=tuple(
                AParticipant(name, Fraction(0), Fraction(100))
                for name in names
            ),
            expected_participants=frozenset(names),
        )
        initial_book = ADeadlineBook()
        initial = _state_data(context, initial_book)

        declarations = (
            # Reverse identity order deliberately: commutativity, not
            # insertion order, must determine the stable claimed identities.
            ("device-01", "event-01", 10, True),
            ("device-00", "event-00", 10, True),
            # A second simultaneous group has no declared causal or
            # commutative relation. It must remain UNKNOWN at close.
            ("device-03", "event-03", 20, False),
            ("device-02", "event-02", 20, False),
        )
        operations = []
        for owner, identity, instant, commutative in declarations:
            operations.append({
                "sequence": len(operations),
                "name": "publish",
                "args": {
                    "owner": owner,
                    "identity": identity,
                    "generation": 0,
                    "at_common": Fraction(instant),
                    "effect_id": f"effect-{identity}",
                    "commutative": commutative,
                },
            })
        operations.extend((
            {
                "sequence": len(operations),
                "name": "close_watermark",
                "args": {"target_common": Fraction(10)},
            },
            {
                "sequence": len(operations) + 1,
                "name": "close_watermark",
                "args": {"target_common": Fraction(20)},
            },
        ))

        fixture = capture_differential_fixture(
            fixture_id="W02_DENSE_SIMULTANEOUS_COMMUTATIVE_AND_UNKNOWN_GROUPS",
            provenance={
                "gate": "R3",
                "workload": "W02",
                "classification": "OBSERVATION",
                "scenario": (
                    "A two-member commutative group at common instant 10 is "
                    "claimed with identity-stable ordering despite reverse "
                    "publication order; a two-member UNKNOWN group at 20 "
                    "remains unclaimed and does not advance the watermark."
                ),
                "scope": "synthetic P-A/R2 simultaneous-deadline model comparison",
                "limitations": (
                    "No physical SMS event ordering, device phase, scheduler "
                    "behavior or hardware commutativity is established."
                ),
                "contract": "R1 v0.3.18; simultaneous deadline groups",
            },
            initial_state=initial,
            operations=tuple(operations),
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), 6)

        commutative_cut = fixture["cuts"][4]["left_state"]
        self.assertEqual(commutative_cut["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(
            commutative_cut["claimed"],
            [["event-00", 0], ["event-01", 0]],
        )
        self.assertEqual(
            tuple(item["status"] for item in commutative_cut["state"]["deadlines"][:2]),
            (ADeadlineStatus.CLAIMED.value, ADeadlineStatus.CLAIMED.value),
        )
        self.assertEqual(
            commutative_cut["state"]["watermark_common"],
            {"$fraction": [10, 1]},
        )

        unknown_cut = fixture["cuts"][5]["left_state"]
        self.assertEqual(unknown_cut["code"], ADeadlineCode.INDETERMINATE.value)
        self.assertEqual(unknown_cut["claimed"], [])
        self.assertEqual(
            unknown_cut["state"]["watermark_common"],
            {"$fraction": [10, 1]},
        )
        self.assertEqual(
            tuple(item["status"] for item in unknown_cut["state"]["deadlines"][2:]),
            (ADeadlineStatus.SCHEDULED.value, ADeadlineStatus.SCHEDULED.value),
        )
        self.assertEqual(active_deadlines(_p_a_models_for_final(unknown_cut)), (
            # Both UNKNOWN deadlines remain active and unclaimed.
            ("device-02", "event-02", 0, Fraction(20)),
            ("device-03", "event-03", 0, Fraction(20)),
        ))

        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


def _p_a_models_for_final(result):
    """Rebuild the P-A deadline book for the final captured cut."""
    from test_w01_replayable_fixture import _p_a_models

    return _p_a_models(result)[1]


if __name__ == "__main__":
    unittest.main(verbosity=2)
