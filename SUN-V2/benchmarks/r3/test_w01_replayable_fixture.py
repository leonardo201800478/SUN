"""Replayable W01 fixture for sparse autonomous deadline lifecycle.

The fixture captures a sparse sequence of independently owned deadlines,
watermark claims, exactly-once dispatch and completion. It compares P-A with
the independent R2 oracle using exact rational coordinates. This is synthetic
model evidence, not a physical SMS deadline trace.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_deadline_lifecycle import (  # noqa: E402
    ADeadline,
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    ADeadlineStatus,
    AParticipant,
    active_deadlines,
    close_watermark,
    complete_deadline,
    dispatch_deadline,
    publish_deadline,
)
from deadline_reference import (  # noqa: E402
    Deadline as R2Deadline,
    DeadlineBook as R2DeadlineBook,
    DeadlineStatus as R2DeadlineStatus,
    active_deadlines as r2_active_deadlines,
    close_watermark as r2_close_watermark,
    complete_deadline as r2_complete_deadline,
    dispatch_deadline as r2_dispatch_deadline,
    publish_deadline as r2_publish_deadline,
)
from chronos_reference import ClockDomain, make_state as r2_make_state  # noqa: E402
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)


def _fraction(value):
    """Restore exact rational values encoded by the replay harness."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _state_data(context, book):
    """Serialize all participants, deadline generations and global book state."""
    return {
        "participants": [
            {
                "name": item.name,
                "progress": item.progress,
                "safe_horizon": item.safe_horizon,
                "units_per_common": item.units_per_common,
                "common_offset": item.common_offset,
            }
            for item in context.participants
        ],
        "expected_participants": sorted(context.expected_participants),
        "watermark_common": book.watermark_common,
        "deadlines": [
            {
                "owner": item.owner,
                "identity": item.identity,
                "generation": item.generation,
                "at_common": item.at_common,
                "effect_id": item.effect_id,
                "commutative": item.commutative,
                "phase_order": item.phase_order,
                "phase_evidence": item.phase_evidence,
                "microstep": item.microstep,
                "status": item.status.value,
            }
            for item in book.deadlines
        ],
        "pending_cancellations": [],
        "max_microsteps": book.max_microsteps,
    }


def _unwrap(data):
    """Extract the full state after an operation result envelope."""
    return data["state"] if "state" in data else data


def _p_a_models(data):
    """Reconstruct only P-A-owned context and deadline-book types."""
    data = _unwrap(data)
    participants = tuple(
        AParticipant(
            item["name"],
            _fraction(item["progress"]),
            _fraction(item["safe_horizon"]),
            _fraction(item["units_per_common"]),
            _fraction(item["common_offset"]),
        )
        for item in data["participants"]
    )
    context = ADeadlineContext(
        participants=participants,
        expected_participants=frozenset(data["expected_participants"]),
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
    )
    deadlines = tuple(
        ADeadline(
            owner=item["owner"],
            identity=item["identity"],
            generation=item["generation"],
            at_common=_fraction(item["at_common"]),
            effect_id=item["effect_id"],
            commutative=item["commutative"],
            phase_order=item["phase_order"],
            phase_evidence=item["phase_evidence"],
            microstep=item["microstep"],
            status= ADeadlineStatus(item["status"]),
        )
        for item in data["deadlines"]
    )
    book = ADeadlineBook(
        deadlines=deadlines,
        pending_cancellations=tuple(),
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _r2_models(data):
    """Reconstruct the independent R2 state without sharing P-A model types."""
    data = _unwrap(data)
    domains = tuple(
        ClockDomain(
            domain_id=item["name"],
            local_to_common=_fraction(item["units_per_common"]),
            common_offset=_fraction(item["common_offset"]),
            progress_local=(
                _fraction(item["progress"]) - _fraction(item["common_offset"])
            ) / _fraction(item["units_per_common"]),
            horizon_local=(
                _fraction(item["safe_horizon"]) - _fraction(item["common_offset"])
            ) / _fraction(item["units_per_common"]),
        )
        for item in data["participants"]
    )
    context = r2_make_state(
        profile="R3-W01-SPARSE-AUTONOMOUS-DEADLINES",
        domains=domains,
        expected_participants=data["expected_participants"],
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
        obligations=tuple(),
    )
    deadlines = tuple(
        R2Deadline(
            owner=item["owner"],
            identity=item["identity"],
            generation=item["generation"],
            at_common=_fraction(item["at_common"]),
            effect_id=item["effect_id"],
            commutative=item["commutative"],
            phase_order=item["phase_order"],
            phase_evidence=item["phase_evidence"],
            microstep=item["microstep"],
            status=R2DeadlineStatus(item["status"]),
        )
        for item in data["deadlines"]
    )
    book = R2DeadlineBook(
        deadlines=deadlines,
        pending_cancellations=tuple(),
        watermark_common=(
            None if data["watermark_common"] is None
            else _fraction(data["watermark_common"])
        ),
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _result_data(result, context):
    """Capture status, claims and the complete next state at each cut."""
    next_context = ADeadlineContext(
        participants=context.participants,
        expected_participants=context.expected_participants,
        watermark_common=result.book.watermark_common,
    )
    return {
        "code": result.code.value,
        "claimed": [list(item) for item in result.claimed],
        "detail": result.detail,
        "state": _state_data(next_context, result.book),
    }


def _p_a_step(data, name, args):
    """Execute one declared operation exclusively through P-A transitions."""
    context, book = _p_a_models(data)
    if name == "publish":
        result = publish_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
            effect_id=args["effect_id"],
        )
    elif name == "close_watermark":
        result = close_watermark(book, _fraction(args["target_common"]))
    elif name == "dispatch":
        result = dispatch_deadline(
            book, identity=args["identity"], generation=args["generation"]
        )
    elif name == "complete":
        result = complete_deadline(
            book, identity=args["identity"], generation=args["generation"]
        )
    else:
        raise ValueError(f"unsupported W01 operation: {name}")
    return _result_data(result, context)


def _r2_step(data, name, args):
    """Execute one declared operation exclusively through independent R2."""
    context, book = _r2_models(data)
    if name == "publish":
        result = r2_publish_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
            effect_id=args["effect_id"],
        )
    elif name == "close_watermark":
        result = r2_close_watermark(book, _fraction(args["target_common"]))
    elif name == "dispatch":
        result = r2_dispatch_deadline(
            book, identity=args["identity"], generation=args["generation"]
        )
    elif name == "complete":
        result = r2_complete_deadline(
            book, identity=args["identity"], generation=args["generation"]
        )
    else:
        raise ValueError(f"unsupported W01 operation: {name}")
    return {
        "code": result.code.value,
        "claimed": [list(item) for item in result.claimed],
        "detail": result.detail,
        "state": {
            "participants": [
                {
                    "name": item.domain_id,
                    "progress": item.progress_common(),
                    "safe_horizon": item.horizon_common(),
                    "units_per_common": item.local_to_common,
                    "common_offset": item.common_offset,
                }
                for item in context.domains
            ],
            "expected_participants": sorted(context.expected_participants),
            "watermark_common": result.book.watermark_common,
            "deadlines": [
                {
                    "owner": item.owner,
                    "identity": item.identity,
                    "generation": item.generation,
                    "at_common": item.at_common,
                    "effect_id": item.effect_id,
                    "commutative": item.commutative,
                    "phase_order": item.phase_order,
                    "phase_evidence": item.phase_evidence,
                    "microstep": item.microstep,
                    "status": item.status.value,
                }
                for item in result.book.deadlines
            ],
            "pending_cancellations": [],
            "max_microsteps": result.book.max_microsteps,
        },
    }


class W01ReplayableSparseDeadlineTests(unittest.TestCase):
    def test_w01_sparse_autonomous_deadlines_are_captured_and_replayable(self):
        initial_context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant(
                    "vdp", Fraction(0), Fraction(100), Fraction(3, 2)
                ),
            ),
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        initial_book = ADeadlineBook()
        initial = _state_data(initial_context, initial_book)

        operations = []
        for index, instant in enumerate((10, 30, 50, 80)):
            identity = f"sparse-deadline-{index:02d}"
            operations.extend((
                {
                    "sequence": len(operations),
                    "name": "publish",
                    "args": {
                        "owner": "cpu" if index % 2 == 0 else "vdp",
                        "identity": identity,
                        "generation": 0,
                        "at_common": Fraction(instant),
                        "effect_id": f"effect-{index:02d}",
                    },
                },
                {
                    "sequence": len(operations) + 1,
                    "name": "close_watermark",
                    "args": {"target_common": Fraction(instant)},
                },
                {
                    "sequence": len(operations) + 2,
                    "name": "dispatch",
                    "args": {"identity": identity, "generation": 0},
                },
                {
                    "sequence": len(operations) + 3,
                    "name": "complete",
                    "args": {"identity": identity, "generation": 0},
                },
            ))

        fixture = capture_differential_fixture(
            fixture_id="W01_SPARSE_AUTONOMOUS_DEADLINES_EXACTLY_ONCE",
            provenance={
                "gate": "R3",
                "workload": "W01",
                "classification": "OBSERVATION",
                "scenario": (
                    "Four sparse autonomous deadlines at common instants "
                    "10, 30, 50 and 80; each is published, claimed by a "
                    "closed watermark, dispatched once and completed."
                ),
                "scope": "synthetic P-A/R2 deadline-model comparison",
                "limitations": (
                    "No physical SMS timing, event density, device phase, "
                    "scheduler performance or hardware deadline ownership "
                    "is established by this abstract fixture."
                ),
                "contract": "R1 v0.3.18; sparse deadline lifecycle",
            },
            initial_state=initial,
            operations=tuple(operations),
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), 16)
        final = fixture["cuts"][-1]["left_state"]
        self.assertEqual(final["code"], "Accepted")
        self.assertEqual(_fraction(final["state"]["watermark_common"]), Fraction(80))
        self.assertEqual(
            tuple(item["status"] for item in final["state"]["deadlines"]),
            ("Completed", "Completed", "Completed", "Completed"),
        )
        self.assertEqual(active_deadlines(_p_a_models(final)[1]), ())
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
