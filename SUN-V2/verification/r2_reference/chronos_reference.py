"""Independent CHRONOS R2 reference oracle.

This module is verification-only. It is deliberately immutable, exact, serial,
and slow. It is not a scheduler, runtime, production implementation, or V1
dependency.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction
from hashlib import sha256
from typing import Iterable


class ResultKind(str, Enum):
    REACHED = "Reached"
    STOPPED_AT = "StoppedAt"
    BLOCKED = "Blocked"
    FAILED = "Failed"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"
    INDETERMINATE_CAUSAL_ORDER = "IndeterminateCausalOrder"
    UNSUPPORTED_SNAPSHOT = "UnsupportedSnapshot"


class StopReason(str, Enum):
    SAFE_HORIZON = "SafeHorizon"
    PENDING_OBLIGATION = "PendingObligation"


@dataclass(frozen=True)
class ClockDomain:
    """One exact local clock domain mapped onto the common time line."""

    domain_id: str
    local_to_common: Fraction
    common_offset: Fraction = Fraction(0)
    progress_local: Fraction = Fraction(0)
    horizon_local: Fraction = Fraction(0)

    def progress_common(self) -> Fraction:
        """Convert local progress into exact common time."""
        return self.common_offset + self.progress_local * self.local_to_common

    def horizon_common(self) -> Fraction:
        """Convert the local safe horizon into exact common time."""
        return self.common_offset + self.horizon_local * self.local_to_common

    def local_at_common(self, common_time: Fraction) -> Fraction:
        """Invert the exact affine clock mapping without rounding."""
        return (common_time - self.common_offset) / self.local_to_common


@dataclass(frozen=True)
class Obligation:
    """A causal obligation known to the reference model."""

    obligation_id: str
    owner: str
    at_common: Fraction
    resolved: bool = False
    commutative: bool = False


@dataclass(frozen=True)
class ReferenceState:
    """Complete immutable logical state for the initial R2-A model."""

    profile: str
    domains: tuple[ClockDomain, ...]
    expected_participants: frozenset[str]
    watermark_common: Fraction | None = None
    obligations: tuple[Obligation, ...] = ()


@dataclass(frozen=True)
class AdvanceResult:
    kind: ResultKind
    requested_common: Fraction
    reached_common: Fraction
    reason: StopReason | None = None
    obligation_id: str | None = None
    owner: str | None = None
    detail: str | None = None


@dataclass(frozen=True)
class AdvanceEvaluation:
    state: ReferenceState
    result: AdvanceResult


def _is_exact_time(value: object) -> bool:
    """Accept only exact integer/rational values; floats are never time truth."""
    return isinstance(value, (int, Fraction)) and not isinstance(value, bool)


def _validate_state(state: ReferenceState) -> str | None:
    """Return a deterministic invariant/input error, or None when valid."""
    if not isinstance(state, ReferenceState):
        return "reference state has an invalid type"
    if not isinstance(state.profile, str) or not state.profile.strip():
        return "profile must be non-empty"
    if not isinstance(state.domains, tuple) or any(
        not isinstance(domain, ClockDomain) for domain in state.domains
    ):
        return "domain collection must contain only ClockDomain entries"
    if not isinstance(state.expected_participants, frozenset) or any(
        not isinstance(item, str) or not item.strip()
        for item in state.expected_participants
    ):
        return "expected participant collection must contain non-empty string identities"
    if not isinstance(state.obligations, tuple) or any(
        not isinstance(item, Obligation) for item in state.obligations
    ):
        return "obligation collection must contain only Obligation entries"
    # Validate the watermark before any incomplete-participant early return can
    # expose it as a blocked-progress coordinate.
    if state.watermark_common is not None and not _is_exact_time(state.watermark_common):
        return "watermark must use exact integer/rational time"
    ids = [domain.domain_id for domain in state.domains]
    if any(not isinstance(item, str) or not item.strip() for item in ids):
        return "domain identity must be a non-empty string"
    if len(ids) != len(set(ids)):
        return "duplicate domain identity"
    for domain in state.domains:
        if not all(
            _is_exact_time(value)
            for value in (
                domain.local_to_common,
                domain.common_offset,
                domain.progress_local,
                domain.horizon_local,
            )
        ):
            return f"non-exact time value in domain {domain.domain_id}"
        if domain.local_to_common <= 0:
            return f"invalid clock ratio for {domain.domain_id}"
        if domain.horizon_local < domain.progress_local:
            return f"safe horizon regressed behind progress for {domain.domain_id}"
    obligation_ids = [item.obligation_id for item in state.obligations]
    if any(not isinstance(item, str) or not item.strip() for item in obligation_ids):
        return "obligation identity must be non-empty"
    if any(
        not isinstance(item.owner, str) or not item.owner.strip()
        for item in state.obligations
    ):
        return "obligation owner must be non-empty"
    if any(
        not isinstance(item.resolved, bool) or not isinstance(item.commutative, bool)
        for item in state.obligations
    ):
        return "obligation flags must be boolean"
    if len(obligation_ids) != len(set(obligation_ids)):
        return "duplicate obligation identity"
    if any(not _is_exact_time(item.at_common) for item in state.obligations):
        return "obligation timestamp must use exact integer/rational time"
    actual = set(ids)
    expected = set(state.expected_participants)
    if actual - expected:
        return "unexpected participant is present"

    # With an incomplete set, use known participants only for a one-sided
    # invariant check: a known H below known P/W proves a violation. Never
    # expose this partial maximum as A(E) or as an advance frontier.
    frontier = _machine_frontier(state)
    if state.domains:
        safe_horizon = min(domain.horizon_common() for domain in state.domains)
        if safe_horizon < frontier:
            return "safe horizon is behind the confirmed machine frontier"
    # W is a closed global cut: no unresolved obligation may remain at or
    # before it. This is stronger than equality with a participant P_d, where
    # StoppedAt(P_d) is still a valid endpoint result.
    if state.watermark_common is not None and any(
        not item.resolved and item.at_common <= state.watermark_common
        for item in state.obligations
    ):
        return "unresolved obligation is at or behind the confirmed watermark"
    # A known participant already beyond an unresolved global obligation proves
    # an invariant violation even when the full participant set is incomplete.
    if any(
        not item.resolved and item.at_common < frontier
        for item in state.obligations
    ):
        return "unresolved obligation lies behind the confirmed machine frontier"
    if expected - actual:
        return "incomplete participant set"
    return None


def _state_at_common_time(
    state: ReferenceState, common_time: Fraction
) -> ReferenceState:
    """Return a new state with every participant advanced to one common cut."""
    updated = tuple(
        replace(domain, progress_local=domain.local_at_common(common_time))
        for domain in state.domains
    )
    return replace(state, domains=updated)


def _machine_frontier(state: ReferenceState) -> Fraction:
    """Return max(W, converted P_d) after participant validation."""
    progresses = [domain.progress_common() for domain in state.domains]
    if state.watermark_common is not None:
        progresses.append(state.watermark_common)
    return max(progresses, default=Fraction(0))


def advance(state: ReferenceState, target_common: Fraction) -> AdvanceEvaluation:
    """Evaluate one bounded advance using the R1 result taxonomy.

    The oracle scans all obligations and all participants. It does not use a
    priority queue or dispatch effects. A pending obligation is a stopping
    boundary; no operation after that boundary is applied.
    """
    if not _is_exact_time(target_common):
        known_watermark = (
            state.watermark_common
            if isinstance(state, ReferenceState)
            and _is_exact_time(state.watermark_common)
            else Fraction(0)
        )
        return AdvanceEvaluation(
            state,
            AdvanceResult(
                ResultKind.INVALID_INPUT,
                Fraction(0),
                known_watermark,
                detail="target must use exact integer/rational time; floats are forbidden",
            ),
        )

    invalid = _validate_state(state)
    if invalid is not None:
        if invalid == "incomplete participant set":
            return AdvanceEvaluation(
                state,
                AdvanceResult(
                    ResultKind.BLOCKED,
                    target_common,
                    (
                        state.watermark_common
                        if _is_exact_time(state.watermark_common)
                        else Fraction(0)
                    ),
                    detail="participant set incomplete; no partial frontier or A(E) computed",
                ),
            )
        kind = (
            ResultKind.INVARIANT_VIOLATION
            if (
                "regressed" in invalid
                or "behind the confirmed" in invalid
                or "confirmed watermark" in invalid
                or "safe horizon is behind" in invalid
            )
            else ResultKind.INVALID_INPUT
        )
        # Validation failed: never convert rejected domain state to build an error result.
        known_watermark = (
            state.watermark_common
            if isinstance(state, ReferenceState)
            and _is_exact_time(state.watermark_common)
            else Fraction(0)
        )
        return AdvanceEvaluation(
            state,
            AdvanceResult(kind, target_common, known_watermark, detail=invalid),
        )

    if not state.domains:
        return AdvanceEvaluation(
            state,
            AdvanceResult(
                ResultKind.BLOCKED,
                target_common,
                state.watermark_common or Fraction(0),
                detail="no temporal participants",
            ),
        )

    frontier = _machine_frontier(state)
    if target_common < frontier:
        return AdvanceEvaluation(
            state,
            AdvanceResult(
                ResultKind.LATE_INPUT,
                target_common,
                frontier,
                detail="requested target is earlier than the confirmed machine frontier",
            ),
        )

    safe_horizon = min(domain.horizon_common() for domain in state.domains)
    bounded_target = min(target_common, safe_horizon)

    # Full scan is intentional: this is a reference interpreter, not an agenda.
    pending = [
        obligation
        for obligation in state.obligations
        if not obligation.resolved and obligation.at_common <= bounded_target
    ]

    if pending:
        earliest = min(item.at_common for item in pending)
        simultaneous = [item for item in pending if item.at_common == earliest]
        if len(simultaneous) > 1 and not all(item.commutative for item in simultaneous):
            return AdvanceEvaluation(
                state,
                AdvanceResult(
                    ResultKind.INDETERMINATE_CAUSAL_ORDER,
                    target_common,
                    frontier,
                    detail="non-commutative obligations share an instant without a proven order",
                ),
            )
        first = min(simultaneous, key=lambda item: item.obligation_id)
        next_state = _state_at_common_time(state, earliest)
        return AdvanceEvaluation(
            next_state,
            AdvanceResult(
                ResultKind.STOPPED_AT,
                target_common,
                earliest,
                reason=StopReason.PENDING_OBLIGATION,
                obligation_id=first.obligation_id,
                owner=first.owner,
                detail="causal obligation remains unresolved at the stopping coordinate",
            ),
        )

    if target_common > safe_horizon:
        next_state = _state_at_common_time(state, safe_horizon)
        return AdvanceEvaluation(
            next_state,
            AdvanceResult(
                ResultKind.STOPPED_AT,
                target_common,
                safe_horizon,
                reason=StopReason.SAFE_HORIZON,
                detail="requested target exceeds the inclusive safe horizon",
            ),
        )

    next_state = _state_at_common_time(state, target_common)
    return AdvanceEvaluation(
        next_state,
        AdvanceResult(
            ResultKind.REACHED,
            target_common,
            target_common,
            detail="target reached and all modeled obligations through target are resolved",
        ),
    )


def resolve_obligation(
    state: ReferenceState, obligation_id: str
) -> ReferenceState:
    """Resolve one named obligation without changing virtual time."""
    found = False
    updated: list[Obligation] = []
    for item in state.obligations:
        if item.obligation_id == obligation_id:
            found = True
            updated.append(replace(item, resolved=True))
        else:
            updated.append(item)
    if not found:
        raise KeyError(obligation_id)
    return replace(state, obligations=tuple(updated))


def canonical_state(state: ReferenceState) -> tuple:
    """Return a stable, hashable representation for before/after assertions."""
    domains = tuple(
        (
            d.domain_id,
            d.local_to_common,
            d.common_offset,
            d.progress_local,
            d.horizon_local,
        )
        for d in sorted(state.domains, key=lambda item: item.domain_id)
    )
    obligations = tuple(
        (
            o.obligation_id,
            o.owner,
            o.at_common,
            o.resolved,
            o.commutative,
        )
        for o in sorted(state.obligations, key=lambda item: item.obligation_id)
    )
    return (
        state.profile,
        domains,
        tuple(sorted(state.expected_participants)),
        state.watermark_common,
        obligations,
    )


def state_digest(state: ReferenceState) -> str:
    """Hash the canonical logical state for deterministic reports."""
    return sha256(repr(canonical_state(state)).encode("utf-8")).hexdigest()


def make_state(
    *,
    profile: str = "SMS-NTSC",
    domains: Iterable[ClockDomain] | None = None,
    expected_participants: Iterable[str] | None = None,
    watermark_common: Fraction | None = None,
    obligations: Iterable[Obligation] = (),
) -> ReferenceState:
    """Create a state with an explicit participant set."""
    domain_tuple = tuple(domains or ())
    expected = (
        frozenset(expected_participants)
        if expected_participants is not None
        else frozenset(d.domain_id for d in domain_tuple)
    )
    return ReferenceState(
        profile=profile,
        domains=domain_tuple,
        expected_participants=expected,
        watermark_common=watermark_common,
        obligations=tuple(obligations),
    )
