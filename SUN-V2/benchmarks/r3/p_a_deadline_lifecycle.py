"""P-A disposable linear-scan deadline lifecycle prototype.

Owns its data structures and algorithms. It intentionally uses immutable tuples
and full scans, not the R2 oracle or a production event agenda.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum
from fractions import Fraction

from p_a_instruction_catchup import AObligation, AParticipant


class ADeadlineStatus(str, Enum):
    SCHEDULED = "Scheduled"
    CANCEL_PENDING = "CancelPending"
    CANCELLED = "Cancelled"
    CLAIMED = "Claimed"
    DISPATCHED = "Dispatched"
    COMPLETED = "Completed"
    FAILED = "Failed"


class ADeadlineCode(str, Enum):
    ACCEPTED = "Accepted"
    BLOCKED = "Blocked"
    INVALID_INPUT = "InvalidInput"
    LATE_INPUT = "LateInput"
    INDETERMINATE = "IndeterminateCausalOrder"
    RESOURCE_EXHAUSTED = "ResourceExhausted"
    INVARIANT_VIOLATION = "InvariantViolation"
    STALE_IGNORED = "StaleIgnored"


@dataclass(frozen=True)
class APhaseBasis:
    """Structured, auditable comparison basis for profile phase coordinates.

    Prototype metadata only: it does not prove that a real SMS profile has
    supplied or validated the referenced evidence. Real profiles remain
    UNKNOWN until an external validation record is established.
    """

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
class ADeadline:
    owner: str
    identity: str
    generation: int
    at_common: Fraction
    effect_id: str
    commutative: bool = False
    phase_order: int | None = None
    phase_evidence: str | None = None
    microstep: int = 0
    status: ADeadlineStatus = ADeadlineStatus.SCHEDULED
    phase_basis: APhaseBasis | None = None


@dataclass(frozen=True)
class ACancellation:
    owner: str
    identity: str
    generation: int
    at_common: Fraction
    microstep_level: int | None = None
    phase_order: int | None = None
    phase_evidence: str | None = None
    phase_basis: APhaseBasis | None = None


@dataclass(frozen=True)
class ADeadlineContext:
    participants: tuple[AParticipant, ...]
    expected_participants: frozenset[str]
    obligations: tuple[AObligation, ...] = ()
    watermark_common: Fraction | None = None


@dataclass(frozen=True)
class ADeadlineBook:
    deadlines: tuple[ADeadline, ...] = ()
    pending_cancellations: tuple[ACancellation, ...] = ()
    watermark_common: Fraction | None = None
    max_microsteps: int = 32


@dataclass(frozen=True)
class ADeadlineResult:
    book: ADeadlineBook
    code: ADeadlineCode
    detail: str = ""
    claimed: tuple[tuple[str, int], ...] = ()


def _exact(value: object) -> bool:
    return isinstance(value, (int, Fraction)) and not isinstance(value, bool)


def _valid_phase_basis(value: object) -> bool:
    if not isinstance(value, APhaseBasis):
        return False
    text_fields = (
        value.profile_id, value.profile_revision, value.coordinate, value.unit,
        value.basis_id, value.basis_revision, value.validation_record_id,
    )
    if any(not isinstance(item, str) or not item.strip() for item in text_fields):
        return False
    if (
        not isinstance(value.scale_numerator, int)
        or isinstance(value.scale_numerator, bool)
        or value.scale_numerator <= 0
        or not isinstance(value.scale_denominator, int)
        or isinstance(value.scale_denominator, bool)
        or value.scale_denominator <= 0
    ):
        return False
    digest = value.validation_record_sha256
    return (
        isinstance(digest, str) and len(digest) == 64
        and all(char in "0123456789abcdef" for char in digest)
    )


def _find(book: ADeadlineBook, identity: str, generation: int) -> ADeadline | None:
    return next(
        (item for item in book.deadlines
         if item.identity == identity and item.generation == generation),
        None,
    )


def _replace_deadline(book: ADeadlineBook, updated: ADeadline) -> ADeadlineBook:
    return replace(
        book,
        deadlines=tuple(
            updated if (item.identity, item.generation) ==
            (updated.identity, updated.generation) else item
            for item in book.deadlines
        ),
    )


def _validate_context(context: object) -> str | None:
    if not isinstance(context, ADeadlineContext):
        return "deadline context has an invalid type"
    if not isinstance(context.participants, tuple) or any(
        not isinstance(item, AParticipant) for item in context.participants
    ):
        return "participant collection is invalid"
    if not isinstance(context.expected_participants, frozenset) or any(
        not isinstance(item, str) or not item.strip()
        for item in context.expected_participants
    ):
        return "expected participant set is invalid"
    if not isinstance(context.obligations, tuple) or any(
        not isinstance(item, AObligation) for item in context.obligations
    ):
        return "obligation collection is invalid"
    if context.watermark_common is not None and not _exact(context.watermark_common):
        return "watermark is not exact"
    ids = [item.name for item in context.participants]
    if len(ids) != len(set(ids)):
        return "duplicate temporal participant"
    if set(ids) - set(context.expected_participants):
        return "unexpected temporal participant"
    for item in context.participants:
        if not all(_exact(value) for value in (
            item.progress, item.safe_horizon, item.units_per_common, item.common_offset
        )):
            return "participant clock values are not exact"
        if item.units_per_common <= 0:
            return "participant clock conversion is not comparable"
        if item.safe_horizon < item.progress:
            return "participant safe horizon regressed behind progress"
    obligation_ids = [item.identity for item in context.obligations]
    if len(obligation_ids) != len(set(obligation_ids)):
        return "duplicate obligation identity"
    if context.participants:
        frontier = max(
            [item.progress for item in context.participants]
            + ([context.watermark_common] if context.watermark_common is not None else [])
        )
        safe_horizon = min(item.safe_horizon for item in context.participants)
        if safe_horizon < frontier:
            return "safe horizon is behind the confirmed machine frontier"
    else:
        frontier = context.watermark_common or Fraction(0)
    if context.watermark_common is not None and any(
        not item.resolved and item.at_common <= context.watermark_common
        for item in context.obligations
    ):
        return "unresolved obligation is at or behind the confirmed watermark"
    if any(
        not item.resolved and item.at_common < frontier
        for item in context.obligations
    ):
        return "unresolved obligation lies behind the confirmed machine frontier"
    return None


def _validate_book(book: object) -> str | None:
    if not isinstance(book, ADeadlineBook):
        return "deadline book has an invalid type"
    if not isinstance(book.deadlines, tuple) or not isinstance(
        book.pending_cancellations, tuple
    ):
        return "deadline book collections must be tuples"
    if book.watermark_common is not None and not _exact(book.watermark_common):
        return "deadline book watermark is not exact"
    if (
        not isinstance(book.max_microsteps, int)
        or isinstance(book.max_microsteps, bool)
        or book.max_microsteps <= 0
    ):
        return "maximum microsteps must be a positive integer"

    keys: list[tuple[str, int]] = []
    latest: dict[str, int] = {}
    for item in book.deadlines:
        if not isinstance(item, ADeadline):
            return "deadline collection contains an invalid object"
        if (
            not isinstance(item.owner, str) or not item.owner.strip()
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
            or (item.phase_basis is not None and (
                not _valid_phase_basis(item.phase_basis)
                or item.phase_order is None or item.phase_evidence is None
            ))
            or not isinstance(item.microstep, int) or isinstance(item.microstep, bool)
            or item.microstep < 0 or not isinstance(item.status, ADeadlineStatus)
        ):
            return "deadline entry is structurally invalid"
        if item.microstep > book.max_microsteps:
            return "deadline microstep exceeds maximum book limit"
        keys.append((item.identity, item.generation))
        latest[item.identity] = max(latest.get(item.identity, item.generation), item.generation)
    if len(keys) != len(set(keys)):
        return "deadline identity/generation pair is duplicated"

    active = {
        ADeadlineStatus.SCHEDULED, ADeadlineStatus.CANCEL_PENDING,
        ADeadlineStatus.CLAIMED, ADeadlineStatus.DISPATCHED,
    }
    if any(
        item.generation < latest[item.identity] and item.status in active
        for item in book.deadlines
    ):
        return "older deadline generation remains active after a newer generation"

    pending_keys: list[tuple[str, int]] = []
    for item in book.pending_cancellations:
        if not isinstance(item, ACancellation):
            return "pending cancellation collection contains an invalid object"
        if (
            not isinstance(item.owner, str) or not item.owner.strip()
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
            or ((item.phase_order is None) != (item.phase_evidence is None))
            or (item.phase_basis is not None and (
                not _valid_phase_basis(item.phase_basis)
                or item.phase_order is None or item.phase_evidence is None
            ))
        ):
            return "pending cancellation entry is structurally invalid"
        key = (item.identity, item.generation)
        if key in pending_keys:
            return "pending cancellation identity/generation is duplicated"
        pending_keys.append(key)
        deadline = _find(book, item.identity, item.generation)
        if deadline is None:
            return "pending cancellation has no matching deadline generation"
        if deadline.owner != item.owner:
            return "pending cancellation owner does not match deadline owner"
        if deadline.status is not ADeadlineStatus.CANCEL_PENDING:
            return "pending cancellation does not match a cancel-pending deadline"
    for item in book.deadlines:
        if (
            item.status is ADeadlineStatus.CANCEL_PENDING
            and (item.identity, item.generation) not in pending_keys
        ):
            return "cancel-pending deadline has no matching cancellation request"
    return None


def _precedes(request: ACancellation, deadline: ADeadline) -> bool | None:
    if request.microstep_level is not None:
        if request.microstep_level != deadline.microstep:
            return request.microstep_level < deadline.microstep
    if (
        request.phase_order is not None
        and deadline.phase_order is not None
        and request.phase_evidence == deadline.phase_evidence
        and _valid_phase_basis(request.phase_basis)
        and request.phase_basis == deadline.phase_basis
        and request.phase_order != deadline.phase_order
    ):
        return request.phase_order < deadline.phase_order
    return None


def _validate_book_state(book: object) -> tuple[ADeadlineCode, str] | None:
    error = _validate_book(book)
    if error is not None:
        return ADeadlineCode.INVALID_INPUT, error
    assert isinstance(book, ADeadlineBook)
    if book.watermark_common is not None and any(
        item.status is ADeadlineStatus.SCHEDULED
        and item.at_common <= book.watermark_common
        for item in book.deadlines
    ):
        return ADeadlineCode.INVALID_INPUT, "scheduled deadline is at or behind closed watermark"
    claimed = (
        ADeadlineStatus.CLAIMED, ADeadlineStatus.DISPATCHED,
        ADeadlineStatus.COMPLETED, ADeadlineStatus.FAILED,
    )
    if any(
        item.status in claimed and (
            book.watermark_common is None or item.at_common > book.watermark_common
        )
        for item in book.deadlines
    ):
        return ADeadlineCode.INVARIANT_VIOLATION, "claimed or processed deadline is beyond closed watermark"
    for request in book.pending_cancellations:
        deadline = _find(book, request.identity, request.generation)
        assert deadline is not None
        if request.at_common < deadline.at_common:
            if book.watermark_common is not None and book.watermark_common >= deadline.at_common:
                return ADeadlineCode.INVARIANT_VIOLATION, "pending C<D cancellation survived closure of W through D"
            continue
        if request.at_common == deadline.at_common:
            if book.watermark_common is not None and book.watermark_common >= deadline.at_common:
                return ADeadlineCode.LATE_INPUT, "pending cancellation at D cannot suppress after W closed D"
            precedes = _precedes(request, deadline)
            if precedes is True:
                continue
            if precedes is False:
                return ADeadlineCode.LATE_INPUT, "pending cancellation follows deadline claim"
            return ADeadlineCode.INDETERMINATE, "pending cancellation at D has unknown causal order"
        return ADeadlineCode.LATE_INPUT, "pending cancellation after D cannot suppress deadline"
    return None


def _context_error_code(detail: str) -> ADeadlineCode:
    if "not exact" in detail:
        return ADeadlineCode.INVALID_INPUT
    if "clock conversion" in detail:
        return ADeadlineCode.INDETERMINATE
    if "safe horizon" in detail or "unresolved obligation" in detail:
        return ADeadlineCode.INVARIANT_VIOLATION
    return ADeadlineCode.INVALID_INPUT


def _admission_frontier(
    book: ADeadlineBook, context: ADeadlineContext, *, owner: str
) -> tuple[Fraction | None, ADeadlineCode | None, str]:
    book_error = _validate_book_state(book)
    if book_error is not None:
        return None, book_error[0], book_error[1]
    context_error = _validate_context(context)
    if context_error is not None:
        return None, _context_error_code(context_error), context_error
    if context.watermark_common != book.watermark_common:
        return None, ADeadlineCode.INDETERMINATE, "deadline and reference watermarks disagree"

    actual = {item.name for item in context.participants}
    expected = set(context.expected_participants)
    if actual - expected:
        return None, ADeadlineCode.INVALID_INPUT, "unexpected temporal participant"
    if not expected:
        return None, ADeadlineCode.BLOCKED, "required participant set is empty; A(E) unavailable"
    if owner not in expected:
        return None, ADeadlineCode.BLOCKED, "deadline owner is absent from declared participant set"
    if expected - actual:
        return None, ADeadlineCode.BLOCKED, "participant set incomplete; A(E) unavailable"
    if owner not in actual:
        return None, ADeadlineCode.INVALID_INPUT, "deadline owner is not a temporal participant"

    progresses = [item.progress for item in context.participants]
    if book.watermark_common is not None:
        progresses.append(book.watermark_common)
    return max(progresses, default=Fraction(0)), None, "complete admission frontier"


def publish_deadline(
    book: ADeadlineBook, context: ADeadlineContext, *,
    owner: str, identity: str, generation: int, at_common: Fraction,
    effect_id: str, microstep: bool = False, commutative: bool = False,
    phase_order: int | None = None, phase_evidence: str | None = None,
    microstep_level: int | None = None,
    phase_basis: APhaseBasis | None = None,
) -> ADeadlineResult:
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(effect_id, str) or not effect_id.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0 or not _exact(at_common)
        or not isinstance(microstep, bool) or not isinstance(commutative, bool)
        or (phase_order is not None and (
            not isinstance(phase_order, int) or isinstance(phase_order, bool)
        ))
        or (phase_evidence is not None and (
            not isinstance(phase_evidence, str) or not phase_evidence.strip()
        ))
        or ((phase_order is None) != (phase_evidence is None))
        or (phase_basis is not None and (
            not _valid_phase_basis(phase_basis)
            or phase_order is None or phase_evidence is None
        ))
        or (microstep_level is not None and (
            not isinstance(microstep_level, int) or isinstance(microstep_level, bool)
            or microstep_level <= 0
        ))
        or (microstep_level is not None and not microstep)
    ):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "invalid deadline descriptor")
    book_error = _validate_book_state(book)
    if book_error is not None:
        return ADeadlineResult(book, book_error[0], book_error[1])
    context_error = _validate_context(context)
    if context_error is not None:
        return ADeadlineResult(book, _context_error_code(context_error), context_error)
    effective_microstep = microstep_level if microstep_level is not None else (1 if microstep else 0)
    if effective_microstep > book.max_microsteps:
        return ADeadlineResult(book, ADeadlineCode.RESOURCE_EXHAUSTED, "deadline microstep exceeds finite book limit")
    actual = {item.name for item in context.participants}
    expected = set(context.expected_participants)
    if actual - expected:
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "unexpected temporal participant")
    if expected - actual:
        return ADeadlineResult(book, ADeadlineCode.BLOCKED, "participant set incomplete; A(E) unavailable")
    domain = next((item for item in context.participants if item.name == owner), None)
    if domain is None:
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "deadline owner is not a temporal participant")
    if context.watermark_common != book.watermark_common:
        return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "deadline and reference watermarks disagree")
    if at_common < domain.progress or (
        book.watermark_common is not None and at_common <= book.watermark_common
    ):
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "deadline is behind confirmed progress or closed W")
    if at_common == domain.progress and not microstep:
        return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "D=P_d requires an explicit microstep")
    prior = [item for item in book.deadlines if item.identity == identity]
    if prior and generation <= max(item.generation for item in prior):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "generation must increase monotonically")
    if any(item.status in (
        ADeadlineStatus.SCHEDULED, ADeadlineStatus.CANCEL_PENDING,
        ADeadlineStatus.CLAIMED, ADeadlineStatus.DISPATCHED,
    ) for item in prior):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "prior generation is not terminal")
    deadline = ADeadline(
        owner, identity, generation, Fraction(at_common), effect_id,
        commutative, phase_order, phase_evidence,
        microstep_level if microstep_level is not None else (1 if microstep else 0),
        phase_basis=phase_basis,
    )
    return ADeadlineResult(
        replace(book, deadlines=book.deadlines + (deadline,)),
        ADeadlineCode.ACCEPTED, "deadline admitted",
    )


def cancel_deadline(
    book: ADeadlineBook, context: ADeadlineContext, *,
    owner: str, identity: str, generation: int, at_common: Fraction,
    microstep_level: int | None = None, phase_order: int | None = None,
    phase_evidence: str | None = None, phase_basis: APhaseBasis | None = None,
) -> ADeadlineResult:
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
        or (phase_basis is not None and (
            not _valid_phase_basis(phase_basis)
            or phase_order is None or phase_evidence is None
        ))
    ):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "invalid cancellation descriptor")
    book_error = _validate_book_state(book)
    if book_error is not None:
        return ADeadlineResult(book, book_error[0], book_error[1])
    deadline = _find(book, identity, generation)
    if deadline is None or deadline.owner != owner:
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "unknown owner, identity, or generation")
    frontier, frontier_error, detail = _admission_frontier(book, context, owner=owner)
    if frontier_error is not None:
        return ADeadlineResult(book, frontier_error, detail)
    assert frontier is not None
    if microstep_level is not None and microstep_level > book.max_microsteps:
        return ADeadlineResult(book, ADeadlineCode.RESOURCE_EXHAUSTED, "cancellation microstep exceeds finite book limit")
    if deadline.status is not ADeadlineStatus.SCHEDULED:
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "deadline is no longer cancellable")
    if at_common <= frontier:
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "C<=A(E) is retroactive")
    if at_common == deadline.at_common:
        request = ACancellation(owner, identity, generation, Fraction(at_common),
                                microstep_level, phase_order, phase_evidence, phase_basis)
        precedes = _precedes(request, deadline)
        if precedes is None:
            return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "C=D requires explicit causal order")
        if not precedes:
            return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "same-instant cancellation follows deadline claim")
    if at_common > deadline.at_common:
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "C>D cannot suppress deadline")
    if book.watermark_common is not None and book.watermark_common >= deadline.at_common:
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "W already closed deadline instant")
    request = ACancellation(owner, identity, generation, Fraction(at_common),
                            microstep_level, phase_order, phase_evidence, phase_basis)
    next_book = _replace_deadline(book, replace(deadline, status=ADeadlineStatus.CANCEL_PENDING))
    next_book = replace(next_book, pending_cancellations=next_book.pending_cancellations + (request,))
    return ADeadlineResult(next_book, ADeadlineCode.ACCEPTED, "cancellation admitted; arbitration pending")


def arbitrate_cancellations(book: ADeadlineBook) -> ADeadlineResult:
    error = _validate_book_state(book)
    if error is not None:
        return ADeadlineResult(book, error[0], error[1])
    next_book = book
    for request in book.pending_cancellations:
        deadline = _find(next_book, request.identity, request.generation)
        if deadline is None or deadline.status is not ADeadlineStatus.CANCEL_PENDING:
            return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "pending cancellation lost active generation")
        next_book = _replace_deadline(next_book, replace(deadline, status=ADeadlineStatus.CANCELLED))
    next_book = replace(next_book, pending_cancellations=())
    return ADeadlineResult(next_book, ADeadlineCode.ACCEPTED, "all admitted cancellations arbitrated")


def _causal_order_known(group: list[ADeadline]) -> bool:
    """Require a relation for the entire simultaneous group, not each level alone.

    Microstep numbers are not evidence of physical causality. A fully commutative
    group may use any deterministic order; otherwise every member must share a
    validated phase basis and a unique phase order across the whole instant.
    """
    if len(group) < 2 or all(item.commutative for item in group):
        return True
    if any(
        item.phase_order is None
        or item.phase_evidence is None
        or not _valid_phase_basis(item.phase_basis)
        for item in group
    ):
        return False
    if len({item.phase_order for item in group}) != len(group):
        return False
    first_basis = group[0].phase_basis
    return all(item.phase_basis == first_basis for item in group[1:])


def close_watermark(book: ADeadlineBook, target_common: Fraction) -> ADeadlineResult:
    error = _validate_book_state(book)
    if error is not None:
        return ADeadlineResult(book, error[0], error[1])
    if not _exact(target_common):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "watermark target must be exact")
    if book.watermark_common is not None and target_common < book.watermark_common:
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "watermark cannot regress")
    for request in book.pending_cancellations:
        deadline = _find(book, request.identity, request.generation)
        if deadline is not None and deadline.at_common <= target_common:
            return ADeadlineResult(book, ADeadlineCode.BLOCKED, "admitted C<D cancellation must be arbitrated before W closes D")
    due_by_instant: dict[Fraction, list[ADeadline]] = {}
    for item in book.deadlines:
        if item.status is ADeadlineStatus.SCHEDULED and item.at_common <= target_common:
            due_by_instant.setdefault(item.at_common, []).append(item)
    instants = sorted(due_by_instant)
    for instant in instants:
        if not _causal_order_known(due_by_instant[instant]):
            return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "simultaneous non-commutative events have no proven causal order")
    ordered: list[ADeadline] = []
    for instant in instants:
        group = due_by_instant[instant]
        if all(item.commutative for item in group):
            ordered.extend(sorted(group, key=lambda item: item.identity))
        else:
            # The validated phase basis/order spans every microstep in this
            # simultaneous group; numeric microstep cannot add precedence.
            ordered.extend(sorted(group, key=lambda item: item.phase_order))
    next_book = book
    for item in ordered:
        next_book = _replace_deadline(next_book, replace(item, status=ADeadlineStatus.CLAIMED))
    next_book = replace(next_book, watermark_common=Fraction(target_common))
    return ADeadlineResult(
        next_book, ADeadlineCode.ACCEPTED, "watermark closed after causal claims",
        tuple((item.identity, item.generation) for item in ordered),
    )


def dispatch_deadline(
    book: ADeadlineBook, *, identity: str, generation: int
) -> ADeadlineResult:
    error = _validate_book_state(book)
    if error is not None:
        return ADeadlineResult(book, error[0], error[1])
    if (
        not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
    ):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "invalid dispatch identity or generation")
    deadline = _find(book, identity, generation)
    if deadline is None:
        return ADeadlineResult(book, ADeadlineCode.STALE_IGNORED, "stale identity or generation ignored")
    if deadline.status in (ADeadlineStatus.DISPATCHED, ADeadlineStatus.COMPLETED):
        return ADeadlineResult(book, ADeadlineCode.ACCEPTED, "dispatch already recorded; no repeated effect")
    if deadline.status is not ADeadlineStatus.CLAIMED:
        return ADeadlineResult(book, ADeadlineCode.STALE_IGNORED, "deadline is not claimed; no effect dispatched")
    return ADeadlineResult(
        _replace_deadline(book, replace(deadline, status=ADeadlineStatus.DISPATCHED)),
        ADeadlineCode.ACCEPTED, "claimed deadline dispatched exactly once",
    )


def complete_deadline(
    book: ADeadlineBook, *, identity: str, generation: int
) -> ADeadlineResult:
    error = _validate_book_state(book)
    if error is not None:
        return ADeadlineResult(book, error[0], error[1])
    if (
        not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0
    ):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "invalid completion identity or generation")
    deadline = _find(book, identity, generation)
    if deadline is None or deadline.status is not ADeadlineStatus.DISPATCHED:
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "only a dispatched generation can complete")
    return ADeadlineResult(
        _replace_deadline(book, replace(deadline, status=ADeadlineStatus.COMPLETED)),
        ADeadlineCode.ACCEPTED, "deadline completed",
    )


def replace_deadline(
    book: ADeadlineBook, context: ADeadlineContext, *,
    owner: str, identity: str, generation: int, cancel_at: Fraction,
    new_generation: int, new_at_common: Fraction, new_effect_id: str,
    microstep: bool = False, commutative: bool = False,
) -> ADeadlineResult:
    if (
        not isinstance(owner, str) or not owner.strip()
        or not isinstance(identity, str) or not identity.strip()
        or not isinstance(generation, int) or isinstance(generation, bool)
        or generation < 0 or not isinstance(new_generation, int)
        or isinstance(new_generation, bool) or new_generation <= generation
        or not _exact(cancel_at) or not _exact(new_at_common)
        or not isinstance(new_effect_id, str) or not new_effect_id.strip()
        or not isinstance(microstep, bool) or not isinstance(commutative, bool)
    ):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "invalid replacement descriptor")
    book_error = _validate_book_state(book)
    if book_error is not None:
        return ADeadlineResult(book, book_error[0], book_error[1])
    prior = [item for item in book.deadlines if item.identity == identity]
    if prior and new_generation <= max(item.generation for item in prior):
        return ADeadlineResult(book, ADeadlineCode.INVALID_INPUT, "replacement generation must increase monotonically")
    cancelled = cancel_deadline(
        book, context, owner=owner, identity=identity, generation=generation,
        at_common=cancel_at,
    )
    if cancelled.code is not ADeadlineCode.ACCEPTED:
        return cancelled
    arbitrated = arbitrate_cancellations(cancelled.book)
    if arbitrated.code is not ADeadlineCode.ACCEPTED:
        return ADeadlineResult(book, arbitrated.code, arbitrated.detail)
    published = publish_deadline(
        arbitrated.book, context, owner=owner, identity=identity,
        generation=new_generation, at_common=new_at_common,
        effect_id=new_effect_id, microstep=microstep, commutative=commutative,
    )
    if published.code is not ADeadlineCode.ACCEPTED:
        return ADeadlineResult(book, published.code, f"replacement rejected atomically: {published.detail}")
    if new_at_common < cancel_at:
        return ADeadlineResult(book, ADeadlineCode.LATE_INPUT, "replacement deadline precedes replacement operation")
    if new_at_common == cancel_at:
        return ADeadlineResult(book, ADeadlineCode.INDETERMINATE, "replacement deadline coincides with cancellation without explicit causal relation")
    return ADeadlineResult(published.book, ADeadlineCode.ACCEPTED, "old generation cancelled and replacement published atomically")


def active_deadlines(book: ADeadlineBook) -> tuple[ADeadline, ...]:
    terminal = {ADeadlineStatus.CANCELLED, ADeadlineStatus.COMPLETED, ADeadlineStatus.FAILED}
    return tuple(sorted(
        (item for item in book.deadlines if item.status not in terminal),
        key=lambda item: (item.at_common, item.identity, item.generation),
    ))
