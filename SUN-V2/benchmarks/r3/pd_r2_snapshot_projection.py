"""Explicit partial P-D -> R2-T11 snapshot projection for W09.

This is a typed comparison adapter, not a machine-state conversion. It maps
only watermark and deadline identity/generation/time/lifecycle fields whose
meaning is shared. Candidate-only fields are listed as unrepresented.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from p_d_discrete_agenda import AgendaState, EventStatus, _validate as validate_pd
from snapshot_reference import DeadlineRecord, DeadlineState


class ProjectionCode(str, Enum):
    OK = "Ok"
    UNSUPPORTED = "UnsupportedProjection"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class R2SharedProjection:
    watermark: object
    deadlines: tuple[DeadlineRecord, ...]
    unrepresented: tuple[str, ...]


@dataclass(frozen=True)
class ProjectionResult:
    code: ProjectionCode
    projection: R2SharedProjection | None = None
    detail: str = ""


_STATUS_MAP = {
    EventStatus.SCHEDULED: DeadlineState.SCHEDULED,
    EventStatus.CANCELLED: DeadlineState.CANCELLED,
    EventStatus.SUPERSEDED: DeadlineState.SUPERSEDED,
    EventStatus.CLAIMED: DeadlineState.CLAIMED,
    EventStatus.COMPLETED: DeadlineState.COMPLETED,
}


def project_pd_shared_state(state: AgendaState) -> ProjectionResult:
    """Project only P-D fields with demonstrated R2-T11 correspondence.

    Event effect IDs, constraints, heap ordering, pending cancellation records,
    phase metadata, and the W08 TransactionBook are deliberately not coerced
    into R2 fields. CancelPending has no R2 DeadlineState counterpart and
    therefore makes this projection unsupported rather than guessed.
    """
    if not isinstance(state, AgendaState):
        return ProjectionResult(ProjectionCode.INVALID_INPUT, detail="expected AgendaState")
    invalid = validate_pd(state)
    if invalid is not None:
        return ProjectionResult(ProjectionCode.INVALID_INPUT, detail=invalid)

    records = []
    for event in sorted(state.events, key=lambda item: (item.identity, item.generation)):
        mapped = _STATUS_MAP.get(event.status)
        if mapped is None:
            return ProjectionResult(
                ProjectionCode.UNSUPPORTED,
                detail=f"P-D status {event.status.value} has no R2 deadline-state counterpart",
            )
        records.append(DeadlineRecord(
            identity=event.identity,
            generation=event.generation,
            at_common=event.at_common,
            state=mapped,
        ))

    return ProjectionResult(
        ProjectionCode.OK,
        R2SharedProjection(
            watermark=state.watermark_common,
            deadlines=tuple(records),
            unrepresented=(
                "AgendaEvent.effect_id and phase metadata",
                "AgendaState.heap and constraints",
                "AgendaState.pending_cancellations",
                "AgendaState.transactions (W08 TransactionBook)",
                "R2 domains, interrupts, and input_cursor (absent from P-D)",
                "R2 deadline committed/pending effect ledgers (not mapped)",
            ),
        ),
    )
