"""Replayable W05 fixture for deadline cancellation race boundaries.

The fixture distinguishes C<D admission/arbitration, C=D UNKNOWN without an
explicit order, and C>D rejection, including watermark closure around the
admitted cancellation. Synthetic P-A/R2 evidence only.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_deadline_lifecycle import (  # noqa: E402
    ACancellation,
    ADeadline,
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    ADeadlineStatus,
    AParticipant,
    arbitrate_cancellations,
    cancel_deadline,
    close_watermark,
    publish_deadline,
)
from deadline_reference import (  # noqa: E402
    Cancellation as R2Cancellation,
    Deadline as R2Deadline,
    DeadlineBook as R2DeadlineBook,
    DeadlineStatus as R2DeadlineStatus,
    arbitrate_cancellations as r2_arbitrate_cancellations,
    cancel_deadline as r2_cancel_deadline,
    close_watermark as r2_close_watermark,
    publish_deadline as r2_publish_deadline,
)
from chronos_reference import (  # noqa: E402
    ClockDomain, Obligation as R2Obligation, make_state as r2_make_state,
)
from p_a_instruction_catchup import AObligation  # noqa: E402
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
    """Serialize all participants, deadlines, pending cancellations and W."""
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
        "obligations": [
            {
                "identity": item.identity,
                "owner": item.owner,
                "at_common": item.at_common,
                "resolved": item.resolved,
                "commutative": item.commutative,
            }
            for item in context.obligations
        ],
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
        "pending_cancellations": [
            {
                "owner": item.owner,
                "identity": item.identity,
                "generation": item.generation,
                "at_common": item.at_common,
                "microstep_level": item.microstep_level,
                "phase_order": item.phase_order,
                "phase_evidence": item.phase_evidence,
            }
            for item in book.pending_cancellations
        ],
        "max_microsteps": book.max_microsteps,
    }


def _p_a_models(data):
    """Reconstruct P-A-owned context, deadlines and cancellation requests."""
    if "state" in data:
        data = data["state"]
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
    watermark = (
        None if data["watermark_common"] is None
        else _fraction(data["watermark_common"])
    )
    obligations = tuple(
        AObligation(
            item["identity"], item["owner"], _fraction(item["at_common"]),
            item["resolved"], item["commutative"],
        )
        for item in data.get("obligations", [])
    )
    context = ADeadlineContext(
        participants=participants,
        expected_participants=frozenset(data["expected_participants"]),
        obligations=obligations,
        watermark_common=watermark,
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
            status=ADeadlineStatus(item["status"]),
        )
        for item in data["deadlines"]
    )
    cancellations = tuple(
        ACancellation(
            owner=item["owner"],
            identity=item["identity"],
            generation=item["generation"],
            at_common=_fraction(item["at_common"]),
            microstep_level=item["microstep_level"],
            phase_order=item["phase_order"],
            phase_evidence=item["phase_evidence"],
        )
        for item in data["pending_cancellations"]
    )
    book = ADeadlineBook(
        deadlines=deadlines,
        pending_cancellations=cancellations,
        watermark_common=watermark,
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _r2_models(data):
    """Reconstruct the independent R2 reference state and deadline book."""
    if "state" in data:
        data = data["state"]
    watermark = (
        None if data["watermark_common"] is None
        else _fraction(data["watermark_common"])
    )
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
    obligations = tuple(
        R2Obligation(
            item["identity"], item["owner"], _fraction(item["at_common"]),
            item["resolved"], item["commutative"],
        )
        for item in data.get("obligations", [])
    )
    context = r2_make_state(
        profile="R3-W05-CANCELLATION-RACES",
        domains=domains,
        expected_participants=data["expected_participants"],
        watermark_common=watermark,
        obligations=obligations,
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
    cancellations = tuple(
        R2Cancellation(
            owner=item["owner"],
            identity=item["identity"],
            generation=item["generation"],
            at_common=_fraction(item["at_common"]),
            microstep_level=item["microstep_level"],
            phase_order=item["phase_order"],
            phase_evidence=item["phase_evidence"],
        )
        for item in data["pending_cancellations"]
    )
    book = R2DeadlineBook(
        deadlines=deadlines,
        pending_cancellations=cancellations,
        watermark_common=watermark,
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _p_a_result_data(result, context):
    """Capture P-A outcome and full next state, including pending requests."""
    next_context = ADeadlineContext(
        participants=context.participants,
        expected_participants=context.expected_participants,
        obligations=context.obligations,
        watermark_common=result.book.watermark_common,
    )
    return {
        "code": result.code.value,
        "claimed": [list(item) for item in result.claimed],
        "state": _state_data(next_context, result.book),
    }


def _r2_result_data(result, context):
    """Capture R2 outcome and full next state with independent domain types."""
    return {
        "code": result.code.value,
        "claimed": [list(item) for item in result.claimed],
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
            "obligations": [
                {
                    "identity": item.obligation_id,
                    "owner": item.owner,
                    "at_common": item.at_common,
                    "resolved": item.resolved,
                    "commutative": item.commutative,
                }
                for item in context.obligations
            ],
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
            "pending_cancellations": [
                {
                    "owner": item.owner,
                    "identity": item.identity,
                    "generation": item.generation,
                    "at_common": item.at_common,
                    "microstep_level": item.microstep_level,
                    "phase_order": item.phase_order,
                    "phase_evidence": item.phase_evidence,
                }
                for item in result.book.pending_cancellations
            ],
            "max_microsteps": result.book.max_microsteps,
        },
    }


def _p_a_step(data, name, args):
    """Apply one operation exclusively through P-A deadline transitions."""
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
    elif name == "cancel":
        result = cancel_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
        )
    elif name == "arbitrate_cancellations":
        result = arbitrate_cancellations(book)
    elif name == "close_watermark":
        result = close_watermark(book, _fraction(args["target_common"]))
    else:
        raise ValueError(f"unsupported W05 operation: {name}")
    return _p_a_result_data(result, context)


def _r2_step(data, name, args):
    """Apply one operation exclusively through independent R2 transitions."""
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
    elif name == "cancel":
        result = r2_cancel_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
        )
    elif name == "arbitrate_cancellations":
        result = r2_arbitrate_cancellations(book)
    elif name == "close_watermark":
        result = r2_close_watermark(book, _fraction(args["target_common"]))
    else:
        raise ValueError(f"unsupported W05 operation: {name}")
    return _r2_result_data(result, context)


class W05ReplayableCancellationRaceTests(unittest.TestCase):
    def test_w05_c_before_equal_after_deadline_and_watermark_arbitration(self):
        context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        initial = _state_data(context, ADeadlineBook())
        operations = (
            {
                "sequence": 0, "name": "publish",
                "args": {
                    "owner": "cpu", "identity": "cancel-before",
                    "generation": 0, "at_common": Fraction(10),
                    "effect_id": "effect-before",
                },
            },
            {
                "sequence": 1, "name": "publish",
                "args": {
                    "owner": "cpu", "identity": "cancel-equal",
                    "generation": 0, "at_common": Fraction(20),
                    "effect_id": "effect-equal",
                },
            },
            {
                "sequence": 2, "name": "publish",
                "args": {
                    "owner": "vdp", "identity": "cancel-after",
                    "generation": 0, "at_common": Fraction(30),
                    "effect_id": "effect-after",
                },
            },
            {
                "sequence": 3, "name": "cancel",
                "args": {
                    "owner": "cpu", "identity": "cancel-before",
                    "generation": 0, "at_common": Fraction(8),
                },
            },
            {
                "sequence": 4, "name": "close_watermark",
                "args": {"target_common": Fraction(10)},
            },
            {
                "sequence": 5, "name": "arbitrate_cancellations",
                "args": {},
            },
            {
                "sequence": 6, "name": "close_watermark",
                "args": {"target_common": Fraction(10)},
            },
            {
                "sequence": 7, "name": "cancel",
                "args": {
                    "owner": "cpu", "identity": "cancel-equal",
                    "generation": 0, "at_common": Fraction(20),
                },
            },
            {
                "sequence": 8, "name": "cancel",
                "args": {
                    "owner": "vdp", "identity": "cancel-after",
                    "generation": 0, "at_common": Fraction(32),
                },
            },
            {
                "sequence": 9, "name": "close_watermark",
                "args": {"target_common": Fraction(30)},
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W05_CANCELLATION_C_LT_D_EQ_D_GT_D_AND_WATERMARK_ARBITRATION",
            provenance={
                "gate": "R3",
                "workload": "W05",
                "classification": "OBSERVATION",
                "scenario": (
                    "A C<D cancellation is admitted, blocks watermark closure "
                    "until arbitration, then permits closure. A C=D request "
                    "without explicit causal order is INDETERMINATE and a C>D "
                    "request is LATE_INPUT; both preserve the book."
                ),
                "scope": "synthetic P-A/R2 cancellation and watermark comparison",
                "limitations": (
                    "No physical SMS cancellation source, event ordering, "
                    "deadline hardware or profile phase is established."
                ),
                "contract": "R1 v0.3.18; cancellation admission and watermark closure",
            },
            initial_state=initial,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["operations"]), 10)
        self.assertEqual(len(fixture["cuts"]), 10)

        before = fixture["cuts"][3]["left_state"]
        self.assertEqual(before["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(len(before["state"]["pending_cancellations"]), 1)
        self.assertEqual(
            before["state"]["deadlines"][0]["status"],
            ADeadlineStatus.CANCEL_PENDING.value,
        )

        blocked_close = fixture["cuts"][4]["left_state"]
        self.assertEqual(blocked_close["code"], ADeadlineCode.BLOCKED.value)
        self.assertEqual(blocked_close["claimed"], [])
        self.assertEqual(blocked_close["state"], before["state"])

        arbitrated = fixture["cuts"][5]["left_state"]
        self.assertEqual(arbitrated["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(arbitrated["state"]["pending_cancellations"], [])
        self.assertEqual(
            arbitrated["state"]["deadlines"][0]["status"],
            ADeadlineStatus.CANCELLED.value,
        )

        closed_ten = fixture["cuts"][6]["left_state"]
        self.assertEqual(closed_ten["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(_fraction(closed_ten["state"]["watermark_common"]), Fraction(10))
        self.assertEqual(
            [item["status"] for item in closed_ten["state"]["deadlines"]],
            [
                ADeadlineStatus.CANCELLED.value,
                ADeadlineStatus.SCHEDULED.value,
                ADeadlineStatus.SCHEDULED.value,
            ],
        )

        equal = fixture["cuts"][7]["left_state"]
        self.assertEqual(equal["code"], ADeadlineCode.INDETERMINATE.value)
        self.assertEqual(equal["state"], closed_ten["state"])

        after = fixture["cuts"][8]["left_state"]
        self.assertEqual(after["code"], ADeadlineCode.LATE_INPUT.value)
        self.assertEqual(after["state"], closed_ten["state"])

        closed_thirty = fixture["cuts"][9]["left_state"]
        self.assertEqual(closed_thirty["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(_fraction(closed_thirty["state"]["watermark_common"]), Fraction(30))
        self.assertEqual(
            [item["status"] for item in closed_thirty["state"]["deadlines"]],
            [
                ADeadlineStatus.CANCELLED.value,
                ADeadlineStatus.CLAIMED.value,
                ADeadlineStatus.CLAIMED.value,
            ],
        )

        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


    def test_w05_unresolved_obligations_match_r2_admission_frontier(self):
        names = frozenset(("cpu", "vdp"))
        cases = (
            (
                "UNRESOLVED_BEHIND_FRONTIER",
                (AParticipant("cpu", Fraction(5), Fraction(10)),
                 AParticipant("vdp", Fraction(0), Fraction(20))),
                (AObligation("behind", "cpu", Fraction(4)),),
                None,
                ADeadlineCode.INVARIANT_VIOLATION.value,
            ),
            (
                "UNRESOLVED_AT_WATERMARK",
                (AParticipant("cpu", Fraction(0), Fraction(10)),
                 AParticipant("vdp", Fraction(0), Fraction(10))),
                (AObligation("at-watermark", "cpu", Fraction(6)),),
                Fraction(6),
                ADeadlineCode.INVARIANT_VIOLATION.value,
            ),
            (
                "UNRESOLVED_AHEAD_OF_FRONTIER",
                (AParticipant("cpu", Fraction(5), Fraction(10)),
                 AParticipant("vdp", Fraction(0), Fraction(20))),
                (AObligation("future", "cpu", Fraction(7)),),
                None,
                ADeadlineCode.ACCEPTED.value,
            ),
            (
                "RESOLVED_BEHIND_FRONTIER",
                (AParticipant("cpu", Fraction(5), Fraction(10)),
                 AParticipant("vdp", Fraction(0), Fraction(20))),
                (AObligation("resolved", "cpu", Fraction(4), resolved=True),),
                None,
                ADeadlineCode.ACCEPTED.value,
            ),
        )
        for name, participants, obligations, watermark, expected_code in cases:
            with self.subTest(case=name):
                context = ADeadlineContext(
                    participants, names, obligations, watermark
                )
                book = ADeadlineBook(watermark_common=watermark)
                fixture = capture_differential_fixture(
                    fixture_id=f"W05_OBLIGATION_{name}",
                    provenance={
                        "gate": "R3",
                        "workload": "W05",
                        "classification": "OBSERVATION",
                        "scenario": name,
                        "scope": "P-A/R2 unresolved causal obligation admission",
                        "limitations": (
                            "Synthetic common-time obligations only; no physical "
                            "SMS bus or interrupt source is established."
                        ),
                        "contract": "R1 v0.3.18; independent R2 deadline reference",
                    },
                    initial_state=_state_data(context, book),
                    operations=(
                        {
                            "sequence": 0,
                            "name": "publish",
                            "args": {
                                "owner": "cpu",
                                "identity": f"obligation-{name.lower()}",
                                "generation": 0,
                                "at_common": Fraction(10),
                                "effect_id": "obligation-effect",
                            },
                        },
                    ),
                    left_step=_p_a_step,
                    right_step=_r2_step,
                )
                record_replayable_fixture(fixture)
                self.assertTrue(fixture["all_equal"])
                self.assertIsNone(fixture["first_divergence"])
                self.assertEqual(fixture["cuts"][0]["left_state"]["code"], expected_code)

    def test_w05_watermark_may_pass_c_before_arbitration_but_not_d(self):
        context = ADeadlineContext(
            participants=(
                AParticipant("cpu", Fraction(0), Fraction(100)),
                AParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        operations = (
            {
                "sequence": 0, "name": "publish",
                "args": {
                    "owner": "cpu", "identity": "watermark-before-d",
                    "generation": 0, "at_common": Fraction(10),
                    "effect_id": "effect-watermark",
                },
            },
            {
                "sequence": 1, "name": "cancel",
                "args": {
                    "owner": "cpu", "identity": "watermark-before-d",
                    "generation": 0, "at_common": Fraction(8),
                },
            },
            {
                "sequence": 2, "name": "close_watermark",
                "args": {"target_common": Fraction(9)},
            },
            {
                "sequence": 3, "name": "arbitrate_cancellations",
                "args": {},
            },
            {
                "sequence": 4, "name": "close_watermark",
                "args": {"target_common": Fraction(10)},
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W05_WATERMARK_CROSSES_C_BUT_NOT_D_BEFORE_ARBITRATION",
            provenance={
                "gate": "R3",
                "workload": "W05",
                "classification": "OBSERVATION",
                "scenario": (
                    "An admitted C<D cancellation may remain pending while W "
                    "advances beyond C but below D; arbitration is required "
                    "before W crosses D."
                ),
                "scope": "synthetic P-A/R2 cancellation and watermark comparison",
                "limitations": (
                    "No physical SMS cancellation source, event ordering, "
                    "deadline hardware or profile phase is established."
                ),
                "contract": "R1 v0.3.18; R2-T06",
            },
            initial_state=_state_data(context, ADeadlineBook()),
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), 5)
        pending = fixture["cuts"][1]["left_state"]
        self.assertEqual(pending["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(len(pending["state"]["pending_cancellations"]), 1)

        crossed_c = fixture["cuts"][2]["left_state"]
        self.assertEqual(crossed_c["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(_fraction(crossed_c["state"]["watermark_common"]), Fraction(9))
        self.assertEqual(len(crossed_c["state"]["pending_cancellations"]), 1)

        arbitrated = fixture["cuts"][3]["left_state"]
        self.assertEqual(arbitrated["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(arbitrated["state"]["pending_cancellations"], [])
        self.assertEqual(
            arbitrated["state"]["deadlines"][0]["status"],
            ADeadlineStatus.CANCELLED.value,
        )

        crossed_d = fixture["cuts"][4]["left_state"]
        self.assertEqual(crossed_d["code"], ADeadlineCode.ACCEPTED.value)
        self.assertEqual(_fraction(crossed_d["state"]["watermark_common"]), Fraction(10))

if __name__ == "__main__":
    unittest.main(verbosity=2)
