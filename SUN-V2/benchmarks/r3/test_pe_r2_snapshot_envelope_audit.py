"""W09 tests for P-E envelope metadata and explicit non-projection boundaries."""
from __future__ import annotations

import sys
import unittest
from dataclasses import replace
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "benchmarks" / "r3"))

from p_e_sync_on_demand import SyncDomain, SyncState  # noqa: E402
from r3_candidate_snapshots import SnapshotCode, capture  # noqa: E402
from pe_r2_snapshot_envelope_audit import (  # noqa: E402
    EnvelopeAuditCode, audit_pe_envelope,
)


class PeR2EnvelopeAuditTests(unittest.TestCase):
    def _artifact(self):
        state = SyncState(
            domains=(SyncDomain("cpu", Fraction(2), Fraction(8)),),
            transitions=(),
        )
        result = capture(
            "P-E", state, profile="SMS-NTSC",
            model_version="P-E-R3-W09-1", input_cursor=7,
        )
        self.assertEqual(result.code, SnapshotCode.OK)
        return result.artifact

    def test_audits_only_shared_envelope_and_marks_cursor_as_harness_metadata(self):
        result = audit_pe_envelope(self._artifact())
        self.assertEqual(result.code, EnvelopeAuditCode.OK)
        audit = result.audit
        self.assertEqual(audit.profile, "SMS-NTSC")
        self.assertEqual(audit.model_version, "P-E-R3-W09-1")
        self.assertEqual(audit.harness_input_cursor, 7)
        self.assertIn("input_cursor is harness metadata", audit.candidate_only)
        self.assertTrue(any("safe_horizon" in item for item in audit.not_mapped_to_r2))
        self.assertTrue(any("not R2 deadlines" in item for item in audit.not_mapped_to_r2))
        self.assertTrue(any("interrupts/INT-ack" in item for item in audit.not_mapped_to_r2))

    def test_does_not_accept_p_d_artifact_as_p_e_envelope(self):
        artifact = replace(self._artifact(), candidate="P-D")
        result = audit_pe_envelope(artifact)
        self.assertEqual(result.code, EnvelopeAuditCode.UNSUPPORTED)
        self.assertIsNone(result.audit)

    def test_rejects_boolean_cursor_without_producing_audit(self):
        artifact = replace(self._artifact(), input_cursor=True)
        result = audit_pe_envelope(artifact)
        self.assertEqual(result.code, EnvelopeAuditCode.INVALID_INPUT)
        self.assertIsNone(result.audit)


if __name__ == "__main__":
    unittest.main(verbosity=2)
