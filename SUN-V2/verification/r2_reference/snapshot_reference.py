"""Immutable causal-cut snapshot/restore oracle for R2-T11."""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from fractions import Fraction


class SnapshotCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"
    INCOMPATIBLE = "IncompatibleSnapshot"
    UNSUPPORTED = "UnsupportedSnapshot"


class DeadlineState(str, Enum):
    SCHEDULED = "Scheduled"
    CANCELLED = "Cancelled"
    SUPERSEDED = "Superseded"
    CLAIMED = "Claimed"
    DISPATCHED = "Dispatched"
    COMPLETED = "Completed"
    FAILED = "Failed"


class TransactionState(str, Enum):
    CREATED = "Created"
    IN_PROGRESS = "InProgress"
    WAITING = "Waiting"
    READY = "Ready"
    COMPLETED = "Completed"
    CANCELLED = "Cancelled"
    FAILED = "Failed"


class AckPresence(str, Enum):
    NONE = "None"
    PRESENT = "Present"
    UNKNOWN = "Unknown"


class AckCommit(str, Enum):
    NOT_APPLICABLE = "NotApplicable"
    PENDING = "Pending"
    COMMITTED = "Committed"
    UNKNOWN = "Unknown"


@dataclass(frozen=True)
class DomainState:
    name: str
    progress_common: Fraction
    residual: Fraction


@dataclass(frozen=True)
class DeadlineRecord:
    identity: str
    generation: int
    at_common: Fraction
    state: DeadlineState
    committed_effects: tuple[str, ...] = ()
    pending_effects: tuple[str, ...] = ()
    error: str | None = None


@dataclass(frozen=True)
class TransactionRecord:
    identity: str
    generation: int
    state: TransactionState
    progress_common: Fraction
    next_marker: str | None = None
    wait_source: str | None = None
    sampled: tuple[tuple[str, str], ...] = ()
    committed_effects: tuple[str, ...] = ()
    pending_effects: tuple[str, ...] = ()
    error: str | None = None


@dataclass(frozen=True)
class InterruptRecord:
    int_level: bool = False
    nmi_level: bool = True
    nmi_edge_pending: bool = False
    accepted: str | None = None
    acknowledge_active: bool = False
    vector_sampled: str | None = None
    device_effect_presence: AckPresence = AckPresence.NONE
    device_effect_commit: AckCommit = AckCommit.NOT_APPLICABLE
    commit_at_common: Fraction | None = None


@dataclass(frozen=True)
class MachineState:
    profile: str
    model_version: str
    domains: tuple[DomainState, ...]
    watermark: Fraction | None
    deadlines: tuple[DeadlineRecord, ...]
    transactions: tuple[TransactionRecord, ...]
    interrupts: InterruptRecord
    input_cursor: int


@dataclass(frozen=True)
class SnapshotArtifact:
    profile: str
    model_version: str
    state: MachineState


@dataclass(frozen=True)
class SnapshotEvaluation:
    code: SnapshotCode
    active_state: MachineState
    artifact: SnapshotArtifact | None = None
    restored_state: MachineState | None = None
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational coordinates and reject float/bool values."""
    return type(value) is int or type(value) is Fraction


def _unique_strings(values: tuple[str, ...]) -> bool:
    """Validate an immutable, nonblank, duplicate-free identity collection."""
    return (
        type(values) is tuple
        and all(type(value) is str and value.strip() for value in values)
        and len(set(values)) == len(values)
    )


def _validate_state(state: object) -> str | None:
    """Validate every component and partial-effect ledger before capture/restore."""
    if not isinstance(state, MachineState):
        return "invalid machine state"
    if any(type(value) is not str or not value.strip() for value in (state.profile, state.model_version)):
        return "invalid profile/version"
    if type(state.domains) is not tuple or type(state.deadlines) is not tuple or type(state.transactions) is not tuple:
        return "invalid state collections"
    if state.watermark is not None and not _exact(state.watermark):
        return "invalid watermark"
    if type(state.input_cursor) is not int or state.input_cursor < 0:
        return "invalid replay cursor"
    domain_names = set()
    for domain in state.domains:
        if not isinstance(domain, DomainState) or type(domain.name) is not str or not domain.name.strip() or domain.name in domain_names:
            return "invalid domain identity"
        domain_names.add(domain.name)
        if not _exact(domain.progress_common) or not _exact(domain.residual):
            return "invalid domain coordinate"
    deadline_ids = set()
    for deadline in state.deadlines:
        if not isinstance(deadline, DeadlineRecord) or type(deadline.identity) is not str or not deadline.identity.strip() or deadline.identity in deadline_ids:
            return "invalid deadline identity"
        deadline_ids.add(deadline.identity)
        if type(deadline.generation) is not int or deadline.generation < 0 or not _exact(deadline.at_common) or not isinstance(deadline.state, DeadlineState):
            return "invalid deadline descriptor"
        if not _unique_strings(deadline.committed_effects) or not _unique_strings(deadline.pending_effects):
            return "invalid deadline effect ledger"
        if set(deadline.committed_effects).intersection(deadline.pending_effects):
            return "overlapping deadline effect ledger"
        if deadline.state is DeadlineState.COMPLETED and deadline.pending_effects:
            return "completed deadline has pending effects"
        if deadline.state is DeadlineState.FAILED and (type(deadline.error) is not str or not deadline.error.strip()):
            return "failed deadline lacks error"
    transaction_ids = set()
    for tx in state.transactions:
        if not isinstance(tx, TransactionRecord) or type(tx.identity) is not str or not tx.identity.strip() or tx.identity in transaction_ids:
            return "invalid transaction identity"
        transaction_ids.add(tx.identity)
        if type(tx.generation) is not int or tx.generation < 0 or not isinstance(tx.state, TransactionState) or not _exact(tx.progress_common):
            return "invalid transaction descriptor"
        if tx.wait_source is not None and (type(tx.wait_source) is not str or not tx.wait_source.strip()):
            return "invalid wait source"
        if type(tx.sampled) is not tuple or any(
            type(pair) is not tuple or len(pair) != 2 or type(pair[0]) is not str
            or not pair[0].strip() or type(pair[1]) is not str
            for pair in tx.sampled
        ):
            return "invalid sample ledger"
        if len({pair[0] for pair in tx.sampled}) != len(tx.sampled):
            return "duplicate sample marker"
        if not _unique_strings(tx.committed_effects) or not _unique_strings(tx.pending_effects):
            return "invalid transaction effect ledger"
        if set(tx.committed_effects).intersection(tx.pending_effects):
            return "overlapping transaction effects"
        if tx.state is TransactionState.WAITING and (tx.wait_source is None or not tx.wait_source.strip()):
            return "waiting transaction lacks wait source"
        if tx.state is TransactionState.COMPLETED and tx.pending_effects:
            return "completed transaction has pending effects"
        if tx.state is TransactionState.FAILED and (type(tx.error) is not str or not tx.error.strip()):
            return "failed transaction lacks error"
    irq = state.interrupts
    if not isinstance(irq, InterruptRecord):
        return "invalid interrupt state"
    if any(type(flag) is not bool for flag in (irq.int_level, irq.nmi_level, irq.nmi_edge_pending, irq.acknowledge_active)):
        return "invalid interrupt flags"
    if irq.accepted not in (None, "INT", "NMI") or (irq.vector_sampled is not None and type(irq.vector_sampled) is not str):
        return "invalid interrupt sample"
    if not isinstance(irq.device_effect_presence, AckPresence) or not isinstance(irq.device_effect_commit, AckCommit):
        return "invalid acknowledge effect state"
    if irq.commit_at_common is not None and not _exact(irq.commit_at_common):
        return "invalid acknowledge commit coordinate"
    if irq.device_effect_presence is AckPresence.NONE and irq.device_effect_commit is not AckCommit.NOT_APPLICABLE:
        return "ack commit exists without device effect"
    if irq.device_effect_presence is AckPresence.PRESENT and irq.device_effect_commit is AckCommit.NOT_APPLICABLE:
        return "present device effect lacks commit state"
    if irq.device_effect_commit is AckCommit.COMMITTED and irq.device_effect_presence is not AckPresence.PRESENT:
        return "committed effect lacks known presence"
    return None


def capture(state: MachineState) -> SnapshotEvaluation:
    """Capture a supported immutable cut; UNKNOWN INT-ack state produces no artifact."""
    invalid = _validate_state(state)
    if invalid:
        return SnapshotEvaluation(SnapshotCode.INVALID_INPUT, state, detail=invalid)
    irq = state.interrupts
    ambiguous = irq.acknowledge_active and (
        irq.device_effect_presence is AckPresence.UNKNOWN
        or (
            irq.device_effect_presence is AckPresence.PRESENT
            and (
                irq.device_effect_commit is AckCommit.UNKNOWN
                or (irq.device_effect_commit is AckCommit.PENDING and irq.commit_at_common is None)
            )
        )
    )
    if ambiguous:
        return SnapshotEvaluation(
            SnapshotCode.UNSUPPORTED,
            state,
            detail="INT acknowledge effect presence/commit boundary is UNKNOWN",
        )
    artifact = SnapshotArtifact(state.profile, state.model_version, state)
    return SnapshotEvaluation(SnapshotCode.OK, state, artifact=artifact)


def restore(
    active_state: MachineState,
    artifact: SnapshotArtifact,
    *,
    expected_profile: str,
    expected_version: str,
) -> SnapshotEvaluation:
    """Validate the complete artifact before exposing the restored machine state."""
    invalid = _validate_state(active_state)
    if invalid:
        return SnapshotEvaluation(SnapshotCode.INVALID_INPUT, active_state, detail=invalid)
    if not isinstance(artifact, SnapshotArtifact) or _validate_state(artifact.state) is not None:
        return SnapshotEvaluation(SnapshotCode.INCOMPATIBLE, active_state, detail="snapshot artifact is invalid")
    if (
        type(expected_profile) is not str or not expected_profile.strip()
        or type(expected_version) is not str or not expected_version.strip()
    ):
        return SnapshotEvaluation(SnapshotCode.INVALID_INPUT, active_state, detail="invalid expected profile/version")
    if artifact.profile != expected_profile or artifact.model_version != expected_version:
        return SnapshotEvaluation(SnapshotCode.INCOMPATIBLE, active_state, detail="snapshot profile/model version mismatch")
    if artifact.state.profile != artifact.profile or artifact.state.model_version != artifact.model_version:
        return SnapshotEvaluation(SnapshotCode.INCOMPATIBLE, active_state, detail="artifact metadata disagrees with state")
    if capture(artifact.state).code is not SnapshotCode.OK:
        return SnapshotEvaluation(SnapshotCode.INCOMPATIBLE, active_state, detail="artifact contains an unsupported causal cut")
    return SnapshotEvaluation(SnapshotCode.OK, active_state, artifact=artifact, restored_state=artifact.state)
