"""Replayable W06 fixture for UNKNOWN phase-basis comparability.

Two simultaneous non-commutative deadlines carry distinct, syntactically valid
phase bases. P-A refuses to compare them; the R2 oracle accepts the numeric
phase_order fields. This is an intentional model divergence, not physical SMS
phase evidence or authentication of either synthetic validation record.
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
    APhaseBasis,
    close_watermark,
    publish_deadline,
)
from deadline_reference import (  # noqa: E402
    Deadline as R2Deadline,
    DeadlineBook as R2DeadlineBook,
    DeadlineStatus as R2DeadlineStatus,
    close_watermark as r2_close_watermark,
    publish_deadline as r2_publish_deadline,
)
from chronos_reference import ClockDomain, make_state as r2_make_state  # noqa: E402
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)


def _fraction(value):
    """Restore exact rational coordinates from the replay harness."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _basis_from_data(data):
    """Rebuild a P-A-owned phase basis from fixture metadata."""
    return APhaseBasis(
        profile_id=data["profile_id"],
        profile_revision=data["profile_revision"],
        coordinate=data["coordinate"],
        unit=data["unit"],
        scale_numerator=data["scale_numerator"],
        scale_denominator=data["scale_denominator"],
        basis_id=data["basis_id"],
        basis_revision=data["basis_revision"],
        validation_record_id=data["validation_record_id"],
        validation_record_sha256=data["validation_record_sha256"],
    )


def _state_data(context, book, phase_basis_descriptors):
    """Serialize complete deadline state plus explicit non-R2 basis sidecars."""
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
        # R2 has no phase_basis field. Preserve the declarative evidence
        # descriptors as fixture metadata so each adapter receives identical
        # inputs; only P-A interprets them as part of its comparability rule.
        "phase_basis_descriptors": phase_basis_descriptors,
    }


def _p_a_models(data):
    """Reconstruct P-A-owned deadlines, including their basis descriptors."""
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
    context = ADeadlineContext(
        participants=participants,
        expected_participants=frozenset(data["expected_participants"]),
        watermark_common=watermark,
    )
    basis_by_key = {
        (item["identity"], item["generation"]): _basis_from_data(item["basis"])
        for item in data["phase_basis_descriptors"]
    }
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
            phase_basis=basis_by_key.get((item["identity"], item["generation"])),
        )
        for item in data["deadlines"]
    )
    book = ADeadlineBook(
        deadlines=deadlines,
        pending_cancellations=tuple(),
        watermark_common=watermark,
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _r2_models(data):
    """Reconstruct R2-owned state; basis sidecars remain opaque to the oracle."""
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
    context = r2_make_state(
        profile="R3-W06-SYNTHETIC-PHASE-BASIS-COMPARISON",
        domains=domains,
        expected_participants=data["expected_participants"],
        watermark_common=watermark,
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
        watermark_common=watermark,
        max_microsteps=data["max_microsteps"],
    )
    return context, book


def _p_a_result_data(result, context, phase_basis_descriptors):
    """Capture P-A outcome and complete state plus basis sidecars."""
    next_context = ADeadlineContext(
        participants=context.participants,
        expected_participants=context.expected_participants,
        watermark_common=result.book.watermark_common,
    )
    return {
        "code": result.code.value,
        "claimed": [list(item) for item in result.claimed],
        "state": _state_data(next_context, result.book, phase_basis_descriptors),
    }


def _r2_result_data(result, context, phase_basis_descriptors):
    """Capture R2 outcome and state while preserving opaque fixture metadata."""
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
            "phase_basis_descriptors": phase_basis_descriptors,
        },
    }


def _basis_descriptor(identity, generation, basis):
    """Return a stable sidecar entry; values are synthetic and unauthenticated."""
    return {
        "identity": identity,
        "generation": generation,
        "basis": {
            "profile_id": basis["profile_id"],
            "profile_revision": basis["profile_revision"],
            "coordinate": basis["coordinate"],
            "unit": basis["unit"],
            "scale_numerator": basis["scale_numerator"],
            "scale_denominator": basis["scale_denominator"],
            "basis_id": basis["basis_id"],
            "basis_revision": basis["basis_revision"],
            "validation_record_id": basis["validation_record_id"],
            "validation_record_sha256": basis["validation_record_sha256"],
        },
    }


def _p_a_step(data, name, args):
    """Apply one operation exclusively through P-A deadline transitions."""
    context, book = _p_a_models(data)
    if "state" in data:
        data = data["state"]
    phase_bases = list(data["phase_basis_descriptors"])
    if name == "publish":
        basis = args["phase_basis"]
        result = publish_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
            effect_id=args["effect_id"],
            commutative=args["commutative"],
            phase_order=args["phase_order"],
            phase_evidence=args["phase_evidence"],
            phase_basis=_basis_from_data(basis),
        )
        phase_bases.append(
            _basis_descriptor(args["identity"], args["generation"], basis)
        )
    elif name == "close_watermark":
        result = close_watermark(book, _fraction(args["target_common"]))
    else:
        raise ValueError(f"unsupported W06 operation: {name}")
    return _p_a_result_data(result, context, phase_bases)


def _r2_step(data, name, args):
    """Apply one operation through R2; basis sidecars do not affect R2 semantics."""
    context, book = _r2_models(data)
    if "state" in data:
        data = data["state"]
    phase_bases = list(data["phase_basis_descriptors"])
    if name == "publish":
        result = r2_publish_deadline(
            book, context,
            owner=args["owner"],
            identity=args["identity"],
            generation=args["generation"],
            at_common=_fraction(args["at_common"]),
            effect_id=args["effect_id"],
            commutative=args["commutative"],
            phase_order=args["phase_order"],
            phase_evidence=args["phase_evidence"],
        )
        phase_bases.append(
            _basis_descriptor(args["identity"], args["generation"], args["phase_basis"])
        )
    elif name == "close_watermark":
        result = r2_close_watermark(book, _fraction(args["target_common"]))
    else:
        raise ValueError(f"unsupported W06 operation: {name}")
    return _r2_result_data(result, context, phase_bases)


class W06ReplayablePhaseBasisUnknownTests(unittest.TestCase):
    def test_w06_distinct_phase_bases_remain_unknown_against_r2_baseline(self):
        participants = (
            AParticipant("cpu", Fraction(0), Fraction(100)),
            AParticipant("vdp", Fraction(0), Fraction(100)),
        )
        context = ADeadlineContext(
            participants=participants,
            expected_participants=frozenset({"cpu", "vdp"}),
        )
        initial = _state_data(context, ADeadlineBook(), [])
        basis_a = {
            "profile_id": "profile-A",
            "profile_revision": "rev-1",
            "coordinate": "phase-coordinate",
            "unit": "phase-unit",
            "scale_numerator": 1,
            "scale_denominator": 1,
            "basis_id": "basis-A",
            "basis_revision": "rev-1",
            "validation_record_id": "synthetic-validation-A",
            "validation_record_sha256": "a" * 64,
        }
        basis_b = {
            "profile_id": "profile-B",
            "profile_revision": "rev-1",
            "coordinate": "phase-coordinate",
            "unit": "phase-unit",
            "scale_numerator": 1,
            "scale_denominator": 1,
            "basis_id": "basis-B",
            "basis_revision": "rev-1",
            "validation_record_id": "synthetic-validation-B",
            "validation_record_sha256": "b" * 64,
        }
        operations = (
            {
                "sequence": 0,
                "name": "publish",
                "args": {
                    "owner": "cpu",
                    "identity": "phase-a",
                    "generation": 0,
                    "at_common": Fraction(10),
                    "effect_id": "effect-a",
                    "commutative": False,
                    "phase_order": 1,
                    "phase_evidence": "capture-A",
                    "phase_basis": basis_a,
                },
            },
            {
                "sequence": 1,
                "name": "publish",
                "args": {
                    "owner": "vdp",
                    "identity": "phase-b",
                    "generation": 0,
                    "at_common": Fraction(10),
                    "effect_id": "effect-b",
                    "commutative": False,
                    "phase_order": 2,
                    "phase_evidence": "capture-B",
                    "phase_basis": basis_b,
                },
            },
            {
                "sequence": 2,
                "name": "close_watermark",
                "args": {"target_common": Fraction(10)},
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W06_DISTINCT_SYNTHETIC_PHASE_BASES_KEEP_P_A_UNKNOWN",
            provenance={
                "gate": "R3",
                "workload": "W06",
                "classification": "OBSERVATION",
                "scenario": (
                    "Two simultaneous non-commutative deadlines carry distinct "
                    "synthetic phase bases. P-A requires one comparable basis "
                    "and returns INDETERMINATE; the R2 baseline uses the numeric "
                    "phase_order fields and accepts the group."
                ),
                "scope": "synthetic P-A/R2 phase-basis comparability differential",
                "limitations": (
                    "Validation record identifiers and SHA-256 strings are "
                    "synthetic placeholders, not external records or trust "
                    "anchors. No SMS profile phase, clock relation, physical "
                    "ordering or hardware behavior is authenticated."
                ),
                "contract": "R1 v0.3.18; simultaneous order remains UNKNOWN without a common validated basis",
            },
            initial_state=initial,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertFalse(fixture["all_equal"])
        self.assertEqual(fixture["first_divergence"]["sequence"], 2)
        self.assertEqual(len(fixture["cuts"]), 3)
        self.assertTrue(fixture["cuts"][0]["equal"])
        self.assertTrue(fixture["cuts"][1]["equal"])

        p_a = fixture["cuts"][2]["left_state"]
        r2 = fixture["cuts"][2]["right_state"]
        self.assertEqual(p_a["code"], ADeadlineCode.INDETERMINATE.value)
        self.assertEqual(p_a["claimed"], [])
        self.assertIsNone(p_a["state"]["watermark_common"])
        self.assertEqual(
            [item["status"] for item in p_a["state"]["deadlines"]],
            [ADeadlineStatus.SCHEDULED.value, ADeadlineStatus.SCHEDULED.value],
        )
        self.assertEqual(r2["code"], "Accepted")
        self.assertEqual(r2["claimed"], [["phase-a", 0], ["phase-b", 0]])
        self.assertEqual(_fraction(r2["state"]["watermark_common"]), Fraction(10))
        self.assertEqual(
            [item["status"] for item in r2["state"]["deadlines"]],
            [R2DeadlineStatus.CLAIMED.value, R2DeadlineStatus.CLAIMED.value],
        )

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
