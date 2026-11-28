"""Disposable W08 composition bridge for TransactionBook and ResourceLedger.

The reservation amount and marker-to-effect mapping are fixture inputs. They are
not inferred from hardware, marker IDs are not device-effect IDs, and this
bridge is not a production bus or a normative transaction-capacity policy.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum

from r3_resource_ledger import (
    ResourceCode, ResourceLedger, _validate as _validate_resource,
    commit_effect as commit_resource_effect, fail as fail_resource,
    recover as recover_resource, reserve as reserve_resource,
)
from r3_transaction_core import (
    MarkerKind, TransactionBook, TxCode, TxSpec, TxState, _validate_book,
    _validate_spec, commit_effect as commit_tx_marker,
    fail as fail_tx, reach_next_marker, recover as recover_tx,
    sample_marker as sample_tx_marker, start as start_tx, submit as submit_tx,
)


class BridgeCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVARIANT_VIOLATION = "InvariantViolation"
    INCOMPARABLE_RECOVERY = "IncomparableRecovery"


@dataclass(frozen=True)
class TransactionResourceState:
    book: TransactionBook
    resources: ResourceLedger
    marker_effects: tuple[tuple[str, str, str], ...] = ()


@dataclass(frozen=True)
class RecoveryBoundaryAudit:
    transaction_after: TransactionBook
    resource_after: ResourceLedger
    not_equivalent: tuple[str, ...]


@dataclass(frozen=True)
class BridgeResult:
    code: BridgeCode
    state: TransactionResourceState
    detail: str = ""
    recovery_audit: RecoveryBoundaryAudit | None = None
    sampled_value: str | None = None


def _validate_state(state: object) -> str | None:
    if not isinstance(state, TransactionResourceState):
        return "invalid transaction/resource state"
    invalid = _validate_book(state.book)
    if invalid is not None:
        return invalid
    invalid = _validate_resource(state.resources)
    if invalid is not None:
        return invalid
    if type(state.marker_effects) is not tuple:
        return "invalid marker/effect mapping"
    if len(state.book.transactions) > 1 or len(state.resources.reservations) > 1:
        return "bridge state is scoped to one transaction and one reservation"
    if not state.book.transactions:
        if state.resources.reservations or state.marker_effects:
            return "resource reservation exists without its transaction"
        return None
    tx = state.book.transactions[0]
    reservation = state.resources.reservations[0] if state.resources.reservations else None
    if reservation is None or reservation[0] != tx.spec.identity:
        return "transaction and resource reservation identities differ"
    mapping: dict[str, str] = {}
    effect_ids: set[str] = set()
    for item in state.marker_effects:
        if (
            type(item) is not tuple or len(item) != 3
            or item[0] != tx.spec.identity
            or type(item[1]) is not str or not item[1].strip()
            or type(item[2]) is not str or not item[2].strip()
            or item[1] in mapping or item[2] in effect_ids
        ):
            return "invalid or duplicate marker/effect mapping"
        mapping[item[1]] = item[2]
        effect_ids.add(item[2])
    commit_markers = {
        marker.identity for marker in tx.spec.markers
        if marker.kind is MarkerKind.COMMIT
    }
    if set(mapping) != commit_markers or set(reservation[2]) != effect_ids:
        return "marker/effect mapping does not match the reservation descriptor"
    committed_markers = set(tx.committed)
    expected_committed = {
        effect for marker, effect in mapping.items()
        if marker in committed_markers
    }
    expected_pending = effect_ids - expected_committed
    if set(state.resources.committed_effects) != expected_committed:
        return "resource committed effects disagree with transaction commit markers"
    if set(state.resources.pending_effects) != expected_pending:
        return "resource pending effects disagree with uncommitted transaction markers"
    if state.resources.completed:
        return "resource recovery completion has no equivalent transaction terminal state"
    if state.resources.failed != (tx.state is TxState.FAILED):
        return "transaction/resource failure states diverge"
    return None


def _tx_code(code: TxCode) -> BridgeCode:
    return BridgeCode(code.value)


def _resource_code(code: ResourceCode) -> BridgeCode:
    return BridgeCode(code.value)


def reserve_and_submit(
    state: TransactionResourceState,
    spec: TxSpec,
    *,
    amount: int,
    marker_effects: tuple[tuple[str, str], ...],
) -> BridgeResult:
    """Atomically reserve caller-specified budget and admit one transaction."""
    invalid = _validate_state(state)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    invalid = _validate_spec(spec)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    if state.book.transactions and state.book.transactions[0].spec.identity != spec.identity:
        return BridgeResult(
            BridgeCode.INVALID_INPUT, state,
            "composition bridge admits one transaction identity per isolated state",
        )
    if type(marker_effects) is not tuple:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, "marker_effects must be a tuple")
    commit_markers = {
        marker.identity for marker in spec.markers
        if marker.kind is MarkerKind.COMMIT
    }
    pairs: dict[str, str] = {}
    effects: set[str] = set()
    for item in marker_effects:
        if (
            type(item) is not tuple or len(item) != 2
            or type(item[0]) is not str or not item[0].strip()
            or type(item[1]) is not str or not item[1].strip()
            or item[0] in pairs or item[1] in effects
        ):
            return BridgeResult(BridgeCode.INVALID_INPUT, state, "invalid marker/effect descriptor")
        pairs[item[0]] = item[1]
        effects.add(item[1])
    if not commit_markers or set(pairs) != commit_markers:
        return BridgeResult(
            BridgeCode.INVALID_INPUT, state,
            "every commit marker requires exactly one fixture-supplied effect mapping",
        )

    mapping = tuple(
        (spec.identity, marker, effect) for marker, effect in marker_effects
    )
    if state.book.transactions and state.marker_effects != mapping:
        return BridgeResult(
            BridgeCode.INVALID_INPUT, state,
            "retry changed the transaction marker/effect mapping",
        )
    reserved = reserve_resource(
        state.resources, spec.identity, amount,
        tuple(effect for _, effect in marker_effects),
    )
    if reserved.code is not ResourceCode.OK:
        return BridgeResult(_resource_code(reserved.code), state, reserved.detail)
    submitted = submit_tx(state.book, spec)
    if submitted.code is not TxCode.OK:
        # Immutable results let us discard the tentative reservation atomically.
        return BridgeResult(_tx_code(submitted.code), state, submitted.detail)

    updated = replace(
        state, book=submitted.book, resources=reserved.ledger,
        marker_effects=mapping,
    )
    invalid = _validate_state(updated)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    return BridgeResult(BridgeCode.OK, updated)


def start_transaction(state: TransactionResourceState) -> BridgeResult:
    """Advance the transaction lifecycle without changing its resource budget."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    result = start_tx(state.book, tx.spec.identity)
    if result.code is not TxCode.OK:
        return BridgeResult(_tx_code(result.code), state, result.detail)
    return BridgeResult(BridgeCode.OK, replace(state, book=result.book))


def reach_transaction_marker(
    state: TransactionResourceState, at_common
) -> BridgeResult:
    """Move to the next declared marker; resource effects remain pending."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    result = reach_next_marker(state.book, tx.spec.identity, at_common)
    if result.code is not TxCode.OK:
        return BridgeResult(_tx_code(result.code), state, result.detail)
    return BridgeResult(BridgeCode.OK, replace(state, book=result.book))


def sample_transaction_marker(
    state: TransactionResourceState, marker_id: str, sampled_value: str
) -> BridgeResult:
    """Capture a SAMPLE marker in TransactionBook; ResourceLedger is unchanged."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    sampled = sample_tx_marker(
        state.book, tx.spec.identity, marker_id, sampled_value
    )
    if sampled.code is not TxCode.OK:
        return BridgeResult(_tx_code(sampled.code), state, sampled.detail)
    updated = replace(state, book=sampled.book)
    invalid = _validate_state(updated)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    return BridgeResult(
        BridgeCode.OK, updated, sampled_value=sampled.value
    )


def commit_marker_with_resource(
    state: TransactionResourceState, marker_id: str
) -> BridgeResult:
    """Commit one transaction marker and its mapped resource effect atomically."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    effect_id = next(
        (effect for owner, marker, effect in state.marker_effects
         if owner == tx.spec.identity and marker == marker_id),
        None,
    )
    if effect_id is None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, "commit marker has no effect mapping")
    committed_tx = commit_tx_marker(state.book, tx.spec.identity, marker_id)
    if committed_tx.code is not TxCode.OK:
        return BridgeResult(_tx_code(committed_tx.code), state, committed_tx.detail)
    committed_resource = commit_resource_effect(state.resources, effect_id)
    if committed_resource.code is not ResourceCode.OK:
        # Do not publish the transaction marker if its paired effect cannot commit.
        return BridgeResult(_resource_code(committed_resource.code), state, committed_resource.detail)
    updated = replace(
        state, book=committed_tx.book, resources=committed_resource.ledger
    )
    invalid = _validate_state(updated)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    return BridgeResult(BridgeCode.OK, updated)


def fail_transaction_with_resource(
    state: TransactionResourceState, error: str
) -> BridgeResult:
    """Fail both ledgers atomically, retaining the exact committed/pending prefixes."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    failed_tx = fail_tx(state.book, tx.spec.identity, error)
    if failed_tx.code is not TxCode.OK:
        return BridgeResult(_tx_code(failed_tx.code), state, failed_tx.detail)
    failed_resource = fail_resource(state.resources, error)
    if failed_resource.code is not ResourceCode.OK:
        return BridgeResult(_resource_code(failed_resource.code), state, failed_resource.detail)
    updated = replace(
        state, book=failed_tx.book, resources=failed_resource.ledger
    )
    invalid = _validate_state(updated)
    if invalid is not None:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid)
    return BridgeResult(BridgeCode.OK, updated)


def audit_recovery_boundary(state: TransactionResourceState) -> BridgeResult:
    """Report, without mutation, that transaction and resource recovery differ."""
    invalid = _validate_state(state)
    if invalid is not None or not state.book.transactions:
        return BridgeResult(BridgeCode.INVALID_INPUT, state, invalid or "no transaction")
    tx = state.book.transactions[0]
    recovered_tx = recover_tx(state.book, tx.spec.identity)
    recovered_resource = recover_resource(state.resources)
    if recovered_tx.code is not TxCode.OK or recovered_resource.code is not ResourceCode.OK:
        return BridgeResult(
            BridgeCode.INVALID_INPUT, state,
            "both ledgers must be Failed before recovery boundary audit",
        )
    audit = RecoveryBoundaryAudit(
        transaction_after=recovered_tx.book,
        resource_after=recovered_resource.ledger,
        not_equivalent=(
            "TransactionBook.recover returns the transaction to InProgress and preserves uncommitted commit markers",
            "ResourceLedger.recover terminally completes the ledger and commits every pending effect",
            "no one-to-one recovery transition is defined; applying both would desynchronize marker and effect ledgers",
        ),
    )
    return BridgeResult(
        BridgeCode.INCOMPARABLE_RECOVERY, state,
        "recovery semantics are not equivalent; original composite state preserved",
        recovery_audit=audit,
    )
