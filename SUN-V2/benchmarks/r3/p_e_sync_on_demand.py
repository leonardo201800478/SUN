"""Disposable P-E candidate: synchronization-on-demand.

The coordinator computes a declared dependency closure only for the requested
observation. There is no global progress agenda and unrelated domains are not
advanced. Time coordinates are exact and UNKNOWN event pairs remain unresolved.
"""
from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import Enum
from fractions import Fraction
from types import MappingProxyType


class SyncCode(str, Enum):
    REACHED = "Reached"
    STOPPED_AT = "StoppedAt"
    BLOCKED = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"
    LATE_INPUT = "LateInput"
    INVALID_INPUT = "InvalidInput"


class PairRelation(str, Enum):
    FIRST_BEFORE_SECOND = "first_before_second"
    SECOND_BEFORE_FIRST = "second_before_first"
    COMMUTATIVE = "commutative"
    UNKNOWN = "unknown"


@dataclass(frozen=True)
class SyncDomain:
    identity: str
    progress: Fraction
    safe_horizon: Fraction


@dataclass(frozen=True)
class SyncPhaseBasis:
    profile_id: str
    profile_revision: str
    coordinate: str
    unit: str
    scale_numerator: int
    scale_denominator: int
    basis_id: str
    basis_revision: str
    validation_record_id: str
    validation_record_sha256: str


@dataclass(frozen=True)
class SyncTransition:
    identity: str
    owner: str
    at_common: Fraction
    depends_on: tuple[str, ...] = ()
    microstep: int = 0
    phase_order: int | None = None
    phase_evidence: str | None = None
    phase_basis: SyncPhaseBasis | None = None


@dataclass(frozen=True)
class SyncConstraint:
    first_id: str
    second_id: str
    relation: PairRelation


@dataclass(frozen=True)
class SyncState:
    domains: tuple[SyncDomain, ...]
    transitions: tuple[SyncTransition, ...]
    constraints: tuple[SyncConstraint, ...] = ()
    observed: tuple[str, ...] = ()
    _transition_by_id: Mapping[str, SyncTransition] = field(
        default_factory=lambda: MappingProxyType({}), repr=False, compare=False
    )
    _transitions_by_owner: Mapping[str, tuple[SyncTransition, ...]] = field(
        default_factory=lambda: MappingProxyType({}), repr=False, compare=False
    )
    _relation_index: Mapping[frozenset[str], SyncConstraint] = field(
        default_factory=lambda: MappingProxyType({}), repr=False, compare=False
    )
    _validated: bool = field(default=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        if not self._transition_by_id and self.transitions:
            by_id = {
                item.identity: item for item in self.transitions
                if isinstance(item, SyncTransition) and isinstance(item.identity, str)
            }
            object.__setattr__(self, "_transition_by_id", MappingProxyType(by_id))
        if not self._transitions_by_owner and self.transitions:
            grouped: dict[str, list[SyncTransition]] = {}
            for item in self.transitions:
                if isinstance(item, SyncTransition) and isinstance(item.owner, str):
                    grouped.setdefault(item.owner, []).append(item)
            object.__setattr__(
                self,
                "_transitions_by_owner",
                MappingProxyType({key: tuple(value) for key, value in grouped.items()}),
            )
        if not self._relation_index and self.constraints:
            relations = {
                frozenset((item.first_id, item.second_id)): item
                for item in self.constraints
                if isinstance(item, SyncConstraint)
                and isinstance(item.first_id, str)
                and isinstance(item.second_id, str)
            }
            object.__setattr__(self, "_relation_index", MappingProxyType(relations))
        # The state is immutable after construction. Validate once here so
        # repeated sync cuts can reuse the immutable indexes without rescanning
        # the entire state on every observation.
        if not self._validated and _validate(self) is None:
            object.__setattr__(self, "_validated", True)


@dataclass(frozen=True)
class SyncResult:
    state: SyncState
    code: SyncCode
    reached_common: Fraction | None = None
    synchronized_domains: tuple[str, ...] = ()
    detail: str = ""


def _exact(value: object) -> bool:
    """Accept exact integer/rational coordinates only."""
    return type(value) in (int, Fraction)


def _valid_phase_basis(value: object) -> bool:
    """Validate descriptor structure; do not treat it as external authentication."""
    if not isinstance(value, SyncPhaseBasis):
        return False
    strings = (
        value.profile_id, value.profile_revision, value.coordinate, value.unit,
        value.basis_id, value.basis_revision, value.validation_record_id,
        value.validation_record_sha256,
    )
    return (
        all(isinstance(item, str) and item.strip() for item in strings)
        and type(value.scale_numerator) is int and value.scale_numerator > 0
        and type(value.scale_denominator) is int and value.scale_denominator > 0
        and len(value.validation_record_sha256) == 64
        and all(char in "0123456789abcdef"
                for char in value.validation_record_sha256)
    )


def _phase_relation(first: SyncTransition, second: SyncTransition) -> PairRelation:
    """Compare phase order only on one identical, structurally valid basis."""
    if (
        first.phase_order is None or second.phase_order is None
        or first.phase_evidence is None or second.phase_evidence is None
        or not first.phase_evidence.strip() or not second.phase_evidence.strip()
        or not _valid_phase_basis(first.phase_basis)
        or first.phase_basis != second.phase_basis
        or first.phase_order == second.phase_order
    ):
        return PairRelation.UNKNOWN
    return (
        PairRelation.FIRST_BEFORE_SECOND
        if first.phase_order < second.phase_order
        else PairRelation.SECOND_BEFORE_FIRST
    )


def _validate(state: object) -> str | None:
    """Validate domain progress, transition ownership and dependency identities."""
    if not isinstance(state, SyncState):
        return "invalid sync state"
    if type(state.domains) is not tuple or type(state.transitions) is not tuple:
        return "state collections must be immutable tuples"
    domain_ids: set[str] = set()
    for domain in state.domains:
        if (
            not isinstance(domain, SyncDomain)
            or not isinstance(domain.identity, str) or not domain.identity.strip()
            or not _exact(domain.progress) or not _exact(domain.safe_horizon)
            or domain.progress > domain.safe_horizon
            or domain.identity in domain_ids
        ):
            return "invalid or duplicated domain state"
        domain_ids.add(domain.identity)
    transition_ids: set[str] = set()
    for transition in state.transitions:
        if (
            not isinstance(transition, SyncTransition)
            or not isinstance(transition.identity, str) or not transition.identity.strip()
            or not isinstance(transition.owner, str) or transition.owner not in domain_ids
            or not _exact(transition.at_common)
            or type(transition.microstep) is not int or transition.microstep < 0
            or (transition.phase_order is not None and type(transition.phase_order) is not int)
            or (transition.phase_evidence is not None and (
                not isinstance(transition.phase_evidence, str) or not transition.phase_evidence.strip()
            ))
            or ((transition.phase_order is None) != (transition.phase_evidence is None))
            or (transition.phase_basis is not None and (
                not _valid_phase_basis(transition.phase_basis)
                or transition.phase_order is None or transition.phase_evidence is None
            ))
            or type(transition.depends_on) is not tuple
            or any(not isinstance(item, str) or not item.strip()
                   for item in transition.depends_on)
            or transition.identity in transition_ids
            or transition.identity in transition.depends_on
            or len(set(transition.depends_on)) != len(transition.depends_on)
        ):
            return "invalid or duplicated transition descriptor"
        transition_ids.add(transition.identity)
    transition_by_id = state._transition_by_id
    for transition in state.transitions:
        if any(item not in transition_ids for item in transition.depends_on):
            return "transition dependency does not exist"
        if any(
            transition_by_id[item].at_common > transition.at_common
            for item in transition.depends_on
        ):
            return "causal predecessor is later than its dependent transition"
    if type(state.observed) is not tuple or any(
        not isinstance(item, str) or item not in transition_ids
        for item in state.observed
    ):
        return "invalid observed transition ledger"
    if len(set(state.observed)) != len(state.observed):
        return "duplicate observed transition"
    observed_set = set(state.observed)
    if any(
        dependency not in observed_set
        for identity in observed_set
        for dependency in transition_by_id[identity].depends_on
    ):
        return "observed transition is missing an observed predecessor"
    if type(state.constraints) is not tuple:
        return "constraints must be an immutable tuple"
    pairs: set[frozenset[str]] = set()
    for item in state.constraints:
        if (
            not isinstance(item, SyncConstraint)
            or not isinstance(item.first_id, str) or item.first_id not in transition_ids
            or not isinstance(item.second_id, str) or item.second_id not in transition_ids
            or item.first_id == item.second_id
            or not isinstance(item.relation, PairRelation)
        ):
            return "invalid pairwise relation descriptor"
        pair = frozenset((item.first_id, item.second_id))
        if pair in pairs:
            return "duplicate unordered pair relation"
        pairs.add(pair)
    return None


def _relation(state: SyncState, first: str, second: str) -> PairRelation:
    """Read a declared relation, or UNKNOWN when no pair evidence exists."""
    item = state._relation_index.get(frozenset((first, second)))
    if item is None:
        return PairRelation.UNKNOWN
    if (item.first_id, item.second_id) == (first, second):
        return item.relation
    if item.relation is PairRelation.FIRST_BEFORE_SECOND:
        return PairRelation.SECOND_BEFORE_FIRST
    if item.relation is PairRelation.SECOND_BEFORE_FIRST:
        return PairRelation.FIRST_BEFORE_SECOND
    return item.relation


def _dependency_order(first: SyncTransition, second: SyncTransition) -> PairRelation:
    """Return the causal relation induced by declared transition dependencies."""
    if first.identity in second.depends_on:
        return PairRelation.FIRST_BEFORE_SECOND
    if second.identity in first.depends_on:
        return PairRelation.SECOND_BEFORE_FIRST
    return PairRelation.UNKNOWN


def _closure(
    state: SyncState, requested: tuple[str, ...], target: Fraction
) -> tuple[set[str], set[str]] | None:
    """Compute only the requested domains and transitive causal predecessors."""
    selected_domains = set(requested)
    selected_transitions: set[str] = set()
    queue: list[SyncTransition] = []

    def include_owner(owner: str) -> None:
        for transition in sorted(
            state._transitions_by_owner.get(owner, ()),
            key=lambda item: (item.at_common, item.identity),
        ):
            if (
                transition.at_common <= target
                and transition.identity not in selected_transitions
            ):
                selected_transitions.add(transition.identity)
                queue.append(transition)

    for owner in sorted(selected_domains):
        include_owner(owner)

    cursor = 0
    while cursor < len(queue):
        transition = queue[cursor]
        cursor += 1
        for dependency_id in transition.depends_on:
            dependency = state._transition_by_id[dependency_id]
            if dependency.at_common > target:
                return None
            if dependency.owner not in selected_domains:
                selected_domains.add(dependency.owner)
                include_owner(dependency.owner)
            if dependency.identity not in selected_transitions:
                selected_transitions.add(dependency.identity)
                queue.append(dependency)

    return selected_domains, selected_transitions

def _choose(
    state: SyncState, group: tuple[SyncTransition, ...]
) -> SyncTransition | None:
    """Choose a minimal same-instant transition using one indexed relation view."""
    if len(group) == 1:
        return group[0]
    incoming = {item.identity: 0 for item in group}
    by_id = {item.identity: item for item in group}
    relation_index = state._relation_index

    def declared_relation(first_id: str, second_id: str) -> PairRelation:
        item = relation_index.get(frozenset((first_id, second_id)))
        if item is None:
            return PairRelation.UNKNOWN
        if (item.first_id, item.second_id) == (first_id, second_id):
            return item.relation
        if item.relation is PairRelation.FIRST_BEFORE_SECOND:
            return PairRelation.SECOND_BEFORE_FIRST
        if item.relation is PairRelation.SECOND_BEFORE_FIRST:
            return PairRelation.FIRST_BEFORE_SECOND
        return item.relation

    ids = [item.identity for item in group]
    for index, first_id in enumerate(ids):
        for second_id in ids[index + 1:]:
            first = by_id[first_id]
            second = by_id[second_id]
            relation = _dependency_order(first, second)
            declared = declared_relation(first_id, second_id)
            if relation is PairRelation.UNKNOWN:
                relation = declared
            elif declared is not PairRelation.UNKNOWN and declared is not relation:
                return None
            if relation is PairRelation.UNKNOWN:
                relation = _phase_relation(first, second)
            if relation is PairRelation.UNKNOWN:
                return None
            if (
                relation is PairRelation.FIRST_BEFORE_SECOND
                and first.microstep > second.microstep
            ) or (
                relation is PairRelation.SECOND_BEFORE_FIRST
                and second.microstep > first.microstep
            ):
                return None
            if relation is PairRelation.FIRST_BEFORE_SECOND:
                incoming[second_id] += 1
            elif relation is PairRelation.SECOND_BEFORE_FIRST:
                incoming[first_id] += 1
    ready = sorted(identity for identity, count in incoming.items() if count == 0)
    if not ready:
        return None
    if len(ready) > 1:
        for index, first_id in enumerate(ready):
            for second_id in ready[index + 1:]:
                if (
                    _dependency_order(by_id[first_id], by_id[second_id])
                    is PairRelation.UNKNOWN
                    and declared_relation(first_id, second_id)
                    is not PairRelation.COMMUTATIVE
                ):
                    return None
    return by_id[ready[0]]


def synchronize(
    state: SyncState,
    *,
    requested_domains: tuple[str, ...],
    target_common: int | Fraction,
) -> SyncResult:
    """Advance only the requested domains and their transitive causal closure."""
    invalid = (
        _validate(state)
        if not isinstance(state, SyncState) or not state._validated
        else None
    )
    if invalid is not None:
        return SyncResult(state, SyncCode.INVALID_INPUT, detail=invalid)
    if (
        type(requested_domains) is not tuple or not requested_domains
        or any(not isinstance(item, str) or not item.strip() for item in requested_domains)
        or len(set(requested_domains)) != len(requested_domains)
        or not _exact(target_common)
    ):
        return SyncResult(state, SyncCode.INVALID_INPUT, detail="invalid synchronization request")
    domains = {item.identity: item for item in state.domains}
    if any(item not in domains for item in requested_domains):
        return SyncResult(state, SyncCode.BLOCKED, detail="requested participant is unknown")
    target = Fraction(target_common)
    if any(target < domains[item].progress for item in requested_domains):
        return SyncResult(state, SyncCode.LATE_INPUT, detail="target is behind requested domain progress")
    closure = _closure(state, requested_domains, target)
    if closure is None:
        return SyncResult(state, SyncCode.BLOCKED, detail="dependency closure is incomplete or lies beyond target")
    selected_domains, selected_transitions = closure
    if any(target < domains[item].progress for item in selected_domains):
        return SyncResult(state, SyncCode.LATE_INPUT, detail="target is behind a causal participant")
    frontier = min(target, *(domains[item].safe_horizon for item in selected_domains))
    if any(frontier < domains[item].progress for item in selected_domains):
        return SyncResult(state, SyncCode.INVALID_INPUT, detail="causal safe horizon regresses an existing participant")
    pending = sorted(
        (
            state._transition_by_id[identity]
            for identity in selected_transitions
            if identity not in state.observed
            and state._transition_by_id[identity].at_common <= frontier
        ),
        key=lambda item: (item.at_common, item.identity),
    )
    observed = list(state.observed)
    while pending:
        earliest = min(item.at_common for item in pending)
        group = tuple(item for item in pending if item.at_common == earliest)
        selected = _choose(state, group)
        if selected is None:
            return SyncResult(
                state, SyncCode.INDETERMINATE, detail="same-instant dependency relation is UNKNOWN or cyclic"
            )
        if any(dependency not in observed for dependency in selected.depends_on):
            return SyncResult(
                state, SyncCode.BLOCKED, detail="causal predecessor has not been observed"
            )
        observed.append(selected.identity)
        pending.remove(selected)
    updated_domains = tuple(
        SyncDomain(
            item.identity,
            frontier if item.identity in selected_domains else item.progress,
            item.safe_horizon,
        )
        for item in state.domains
    )
    updated = SyncState(
        updated_domains,
        state.transitions,
        state.constraints,
        tuple(observed),
        state._transition_by_id,
        state._transitions_by_owner,
        state._relation_index,
        True,
    )
    code = SyncCode.REACHED if frontier == target else SyncCode.STOPPED_AT
    return SyncResult(
        updated, code, frontier, tuple(sorted(selected_domains)),
        "requested causal closure synchronized" if code is SyncCode.REACHED
        else "safe horizon stopped the requested causal closure",
    )
