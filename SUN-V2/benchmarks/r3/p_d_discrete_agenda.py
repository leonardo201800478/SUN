"""Disposable P-D candidate: central discrete agenda.

The agenda owns event ordering only. Device state remains outside this module.
All temporal coordinates are exact; insertion order never resolves UNKNOWN ties.
"""
from __future__ import annotations

import heapq
from dataclasses import dataclass, field, replace
from enum import Enum
from fractions import Fraction
from typing import Callable

from r3_transaction_core import (
    TransactionBook,
    TxCode,
    TxResult,
    _validate_book,
)


class AgendaCode(str, Enum):
    ACCEPTED = "Accepted"
    CLAIMED = "Claimed"
    CANCELLED = "Cancelled"
    COMPLETED = "Completed"
    BLOCKED = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    STALE_IGNORED = "StaleIgnored"


class EventStatus(str, Enum):
    SCHEDULED = "Scheduled"
    CANCELLED = "Cancelled"
    CANCEL_PENDING = "CancelPending"
    SUPERSEDED = "Superseded"
    CLAIMED = "Claimed"
    DISPATCHED = "Dispatched"
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
class AgendaObligation:
    """Known causal boundary that must not be crossed while unresolved."""

    identity: str
    owner: str
    at_common: Fraction
    resolved: bool = False
    commutative: bool = False


@dataclass(frozen=True)
class AgendaContext:
    participants: tuple[AgendaParticipant, ...]
    expected_participants: frozenset[str]
    watermark_common: Fraction | None = None
    obligations: tuple[AgendaObligation, ...] = ()


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
    max_microsteps: int = 32
    transactions: TransactionBook = field(default_factory=TransactionBook)


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


def _context_view(
    state: AgendaState, context: AgendaContext
) -> tuple[AgendaCode | None, str, dict[str, AgendaParticipant], Fraction, Fraction]:
    """Validate the participant contract and derive admission/safe frontiers."""
    if (
        not isinstance(context, AgendaContext)
        or type(context.participants) is not tuple
        or type(context.expected_participants) is not frozenset
        or type(context.obligations) is not tuple
        or any(not isinstance(item, str) or not item.strip()
               for item in context.expected_participants)
        or (
            context.watermark_common is not None
            and not _exact(context.watermark_common)
        )
    ):
        return AgendaCode.INVALID_INPUT, "invalid temporal context", {}, Fraction(0), Fraction(0)
    if context.watermark_common != state.watermark_common:
        return AgendaCode.INDETERMINATE, "context watermark differs from agenda watermark", {}, Fraction(0), Fraction(0)
    obligation_ids = set()
    for item in context.obligations:
        if (
            not isinstance(item, AgendaObligation)
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.owner, str) or not item.owner.strip()
            or not _exact(item.at_common)
            or type(item.resolved) is not bool
            or type(item.commutative) is not bool
            or item.identity in obligation_ids
        ):
            return AgendaCode.INVALID_INPUT, "invalid or duplicate causal obligation", {}, Fraction(0), Fraction(0)
        obligation_ids.add(item.identity)
    if not context.expected_participants:
        return AgendaCode.BLOCKED, "expected participant set is empty", {}, Fraction(0), Fraction(0)
    participants: dict[str, AgendaParticipant] = {}
    for item in context.participants:
        if (
            not isinstance(item, AgendaParticipant)
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not _exact(item.progress_common)
            or not _exact(item.safe_horizon_common)
            or item.identity in participants
        ):
            return AgendaCode.INVALID_INPUT, "invalid or duplicate participant", {}, Fraction(0), Fraction(0)
        if item.safe_horizon_common < item.progress_common:
            return AgendaCode.INVARIANT_VIOLATION, "participant safe horizon regressed behind progress", {}, Fraction(0), Fraction(0)
        if item.identity not in context.expected_participants:
            return AgendaCode.INVALID_INPUT, "undeclared participant", {}, Fraction(0), Fraction(0)
        participants[item.identity] = item
    missing = context.expected_participants - participants.keys()
    if missing:
        return AgendaCode.BLOCKED, "required participant progress is incomplete", participants, Fraction(0), Fraction(0)
    progress = max(item.progress_common for item in participants.values())
    frontier = max(
        progress,
        context.watermark_common if context.watermark_common is not None else progress,
    )
    safe = min(item.safe_horizon_common for item in participants.values())
    if safe < frontier:
        return AgendaCode.INVARIANT_VIOLATION, "safe horizon is behind the admission frontier", participants, frontier, safe
    if context.watermark_common is not None and any(
        not item.resolved and item.at_common <= context.watermark_common
        for item in context.obligations
    ):
        return AgendaCode.INVARIANT_VIOLATION, "unresolved obligation is at or behind the confirmed watermark", participants, frontier, safe
    if any(
        not item.resolved and item.at_common < frontier
        for item in context.obligations
    ):
        return AgendaCode.INVARIANT_VIOLATION, "unresolved obligation lies behind the admission frontier", participants, frontier, safe
    return None, "", participants, frontier, safe


def _cancellation_precedes(
    request: AgendaCancellation, deadline: AgendaEvent
) -> bool | None:
    """Return whether a same-instant cancellation is proven before its deadline."""
    if request.microstep_level is not None:
        if request.microstep_level != deadline.microstep:
            return request.microstep_level < deadline.microstep
    if (
        request.phase_order is not None
        and request.phase_evidence is not None
        and request.phase_basis is not None
        and deadline.phase_order is not None
        and deadline.phase_evidence is not None
        and deadline.phase_basis == request.phase_basis
        and _valid_phase_basis(request.phase_basis)
        and request.phase_order != deadline.phase_order
    ):
        return request.phase_order < deadline.phase_order
    return None


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


def apply_transaction(
    state: AgendaState, operation: Callable, *args, **kwargs
) -> tuple[AgendaState, TxResult]:
    """Apply one shared W08 operation and carry its immutable book in P-D state."""
    if not callable(operation):
        result = TxResult(
            TxCode.INVALID_INPUT, state.transactions,
            detail="transaction operation must be callable",
        )
        return state, result
    result = operation(state.transactions, *args, **kwargs)
    if not isinstance(result, TxResult):
        invalid = TxResult(
            TxCode.INVALID_INPUT, state.transactions,
            detail="transaction operation returned an invalid result",
        )
        return state, invalid
    invalid_book = _validate_book(result.book)
    if invalid_book is not None:
        invalid = TxResult(
            TxCode.INVALID_INPUT, state.transactions,
            detail="transaction operation produced an invalid book: " + invalid_book,
        )
        return state, invalid
    return replace(state, transactions=result.book), result


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
        state.max_microsteps, state.transactions,
    )


def _validate(state: object) -> str | None:
    """Validate agenda, heap, generations and pairwise constraints atomically."""
    if not isinstance(state, AgendaState):
        return "invalid agenda state"
    if type(state.max_events) is not int or state.max_events <= 0:
        return "max_events must be a positive integer"
    if type(state.max_microsteps) is not int or state.max_microsteps <= 0:
        return "max_microsteps must be a positive integer"
    invalid_transactions = _validate_book(state.transactions)
    if invalid_transactions is not None:
        return "invalid transaction book: " + invalid_transactions
    if state.watermark_common is not None and not _exact(state.watermark_common):
        return "watermark must be an exact coordinate"
    if type(state.pending_cancellations) is not tuple:
        return "pending cancellations must be an immutable tuple"
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
            or type(event.microstep) is not int
            or event.microstep < 0 or event.microstep > state.max_microsteps
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
    events_by_key = {
        (item.identity, item.generation): item for item in state.events
    }
    pending_keys: set[tuple[str, int]] = set()
    for request in state.pending_cancellations:
        if (
            not isinstance(request, AgendaCancellation)
            or not isinstance(request.identity, str) or not request.identity.strip()
            or not isinstance(request.owner, str) or not request.owner.strip()
            or type(request.generation) is not int or request.generation < 0
            or not _exact(request.at_common)
            or (request.microstep_level is not None and (
                type(request.microstep_level) is not int
                or request.microstep_level < 0
                or request.microstep_level > state.max_microsteps
            ))
            or (request.phase_order is not None and type(request.phase_order) is not int)
            or (request.phase_evidence is not None and (
                not isinstance(request.phase_evidence, str)
                or not request.phase_evidence.strip()
            ))
            or ((request.phase_order is None) != (request.phase_evidence is None))
            or (request.phase_basis is not None and (
                not _valid_phase_basis(request.phase_basis)
                or request.phase_order is None or request.phase_evidence is None
            ))
        ):
            return "invalid pending cancellation descriptor"
        key = (request.identity, request.generation)
        if key in pending_keys:
            return "duplicate pending cancellation"
        pending_keys.add(key)
        event = events_by_key.get(key)
        if (
            event is None
            or event.status is not EventStatus.CANCEL_PENDING
            or event.owner != request.owner
            or request.at_common > event.at_common
            or (
                state.watermark_common is not None
                and state.watermark_common >= event.at_common
            )
        ):
            return "pending cancellation and event record disagree"
    processed_statuses = (
        EventStatus.DISPATCHED,
        EventStatus.COMPLETED,
    )
    if any(
        event.status in processed_statuses
        and (
            state.watermark_common is None
            or event.at_common > state.watermark_common
        )
        for event in state.events
    ):
        return "dispatched or completed event is beyond the closed watermark"
    if state.watermark_common is not None and any(
        event.status is EventStatus.CLAIMED
        and event.at_common > state.watermark_common
        for event in state.events
    ):
        return "claimed event is beyond the closed watermark"
    active_statuses = {
        EventStatus.SCHEDULED,
        EventStatus.CANCEL_PENDING,
        EventStatus.CLAIMED,
        EventStatus.DISPATCHED,
    }
    if any(
        event.generation < max_generation[event.identity]
        and event.status in active_statuses
        for event in state.events
    ):
        return "older event generation remains active after a newer generation"
    for event in state.events:
        if event.status is EventStatus.CANCEL_PENDING and (
            event.identity, event.generation
        ) not in pending_keys:
            return "cancel-pending event has no cancellation record"
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
    if microstep > state.max_microsteps:
        return AgendaResult(state, AgendaCode.RESOURCE_EXHAUSTED, detail="microstep limit exceeded")
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
    if any(item.status is EventStatus.CANCEL_PENDING for item in prior):
        return AgendaResult(
            state, AgendaCode.INVALID_INPUT,
            detail="prior generation cancellation must be arbitrated before replacement",
        )
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
        state.watermark_common, state.pending_cancellations,
        state.max_microsteps, state.transactions,
    )
    return AgendaResult(updated, AgendaCode.ACCEPTED, event)


def publish_with_context(
    state: AgendaState,
    context: AgendaContext,
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
    """Publish only against a complete participant contract and matching watermark."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    code, detail, participants, _, _ = _context_view(state, context)
    if code is not None:
        return AgendaResult(state, code, detail=detail)
    if owner not in context.expected_participants:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="deadline owner is not declared")
    participant = participants.get(owner)
    if participant is None:
        return AgendaResult(state, AgendaCode.BLOCKED, detail="deadline owner progress is missing")
    if not _exact(at_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="deadline coordinate must be exact")
    if (
        state.watermark_common is not None
        and Fraction(at_common) <= state.watermark_common
    ):
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="deadline is at or behind the closed watermark")
    if Fraction(at_common) < participant.progress_common:
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="deadline is behind owner progress")
    if Fraction(at_common) == participant.progress_common and microstep == 0:
        return AgendaResult(state, AgendaCode.INDETERMINATE, detail="same-frontier publication requires an explicit microstep")
    if type(microstep) is not int or microstep < 0:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid microstep")
    if microstep > state.max_microsteps:
        return AgendaResult(state, AgendaCode.RESOURCE_EXHAUSTED, detail="microstep limit exceeded")
    if (
        isinstance(identity, str) and identity.strip()
        and type(generation) is int and generation >= 0
    ):
        prior = tuple(item for item in state.events if item.identity == identity)
        if prior and generation > max(item.generation for item in prior):
            if any(item.status in (
                EventStatus.SCHEDULED,
                EventStatus.CANCEL_PENDING,
                EventStatus.CLAIMED,
            ) for item in prior):
                return AgendaResult(
                    state, AgendaCode.INVALID_INPUT,
                    detail="prior generation is not terminal",
                )
    return publish(
        state,
        identity=identity,
        generation=generation,
        owner=owner,
        at_common=at_common,
        effect_id=effect_id,
        microstep=microstep,
        phase_order=phase_order,
        phase_evidence=phase_evidence,
        phase_basis=phase_basis,
    )


def request_cancel(
    state: AgendaState,
    context: AgendaContext,
    *,
    owner: str,
    identity: str,
    generation: int,
    at_common: int | Fraction,
    microstep_level: int | None = None,
    phase_order: int | None = None,
    phase_evidence: str | None = None,
    phase_basis: AgendaPhaseBasis | None = None,
) -> AgendaResult:
    """Admit a cancellation only when it is ahead of A(E) and before the deadline."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    code, detail, participants, frontier, _ = _context_view(state, context)
    if code is not None:
        return AgendaResult(state, code, detail=detail)
    if not isinstance(owner, str) or not owner.strip():
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid cancellation owner")
    if owner not in context.expected_participants:
        return AgendaResult(state, AgendaCode.BLOCKED, detail="cancellation owner is absent from the declared participant set")
    if (
        not isinstance(identity, str) or not identity.strip()
        or type(generation) is not int or generation < 0
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid deadline identity or generation")
    if not _exact(at_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="cancellation coordinate must be exact")
    if (
        microstep_level is not None
        and (type(microstep_level) is not int or microstep_level < 0)
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid cancellation microstep")
    if microstep_level is not None and microstep_level > state.max_microsteps:
        return AgendaResult(state, AgendaCode.RESOURCE_EXHAUSTED, detail="microstep limit exceeded")
    if (phase_order is None) != (phase_evidence is None):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="phase order and evidence must be supplied together")
    if phase_order is not None and (
        type(phase_order) is not int
        or not isinstance(phase_evidence, str)
        or not phase_evidence.strip()
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid cancellation phase descriptor")
    if phase_basis is not None and (
        not _valid_phase_basis(phase_basis)
        or phase_order is None or phase_evidence is None
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid cancellation phase basis")
    event = _find(state, identity, generation)
    if event is None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="unknown deadline generation")
    if event.owner != owner:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, event, "cancellation owner does not own deadline")
    if event.status is not EventStatus.SCHEDULED:
        return AgendaResult(state, AgendaCode.LATE_INPUT, event, "deadline is not cancellable")
    if Fraction(at_common) <= frontier:
        return AgendaResult(state, AgendaCode.LATE_INPUT, event, "cancellation is at or behind A(E)")
    if (
        state.watermark_common is not None
        and state.watermark_common >= event.at_common
    ):
        return AgendaResult(state, AgendaCode.LATE_INPUT, event, "deadline is at or behind the closed watermark")
    if Fraction(at_common) > event.at_common:
        return AgendaResult(state, AgendaCode.LATE_INPUT, event, "cancellation is after the deadline")

    request = AgendaCancellation(
        identity, generation, owner, Fraction(at_common), microstep_level,
        phase_order, phase_evidence, phase_basis,
    )
    if Fraction(at_common) == event.at_common:
        precedes = _cancellation_precedes(request, event)
        if precedes is None:
            return AgendaResult(state, AgendaCode.INDETERMINATE, event, "same-instant cancellation order is UNKNOWN")
        if not precedes:
            return AgendaResult(state, AgendaCode.LATE_INPUT, event, "deadline precedes cancellation")

    updated_event = AgendaEvent(
        event.identity, event.generation, event.owner, event.at_common,
        event.effect_id, EventStatus.CANCEL_PENDING, event.microstep,
        event.phase_order, event.phase_evidence, event.phase_basis,
    )
    events = tuple(
        updated_event if (item.identity, item.generation) == (identity, generation) else item
        for item in state.events
    )
    heap_items = [item for item in state.heap if item != _key(event)]
    heapq.heapify(heap_items)
    heap = tuple(heap_items)
    pending = tuple(sorted(
        state.pending_cancellations + (request,),
        key=lambda item: (item.at_common, item.identity, item.generation),
    ))
    updated = AgendaState(
        heap, events, state.constraints, state.max_events,
        state.watermark_common, pending, state.max_microsteps,
        state.transactions,
    )
    return AgendaResult(updated, AgendaCode.ACCEPTED, updated_event)


def arbitrate_cancellations(state: AgendaState) -> AgendaResult:
    """Commit every admitted pre-deadline cancellation before watermark closure."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if not state.pending_cancellations:
        return AgendaResult(state, AgendaCode.ACCEPTED, detail="no pending cancellations")
    replacements: dict[tuple[str, int], AgendaEvent] = {}
    for request in state.pending_cancellations:
        event = _find(state, request.identity, request.generation)
        if event is None or event.status is not EventStatus.CANCEL_PENDING:
            return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="pending cancellation lost its event")
        if request.at_common > event.at_common:
            return AgendaResult(state, AgendaCode.LATE_INPUT, event, "cancellation is after the deadline")
        if request.at_common == event.at_common:
            precedes = _cancellation_precedes(request, event)
            if precedes is None:
                return AgendaResult(state, AgendaCode.INDETERMINATE, event, "same-instant cancellation order is UNKNOWN")
            if not precedes:
                return AgendaResult(state, AgendaCode.LATE_INPUT, event, "deadline precedes cancellation")
        replacements[(event.identity, event.generation)] = AgendaEvent(
            event.identity, event.generation, event.owner, event.at_common,
            event.effect_id, EventStatus.CANCELLED, event.microstep,
            event.phase_order, event.phase_evidence, event.phase_basis,
        )
    events = tuple(
        replacements.get((item.identity, item.generation), item)
        for item in state.events
    )
    updated = AgendaState(
        state.heap, events, state.constraints, state.max_events,
        state.watermark_common, (), state.max_microsteps, state.transactions,
    )
    return AgendaResult(updated, AgendaCode.ACCEPTED, detail="pending cancellations arbitrated")


def replace_with_context(
    state: AgendaState,
    context: AgendaContext,
    *,
    owner: str,
    identity: str,
    generation: int,
    cancel_at: int | Fraction,
    new_generation: int,
    new_at_common: int | Fraction,
    new_effect_id: str,
    microstep: bool = False,
) -> AgendaResult:
    """Atomically cancel/arbitrate an old generation and publish its successor.

    The operation mirrors the P-A/R2 replacement contract: cancellation must
    be admissible and arbitrated before the replacement is published. Any
    failure returns the original state, so a rejected replacement cannot
    leave the old generation cancelled.
    """
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or type(generation) is not int or generation < 0
        or type(new_generation) is not int or new_generation <= generation
        or not _exact(cancel_at) or not _exact(new_at_common)
        or not isinstance(new_effect_id, str) or not new_effect_id.strip()
        or not isinstance(microstep, bool)
    ):
        return AgendaResult(
            state, AgendaCode.INVALID_INPUT,
            detail="invalid replacement descriptor",
        )

    prior = tuple(item for item in state.events if item.identity == identity)
    if prior and new_generation <= max(item.generation for item in prior):
        return AgendaResult(
            state, AgendaCode.INVALID_INPUT,
            detail="replacement generation must increase monotonically",
        )

    cancelled = request_cancel(
        state, context, owner=owner, identity=identity,
        generation=generation, at_common=cancel_at,
    )
    if cancelled.code is not AgendaCode.ACCEPTED:
        return AgendaResult(
            state, cancelled.code, cancelled.event, cancelled.detail,
        )

    arbitrated = arbitrate_cancellations(cancelled.state)
    if arbitrated.code is not AgendaCode.ACCEPTED:
        return AgendaResult(state, arbitrated.code, detail=arbitrated.detail)

    published = publish_with_context(
        arbitrated.state, context, owner=owner, identity=identity,
        generation=new_generation, at_common=new_at_common,
        effect_id=new_effect_id, microstep=1 if microstep else 0,
    )
    if published.code is not AgendaCode.ACCEPTED:
        return AgendaResult(
            state, published.code,
            detail=f"replacement rejected atomically: {published.detail}",
        )

    if Fraction(new_at_common) < Fraction(cancel_at):
        return AgendaResult(
            state, AgendaCode.LATE_INPUT,
            detail="replacement deadline precedes replacement operation",
        )
    if Fraction(new_at_common) == Fraction(cancel_at):
        return AgendaResult(
            state, AgendaCode.INDETERMINATE,
            detail="replacement deadline coincides with cancellation without explicit causal relation",
        )
    return AgendaResult(
        published.state, AgendaCode.ACCEPTED, published.event,
        "old generation cancelled and replacement published atomically",
    )


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
        updated.watermark_common, updated.pending_cancellations,
        updated.max_microsteps, updated.transactions,
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
    for request in state.pending_cancellations:
        event = _find(state, request.identity, request.generation)
        if event is not None and (
            request.at_common <= target or event.at_common <= target
        ):
            return AgendaResult(state, AgendaCode.BLOCKED, event, "pending cancellation must be arbitrated before claim")
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
        state.watermark_common, state.pending_cancellations,
        state.max_microsteps, state.transactions,
    )
    return AgendaResult(updated, AgendaCode.CLAIMED, claimed)


def close_watermark(
    state: AgendaState,
    target_common: int | Fraction,
    context: AgendaContext | None = None,
) -> AgendaResult:
    """Atomically claim due events and close one exact, optionally bounded frontier."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if not _exact(target_common):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="watermark target must be exact")
    target = Fraction(target_common)
    if state.watermark_common is not None and target < state.watermark_common:
        return AgendaResult(state, AgendaCode.LATE_INPUT, detail="watermark cannot regress")
    if context is not None:
        code, detail, _, frontier, safe = _context_view(state, context)
        if code is not None:
            return AgendaResult(state, code, detail=detail)
        if target < frontier:
            return AgendaResult(state, AgendaCode.LATE_INPUT, detail="watermark is behind participant progress")
        if target > safe:
            return AgendaResult(state, AgendaCode.BLOCKED, detail="watermark exceeds the global safe horizon")
    for request in state.pending_cancellations:
        event = _find(state, request.identity, request.generation)
        # C may already be behind W while D remains ahead. The admitted
        # cancellation must be arbitrated before W crosses D, not before W
        # crosses the request coordinate C.
        if event is not None and event.at_common <= target:
            return AgendaResult(state, AgendaCode.BLOCKED, event, "pending cancellation must be arbitrated before watermark closes its deadline")

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
        tuple(heap), events, state.constraints, state.max_events, target,
        state.pending_cancellations, state.max_microsteps,
        state.transactions,
    )
    return AgendaResult(
        updated, AgendaCode.ACCEPTED,
        detail="all due agenda events claimed before watermark close",
        claimed=tuple(claimed_order),
    )


def dispatch_event(state: AgendaState, identity: str, generation: int) -> AgendaResult:
    """Dispatch only a claimed current generation; repeated dispatch is idempotent."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if (
        not isinstance(identity, str) or not identity.strip()
        or type(generation) is not int or generation < 0
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid dispatch identity or generation")
    event = _find(state, identity, generation)
    if event is None:
        return AgendaResult(state, AgendaCode.STALE_IGNORED, detail="stale identity or generation ignored")
    if event.status in (EventStatus.DISPATCHED, EventStatus.COMPLETED):
        return AgendaResult(state, AgendaCode.ACCEPTED, event, "dispatch already recorded; no repeated effect")
    if event.status is EventStatus.CLAIMED and (
        state.watermark_common is None
        or event.at_common > state.watermark_common
    ):
        return AgendaResult(
            state, AgendaCode.INVARIANT_VIOLATION, event,
            "dispatch requires the event to be claimed under a closed watermark",
        )
    if event.status is not EventStatus.CLAIMED:
        return AgendaResult(state, AgendaCode.STALE_IGNORED, event, "event is not claimed; no effect dispatched")
    dispatched = AgendaEvent(
        event.identity, event.generation, event.owner, event.at_common,
        event.effect_id, EventStatus.DISPATCHED, event.microstep,
        event.phase_order, event.phase_evidence, event.phase_basis,
    )
    updated = _replace(state, dispatched)
    return AgendaResult(updated, AgendaCode.ACCEPTED, dispatched, "claimed event dispatched exactly once")


def complete(state: AgendaState, identity: str, generation: int) -> AgendaResult:
    """Complete a dispatched event generation exactly once."""
    invalid = _validate(state)
    if invalid is not None:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail=invalid)
    if (
        not isinstance(identity, str) or not identity.strip()
        or type(generation) is not int or generation < 0
    ):
        return AgendaResult(state, AgendaCode.INVALID_INPUT, detail="invalid completion identity or generation")
    event = _find(state, identity, generation)
    if event is None or event.status is not EventStatus.DISPATCHED:
        return AgendaResult(state, AgendaCode.INVALID_INPUT, event, "only a dispatched generation can complete")
    completed = AgendaEvent(
        event.identity, event.generation, event.owner, event.at_common,
        event.effect_id, EventStatus.COMPLETED, event.microstep,
        event.phase_order, event.phase_evidence, event.phase_basis,
    )
    updated = _replace(state, completed)
    return AgendaResult(updated, AgendaCode.ACCEPTED, completed)
