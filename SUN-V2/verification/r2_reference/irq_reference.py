"""Independent IRQ/NMI observation and acknowledge oracle for R2-T10."""
from __future__ import annotations
from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction


class Line(str, Enum):
    INT = "INT"
    NMI = "NMI"


class AckEffect(str, Enum):
    NONE = "None"
    COMMIT = "Commit"
    UNKNOWN = "Unknown"


class IrqCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"
    INDETERMINATE = "IndeterminateCausalOrder"
    UNSUPPORTED_SNAPSHOT = "UnsupportedSnapshot"


@dataclass(frozen=True)
class LineTransition:
    identity: str
    line: Line
    at_common: Fraction
    asserted: bool


@dataclass(frozen=True)
class AckProfile:
    effect: AckEffect
    commit_at_common: Fraction | None = None


@dataclass(frozen=True)
class IrqState:
    int_level: bool = False
    nmi_level: bool = True
    nmi_edge_pending: bool = False
    iff1: bool = False
    ei_delay: int = 0
    transitions: tuple[LineTransition, ...] = ()
    applied_transitions: tuple[str, ...] = ()
    last_sample_common: Fraction = Fraction(0)
    accepted: str | None = None
    ack_active: bool = False
    vector_sampled: str | None = None
    device_effect_committed: bool = False


@dataclass(frozen=True)
class IrqEvaluation:
    code: IrqCode
    state: IrqState
    value: str | None = None
    snapshot: IrqState | None = None
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational time and reject float/bool coordinates."""
    return type(value) is int or type(value) is Fraction


def _validate(state: object) -> str | None:
    """Validate all line, CPU, transition, and acknowledge state before use."""
    if not isinstance(state, IrqState) or type(state.transitions) is not tuple or type(state.applied_transitions) is not tuple:
        return "invalid IRQ state"
    flags = (state.int_level, state.nmi_level, state.nmi_edge_pending, state.iff1, state.ack_active, state.device_effect_committed)
    if any(type(flag) is not bool for flag in flags):
        return "invalid IRQ flag"
    if type(state.ei_delay) is not int or state.ei_delay < 0 or not _exact(state.last_sample_common):
        return "invalid IRQ counter/time"
    identities = set()
    for item in state.transitions:
        if not isinstance(item, LineTransition) or type(item.identity) is not str or not item.identity.strip() or item.identity in identities:
            return "invalid line transition"
        identities.add(item.identity)
        if not isinstance(item.line, Line) or not _exact(item.at_common) or type(item.asserted) is not bool:
            return "invalid line transition descriptor"
    if len(set(state.applied_transitions)) != len(state.applied_transitions) or not set(state.applied_transitions).issubset(identities):
        return "invalid applied transition ledger"
    if state.accepted not in (None, "INT", "NMI"):
        return "invalid accepted interrupt"
    if state.vector_sampled is not None and type(state.vector_sampled) is not str:
        return "invalid sampled vector"
    return None


def publish_transition(state: IrqState, transition: LineTransition) -> IrqEvaluation:
    """Publish an owner transition without changing the CPU-observed line yet."""
    invalid = _validate(state)
    if invalid:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid)
    if (
        not isinstance(transition, LineTransition)
        or type(transition.identity) is not str
        or not transition.identity.strip()
        or not isinstance(transition.line, Line)
        or not _exact(transition.at_common)
        or type(transition.asserted) is not bool
    ):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail="invalid transition descriptor")
    if transition.identity in {item.identity for item in state.transitions}:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail="duplicate transition identity")
    if transition.at_common <= state.last_sample_common:
        return IrqEvaluation(IrqCode.LATE_INPUT, state, detail="transition is at/behind sampled frontier")
    updated = replace(state, transitions=state.transitions + (transition,))
    return IrqEvaluation(IrqCode.OK, updated)


def _apply_through(state: IrqState, at_common: Fraction) -> tuple[IrqState, IrqCode | None]:
    """Apply only transitions at/before the requested CPU observation point."""
    if at_common < state.last_sample_common:
        return state, IrqCode.LATE_INPUT
    pending = [
        item for item in state.transitions
        if item.identity not in state.applied_transitions and item.at_common <= at_common
    ]
    by_time = {}
    for item in pending:
        by_time.setdefault(item.at_common, []).append(item)
    for items in by_time.values():
        for line in Line:
            same_line = [item for item in items if item.line is line]
            if len(same_line) > 1 and len({item.asserted for item in same_line}) > 1:
                return state, IrqCode.INDETERMINATE
    current = state
    applied = list(state.applied_transitions)
    for item in sorted(pending, key=lambda value: (value.at_common, value.identity)):
        if item.line is Line.INT:
            current = replace(current, int_level=item.asserted)
        else:
            falling_edge = current.nmi_level and not item.asserted
            current = replace(
                current,
                nmi_level=item.asserted,
                nmi_edge_pending=current.nmi_edge_pending or falling_edge,
            )
        applied.append(item.identity)
    return replace(current, applied_transitions=tuple(applied), last_sample_common=at_common), None


def execute_ei(state: IrqState) -> IrqEvaluation:
    """Set IFF1 and inhibit INT acceptance through the following instruction."""
    invalid = _validate(state)
    if invalid:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid)
    return IrqEvaluation(IrqCode.OK, replace(state, iff1=True, ei_delay=1))


def retire_instruction(state: IrqState) -> IrqEvaluation:
    """Consume the EI inhibit marker when the following instruction retires."""
    invalid = _validate(state)
    if invalid:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid)
    return IrqEvaluation(IrqCode.OK, replace(state, ei_delay=max(0, state.ei_delay - 1)))


def sample_int_boundary(state: IrqState, at_common: Fraction) -> IrqEvaluation:
    """Observe INT only at the declared instruction-boundary sample point."""
    invalid = _validate(state)
    if invalid or not _exact(at_common):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid or "invalid sample time")
    advanced, error = _apply_through(state, at_common)
    if error:
        return IrqEvaluation(error, state, detail="cannot establish INT line at sample point")
    if advanced.accepted is not None:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="interrupt already accepted")
    accepted = "INT" if advanced.int_level and advanced.iff1 and advanced.ei_delay == 0 and not advanced.nmi_edge_pending else None
    updated = replace(advanced, accepted=accepted)
    return IrqEvaluation(IrqCode.OK, updated, value=accepted)


def sample_instruction_end(state: IrqState, at_common: Fraction) -> IrqEvaluation:
    """Recognize a pending NMI falling edge at instruction end."""
    invalid = _validate(state)
    if invalid or not _exact(at_common):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid or "invalid sample time")
    advanced, error = _apply_through(state, at_common)
    if error:
        return IrqEvaluation(error, state, detail="cannot establish NMI line at instruction end")
    if advanced.accepted is not None:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="interrupt already accepted")
    accepted = "NMI" if advanced.nmi_edge_pending else None
    updated = replace(advanced, accepted=accepted, nmi_edge_pending=False if accepted else advanced.nmi_edge_pending)
    return IrqEvaluation(IrqCode.OK, updated, value=accepted)


def begin_acknowledge(state: IrqState) -> IrqEvaluation:
    """Start the dedicated INT acknowledge transaction after INT acceptance."""
    invalid = _validate(state)
    if invalid:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid)
    if state.accepted != "INT" or state.ack_active:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="INT acknowledge is not available")
    return IrqEvaluation(IrqCode.OK, replace(state, ack_active=True, vector_sampled=None, device_effect_committed=False))


def sample_ack_vector(state: IrqState, vector: str) -> IrqEvaluation:
    """Capture the interrupt vector once, independently of device-side commit."""
    invalid = _validate(state)
    if invalid:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid)
    if state.vector_sampled is not None:
        return IrqEvaluation(IrqCode.OK, state, value=state.vector_sampled)
    if not state.ack_active or type(vector) is not str:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="vector sample outside acknowledge")
    updated = replace(state, vector_sampled=vector)
    return IrqEvaluation(IrqCode.OK, updated, value=vector)


def commit_ack_device_effect(state: IrqState, profile: AckProfile, at_common: Fraction) -> IrqEvaluation:
    """Commit only a profile-declared device effect at its own known boundary."""
    invalid = _validate(state)
    if invalid or not isinstance(profile, AckProfile) or not isinstance(profile.effect, AckEffect) or not _exact(at_common):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid or "invalid acknowledge profile/time")
    if profile.effect is AckEffect.NONE:
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail="profile declares no device effect")
    if profile.effect is AckEffect.UNKNOWN or not _exact(profile.commit_at_common):
        return IrqEvaluation(IrqCode.INDETERMINATE, state, detail="device effect/commit boundary is UNKNOWN")
    if not state.ack_active:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="acknowledge is not active")
    if state.device_effect_committed:
        return IrqEvaluation(IrqCode.OK, state)
    if at_common < profile.commit_at_common:
        return IrqEvaluation(IrqCode.BLOCKED, state, detail="device commit boundary not reached")
    if at_common > profile.commit_at_common:
        return IrqEvaluation(IrqCode.INVARIANT_VIOLATION, state, detail="device commit boundary was missed")
    return IrqEvaluation(IrqCode.OK, replace(state, device_effect_committed=True))


def complete_acknowledge(state: IrqState, profile: AckProfile) -> IrqEvaluation:
    """Close acknowledge only after vector sample and any known device effect."""
    invalid = _validate(state)
    if invalid or not isinstance(profile, AckProfile) or not isinstance(profile.effect, AckEffect):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid or "invalid acknowledge profile")
    if not state.ack_active or state.vector_sampled is None:
        return IrqEvaluation(IrqCode.BLOCKED, state, detail="acknowledge sample remains pending")
    if profile.effect is AckEffect.UNKNOWN:
        return IrqEvaluation(IrqCode.INDETERMINATE, state, detail="device effect remains UNKNOWN")
    if profile.effect is AckEffect.COMMIT and not state.device_effect_committed:
        return IrqEvaluation(IrqCode.BLOCKED, state, detail="device effect commit remains pending")
    return IrqEvaluation(IrqCode.OK, replace(state, ack_active=False))


def snapshot(state: IrqState, profile: AckProfile) -> IrqEvaluation:
    """Return UnsupportedSnapshot in an ambiguous UNKNOWN acknowledge window."""
    invalid = _validate(state)
    if invalid or not isinstance(profile, AckProfile) or not isinstance(profile.effect, AckEffect):
        return IrqEvaluation(IrqCode.INVALID_INPUT, state, detail=invalid or "invalid snapshot profile")
    if state.ack_active and (
        profile.effect is AckEffect.UNKNOWN
        or (profile.effect is AckEffect.COMMIT and not _exact(profile.commit_at_common))
    ):
        return IrqEvaluation(IrqCode.UNSUPPORTED_SNAPSHOT, state, detail="INT acknowledge effect/commit boundary is UNKNOWN")
    return IrqEvaluation(IrqCode.OK, state, snapshot=state)
