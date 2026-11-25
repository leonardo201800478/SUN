"""P-A disposable finite same-instant microstep prototype.

This candidate owns its immutable state types and transition logic. It models
logical obligations only; it does not assign physical chip phases or implement
a production dispatcher.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from fractions import Fraction


class AStepCode(str, Enum):
    ADVANCED = "Advanced"
    BLOCKED = "Blocked"
    COMPLETE = "Complete"
    INDETERMINATE = "IndeterminateCausalOrder"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class AStepObligation:
    """Named obligation pending at one logical microstep."""

    identity: str
    owner: str
    microstep: int = 0
    commutative: bool = False


@dataclass(frozen=True)
class AStepCycle:
    """Immutable same-instant cycle with a finite logical microstep budget."""

    at_common: Fraction
    max_microsteps: int
    next_microstep: int
    pending: tuple[AStepObligation, ...]
    resolved: tuple[str, ...] = ()


@dataclass(frozen=True)
class AStepResult:
    """Result of one microstep operation and its immutable successor state."""

    state: AStepCycle
    code: AStepCode
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept integer/rational time while excluding booleans and floats."""
    return isinstance(value, (int, Fraction)) and not isinstance(value, bool)


def _validate(state: object) -> str | None:
    """Validate all state fields before dereferencing or processing obligations."""
    if not isinstance(state, AStepCycle):
        return "microstep cycle has an invalid type"
    if not _exact(state.at_common):
        return "microstep instant must be exact"
    if (
        not isinstance(state.max_microsteps, int)
        or isinstance(state.max_microsteps, bool)
        or state.max_microsteps <= 0
        or not isinstance(state.next_microstep, int)
        or isinstance(state.next_microstep, bool)
        or state.next_microstep < 1
    ):
        return "microstep limit or cursor is invalid"
    if type(state.pending) is not tuple or type(state.resolved) is not tuple:
        return "microstep collections must be tuples"

    identities: set[str] = set()
    for item in state.pending:
        if (
            not isinstance(item, AStepObligation)
            or not isinstance(item.identity, str)
            or not item.identity.strip()
            or not isinstance(item.owner, str)
            or not item.owner.strip()
            or not isinstance(item.microstep, int)
            or isinstance(item.microstep, bool)
            or item.microstep < 0
            or item.microstep > state.max_microsteps
            or not isinstance(item.commutative, bool)
        ):
            return "invalid pending microstep obligation"
        if item.identity in identities:
            return "microstep obligation identity is duplicated"
        identities.add(item.identity)

    for identity in state.resolved:
        if not isinstance(identity, str) or not identity.strip():
            return "invalid resolved obligation identity"
        if identity in identities:
            return "microstep obligation identity is duplicated"
        identities.add(identity)
    if len(set(state.resolved)) != len(state.resolved):
        return "resolved obligation identity is duplicated"
    return None


def make_cycle(
    at_common: Fraction,
    obligations: tuple[tuple[str, str, bool], ...],
    *,
    max_microsteps: int = 32,
) -> AStepResult:
    """Create a cycle with named initial obligations at logical microstep zero."""
    if (
        not _exact(at_common)
        or type(obligations) is not tuple
        or not isinstance(max_microsteps, int)
        or isinstance(max_microsteps, bool)
        or max_microsteps <= 0
    ):
        empty = AStepCycle(Fraction(0), 1, 1, ())
        return AStepResult(
            empty, AStepCode.INVALID_INPUT, "invalid microstep cycle descriptor"
        )

    pending: list[AStepObligation] = []
    seen: set[str] = set()
    for descriptor in obligations:
        if (
            type(descriptor) is not tuple
            or len(descriptor) != 3
            or not isinstance(descriptor[0], str)
            or not descriptor[0].strip()
            or not isinstance(descriptor[1], str)
            or not descriptor[1].strip()
            or not isinstance(descriptor[2], bool)
        ):
            empty = AStepCycle(Fraction(at_common), max_microsteps, 1, ())
            return AStepResult(
                empty, AStepCode.INVALID_INPUT, "invalid initial microstep obligation"
            )
        if descriptor[0] in seen:
            empty = AStepCycle(Fraction(at_common), max_microsteps, 1, ())
            return AStepResult(
                empty, AStepCode.INVALID_INPUT,
                "initial microstep obligation identity is duplicated",
            )
        seen.add(descriptor[0])
        pending.append(AStepObligation(descriptor[0], descriptor[1], 0, descriptor[2]))

    state = AStepCycle(
        Fraction(at_common),
        max_microsteps,
        1,
        tuple(sorted(pending, key=lambda item: (item.microstep, item.identity))),
    )
    return AStepResult(
        state,
        AStepCode.COMPLETE if not pending else AStepCode.ADVANCED,
        "cycle initialized",
    )


def process_obligation(
    state: AStepCycle,
    obligation_id: str,
    *,
    resolves_obligation: bool,
    emitted: tuple[tuple[str, str, bool], ...] = (),
) -> AStepResult:
    """Resolve one obligation and optionally emit children at the next microstep.

    Every progressing operation consumes an existing obligation. Repeated/no-op
    attempts do not advance the cursor. UNKNOWN simultaneous non-commutative
    obligations remain indeterminate and preserve the original state.
    """
    invalid = _validate(state)
    if invalid is not None:
        return AStepResult(state, AStepCode.INVALID_INPUT, invalid)
    if (
        not isinstance(obligation_id, str)
        or not obligation_id.strip()
        or not isinstance(resolves_obligation, bool)
        or type(emitted) is not tuple
    ):
        return AStepResult(
            state, AStepCode.INVALID_INPUT, "invalid microstep operation descriptor"
        )

    children: list[AStepObligation] = []
    for descriptor in emitted:
        if (
            type(descriptor) is not tuple
            or len(descriptor) != 3
            or not isinstance(descriptor[0], str)
            or not descriptor[0].strip()
            or not isinstance(descriptor[1], str)
            or not descriptor[1].strip()
            or not isinstance(descriptor[2], bool)
        ):
            return AStepResult(
                state, AStepCode.INVALID_INPUT, "invalid emitted obligation descriptor"
            )
        children.append(
            AStepObligation(
                descriptor[0], descriptor[1], state.next_microstep, descriptor[2]
            )
        )

    if not resolves_obligation and children:
        return AStepResult(
            state,
            AStepCode.INVALID_INPUT,
            "an unresolved obligation cannot emit a progressing child",
        )

    current = next(
        (item for item in state.pending if item.identity == obligation_id), None
    )
    if current is None:
        return AStepResult(
            state,
            AStepCode.BLOCKED,
            "no matching pending obligation; repeated operation made no progress",
        )

    same_level = tuple(
        item for item in state.pending if item.microstep == current.microstep
    )
    if len(same_level) > 1 and not all(item.commutative for item in same_level):
        return AStepResult(
            state,
            AStepCode.INDETERMINATE,
            "simultaneous non-commutative obligations have no phase evidence",
        )
    if not resolves_obligation:
        return AStepResult(
            state,
            AStepCode.BLOCKED,
            "microstep did not resolve an identifiable obligation",
        )

    existing_ids = {item.identity for item in state.pending} | set(state.resolved)
    emitted_ids = [item.identity for item in children]
    if len(emitted_ids) != len(set(emitted_ids)) or existing_ids.intersection(emitted_ids):
        return AStepResult(
            state, AStepCode.INVALID_INPUT, "emitted obligation identity is not unique"
        )
    if children and state.next_microstep > state.max_microsteps:
        return AStepResult(
            state,
            AStepCode.RESOURCE_EXHAUSTED,
            "finite microstep limit exhausted before child admission",
        )

    pending = tuple(sorted(
        tuple(item for item in state.pending if item.identity != obligation_id)
        + tuple(children),
        key=lambda item: (item.microstep, item.identity),
    ))
    resolved = tuple(sorted(state.resolved + (obligation_id,)))
    next_microstep = state.next_microstep + 1 if children else state.next_microstep
    updated = AStepCycle(
        state.at_common, state.max_microsteps, next_microstep, pending, resolved
    )
    return AStepResult(
        updated,
        AStepCode.COMPLETE if not pending else AStepCode.ADVANCED,
        "obligation resolved and children admitted" if children else "obligation resolved",
    )


def close_cycle(state: AStepCycle) -> AStepResult:
    """Close only an empty cycle; pending or UNKNOWN work cannot be skipped."""
    invalid = _validate(state)
    if invalid is not None:
        return AStepResult(state, AStepCode.INVALID_INPUT, invalid)
    if not state.pending:
        return AStepResult(state, AStepCode.COMPLETE, "cycle has no pending obligations")

    for item in state.pending:
        same_level = tuple(
            other for other in state.pending if other.microstep == item.microstep
        )
        if len(same_level) > 1 and not all(other.commutative for other in same_level):
            return AStepResult(
                state,
                AStepCode.INDETERMINATE,
                "cycle closure cannot order simultaneous non-commutative obligations",
            )
    return AStepResult(
        state, AStepCode.BLOCKED, "cycle still has pending obligations"
    )
