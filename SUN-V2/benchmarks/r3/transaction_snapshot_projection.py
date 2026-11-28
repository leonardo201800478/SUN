"""Explicit partial R3 TransactionBook -> R2-T11 TransactionRecord projection.

This adapter maps only fields with matching state semantics and representation.
It does not equate commit-marker IDs with device effect IDs, nor infer pending
effects from the candidate transaction descriptor.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from r3_transaction_core import (
    TransactionBook, TxState, _validate_book,
)
from snapshot_reference import TransactionRecord, TransactionState


class TransactionProjectionCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class TransactionProjection:
    records: tuple[TransactionRecord, ...]
    unrepresented: tuple[str, ...]


@dataclass(frozen=True)
class TransactionProjectionResult:
    code: TransactionProjectionCode
    projection: TransactionProjection | None = None
    detail: str = ""


_STATE_MAP = {
    TxState.CREATED: TransactionState.CREATED,
    TxState.IN_PROGRESS: TransactionState.IN_PROGRESS,
    TxState.WAITING: TransactionState.WAITING,
    TxState.READY: TransactionState.READY,
    TxState.COMPLETED: TransactionState.COMPLETED,
    TxState.CANCELLED: TransactionState.CANCELLED,
    TxState.FAILED: TransactionState.FAILED,
}


def project_transaction_book(book: TransactionBook) -> TransactionProjectionResult:
    """Project common transaction identity, lifecycle and sampled-state fields."""
    if not isinstance(book, TransactionBook):
        return TransactionProjectionResult(
            TransactionProjectionCode.INVALID_INPUT,
            detail="expected TransactionBook",
        )
    invalid = _validate_book(book)
    if invalid is not None:
        return TransactionProjectionResult(
            TransactionProjectionCode.INVALID_INPUT, detail=invalid
        )

    records = tuple(
        TransactionRecord(
            identity=tx.spec.identity,
            generation=tx.spec.generation,
            state=_STATE_MAP[tx.state],
            progress_common=tx.progress_common,
            next_marker=tx.ready_marker,
            wait_source=tx.wait_source,
            sampled=tx.sampled,
            error=tx.failure,
        )
        for tx in sorted(book.transactions, key=lambda item: item.spec.identity)
    )
    return TransactionProjectionResult(
        TransactionProjectionCode.OK,
        TransactionProjection(
            records=records,
            unrepresented=(
                "TxSpec origin/destination/operation/profile/payload and admission/start descriptors",
                "TxSpec base duration and automatic/external TW budgets",
                "TxSpec marker schedule and irreversible-marker declarations",
                "Transaction.base_consumed and automatic/external TW consumption ledgers",
                "Transaction.committed contains marker IDs, not proven R2 device effect IDs",
                "R2 committed_effects and pending_effects have no candidate-equivalent ledger",
                "R2 deadline/domain/interrupt/input replay state is outside TransactionBook",
            ),
        ),
    )
