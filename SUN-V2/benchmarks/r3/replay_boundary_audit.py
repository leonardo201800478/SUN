"""W09 audit of the semantic boundary between candidate operation replay and R2 input replay.

This module deliberately refuses to equate the candidate-local operation suffix
with R2's accepted external-input log. It is an audit report, not a replay adapter.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from r3_candidate_snapshots import ReplayEvent, SnapshotArtifact
from replay_reference import ReplayState, _validate_state


class ReplayBoundaryCode(str, Enum):
    INCOMPARABLE = "IncomparableReplayModels"
    PROFILE_MISMATCH = "ProfileMismatch"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class ReplayBoundaryAudit:
    candidate: str
    candidate_profile: str
    r2_profile: str
    profile_matches: bool
    candidate_harness_cursor: int
    candidate_suffix_sequences: tuple[int, ...]
    r2_accepted_sequences: tuple[int, ...]
    sequence_spaces_comparable: bool
    not_comparable: tuple[str, ...]


@dataclass(frozen=True)
class ReplayBoundaryResult:
    code: ReplayBoundaryCode
    audit: ReplayBoundaryAudit | None = None
    detail: str = ""


def audit_candidate_replay_boundary(
    artifact: SnapshotArtifact,
    suffix: tuple[ReplayEvent, ...],
    r2_state: ReplayState,
) -> ReplayBoundaryResult:
    """Inventory both replay ledgers without coercing either into the other."""
    if (
        not isinstance(artifact, SnapshotArtifact)
        or type(artifact.candidate) is not str
        or artifact.candidate not in ("P-D", "P-E")
        or type(artifact.profile) is not str or not artifact.profile.strip()
        or type(artifact.input_cursor) is not int or artifact.input_cursor < 0
        or type(suffix) is not tuple
    ):
        return ReplayBoundaryResult(
            ReplayBoundaryCode.INVALID_INPUT, detail="invalid candidate snapshot/replay envelope"
        )
    if not isinstance(r2_state, ReplayState):
        return ReplayBoundaryResult(
            ReplayBoundaryCode.INVALID_INPUT, detail="expected R2 ReplayState"
        )
    invalid = _validate_state(r2_state)
    if invalid is not None:
        return ReplayBoundaryResult(ReplayBoundaryCode.INVALID_INPUT, detail=invalid)

    sequences = []
    for event in suffix:
        if (
            not isinstance(event, ReplayEvent)
            or type(event.sequence) is not int or event.sequence < 1
            or type(event.operation) is not str or not event.operation.strip()
            or type(event.arguments) is not bytes
        ):
            return ReplayBoundaryResult(
                ReplayBoundaryCode.INVALID_INPUT, detail="invalid candidate replay operation"
            )
        sequences.append(event.sequence)
    expected = list(range(artifact.input_cursor + 1,
                           artifact.input_cursor + 1 + len(sequences)))
    if sequences != expected:
        return ReplayBoundaryResult(
            ReplayBoundaryCode.INVALID_INPUT,
            detail="candidate operation suffix is not contiguous after its harness cursor",
        )

    accepted_sequences = tuple(item.sequence for item in r2_state.accepted)
    profile_matches = artifact.profile == r2_state.profile
    audit = ReplayBoundaryAudit(
        candidate=artifact.candidate,
        candidate_profile=artifact.profile,
        r2_profile=r2_state.profile,
        profile_matches=profile_matches,
        candidate_harness_cursor=artifact.input_cursor,
        candidate_suffix_sequences=tuple(sequences),
        r2_accepted_sequences=accepted_sequences,
        sequence_spaces_comparable=False,
        not_comparable=(
            "candidate input_cursor is harness operation metadata, not R2 accepted-input sequence state",
            "candidate ReplayEvent.operation/arguments are not R2 ExternalInput identity/class/owner/time/payload/participants",
            "R2 accepted input log and applied_ids have no candidate operation-suffix counterpart",
            "R2 pending_obligations and recalculate_required are not represented by candidate replay metadata",
            "R2 virtual_effects are not derivable from candidate operation sequence numbers",
            "matching profile does not establish replay semantic equivalence",
        ),
    )
    return ReplayBoundaryResult(
        ReplayBoundaryCode.INCOMPARABLE if profile_matches
        else ReplayBoundaryCode.PROFILE_MISMATCH,
        audit=audit,
    )
