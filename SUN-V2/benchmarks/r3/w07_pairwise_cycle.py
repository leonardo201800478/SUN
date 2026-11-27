"""Disposable W07 pairwise-relation microstep candidate.

Unlike the existing unary-flag prototype, this model requires an explicit
relation for every pair of simultaneously pending obligations. Microstep
numbers never manufacture causality or commutativity.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class PairRelation(str, Enum):
    FIRST_CAUSES_SECOND = "first_causes_second"
    SECOND_CAUSES_FIRST = "second_causes_first"
    COMMUTATIVE = "commutative"
    UNKNOWN = "unknown"


class StepCode(str, Enum):
    ADVANCED = "Advanced"
    COMPLETE = "Complete"
    BLOCKED = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class Obligation:
    identity: str
    owner: str
    microstep: int


@dataclass(frozen=True)
class PairConstraint:
    first_id: str
    second_id: str
    relation: PairRelation


@dataclass(frozen=True)
class PairwiseCycle:
    at_common: int
    pending: tuple[Obligation, ...]
    constraints: tuple[PairConstraint, ...]
    resolved: tuple[str, ...] = ()
    future: tuple[Obligation, ...] = ()
    max_microsteps: int = 32


@dataclass(frozen=True)
class StepResult:
    state: PairwiseCycle
    code: StepCode
    detail: str = ""


def _validate(state: PairwiseCycle) -> str | None:
    if not isinstance(state, PairwiseCycle) or type(state.at_common) is not int:
        return "invalid cycle or common instant"
    if type(state.max_microsteps) is not int or state.max_microsteps <= 0:
        return "maximum microsteps must be a positive integer"
    if type(state.pending) is not tuple or type(state.constraints) is not tuple:
        return "cycle collections must be immutable tuples"
    ids: set[str] = set()
    for item in state.pending:
        if (
            not isinstance(item, Obligation)
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.owner, str) or not item.owner.strip()
            or type(item.microstep) is not int or item.microstep < 0
            or item.microstep > state.max_microsteps
            or item.identity in ids
        ):
            return "invalid or duplicated pending obligation"
        ids.add(item.identity)
    if type(state.resolved) is not tuple or any(
        not isinstance(item, str) or not item.strip() for item in state.resolved
    ):
        return "invalid resolved ledger"
    if len(set(state.resolved)) != len(state.resolved) or ids.intersection(state.resolved):
        return "duplicate identity across pending/resolved ledger"
    if type(state.future) is not tuple:
        return "future declarations must be an immutable tuple"
    future_ids: set[str] = set()
    for item in state.future:
        if (
            not isinstance(item, Obligation)
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.owner, str) or not item.owner.strip()
            or type(item.microstep) is not int or item.microstep < 0
            or item.identity in ids
            or item.identity in state.resolved
            or item.identity in future_ids
        ):
            return "invalid, duplicated, or conflicting future declaration"
        future_ids.add(item.identity)
    known_ids = ids | set(state.resolved) | future_ids
    obligations_by_id = {item.identity: item for item in state.pending + state.future}
    pairs: set[frozenset[str]] = set()
    for item in state.constraints:
        if (
            not isinstance(item, PairConstraint)
            or not isinstance(item.first_id, str) or not item.first_id.strip()
            or not isinstance(item.second_id, str) or not item.second_id.strip()
            or item.first_id == item.second_id
            or item.first_id not in known_ids
            or item.second_id not in known_ids
            or not isinstance(item.relation, PairRelation)
        ):
            return "invalid pairwise relation descriptor"
        key = frozenset((item.first_id, item.second_id))
        if key in pairs:
            return "duplicate unordered pair relation"
        pairs.add(key)
        first = obligations_by_id.get(item.first_id)
        second = obligations_by_id.get(item.second_id)
        if first is not None and second is not None:
            if (
                item.relation is PairRelation.FIRST_CAUSES_SECOND
                and first.microstep > second.microstep
            ):
                return "causal relation contradicts declared microstep order"
            if (
                item.relation is PairRelation.SECOND_CAUSES_FIRST
                and second.microstep > first.microstep
            ):
                return "causal relation contradicts declared microstep order"
    return None


def _relation(state: PairwiseCycle, requested: str, other: str) -> PairRelation:
    for item in state.constraints:
        if (item.first_id, item.second_id) == (requested, other):
            if item.relation is PairRelation.FIRST_CAUSES_SECOND:
                return PairRelation.FIRST_CAUSES_SECOND
            if item.relation is PairRelation.SECOND_CAUSES_FIRST:
                return PairRelation.SECOND_CAUSES_FIRST
            return item.relation
        if (item.first_id, item.second_id) == (other, requested):
            if item.relation is PairRelation.FIRST_CAUSES_SECOND:
                return PairRelation.SECOND_CAUSES_FIRST
            if item.relation is PairRelation.SECOND_CAUSES_FIRST:
                return PairRelation.FIRST_CAUSES_SECOND
            return item.relation
    return PairRelation.UNKNOWN


def process(state: PairwiseCycle, requested_id: str, *, emitted: tuple[Obligation, ...] = ()) -> StepResult:
    """Resolve one obligation only when all pending pair relations permit it.

    UNKNOWN returns IndeterminateCausalOrder with the exact original state.
    A predecessor relation returns Blocked; a successor or commutative pair
    permits progress. New child/pending pairs default to UNKNOWN.
    """
    invalid = _validate(state)
    if invalid is not None:
        return StepResult(state, StepCode.INVALID_INPUT, invalid)
    if not isinstance(requested_id, str) or not requested_id.strip() or type(emitted) is not tuple:
        return StepResult(state, StepCode.INVALID_INPUT, "invalid operation descriptor")
    current = next((item for item in state.pending if item.identity == requested_id), None)
    if current is None:
        return StepResult(state, StepCode.BLOCKED, "requested obligation is not pending")
    emitted_ids: list[str] = []
    future_by_id = {item.identity: item for item in state.future}
    for child in emitted:
        if (
            not isinstance(child, Obligation)
            or not isinstance(child.identity, str) or not child.identity.strip()
            or not isinstance(child.owner, str) or not child.owner.strip()
            or type(child.microstep) is not int or child.microstep < 0
        ):
            return StepResult(state, StepCode.INVALID_INPUT, "invalid emitted obligation")
        emitted_ids.append(child.identity)
        declared = future_by_id.get(child.identity)
        if declared is None:
            return StepResult(
                state, StepCode.INVALID_INPUT,
                f"emitted obligation {child.identity} was not declared as future",
            )
        if declared != child:
            return StepResult(
                state, StepCode.INVALID_INPUT,
                f"emitted obligation {child.identity} differs from its future declaration",
            )
        if child.microstep > state.max_microsteps:
            return StepResult(
                state, StepCode.RESOURCE_EXHAUSTED,
                f"emitted obligation {child.identity} exceeds finite microstep limit {state.max_microsteps}",
            )
        if child.microstep <= current.microstep:
            return StepResult(
                state, StepCode.INVALID_INPUT,
                f"emitted obligation {child.identity} is not assigned to a later microstep",
            )
    if len(set(emitted_ids)) != len(emitted_ids):
        return StepResult(state, StepCode.INVALID_INPUT, "duplicate emitted obligation identity")
    earliest_microstep = min(item.microstep for item in state.pending)
    if current.microstep > earliest_microstep:
        for earlier in state.pending:
            if earlier.microstep >= current.microstep:
                continue
            relation = _relation(state, requested_id, earlier.identity)
            if relation is PairRelation.UNKNOWN:
                return StepResult(
                    state, StepCode.INDETERMINATE,
                    f"relation between {requested_id} and {earlier.identity} is UNKNOWN",
                )
            if relation is PairRelation.SECOND_CAUSES_FIRST:
                return StepResult(
                    state, StepCode.BLOCKED,
                    f"{earlier.identity} is a predecessor in an earlier microstep",
                )
            if relation is PairRelation.COMMUTATIVE:
                return StepResult(
                    state, StepCode.BLOCKED,
                    f"earlier microstep {earlier.microstep} must resolve before {current.microstep}",
                )
    for other in state.pending:
        if other.identity == requested_id:
            continue
        relation = _relation(state, requested_id, other.identity)
        if relation is PairRelation.UNKNOWN:
            return StepResult(
                state, StepCode.INDETERMINATE,
                f"relation between {requested_id} and {other.identity} is UNKNOWN",
            )
        if relation is PairRelation.SECOND_CAUSES_FIRST:
            return StepResult(
                state, StepCode.BLOCKED,
                f"{other.identity} is a declared predecessor of {requested_id}",
            )
    pending = tuple(sorted(
        tuple(item for item in state.pending if item.identity != requested_id) + emitted,
        key=lambda item: (item.microstep, item.identity),
    ))
    emitted_set = set(emitted_ids)
    remaining_future = tuple(
        item for item in state.future if item.identity not in emitted_set
    )
    updated = PairwiseCycle(
        at_common=state.at_common,
        pending=pending,
        constraints=state.constraints,
        resolved=tuple(sorted(state.resolved + (requested_id,))),
        future=remaining_future,
    )
    return StepResult(
        updated, StepCode.COMPLETE if not pending else StepCode.ADVANCED,
        "obligation resolved under explicit pairwise relations",
    )
