"""Disposable candidate-local W09 snapshot/restore/replay support for P-D and P-E.

This module serializes the complete state represented by each R3 candidate,
including pending agenda/cancellation records, dependency indexes' source data,
and the shared W08 TransactionBook. It is not a full machine snapshot and does
not claim equivalence for CPU/VDP/PSG/IRQ state absent from these candidates.
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, fields, is_dataclass
from enum import Enum
from fractions import Fraction
from typing import Callable

from p_d_discrete_agenda import (
    AgendaCancellation, AgendaConstraint, AgendaEvent, AgendaPhaseBasis,
    AgendaState, EventStatus, PairRelation as AgendaPairRelation,
    _validate as _validate_pd,
)
from p_e_sync_on_demand import (
    PairRelation as SyncPairRelation, SyncConstraint, SyncDomain,
    SyncPhaseBasis, SyncState, SyncTransition, _validate as _validate_pe,
)
from r3_transaction_core import (
    MarkerKind, Transaction, TransactionBook, TxMarker, TxSpec, TxState,
    _validate_book,
)


SCHEMA = "sun-r3-candidate-snapshot-v1"
_CANDIDATES = ("P-D", "P-E")


class SnapshotCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"
    INCOMPATIBLE = "IncompatibleSnapshot"


@dataclass(frozen=True)
class SnapshotArtifact:
    candidate: str
    profile: str
    model_version: str
    input_cursor: int
    payload: bytes
    digest: str


@dataclass(frozen=True)
class SnapshotResult:
    code: SnapshotCode
    active_state: object
    artifact: SnapshotArtifact | None = None
    restored_state: object | None = None
    detail: str = ""


@dataclass(frozen=True)
class ReplayEvent:
    sequence: int
    operation: str
    arguments: bytes


@dataclass(frozen=True)
class ReplayResult:
    code: SnapshotCode
    state: object
    input_cursor: int
    detail: str = ""


_RECORD_TYPES = {
    f"{item.__module__}.{item.__qualname__}": item
    for item in (
        AgendaCancellation, AgendaConstraint, AgendaEvent, AgendaPhaseBasis,
        AgendaState, SyncConstraint, SyncDomain, SyncPhaseBasis, SyncState,
        SyncTransition, Transaction, TransactionBook, TxMarker, TxSpec,
    )
}
_ENUM_TYPES = {
    f"{item.__module__}.{item.__qualname__}": item
    for item in (
        EventStatus, AgendaPairRelation, SyncPairRelation, MarkerKind, TxState,
    )
}


def _encode_value(value):
    """Encode immutable candidate values with explicit type tags and exact rationals."""
    if isinstance(value, Enum):
        return {"$enum": value.__class__.__module__ + "." + value.__class__.__qualname__,
                "value": value.value}
    if type(value) is Fraction:
        return {"$fraction": [value.numerator, value.denominator]}
    if type(value) is tuple:
        return {"$tuple": [_encode_value(item) for item in value]}
    if type(value) is frozenset:
        return {"$frozenset": sorted(value)}
    if is_dataclass(value) and not isinstance(value, type):
        key = value.__class__.__module__ + "." + value.__class__.__qualname__
        if key not in _RECORD_TYPES:
            raise ValueError("unregistered snapshot record type")
        return {
            "$record": key,
            "fields": {
                item.name: _encode_value(getattr(value, item.name))
                for item in fields(value) if not item.name.startswith("_")
            },
        }
    if value is None or type(value) in (str, int, bool):
        return value
    raise ValueError("unsupported value in candidate snapshot")


def _decode_value(value):
    """Decode only registered records/enums; never import types from artifact data."""
    if value is None or type(value) in (str, int, bool):
        return value
    if type(value) is not dict:
        raise ValueError("invalid encoded snapshot value")
    if set(value) == {"$fraction"}:
        pair = value["$fraction"]
        if (type(pair) is not list or len(pair) != 2
                or type(pair[0]) is not int or type(pair[1]) is not int
                or pair[1] <= 0):
            raise ValueError("invalid exact rational")
        return Fraction(pair[0], pair[1])
    if set(value) == {"$tuple"}:
        if type(value["$tuple"]) is not list:
            raise ValueError("invalid tuple encoding")
        return tuple(_decode_value(item) for item in value["$tuple"])
    if set(value) == {"$frozenset"}:
        items = value["$frozenset"]
        if type(items) is not list or any(type(item) is not str for item in items):
            raise ValueError("invalid frozenset encoding")
        if len(items) != len(set(items)):
            raise ValueError("duplicate frozenset member")
        return frozenset(items)
    if set(value) == {"$enum", "value"}:
        enum_type = _ENUM_TYPES.get(value["$enum"])
        if enum_type is None:
            raise ValueError("unregistered snapshot enum")
        return enum_type(value["value"])
    if set(value) == {"$record", "fields"}:
        record_type = _RECORD_TYPES.get(value["$record"])
        raw_fields = value["fields"]
        if record_type is None or type(raw_fields) is not dict:
            raise ValueError("unregistered snapshot record")
        allowed = {item.name for item in fields(record_type)
                   if not item.name.startswith("_")}
        if set(raw_fields) != allowed:
            raise ValueError("snapshot record fields do not match schema")
        decoded = {name: _decode_value(item) for name, item in raw_fields.items()}
        return record_type(**decoded)
    raise ValueError("unknown or noncanonical snapshot value")


def _validate_candidate(candidate: str, state: object) -> str | None:
    """Validate the candidate's semantic state and its complete W08 transaction ledger."""
    if candidate == "P-D" and isinstance(state, AgendaState):
        return _validate_pd(state)
    if candidate == "P-E" and isinstance(state, SyncState):
        error = _validate_pe(state)
        if error is not None:
            return error
        return _validate_book(state.transactions)
    return "candidate/state type mismatch"


def _canonical_payload(candidate, profile, model_version, input_cursor, state) -> bytes:
    envelope = {
        "schema": SCHEMA,
        "candidate": candidate,
        "profile": profile,
        "model_version": model_version,
        "input_cursor": input_cursor,
        "state": _encode_value(state),
    }
    return json.dumps(
        envelope, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def capture(
    candidate: str,
    state: object,
    *,
    profile: str,
    model_version: str,
    input_cursor: int,
) -> SnapshotResult:
    """Capture one complete P-D/P-E candidate state as canonical bytes and SHA-256."""
    if (
        candidate not in _CANDIDATES
        or type(profile) is not str or not profile.strip()
        or type(model_version) is not str or not model_version.strip()
        or type(input_cursor) is not int or input_cursor < 0
    ):
        return SnapshotResult(SnapshotCode.INVALID_INPUT, state, detail="invalid snapshot metadata")
    invalid = _validate_candidate(candidate, state)
    if invalid is not None:
        return SnapshotResult(SnapshotCode.INVALID_INPUT, state, detail=invalid)
    try:
        payload = _canonical_payload(candidate, profile, model_version, input_cursor, state)
    except (TypeError, ValueError) as error:
        return SnapshotResult(SnapshotCode.INVALID_INPUT, state, detail=str(error))
    artifact = SnapshotArtifact(
        candidate, profile, model_version, input_cursor,
        payload, hashlib.sha256(payload).hexdigest(),
    )
    return SnapshotResult(SnapshotCode.OK, state, artifact=artifact)


def _decode_artifact(artifact: SnapshotArtifact):
    """Verify checksum, canonical encoding, metadata and candidate invariants."""
    if not isinstance(artifact, SnapshotArtifact):
        raise ValueError("invalid snapshot artifact type")
    if (
        type(artifact.candidate) is not str or artifact.candidate not in _CANDIDATES
        or type(artifact.profile) is not str or not artifact.profile.strip()
        or type(artifact.model_version) is not str or not artifact.model_version.strip()
        or type(artifact.input_cursor) is not int or artifact.input_cursor < 0
        or type(artifact.payload) is not bytes
        or type(artifact.digest) is not str
    ):
        raise ValueError("invalid snapshot artifact fields")
    if hashlib.sha256(artifact.payload).hexdigest() != artifact.digest:
        raise ValueError("snapshot payload digest mismatch")
    envelope = json.loads(artifact.payload.decode("utf-8"))
    if type(envelope) is not dict or set(envelope) != {
        "schema", "candidate", "profile", "model_version", "input_cursor", "state"
    }:
        raise ValueError("invalid snapshot envelope")
    canonical = json.dumps(
        envelope, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    if canonical != artifact.payload:
        raise ValueError("snapshot payload is not canonical")
    if (
        envelope["schema"] != SCHEMA
        or envelope["candidate"] != artifact.candidate
        or envelope["profile"] != artifact.profile
        or envelope["model_version"] != artifact.model_version
        or envelope["input_cursor"] != artifact.input_cursor
    ):
        raise ValueError("snapshot metadata disagrees with payload")
    state = _decode_value(envelope["state"])
    invalid = _validate_candidate(artifact.candidate, state)
    if invalid is not None:
        raise ValueError(invalid)
    return state


def restore(
    active_state: object,
    artifact: SnapshotArtifact,
    *,
    expected_candidate: str,
    expected_profile: str,
    expected_version: str,
) -> SnapshotResult:
    """Validate the entire artifact before returning a restored immutable state."""
    active_error = _validate_candidate(expected_candidate, active_state)
    if active_error is not None:
        return SnapshotResult(SnapshotCode.INVALID_INPUT, active_state, detail=active_error)
    if (
        type(expected_profile) is not str or not expected_profile.strip()
        or type(expected_version) is not str or not expected_version.strip()
        or expected_candidate not in _CANDIDATES
    ):
        return SnapshotResult(SnapshotCode.INVALID_INPUT, active_state, detail="invalid restore expectations")
    if (
        not isinstance(artifact, SnapshotArtifact)
        or artifact.candidate != expected_candidate
        or artifact.profile != expected_profile
        or artifact.model_version != expected_version
    ):
        return SnapshotResult(SnapshotCode.INCOMPATIBLE, active_state, detail="snapshot candidate/profile/version mismatch")
    try:
        restored = _decode_artifact(artifact)
    except (ValueError, TypeError, KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return SnapshotResult(SnapshotCode.INCOMPATIBLE, active_state, detail=str(error))
    return SnapshotResult(
        SnapshotCode.OK, active_state, artifact=artifact, restored_state=restored
    )


def make_replay_event(sequence: int, operation: str, arguments: dict) -> ReplayEvent:
    """Build one canonical, sequence-numbered operation for candidate replay."""
    if (
        type(sequence) is not int or sequence < 1
        or type(operation) is not str or not operation.strip()
        or type(arguments) is not dict
    ):
        raise ValueError("invalid replay event")
    payload = json.dumps(
        arguments, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return ReplayEvent(sequence, operation, payload)


def replay_from_snapshot(
    artifact: SnapshotArtifact,
    events: tuple[ReplayEvent, ...],
    *,
    expected_candidate: str,
    expected_profile: str,
    expected_version: str,
    apply: Callable[[object, str, dict], object],
) -> ReplayResult:
    """Replay a contiguous accepted-operation suffix in sequence order, atomically."""
    try:
        base_state = _decode_artifact(artifact)
    except (ValueError, TypeError, KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return ReplayResult(SnapshotCode.INCOMPATIBLE, None, 0, str(error))
    if (
        type(expected_candidate) is not str or expected_candidate not in _CANDIDATES
        or type(expected_profile) is not str or not expected_profile.strip()
        or type(expected_version) is not str or not expected_version.strip()
    ):
        return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, artifact.input_cursor,
                            "invalid replay compatibility expectations")
    if (
        artifact.candidate != expected_candidate
        or artifact.profile != expected_profile
        or artifact.model_version != expected_version
    ):
        return ReplayResult(SnapshotCode.INCOMPATIBLE, base_state, artifact.input_cursor,
                            "snapshot candidate/profile/version mismatch")
    cursor = artifact.input_cursor
    if type(events) is not tuple or not callable(apply):
        return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, "invalid replay input")
    parsed = []
    for event in events:
        if (
            not isinstance(event, ReplayEvent)
            or type(event.sequence) is not int or event.sequence < 1
            or type(event.operation) is not str or not event.operation.strip()
            or type(event.arguments) is not bytes
        ):
            return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, "invalid replay event")
        try:
            args = json.loads(event.arguments.decode("utf-8"))
            canonical = json.dumps(
                args, sort_keys=True, separators=(",", ":"), ensure_ascii=False
            ).encode("utf-8")
        except (UnicodeDecodeError, json.JSONDecodeError, TypeError, ValueError):
            return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, "invalid replay arguments")
        if type(args) is not dict or canonical != event.arguments:
            return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, "replay arguments are not canonical object data")
        parsed.append((event.sequence, event.operation, args))
    ordered = sorted(parsed, key=lambda item: item[0])
    sequences = [item[0] for item in ordered]
    if (
        len(sequences) != len(set(sequences))
        or sequences != list(range(cursor + 1, cursor + 1 + len(sequences)))
    ):
        return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, "replay sequence is duplicated or non-contiguous")
    staged = base_state
    try:
        for _, operation, arguments in ordered:
            staged = apply(staged, operation, arguments)
            invalid = _validate_candidate(artifact.candidate, staged)
            if invalid is not None:
                raise ValueError(invalid)
    except Exception as error:
        return ReplayResult(SnapshotCode.INVALID_INPUT, base_state, cursor, str(error))
    next_cursor = sequences[-1] if sequences else cursor
    return ReplayResult(SnapshotCode.OK, staged, next_cursor)
