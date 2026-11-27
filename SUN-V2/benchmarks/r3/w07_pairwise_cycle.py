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


@dataclass(frozen=True)
class StepResult:
    state: PairwiseCycle
    code: StepCode
    detail: str = ""


def _validate(state: PairwiseCycle) -> str | None:
    if not isinstance(state, PairwiseCycle) or type(state.at_common) is not int:
        return "invalid cycle or common instant"
    if type(state.pending) is not tuple or type(state.constraints) is not tuple:
        return "cycle collections must be immutable tuples"
    ids: set[str] = set()
    for item in state.pending:
        if (
            not isinstance(item, Obligation)
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.owner, str) or not item.owner.strip()
            or type(item.microstep) is not int or item.microstep < 0
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
    pairs: set[frozenset[str]] = set()
    for item in state.constraints:
        if (
            not isinstance(item, PairConstraint)
            or not isinstance(item.first_id, str) or not item.first_id.strip()
            or not isinstance(item.second_id, str) or not item.second_id.strip()
            or item.first_id == item.second_id
            or not isinstance(item.relation, PairRelation)
        ):
            return "invalid pairwise relation descriptor"
        key = frozenset((item.first_id, item.second_id))
        if key in pairs:
            return "duplicate unordered pair relation"
        pairs.add(key)
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
    emitted_ids = [item.identity for item in emitted if isinstance(item, Obligation)]
    if len(emitted_ids) != len(emitted) or len(set(emitted_ids)) != len(emitted_ids):
        return StepResult(state, StepCode.INVALID_INPUT, "invalid emitted obligations")
    existing = {item.identity for item in state.pending} | set(state.resolved)
    if existing.intersection(emitted_ids):
        return StepResult(state, StepCode.INVALID_INPUT, "emitted identity is not unique")
    for child in emitted:
        if (
            not isinstance(child.owner, str) or not child.owner.strip()
            or type(child.microstep) is not int or child.microstep < 0
        ):
            return StepResult(state, StepCode.INVALID_INPUT, "invalid emitted obligation")
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
    updated = PairwiseCycle(
        state.at_common, pending, state.constraints,
        tuple(sorted(state.resolved + (requested_id,))),
    )
    return StepResult(
        updated, StepCode.COMPLETE if not pending else StepCode.ADVANCED,
        "obligation resolved under explicit pairwise relations",
    )
