"""Independent finite same-instant obligation-cycle oracle for CHRONOS R2-T14.

Verification-only model. It represents logical obligations and microsteps with
immutable tuples; it does not select a production dispatcher or physical phase.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from fractions import Fraction


class MicrostepCode(str, Enum):
    ADVANCED = "Advanced"
    COMPLETE = "Complete"
    BLOCKED = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"
    INVALID_INPUT = "InvalidInput"
    RESOURCE_EXHAUSTED = "ResourceExhausted"


@dataclass(frozen=True)
class MicrostepObligation:
    """One identifiable same-instant obligation."""

    obligation_id: str
    owner: str
    microstep: int = 0
    commutative: bool = False


@dataclass(frozen=True)
class MicrostepCycle:
    """Immutable logical cycle; no wall-clock or physical phase is modeled."""

    at_common: Fraction
    max_microsteps: int
    next_microstep: int
    pending: tuple[MicrostepObligation, ...]
    resolved: tuple[str, ...] = ()


@dataclass(frozen=True)
class MicrostepEvaluation:
    state: MicrostepCycle
    code: MicrostepCode
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept integer/rational time and reject bool/float values."""
    return isinstance(value, (int, Fraction)) and not isinstance(value, bool)


def _validate_cycle(state: object) -> str | None:
    """Validate cycle structure before reading obligation identities."""
    if not isinstance(state, MicrostepCycle):
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
    if not isinstance(state.pending, tuple) or not isinstance(state.resolved, tuple):
        return "microstep collections must be tuples"
    identities: list[str] = []
    for item in state.pending:
        if not isinstance(item, MicrostepObligation):
            return "pending collection contains an invalid obligation"
        if (
            not isinstance(item.obligation_id, str)
            or not item.obligation_id.strip()
            or not isinstance(item.owner, str)
            or not item.owner.strip()
            or not isinstance(item.microstep, int)
            or isinstance(item.microstep, bool)
            or item.microstep < 0
            or item.microstep > state.max_microsteps
            or not isinstance(item.commutative, bool)
        ):
            return "pending obligation descriptor is invalid"
        identities.append(item.obligation_id)
    if any(not isinstance(item, str) or not item.strip() for item in state.resolved):
        return "resolved obligation identity is invalid"
    identities.extend(state.resolved)
    if len(identities) != len(set(identities)):
        return "microstep obligation identity is duplicated"
    return None


def make_cycle(
    at_common: Fraction,
    initial: tuple[tuple[str, str, bool], ...],
    *,
    max_microsteps: int = 32,
) -> MicrostepEvaluation:
    """Create a cycle with named initial obligations at logical microstep zero."""
    if (
        not _exact(at_common)
        or not isinstance(initial, tuple)
        or not isinstance(max_microsteps, int)
        or isinstance(max_microsteps, bool)
        or max_microsteps <= 0
    ):
        empty = MicrostepCycle(Fraction(0), 1, 1, ())
        return MicrostepEvaluation(
            empty, MicrostepCode.INVALID_INPUT, "invalid microstep cycle descriptor"
        )
    pending: list[MicrostepObligation] = []
    for descriptor in initial:
        if (
            not isinstance(descriptor, tuple)
            or len(descriptor) != 3
            or not isinstance(descriptor[0], str)
            or not descriptor[0].strip()
            or not isinstance(descriptor[1], str)
            or not descriptor[1].strip()
            or not isinstance(descriptor[2], bool)
        ):
            empty = MicrostepCycle(Fraction(at_common), max_microsteps, 1, ())
            return MicrostepEvaluation(
                empty, MicrostepCode.INVALID_INPUT,
                "invalid initial microstep obligation",
            )
        pending.append(
            MicrostepObligation(descriptor[0], descriptor[1], 0, descriptor[2])
        )
    if len({item.obligation_id for item in pending}) != len(pending):
        empty = MicrostepCycle(Fraction(at_common), max_microsteps, 1, ())
        return MicrostepEvaluation(
            empty, MicrostepCode.INVALID_INPUT,
            "initial microstep obligation identity is duplicated",
        )
    state = MicrostepCycle(
        Fraction(at_common),
        max_microsteps,
        1,
        tuple(sorted(pending, key=lambda item: (item.microstep, item.obligation_id))),
    )
    return MicrostepEvaluation(
        state,
        MicrostepCode.COMPLETE if not pending else MicrostepCode.ADVANCED,
        "cycle initialized",
    )


def process_obligation(
    state: MicrostepCycle,
    obligation_id: str,
    *,
    resolves_obligation: bool,
    emitted: tuple[tuple[str, str, bool], ...] = (),
) -> MicrostepEvaluation:
    """Resolve one obligation and optionally emit children at the next microstep.

    A repeated/no-op attempt cannot advance the cursor. Simultaneous
    non-commutative obligations without phase evidence remain indeterminate.
    """
    invalid = _validate_cycle(state)
    if invalid is not None:
        return MicrostepEvaluation(state, MicrostepCode.INVALID_INPUT, invalid)
    if (
        not isinstance(obligation_id, str)
        or not obligation_id.strip()
        or not isinstance(resolves_obligation, bool)
        or not isinstance(emitted, tuple)
    ):
        return MicrostepEvaluation(
            state, MicrostepCode.INVALID_INPUT, "invalid microstep operation descriptor"
        )
    children: list[MicrostepObligation] = []
    for descriptor in emitted:
        if (
            not isinstance(descriptor, tuple)
            or len(descriptor) != 3
            or not isinstance(descriptor[0], str)
            or not descriptor[0].strip()
            or not isinstance(descriptor[1], str)
            or not descriptor[1].strip()
            or not isinstance(descriptor[2], bool)
        ):
            return MicrostepEvaluation(
                state, MicrostepCode.INVALID_INPUT, "invalid emitted obligation descriptor"
            )
        children.append(
            MicrostepObligation(
                descriptor[0], descriptor[1], state.next_microstep, descriptor[2]
            )
        )
    if not resolves_obligation and children:
        return MicrostepEvaluation(
            state, MicrostepCode.INVALID_INPUT,
            "an unresolved obligation cannot emit a progressing child",
        )
    current = next(
        (item for item in state.pending if item.obligation_id == obligation_id),
        None,
    )
    if current is None:
        return MicrostepEvaluation(
            state, MicrostepCode.BLOCKED,
            "no matching pending obligation; repeated operation made no progress",
        )
    same_level = tuple(
        item for item in state.pending if item.microstep == current.microstep
    )
    if len(same_level) > 1 and not all(item.commutative for item in same_level):
        return MicrostepEvaluation(
            state, MicrostepCode.INDETERMINATE,
            "simultaneous non-commutative obligations have no phase evidence",
        )
    if not resolves_obligation:
        return MicrostepEvaluation(
            state, MicrostepCode.BLOCKED,
            "microstep did not resolve an identifiable obligation",
        )
    existing_ids = {
        item.obligation_id for item in state.pending
    } | set(state.resolved)
    emitted_ids = [item.obligation_id for item in children]
    if (
        len(emitted_ids) != len(set(emitted_ids))
        or existing_ids.intersection(emitted_ids)
    ):
        return MicrostepEvaluation(
            state, MicrostepCode.INVALID_INPUT,
            "emitted obligation identity is not unique",
        )
    if children and state.next_microstep > state.max_microsteps:
        return MicrostepEvaluation(
            state, MicrostepCode.RESOURCE_EXHAUSTED,
            "finite microstep limit exhausted before child admission",
        )
    pending = tuple(sorted(
        tuple(item for item in state.pending if item.obligation_id != obligation_id)
        + tuple(children),
        key=lambda item: (item.microstep, item.obligation_id),
    ))
    resolved = tuple(sorted(state.resolved + (obligation_id,)))
    next_microstep = state.next_microstep + 1 if children else state.next_microstep
    updated = MicrostepCycle(
        state.at_common, state.max_microsteps, next_microstep, pending, resolved
    )
    return MicrostepEvaluation(
        updated,
        MicrostepCode.COMPLETE if not pending else MicrostepCode.ADVANCED,
        "obligation resolved and children admitted" if children
        else "obligation resolved",
    )


def close_cycle(state: MicrostepCycle) -> MicrostepEvaluation:
    """Close only an empty cycle; pending or UNKNOWN work cannot be skipped."""
    invalid = _validate_cycle(state)
    if invalid is not None:
        return MicrostepEvaluation(state, MicrostepCode.INVALID_INPUT, invalid)
    if not state.pending:
        return MicrostepEvaluation(state, MicrostepCode.COMPLETE, "cycle has no pending obligations")
    for item in state.pending:
        same_level = tuple(
            other for other in state.pending if other.microstep == item.microstep
        )
        if len(same_level) > 1 and not all(other.commutative for other in same_level):
            return MicrostepEvaluation(
                state, MicrostepCode.INDETERMINATE,
                "watermark/cycle closure cannot order simultaneous non-commutative obligations",
            )
    return MicrostepEvaluation(
        state, MicrostepCode.BLOCKED,
        "cycle still has identifiable pending obligations",
    )
