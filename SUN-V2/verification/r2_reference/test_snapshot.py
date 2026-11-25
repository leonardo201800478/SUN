"""R2-T11 snapshot/restore state and atomicity regressions."""
import unittest
from fractions import Fraction

from snapshot_reference import (
    AckCommit, AckPresence, DeadlineRecord, DeadlineState, DomainState,
    InterruptRecord, MachineState, SnapshotArtifact, SnapshotCode,
    TransactionRecord, TransactionState, capture, restore,
)


def machine(**changes):
    values = dict(
        profile="SMS-NTSC",
        model_version="R1-0.3.18",
        domains=(DomainState("cpu", Fraction(10), Fraction(1, 3)),),
        watermark=Fraction(8),
        deadlines=(),
        transactions=(),
        interrupts=InterruptRecord(),
        input_cursor=7,
    )
    values.update(changes)
    return MachineState(**values)


class R2T11SnapshotTests(unittest.TestCase):
    def test_every_deadline_lifecycle_state_round_trips(self):
        for status in DeadlineState:
            committed = ("e1",) if status in (DeadlineState.DISPATCHED, DeadlineState.COMPLETED, DeadlineState.FAILED) else ()
            pending = ("e2",) if status in (DeadlineState.DISPATCHED, DeadlineState.FAILED) else ()
            error = "fault" if status is DeadlineState.FAILED else None
            deadline = DeadlineRecord("d", 1, Fraction(9), status, committed, pending, error)
            state = machine(deadlines=(deadline,))
            artifact = capture(state)
            self.assertEqual(artifact.code, SnapshotCode.OK)
            restored = restore(machine(), artifact.artifact, expected_profile="SMS-NTSC", expected_version="R1-0.3.18")
            self.assertEqual(restored.restored_state, state)

    def test_every_bus_transaction_control_state_round_trips(self):
        for status in TransactionState:
            error = "fault" if status is TransactionState.FAILED else None
            pending = () if status is TransactionState.COMPLETED else ("w2",)
            tx = TransactionRecord(
                "tx", 2, status, Fraction(5), next_marker="commit",
                wait_source="fixture" if status is TransactionState.WAITING else None,
                sampled=(("read", "0x44"),), committed_effects=("w1",),
                pending_effects=pending, error=error,
            )
            state = machine(transactions=(tx,))
            artifact = capture(state)
            self.assertEqual(artifact.code, SnapshotCode.OK)
            self.assertEqual(
                restore(machine(), artifact.artifact, expected_profile="SMS-NTSC", expected_version="R1-0.3.18").restored_state,
                state,
            )

    def test_snapshot_between_sample_and_commit_preserves_sample_and_pending_effect(self):
        tx = TransactionRecord(
            "tx", 0, TransactionState.IN_PROGRESS, Fraction(3), next_marker="commit",
            sampled=(("read-status", "0x81"),), pending_effects=("write-port",),
        )
        state = machine(transactions=(tx,))
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.transactions[0].sampled, (("read-status", "0x81"),))
        self.assertEqual(restored.restored_state.transactions[0].pending_effects, ("write-port",))

    def test_snapshot_after_commit_preserves_effect_without_reapplication(self):
        tx = TransactionRecord(
            "tx", 0, TransactionState.IN_PROGRESS, Fraction(4),
            committed_effects=("write-port",), pending_effects=("publish-irq",),
        )
        state = machine(transactions=(tx,))
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.transactions[0].committed_effects, ("write-port",))

    def test_ack_vector_and_device_commit_ledger_are_independent_in_snapshot(self):
        irq = InterruptRecord(
            int_level=True, accepted="INT", acknowledge_active=True, vector_sampled="0x38",
            device_effect_presence=AckPresence.PRESENT, device_effect_commit=AckCommit.PENDING,
            commit_at_common=Fraction(12),
        )
        state = machine(interrupts=irq)
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.interrupts.vector_sampled, "0x38")
        self.assertEqual(restored.restored_state.interrupts.device_effect_commit, AckCommit.PENDING)

    def test_unknown_ack_effect_presence_returns_unsupported_without_artifact_or_mutation(self):
        irq = InterruptRecord(
            accepted="INT", acknowledge_active=True,
            device_effect_presence=AckPresence.UNKNOWN, device_effect_commit=AckCommit.UNKNOWN,
        )
        state = machine(interrupts=irq)
        result = capture(state)
        self.assertEqual(result.code, SnapshotCode.UNSUPPORTED)
        self.assertIsNone(result.artifact)
        self.assertEqual(result.active_state, state)

    def test_unknown_ack_commit_boundary_returns_unsupported_without_artifact(self):
        irq = InterruptRecord(
            accepted="INT", acknowledge_active=True, device_effect_presence=AckPresence.PRESENT,
            device_effect_commit=AckCommit.PENDING, commit_at_common=None,
        )
        result = capture(machine(interrupts=irq))
        self.assertEqual(result.code, SnapshotCode.UNSUPPORTED)
        self.assertIsNone(result.artifact)

    def test_unknown_ack_profile_does_not_block_snapshot_outside_ack_window(self):
        irq = InterruptRecord(device_effect_presence=AckPresence.UNKNOWN, device_effect_commit=AckCommit.UNKNOWN)
        state = machine(interrupts=irq)
        self.assertEqual(capture(state).code, SnapshotCode.OK)

    def test_known_profile_with_no_ack_device_effect_is_supported(self):
        irq = InterruptRecord(accepted="INT", acknowledge_active=True, vector_sampled="0x38")
        self.assertEqual(capture(machine(interrupts=irq)).code, SnapshotCode.OK)

    def test_wrong_profile_restore_is_atomic(self):
        source = machine()
        artifact = capture(source).artifact
        active = machine(watermark=Fraction(9))
        result = restore(active, artifact, expected_profile="SG-1000", expected_version="R1-0.3.18")
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_wrong_model_version_restore_is_atomic(self):
        source = machine()
        artifact = capture(source).artifact
        active = machine(watermark=Fraction(9))
        result = restore(active, artifact, expected_profile="SMS-NTSC", expected_version="other")
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)

    def test_failed_deadline_preserves_error_and_partial_effect_ledger(self):
        deadline = DeadlineRecord("d", 3, Fraction(9), DeadlineState.FAILED, ("effect-1",), ("effect-2",), "bus fault")
        state = machine(deadlines=(deadline,))
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.deadlines[0], deadline)

    def test_cancelled_old_generation_and_new_generation_are_both_preserved(self):
        records = (
            DeadlineRecord("old", 1, Fraction(5), DeadlineState.SUPERSEDED),
            DeadlineRecord("new", 2, Fraction(8), DeadlineState.SCHEDULED),
        )
        state = machine(deadlines=records)
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.deadlines, records)

    def test_replay_cursor_and_domain_residual_are_preserved(self):
        state = machine(input_cursor=123, domains=(DomainState("cpu", Fraction(11), Fraction(2, 7)),))
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.input_cursor, 123)
        self.assertEqual(restored.restored_state.domains, state.domains)

    def test_invalid_duplicate_deadline_identity_prevents_snapshot(self):
        deadline = DeadlineRecord("same", 0, Fraction(1), DeadlineState.SCHEDULED)
        state = machine(deadlines=(deadline, deadline))
        result = capture(state)
        self.assertEqual(result.code, SnapshotCode.INVALID_INPUT)
        self.assertIsNone(result.artifact)
        self.assertEqual(result.active_state, state)

    def test_failed_transaction_does_not_auto_recover_on_restore(self):
        tx = TransactionRecord(
            "tx", 0, TransactionState.FAILED, Fraction(5),
            committed_effects=("w1",), pending_effects=("w2",), error="fault",
        )
        state = machine(transactions=(tx,))
        restored = restore(machine(), capture(state).artifact, expected_profile=state.profile, expected_version=state.model_version)
        self.assertEqual(restored.restored_state.transactions[0].state, TransactionState.FAILED)
        self.assertEqual(restored.restored_state.transactions[0].committed_effects, ("w1",))

    def test_invalid_artifact_does_not_partially_replace_active_state(self):
        active = machine(watermark=Fraction(9))
        invalid_state = machine(deadlines=(DeadlineRecord("d", -1, Fraction(1), DeadlineState.SCHEDULED),))
        invalid_artifact = SnapshotArtifact("SMS-NTSC", "R1-0.3.18", invalid_state)
        result = restore(active, invalid_artifact, expected_profile="SMS-NTSC", expected_version="R1-0.3.18")
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
