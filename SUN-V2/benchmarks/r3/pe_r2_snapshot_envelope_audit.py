"""P-E W09 envelope audit against R2 metadata; not a machine-state projection.

Only profile/model-version metadata is shared. The candidate identifier and
input cursor remain candidate/harness metadata; SyncState fields are not
coerced into R2 DomainState, DeadlineRecord, or InterruptRecord.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from r3_candidate_snapshots import SnapshotArtifact as CandidateSnapshotArtifact
from snapshot_reference import SnapshotArtifact as R2SnapshotArtifact


class EnvelopeAuditCode(str, Enum):
    OK = "Ok"
    UNSUPPORTED = "Unsupported"
    INVALID_INPUT = "InvalidInput"
    MISMATCH = "MetadataMismatch"


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


@dataclass(frozen=True)
class PEEnvelopeComparison:
    profile_matches: bool
    candidate_model_version: str
    r2_model_version: str
    model_versions_comparable: bool
    harness_input_cursor: int
    cursor_comparable_to_r2: bool
    not_comparable: tuple[str, ...]


@dataclass(frozen=True)
class EnvelopeComparisonResult:
    code: EnvelopeAuditCode
    comparison: PEEnvelopeComparison | None = None
    detail: str = ""


def audit_pe_envelope(artifact: CandidateSnapshotArtifact) -> EnvelopeAuditResult:
    """Audit P-E envelope metadata without projecting SyncState into MachineState."""
    if not isinstance(artifact, CandidateSnapshotArtifact):
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


def compare_pe_r2_envelopes(
    candidate: CandidateSnapshotArtifact,
    reference: R2SnapshotArtifact,
) -> EnvelopeComparisonResult:
    """Compare only profile/version metadata; cursor and machine state are not projected."""
    audit = audit_pe_envelope(candidate)
    if audit.code is not EnvelopeAuditCode.OK:
        return EnvelopeComparisonResult(audit.code, detail=audit.detail)
    if not isinstance(reference, R2SnapshotArtifact):
        return EnvelopeComparisonResult(
            EnvelopeAuditCode.INVALID_INPUT, detail="expected R2 SnapshotArtifact"
        )
    if (
        type(reference.profile) is not str or not reference.profile.strip()
        or type(reference.model_version) is not str or not reference.model_version.strip()
        or reference.state.profile != reference.profile
        or reference.state.model_version != reference.model_version
    ):
        return EnvelopeComparisonResult(
            EnvelopeAuditCode.INVALID_INPUT, detail="invalid or inconsistent R2 envelope metadata"
        )
    profile_matches = candidate.profile == reference.profile
    comparison = PEEnvelopeComparison(
        profile_matches=profile_matches,
        candidate_model_version=candidate.model_version,
        r2_model_version=reference.model_version,
        model_versions_comparable=False,
        harness_input_cursor=candidate.input_cursor,
        cursor_comparable_to_r2=False,
        not_comparable=(
            "P-E and R2 model_version identify different model namespaces; equality is not semantic equivalence",
            "P-E input_cursor has no R2 accepted-input-log counterpart",
            "P-E SyncState is not projected into R2 MachineState",
            "P-E TransactionBook equivalence is not established by envelope metadata",
            "P-E has no R2 InterruptRecord/INT-ack representation",
        ),
    )
    code = EnvelopeAuditCode.OK if profile_matches else EnvelopeAuditCode.MISMATCH
    return EnvelopeComparisonResult(code, comparison)
