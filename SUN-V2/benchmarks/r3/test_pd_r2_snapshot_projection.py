"""W09 tests for the explicit shared-field P-D -> R2 projection."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "verification" / "r2_reference"))

from p_d_discrete_agenda import (  # noqa: E402
    AgendaCancellation, AgendaEvent, AgendaState, EventStatus,
)
from pd_r2_snapshot_projection import (  # noqa: E402
    ProjectionCode, project_pd_shared_state,
)
from snapshot_reference import (  # noqa: E402
    DeadlineRecord, DeadlineState, InterruptRecord, MachineState,
    SnapshotCode, capture as r2_capture, restore as r2_restore,
)


def candidate_state():
    return AgendaState(
        heap=((Fraction(9), "pending", 3),),
        events=(
            AgendaEvent("done", 2, "cpu", Fraction(8), "write-done", EventStatus.COMPLETED),
            AgendaEvent("old", 1, "cpu", Fraction(5), "old-write", EventStatus.SUPERSEDED),
            AgendaEvent("pending", 3, "vdp", Fraction(9), "write-pending", EventStatus.SCHEDULED),
            AgendaEvent("claimed", 4, "cpu", Fraction(10), "write-claimed", EventStatus.CLAIMED),
            AgendaEvent("cancelled", 5, "vdp", Fraction(11), "write-cancelled", EventStatus.CANCELLED),
        ),
        watermark_common=Fraction(7),
    )


class PdR2SnapshotProjectionTests(unittest.TestCase):
    def test_maps_only_shared_deadline_fields_and_watermark(self):
        result = project_pd_shared_state(candidate_state())
        self.assertEqual(result.code, ProjectionCode.OK)
        projection = result.projection
        self.assertEqual(projection.watermark, Fraction(7))
        self.assertEqual(projection.deadlines, (
            DeadlineRecord("cancelled", 5, Fraction(11), DeadlineState.CANCELLED),
            DeadlineRecord("claimed", 4, Fraction(10), DeadlineState.CLAIMED),
            DeadlineRecord("done", 2, Fraction(8), DeadlineState.COMPLETED),
            DeadlineRecord("old", 1, Fraction(5), DeadlineState.SUPERSEDED),
            DeadlineRecord("pending", 3, Fraction(9), DeadlineState.SCHEDULED),
        ))
        self.assertTrue(any("effect ledgers" in item for item in projection.unrepresented))
        self.assertTrue(any("TransactionBook" in item for item in projection.unrepresented))
        self.assertTrue(any("interrupts" in item for item in projection.unrepresented))

    def test_projected_shared_fields_survive_r2_capture_and_restore(self):
        projection = project_pd_shared_state(candidate_state()).projection
        # This R2 fixture is a projection envelope, not a claim that absent
        # P-D machine fields (domains, transactions, IRQ and replay cursor)
        # are semantically equivalent to empty R2 fields.
        r2_state = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=projection.watermark,
            deadlines=projection.deadlines,
            transactions=(),
            interrupts=InterruptRecord(),
            input_cursor=0,
        )
        captured = r2_capture(r2_state)
        self.assertEqual(captured.code, SnapshotCode.OK)
        restored = r2_restore(
            r2_state, captured.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        self.assertEqual(restored.restored_state.watermark, projection.watermark)
        self.assertEqual(restored.restored_state.deadlines, projection.deadlines)
        self.assertEqual(
            tuple((item.identity, item.generation, item.at_common, item.state)
                  for item in restored.restored_state.deadlines),
            tuple((item.identity, item.generation, item.at_common, item.state)
                  for item in projection.deadlines),
        )

    def test_cancel_pending_is_not_guessed_as_cancelled_or_scheduled(self):
        state = AgendaState(
            events=(
                AgendaEvent("cancel-pending", 1, "cpu", Fraction(4), "effect",
                            EventStatus.CANCEL_PENDING),
            ),
            pending_cancellations=(
                AgendaCancellation("cancel-pending", 1, "cpu", Fraction(3)),
            ),
        )
        result = project_pd_shared_state(state)
        self.assertEqual(result.code, ProjectionCode.UNSUPPORTED)
        self.assertIsNone(result.projection)
        self.assertIn("no R2 deadline-state counterpart", result.detail)

    def test_invalid_candidate_state_is_rejected_without_projection(self):
        state = AgendaState(events=(
            AgendaEvent("duplicate", 0, "cpu", Fraction(1), "a"),
            AgendaEvent("duplicate", 0, "cpu", Fraction(2), "b"),
        ))
        result = project_pd_shared_state(state)
        self.assertEqual(result.code, ProjectionCode.INVALID_INPUT)
        self.assertIsNone(result.projection)


if __name__ == "__main__":
    unittest.main(verbosity=2)
