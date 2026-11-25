"""Deterministic accepted-input and replay oracle for R2-T12."""
from __future__ import annotations
from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction


class ReplayCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INDETERMINATE = "IndeterminateCausalOrder"
    INVARIANT_VIOLATION = "InvariantViolation"


@dataclass(frozen=True)
class Participant:
    name: str
    progress_common: Fraction | None
    horizon_common: Fraction | None
    conversion_valid: bool = True


@dataclass(frozen=True)
class InputContract:
    input_class: str
    participants: tuple[str, ...]


@dataclass(frozen=True)
class ExternalInput:
    identity: str
    input_class: str
    owner: str
    at_common: Fraction
    payload: str
    participants: tuple[str, ...]
    logical_order: int | None = None
    commutative: bool = False


@dataclass(frozen=True)
class AcceptedInput:
    sequence: int
    event: ExternalInput


@dataclass(frozen=True)
class ReplayState:
    profile: str
    participants: tuple[Participant, ...]
    contracts: tuple[InputContract, ...]
    watermark: Fraction | None = None
    accepted: tuple[AcceptedInput, ...] = ()
    applied_ids: tuple[str, ...] = ()
    pending_obligations: tuple[Fraction, ...] = ()
    recalculate_required: tuple[str, ...] = ()
    virtual_effects: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class ReplayEvaluation:
    code: ReplayCode
    state: ReplayState
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational time and reject floating-point coordinates."""
    return type(value) is int or type(value) is Fraction


def _validate_event(event: object) -> str | None:
    """Validate an external event before looking up its temporal admission frontier."""
    if not isinstance(event, ExternalInput):
        return "invalid input event"
    if any(type(value) is not str or not value.strip() for value in (
        event.identity, event.input_class, event.owner, event.payload
    )):
        return "invalid event identity/payload"
    if not _exact(event.at_common):
        return "invalid event time"
    if (
        type(event.participants) is not tuple
        or not event.participants
        or any(type(name) is not str or not name.strip() for name in event.participants)
        or len(set(event.participants)) != len(event.participants)
    ):
        return "invalid event participants"
    if event.logical_order is not None and (
        type(event.logical_order) is not int or event.logical_order < 0
    ):
        return "invalid logical order"
    if type(event.commutative) is not bool:
        return "invalid commutativity declaration"
    return None


def _validate_state(state: object) -> str | None:
    """Validate participant contracts, admitted sequence, and replay ledgers."""
    if not isinstance(state, ReplayState) or type(state.profile) is not str or not state.profile.strip():
        return "invalid replay state/profile"
    if type(state.participants) is not tuple or type(state.contracts) is not tuple or type(state.accepted) is not tuple:
        return "invalid replay collections"
    if (
        type(state.recalculate_required) is not tuple
        or type(state.pending_obligations) is not tuple
        or type(state.applied_ids) is not tuple
        or type(state.virtual_effects) is not tuple
    ):
        return "invalid replay ledgers"
    if state.watermark is not None and not _exact(state.watermark):
        return "invalid watermark"
    names = set()
    for participant in state.participants:
        if (
            not isinstance(participant, Participant)
            or type(participant.name) is not str
            or not participant.name.strip()
            or participant.name in names
        ):
            return "invalid participant"
        names.add(participant.name)
        if participant.progress_common is not None and not _exact(participant.progress_common):
            return "invalid participant progress"
        if participant.horizon_common is not None and not _exact(participant.horizon_common):
            return "invalid participant horizon"
        if type(participant.conversion_valid) is not bool:
            return "invalid conversion validity"
    classes = set()
    for contract in state.contracts:
        if (
            not isinstance(contract, InputContract)
            or type(contract.input_class) is not str
            or not contract.input_class.strip()
            or contract.input_class in classes
        ):
            return "invalid input contract"
        classes.add(contract.input_class)
        if (
            type(contract.participants) is not tuple
            or not contract.participants
            or any(type(name) is not str or not name.strip() for name in contract.participants)
            or len(set(contract.participants)) != len(contract.participants)
        ):
            return "invalid contract participants"
    if any(type(name) is not str or name not in names for name in state.recalculate_required):
        return "invalid recalculation participant"
    if any(not _exact(value) for value in state.pending_obligations):
        return "invalid pending obligation"
    if any(type(identity) is not str for identity in state.applied_ids) or len(set(state.applied_ids)) != len(state.applied_ids):
        return "invalid applied input ledger"
    if any(
        not isinstance(item, AcceptedInput)
        or type(item.sequence) is not int
        or item.sequence < 1
        or _validate_event(item.event)
        for item in state.accepted
    ):
        return "invalid accepted input log"
    if tuple(item.sequence for item in state.accepted) != tuple(range(1, len(state.accepted) + 1)):
        return "non-contiguous accepted input sequence"
    accepted_ids = tuple(item.event.identity for item in state.accepted)
    if len(set(accepted_ids)) != len(accepted_ids) or not set(state.applied_ids).issubset(accepted_ids):
        return "invalid accepted/applied identity relation"
    if any(
        type(pair) is not tuple or len(pair) != 2
        or type(pair[0]) is not str or type(pair[1]) is not str
        for pair in state.virtual_effects
    ):
        return "invalid virtual effect ledger"
    return None


def _frontier(state: ReplayState, participants: tuple[str, ...]) -> tuple[ReplayCode, Fraction | None]:
    """Build A(E) only after all declared participants and conversions are available."""
    by_name = {participant.name: participant for participant in state.participants}
    if not participants or any(name not in by_name for name in participants):
        return ReplayCode.BLOCKED, None
    members = [by_name[name] for name in participants]
    if any(member.progress_common is None for member in members):
        return ReplayCode.BLOCKED, None
    if any(not member.conversion_valid for member in members):
        return ReplayCode.INDETERMINATE, None
    values = [member.progress_common for member in members]
    if state.watermark is not None:
        values.append(state.watermark)
    return ReplayCode.OK, max(values)


def accept_input(state: ReplayState, event: ExternalInput) -> ReplayEvaluation:
    """Admit an input using the complete A(E), then invalidate participant horizons."""
    invalid = _validate_state(state) or _validate_event(event)
    if invalid:
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, invalid)
    existing = next((item for item in state.accepted if item.event.identity == event.identity), None)
    if existing is not None:
        if existing.event != event:
            return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, "input ID reused with divergent descriptor")
        return ReplayEvaluation(ReplayCode.OK, state)
    contract = next((item for item in state.contracts if item.input_class == event.input_class), None)
    if contract is None or event.owner not in contract.participants or set(event.participants) != set(contract.participants):
        return ReplayEvaluation(ReplayCode.BLOCKED, state, "profile participant contract is incomplete or mismatched")
    code, frontier = _frontier(state, contract.participants)
    if code is not ReplayCode.OK:
        return ReplayEvaluation(code, state, "complete admission frontier unavailable")
    if event.at_common <= frontier:
        return ReplayEvaluation(ReplayCode.LATE_INPUT, state, "input is at/behind A(E)")
    accepted = AcceptedInput(len(state.accepted) + 1, event)
    invalidated = tuple(sorted(set(state.recalculate_required).union(contract.participants)))
    updated = replace(
        state,
        accepted=state.accepted + (accepted,),
        recalculate_required=invalidated,
    )
    return ReplayEvaluation(ReplayCode.OK, updated)


def accept_batch(state: ReplayState, events: tuple[ExternalInput, ...]) -> ReplayEvaluation:
    """Serialize producers by logical order, never by host arrival/completion order."""
    invalid = _validate_state(state)
    if invalid:
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, invalid)
    if type(events) is not tuple or any(_validate_event(event) for event in events):
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, "invalid event batch")
    if len(events) > 1:
        by_time = {}
        for event in events:
            by_time.setdefault(event.at_common, []).append(event)
        for same_time in by_time.values():
            if len(same_time) > 1:
                orders = [event.logical_order for event in same_time]
                if all(order is not None for order in orders) and len(set(orders)) != len(orders):
                    if not all(event.commutative for event in same_time):
                        return ReplayEvaluation(ReplayCode.INDETERMINATE, state, "simultaneous inputs have conflicting logical order")
                if not all(order is not None for order in orders) and not all(event.commutative for event in same_time):
                    return ReplayEvaluation(ReplayCode.INDETERMINATE, state, "simultaneous concurrent inputs lack logical order")
        ordered = tuple(sorted(
            events,
            key=lambda event: (
                event.at_common,
                event.logical_order if event.logical_order is not None else -1,
                event.identity,
            ),
        ))
    else:
        ordered = events
    current = state
    for event in ordered:
        result = accept_input(current, event)
        if result.code is not ReplayCode.OK:
            return result
        current = result.state
    return ReplayEvaluation(ReplayCode.OK, current)


def recalculate_horizon(state: ReplayState, participant_name: str, horizon: Fraction) -> ReplayEvaluation:
    """Republish one participant's safe horizon after an accepted input invalidates it."""
    invalid = _validate_state(state)
    if invalid or type(participant_name) is not str or not _exact(horizon):
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, invalid or "invalid horizon update")
    if participant_name not in state.recalculate_required:
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, "participant has no invalidated horizon")
    participant = next(item for item in state.participants if item.name == participant_name)
    if participant.progress_common is not None and horizon < participant.progress_common:
        return ReplayEvaluation(ReplayCode.INVARIANT_VIOLATION, state, "safe horizon is behind participant progress")
    updated_participants = tuple(
        replace(participant, horizon_common=horizon) if participant.name == participant_name else participant
        for participant in state.participants
    )
    required = tuple(name for name in state.recalculate_required if name != participant_name)
    return ReplayEvaluation(ReplayCode.OK, replace(state, participants=updated_participants, recalculate_required=required))


def advance_to(state: ReplayState, target: Fraction) -> ReplayEvaluation:
    """Apply accepted inputs in recorded order after all affected horizons are republished."""
    invalid = _validate_state(state)
    if invalid or not _exact(target):
        return ReplayEvaluation(ReplayCode.INVALID_INPUT, state, invalid or "invalid target")
    if state.recalculate_required:
        return ReplayEvaluation(ReplayCode.BLOCKED, state, "participant horizons require recalculation")
    if not state.participants or any(
        participant.progress_common is None
        or participant.horizon_common is None
        or not participant.conversion_valid
        for participant in state.participants
    ):
        return ReplayEvaluation(ReplayCode.BLOCKED, state, "participant progress/horizon incomplete")
    frontier = max(
        [participant.progress_common for participant in state.participants]
        + ([state.watermark] if state.watermark is not None else [])
    )
    if target < frontier:
        return ReplayEvaluation(ReplayCode.LATE_INPUT, state, "advance target behind confirmed frontier")
    if target == frontier:
        if any(obligation <= target for obligation in state.pending_obligations):
            return ReplayEvaluation(ReplayCode.BLOCKED, state, "obligation remains at idempotent frontier")
        return ReplayEvaluation(ReplayCode.OK, state)
    if any(obligation <= target for obligation in state.pending_obligations):
        return ReplayEvaluation(ReplayCode.BLOCKED, state, "advance would cross an unresolved obligation")
    horizon = min(participant.horizon_common for participant in state.participants)
    if target > horizon:
        return ReplayEvaluation(ReplayCode.BLOCKED, state, "target exceeds global safe horizon")
    due = [
        item for item in state.accepted
        if item.event.identity not in state.applied_ids and item.event.at_common <= target
    ]
    effects = state.virtual_effects + tuple((item.event.identity, item.event.payload) for item in due)
    applied = state.applied_ids + tuple(item.event.identity for item in due)
    participants = tuple(replace(participant, progress_common=target) for participant in state.participants)
    return ReplayEvaluation(
        ReplayCode.OK,
        replace(
            state,
            participants=participants,
            watermark=target,
            applied_ids=applied,
            virtual_effects=effects,
        ),
    )


def replay(initial: ReplayState, inputs: tuple[ExternalInput, ...], *, target: Fraction) -> ReplayEvaluation:
    """Replay an ordered input sequence through the same admission and advancement rules."""
    admitted = accept_batch(initial, inputs)
    if admitted.code is not ReplayCode.OK:
        return admitted
    current = admitted.state
    for name in tuple(current.recalculate_required):
        participant = next(item for item in current.participants if item.name == name)
        if participant.horizon_common is None:
            return ReplayEvaluation(ReplayCode.BLOCKED, current, "participant has no replacement safe horizon")
        recalculated = recalculate_horizon(current, name, participant.horizon_common)
        if recalculated.code is not ReplayCode.OK:
            return recalculated
        current = recalculated.state
    return advance_to(current, target)
