"""P-A disposable instruction/budget plus catch-up prototype.

This module is intentionally separate from the R2 oracle and has no runtime
consumers. It uses a declared participant set and a full scan (no heap/agenda).
The implementation models abstract common-time work only; it is not an SMS core.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from fractions import Fraction
from typing import Iterable


@dataclass(frozen=True)
class AParticipant:
    """Participant with exact progress, safe horizon and a rational work rate."""

    name: str
    progress: Fraction
    safe_horizon: Fraction
    units_per_common: Fraction = Fraction(1)

    def __post_init__(self) -> None:
        if not self.name or not self.name.strip():
            raise ValueError("participant name must be non-empty")
        if self.units_per_common <= 0:
            raise ValueError("units_per_common must be positive")
        if self.safe_horizon < self.progress:
            raise ValueError("safe horizon cannot precede progress")


@dataclass(frozen=True)
class AObligation:
    """Pending causal boundary for an owning participant."""

    identity: str
    owner: str
    at_common: Fraction
    resolved: bool = False
    commutative: bool = False

    def __post_init__(self) -> None:
        if not self.identity or not self.identity.strip():
            raise ValueError("obligation identity must be non-empty")
        if not self.owner or not self.owner.strip():
            raise ValueError("obligation owner must be non-empty")


@dataclass(frozen=True)
class AState:
    """Immutable P-A state: participants and a full-scanned obligation tuple."""

    participants: tuple[AParticipant, ...]
    obligations: tuple[AObligation, ...] = ()


@dataclass(frozen=True)
class AResult:
    """Outcome of a bounded budget/catch-up operation."""

    code: str
    requested: Fraction
    reached: Fraction
    reason: str
    touched_participants: int
    participants_inspected: int
    obligations_inspected: int
    chunks: int
    next_state: AState


def _common_frontier(state: AState) -> Fraction:
    if not state.participants:
        return Fraction(0)
    return min(item.progress for item in state.participants)


def _validate(state: AState) -> None:
    if not isinstance(state, AState):
        raise ValueError("state must be AState")
    names = [item.name for item in state.participants]
    if len(names) != len(set(names)):
        raise ValueError("participant identities must be unique")
    if any(item.owner not in set(names) for item in state.obligations):
        raise ValueError("obligation owner must be a declared participant")
    ids = [item.identity for item in state.obligations]
    if len(ids) != len(set(ids)):
        raise ValueError("obligation identities must be unique")


def advance_budgeted(
    state: AState,
    *,
    target: Fraction,
    chunk: Fraction,
) -> AResult:
    """Advance all participants in bounded chunks, with catch-up and full scans.

    The requested target is bounded by the minimum participant safe horizon.
    The baseline inspects every obligation on each chunk; it has no agenda.
    Simultaneous pending non-commutative obligations without an explicit order
    return IndeterminateCausalOrder without advancing any participant.
    """
    _validate(state)
    if not isinstance(target, (int, Fraction)) or isinstance(target, bool):
        raise ValueError("target must be an exact rational coordinate")
    if not isinstance(chunk, (int, Fraction)) or isinstance(chunk, bool) or chunk <= 0:
        raise ValueError("chunk must be a positive exact rational coordinate")
    if not state.participants:
        return AResult(
            "Blocked", Fraction(target), Fraction(0), "NoParticipants",
            0, 0, 0, 0, state,
        )
    frontier = _common_frontier(state)
    if target < frontier:
        return AResult(
            "LateInput", Fraction(target), frontier, "TargetBeforeFrontier",
            0, 0, 0, 0, state,
        )
    safe_horizon = min(item.safe_horizon for item in state.participants)
    bounded = min(Fraction(target), safe_horizon)
    pending = [
        item for item in state.obligations
        if not item.resolved and item.at_common <= bounded
    ]
    if pending:
        earliest = min(item.at_common for item in pending)
        tied = [item for item in pending if item.at_common == earliest]
        if len(tied) > 1 and not all(item.commutative for item in tied):
            return AResult(
                "IndeterminateCausalOrder", Fraction(target), frontier,
                "UnknownSimultaneousObligations", 0, len(state.participants),
                len(state.obligations), 0, state,
            )
        first = min(tied, key=lambda item: item.identity)
        bounded = min(bounded, earliest)
        stop_reason = f"PendingObligation:{first.identity}"
    elif Fraction(target) > safe_horizon:
        stop_reason = "SafeHorizon"
    else:
        stop_reason = "TargetReached"

    chunks = 0
    updated = list(state.participants)
    touched_ids: set[str] = set()
    inspected_participants = 0
    inspected_obligations = 0
    current = frontier
    while current < bounded:
        next_common = min(current + Fraction(chunk), bounded)
        # P-A performs a full scan at every budget boundary; there is no agenda.
        inspected_obligations += len(state.obligations)
        inspected_participants += len(updated)
        next_participants: list[AParticipant] = []
        for participant in updated:
            next_progress = max(
                participant.progress,
                min(next_common, participant.safe_horizon),
            )
            if next_progress != participant.progress:
                touched_ids.add(participant.name)
            next_participants.append(replace(participant, progress=next_progress))
        updated = next_participants
        chunks += 1
        current = next_common
    next_state = replace(state, participants=tuple(updated))
    reached = min(item.progress for item in next_state.participants)
    return AResult(
        "StoppedAt" if stop_reason != "TargetReached" else "Reached",
        Fraction(target), reached, stop_reason, len(touched_ids),
        inspected_participants, inspected_obligations, chunks, next_state,
    )

def state_digest(state: AState) -> tuple:
    """Canonical pure-Python digest input for exact state equivalence checks."""
    _validate(state)
    return (
        tuple(sorted(
            (p.name, p.progress.numerator, p.progress.denominator,
             p.safe_horizon.numerator, p.safe_horizon.denominator)
            for p in state.participants
        )),
        tuple(sorted(
            (o.identity, o.owner, o.at_common.numerator, o.at_common.denominator,
             o.resolved, o.commutative)
            for o in state.obligations
        )),
    )
