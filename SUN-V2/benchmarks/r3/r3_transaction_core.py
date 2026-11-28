"""Strategy-neutral transaction lifecycle shared by disposable P-D/P-E prototypes.

This is an independent R3 implementation of the W08 logical transaction contract.
It models sample/commit markers and exact ledgers only; it is not a production bus.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction


class TxState(str, Enum):
    CREATED = "Created"
    IN_PROGRESS = "InProgress"
    WAITING = "Waiting"
    READY = "Ready"
    COMPLETED = "Completed"
    CANCELLED = "Cancelled"
    FAILED = "Failed"


class TxCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"


class MarkerKind(str, Enum):
    SAMPLE = "Sample"
    COMMIT = "Commit"


@dataclass(frozen=True)
class TxMarker:
    identity: str
    kind: MarkerKind
    at_common: Fraction
    irreversible: bool = False


@dataclass(frozen=True)
class TxSpec:
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
    markers: tuple[TxMarker, ...]


@dataclass(frozen=True)
class Transaction:
    spec: TxSpec
    state: TxState = TxState.CREATED
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
class TransactionBook:
    transactions: tuple[Transaction, ...] = ()


@dataclass(frozen=True)
class TxResult:
    code: TxCode
    book: TransactionBook
    transaction: Transaction | None = None
    value: str | None = None
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational coordinates, rejecting floats and bools."""
    return type(value) in (int, Fraction)


def _validate_spec(spec: object) -> str | None:
    """Validate the immutable retry descriptor and ordered marker schedule."""
    if not isinstance(spec, TxSpec):
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
    if any(not _exact(value) for value in (
        spec.admitted_at, spec.start_at, spec.duration_base
    )):
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
        if (
            not isinstance(marker, TxMarker)
            or type(marker.identity) is not str
            or not marker.identity.strip()
            or marker.identity in identities
            or not isinstance(marker.kind, MarkerKind)
            or not _exact(marker.at_common)
            or marker.at_common < previous
            or type(marker.irreversible) is not bool
        ):
            return "invalid marker descriptor or order"
        identities.add(marker.identity)
        previous = marker.at_common
    return None


def _validate_book(book: object) -> str | None:
    """Validate book uniqueness and every transaction's local ledgers."""
    if not isinstance(book, TransactionBook) or type(book.transactions) is not tuple:
        return "invalid transaction book"
    identities: set[str] = set()
    for tx in book.transactions:
        if not isinstance(tx, Transaction):
            return "invalid transaction record"
        invalid = _validate_spec(tx.spec)
        if invalid is not None:
            return invalid
        if tx.spec.identity in identities:
            return "duplicate transaction identity"
        identities.add(tx.spec.identity)
        if not isinstance(tx.state, TxState):
            return "invalid transaction state"
        if not all(_exact(value) for value in (
            tx.progress_common, tx.base_consumed
        )) or tx.base_consumed < 0 or tx.base_consumed > tx.spec.duration_base:
            return "invalid transaction progress or duration ledger"
        if (
            type(tx.automatic_tw_consumed) is not int
            or tx.automatic_tw_consumed < 0
            or tx.automatic_tw_consumed > tx.spec.automatic_tws
            or type(tx.external_tw_consumed) is not int
            or tx.external_tw_consumed < 0
            or tx.external_tw_consumed > tx.spec.external_tws
            or type(tx.recoveries) is not int or tx.recoveries < 0
        ):
            return "invalid TW or recovery ledger"
        marker_by_id = {item.identity: item for item in tx.spec.markers}
        if type(tx.sampled) is not tuple:
            return "invalid sample ledger"
        sampled_ids = [item[0] for item in tx.sampled if type(item) is tuple and len(item) == 2]
        if (
            len(sampled_ids) != len(tx.sampled)
            or len(set(sampled_ids)) != len(sampled_ids)
            or any(
                identity not in marker_by_id
                or marker_by_id[identity].kind is not MarkerKind.SAMPLE
                or type(value) is not str
                for identity, value in tx.sampled
            )
        ):
            return "invalid sample ledger"
        if (
            type(tx.committed) is not tuple
            or len(set(tx.committed)) != len(tx.committed)
            or any(
                identity not in marker_by_id
                or marker_by_id[identity].kind is not MarkerKind.COMMIT
                for identity in tx.committed
            )
        ):
            return "invalid commit ledger"
        if tx.ready_marker is not None and tx.ready_marker not in marker_by_id:
            return "unknown ready marker"
        if tx.wait_source is not None and tx.wait_source not in (
            "external", "fixture", "dependency"
        ):
            return "invalid wait source"
        if tx.failure is not None and type(tx.failure) is not str:
            return "invalid failure descriptor"
    return None


def _find(book: TransactionBook, identity: str) -> Transaction | None:
    """Find a transaction by stable request identity."""
    return next((tx for tx in book.transactions if tx.spec.identity == identity), None)


def _store(book: TransactionBook, updated: Transaction) -> TransactionBook:
    """Replace exactly one record while preserving the remaining ledger."""
    return TransactionBook(tuple(
        updated if tx.spec.identity == updated.spec.identity else tx
        for tx in book.transactions
    ))


def _result(code, book, tx=None, value=None, detail="") -> TxResult:
    """Build an immutable operation result."""
    return TxResult(code, book, tx, value, detail)


def _completed_markers(tx: Transaction) -> set[str]:
    """Return marker identities already sampled or committed."""
    return {identity for identity, _ in tx.sampled}.union(tx.committed)


def submit(book: TransactionBook, spec: TxSpec) -> TxResult:
    """Admit a request; exact retries are idempotent, divergent reuse is invalid."""
    invalid = _validate_book(book)
    if invalid is not None:
        return _result(TxCode.INVALID_INPUT, book, detail=invalid)
    invalid = _validate_spec(spec)
    if invalid is not None:
        return _result(TxCode.INVALID_INPUT, book, detail=invalid)
    existing = _find(book, spec.identity)
    if existing is not None:
        if existing.spec != spec:
            return _result(
                TxCode.INVALID_INPUT, book, existing,
                detail="ID reused with divergent descriptor",
            )
        return _result(TxCode.OK, book, existing)
    tx = Transaction(spec=spec, progress_common=Fraction(spec.admitted_at))
    return _result(TxCode.OK, TransactionBook(book.transactions + (tx,)), tx)


def start(book: TransactionBook, identity: str) -> TxResult:
    """Move Created to InProgress; active retries are idempotent."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is TxState.IN_PROGRESS:
        return _result(TxCode.OK, book, tx)
    if tx.state is not TxState.CREATED:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="start transition is not permitted")
    updated = replace(tx, state=TxState.IN_PROGRESS, progress_common=Fraction(tx.spec.start_at))
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def enter_waiting(book: TransactionBook, identity: str, source: str) -> TxResult:
    """Enter Waiting with a separately identified external/fixture/dependency source."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if source not in ("external", "fixture", "dependency"):
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid wait source")
    if tx.state is not TxState.IN_PROGRESS:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="wait requires InProgress")
    updated = replace(tx, state=TxState.WAITING, wait_source=source)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def release_waiting(book: TransactionBook, identity: str, source: str) -> TxResult:
    """Resume only when the same named wait source is released."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not TxState.WAITING or tx.wait_source != source:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="wait source/state mismatch")
    updated = replace(tx, state=TxState.IN_PROGRESS, wait_source=None)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def reach_next_marker(book: TransactionBook, identity: str, at_common: Fraction) -> TxResult:
    """Reach only the next declared sample/commit marker."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if not _exact(at_common):
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid marker time")
    if tx.state is not TxState.IN_PROGRESS:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="transaction is not in progress")
    if at_common < tx.progress_common:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="transaction progress regressed")
    completed = _completed_markers(tx)
    pending = next((marker for marker in tx.spec.markers if marker.identity not in completed), None)
    if pending is None:
        return _result(TxCode.BLOCKED, book, tx, detail="no marker remains")
    if at_common < pending.at_common:
        return _result(TxCode.BLOCKED, book, tx, detail="marker boundary not reached")
    updated = replace(
        tx, state=TxState.READY, progress_common=Fraction(pending.at_common),
        ready_marker=pending.identity,
    )
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def sample_marker(book: TransactionBook, identity: str, marker_id: str, sampled_value: str) -> TxResult:
    """Capture a sample exactly once; a retry returns the first captured value."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    prior = next((value for key, value in tx.sampled if key == marker_id), None)
    if prior is not None:
        return _result(TxCode.OK, book, tx, prior)
    marker = next((item for item in tx.spec.markers if item.identity == marker_id), None)
    if marker is None or marker.kind is not MarkerKind.SAMPLE or type(sampled_value) is not str:
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid sample marker or value")
    if tx.state is not TxState.READY or tx.ready_marker != marker_id:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="sample is not the reached frontier")
    updated = replace(
        tx, state=TxState.IN_PROGRESS, ready_marker=None,
        sampled=tx.sampled + ((marker_id, sampled_value),),
    )
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated, sampled_value)


def commit_effect(book: TransactionBook, identity: str, marker_id: str) -> TxResult:
    """Apply a named irreversible effect once at its reached boundary."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if marker_id in tx.committed:
        return _result(TxCode.OK, book, tx)
    marker = next((item for item in tx.spec.markers if item.identity == marker_id), None)
    if marker is None or marker.kind is not MarkerKind.COMMIT:
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid commit marker")
    if tx.state is not TxState.READY or tx.ready_marker != marker_id:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="commit is not the reached frontier")
    updated = replace(
        tx, state=TxState.IN_PROGRESS, ready_marker=None,
        committed=tx.committed + (marker_id,),
    )
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def consume_base_time(book: TransactionBook, identity: str, amount: Fraction) -> TxResult:
    """Consume exact base duration independently of automatic/external TW budgets."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if not _exact(amount) or amount < 0:
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid base duration")
    if tx.state is not TxState.IN_PROGRESS:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="base time requires InProgress")
    if tx.base_consumed + amount > tx.spec.duration_base:
        return _result(TxCode.INVALID_INPUT, book, tx, detail="base duration over-consumption")
    updated = replace(tx, base_consumed=tx.base_consumed + amount)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def consume_tw(book: TransactionBook, identity: str, *, automatic: bool) -> TxResult:
    """Consume one TW from exactly one independent budget."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if type(automatic) is not bool:
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid TW source")
    if tx.state is not TxState.IN_PROGRESS:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="TW consumption requires InProgress")
    if automatic:
        if tx.automatic_tw_consumed >= tx.spec.automatic_tws:
            return _result(TxCode.INVALID_INPUT, book, tx, detail="automatic TW budget exhausted")
        updated = replace(tx, automatic_tw_consumed=tx.automatic_tw_consumed + 1)
    else:
        if tx.external_tw_consumed >= tx.spec.external_tws:
            return _result(TxCode.INVALID_INPUT, book, tx, detail="external TW budget exhausted")
        updated = replace(tx, external_tw_consumed=tx.external_tw_consumed + 1)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def complete(book: TransactionBook, identity: str) -> TxResult:
    """Complete only after every marker, base duration and both TW budgets finish."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not TxState.IN_PROGRESS:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="completion requires InProgress")
    all_markers = {marker.identity for marker in tx.spec.markers}
    if (
        _completed_markers(tx) != all_markers
        or tx.base_consumed != tx.spec.duration_base
        or tx.automatic_tw_consumed != tx.spec.automatic_tws
        or tx.external_tw_consumed != tx.spec.external_tws
    ):
        return _result(TxCode.BLOCKED, book, tx, detail="required markers or duration remain")
    updated = replace(tx, state=TxState.COMPLETED)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def cancel(book: TransactionBook, identity: str) -> TxResult:
    """Cancel only before any irreversible sample/commit; never simulate rollback."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    marker_by_id = {marker.identity: marker for marker in tx.spec.markers}
    irreversible_sample = any(marker_by_id[key].irreversible for key, _ in tx.sampled)
    if tx.committed or irreversible_sample:
        return _result(TxCode.LATE_INPUT, book, tx, detail="irreversible effect already applied")
    if tx.state in (TxState.COMPLETED, TxState.CANCELLED, TxState.FAILED):
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="terminal state cannot be cancelled")
    updated = replace(tx, state=TxState.CANCELLED, ready_marker=None, wait_source=None)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def fail(book: TransactionBook, identity: str, error: str) -> TxResult:
    """Record failure while preserving the exact committed/sample prefix."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if type(error) is not str or not error.strip():
        return _result(TxCode.INVALID_INPUT, book, tx, detail="invalid failure")
    if tx.state in (TxState.COMPLETED, TxState.CANCELLED, TxState.FAILED):
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="failure transition is not permitted")
    updated = replace(tx, state=TxState.FAILED, ready_marker=None, wait_source=None, failure=error)
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)


def recover(book: TransactionBook, identity: str) -> TxResult:
    """Resume Failed only by an explicit recovery transition."""
    tx = _find(book, identity)
    if tx is None:
        return _result(TxCode.INVALID_INPUT, book, detail="unknown transaction")
    if tx.state is not TxState.FAILED:
        return _result(TxCode.INVARIANT_VIOLATION, book, tx, detail="explicit recovery requires Failed")
    updated = replace(
        tx, state=TxState.IN_PROGRESS, ready_marker=None, wait_source=None,
        recoveries=tx.recoveries + 1,
    )
    new_book = _store(book, updated)
    return _result(TxCode.OK, new_book, updated)
