"""Independent disposable R3 candidate for the generic R2-T15 resource ledger.

This ledger models an explicit abstract budget. It is deliberately separate from
TransactionBook: capacity units are supplied by the caller and are not a
transaction-count, memory-size, bus-cycle, or hardware resource limit.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum


class ResourceCode(str, Enum):
    OK = "Ok"
    INVALID_INPUT = "InvalidInput"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    INVARIANT_VIOLATION = "InvariantViolation"


@dataclass(frozen=True)
class ResourceLedger:
    capacity: int
    used: int = 0
    reservations: tuple[tuple[str, int, tuple[str, ...]], ...] = ()
    pending_effects: tuple[str, ...] = ()
    committed_effects: tuple[str, ...] = ()
    failed: bool = False
    completed: bool = False
    error: str | None = None


@dataclass(frozen=True)
class ResourceResult:
    code: ResourceCode
    ledger: ResourceLedger
    detail: str = ""


def _validate(ledger: object) -> str | None:
    """Check budget conservation, reservation uniqueness and effect partitions."""
    if not isinstance(ledger, ResourceLedger):
        return "invalid resource ledger"
    if (
        type(ledger.capacity) is not int
        or ledger.capacity < 0
        or type(ledger.used) is not int
        or ledger.used < 0
        or ledger.used > ledger.capacity
    ):
        return "invalid resource counters"
    if any(type(items) is not tuple for items in (
        ledger.reservations, ledger.pending_effects, ledger.committed_effects
    )):
        return "invalid resource collections"

    reservation_ids: set[str] = set()
    reserved_total = 0
    for item in ledger.reservations:
        if (
            type(item) is not tuple
            or len(item) != 3
            or type(item[0]) is not str
            or not item[0].strip()
            or type(item[1]) is not int
            or item[1] <= 0
            or type(item[2]) is not tuple
            or not item[2]
            or any(type(effect) is not str or not effect.strip() for effect in item[2])
            or len(item[2]) != len(set(item[2]))
            or item[0] in reservation_ids
        ):
            return "invalid or duplicate reservation"
        reservation_ids.add(item[0])
        reserved_total += item[1]
    if reserved_total != ledger.used:
        return "reservation total disagrees with usage"

    pending = ledger.pending_effects
    committed = ledger.committed_effects
    if any(type(effect) is not str or not effect.strip() for effect in pending + committed):
        return "invalid effect identity"
    if len(pending) != len(set(pending)) or len(committed) != len(set(committed)):
        return "duplicate effect identity"
    if set(pending).intersection(committed):
        return "effect both pending and committed"
    if type(ledger.failed) is not bool or type(ledger.completed) is not bool:
        return "invalid resource state"
    if ledger.failed and (type(ledger.error) is not str or not ledger.error.strip()):
        return "failed ledger lacks error"
    if ledger.completed and (ledger.failed or pending):
        return "invalid completed ledger"
    return None


def reserve(
    ledger: ResourceLedger,
    identity: str,
    amount: int,
    effects: tuple[str, ...],
) -> ResourceResult:
    """Atomically reserve an explicit budget and publish its pending effects."""
    invalid = _validate(ledger)
    if (
        invalid is not None
        or type(identity) is not str
        or not identity.strip()
        or type(amount) is not int
        or amount <= 0
        or type(effects) is not tuple
        or not effects
        or any(type(effect) is not str or not effect.strip() for effect in effects)
        or len(effects) != len(set(effects))
    ):
        return ResourceResult(
            ResourceCode.INVALID_INPUT, ledger,
            invalid or "invalid reservation descriptor",
        )

    previous = next((item for item in ledger.reservations if item[0] == identity), None)
    if previous is not None:
        if previous[1:] != (amount, effects):
            return ResourceResult(
                ResourceCode.INVALID_INPUT, ledger,
                "reservation identity reused with divergent descriptor",
            )
        return ResourceResult(ResourceCode.OK, ledger)
    if ledger.failed or ledger.completed:
        return ResourceResult(
            ResourceCode.INVARIANT_VIOLATION, ledger,
            "terminal resource ledger cannot reserve",
        )
    if ledger.used + amount > ledger.capacity:
        return ResourceResult(
            ResourceCode.RESOURCE_EXHAUSTED, ledger,
            "finite resource capacity exhausted",
        )
    occupied = set(ledger.pending_effects) | set(ledger.committed_effects)
    if occupied.intersection(effects):
        return ResourceResult(
            ResourceCode.INVALID_INPUT, ledger,
            "effect identity already exists",
        )

    updated = replace(
        ledger,
        used=ledger.used + amount,
        reservations=ledger.reservations + ((identity, amount, effects),),
        pending_effects=ledger.pending_effects + effects,
    )
    return ResourceResult(ResourceCode.OK, updated)


def commit_effect(ledger: ResourceLedger, identity: str) -> ResourceResult:
    """Commit a pending effect once; retries of committed IDs are idempotent."""
    invalid = _validate(ledger)
    if invalid is not None or type(identity) is not str or not identity.strip():
        return ResourceResult(
            ResourceCode.INVALID_INPUT, ledger,
            invalid or "invalid effect identity",
        )
    if identity in ledger.committed_effects:
        return ResourceResult(ResourceCode.OK, ledger)
    if identity not in ledger.pending_effects or ledger.failed or ledger.completed:
        return ResourceResult(
            ResourceCode.INVARIANT_VIOLATION, ledger,
            "effect is not pending on an active ledger",
        )
    updated = replace(
        ledger,
        pending_effects=tuple(effect for effect in ledger.pending_effects if effect != identity),
        committed_effects=ledger.committed_effects + (identity,),
    )
    return ResourceResult(ResourceCode.OK, updated)


def fail(ledger: ResourceLedger, error: str) -> ResourceResult:
    """Record failure while retaining already committed and pending effects."""
    invalid = _validate(ledger)
    if invalid is not None or type(error) is not str or not error.strip():
        return ResourceResult(
            ResourceCode.INVALID_INPUT, ledger,
            invalid or "invalid failure",
        )
    if ledger.failed or ledger.completed:
        return ResourceResult(
            ResourceCode.INVARIANT_VIOLATION, ledger,
            "terminal ledger cannot fail again",
        )
    return ResourceResult(ResourceCode.OK, replace(ledger, failed=True, error=error))


def recover(ledger: ResourceLedger) -> ResourceResult:
    """Commit only the pending suffix after failure and finish the ledger."""
    invalid = _validate(ledger)
    if invalid is not None:
        return ResourceResult(ResourceCode.INVALID_INPUT, ledger, invalid)
    if not ledger.failed:
        return ResourceResult(
            ResourceCode.INVARIANT_VIOLATION, ledger,
            "recovery requires Failed",
        )
    updated = replace(
        ledger,
        pending_effects=(),
        committed_effects=ledger.committed_effects + ledger.pending_effects,
        failed=False,
        completed=True,
    )
    return ResourceResult(ResourceCode.OK, updated)
