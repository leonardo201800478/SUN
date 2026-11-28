"""Disposable P-D candidate: central discrete agenda.

The agenda owns event ordering only. Device state remains outside this module.
All temporal coordinates are exact; insertion order never resolves UNKNOWN ties.
"""
from __future__ import annotations

import heapq
from dataclasses import dataclass
from enum import Enum
from fractions import Fraction


class AgendaCode(str, Enum):
    ACCEPTED = "Accepted"
    CLAIMED = "Claimed"
    CANCELLED = "Cancelled"
    COMPLETED = "Completed"
    BLOCKED = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    RESOURCE_EXHAUSTED = "ResourceExhausted"


class EventStatus(str, Enum):
    SCHEDULED = "Scheduled"
    CANCELLED = "Cancelled"
    SUPERSEDED = "Superseded"
    CLAIMED = "Claimed"
    COMPLETED = "Completed"


class PairRelation(str, Enum):
    FIRST_BEFORE_SECOND = "first_before_second"
    SECOND_BEFORE_FIRST = "second_before_first"
    COMMUTATIVE = "commutative"
    UNKNOWN = "unknown"


@dataclass(frozen=True)
class AgendaPhaseBasis:
    profile_id: str
    profile_revision: str
    coordinate: str
    unit: str
    scale_numerator: int
    scale_denominator: int
    basis_id: str
    basis_revision: str
    validation_record_id: str
    validation_record_sha256: str


@dataclass(frozen=True)
class AgendaEvent:
    identity: str
    generation: int
    owner: str
    at_common: Fraction
    effect_id: str
    status: EventStatus = EventStatus.SCHEDULED
    microstep: int = 0
    phase_order: int | None = None
    phase_evidence: str | None = None
    phase_basis: AgendaPhaseBasis | None = None


@dataclass(frozen=True)
class AgendaConstraint:
    first_id: str
    second_id: str
    relation: PairRelation


@dataclass(frozen=True)
class AgendaState:
    heap: tuple[tuple[Fraction, str, int], ...] = ()
    events: tuple[AgendaEvent, ...] = ()
    constraints: tuple[AgendaConstraint, ...] = ()
    max_events: int = 4096


@dataclass(frozen=True)
class AgendaResult:
    state: AgendaState
    code: AgendaCode
    event: AgendaEvent | None = None
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept only exact integer/rational coordinates, never bool or float."""
    return type(value) in (int, Fraction)


def _valid_phase_basis(value: object) -> bool:
    """Validate basis shape only; this does not authenticate external evidence."""
    if not isinstance(value, AgendaPhaseBasis):
        return False
    strings = (
        value.profile_id, value.profile_revision, value.coordinate, value.unit,
        value.basis_id, value.basis_revision, value.validation_record_id,
        value.validation_record_sha256,
    )
    return (
        all(isinstance(item, str) and item.strip() for item in strings)
        and type(value.scale_numerator) is int and value.scale_numerator > 0
        and type(value.scale_denominator) is int and value.scale_denominator > 0
        and len(value.validation_record_sha256) == 64
        and all(char in "0123456789abcdefABCDEF"
                for char in value.validation_record_sha256)
    )


def _phase_relation(first: AgendaEvent, second: AgendaEvent) -> PairRelation:
    """Use phase order only when both events declare one identical valid basis."""
    if (
        first.phase_order is None or second.phase_order is None
        or first.phase_evidence is None or second.phase_evidence is None
        or not first.phase_evidence.strip() or not second.phase_evidence.strip()
        or not _valid_phase_basis(first.phase_basis)
        or first.phase_basis != second.phase_basis
        or first.phase_order == second.phase_order
    ):
        return PairRelation.UNKNOWN
    return (
        PairRelation.FIRST_BEFORE_SECOND
        if first.phase_order < second.phase_order
        else PairRelation.SECOND_BEFORE_FIRST
    )


def _key(event: AgendaEvent) -> tuple[Fraction, str, int]:
    """Return a stable heap key; identity is not a physical phase order."""
    return (event.at_common, event.identity, event.generation)


def _find(state: AgendaState, identity: str, generation: int) -> AgendaEvent | None:
    """Find one immutable event generation."""
    return next(
        (item for item in state.events
         if item.identity == identity and item.generation == generation),
        None,
    )


def _replace(state: AgendaState, updated: AgendaEvent) -> AgendaState:
    """Replace one event record without mutating the original state."""
    events = tuple(
        updated if (item.identity, item.generation) ==
        (updated.identity, updated.generation) else item
        for item in state.events
    )
    return AgendaState(state.heap, events, state.constraints, state.max_events)


def _validate(state: object) -> str | None:
    """Validate agenda, heap, generations and pairwise constraints atomically."""
    if not isinstance(state, AgendaState):
        return "invalid agenda state"
    if type(state.max_events) is not int or state.max_events <= 0:
        return "max_events must be a positive integer"
    if type(state.heap) is not tuple or type(state.events) is not tuple:
        return "agenda collections must be immutable tuples"
    keys: set[tuple[str, int]] = set()
    max_generation: dict[str, int] = {}
    scheduled: set[tuple[Fraction, str, int]] = set()
    for event in state.events:
        if (
            not isinstance(event, AgendaEvent)
            or not isinstance(event.identity, str) or not event.identity.strip()
            or not isinstance(event.owner, str) or not event.owner.strip()
            or not isinstance(event.effect_id, str) or not event.effect_id.strip()
            or not _exact(event.at_common)
            or type(event.generation) is not int or event.generation < 0
            or not isinstance(event.status, EventStatus)
            or type(event.microstep) is not int or event.microstep < 0
            or (event.phase_order is not None and type(event.phase_order) is not int)
            or (event.phase_evidence is not None and (
                not isinstance(event.phase_evidence, str) or not event.phase_evidence.strip()
            ))
            or (event.phase_basis is not None and not _valid_phase_basis(event.phase_basis))
        ):
            return "invalid event descriptor"
        key = (event.identity, event.generation)
        if key in keys:
            return "duplicate event generation"
        keys.add(key)
        max_generation[event.identity] = max(
            event.generation, max_generation.get(event.identity, -1)
        )
        if event.status is EventStatus.SCHEDULED:
            scheduled.add(_key(event))
    if len(state.events) > state.max_events:
        return "agenda capacity exceeded"
    for item in state.heap:
        if (
            type(item) is not tuple or len(item) != 3
            or not _exact(item[0])
            or not isinstance(item[1], str) or not item[1].strip()
            or type(item[2]) is not int or item[2] < 0
        ):
            return "invalid heap entry"
    heap_keys = set(state.heap)
    if heap_keys != scheduled or len(state.heap) != len(scheduled):
        return "heap and scheduled event records disagree"
    for index in range(1, len(state.heap)):
        parent = (index - 1) // 2
        if state.heap[parent] > state.heap[index]:
            return "heap property is invalid"
    if type(state.constraints) is not tuple:
        return "constraints must be an immutable tuple"
    known = {event.identity for event in state.events}
    pairs: set[frozenset[str]] = set()
    for item in state.constraints:
        if (
            not isinstance(item, AgendaConstraint)
            or not isinstance(item.first_id, str) or not item.first_id.strip()
            or not isinstance(item.second_id, str) or not item.second_id.strip()
            or item.first_id == item.second_id
            or item.first_id not in known or item.second_id not in known
            or not isinstance(item.relation, PairRelation)
        ):
            return "invalid pairwise relation"
        pair = frozenset((item.first_id, item.second_id))
        if pair in pairs:
            return "duplicate unordered pair relation"
        pairs.add(pair)
    return None


def _relation(state: AgendaState, first: str, second: str) -> PairRelation:
    """Resolve a declared pair relation in the requested orientation."""
    for item in state.constraints:
        if (item.first_id, item.second_id) == (first, second):
            if item.relation is PairRelation.SECOND_BEFORE_FIRST:
                return PairRelation.SECOND_BEFORE_FIRST
            return item.relation
        if (item.first_id, item.second_id) == (second, first):
            if item.relation is PairRelation.FIRST_BEFORE_SECOND:
                return PairRelation.SECOND_BEFORE_FIRST
            if item.relation is PairRelation.SECOND_BEFORE_FIRST:
                return PairRelation.FIRST_BEFORE_SECOND
            return item.relation
    return PairRelation.UNKNOWN


def publish(
    state: AgendaState,
    *,
    identity: str,
    generation: int,
    owner: str,
    at_common: int | Fraction,
    effect_id: str,
    microstep: int = 0,
    phase_order: int | None = None,
    phase_evidence: str | None = None,
    phase_basis: AgendaPhaseBasis | None = None,
) -> AgendaResult:
    """Publish an owned deadline and atomically fence any older scheduled generation."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if (
        not isinstance(identity, str) or not identity.strip()
        or not isinstance(owner, str) or not owner.strip()
        or not isinstance(effect_id, str) or not effect_id.strip()
        or type(generation) is not int or generation < 0
        or not _exact(at_common)
        or type(microstep) is not int or microstep < 0
        or (phase_order is not None and type(phase_order) is not int)
        or (phase_evidence is not None and (
            not isinstance(phase_evidence, str) or not phase_evidence.strip()
        ))
        or (phase_basis is not None and not _valid_phase_basis(phase_basis))
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid publish descriptor")
    if len(state.events) >= state.max_events:
        return AgendaResult(state, AgendaCode.RESOURCE_EXHAUSTED, detail="agenda capacity exhausted")
    prior = [item for item in state.events if item.identity == identity]
    if prior and generation <= max(item.generation for item in prior):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="generation must increase")
    if any(item.status is EventStatus.CLAIMED for item in prior):
        return AgendaResult(state, AgendaCode.BLOCKED, detail="claimed generation must be completed before replacement")
    events = list(state.events)
    heap = list(state.heap)
    for item in prior:
        if item.status is EventStatus.SCHEDULED:
            events[events.index(item)] = AgendaEvent(
                item.identity, item.generation, item.owner, item.at_common,
                item.effect_id, EventStatus.SUPERSEDED, item.microstep,
                item.phase_order, item.phase_evidence, item.phase_basis,
            )
            heap.remove(_key(item))
            heapq.heapify(heap)
    event = AgendaEvent(
        identity, generation, owner, Fraction(at_common), effect_id,
        EventStatus.SCHEDULED, microstep, phase_order, phase_evidence, phase_basis,
    )
    events.append(event)
    heapq.heappush(heap, _key(event))
    updated = AgendaState(tuple(heap), tuple(events), state.constraints, state.max_events)
    return AgendaResult(updated, AgendaCode.ACCEPTED, event)


def cancel(state: AgendaState, identity: str, generation: int) -> AgendaResult:
    """Cancel only a scheduled generation; claimed effects are irreversible."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    event = _find(state, identity, generation)
    if event is None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="unknown event generation")
    if event.status is EventStatus.CLAIMED:
        return AgendaResult(state, AgendaCode.LATE_INPUT, event, "claimed event cannot be cancelled")
    if event.status is not EventStatus.SCHEDULED:
        return AgendaResult(state, AgendaCode.BLOCKED, event, "event is not scheduled")
    updated_event = AgendaEvent(
        event.identity, event.generation, event.owner, event.at_common,
        event.effect_id, EventStatus.CANCELLED, event.microstep,
        event.phase_order, event.phase_evidence, event.phase_basis,
    )
    updated = _replace(state, updated_event)
    heap = list(updated.heap)
    heap.remove(_key(event))
    heapq.heapify(heap)
    updated = AgendaState(tuple(heap), updated.events, updated.constraints, updated.max_events)
    return AgendaResult(updated, AgendaCode.CANCELLED, updated_event)


def _select_same_instant(state: AgendaState, group: tuple[AgendaEvent, ...]) -> AgendaEvent | None:
    """Choose a causally minimal member; unresolved or cyclic ties stay UNKNOWN."""
    if len(group) == 1:
        return group[0]
    incoming = {item.identity: 0 for item in group}
    edges: dict[str, set[str]] = {item.identity: set() for item in group}
    ids = [item.identity for item in group]
    events_by_id = {item.identity: item for item in group}
    for index, first in enumerate(ids):
        for second in ids[index + 1:]:
            relation = _relation(state, first, second)
            if relation is PairRelation.UNKNOWN:
                relation = _phase_relation(events_by_id[first], events_by_id[second])
            if relation is PairRelation.UNKNOWN:
                return None
            if (
                relation is PairRelation.FIRST_BEFORE_SECOND
                and events_by_id[first].microstep > events_by_id[second].microstep
            ) or (
                relation is PairRelation.SECOND_BEFORE_FIRST
                and events_by_id[second].microstep > events_by_id[first].microstep
            ):
                return None
            if relation is PairRelation.FIRST_BEFORE_SECOND:
                edges[first].add(second)
                incoming[second] += 1
            elif relation is PairRelation.SECOND_BEFORE_FIRST:
                edges[second].add(first)
                incoming[first] += 1
    ready = sorted(identity for identity, count in incoming.items() if count == 0)
    if not ready:
        return None
    if len(ready) > 1:
        for index, first in enumerate(ready):
            for second in ready[index + 1:]:
                if _relation(state, first, second) is not PairRelation.COMMUTATIVE:
                    return None
    return next(item for item in group if item.identity == ready[0])


def claim_next(state: AgendaState, target_common: int | Fraction) -> AgendaResult:
    """Claim the earliest due event without imposing insertion-order precedence."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if not _exact(target_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="target must be exact")
    target = Fraction(target_common)
    if state.heap and target < state.heap[0][0]:
        return AgendaResult(state, AgendaCode.BLOCKED, detail="no event due at target")
    if not state.heap or state.heap[0][0] > target:
        return AgendaResult(state, AgendaCode.BLOCKED, detail="no scheduled event is due")
    heap = list(state.heap)
    earliest = heap[0][0]
    same_instant_keys = []
    while heap and heap[0][0] == earliest:
        same_instant_keys.append(heapq.heappop(heap))
    group = tuple(
        _find(state, identity, generation)
        for _, identity, generation in same_instant_keys
    )
    if any(item is None for item in group):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="heap references a missing event")
    selected = _select_same_instant(state, group)
    if selected is None:
        return AgendaResult(state, AgendaCode.INDETERMINATE, detail="same-instant event relation is UNKNOWN or cyclic")
    for key in same_instant_keys:
        if key != _key(selected):
            heapq.heappush(heap, key)
    claimed = AgendaEvent(
        selected.identity, selected.generation, selected.owner,
        selected.at_common, selected.effect_id, EventStatus.CLAIMED,
        selected.microstep, selected.phase_order,
        selected.phase_evidence, selected.phase_basis,
    )
    events = tuple(
        claimed if (item.identity, item.generation) ==
        (selected.identity, selected.generation) else item
        for item in state.events
    )
    updated = AgendaState(tuple(heap), events, state.constraints, state.max_events)
    return AgendaResult(updated, AgendaCode.CLAIMED, claimed)


def complete(state: AgendaState, identity: str, generation: int) -> AgendaResult:
    """Complete a claimed event generation exactly once."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    event = _find(state, identity, generation)
    if event is None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="unknown event generation")
    if event.status is not EventStatus.CLAIMED:
        return AgendaResult(state, AgendaCode.BLOCKED, event, "only a claimed event can complete")
    completed = AgendaEvent(
        event.identity, event.generation, event.owner, event.at_common,
        event.effect_id, EventStatus.COMPLETED, event.microstep,
        event.phase_order, event.phase_evidence, event.phase_basis,
    )
    updated = _replace(state, completed)
    return AgendaResult(updated, AgendaCode.COMPLETED, completed)
