"""P-A W09 canonical snapshot/restore and deterministic replay candidate.

Snapshots are canonical JSON bytes with a SHA-256 digest. Replay applies accepted
input records by sequence number, not worker completion order. This is a disposable
Python candidate, not a production serializer or hardware-state implementation.
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction


class PASnapshotCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"
    INCOMPATIBLE = "IncompatibleSnapshot"
    UNSUPPORTED = "UnsupportedSnapshot"
    LATE_INPUT = "LateInput"


class PADeadlineState(str, Enum):
    SCHEDULED = "Scheduled"
    CANCELLED = "Cancelled"
    SUPERSEDED = "Superseded"
    CLAIMED = "Claimed"
    DISPATCHED = "Dispatched"
    COMPLETED = "Completed"
    FAILED = "Failed"


class PATransactionState(str, Enum):
    CREATED = "Created"
    IN_PROGRESS = "InProgress"
    WAITING = "Waiting"
    READY = "Ready"
    COMPLETED = "Completed"
    CANCELLED = "Cancelled"
    FAILED = "Failed"


class PAAckPresence(str, Enum):
    NONE = "None"
    PRESENT = "Present"
    UNKNOWN = "Unknown"


class PAAckCommit(str, Enum):
    NOT_APPLICABLE = "NotApplicable"
    PENDING = "Pending"
    COMMITTED = "Committed"
    UNKNOWN = "Unknown"


@dataclass(frozen=True)
class PADomainState:
    name: str
    progress_common: Fraction
    residual: Fraction


@dataclass(frozen=True)
class PADeadlineRecord:
    identity: str
    generation: int
    at_common: Fraction
    state: PADeadlineState
    committed_effects: tuple[str, ...] = ()
    pending_effects: tuple[str, ...] = ()
    error: str | None = None


@dataclass(frozen=True)
class PATransactionRecord:
    identity: str
    generation: int
    state: PATransactionState
    progress_common: Fraction
    next_marker: str | None = None
    wait_source: str | None = None
    sampled: tuple[tuple[str, str], ...] = ()
    committed_effects: tuple[str, ...] = ()
    pending_effects: tuple[str, ...] = ()
    error: str | None = None


@dataclass(frozen=True)
class PAInterruptRecord:
    int_level: bool = False
    nmi_level: bool = True
    nmi_edge_pending: bool = False
    accepted: str | None = None
    acknowledge_active: bool = False
    vector_sampled: str | None = None
    device_effect_presence: PAAckPresence = PAAckPresence.NONE
    device_effect_commit: PAAckCommit = PAAckCommit.NOT_APPLICABLE
    commit_at_common: Fraction | None = None


@dataclass(frozen=True)
class PAMachineState:
    profile: str
    model_version: str
    domains: tuple[PADomainState, ...]
    watermark: Fraction | None
    deadlines: tuple[PADeadlineRecord, ...]
    transactions: tuple[PATransactionRecord, ...]
    interrupts: PAInterruptRecord
    input_cursor: int


@dataclass(frozen=True)
class PASnapshotArtifact:
    profile: str
    model_version: str
    payload: bytes
    digest: str


@dataclass(frozen=True)
class PASnapshotResult:
    code: PASnapshotCode
    active_state: PAMachineState
    artifact: PASnapshotArtifact | None = None
    restored_state: PAMachineState | None = None
    detail: str = ""


@dataclass(frozen=True)
class PAReplayEvent:
    sequence: int
    at_common: Fraction
    domain: str
    progress_common: Fraction
    residual: Fraction


@dataclass(frozen=True)
class PAReplayResult:
    code: PASnapshotCode
    state: PAMachineState | None
    digest: str | None = None
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
    """Validate every component and partial-effect ledger before snapshotting."""
    if not isinstance(state, PAMachineState):
        return "invalid machine state"
    if any(type(value) is not str or not value.strip() for value in (state.profile, state.model_version)):
        return "invalid profile/version"
    if type(state.domains) is not tuple or type(state.deadlines) is not tuple or type(state.transactions) is not tuple:
        return "invalid state collections"
    if state.watermark is not None and not _exact(state.watermark):
        return "invalid watermark"
    if type(state.input_cursor) is not int or state.input_cursor < 0:
        return "invalid replay cursor"

    domain_names: set[str] = set()
    for domain in state.domains:
        if (
            not isinstance(domain, PADomainState)
            or type(domain.name) is not str or not domain.name.strip()
            or domain.name in domain_names
        ):
            return "invalid domain identity"
        domain_names.add(domain.name)
        if not _exact(domain.progress_common) or not _exact(domain.residual):
            return "invalid domain coordinate"

    deadline_keys: set[tuple[str, int]] = set()
    for deadline in state.deadlines:
        if (
            not isinstance(deadline, PADeadlineRecord)
            or type(deadline.identity) is not str or not deadline.identity.strip()
            or type(deadline.generation) is not int or deadline.generation < 0
            or (deadline.identity, deadline.generation) in deadline_keys
        ):
            return "invalid deadline identity/generation"
        deadline_keys.add((deadline.identity, deadline.generation))
        if (
            type(deadline.generation) is not int or deadline.generation < 0
            or not _exact(deadline.at_common)
            or not isinstance(deadline.state, PADeadlineState)
        ):
            return "invalid deadline descriptor"
        if (
            not _unique_strings(deadline.committed_effects)
            or not _unique_strings(deadline.pending_effects)
            or set(deadline.committed_effects).intersection(deadline.pending_effects)
        ):
            return "invalid deadline effect ledger"
        if deadline.state is PADeadlineState.COMPLETED and deadline.pending_effects:
            return "completed deadline has pending effects"
        if deadline.state is PADeadlineState.FAILED and (
            type(deadline.error) is not str or not deadline.error.strip()
        ):
            return "failed deadline lacks error"

    transaction_ids: set[str] = set()
    for tx in state.transactions:
        if (
            not isinstance(tx, PATransactionRecord)
            or type(tx.identity) is not str or not tx.identity.strip()
            or tx.identity in transaction_ids
        ):
            return "invalid transaction identity"
        transaction_ids.add(tx.identity)
        if (
            type(tx.generation) is not int or tx.generation < 0
            or not isinstance(tx.state, PATransactionState)
            or not _exact(tx.progress_common)
        ):
            return "invalid transaction descriptor"
        if tx.wait_source is not None and (
            type(tx.wait_source) is not str or not tx.wait_source.strip()
        ):
            return "invalid wait source"
        if type(tx.sampled) is not tuple or any(
            type(pair) is not tuple or len(pair) != 2
            or type(pair[0]) is not str or not pair[0].strip()
            or type(pair[1]) is not str
            for pair in tx.sampled
        ):
            return "invalid sample ledger"
        if len({pair[0] for pair in tx.sampled}) != len(tx.sampled):
            return "duplicate sample marker"
        if (
            not _unique_strings(tx.committed_effects)
            or not _unique_strings(tx.pending_effects)
            or set(tx.committed_effects).intersection(tx.pending_effects)
        ):
            return "invalid transaction effect ledger"
        if tx.state is PATransactionState.WAITING and (
            tx.wait_source is None or not tx.wait_source.strip()
        ):
            return "waiting transaction lacks wait source"
        if tx.state is PATransactionState.COMPLETED and tx.pending_effects:
            return "completed transaction has pending effects"
        if tx.state is PATransactionState.FAILED and (
            type(tx.error) is not str or not tx.error.strip()
        ):
            return "failed transaction lacks error"

    irq = state.interrupts
    if not isinstance(irq, PAInterruptRecord):
        return "invalid interrupt state"
    if any(type(flag) is not bool for flag in (
        irq.int_level, irq.nmi_level, irq.nmi_edge_pending, irq.acknowledge_active
    )):
        return "invalid interrupt flags"
    if irq.accepted not in (None, "INT", "NMI") or (
        irq.vector_sampled is not None and type(irq.vector_sampled) is not str
    ):
        return "invalid interrupt sample"
    if not isinstance(irq.device_effect_presence, PAAckPresence) or not isinstance(
        irq.device_effect_commit, PAAckCommit
    ):
        return "invalid acknowledge effect state"
    if irq.commit_at_common is not None and not _exact(irq.commit_at_common):
        return "invalid acknowledge commit coordinate"
    if (
        irq.device_effect_presence is PAAckPresence.NONE
        and irq.device_effect_commit is not PAAckCommit.NOT_APPLICABLE
    ):
        return "ack commit exists without device effect"
    if (
        irq.device_effect_presence is PAAckPresence.PRESENT
        and irq.device_effect_commit is PAAckCommit.NOT_APPLICABLE
    ):
        return "present device effect lacks commit state"
    if (
        irq.device_effect_commit is PAAckCommit.COMMITTED
        and irq.device_effect_presence is not PAAckPresence.PRESENT
    ):
        return "committed effect lacks known presence"
    return None


def _ambiguous_ack(state: PAMachineState) -> bool:
    """Identify only unsupported cuts inside an ambiguous INT acknowledge window."""
    irq = state.interrupts
    return irq.acknowledge_active and (
        irq.device_effect_presence is PAAckPresence.UNKNOWN
        or (
            irq.device_effect_presence is PAAckPresence.PRESENT
            and (
                irq.device_effect_commit is PAAckCommit.UNKNOWN
                or (
                    irq.device_effect_commit is PAAckCommit.PENDING
                    and irq.commit_at_common is None
                )
            )
        )
    )


def _encode_exact(value: int | Fraction):
    """Encode integer and rational coordinates without losing their exact type."""
    if type(value) is int:
        return {"kind": "int", "value": value}
    return {
        "kind": "fraction",
        "numerator": value.numerator,
        "denominator": value.denominator,
    }


def _decode_exact(value):
    """Decode one exact coordinate, rejecting malformed or noncanonical values."""
    if type(value) is not dict:
        raise ValueError("invalid exact coordinate")
    if value.get("kind") == "int" and type(value.get("value")) is int:
        return value["value"]
    if (
        value.get("kind") == "fraction"
        and type(value.get("numerator")) is int
        and type(value.get("denominator")) is int
        and value["denominator"] > 0
    ):
        return Fraction(value["numerator"], value["denominator"])
    raise ValueError("invalid exact coordinate")


def _encode_state(state: PAMachineState) -> bytes:
    """Serialize a validated state into canonical UTF-8 JSON bytes."""
    irq = state.interrupts
    data = {
        "profile": state.profile,
        "model_version": state.model_version,
        "domains": [
            {
                "name": item.name,
                "progress_common": _encode_exact(item.progress_common),
                "residual": _encode_exact(item.residual),
            }
            for item in state.domains
        ],
        "watermark": None if state.watermark is None else _encode_exact(state.watermark),
        "deadlines": [
            {
                "identity": item.identity,
                "generation": item.generation,
                "at_common": _encode_exact(item.at_common),
                "state": item.state.value,
                "committed_effects": list(item.committed_effects),
                "pending_effects": list(item.pending_effects),
                "error": item.error,
            }
            for item in state.deadlines
        ],
        "transactions": [
            {
                "identity": item.identity,
                "generation": item.generation,
                "state": item.state.value,
                "progress_common": _encode_exact(item.progress_common),
                "next_marker": item.next_marker,
                "wait_source": item.wait_source,
                "sampled": [list(pair) for pair in item.sampled],
                "committed_effects": list(item.committed_effects),
                "pending_effects": list(item.pending_effects),
                "error": item.error,
            }
            for item in state.transactions
        ],
        "interrupts": {
            "int_level": irq.int_level,
            "nmi_level": irq.nmi_level,
            "nmi_edge_pending": irq.nmi_edge_pending,
            "accepted": irq.accepted,
            "acknowledge_active": irq.acknowledge_active,
            "vector_sampled": irq.vector_sampled,
            "device_effect_presence": irq.device_effect_presence.value,
            "device_effect_commit": irq.device_effect_commit.value,
            "commit_at_common": (
                None if irq.commit_at_common is None
                else _encode_exact(irq.commit_at_common)
            ),
        },
        "input_cursor": state.input_cursor,
    }
    return json.dumps(
        data, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _decode_state(payload: bytes) -> PAMachineState:
    """Decode canonical state bytes into immutable P-A records."""
    if type(payload) is not bytes:
        raise ValueError("snapshot payload must be bytes")
    data = json.loads(payload.decode("utf-8"))
    domains = tuple(
        PADomainState(
            item["name"], _decode_exact(item["progress_common"]),
            _decode_exact(item["residual"]),
        )
        for item in data["domains"]
    )
    deadlines = tuple(
        PADeadlineRecord(
            item["identity"], item["generation"], _decode_exact(item["at_common"]),
            PADeadlineState(item["state"]), tuple(item["committed_effects"]),
            tuple(item["pending_effects"]), item["error"],
        )
        for item in data["deadlines"]
    )
    transactions = tuple(
        PATransactionRecord(
            item["identity"], item["generation"], PATransactionState(item["state"]),
            _decode_exact(item["progress_common"]), item["next_marker"],
            item["wait_source"], tuple(tuple(pair) for pair in item["sampled"]),
            tuple(item["committed_effects"]), tuple(item["pending_effects"]),
            item["error"],
        )
        for item in data["transactions"]
    )
    raw_irq = data["interrupts"]
    irq = PAInterruptRecord(
        raw_irq["int_level"], raw_irq["nmi_level"], raw_irq["nmi_edge_pending"],
        raw_irq["accepted"], raw_irq["acknowledge_active"],
        raw_irq["vector_sampled"], PAAckPresence(raw_irq["device_effect_presence"]),
        PAAckCommit(raw_irq["device_effect_commit"]),
        None if raw_irq["commit_at_common"] is None
        else _decode_exact(raw_irq["commit_at_common"]),
    )
    return PAMachineState(
        data["profile"], data["model_version"], domains,
        None if data["watermark"] is None else _decode_exact(data["watermark"]),
        deadlines, transactions, irq, data["input_cursor"],
    )


def _artifact_state(artifact: PASnapshotArtifact) -> PAMachineState:
    """Verify metadata and payload digest before decoding an artifact."""
    if not isinstance(artifact, PASnapshotArtifact):
        raise ValueError("invalid snapshot artifact")
    if (
        type(artifact.profile) is not str or not artifact.profile.strip()
        or type(artifact.model_version) is not str or not artifact.model_version.strip()
        or type(artifact.digest) is not str
        or type(artifact.payload) is not bytes
    ):
        raise ValueError("invalid snapshot artifact fields")
    actual = hashlib.sha256(artifact.payload).hexdigest()
    if actual != artifact.digest:
        raise ValueError("snapshot payload digest mismatch")
    state = _decode_state(artifact.payload)
    if _encode_state(state) != artifact.payload:
        raise ValueError("snapshot payload is not canonical")
    if state.profile != artifact.profile or state.model_version != artifact.model_version:
        raise ValueError("snapshot metadata disagrees with payload")
    invalid = _validate_state(state)
    if invalid is not None:
        raise ValueError(invalid)
    if _ambiguous_ack(state):
        raise ValueError("snapshot contains an unsupported causal cut")
    return state


def _state_digest(state: PAMachineState) -> str:
    """Return a deterministic digest over the canonical state representation."""
    return hashlib.sha256(_encode_state(state)).hexdigest()


def capture(state: PAMachineState) -> PASnapshotResult:
    """Capture a supported immutable cut as canonical JSON bytes plus digest."""
    invalid = _validate_state(state)
    if invalid:
        return PASnapshotResult(PASnapshotCode.INVALID_INPUT, state, detail=invalid)
    if _ambiguous_ack(state):
        return PASnapshotResult(
            PASnapshotCode.UNSUPPORTED, state,
            detail="INT acknowledge effect presence/commit boundary is UNKNOWN",
        )
    payload = _encode_state(state)
    artifact = PASnapshotArtifact(
        state.profile, state.model_version, payload,
        hashlib.sha256(payload).hexdigest(),
    )
    return PASnapshotResult(PASnapshotCode.OK, state, artifact=artifact)


def restore(
    active_state: PAMachineState,
    artifact: PASnapshotArtifact,
    *,
    expected_profile: str,
    expected_version: str,
) -> PASnapshotResult:
    """Validate all metadata, checksum and causal state before exposing restore."""
    invalid = _validate_state(active_state)
    if invalid:
        return PASnapshotResult(PASnapshotCode.INVALID_INPUT, active_state, detail=invalid)
    if (
        type(expected_profile) is not str or not expected_profile.strip()
        or type(expected_version) is not str or not expected_version.strip()
    ):
        return PASnapshotResult(
            PASnapshotCode.INVALID_INPUT, active_state,
            detail="invalid expected profile/version",
        )
    if not isinstance(artifact, PASnapshotArtifact):
        return PASnapshotResult(
            PASnapshotCode.INCOMPATIBLE, active_state,
            detail="snapshot artifact is invalid",
        )
    if artifact.profile != expected_profile or artifact.model_version != expected_version:
        return PASnapshotResult(
            PASnapshotCode.INCOMPATIBLE, active_state,
            detail="snapshot profile/model version mismatch",
        )
    try:
        restored = _artifact_state(artifact)
    except (ValueError, TypeError, KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return PASnapshotResult(
            PASnapshotCode.INCOMPATIBLE, active_state, detail=str(error)
        )
    return PASnapshotResult(
        PASnapshotCode.OK, active_state, artifact=artifact, restored_state=restored
    )


def replay_from_snapshot(
    artifact: PASnapshotArtifact,
    events: tuple[PAReplayEvent, ...],
) -> PAReplayResult:
    """Replay accepted inputs in sequence order behind the global admission frontier."""
    try:
        state = _artifact_state(artifact)
    except (ValueError, TypeError, KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return PAReplayResult(PASnapshotCode.INCOMPATIBLE, None, detail=str(error))
    if type(events) is not tuple:
        return PAReplayResult(
            PASnapshotCode.INVALID_INPUT, state, _state_digest(state),
            "replay events must be an immutable tuple",
        )

    seen: set[int] = set()
    for event in events:
        if (
            not isinstance(event, PAReplayEvent)
            or type(event.sequence) is not int or event.sequence <= 0
            or not _exact(event.at_common)
            or type(event.domain) is not str or not event.domain.strip()
            or not _exact(event.progress_common)
            or not _exact(event.residual)
            or event.sequence in seen
        ):
            return PAReplayResult(
                PASnapshotCode.INVALID_INPUT, state, _state_digest(state),
                "invalid or duplicate replay event",
            )
        seen.add(event.sequence)

    ordered = tuple(sorted(events, key=lambda item: item.sequence))
    expected_sequence = state.input_cursor + 1
    previous_time = state.watermark
    domains = list(state.domains)
    # Replay records are an already-admitted batch. Compute A(E) once from
    # the snapshot cut (W and every participant P_d); applying one record must
    # not retroactively invalidate another record in that same accepted batch.
    frontier_values = [domain.progress_common for domain in domains]
    if state.watermark is not None:
        frontier_values.append(state.watermark)
    admission_frontier = max(frontier_values) if frontier_values else None
    for event in ordered:
        if event.sequence != expected_sequence:
            return PAReplayResult(
                PASnapshotCode.INVALID_INPUT, state, _state_digest(state),
                "replay sequence has a gap or begins before the cursor",
            )
        if previous_time is not None and event.at_common < previous_time:
            return PAReplayResult(
                PASnapshotCode.LATE_INPUT, state, _state_digest(state),
                "replay event regressed behind a previously admitted input time",
            )
        index = next(
            (i for i, domain in enumerate(domains) if domain.name == event.domain),
            None,
        )
        if index is None:
            return PAReplayResult(
                PASnapshotCode.INVALID_INPUT, state, _state_digest(state),
                "replay references an unknown domain",
            )
        if admission_frontier is not None and event.at_common <= admission_frontier:
            return PAReplayResult(
                PASnapshotCode.LATE_INPUT, state, _state_digest(state),
                "replay event is at or behind the snapshot global admission frontier",
            )
        if event.progress_common < domains[index].progress_common:
            return PAReplayResult(
                PASnapshotCode.INVALID_INPUT, state, _state_digest(state),
                "domain progress regressed during replay",
            )
        domains[index] = replace(
            domains[index], progress_common=event.progress_common,
            residual=event.residual,
        )
        expected_sequence += 1
        previous_time = event.at_common

    replayed = replace(
        state, domains=tuple(domains),
        input_cursor=ordered[-1].sequence if ordered else state.input_cursor,
    )
    return PAReplayResult(
        PASnapshotCode.OK, replayed, _state_digest(replayed),
        "replay applied in accepted input-sequence order",
    )
