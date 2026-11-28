"""W09 tests for the explicit shared-field P-D -> R2 projection."""
from __future__ import annotations

import sys
import unittest
from dataclasses import replace
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
    AckCommit, AckPresence, DeadlineRecord, DeadlineState, InterruptRecord, MachineState,
    SnapshotArtifact as R2SnapshotArtifact, SnapshotCode, capture as r2_capture,
    restore as r2_restore,
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


    def test_every_p_d_lifecycle_status_is_mapped_or_explicitly_unsupported(self):
        base = candidate_state()
        target = next(item for item in base.events if item.identity == "pending")
        expected = {
            EventStatus.SCHEDULED: DeadlineState.SCHEDULED,
            EventStatus.CANCELLED: DeadlineState.CANCELLED,
            EventStatus.SUPERSEDED: DeadlineState.SUPERSEDED,
            EventStatus.CLAIMED: DeadlineState.CLAIMED,
            EventStatus.COMPLETED: DeadlineState.COMPLETED,
        }
        for status in EventStatus:
            with self.subTest(status=status):
                updated = replace(target, status=status)
                events = tuple(
                    updated if item.identity == target.identity else item
                    for item in base.events
                )
                heap = base.heap if status is EventStatus.SCHEDULED else ()
                pending_cancellations = ()
                if status is EventStatus.CANCEL_PENDING:
                    pending_cancellations = (
                        AgendaCancellation(
                            target.identity, target.generation, target.owner,
                            Fraction(8),
                        ),
                    )
                candidate = replace(
                    base, events=events, heap=heap,
                    pending_cancellations=pending_cancellations,
                )
                result = project_pd_shared_state(candidate)
                if status is EventStatus.CANCEL_PENDING:
                    self.assertEqual(result.code, ProjectionCode.UNSUPPORTED)
                    self.assertIsNone(result.projection)
                else:
                    self.assertEqual(result.code, ProjectionCode.OK)
                    mapped = next(
                        item for item in result.projection.deadlines
                        if item.identity == target.identity
                    )
                    self.assertEqual(mapped.state, expected[status])

    def test_projection_is_stable_under_event_permutation_and_effect_id_changes(self):
        state = candidate_state()
        baseline = project_pd_shared_state(state)
        self.assertEqual(baseline.code, ProjectionCode.OK)
        permuted = replace(state, events=tuple(reversed(state.events)))
        changed_effects = replace(
            state,
            events=tuple(
                replace(item, effect_id=f"unmapped-{item.identity}")
                for item in state.events
            ),
        )
        permuted_result = project_pd_shared_state(permuted)
        effects_result = project_pd_shared_state(changed_effects)
        self.assertEqual(permuted_result.code, ProjectionCode.OK)
        self.assertEqual(effects_result.code, ProjectionCode.OK)
        self.assertEqual(permuted_result.projection, baseline.projection)
        self.assertEqual(effects_result.projection, baseline.projection)

    def test_invalid_candidate_state_is_rejected_without_projection(self):
        state = AgendaState(events=(
            AgendaEvent("duplicate", 0, "cpu", Fraction(1), "a"),
            AgendaEvent("duplicate", 0, "cpu", Fraction(2), "b"),
        ))
        result = project_pd_shared_state(state)
        self.assertEqual(result.code, ProjectionCode.INVALID_INPUT)
        self.assertIsNone(result.projection)


    def test_r2_deadline_states_without_p_d_counterparts_remain_unrepresented(self):
        p_d_states = {
            DeadlineState.SCHEDULED,
            DeadlineState.CANCELLED,
            DeadlineState.SUPERSEDED,
            DeadlineState.CLAIMED,
            DeadlineState.COMPLETED,
        }
        self.assertEqual(
            set(DeadlineState) - p_d_states,
            {DeadlineState.DISPATCHED, DeadlineState.FAILED},
        )

    def test_r2_unknown_int_ack_cut_is_unsupported_without_artifact_or_mutation(self):
        state = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=None,
            deadlines=(),
            transactions=(),
            interrupts=InterruptRecord(
                acknowledge_active=True,
                device_effect_presence=AckPresence.UNKNOWN,
                device_effect_commit=AckCommit.UNKNOWN,
            ),
            input_cursor=0,
        )
        captured = r2_capture(state)
        self.assertEqual(captured.code, SnapshotCode.UNSUPPORTED)
        self.assertIs(captured.active_state, state)
        self.assertIsNone(captured.artifact)
        self.assertIsNone(captured.restored_state)


    def test_r2_restore_rejects_forged_unknown_int_ack_artifact_atomically(self):
        active = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=Fraction(3),
            deadlines=(),
            transactions=(),
            interrupts=InterruptRecord(),
            input_cursor=2,
        )
        unsupported_cut = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=Fraction(9),
            deadlines=(),
            transactions=(),
            interrupts=InterruptRecord(
                acknowledge_active=True,
                device_effect_presence=AckPresence.UNKNOWN,
                device_effect_commit=AckCommit.UNKNOWN,
            ),
            input_cursor=8,
        )
        forged_artifact = R2SnapshotArtifact(
            profile=unsupported_cut.profile,
            model_version=unsupported_cut.model_version,
            state=unsupported_cut,
        )
        restored = r2_restore(
            active,
            forged_artifact,
            expected_profile="SMS-NTSC",
            expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.code, SnapshotCode.INCOMPATIBLE)
        self.assertIs(restored.active_state, active)
        self.assertIsNone(restored.artifact)
        self.assertIsNone(restored.restored_state)
        self.assertEqual(active.watermark, Fraction(3))
        self.assertEqual(active.input_cursor, 2)

    def test_projection_roundtrip_preserves_each_shared_deadline_coordinate(self):
        source = candidate_state()
        result = project_pd_shared_state(source)
        self.assertEqual(result.code, ProjectionCode.OK)
        projection = result.projection
        state = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=projection.watermark,
            deadlines=projection.deadlines,
            transactions=(),
            interrupts=InterruptRecord(),
            input_cursor=0,
        )
        captured = r2_capture(state)
        self.assertEqual(captured.code, SnapshotCode.OK)
        restored = r2_restore(
            state,
            captured.artifact,
            expected_profile="SMS-NTSC",
            expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        projected = project_pd_shared_state(source).projection
        actual = restored.restored_state
        self.assertEqual(actual.watermark, projected.watermark)
        self.assertEqual(
            tuple((d.identity, d.generation, d.at_common, d.state)
                  for d in actual.deadlines),
            tuple((d.identity, d.generation, d.at_common, d.state)
                  for d in projected.deadlines),
        )
        self.assertTrue(all(not d.committed_effects and not d.pending_effects
                            for d in actual.deadlines))


if __name__ == "__main__":
    unittest.main(verbosity=2)
