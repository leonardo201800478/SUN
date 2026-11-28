"""Executable W09 boundary matrix: candidate operation suffix != R2 accepted-input replay."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "verification" / "r2_reference"))

from p_d_discrete_agenda import AgendaState  # noqa: E402
from r3_candidate_snapshots import capture, make_replay_event  # noqa: E402
from replay_reference import (  # noqa: E402
    AcceptedInput, ExternalInput, InputContract, Participant, ReplayState,
)
from replay_boundary_audit import (  # noqa: E402
    ReplayBoundaryCode, audit_candidate_replay_boundary,
)


def candidate_artifact(profile="SMS-NTSC", cursor=3):
    result = capture(
        "P-D", AgendaState(), profile=profile,
        model_version="P-D-R3-W09-1", input_cursor=cursor,
    )
    if result.code.value != "Ok":
        raise AssertionError(result.detail)
    return result.artifact


def r2_replay_state(profile="SMS-NTSC"):
    event = ExternalInput(
        identity="input-1",
        input_class="button",
        owner="cpu",
        at_common=Fraction(2),
        payload="A",
        participants=("cpu",),
        logical_order=1,
    )
    return ReplayState(
        profile=profile,
        participants=(Participant("cpu", Fraction(1), Fraction(4)),),
        contracts=(InputContract("button", ("cpu",)),),
        accepted=(AcceptedInput(1, event),),
    )


class ReplayBoundaryAuditTests(unittest.TestCase):
    def test_matching_profile_still_refuses_to_equate_replay_ledgers(self):
        artifact = candidate_artifact(cursor=3)
        suffix = (make_replay_event(4, "arbitrate", {}),)
        r2_state = r2_replay_state()
        result = audit_candidate_replay_boundary(artifact, suffix, r2_state)
        self.assertEqual(result.code, ReplayBoundaryCode.INCOMPARABLE)
        audit = result.audit
        self.assertTrue(audit.profile_matches)
        self.assertEqual(audit.candidate_harness_cursor, 3)
        self.assertEqual(audit.candidate_suffix_sequences, (4,))
        self.assertEqual(audit.r2_accepted_sequences, (1,))
        self.assertFalse(audit.sequence_spaces_comparable)
        self.assertTrue(any("accepted input log" in item for item in audit.not_comparable))
        self.assertTrue(any("virtual_effects" in item for item in audit.not_comparable))
        self.assertEqual(r2_state, r2_replay_state())

    def test_profile_mismatch_is_reported_without_claiming_replay_equivalence(self):
        result = audit_candidate_replay_boundary(
            candidate_artifact(profile="SMS-NTSC"),
            (make_replay_event(4, "arbitrate", {}),),
            r2_replay_state(profile="SG-1000"),
        )
        self.assertEqual(result.code, ReplayBoundaryCode.PROFILE_MISMATCH)
        self.assertFalse(result.audit.profile_matches)
        self.assertFalse(result.audit.sequence_spaces_comparable)

    def test_candidate_operation_gap_is_invalid_and_does_not_produce_audit(self):
        result = audit_candidate_replay_boundary(
            candidate_artifact(cursor=3),
            (make_replay_event(5, "arbitrate", {}),),
            r2_replay_state(),
        )
        self.assertEqual(result.code, ReplayBoundaryCode.INVALID_INPUT)
        self.assertIsNone(result.audit)


if __name__ == "__main__":
    unittest.main(verbosity=2)
