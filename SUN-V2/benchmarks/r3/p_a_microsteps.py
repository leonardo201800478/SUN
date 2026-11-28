"""P-A disposable finite same-instant microstep prototype.

This candidate owns its immutable state types and transition logic. It models
logical obligations only; it does not assign physical chip phases or implement
a production dispatcher. Cross-obligation order is adjudicated pairwise; numeric
microstep levels alone never establish causality or commutativity.
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


class AStepPairRelation(str, Enum):
    FIRST_CAUSES_SECOND = "first_causes_second"
    SECOND_CAUSES_FIRST = "second_causes_first"
    COMMUTATIVE = "commutative"
    UNKNOWN = "unknown"


@dataclass(frozen=True)
class AStepObligation:
    """Named obligation pending at one logical microstep."""

    identity: str
    owner: str
    microstep: int = 0
    commutative: bool = False


@dataclass(frozen=True)
class AStepPairConstraint:
    """Explicit relation for one unordered pair of same-instant obligations."""

    first_id: str
    second_id: str
    relation: AStepPairRelation


@dataclass(frozen=True)
class AStepCycle:
    """Immutable same-instant cycle with explicit pair relations and finite budget."""

    at_common: Fraction
    max_microsteps: int
    next_microstep: int
    pending: tuple[AStepObligation, ...]
    resolved: tuple[str, ...] = ()
    future: tuple[AStepObligation, ...] = ()
    relations: tuple[AStepPairConstraint, ...] = ()


@dataclass(frozen=True)
class AStepResult:
    """Result of one microstep operation and its immutable successor state."""

    state: AStepCycle
    code: AStepCode
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept integer/rational time while excluding booleans and floats."""
    return type(value) is int or type(value) is Fraction


def _validate(state: object) -> str | None:
    """Validate all state fields and pairwise descriptors before processing."""
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
    if (
        type(state.pending) is not tuple
        or type(state.resolved) is not tuple
        or type(state.future) is not tuple
        or type(state.relations) is not tuple
    ):
        return "microstep collections must be tuples"

    identities: set[str] = set()
    obligations_by_id: dict[str, AStepObligation] = {}
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
        obligations_by_id[item.identity] = item

    for identity in state.resolved:
        if not isinstance(identity, str) or not identity.strip():
            return "invalid resolved obligation identity"
        if identity in identities:
            return "microstep obligation identity is duplicated"
        identities.add(identity)
    if len(set(state.resolved)) != len(state.resolved):
        return "resolved obligation identity is duplicated"

    for item in state.future:
        if (
            not isinstance(item, AStepObligation)
            or not isinstance(item.identity, str)
            or not item.identity.strip()
            or not isinstance(item.owner, str)
            or not item.owner.strip()
            or not isinstance(item.microstep, int)
            or isinstance(item.microstep, bool)
            or item.microstep < 0
            or not isinstance(item.commutative, bool)
        ):
            return "invalid future microstep obligation"
        if item.identity in identities:
            return "future microstep identity is duplicated or conflicting"
        identities.add(item.identity)
        obligations_by_id[item.identity] = item

    pair_keys: set[frozenset[str]] = set()
    known_ids = identities
    for constraint in state.relations:
        if (
            not isinstance(constraint, AStepPairConstraint)
            or not isinstance(constraint.first_id, str)
            or not constraint.first_id.strip()
            or not isinstance(constraint.second_id, str)
            or not constraint.second_id.strip()
            or constraint.first_id == constraint.second_id
            or constraint.first_id not in known_ids
            or constraint.second_id not in known_ids
            or not isinstance(constraint.relation, AStepPairRelation)
        ):
            return "invalid pairwise microstep relation"
        key = frozenset((constraint.first_id, constraint.second_id))
        if key in pair_keys:
            return "duplicate unordered pairwise microstep relation"
        pair_keys.add(key)
        first = obligations_by_id.get(constraint.first_id)
        second = obligations_by_id.get(constraint.second_id)
        if first is not None and second is not None:
            if (
                constraint.relation is AStepPairRelation.FIRST_CAUSES_SECOND
                and first.microstep > second.microstep
            ):
                return "pairwise causal relation contradicts microstep assignment"
            if (
                constraint.relation is AStepPairRelation.SECOND_CAUSES_FIRST
                and second.microstep > first.microstep
            ):
                return "pairwise causal relation contradicts microstep assignment"
    return None


def _relation(state: AStepCycle, requested: str, other: str) -> AStepPairRelation:
    """Return the relation oriented from requested to other, defaulting to UNKNOWN."""
    for constraint in state.relations:
        if (constraint.first_id, constraint.second_id) == (requested, other):
            if constraint.relation is AStepPairRelation.FIRST_CAUSES_SECOND:
                return AStepPairRelation.FIRST_CAUSES_SECOND
            if constraint.relation is AStepPairRelation.SECOND_CAUSES_FIRST:
                return AStepPairRelation.SECOND_CAUSES_FIRST
            return constraint.relation
        if (constraint.first_id, constraint.second_id) == (other, requested):
            if constraint.relation is AStepPairRelation.FIRST_CAUSES_SECOND:
                return AStepPairRelation.SECOND_CAUSES_FIRST
            if constraint.relation is AStepPairRelation.SECOND_CAUSES_FIRST:
                return AStepPairRelation.FIRST_CAUSES_SECOND
            return constraint.relation
    return AStepPairRelation.UNKNOWN


def make_cycle(
    at_common: Fraction,
    obligations: tuple[tuple[str, str, bool], ...],
    *,
    max_microsteps: int = 32,
    future: tuple[AStepObligation, ...] = (),
    relations: tuple[AStepPairConstraint, ...] = (),
) -> AStepResult:
    """Create a cycle with initial obligations and optional declared future/pair relations."""
    if (
        not _exact(at_common)
        or type(obligations) is not tuple
        or not isinstance(max_microsteps, int)
        or isinstance(max_microsteps, bool)
        or max_microsteps <= 0
        or type(future) is not tuple
        or type(relations) is not tuple
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
        (),
        future,
        relations,
    )
    invalid = _validate(state)
    if invalid is not None:
        return AStepResult(state, AStepCode.INVALID_INPUT, invalid)
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
    """Resolve one obligation only when every pending pair has an admissible relation.

    Numeric microstep levels are participant-local logical coordinates. They do
    not order different obligations. UNKNOWN pair relations return
    IndeterminateCausalOrder without mutation; explicit predecessor relations
    return Blocked; causal successors and commutative pairs permit progress.
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

    current = next(
        (item for item in state.pending if item.identity == obligation_id), None
    )
    if current is None:
        return AStepResult(
            state,
            AStepCode.BLOCKED,
            "no matching pending obligation; repeated operation made no progress",
        )

    children: list[AStepObligation] = []
    future_by_id = {item.identity: item for item in state.future}
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
        child = AStepObligation(
            descriptor[0], descriptor[1], state.next_microstep, descriptor[2]
        )
        declared = future_by_id.get(child.identity)
        if state.future and declared is None:
            return AStepResult(
                state, AStepCode.INVALID_INPUT,
                f"emitted obligation {child.identity} was not declared as future",
            )
        if declared is not None and declared != child:
            return AStepResult(
                state, AStepCode.INVALID_INPUT,
                f"emitted obligation {child.identity} differs from its future declaration",
            )
        if child.microstep > state.max_microsteps:
            return AStepResult(
                state,
                AStepCode.RESOURCE_EXHAUSTED,
                "finite microstep limit exhausted before child admission",
            )
        if child.microstep <= current.microstep:
            return AStepResult(
                state,
                AStepCode.INVALID_INPUT,
                "emitted obligation is not assigned to a later microstep",
            )
        children.append(child)

    if not resolves_obligation and children:
        return AStepResult(
            state,
            AStepCode.INVALID_INPUT,
            "an unresolved obligation cannot emit a progressing child",
        )
    if not resolves_obligation:
        return AStepResult(
            state,
            AStepCode.BLOCKED,
            "microstep did not resolve an identifiable obligation",
        )

    emitted_ids = [item.identity for item in children]
    existing_ids = {item.identity for item in state.pending} | set(state.resolved)
    if (
        len(emitted_ids) != len(set(emitted_ids))
        or existing_ids.intersection(emitted_ids)
    ):
        return AStepResult(
            state, AStepCode.INVALID_INPUT, "emitted obligation identity is not unique"
        )

    for other in state.pending:
        if other.identity == obligation_id:
            continue
        relation = _relation(state, obligation_id, other.identity)
        if relation is AStepPairRelation.UNKNOWN:
            return AStepResult(
                state,
                AStepCode.INDETERMINATE,
                f"pairwise relation between {obligation_id} and {other.identity} is UNKNOWN",
            )
        if relation is AStepPairRelation.SECOND_CAUSES_FIRST:
            return AStepResult(
                state,
                AStepCode.BLOCKED,
                f"{other.identity} is an explicit predecessor of {obligation_id}",
            )

    pending = tuple(sorted(
        tuple(item for item in state.pending if item.identity != obligation_id)
        + tuple(children),
        key=lambda item: (item.microstep, item.identity),
    ))
    emitted_set = set(emitted_ids)
    remaining_future = tuple(
        item for item in state.future if item.identity not in emitted_set
    )
    resolved = tuple(sorted(state.resolved + (obligation_id,)))
    next_microstep = state.next_microstep + 1 if children else state.next_microstep
    updated = AStepCycle(
        state.at_common,
        state.max_microsteps,
        next_microstep,
        pending,
        resolved,
        remaining_future,
        state.relations,
    )
    return AStepResult(
        updated,
        AStepCode.COMPLETE if not pending else AStepCode.ADVANCED,
        "obligation resolved and children admitted" if children else "obligation resolved",
    )


def close_cycle(state: AStepCycle) -> AStepResult:
    """Close only an empty cycle; unresolved pair relations cannot be skipped."""
    invalid = _validate(state)
    if invalid is not None:
        return AStepResult(state, AStepCode.INVALID_INPUT, invalid)
    if not state.pending:
        return AStepResult(state, AStepCode.COMPLETE, "cycle has no pending obligations")

    for index, first in enumerate(state.pending):
        for second in state.pending[index + 1:]:
            if _relation(state, first.identity, second.identity) is AStepPairRelation.UNKNOWN:
                return AStepResult(
                    state,
                    AStepCode.INDETERMINATE,
                    f"pairwise relation between {first.identity} and {second.identity} is UNKNOWN",
                )
    return AStepResult(
        state, AStepCode.BLOCKED, "cycle still has pending obligations"
    )
