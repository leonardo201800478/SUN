"""P-A W09 snapshot/restore/replay differential and property tests."""
from __future__ import annotations

import hashlib
import sys
import unittest
from dataclasses import replace
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_snapshots import (
    PAAckCommit, PAAckPresence, PADeadlineRecord, PADeadlineState,
    PADomainState, PAInterruptRecord, PAMachineState, PAReplayEvent,
    PASnapshotArtifact, PASnapshotCode, PATransactionRecord,
    PATransactionState, _encode_state, capture, replay_from_snapshot, restore,
)
from trace_fixture_support import record_differential, record_property
from snapshot_reference import (
    AckCommit as R2AckCommit, AckPresence as R2AckPresence,
    DeadlineRecord as R2DeadlineRecord, DeadlineState as R2DeadlineState,
    DomainState as R2DomainState, InterruptRecord as R2InterruptRecord,
    MachineState as R2MachineState, SnapshotArtifact as R2Artifact,
    SnapshotCode as R2Code, TransactionRecord as R2TransactionRecord,
    TransactionState as R2TransactionState,
    capture as r2_capture, restore as r2_restore,
)


def machine(**changes):
    values = dict(
        profile="SMS-NTSC",
        model_version="R1-0.3.18",
        domains=(PADomainState("cpu", Fraction(10), Fraction(1, 3)),),
        watermark=Fraction(8),
        deadlines=(),
        transactions=(),
        interrupts=PAInterruptRecord(),
        input_cursor=7,
    )
    values.update(changes)
    return PAMachineState(**values)


def to_r2(state: PAMachineState) -> R2MachineState:
    return R2MachineState(
        state.profile,
        state.model_version,
        tuple(R2DomainState(item.name, item.progress_common, item.residual) for item in state.domains),
        state.watermark,
        tuple(
            R2DeadlineRecord(
                item.identity, item.generation, item.at_common,
                R2DeadlineState(item.state.value), item.committed_effects,
                item.pending_effects, item.error,
            )
            for item in state.deadlines
        ),
        tuple(
            R2TransactionRecord(
                item.identity, item.generation, R2TransactionState(item.state.value),
                item.progress_common, item.next_marker, item.wait_source,
                item.sampled, item.committed_effects, item.pending_effects, item.error,
            )
            for item in state.transactions
        ),
        R2InterruptRecord(
            state.interrupts.int_level, state.interrupts.nmi_level,
            state.interrupts.nmi_edge_pending, state.interrupts.accepted,
            state.interrupts.acknowledge_active, state.interrupts.vector_sampled,
            R2AckPresence(state.interrupts.device_effect_presence.value),
            R2AckCommit(state.interrupts.device_effect_commit.value),
            state.interrupts.commit_at_common,
        ),
        state.input_cursor,
    )


class PASnapshotTests(unittest.TestCase):
    def compare_capture(self, state):
        actual = capture(state)
        expected = r2_capture(to_r2(state))
        self.assertEqual(actual.code.value, expected.code.value)
        self.assertEqual(actual.active_state, state)
        if actual.code is PASnapshotCode.OK:
            active = machine(watermark=Fraction(9))
            restored = restore(
                active, actual.artifact,
                expected_profile=state.profile, expected_version=state.model_version,
            )
            restored_ref = r2_restore(
                to_r2(active), expected.artifact,
                expected_profile=state.profile, expected_version=state.model_version,
            )
            self.assertEqual(restored.code.value, restored_ref.code.value)
            self.assertEqual(to_r2(restored.restored_state), restored_ref.restored_state)
            record_differential(
                self.id(),
                (actual.code.value, to_r2(restored.restored_state)),
                (expected.code.value, restored_ref.restored_state),
            )
        else:
            self.assertIsNone(actual.artifact)
            record_differential(
                self.id(),
                (actual.code.value, to_r2(state)),
                (expected.code.value, to_r2(state)),
            )
        return actual, expected

    def test_every_deadline_lifecycle_state_round_trips(self):
        for status in PADeadlineState:
            committed = (
                ("e1",) if status in (
                    PADeadlineState.DISPATCHED, PADeadlineState.COMPLETED,
                    PADeadlineState.FAILED,
                ) else ()
            )
            pending = (
                ("e2",) if status in (
                    PADeadlineState.DISPATCHED, PADeadlineState.FAILED,
                ) else ()
            )
            error = "fault" if status is PADeadlineState.FAILED else None
            deadline = PADeadlineRecord(
                "d", 1, Fraction(9), status, committed, pending, error
            )
            self.compare_capture(machine(deadlines=(deadline,)))

    def test_every_transaction_control_state_round_trips(self):
        for status in PATransactionState:
            error = "fault" if status is PATransactionState.FAILED else None
            pending = () if status is PATransactionState.COMPLETED else ("w2",)
            tx = PATransactionRecord(
                "tx", 2, status, Fraction(5), next_marker="commit",
                wait_source="fixture" if status is PATransactionState.WAITING else None,
                sampled=(("read", "0x44"),), committed_effects=("w1",),
                pending_effects=pending, error=error,
            )
            self.compare_capture(machine(transactions=(tx,)))

    def test_snapshot_between_sample_and_commit_preserves_sample_and_pending_effect(self):
        tx = PATransactionRecord(
            "tx", 0, PATransactionState.IN_PROGRESS, Fraction(3),
            next_marker="commit", sampled=(("read-status", "0x81"),),
            pending_effects=("write-port",),
        )
        actual, _ = self.compare_capture(machine(transactions=(tx,)))
        restored = restore(
            machine(), actual.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.restored_state.transactions[0].sampled, tx.sampled)
        self.assertEqual(restored.restored_state.transactions[0].pending_effects, tx.pending_effects)

    def test_snapshot_after_commit_preserves_effect_without_reapplication(self):
        tx = PATransactionRecord(
            "tx", 0, PATransactionState.IN_PROGRESS, Fraction(4),
            committed_effects=("write-port",), pending_effects=("publish-irq",),
        )
        actual, _ = self.compare_capture(machine(transactions=(tx,)))
        restored = restore(
            machine(), actual.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.restored_state.transactions[0].committed_effects, ("write-port",))

    def test_ack_vector_and_device_commit_ledger_are_independent(self):
        irq = PAInterruptRecord(
            int_level=True, accepted="INT", acknowledge_active=True,
            vector_sampled="0x38", device_effect_presence=PAAckPresence.PRESENT,
            device_effect_commit=PAAckCommit.PENDING, commit_at_common=Fraction(12),
        )
        actual, _ = self.compare_capture(machine(interrupts=irq))
        restored = restore(
            machine(), actual.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.restored_state.interrupts.vector_sampled, "0x38")
        self.assertEqual(restored.restored_state.interrupts.device_effect_commit, PAAckCommit.PENDING)

    def test_unknown_ack_effect_presence_is_unsupported_without_artifact(self):
        irq = PAInterruptRecord(
            accepted="INT", acknowledge_active=True,
            device_effect_presence=PAAckPresence.UNKNOWN,
            device_effect_commit=PAAckCommit.UNKNOWN,
        )
        result, _ = self.compare_capture(machine(interrupts=irq))
        self.assertEqual(result.code, PASnapshotCode.UNSUPPORTED)
        self.assertIsNone(result.artifact)

    def test_unknown_ack_commit_boundary_is_unsupported_without_artifact(self):
        irq = PAInterruptRecord(
            accepted="INT", acknowledge_active=True,
            device_effect_presence=PAAckPresence.PRESENT,
            device_effect_commit=PAAckCommit.PENDING, commit_at_common=None,
        )
        result, _ = self.compare_capture(machine(interrupts=irq))
        self.assertEqual(result.code, PASnapshotCode.UNSUPPORTED)
        self.assertIsNone(result.artifact)

    def test_unknown_ack_profile_outside_ack_window_does_not_block_snapshot(self):
        irq = PAInterruptRecord(
            device_effect_presence=PAAckPresence.UNKNOWN,
            device_effect_commit=PAAckCommit.UNKNOWN,
        )
        self.assertEqual(self.compare_capture(machine(interrupts=irq))[0].code, PASnapshotCode.OK)

    def test_known_profile_with_no_ack_device_effect_is_supported(self):
        irq = PAInterruptRecord(
            accepted="INT", acknowledge_active=True, vector_sampled="0x38"
        )
        self.assertEqual(self.compare_capture(machine(interrupts=irq))[0].code, PASnapshotCode.OK)

    def test_wrong_profile_restore_is_atomic(self):
        source = machine()
        artifact = capture(source).artifact
        active = machine(watermark=Fraction(9))
        result = restore(
            active, artifact, expected_profile="SG-1000",
            expected_version="R1-0.3.18",
        )
        expected = r2_restore(
            to_r2(active), r2_capture(to_r2(source)).artifact,
            expected_profile="SG-1000", expected_version="R1-0.3.18",
        )
        self.assertEqual(result.code.value, expected.code.value)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_wrong_model_version_restore_is_atomic(self):
        source = machine()
        artifact = capture(source).artifact
        active = machine(watermark=Fraction(9))
        result = restore(
            active, artifact, expected_profile="SMS-NTSC",
            expected_version="other",
        )
        expected = r2_restore(
            to_r2(active), r2_capture(to_r2(source)).artifact,
            expected_profile="SMS-NTSC", expected_version="other",
        )
        self.assertEqual(result.code.value, expected.code.value)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_failed_deadline_preserves_error_and_partial_effect_ledger(self):
        deadline = PADeadlineRecord(
            "d", 3, Fraction(9), PADeadlineState.FAILED,
            ("effect-1",), ("effect-2",), "bus fault",
        )
        restored = self.compare_capture(machine(deadlines=(deadline,)))[0]
        state = restore(
            machine(), restored.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        ).restored_state
        self.assertEqual(state.deadlines[0], deadline)

    def test_cancelled_old_generation_and_new_generation_are_preserved(self):
        records = (
            PADeadlineRecord("old", 1, Fraction(5), PADeadlineState.SUPERSEDED),
            PADeadlineRecord("new", 2, Fraction(8), PADeadlineState.SCHEDULED),
        )
        actual = self.compare_capture(machine(deadlines=records))[0]
        restored = restore(
            machine(), actual.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.restored_state.deadlines, records)

    def test_replay_cursor_and_domain_residual_are_preserved(self):
        state = machine(
            input_cursor=123,
            domains=(PADomainState("cpu", Fraction(11), Fraction(2, 7)),),
        )
        actual = self.compare_capture(state)[0]
        restored = restore(
            machine(), actual.artifact,
            expected_profile=state.profile, expected_version=state.model_version,
        )
        self.assertEqual(restored.restored_state.input_cursor, 123)
        self.assertEqual(restored.restored_state.domains, state.domains)

    def test_invalid_duplicate_deadline_identity_prevents_snapshot(self):
        deadline = PADeadlineRecord("same", 0, Fraction(1), PADeadlineState.SCHEDULED)
        state = machine(deadlines=(deadline, deadline))
        result, expected = self.compare_capture(state)
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.code.value, expected.code.value)
        self.assertIsNone(result.artifact)

    def test_failed_transaction_does_not_auto_recover_on_restore(self):
        tx = PATransactionRecord(
            "tx", 0, PATransactionState.FAILED, Fraction(5),
            committed_effects=("w1",), pending_effects=("w2",), error="fault",
        )
        actual = self.compare_capture(machine(transactions=(tx,)))[0]
        restored = restore(
            machine(), actual.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.restored_state.transactions[0].state, PATransactionState.FAILED)
        self.assertEqual(restored.restored_state.transactions[0].committed_effects, ("w1",))

    def test_invalid_artifact_does_not_partially_replace_active_state(self):
        active = machine(watermark=Fraction(9))
        invalid_state = machine(
            deadlines=(PADeadlineRecord(
                "d", -1, Fraction(1), PADeadlineState.SCHEDULED
            ),)
        )
        payload = _encode_state(invalid_state)
        artifact = PASnapshotArtifact(
            invalid_state.profile, invalid_state.model_version,
            payload, hashlib.sha256(payload).hexdigest(),
        )
        result = restore(
            active, artifact, expected_profile="SMS-NTSC",
            expected_version="R1-0.3.18",
        )
        expected = r2_restore(
            to_r2(active),
            R2Artifact("SMS-NTSC", "R1-0.3.18", to_r2(invalid_state)),
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(result.code.value, expected.code.value)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_restore_rejects_forged_unsupported_ack_artifact_atomically(self):
        active = machine(watermark=Fraction(9))
        ambiguous = machine(interrupts=PAInterruptRecord(
            accepted="INT", acknowledge_active=True,
            device_effect_presence=PAAckPresence.UNKNOWN,
            device_effect_commit=PAAckCommit.UNKNOWN,
        ))
        payload = _encode_state(ambiguous)
        artifact = PASnapshotArtifact(
            ambiguous.profile, ambiguous.model_version,
            payload, hashlib.sha256(payload).hexdigest(),
        )
        result = restore(
            active, artifact, expected_profile="SMS-NTSC",
            expected_version="R1-0.3.18",
        )
        self.assertEqual(result.code, PASnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_canonical_snapshot_digest_is_repeatable(self):
        state = machine()
        first = capture(state)
        second = capture(state)
        self.assertEqual(first.artifact.payload, second.artifact.payload)
        self.assertEqual(first.artifact.digest, second.artifact.digest)

    def test_payload_tampering_is_rejected_atomically(self):
        source = machine()
        artifact = capture(source).artifact
        tampered = replace(artifact, payload=artifact.payload + b" ")
        active = machine(watermark=Fraction(9))
        result = restore(
            active, tampered, expected_profile="SMS-NTSC",
            expected_version="R1-0.3.18",
        )
        self.assertEqual(result.code, PASnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def replay_fixture(self):
        state = machine(domains=(
            PADomainState("cpu", Fraction(10), Fraction(1, 3)),
            PADomainState("vdp", Fraction(10), Fraction(2, 7)),
        ))
        artifact = capture(state).artifact
        events = (
            PAReplayEvent(8, Fraction(9), "cpu", Fraction(11), Fraction(1, 5)),
            PAReplayEvent(9, Fraction(9), "vdp", Fraction(12), Fraction(1, 4)),
            PAReplayEvent(10, Fraction(10), "cpu", Fraction(13), Fraction(2, 9)),
        )
        return state, artifact, events

    def test_replay_repeated_run_has_identical_state_and_digest(self):
        state, artifact, events = self.replay_fixture()
        first = replay_from_snapshot(artifact, events)
        second = replay_from_snapshot(artifact, events)
        self.assertEqual(first.code, PASnapshotCode.OK)
        self.assertEqual(first.state, second.state)
        self.assertEqual(first.digest, second.digest)
        self.assertEqual(first.state.input_cursor, 10)
        record_property(self.id(), "repeat", first.digest, second.digest)

    def test_replay_arrival_permutation_is_canonical_by_sequence(self):
        state, artifact, events = self.replay_fixture()
        forward = replay_from_snapshot(artifact, events)
        reverse = replay_from_snapshot(artifact, tuple(reversed(events)))
        self.assertEqual(forward.code, PASnapshotCode.OK)
        self.assertEqual(reverse.code, PASnapshotCode.OK)
        self.assertEqual(forward.state, reverse.state)
        self.assertEqual(forward.digest, reverse.digest)
        record_property(self.id(), "arrival-permutation", forward.digest, reverse.digest)

    def test_replay_sequence_gap_is_rejected_without_partial_apply(self):
        artifact, events = self.replay_fixture()
        result = replay_from_snapshot(artifact, (events[1],))
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.state.input_cursor, 7)
        self.assertEqual(result.state.domains, state.domains)
        record_property(self.id(), "atomic-gap", result.state, state)

    def test_replay_duplicate_sequence_is_rejected(self):
        artifact, events = self.replay_fixture()
        result = replay_from_snapshot(artifact, (events[0], events[0]))
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.state.input_cursor, 7)

    def test_replay_progress_regression_is_rejected_atomically(self):
        artifact, _ = self.replay_fixture()
        event = PAReplayEvent(8, Fraction(9), "cpu", Fraction(9), Fraction(1, 5))
        result = replay_from_snapshot(artifact, (event,))
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.state.input_cursor, 7)

    def test_replay_unknown_domain_is_rejected_atomically(self):
        artifact, _ = self.replay_fixture()
        event = PAReplayEvent(8, Fraction(9), "ym2413", Fraction(11), Fraction(1, 5))
        result = replay_from_snapshot(artifact, (event,))
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.state.input_cursor, 7)

    def test_replay_time_regression_is_rejected_atomically(self):
        artifact, events = self.replay_fixture()
        regressed = (
            events[0],
            PAReplayEvent(9, Fraction(8), "vdp", Fraction(12), Fraction(1, 4)),
        )
        result = replay_from_snapshot(artifact, regressed)
        self.assertEqual(result.code, PASnapshotCode.INVALID_INPUT)
        self.assertEqual(result.state.input_cursor, 7)
        record_property(self.id(), "atomic-time-regression", result.state, state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
