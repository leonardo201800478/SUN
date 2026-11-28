"""P-E W09 envelope audit against R2 metadata; not a machine-state projection.

Only profile/model-version metadata is shared. The candidate identifier and
input cursor remain harness/candidate metadata; SyncState fields are not
coerced into R2 DomainState, DeadlineRecord, or InterruptRecord.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from r3_candidate_snapshots import SnapshotArtifact


class EnvelopeAuditCode(str, Enum):
    OK = "Ok"
    UNSUPPORTED = "Unsupported"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class PEEnvelopeAudit:
    profile: str
    model_version: str
    harness_input_cursor: int
    candidate_only: tuple[str, ...]
    not_mapped_to_r2: tuple[str, ...]


@dataclass(frozen=True)
class EnvelopeAuditResult:
    code: EnvelopeAuditCode
    audit: PEEnvelopeAudit | None = None
    detail: str = ""


def audit_pe_envelope(artifact: SnapshotArtifact) -> EnvelopeAuditResult:
    """Audit P-E envelope metadata without projecting SyncState into MachineState."""
    if not isinstance(artifact, SnapshotArtifact):
        return EnvelopeAuditResult(
            EnvelopeAuditCode.INVALID_INPUT, detail="expected candidate SnapshotArtifact"
        )
    if artifact.candidate != "P-E":
        return EnvelopeAuditResult(
            EnvelopeAuditCode.UNSUPPORTED,
            detail="envelope audit is scoped to P-E artifacts",
        )
    if (
        type(artifact.profile) is not str or not artifact.profile.strip()
        or type(artifact.model_version) is not str or not artifact.model_version.strip()
        or type(artifact.input_cursor) is not int or artifact.input_cursor < 0
    ):
        return EnvelopeAuditResult(
            EnvelopeAuditCode.INVALID_INPUT, detail="invalid P-E envelope metadata"
        )
    return EnvelopeAuditResult(
        EnvelopeAuditCode.OK,
        PEEnvelopeAudit(
            profile=artifact.profile,
            model_version=artifact.model_version,
            harness_input_cursor=artifact.input_cursor,
            candidate_only=("candidate=P-E", "input_cursor is harness metadata"),
            not_mapped_to_r2=(
                "SyncDomain.progress/safe_horizon (not R2 progress_common/residual)",
                "SyncTransition and dependency closure (not R2 deadlines)",
                "SyncConstraint and pairwise relations",
                "SyncState.observed and derived indexes",
                "P-E TransactionBook beyond separately demonstrated transaction fields",
                "R2 interrupts/INT-ack have no P-E representation",
            ),
        ),
    )
