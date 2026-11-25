"""Finite-capacity and partial-effect ledger oracle for R2-T15."""
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
class ResourceEvaluation:
    code: ResourceCode
    ledger: ResourceLedger
    detail: str = ""


def _validate(ledger: object) -> str | None:
    """Validate finite counters and the exact pending/committed reservation ledger."""
    if not isinstance(ledger, ResourceLedger):
        return "invalid resource ledger"
    if (
        type(ledger.capacity) is not int or ledger.capacity < 0
        or type(ledger.used) is not int or ledger.used < 0 or ledger.used > ledger.capacity
    ):
        return "invalid resource counters"
    if (
        type(ledger.reservations) is not tuple
        or type(ledger.pending_effects) is not tuple
        or type(ledger.committed_effects) is not tuple
    ):
        return "invalid resource collections"
    if any(
        type(item) is not tuple or len(item) != 3
        or type(item[0]) is not str or not item[0].strip()
        or type(item[1]) is not int or item[1] <= 0
        or type(item[2]) is not tuple or not item[2]
        or any(type(effect) is not str or not effect.strip() for effect in item[2])
        or len(set(item[2])) != len(item[2])
        for item in ledger.reservations
    ):
        return "invalid reservation"
    if len({item[0] for item in ledger.reservations}) != len(ledger.reservations):
        return "duplicate reservation identity"
    if any(
        type(effect) is not str or not effect.strip()
        for effect in ledger.pending_effects + ledger.committed_effects
    ):
        return "invalid effect identity"
    if (
        len(set(ledger.pending_effects)) != len(ledger.pending_effects)
        or len(set(ledger.committed_effects)) != len(ledger.committed_effects)
    ):
        return "duplicate effect identity"
    if set(ledger.pending_effects).intersection(ledger.committed_effects):
        return "effect both pending and committed"
    if type(ledger.failed) is not bool or type(ledger.completed) is not bool:
        return "invalid resource state"
    if ledger.failed and (type(ledger.error) is not str or not ledger.error.strip()):
        return "failed ledger lacks error"
    if ledger.completed and (ledger.failed or ledger.pending_effects):
        return "invalid completed ledger"
    if sum(amount for _, amount, _ in ledger.reservations) != ledger.used:
        return "reservation total disagrees with usage"
    return None


def reserve(
    ledger: ResourceLedger,
    identity: str,
    amount: int,
    effects: tuple[str, ...],
) -> ResourceEvaluation:
    """Reserve a finite budget atomically before creating pending effects."""
    invalid = _validate(ledger)
    if (
        invalid
        or type(identity) is not str or not identity.strip()
        or type(amount) is not int or amount <= 0
        or type(effects) is not tuple or not effects
        or any(type(effect) is not str or not effect.strip() for effect in effects)
        or len(set(effects)) != len(effects)
    ):
        return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, invalid or "invalid reservation descriptor")
    prior = next((item for item in ledger.reservations if item[0] == identity), None)
    if prior is not None:
        if prior[1] != amount or prior[2] != effects:
            return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, "reservation identity reused with divergent descriptor")
        return ResourceEvaluation(ResourceCode.OK, ledger)
    if ledger.failed or ledger.completed:
        return ResourceEvaluation(ResourceCode.INVARIANT_VIOLATION, ledger, "terminal resource ledger cannot reserve")
    if ledger.used + amount > ledger.capacity:
        return ResourceEvaluation(ResourceCode.RESOURCE_EXHAUSTED, ledger, "finite resource capacity exhausted")
    if set(effects).intersection(ledger.pending_effects + ledger.committed_effects):
        return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, "effect identity already exists")
    updated = replace(
        ledger,
        used=ledger.used + amount,
        reservations=ledger.reservations + ((identity, amount, effects),),
        pending_effects=ledger.pending_effects + effects,
    )
    return ResourceEvaluation(ResourceCode.OK, updated)


def commit_effect(ledger: ResourceLedger, identity: str) -> ResourceEvaluation:
    """Move one pending effect to the committed ledger exactly once."""
    invalid = _validate(ledger)
    if invalid or type(identity) is not str or not identity.strip():
        return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, invalid or "invalid effect identity")
    if identity in ledger.committed_effects:
        return ResourceEvaluation(ResourceCode.OK, ledger)
    if identity not in ledger.pending_effects or ledger.failed or ledger.completed:
        return ResourceEvaluation(ResourceCode.INVARIANT_VIOLATION, ledger, "effect is not pending on an active ledger")
    updated = replace(
        ledger,
        pending_effects=tuple(effect for effect in ledger.pending_effects if effect != identity),
        committed_effects=ledger.committed_effects + (identity,),
    )
    return ResourceEvaluation(ResourceCode.OK, updated)


def fail(ledger: ResourceLedger, error: str) -> ResourceEvaluation:
    """Record failure without rolling back any already committed effect."""
    invalid = _validate(ledger)
    if invalid or type(error) is not str or not error.strip():
        return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, invalid or "invalid failure")
    if ledger.failed or ledger.completed:
        return ResourceEvaluation(ResourceCode.INVARIANT_VIOLATION, ledger, "terminal ledger cannot fail again")
    return ResourceEvaluation(ResourceCode.OK, replace(ledger, failed=True, error=error))


def recover(ledger: ResourceLedger) -> ResourceEvaluation:
    """Explicitly commit only pending effects; previously committed IDs are never replayed."""
    invalid = _validate(ledger)
    if invalid:
        return ResourceEvaluation(ResourceCode.INVALID_INPUT, ledger, invalid)
    if not ledger.failed:
        return ResourceEvaluation(ResourceCode.INVARIANT_VIOLATION, ledger, "recovery requires Failed")
    updated = replace(
        ledger,
        pending_effects=(),
        committed_effects=ledger.committed_effects + ledger.pending_effects,
        failed=False,
        completed=True,
    )
    return ResourceEvaluation(ResourceCode.OK, updated)
