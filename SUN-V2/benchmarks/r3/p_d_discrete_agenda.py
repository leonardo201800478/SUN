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
    CANCEL_PENDING = "CancelPending"
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
class AgendaParticipant:
    identity: str
    progress_common: Fraction
    safe_horizon_common: Fraction


@dataclass(frozen=True)
class AgendaContext:
    participants: tuple[AgendaParticipant, ...]
    expected_participants: frozenset[str]
    watermark_common: Fraction | None = None


@dataclass(frozen=True)
class AgendaCancellation:
    identity: str
    generation: int
    owner: str
    at_common: Fraction
    microstep_level: int | None = None
    phase_order: int | None = None
    phase_evidence: str | None = None
    phase_basis: AgendaPhaseBasis | None = None


@dataclass(frozen=True)
class AgendaState:
    heap: tuple[tuple[Fraction, str, int], ...] = ()
    events: tuple[AgendaEvent, ...] = ()
    constraints: tuple[AgendaConstraint, ...] = ()
    max_events: int = 4096
    watermark_common: Fraction | None = None
    pending_cancellations: tuple[AgendaCancellation, ...] = ()


@dataclass(frozen=True)
class AgendaResult:
    state: AgendaState
    code: AgendaCode
    event: AgendaEvent | None = None
    detail: str = ""
    claimed: tuple[tuple[str, int], ...] = ()


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
        and all(char in "0123456789abcdef"
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
    return AgendaState(
        state.heap, events, state.constraints, state.max_events,
        state.watermark_common, state.pending_cancellations,
    )


def _validate(state: object) -> str | None:
    """Validate agenda, heap, generations and pairwise constraints atomically."""
    if not isinstance(state, AgendaState):
        return "invalid agenda state"
    if type(state.max_events) is not int or state.max_events <= 0:
        return "max_events must be a positive integer"
    if state.watermark_common is not None and not _exact(state.watermark_common):
        return "watermark must be an exact coordinate"
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
            or ((event.phase_order is None) != (event.phase_evidence is None))
            or (event.phase_basis is not None and (
                not _valid_phase_basis(event.phase_basis)
                or event.phase_order is None or event.phase_evidence is None
            ))
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
            if (
                state.watermark_common is not None
                and event.at_common <= state.watermark_common
            ):
                return "scheduled event is at or behind the closed watermark"
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
        or ((phase_order is None) != (phase_evidence is None))
        or (phase_basis is not None and (
            not _valid_phase_basis(phase_basis)
            or phase_order is None or phase_evidence is None
        ))
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid publish descriptor")
    if (
        state.watermark_common is not None
        and Fraction(at_common) <= state.watermark_common
    ):
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="deadline is at or behind the closed watermark")
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
    updated = AgendaState(
        tuple(heap), tuple(events), state.constraints, state.max_events,
        state.watermark_common,
    )
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
    updated = AgendaState(
        tuple(heap), updated.events, updated.constraints, updated.max_events,
        updated.watermark_common,
    )
    return AgendaResult(updated, AgendaCode.CANCELLED, updated_event)


def _ordered_same_instant(
    state: AgendaState,
    group: tuple[AgendaEvent, ...],
    relation_index: dict[frozenset[str], AgendaConstraint] | None = None,
) -> tuple[AgendaEvent, ...] | None:
    """Topologically order one timestamp group using explicit pair relations."""
    if len(group) <= 1:
        return group
    if relation_index is None:
        relation_index = {
            frozenset((item.first_id, item.second_id)): item
            for item in state.constraints
        }
    by_id = {item.identity: item for item in group}
    ids = tuple(by_id)
    incoming = {identity: 0 for identity in ids}
    edges: dict[str, set[str]] = {identity: set() for identity in ids}
    pair_relations: dict[frozenset[str], PairRelation] = {}

    def declared_relation(first_id: str, second_id: str) -> PairRelation:
        item = relation_index.get(frozenset((first_id, second_id)))
        if item is None:
            return PairRelation.UNKNOWN
        if (item.first_id, item.second_id) == (first_id, second_id):
            return item.relation
        if item.relation is PairRelation.FIRST_BEFORE_SECOND:
            return PairRelation.SECOND_BEFORE_FIRST
        if item.relation is PairRelation.SECOND_BEFORE_FIRST:
            return PairRelation.FIRST_BEFORE_SECOND
        return item.relation

    for index, first_id in enumerate(ids):
        for second_id in ids[index + 1:]:
            first = by_id[first_id]
            second = by_id[second_id]
            relation = declared_relation(first_id, second_id)
            if relation is PairRelation.UNKNOWN:
                relation = _phase_relation(first, second)
            if relation is PairRelation.UNKNOWN:
                return None
            if (
                relation is PairRelation.FIRST_BEFORE_SECOND
                and first.microstep > second.microstep
            ) or (
                relation is PairRelation.SECOND_BEFORE_FIRST
                and second.microstep > first.microstep
            ):
                return None
            pair_relations[frozenset((first_id, second_id))] = relation
            if relation is PairRelation.FIRST_BEFORE_SECOND:
                edges[first_id].add(second_id)
                incoming[second_id] += 1
            elif relation is PairRelation.SECOND_BEFORE_FIRST:
                edges[second_id].add(first_id)
                incoming[first_id] += 1

    ready = sorted(identity for identity, count in incoming.items() if count == 0)
    ordered: list[AgendaEvent] = []
    while ready:
        if len(ready) > 1:
            for index, first_id in enumerate(ready):
                for second_id in ready[index + 1:]:
                    if pair_relations.get(
                        frozenset((first_id, second_id))
                    ) is not PairRelation.COMMUTATIVE:
                        return None
        selected = ready.pop(0)
        ordered.append(by_id[selected])
        for successor in sorted(edges[selected]):
            incoming[successor] -= 1
            if incoming[successor] == 0:
                ready.append(successor)
        ready.sort()

    if len(ordered) != len(group):
        return None
    return tuple(ordered)


def _select_same_instant(state: AgendaState, group: tuple[AgendaEvent, ...]) -> AgendaEvent | None:
    """Choose the first causally ordered member; unresolved/cyclic ties stay UNKNOWN."""
    ordered = _ordered_same_instant(state, group)
    return ordered[0] if ordered else None

def claim_next(state: AgendaState, target_common: int | Fraction) -> AgendaResult:
    """Claim the earliest due event without imposing insertion-order precedence."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if not _exact(target_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="target must be exact")
    target = Fraction(target_common)
    if state.watermark_common is not None and target < state.watermark_common:
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="target is behind the closed watermark")
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
    updated = AgendaState(
        tuple(heap), events, state.constraints, state.max_events,
        state.watermark_common,
    )
    return AgendaResult(updated, AgendaCode.CLAIMED, claimed)


def close_watermark(state: AgendaState, target_common: int | Fraction) -> AgendaResult:
    """Atomically claim every due event and close one exact common-time frontier."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if not _exact(target_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="watermark target must be exact")
    target = Fraction(target_common)
    if state.watermark_common is not None and target < state.watermark_common:
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="watermark cannot regress")

    heap = list(state.heap)
    by_key = {
        (item.identity, item.generation): item
        for item in state.events
    }
    claimed_events: dict[tuple[str, int], AgendaEvent] = {}
    claimed_order: list[tuple[str, int]] = []
    relation_index = {
        frozenset((item.first_id, item.second_id)): item
        for item in state.constraints
    }
    while heap and heap[0][0] <= target:
        earliest = heap[0][0]
        keys = []
        while heap and heap[0][0] == earliest:
            keys.append(heapq.heappop(heap))
        group = tuple(
            by_key.get((identity, generation))
            for _, identity, generation in keys
        )
        if any(item is None for item in group):
            return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="heap references a missing event")
        ordered = _ordered_same_instant(state, group, relation_index)
        if ordered is None:
            return AgendaResult(
                state, AgendaCode.INDETERMINATE,
                detail="same-instant event relation is UNKNOWN or cyclic",
            )
        for selected in ordered:
            claimed = AgendaEvent(
                selected.identity, selected.generation, selected.owner,
                selected.at_common, selected.effect_id, EventStatus.CLAIMED,
                selected.microstep, selected.phase_order,
                selected.phase_evidence, selected.phase_basis,
            )
            key = (selected.identity, selected.generation)
            claimed_events[key] = claimed
            claimed_order.append(key)

    events = tuple(
        claimed_events.get((item.identity, item.generation), item)
        for item in state.events
    )
    updated = AgendaState(
        tuple(heap), events, state.constraints, state.max_events, target
    )
    return AgendaResult(
        updated, AgendaCode.ACCEPTED,
        detail="all due agenda events claimed before watermark close",
        claimed=tuple(claimed_order),
    )


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
