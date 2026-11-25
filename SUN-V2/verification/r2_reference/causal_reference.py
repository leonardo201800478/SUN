"""Serial causal-synchronization oracle for R2-T05.

This deliberately slow model resolves declared transition dependencies by a
full scan. It is verification-only and has no scheduler/runtime integration.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from fractions import Fraction


class SyncCode(str, Enum):
    SYNCHRONIZED = "Synchronized"
    BLOCKED = "Blocked"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"
    INDETERMINATE = "IndeterminateCausalOrder"


@dataclass(frozen=True)
class Transition:
    """One device-owned transition at an exact common-time coordinate."""

    identity: str
    at_common: Fraction
    requires: tuple[str, ...] = ()
    commutative: bool = False


@dataclass(frozen=True)
class Device:
    """Reference state for one participant in a causal synchronization."""

    name: str
    progress_common: Fraction
    transitions: tuple[Transition, ...] = ()
    applied: tuple[str, ...] = ()


@dataclass(frozen=True)
class CausalState:
    """Immutable collection of devices; no host or runtime state is present."""

    devices: tuple[Device, ...]


@dataclass(frozen=True)
class SyncResult:
    """Outcome plus the exact state and logical transition order."""

    code: SyncCode
    state: CausalState
    applied_order: tuple[str, ...] = ()
    detail: str = ""


def _exact_time(value: object) -> bool:
    """Accept exact integers/Fractions while excluding booleans and floats."""
    return type(value) is int or type(value) is Fraction


def _validate(state: object) -> str | None:
    """Validate the complete immutable graph before resolving any dependency."""
    if not isinstance(state, CausalState) or type(state.devices) is not tuple:
        return "invalid causal state"
    names: set[str] = set()
    identities: set[str] = set()
    descriptors: dict[str, tuple[str, Transition]] = {}
    applied_entries: list[tuple[str, str]] = []
    for device in state.devices:
        if not isinstance(device, Device):
            return "invalid device member"
        if type(device.name) is not str or not device.name.strip():
            return "invalid device identity"
        if device.name in names:
            return "duplicate device identity"
        names.add(device.name)
        if not _exact_time(device.progress_common):
            return "invalid device progress"
        if type(device.transitions) is not tuple or type(device.applied) is not tuple:
            return "invalid device collections"
        if any(type(item) is not str for item in device.applied):
            return "invalid applied identity"
        if len(set(device.applied)) != len(device.applied):
            return "duplicate applied identity"
        for transition in device.transitions:
            if not isinstance(transition, Transition):
                return "invalid transition member"
            if type(transition.identity) is not str or not transition.identity.strip():
                return "invalid transition identity"
            if transition.identity in identities:
                return "duplicate transition identity"
            identities.add(transition.identity)
            descriptors[transition.identity] = (device.name, transition)
            if not _exact_time(transition.at_common):
                return "invalid transition time"
            if type(transition.requires) is not tuple or any(
                type(required) is not str or not required.strip()
                for required in transition.requires
            ):
                return "invalid transition dependencies"
            if len(set(transition.requires)) != len(transition.requires):
                return "duplicate transition dependency"
            if type(transition.commutative) is not bool:
                return "invalid commutativity declaration"
    for device in state.devices:
        applied_entries.extend((device.name, identity) for identity in device.applied)
        for transition in device.transitions:
            if (
                transition.identity not in device.applied
                and transition.at_common < device.progress_common
            ):
                return "unapplied transition is behind owner progress"
    applied_ids = [identity for _, identity in applied_entries]
    if len(applied_ids) != len(set(applied_ids)):
        return "applied transition is recorded by multiple devices"
    if not identities.issuperset(applied_ids):
        return "applied transition has no descriptor"
    applied = set(applied_ids)
    for owner, identity in applied_entries:
        descriptor = descriptors[identity]
        transition_owner, transition = descriptor
        if transition_owner != owner:
            return "applied transition is recorded by a non-owner device"
        if transition.at_common > next(
            device.progress_common for device in state.devices if device.name == owner
        ):
            return "applied transition is ahead of owner progress"
        for required in transition.requires:
            dependency = descriptors.get(required)
            if required not in applied or dependency is None:
                return "applied transition has an unresolved dependency"
            if dependency[1].at_common > transition.at_common:
                return "applied transition depends on a later transition"
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit_applied(identity: str) -> bool:
        if identity in visiting:
            return False
        if identity in visited:
            return True
        visiting.add(identity)
        for required in descriptors[identity][1].requires:
            if required in applied and not visit_applied(required):
                return False
        visiting.remove(identity)
        visited.add(identity)
        return True

    if any(not visit_applied(identity) for identity in applied):
        return "applied transition dependency graph contains a cycle"
    return None


def synchronize(
    state: CausalState,
    *,
    target_device: str,
    at_common: Fraction,
    observes: tuple[str, ...] = (),
    alters: tuple[str, ...] = (),
) -> SyncResult:
    """Synchronize only declared participants to an observation/commit point.

    All required transitions are resolved before any state is changed. Missing
    dependencies and dependency cycles return Blocked atomically. Transitions
    after the requested instant are never applied.
    """
    invalid = _validate(state)
    if invalid is not None:
        return SyncResult(SyncCode.INVALID_INPUT, state, detail=invalid)
    if (
        type(target_device) is not str
        or not target_device.strip()
        or not _exact_time(at_common)
        or type(observes) is not tuple
        or type(alters) is not tuple
        or any(type(name) is not str or not name.strip() for name in observes + alters)
    ):
        return SyncResult(SyncCode.INVALID_INPUT, state, detail="invalid operation descriptor")

    devices = {device.name: device for device in state.devices}
    if target_device not in devices:
        return SyncResult(SyncCode.BLOCKED, state, detail="target participant is unavailable")
    participants = {target_device, *observes, *alters}
    if not participants.issubset(devices):
        return SyncResult(SyncCode.BLOCKED, state, detail="declared participant is unavailable")
    if any(devices[name].progress_common > at_common for name in participants):
        return SyncResult(SyncCode.LATE_INPUT, state, detail="requested point is behind participant progress")

    descriptors = {
        transition.identity: (device.name, transition)
        for device in state.devices
        for transition in device.transitions
    }
    applied = {
        identity for device in state.devices for identity in device.applied
    }
    ordered: list[str] = []
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(identity: str) -> SyncCode | None:
        if identity in applied or identity in visited:
            return None
        if identity in visiting:
            return SyncCode.BLOCKED
        descriptor = descriptors.get(identity)
        if descriptor is None:
            return SyncCode.BLOCKED
        owner, transition = descriptor
        if owner not in participants or transition.at_common > at_common:
            return SyncCode.BLOCKED
        visiting.add(identity)
        for required in transition.requires:
            dependency = descriptors.get(required)
            if required not in applied:
                if dependency is None or dependency[0] not in participants:
                    return SyncCode.BLOCKED
                if dependency[1].at_common > at_common:
                    return SyncCode.BLOCKED
                outcome = visit(required)
                if outcome is not None:
                    return outcome
        visiting.remove(identity)
        visited.add(identity)
        ordered.append(identity)
        return None

    due = [
        transition.identity
        for name in participants
        for transition in devices[name].transitions
        if transition.at_common <= at_common and transition.identity not in applied
    ]
    for identity in due:
        outcome = visit(identity)
        if outcome is not None:
            detail = "dependency cycle or unresolved/out-of-cut dependency"
            return SyncResult(outcome, state, detail=detail)

    by_time: dict[Fraction, list[str]] = {}
    for identity in ordered:
        transition = descriptors[identity][1]
        by_time.setdefault(transition.at_common, []).append(identity)

    # Canonical topological scheduling: dependencies establish order; only
    # simultaneously-ready incomparable transitions may be sorted, and only
    # when every pair in that ready set is explicitly commutative.
    canonical_order: list[str] = []
    resolved = set(applied)
    for instant in sorted(by_time):
        remaining = set(by_time[instant])
        while remaining:
            ready = [
                identity for identity in remaining
                if all(required in resolved for required in descriptors[identity][1].requires)
            ]
            if not ready:
                return SyncResult(
                    SyncCode.BLOCKED,
                    state,
                    detail="same-instant dependency graph has no ready transition",
                )
            ready.sort()
            for index, left in enumerate(ready):
                for right in ready[index + 1:]:
                    left_transition = descriptors[left][1]
                    right_transition = descriptors[right][1]
                    if not (left_transition.commutative and right_transition.commutative):
                        return SyncResult(
                            SyncCode.INDETERMINATE,
                            state,
                            detail="simultaneous incomparable non-commutative transitions have no declared order",
                        )
            canonical_order.extend(ready)
            resolved.update(ready)
            remaining.difference_update(ready)

    updated: list[Device] = []
    for device in state.devices:
        if device.name in participants:
            new_applied = tuple(
                sorted(set(device.applied).union(
                    identity for identity in canonical_order
                    if descriptors[identity][0] == device.name
                ))
            )
            updated.append(
                Device(device.name, at_common, device.transitions, new_applied)
            )
        else:
            updated.append(device)
    return SyncResult(
        SyncCode.SYNCHRONIZED,
        CausalState(tuple(updated)),
        tuple(canonical_order),
    )
