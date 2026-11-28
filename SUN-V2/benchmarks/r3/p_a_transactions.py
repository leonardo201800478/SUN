"""P-A disposable serial transaction lifecycle candidate for W08.

The candidate owns its state types and transitions. It models logical sample,
commit, wait and partial-effect ledgers only; it is not a production bus.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction


class PATxState(str, Enum):
    CREATED = "Created"
    IN_PROGRESS = "InProgress"
    WAITING = "Waiting"
    READY = "Ready"
    COMPLETED = "Completed"
    CANCELLED = "Cancelled"
    FAILED = "Failed"


class PATxCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"


class PAMarkerKind(str, Enum):
    SAMPLE = "Sample"
    COMMIT = "Commit"


@dataclass(frozen=True)
class PAMarker:
    """Named sample or irreversible commit frontier."""

    identity: str
    kind: PAMarkerKind
    at_common: Fraction
    irreversible: bool = False


@dataclass(frozen=True)
class PATransactionSpec:
    """Immutable request identity and complete retry descriptor."""

    identity: str
    generation: int
    origin: str
    destination: str
    operation: str
    profile: str
    payload: str
    admitted_at: Fraction
    start_at: Fraction
    duration_base: Fraction
    automatic_tws: int
    external_tws: int
    markers: tuple[PAMarker, ...]


@dataclass(frozen=True)
class PATransaction:
    spec: PATransactionSpec
    state: PATxState = PATxState.CREATED
    progress_common: Fraction = Fraction(0)
    base_consumed: Fraction = Fraction(0)
    automatic_tw_consumed: int = 0
    external_tw_consumed: int = 0
    ready_marker: str | None = None
    wait_source: str | None = None
    sampled: tuple[tuple[str, str], ...] = ()
    committed: tuple[str, ...] = ()
    failure: str | None = None
    recoveries: int = 0


@dataclass(frozen=True)
class PATransactionBook:
    transactions: tuple[PATransaction, ...] = ()


@dataclass(frozen=True)
class PATxResult:
    code: PATxCode
    book: PATransactionBook
    transaction: PATransaction | None = None
    value: str | None = None
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational coordinates and reject floats/bools."""
    return type(value) is int or type(value) is Fraction


def _validate_spec(spec: object) -> str | None:
    """Validate the entire request before admission or book mutation."""
    if not isinstance(spec, PATransactionSpec):
        return "invalid transaction descriptor"
    for value in (
        spec.identity, spec.origin, spec.destination, spec.operation, spec.profile
    ):
        if type(value) is not str or not value.strip():
            return "invalid descriptor identity"
    if type(spec.payload) is not str:
        return "invalid payload"
    if type(spec.generation) is not int or spec.generation < 0:
        return "invalid generation"
    if any(
        not _exact(value)
        for value in (spec.admitted_at, spec.start_at, spec.duration_base)
    ):
        return "invalid time"
    if spec.start_at < spec.admitted_at or spec.duration_base < 0:
        return "invalid time ordering"
    if type(spec.automatic_tws) is not int or spec.automatic_tws < 0:
        return "invalid automatic TW count"
    if type(spec.external_tws) is not int or spec.external_tws < 0:
        return "invalid external TW count"
    if type(spec.markers) is not tuple:
        return "invalid marker collection"

    identities: set[str] = set()
    previous = spec.start_at
    for marker in spec.markers:
        if not isinstance(marker, PAMarker):
            return "invalid marker"
        if (
            type(marker.identity) is not str
            or not marker.identity.strip()
            or marker.identity in identities
        ):
            return "invalid marker identity"
        identities.add(marker.identity)
        if not isinstance(marker.kind, PAMarkerKind) or not _exact(marker.at_common):
            return "invalid marker descriptor"
        if marker.at_common < previous or type(marker.irreversible) is not bool:
            return "invalid marker boundary or descriptor order"
        previous = marker.at_common
    return None


def _find(book: PATransactionBook, identity: str) -> PATransaction | None:
    """Find a transaction by its stable request identity."""
    return next(
        (tx for tx in book.transactions if tx.spec.identity == identity), None
    )


def _store(book: PATransactionBook, updated: PATransaction) -> PATransactionBook:
    """Replace exactly one transaction while preserving all other records."""
    return PATransactionBook(tuple(
        updated if tx.spec.identity == updated.spec.identity else tx
        for tx in book.transactions
    ))


def _result(code, book, tx=None, value=None, detail=""):
    """Build the immutable operation result."""
    return PATxResult(code, book, tx, value, detail)


def _completed_markers(tx: PATransaction) -> set[str]:
    """Return sample and commit markers already recorded in the ledger."""
    return {identity for identity, _ in tx.sampled}.union(tx.committed)


def submit(book: PATransactionBook, spec: PATransactionSpec) -> PATxResult:
    """Admit a request; identical retries return the exact recorded state."""
    invalid = _validate_spec(spec)
    if invalid:
        return _result(PATxCode.INVALID_INPUT, book, detail=invalid)
    existing = _find(book, spec.identity)
    if existing is not None:
        if existing.spec != spec:
            return _result(
                PATxCode.INVALID_INPUT, book, existing,
                detail="ID reused with divergent descriptor",
            )
        return _result(PATxCode.OK, book, existing)
    tx = PATransaction(spec=spec, progress_common=Fraction(spec.admitted_at))
    return _result(PATxCode.OK, PATransactionBook(book.transactions + (tx,)), tx)


def start(book: PATransactionBook, identity: str) -> PATxResult:
    """Move Created to InProgress; repeating start while active is idempotent."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is PATxState.IN_PROGRESS:
        return _result(PATxCode.OK, book, tx)
    if tx.state is not PATxState.CREATED:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="start transition is not permitted",
        )
    updated = replace(
        tx, state=PATxState.IN_PROGRESS, progress_common=Fraction(tx.spec.start_at)
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def enter_waiting(book: PATransactionBook, identity: str, source: str) -> PATxResult:
    """Enter Waiting with a named external, fixture or dependency source."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if source not in ("external", "fixture", "dependency"):
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid wait source")
    if tx.state is not PATxState.IN_PROGRESS:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="wait transition requires InProgress",
        )
    updated = replace(tx, state=PATxState.WAITING, wait_source=source)
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def release_waiting(book: PATransactionBook, identity: str, source: str) -> PATxResult:
    """Resume only when the same identified wait source is released."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not PATxState.WAITING or tx.wait_source != source:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="wait source/state mismatch",
        )
    updated = replace(tx, state=PATxState.IN_PROGRESS, wait_source=None)
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def reach_next_marker(
    book: PATransactionBook, identity: str, at_common: Fraction
) -> PATxResult:
    """Reach only the next declared marker; never skip or pre-apply later markers."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if not _exact(at_common):
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid marker time")
    if tx.state is not PATxState.IN_PROGRESS:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="transaction is not in progress",
        )
    if at_common < tx.progress_common:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="transaction progress regressed",
        )
    completed = _completed_markers(tx)
    pending = next(
        (marker for marker in tx.spec.markers if marker.identity not in completed),
        None,
    )
    if pending is None:
        return _result(PATxCode.BLOCKED, book, tx, detail="no marker remains")
    if at_common < pending.at_common:
        return _result(PATxCode.BLOCKED, book, tx, detail="marker boundary not reached")
    updated = replace(
        tx, state=PATxState.READY, progress_common=Fraction(pending.at_common),
        ready_marker=pending.identity,
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def sample_marker(
    book: PATransactionBook, identity: str, marker_id: str, sampled_value: str
) -> PATxResult:
    """Capture a read/ack value once; retries return the first captured value."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    prior = next((value for key, value in tx.sampled if key == marker_id), None)
    if prior is not None:
        return _result(PATxCode.OK, book, tx, prior)
    marker = next(
        (item for item in tx.spec.markers if item.identity == marker_id), None
    )
    if (
        marker is None or marker.kind is not PAMarkerKind.SAMPLE
        or type(sampled_value) is not str
    ):
        return _result(
            PATxCode.INVALID_INPUT, book, tx,
            detail="invalid sample marker or value",
        )
    if tx.state is not PATxState.READY or tx.ready_marker != marker_id:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="sample marker is not the reached frontier",
        )
    updated = replace(
        tx, state=PATxState.IN_PROGRESS, ready_marker=None,
        sampled=tx.sampled + ((marker_id, sampled_value),),
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated, sampled_value)


def commit_effect(book: PATransactionBook, identity: str, marker_id: str) -> PATxResult:
    """Commit one named effect exactly once at its reached boundary."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if marker_id in tx.committed:
        return _result(PATxCode.OK, book, tx)
    marker = next(
        (item for item in tx.spec.markers if item.identity == marker_id), None
    )
    if marker is None or marker.kind is not PAMarkerKind.COMMIT:
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid commit marker")
    if tx.state is not PATxState.READY or tx.ready_marker != marker_id:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="commit marker is not the reached frontier",
        )
    updated = replace(
        tx, state=PATxState.IN_PROGRESS, ready_marker=None,
        committed=tx.committed + (marker_id,),
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def consume_base_time(
    book: PATransactionBook, identity: str, amount: Fraction
) -> PATxResult:
    """Record base duration separately from automatic and external TW budgets."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if not _exact(amount) or amount < 0:
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid base duration")
    if tx.state is not PATxState.IN_PROGRESS:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="base time requires InProgress",
        )
    if tx.base_consumed + amount > tx.spec.duration_base:
        return _result(
            PATxCode.INVALID_INPUT, book, tx, detail="base duration over-consumption"
        )
    updated = replace(tx, base_consumed=tx.base_consumed + amount)
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def consume_tw(
    book: PATransactionBook, identity: str, *, automatic: bool
) -> PATxResult:
    """Consume one TW from exactly one declared budget, never merging sources."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if type(automatic) is not bool:
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid TW source")
    if tx.state is not PATxState.IN_PROGRESS:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="TW consumption requires InProgress",
        )
    if automatic:
        if tx.automatic_tw_consumed >= tx.spec.automatic_tws:
            return _result(
                PATxCode.INVALID_INPUT, book, tx,
                detail="automatic TW budget exhausted",
            )
        updated = replace(
            tx, automatic_tw_consumed=tx.automatic_tw_consumed + 1
        )
    else:
        if tx.external_tw_consumed >= tx.spec.external_tws:
            return _result(
                PATxCode.INVALID_INPUT, book, tx,
                detail="external TW budget exhausted",
            )
        updated = replace(tx, external_tw_consumed=tx.external_tw_consumed + 1)
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def complete(book: PATransactionBook, identity: str) -> PATxResult:
    """Complete only after every marker and duration component is consumed."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not PATxState.IN_PROGRESS:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="completion requires InProgress",
        )
    all_markers = {marker.identity for marker in tx.spec.markers}
    if (
        _completed_markers(tx) != all_markers
        or tx.base_consumed != tx.spec.duration_base
        or tx.automatic_tw_consumed != tx.spec.automatic_tws
        or tx.external_tw_consumed != tx.spec.external_tws
    ):
        return _result(PATxCode.BLOCKED, book, tx, detail="required markers or duration remain")
    updated = replace(tx, state=PATxState.COMPLETED)
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def cancel(book: PATransactionBook, identity: str) -> PATxResult:
    """Cancel only before the first irreversible effect; never simulate rollback."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    irreversible_sample = any(
        next(marker for marker in tx.spec.markers if marker.identity == key).irreversible
        for key, _ in tx.sampled
    )
    if tx.committed or irreversible_sample:
        return _result(
            PATxCode.LATE_INPUT, book, tx,
            detail="irreversible effect already applied",
        )
    if tx.state in (PATxState.COMPLETED, PATxState.CANCELLED, PATxState.FAILED):
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="terminal state cannot be cancelled",
        )
    updated = replace(
        tx, state=PATxState.CANCELLED, ready_marker=None, wait_source=None
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def fail(book: PATransactionBook, identity: str, error: str) -> PATxResult:
    """Record terminal failure while preserving the exact partial-effect ledger."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if type(error) is not str or not error.strip():
        return _result(PATxCode.INVALID_INPUT, book, tx, detail="invalid failure")
    if tx.state in (PATxState.COMPLETED, PATxState.CANCELLED, PATxState.FAILED):
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="failure transition is not permitted",
        )
    updated = replace(
        tx, state=PATxState.FAILED, ready_marker=None,
        wait_source=None, failure=error,
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)


def recover(book: PATransactionBook, identity: str) -> PATxResult:
    """Explicitly resume Failed from its existing ledger; retry never recovers it."""
    tx = _find(book, identity)
    if tx is None:
        return _result(PATxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not PATxState.FAILED:
        return _result(
            PATxCode.INVARIANT_VIOLATION, book, tx,
            detail="explicit recovery requires Failed",
        )
    updated = replace(
        tx, state=PATxState.IN_PROGRESS, ready_marker=None,
        wait_source=None, recoveries=tx.recoveries + 1,
    )
    new_book = _store(book, updated)
    return _result(PATxCode.OK, new_book, updated)
