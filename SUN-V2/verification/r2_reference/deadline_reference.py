"""Independent, exact, serial deadline oracle for CHRONOS R2-B.

Verification-only model. It deliberately uses immutable tuples and linear scans;
it does not select a production agenda, phase model, or scheduler architecture.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction
from typing import Iterable

from chronos_reference import ClockDomain, Obligation, ReferenceState


class DeadlineStatus(str, Enum):
    SCHEDULED = "Scheduled"
    CANCEL_PENDING = "CancelPending"
    CANCELLED = "Cancelled"
    CLAIMED = "Claimed"
    DISPATCHED = "Dispatched"
    COMPLETED = "Completed"
    FAILED = "Failed"


class DeadlineCode(str, Enum):
    ACCEPTED = "Accepted"
    BLOCKED = "Blocked"
    INVALID_INPUT = "InvalidInput"
    LATE_INPUT = "LateInput"
    INDETERMINATE = "IndeterminateCausalOrder"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    INVARIANT_VIOLATION = "InvariantViolation"
    STALE_IGNORED = "StaleIgnored"


@dataclass(frozen=True)
class Deadline:
    owner: str
    identity: str
    generation: int
    at_common: Fraction
    effect_id: str
    commutative: bool = False
    phase_order: int | None = None
    phase_evidence: str | None = None
    microstep: int = 0
    status: DeadlineStatus = DeadlineStatus.SCHEDULED


@dataclass(frozen=True)
class Cancellation:
    owner: str
    identity: str
    generation: int
    at_common: Fraction
    microstep_level: int | None = None
    phase_order: int | None = None
    phase_evidence: str | None = None


@dataclass(frozen=True)
class DeadlineBook:
    deadlines: tuple[Deadline, ...] = ()
    pending_cancellations: tuple[Cancellation, ...] = ()
    watermark_common: Fraction | None = None
    max_microsteps: int = 32


@dataclass(frozen=True)
class DeadlineEvaluation:
    book: DeadlineBook
    code: DeadlineCode
    detail: str = ""
    claimed: tuple[tuple[str, int], ...] = ()


def _exact(value: object) -> bool:
    return isinstance(value, (int, Fraction)) and not isinstance(value, bool)


def _replace_deadline(book: DeadlineBook, updated: Deadline) -> DeadlineBook:
    return replace(
        book,
        deadlines=tuple(
            updated if (item.identity, item.generation) ==
            (updated.identity, updated.generation) else item
            for item in book.deadlines
        ),
    )


def _find(book: DeadlineBook, identity: str, generation: int) -> Deadline | None:
    return next(
        (item for item in book.deadlines
         if item.identity == identity and item.generation == generation),
        None,
    )


def _cancellation_precedes_deadline(
    request: Cancellation, deadline: Deadline
) -> bool | None:
    """Return explicit same-instant order; None means causal order is UNKNOWN."""
    if request.microstep_level is not None:
        if request.microstep_level != deadline.microstep:
            return request.microstep_level < deadline.microstep
    if (
        request.phase_order is not None
        and deadline.phase_order is not None
        and request.phase_evidence == deadline.phase_evidence
        and request.phase_order != deadline.phase_order
    ):
        return request.phase_order < deadline.phase_order
    return None



def _validate_reference_state(state: object) -> str | None:
    """Validate all structural fields before participant or temporal evaluation."""
    if not isinstance(state, ReferenceState):
        return "reference state has an invalid type"
    if not isinstance(state.profile, str) or not state.profile.strip():
        return "reference profile must be non-empty"
    if not isinstance(state.domains, tuple) or not isinstance(state.expected_participants, frozenset):
        return "participant containers have invalid types"
    if any(not isinstance(item, ClockDomain) for item in state.domains):
        return "domain collection contains an invalid object"
    if any(not isinstance(item, str) or not item.strip()
           for item in state.expected_participants):
        return "expected participant identity is invalid"
    if state.watermark_common is not None and not _exact(state.watermark_common):
        return "watermark is not exact"
    if any(not isinstance(item.domain_id, str) or not item.domain_id.strip()
           for item in state.domains):
        return "participant identity is invalid"
    if len({item.domain_id for item in state.domains}) != len(state.domains):
        return "duplicate temporal participant"
    if {item.domain_id for item in state.domains} - set(state.expected_participants):
        return "unexpected temporal participant"
    for domain in state.domains:
        if not all(_exact(value) for value in (
            domain.local_to_common, domain.common_offset, domain.progress_local
        )):
            return "participant clock values are not exact"
        if domain.local_to_common <= 0:
            return "participant clock conversion is not comparable"
        if not _exact(domain.horizon_local):
            return "participant safe horizon is not exact"
        if domain.horizon_local < domain.progress_local:
            return "participant safe horizon regressed behind progress"
    if not isinstance(state.obligations, tuple) or any(
        not isinstance(item, Obligation) for item in state.obligations
    ):
        return "obligation collection contains an invalid object"
    obligation_ids = []
    for item in state.obligations:
        if (not isinstance(item.obligation_id, str) or not item.obligation_id.strip()
            or not isinstance(item.owner, str) or not item.owner.strip()
            or not _exact(item.at_common)
            or not isinstance(item.resolved, bool)
            or not isinstance(item.commutative, bool)):
            return "obligation descriptor is invalid"
        obligation_ids.append(item.obligation_id)
    if len(obligation_ids) != len(set(obligation_ids)):
        return "duplicate obligation identity"
    if state.domains:
        safe_horizon = min(domain.horizon_common() for domain in state.domains)
        known_frontier = max(
            [domain.progress_common() for domain in state.domains]
            + ([state.watermark_common] if state.watermark_common is not None else [])
        )
        if safe_horizon < known_frontier:
            return "safe horizon is behind the confirmed machine frontier"
    else:
        known_frontier = (
            state.watermark_common
            if state.watermark_common is not None else Fraction(0)
        )
    # Keep deadline admission consistent with advance(): W is a closed cut,
    # and a known participant frontier cannot lie beyond an unresolved obligation.
    if state.watermark_common is not None and any(
        not item.resolved and item.at_common <= state.watermark_common
        for item in state.obligations
    ):
        return "unresolved obligation is at or behind the confirmed watermark"
    if any(
        not item.resolved and item.at_common < known_frontier
        for item in state.obligations
    ):
        return "unresolved obligation lies behind the confirmed machine frontier"
    return None


def _reference_state_error_code(detail: str) -> DeadlineCode:
    """Map validated reference-state failures to the R1 result taxonomy."""
    if "not exact" in detail:
        return DeadlineCode.INVALID_INPUT
    if "clock conversion" in detail:
        return DeadlineCode.INDETERMINATE
    if "safe horizon" in detail or "unresolved obligation" in detail:
        return DeadlineCode.INVARIANT_VIOLATION
    return DeadlineCode.INVALID_INPUT


def _validate_book(book: object) -> str | None:
    """Reject malformed deadline-book containers and entries before dereferencing."""
    if not isinstance(book, DeadlineBook):
        return "deadline book has an invalid type"
    if not isinstance(book.deadlines, tuple) or not isinstance(book.pending_cancellations, tuple):
        return "deadline book collections must be tuples"
    if (book.watermark_common is not None and not _exact(book.watermark_common)):
        return "deadline book watermark is not exact"
    if (not isinstance(book.max_microsteps, int) or isinstance(book.max_microsteps, bool)
        or book.max_microsteps <= 0):
        return "maximum microsteps must be a positive integer"
    keys = []
    for item in book.deadlines:
        if not isinstance(item, Deadline):
            return "deadline collection contains an invalid object"
        if (not isinstance(item.owner, str) or not item.owner.strip()
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.effect_id, str) or not item.effect_id.strip()
            or not isinstance(item.generation, int) or isinstance(item.generation, bool)
            or item.generation < 0 or not _exact(item.at_common)
            or not isinstance(item.commutative, bool)
            or (item.phase_order is not None and (
                not isinstance(item.phase_order, int) or isinstance(item.phase_order, bool)
            ))
            or (item.phase_evidence is not None and (
                not isinstance(item.phase_evidence, str) or not item.phase_evidence.strip()
            ))
            or ((item.phase_order is None) != (item.phase_evidence is None))
            or not isinstance(item.microstep, int) or isinstance(item.microstep, bool)
            or item.microstep < 0 or not isinstance(item.status, DeadlineStatus)):
            return "deadline entry is structurally invalid"
        if item.microstep > book.max_microsteps:
            return "deadline microstep exceeds maximum book limit"
        keys.append((item.identity, item.generation))
    if len(keys) != len(set(keys)):
        return "deadline identity/generation pair is duplicated"
    latest_generation: dict[str, int] = {}
    for item in book.deadlines:
        latest_generation[item.identity] = max(
            latest_generation.get(item.identity, item.generation),
            item.generation,
        )
    active_states = {
        DeadlineStatus.SCHEDULED,
        DeadlineStatus.CANCEL_PENDING,
        DeadlineStatus.CLAIMED,
        DeadlineStatus.DISPATCHED,
    }
    if any(
        item.generation < latest_generation[item.identity]
        and item.status in active_states
        for item in book.deadlines
    ):
        return "older deadline generation remains active after a newer generation"
    pending_keys = []
    for item in book.pending_cancellations:
        if not isinstance(item, Cancellation):
            return "pending cancellation collection contains an invalid object"
        if (not isinstance(item.owner, str) or not item.owner.strip()
            or not isinstance(item.identity, str) or not item.identity.strip()
            or not isinstance(item.generation, int) or isinstance(item.generation, bool)
            or item.generation < 0 or not _exact(item.at_common)
            or (item.microstep_level is not None and (
                not isinstance(item.microstep_level, int)
                or isinstance(item.microstep_level, bool)
                or item.microstep_level < 0
                or item.microstep_level > book.max_microsteps
            ))
            or (item.phase_order is not None and (
                not isinstance(item.phase_order, int) or isinstance(item.phase_order, bool)
            ))
            or (item.phase_evidence is not None and (
                not isinstance(item.phase_evidence, str) or not item.phase_evidence.strip()
            ))
            or ((item.phase_order is None) != (item.phase_evidence is None))):
            return "pending cancellation entry is structurally invalid"
        key = (item.identity, item.generation)
        if key in pending_keys:
            return "pending cancellation identity/generation is duplicated"
        pending_keys.append(key)
        deadline = next(
            (entry for entry in book.deadlines
             if (entry.identity, entry.generation) == key),
            None,
        )
        if deadline is None:
            return "pending cancellation has no matching deadline generation"
        if deadline.owner != item.owner:
            return "pending cancellation owner does not match deadline owner"
        if deadline.status is not DeadlineStatus.CANCEL_PENDING:
            return "pending cancellation does not match a cancel-pending deadline"
    for item in book.deadlines:
        if (item.status is DeadlineStatus.CANCEL_PENDING
            and (item.identity, item.generation) not in pending_keys):
            return "cancel-pending deadline has no matching cancellation request"
    return None


def _validate_book_state(
    book: object,
) -> tuple[DeadlineCode, str] | None:
    """Validate book structure and the temporal order of pending cancellations."""
    structural_error = _validate_book(book)
    if structural_error is not None:
        return DeadlineCode.INVALID_INPUT, structural_error
    assert isinstance(book, DeadlineBook)
    if book.watermark_common is not None and any(
        item.status is DeadlineStatus.SCHEDULED
        and item.at_common <= book.watermark_common
        for item in book.deadlines
    ):
        return (
            DeadlineCode.INVALID_INPUT,
            "scheduled deadline is at or behind the already closed watermark",
        )
    claimed_states = (
        DeadlineStatus.CLAIMED,
        DeadlineStatus.DISPATCHED,
        DeadlineStatus.COMPLETED,
        DeadlineStatus.FAILED,
    )
    if any(
        item.status in claimed_states
        and (
            book.watermark_common is None
            or item.at_common > book.watermark_common
        )
        for item in book.deadlines
    ):
        return (
            DeadlineCode.INVARIANT_VIOLATION,
            "claimed or processed deadline is beyond the closed watermark",
        )
    for request in book.pending_cancellations:
        deadline = _find(book, request.identity, request.generation)
        assert deadline is not None  # guaranteed by _validate_book()
        if request.at_common < deadline.at_common:
            if (
                book.watermark_common is not None
                and book.watermark_common >= deadline.at_common
            ):
                return (
                    DeadlineCode.INVARIANT_VIOLATION,
                    "pending C<D cancellation survived closure of W through D",
                )
            continue
        if request.at_common == deadline.at_common:
            if (
                book.watermark_common is not None
                and book.watermark_common >= deadline.at_common
            ):
                return (
                    DeadlineCode.LATE_INPUT,
                    "pending cancellation at D cannot suppress after W closed D",
                )
            precedes = _cancellation_precedes_deadline(request, deadline)
            if precedes is True:
                continue
            if precedes is False:
                return (
                    DeadlineCode.LATE_INPUT,
                    "pending cancellation microstep/phase follows the deadline claim",
                )
            return (
                DeadlineCode.INDETERMINATE,
                "pending cancellation at D has no explicit causal-order relation",
            )
        return (
            DeadlineCode.LATE_INPUT,
            "pending cancellation after D cannot suppress the deadline",
        )
    return None


def publish_deadline(
    book: DeadlineBook,
    state: ReferenceState,
    *,
    owner: str,
    identity: str,
    generation: int,
    at_common: Fraction,
    effect_id: str,
    microstep: bool = False,
    commutative: bool = False,
    phase_order: int | None = None,
    phase_evidence: str | None = None,
    microstep_level: int | None = None,
) -> DeadlineEvaluation:
    """Admit a deadline after structural, participant, and temporal validation."""
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(effect_id, str) or not effect_id.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
        or not _exact(at_common)
        or not isinstance(microstep, bool)
        or not isinstance(commutative, bool)
        or (phase_order is not None and (
            not isinstance(phase_order, int) or isinstance(phase_order, bool)
        ))
        or (phase_evidence is not None and (
            not isinstance(phase_evidence, str) or not phase_evidence.strip()
        ))
        or ((phase_order is None) != (phase_evidence is None))
        or (microstep_level is not None and (
            not isinstance(microstep_level, int) or isinstance(microstep_level, bool)
            or microstep_level <= 0
        ))
        or (microstep_level is not None and not microstep)
    ):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "invalid deadline descriptor")
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    state_error = _validate_reference_state(state)
    if state_error is not None:
        return DeadlineEvaluation(
            book, _reference_state_error_code(state_error), state_error
        )
    effective_microstep = (
        microstep_level if microstep_level is not None else (1 if microstep else 0)
    )
    if effective_microstep > book.max_microsteps:
        return DeadlineEvaluation(
            book, DeadlineCode.RESOURCE_EXHAUSTED,
            "deadline microstep exceeds the finite book limit",
        )
    participant_ids = [item.domain_id for item in state.domains]
    expected = set(state.expected_participants)
    domain_ids = set(participant_ids)
    if domain_ids - expected:
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "unexpected temporal participant")
    if expected - domain_ids:
        return DeadlineEvaluation(book, DeadlineCode.BLOCKED,
                                  "participant set incomplete; A(E) unavailable")
    domain = next((item for item in state.domains if item.domain_id == owner), None)
    if domain is None:
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "deadline owner is not a temporal participant")
    if not all(_exact(value) for value in (
        domain.local_to_common, domain.common_offset, domain.progress_local
    )) or domain.local_to_common <= 0:
        return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                  "owner clock conversion is not comparable")
    if (
        (book.watermark_common is not None and not _exact(book.watermark_common))
        or (state.watermark_common is not None and not _exact(state.watermark_common))
    ):
        return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                  "watermark is not exact")
    if state.watermark_common != book.watermark_common:
        return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                  "deadline and reference watermarks disagree")
    progress = domain.progress_common()
    if at_common < progress or (
        book.watermark_common is not None and at_common <= book.watermark_common
    ):
        return DeadlineEvaluation(book, DeadlineCode.LATE_INPUT,
                                  "deadline is behind confirmed progress or closed W")
    if at_common == progress and not microstep:
        return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                  "D=P_d requires an explicit microstep")
    prior = [item for item in book.deadlines if item.identity == identity]
    if prior and generation <= max(item.generation for item in prior):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "generation must increase monotonically")
    if any(item.status in (
        DeadlineStatus.SCHEDULED, DeadlineStatus.CANCEL_PENDING,
        DeadlineStatus.CLAIMED, DeadlineStatus.DISPATCHED,
    ) for item in prior):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "prior generation is not terminal")
    deadline = Deadline(
        owner, identity, generation, Fraction(at_common), effect_id,
        commutative, phase_order, phase_evidence,
        (microstep_level if microstep_level is not None else (1 if microstep else 0)),
    )
    return DeadlineEvaluation(
        replace(book, deadlines=book.deadlines + (deadline,)),
        DeadlineCode.ACCEPTED,
        "deadline admitted",
    )


def _cancellation_admission_frontier(
    book: DeadlineBook, state: ReferenceState, *, owner: str
) -> tuple[Fraction | None, DeadlineCode | None, str]:
    """Derive A(E) from all declared participants, including the direct owner, and W."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return None, book_error[0], book_error[1]
    state_error = _validate_reference_state(state)
    if state_error is not None:
        return None, _reference_state_error_code(state_error), state_error
    if state.watermark_common != book.watermark_common:
        return None, DeadlineCode.INDETERMINATE, "deadline and reference watermarks disagree"

    ids = [item.domain_id for item in state.domains]
    expected = set(state.expected_participants)
    actual = set(ids)
    if actual - expected:
        return None, DeadlineCode.INVALID_INPUT, "unexpected temporal participant"
    if not expected:
        return None, DeadlineCode.BLOCKED, (
            "required participant set is empty; A(E) unavailable"
        )
    if owner not in expected:
        return None, DeadlineCode.BLOCKED, (
            "deadline owner is absent from the declared participant set; A(E) unavailable"
        )
    if expected - actual:
        return None, DeadlineCode.BLOCKED, (
            "participant set incomplete, including a missing deadline owner; A(E) unavailable"
        )

    progresses: list[Fraction] = []
    for domain in state.domains:
        if not all(_exact(value) for value in (
            domain.local_to_common, domain.common_offset, domain.progress_local
        )) or domain.local_to_common <= 0:
            return None, DeadlineCode.INDETERMINATE, (
                f"participant {domain.domain_id} clock conversion is not comparable"
            )
        progresses.append(Fraction(domain.progress_common()))
    if book.watermark_common is not None:
        progresses.append(Fraction(book.watermark_common))
    return max(progresses, default=Fraction(0)), None, "complete admission frontier"


def cancel_deadline(
    book: DeadlineBook,
    state: ReferenceState,
    *,
    owner: str,
    identity: str,
    generation: int,
    at_common: Fraction,
    microstep_level: int | None = None,
    phase_order: int | None = None,
    phase_evidence: str | None = None,
) -> DeadlineEvaluation:
    """Admit a cancellation only after deriving A(E) and any C=D order."""
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or not _exact(at_common)
        or (microstep_level is not None and (
            not isinstance(microstep_level, int) or isinstance(microstep_level, bool)
            or microstep_level < 0
        ))
        or (phase_order is not None and (
            not isinstance(phase_order, int) or isinstance(phase_order, bool)
        ))
        or (phase_evidence is not None and (
            not isinstance(phase_evidence, str) or not phase_evidence.strip()
        ))
        or ((phase_order is None) != (phase_evidence is None))
    ):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "invalid cancellation descriptor")
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    deadline = _find(book, identity, generation)
    if deadline is None or deadline.owner != owner:
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "unknown owner, identity, or generation")
    admission_frontier, frontier_error, frontier_detail = (
        _cancellation_admission_frontier(book, state, owner=owner)
    )
    if frontier_error is not None:
        return DeadlineEvaluation(book, frontier_error, frontier_detail)
    assert admission_frontier is not None
    if microstep_level is not None and microstep_level > book.max_microsteps:
        return DeadlineEvaluation(
            book, DeadlineCode.RESOURCE_EXHAUSTED,
            "cancellation microstep exceeds the finite book limit",
        )
    if deadline.status is not DeadlineStatus.SCHEDULED:
        return DeadlineEvaluation(book, DeadlineCode.LATE_INPUT,
                                  "deadline is no longer cancellable")
    if at_common <= admission_frontier:
        return DeadlineEvaluation(book, DeadlineCode.LATE_INPUT,
                                  "C<=A(E) is retroactive")
    if at_common == deadline.at_common:
        same_instant_request = Cancellation(
            owner, identity, generation, Fraction(at_common),
            microstep_level, phase_order, phase_evidence,
        )
        precedes = _cancellation_precedes_deadline(same_instant_request, deadline)
        if precedes is None:
            return DeadlineEvaluation(
                book, DeadlineCode.INDETERMINATE,
                "C=D requires an explicit profile causal-order or microstep relation",
            )
        if not precedes:
            return DeadlineEvaluation(
                book, DeadlineCode.LATE_INPUT,
                "same-instant cancellation follows the deadline claim",
            )
    if at_common > deadline.at_common:
        return DeadlineEvaluation(book, DeadlineCode.LATE_INPUT,
                                  "C>D cannot suppress the deadline")
    if book.watermark_common is not None and book.watermark_common >= deadline.at_common:
        return DeadlineEvaluation(book, DeadlineCode.LATE_INPUT,
                                  "W already closed the deadline instant")
    pending = Cancellation(
        owner, identity, generation, Fraction(at_common),
        microstep_level, phase_order, phase_evidence,
    )
    updated = replace(deadline, status=DeadlineStatus.CANCEL_PENDING)
    next_book = _replace_deadline(book, updated)
    next_book = replace(
        next_book,
        pending_cancellations=next_book.pending_cancellations + (pending,),
    )
    return DeadlineEvaluation(next_book, DeadlineCode.ACCEPTED,
                              "cancellation admitted; arbitration pending")


def arbitrate_cancellations(book: DeadlineBook) -> DeadlineEvaluation:
    """Apply only owner/identity/generation-consistent admitted cancellations."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    next_book = book
    for request in book.pending_cancellations:
        deadline = _find(next_book, request.identity, request.generation)
        if deadline is None or deadline.status is not DeadlineStatus.CANCEL_PENDING:
            return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                      "pending cancellation lost its active generation")
        next_book = _replace_deadline(
            next_book, replace(deadline, status=DeadlineStatus.CANCELLED)
        )
    next_book = replace(next_book, pending_cancellations=())
    return DeadlineEvaluation(next_book, DeadlineCode.ACCEPTED,
                              "all admitted cancellations arbitrated")


def _causal_order_is_known(group: list[Deadline]) -> bool:
    """Check each logical microstep independently; no insertion-order physics."""
    levels = sorted({item.microstep for item in group})
    for level in levels:
        same_level = [item for item in group if item.microstep == level]
        if len(same_level) < 2 or all(item.commutative for item in same_level):
            continue
        if any(
            item.phase_order is None or item.phase_evidence is None
            for item in same_level
        ):
            return False
        if len({item.phase_order for item in same_level}) != len(same_level):
            return False
    return True


def close_watermark(book: DeadlineBook, target_common: Fraction) -> DeadlineEvaluation:
    """Close W only after cancellation arbitration and causal tie validation."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    if not _exact(target_common):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "watermark target must be exact")
    if book.watermark_common is not None and target_common < book.watermark_common:
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "watermark cannot regress")
    for request in book.pending_cancellations:
        deadline = _find(book, request.identity, request.generation)
        if deadline is not None and deadline.at_common <= target_common:
            return DeadlineEvaluation(book, DeadlineCode.BLOCKED,
                                      "admitted C<D cancellation must be arbitrated before W closes D")
    due = [
        item for item in book.deadlines
        if item.status is DeadlineStatus.SCHEDULED and item.at_common <= target_common
    ]
    instants = sorted({item.at_common for item in due})
    for instant in instants:
        group = [item for item in due if item.at_common == instant]
        if not _causal_order_is_known(group):
            return DeadlineEvaluation(book, DeadlineCode.INDETERMINATE,
                                      "simultaneous non-commutative events have no proven causal order")
    ordered: list[Deadline] = []
    for instant in instants:
        group = [item for item in due if item.at_common == instant]
        for level in sorted({item.microstep for item in group}):
            same_level = [item for item in group if item.microstep == level]
            if len(same_level) > 1 and not all(item.commutative for item in same_level):
                same_level.sort(key=lambda item: item.phase_order)
            else:
                same_level.sort(key=lambda item: item.identity)
            ordered.extend(same_level)
    next_book = book
    for item in ordered:
        next_book = _replace_deadline(
            next_book, replace(item, status=DeadlineStatus.CLAIMED)
        )
    next_book = replace(next_book, watermark_common=Fraction(target_common))
    return DeadlineEvaluation(
        next_book, DeadlineCode.ACCEPTED, "watermark closed after causal claims",
        tuple((item.identity, item.generation) for item in ordered),
    )


def replace_deadline(
    book: DeadlineBook,
    state: ReferenceState,
    *,
    owner: str,
    identity: str,
    generation: int,
    cancel_at: Fraction,
    new_generation: int,
    new_at_common: Fraction,
    new_effect_id: str,
    microstep: bool = False,
    commutative: bool = False,
) -> DeadlineEvaluation:
    """Atomically replace a generation without backdating the new deadline.

    The replacement is one causal operation at cancel_at. A new deadline
    strictly before that instant is retroactive (LateInput); equality has no
    profile causal/microstep relation in this API (Indeterminate).
    """
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
        or not isinstance(new_generation, int) or isinstance(new_generation, bool)
        or new_generation < 0 or new_generation <= generation
        or not _exact(cancel_at) or not _exact(new_at_common)
        or not isinstance(new_effect_id, str) or not new_effect_id.strip()
        or not isinstance(microstep, bool)
        or not isinstance(commutative, bool)
    ):
        return DeadlineEvaluation(
            book, DeadlineCode.INVALID_INPUT, "invalid replacement descriptor"
        )
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    prior = [item for item in book.deadlines if item.identity == identity]
    if prior and new_generation <= max(item.generation for item in prior):
        return DeadlineEvaluation(
            book, DeadlineCode.INVALID_INPUT,
            "replacement generation must increase monotonically",
        )
    cancelled = cancel_deadline(
        book, state, owner=owner, identity=identity, generation=generation,
        at_common=cancel_at,
    )
    if cancelled.code is not DeadlineCode.ACCEPTED:
        return cancelled
    arbitrated = arbitrate_cancellations(cancelled.book)
    if arbitrated.code is not DeadlineCode.ACCEPTED:
        return DeadlineEvaluation(book, arbitrated.code, arbitrated.detail)
    published = publish_deadline(
        arbitrated.book, state, owner=owner, identity=identity,
        generation=new_generation, at_common=new_at_common,
        effect_id=new_effect_id, microstep=microstep,
        commutative=commutative,
    )
    if published.code is not DeadlineCode.ACCEPTED:
        # The replacement is one logical operation: no half-cancellation.
        return DeadlineEvaluation(
            book, published.code,
            f"replacement rejected atomically: {published.detail}",
        )
    # Validate the new publication descriptor and its owner/W admission first.
    # Only then classify its ordering relative to the replacement's C instant.
    if new_at_common < cancel_at:
        return DeadlineEvaluation(
            book, DeadlineCode.LATE_INPUT,
            "replacement deadline precedes the replacement operation",
        )
    if new_at_common == cancel_at:
        return DeadlineEvaluation(
            book, DeadlineCode.INDETERMINATE,
            "replacement deadline coincides with cancellation without an explicit causal relation",
        )
    return DeadlineEvaluation(
        published.book, DeadlineCode.ACCEPTED,
        "old generation cancelled and replacement published atomically",
    )


def dispatch_deadline(
    book: DeadlineBook, *, identity: str, generation: int
) -> DeadlineEvaluation:
    """Dispatch only a claimed current generation; repeated dispatch is idempotent."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    if (
        not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
    ):
        return DeadlineEvaluation(
            book, DeadlineCode.INVALID_INPUT, "invalid dispatch identity or generation"
        )
    deadline = _find(book, identity, generation)
    if deadline is None:
        return DeadlineEvaluation(book, DeadlineCode.STALE_IGNORED,
                                  "stale identity or generation ignored")
    if deadline.status in (DeadlineStatus.DISPATCHED, DeadlineStatus.COMPLETED):
        return DeadlineEvaluation(book, DeadlineCode.ACCEPTED,
                                  "dispatch already recorded; no repeated effect")
    if deadline.status is not DeadlineStatus.CLAIMED:
        return DeadlineEvaluation(book, DeadlineCode.STALE_IGNORED,
                                  "deadline is not claimed; no effect dispatched")
    next_book = _replace_deadline(
        book, replace(deadline, status=DeadlineStatus.DISPATCHED)
    )
    return DeadlineEvaluation(next_book, DeadlineCode.ACCEPTED,
                              "claimed deadline dispatched exactly once")


def complete_deadline(
    book: DeadlineBook, *, identity: str, generation: int
) -> DeadlineEvaluation:
    """Record completion without allowing a second dispatch."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    if (
        not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
    ):
        return DeadlineEvaluation(
            book, DeadlineCode.INVALID_INPUT, "invalid completion identity or generation"
        )
    deadline = _find(book, identity, generation)
    if deadline is None or deadline.status is not DeadlineStatus.DISPATCHED:
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "only a dispatched generation can complete")
    next_book = _replace_deadline(
        book, replace(deadline, status=DeadlineStatus.COMPLETED)
    )
    return DeadlineEvaluation(next_book, DeadlineCode.ACCEPTED,
                              "deadline completed")


def schedule_microstep(
    book: DeadlineBook,
    *,
    owner: str,
    identity: str,
    generation: int,
    at_common: Fraction,
    effect_id: str,
    current_microstep: int,
    commutative: bool = False,
) -> DeadlineEvaluation:
    """Admit a same-instant child only with finite, strictly increasing microstep."""
    book_error = _validate_book_state(book)
    if book_error is not None:
        return DeadlineEvaluation(book, book_error[0], book_error[1])
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
        or not isinstance(effect_id, str) or not effect_id.strip()
        or not isinstance(commutative, bool)
        or not _exact(at_common)
        or not isinstance(current_microstep, int)
        or isinstance(current_microstep, bool)
        or current_microstep < 0
    ):
        return DeadlineEvaluation(
            book, DeadlineCode.INVALID_INPUT, "invalid microstep descriptor"
        )
    if current_microstep >= book.max_microsteps:
        return DeadlineEvaluation(
            book, DeadlineCode.RESOURCE_EXHAUSTED, "microstep limit exhausted"
        )
    if any(item.at_common == at_common and item.microstep > current_microstep
           for item in book.deadlines):
        return DeadlineEvaluation(book, DeadlineCode.INVALID_INPUT,
                                  "microstep progression must be serialized")
    # The synthetic profile state has no authority to assert a physical phase.
    # The explicit microstep is a logical delta-cycle only.
    from chronos_reference import ClockDomain, make_state
    state = make_state(
        profile="ABSTRACT-R2-MICROSTEP",
        domains=[ClockDomain(owner, Fraction(1), progress_local=Fraction(at_common),
                             horizon_local=Fraction(at_common))],
        expected_participants=[owner],
        watermark_common=book.watermark_common,
    )
    return publish_deadline(
        book, state, owner=owner, identity=identity, generation=generation,
        at_common=at_common, effect_id=effect_id, microstep=True,
        microstep_level=current_microstep + 1, commutative=commutative,
    )


def active_deadlines(book: DeadlineBook) -> tuple[Deadline, ...]:
    """Return a stable identity-sorted view of nonterminal deadline generations."""
    terminal = {DeadlineStatus.CANCELLED, DeadlineStatus.COMPLETED, DeadlineStatus.FAILED}
    return tuple(sorted(
        (item for item in book.deadlines if item.status not in terminal),
        key=lambda item: (item.at_common, item.identity, item.generation),
    ))
