"""Replayable W09 snapshot capture/restore fixture.

The fixture compares the P-A serializer/candidate with the independent R2
snapshot oracle over the same declarative source and active states. It is a
synthetic model-level fixture, not a production snapshot or hardware trace.
"""
from __future__ import annotations

import hashlib
import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_snapshots import (  # noqa: E402
    PAAckCommit,
    PAAckPresence,
    PADomainState,
    PAInterruptRecord,
    PAMachineState,
    PASnapshotArtifact,
    _artifact_state,
    _encode_state,
    capture as p_a_capture,
    restore as p_a_restore,
)
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)
from trace_fixture_support import normalize  # noqa: E402
from snapshot_reference import (  # noqa: E402
    AckCommit,
    AckPresence,
    DomainState,
    InterruptRecord,
    MachineState,
    SnapshotArtifact,
    capture as r2_capture,
    restore as r2_restore,
)


def _fraction(value):
    """Restore exact rational coordinates emitted by the replay harness."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _interrupt_data(interrupts):
    """Serialize the complete interrupt/acknowledge observation state."""
    return {
        "int_level": interrupts.int_level,
        "nmi_level": interrupts.nmi_level,
        "nmi_edge_pending": interrupts.nmi_edge_pending,
        "accepted": interrupts.accepted,
        "acknowledge_active": interrupts.acknowledge_active,
        "vector_sampled": interrupts.vector_sampled,
        "device_effect_presence": interrupts.device_effect_presence.value,
        "device_effect_commit": interrupts.device_effect_commit.value,
        "commit_at_common": interrupts.commit_at_common,
    }


def _state_data(state):
    """Serialize the complete state shape used by this snapshot fixture."""
    return {
        "profile": state.profile,
        "model_version": state.model_version,
        "domains": [
            {
                "name": item.name,
                "progress_common": item.progress_common,
                "residual": item.residual,
            }
            for item in state.domains
        ],
        "watermark": state.watermark,
        "deadlines": [],
        "transactions": [],
        "interrupts": _interrupt_data(state.interrupts),
        "input_cursor": state.input_cursor,
    }


def _p_a_state(data):
    """Reconstruct the P-A-owned immutable machine state."""
    irq = data["interrupts"]
    return PAMachineState(
        profile=data["profile"],
        model_version=data["model_version"],
        domains=tuple(
            PADomainState(
                item["name"],
                _fraction(item["progress_common"]),
                _fraction(item["residual"]),
            )
            for item in data["domains"]
        ),
        watermark=(
            None if data["watermark"] is None else _fraction(data["watermark"])
        ),
        deadlines=(),
        transactions=(),
        interrupts=PAInterruptRecord(
            int_level=irq["int_level"],
            nmi_level=irq["nmi_level"],
            nmi_edge_pending=irq["nmi_edge_pending"],
            accepted=irq["accepted"],
            acknowledge_active=irq["acknowledge_active"],
            vector_sampled=irq["vector_sampled"],
            device_effect_presence=PAAckPresence(irq["device_effect_presence"]),
            device_effect_commit=PAAckCommit(irq["device_effect_commit"]),
            commit_at_common=(
                None if irq["commit_at_common"] is None
                else _fraction(irq["commit_at_common"])
            ),
        ),
        input_cursor=data["input_cursor"],
    )


def _r2_state(data):
    """Reconstruct the independent R2-owned immutable machine state."""
    irq = data["interrupts"]
    return MachineState(
        profile=data["profile"],
        model_version=data["model_version"],
        domains=tuple(
            DomainState(
                item["name"],
                _fraction(item["progress_common"]),
                _fraction(item["residual"]),
            )
            for item in data["domains"]
        ),
        watermark=(
            None if data["watermark"] is None else _fraction(data["watermark"])
        ),
        deadlines=(),
        transactions=(),
        interrupts=InterruptRecord(
            int_level=irq["int_level"],
            nmi_level=irq["nmi_level"],
            nmi_edge_pending=irq["nmi_edge_pending"],
            accepted=irq["accepted"],
            acknowledge_active=irq["acknowledge_active"],
            vector_sampled=irq["vector_sampled"],
            device_effect_presence=AckPresence(irq["device_effect_presence"]),
            device_effect_commit=AckCommit(irq["device_effect_commit"]),
            commit_at_common=(
                None if irq["commit_at_common"] is None
                else _fraction(irq["commit_at_common"])
            ),
        ),
        input_cursor=data["input_cursor"],
    )


def _p_a_artifact_data(artifact):
    """Verify the P-A payload digest, then expose its neutral semantic state."""
    if artifact is None:
        return None
    state = _artifact_state(artifact)
    return {
        "profile": artifact.profile,
        "model_version": artifact.model_version,
        "state": _state_data(state),
    }


def _r2_artifact_data(artifact):
    """Expose the independent R2 artifact through the same semantic shape."""
    if artifact is None:
        return None
    return {
        "profile": artifact.profile,
        "model_version": artifact.model_version,
        "state": _state_data(artifact.state),
    }


def _p_a_artifact(data):
    """Rebuild and re-hash a P-A artifact from its captured semantic state."""
    state = _p_a_state(data["state"])
    payload = _encode_state(state)
    return PASnapshotArtifact(
        data["profile"], data["model_version"], payload,
        hashlib.sha256(payload).hexdigest(),
    )


def _r2_artifact(data):
    """Rebuild an R2 artifact using only R2 reference types."""
    return SnapshotArtifact(
        data["profile"], data["model_version"], _r2_state(data["state"])
    )


def _result_data(result, source_state, active_state, artifact_data):
    """Capture status, unchanged active state, artifact and restored state."""
    return {
        "code": result.code.value,
        "source_state": source_state,
        "active_state": active_state,
        "artifact": artifact_data,
        "restored_state": (
            _state_data(result.restored_state)
            if result.restored_state is not None else None
        ),
        "detail": result.detail,
    }


def _p_a_step(data, name, args):
    """Run capture/restore only through the P-A snapshot implementation."""
    source_state = data["source_state"]
    active_state = data["active_state"]
    if name == "capture":
        result = p_a_capture(_p_a_state(source_state))
        artifact_data = _p_a_artifact_data(result.artifact)
    elif name == "restore":
        artifact = _p_a_artifact(data["artifact"])
        result = p_a_restore(
            _p_a_state(active_state),
            artifact,
            expected_profile=args["expected_profile"],
            expected_version=args["expected_version"],
        )
        artifact_data = _p_a_artifact_data(result.artifact)
    else:
        raise ValueError(f"unsupported W09 operation: {name}")
    expected_active = _p_a_state(
        source_state if name == "capture" else active_state
    )
    if result.active_state != expected_active:
        raise AssertionError("P-A snapshot operation mutated its active input")
    return _result_data(result, source_state, active_state, artifact_data)


def _r2_step(data, name, args):
    """Run capture/restore only through the independent R2 snapshot oracle."""
    source_state = data["source_state"]
    active_state = data["active_state"]
    if name == "capture":
        result = r2_capture(_r2_state(source_state))
        artifact_data = _r2_artifact_data(result.artifact)
    elif name == "restore":
        artifact = _r2_artifact(data["artifact"])
        result = r2_restore(
            _r2_state(active_state),
            artifact,
            expected_profile=args["expected_profile"],
            expected_version=args["expected_version"],
        )
        artifact_data = _r2_artifact_data(result.artifact)
    else:
        raise ValueError(f"unsupported W09 operation: {name}")
    expected_active = _r2_state(
        source_state if name == "capture" else active_state
    )
    if result.active_state != expected_active:
        raise AssertionError("R2 snapshot operation mutated its active input")
    return _result_data(result, source_state, active_state, artifact_data)


class W09ReplayableSnapshotTests(unittest.TestCase):

    def test_w09_ack_vector_and_device_commit_ledger_restore_independently(self):
        source = PAMachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(
                PADomainState("cpu", Fraction(10), Fraction(1, 3)),
                PADomainState("vdp", Fraction(10), Fraction(2, 7)),
            ),
            watermark=Fraction(8),
            deadlines=(),
            transactions=(),
            interrupts=PAInterruptRecord(
                int_level=True,
                nmi_level=False,
                accepted="INT",
                acknowledge_active=True,
                vector_sampled="0x38",
                device_effect_presence=PAAckPresence.PRESENT,
                device_effect_commit=PAAckCommit.PENDING,
                commit_at_common=Fraction(12),
            ),
            input_cursor=7,
        )
        active = PAMachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(
                PADomainState("cpu", Fraction(11), Fraction(0)),
                PADomainState("vdp", Fraction(11), Fraction(0)),
            ),
            watermark=Fraction(9),
            deadlines=(),
            transactions=(),
            interrupts=PAInterruptRecord(),
            input_cursor=99,
        )
        initial = {
            "source_state": _state_data(source),
            "active_state": _state_data(active),
            "artifact": None,
            "restored_state": None,
        }
        operations = (
            {"sequence": 0, "name": "capture", "args": {}},
            {
                "sequence": 1,
                "name": "restore",
                "args": {
                    "expected_profile": "SMS-NTSC",
                    "expected_version": "R1-0.3.18",
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W09_ACK_VECTOR_AND_DEVICE_COMMIT_LEDGER_INDEPENDENCE",
            provenance={
                "gate": "R3",
                "workload": "W09",
                "classification": "OBSERVATION",
                "scenario": (
                    "snapshot preserves the sampled interrupt vector independently "
                    "from a still-pending device acknowledge commit and its coordinate"
                ),
                "scope": "synthetic P-A/R2 snapshot model comparison",
                "limitation": (
                    "Not evidence of physical SMS acknowledge timing, vector sampling "
                    "phase, or device-side commit boundary."
                ),
            },
            initial_state=initial,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        expected_active = normalize(initial["active_state"])
        expected_interrupts = {
            "int_level": True,
            "nmi_level": False,
            "nmi_edge_pending": False,
            "accepted": "INT",
            "acknowledge_active": True,
            "vector_sampled": "0x38",
            "device_effect_presence": "Present",
            "device_effect_commit": "Pending",
            "commit_at_common": {"$fraction": [12, 1]},
        }
        for side in ("left_state", "right_state"):
            final = fixture["cuts"][-1][side]
            self.assertEqual(final["code"], "Ok")
            self.assertEqual(final["active_state"], expected_active)
            self.assertEqual(
                final["restored_state"]["interrupts"], expected_interrupts
            )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )

    def test_w09_snapshot_capture_restore_preserves_exact_state(self):
        source = PAMachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(
                PADomainState("cpu", Fraction(10), Fraction(1, 3)),
                PADomainState("vdp", Fraction(10), Fraction(2, 7)),
            ),
            watermark=Fraction(8),
            deadlines=(),
            transactions=(),
            interrupts=PAInterruptRecord(),
            input_cursor=7,
        )
        active = PAMachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(
                PADomainState("cpu", Fraction(11), Fraction(0)),
                PADomainState("vdp", Fraction(11), Fraction(0)),
            ),
            watermark=Fraction(9),
            deadlines=(),
            transactions=(),
            interrupts=PAInterruptRecord(),
            input_cursor=99,
        )
        initial = {
            "source_state": _state_data(source),
            "active_state": _state_data(active),
            "artifact": None,
            "restored_state": None,
        }
        operations = (
            {"sequence": 0, "name": "capture", "args": {}},
            {
                "sequence": 1,
                "name": "restore",
                "args": {
                    "expected_profile": "SMS-NTSC",
                    "expected_version": "R1-0.3.18",
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W09_SNAPSHOT_CAPTURE_RESTORE_EXACT_STATE",
            provenance={
                "gate": "R3",
                "workload": "W09",
                "classification": "OBSERVATION",
                "scenario": "capture and restore an exact multi-domain state",
                "scope": "synthetic P-A/R2 snapshot model comparison",
                "limitation": (
                    "Not a production serializer, hardware snapshot, or "
                    "physical SMS state capture."
                ),
            },
            initial_state=initial,
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        final_left = fixture["cuts"][-1]["left_state"]
        final_right = fixture["cuts"][-1]["right_state"]
        self.assertEqual(final_left["code"], "Ok")
        self.assertEqual(final_right["code"], "Ok")
        expected_active = normalize(initial["active_state"])
        expected_source = normalize(initial["source_state"])
        self.assertEqual(final_left["active_state"], expected_active)
        self.assertEqual(final_right["active_state"], expected_active)
        self.assertEqual(final_left["restored_state"], expected_source)
        self.assertEqual(final_right["restored_state"], expected_source)
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
