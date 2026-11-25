"""Exact, source-preserving WAIT oracle for R2-T09."""
from __future__ import annotations
from dataclasses import dataclass, replace
from enum import Enum


class WaitSource(str, Enum):
    NONE = "NoExternalWait"
    EXTERNAL = "External"
    FIXTURE = "Fixture"
    UNKNOWN = "Unknown"


class WaitState(str, Enum):
    RUNNING = "Running"
    WAITING = "Waiting"
    COMPLETE = "Complete"


class WaitCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    INVALID_INPUT = "InvalidInput"
    INVARIANT_VIOLATION = "InvariantViolation"


@dataclass(frozen=True)
class WaitCycle:
    source: WaitSource
    automatic_tws_required: int
    automatic_tws_consumed: int = 0
    external_tws_consumed: int = 0
    samples: tuple[tuple[str, bool], ...] = ()
    state: WaitState = WaitState.RUNNING
    next_sample_point: str | None = "T2"
    progress_token: int = 0


@dataclass(frozen=True)
class WaitEvaluation:
    code: WaitCode
    cycle: WaitCycle
    detail: str = ""


def start_cycle(source: WaitSource, automatic_tws_required: int) -> WaitEvaluation:
    """Create a cycle with independent automatic-TW and external-WAIT declarations."""
    if not isinstance(source, WaitSource) or type(automatic_tws_required) is not int or automatic_tws_required < 0:
        return WaitEvaluation(WaitCode.INVALID_INPUT, WaitCycle(WaitSource.UNKNOWN, 0), "invalid cycle descriptor")
    point = "T2" if source in (WaitSource.EXTERNAL, WaitSource.FIXTURE, WaitSource.UNKNOWN) else None
    return WaitEvaluation(WaitCode.OK, WaitCycle(source, automatic_tws_required, next_sample_point=point))


def consume_automatic_tw(cycle: WaitCycle) -> WaitEvaluation:
    """Consume one Z80-defined automatic TW without sampling or changing WAIT provenance."""
    if cycle.state is WaitState.COMPLETE:
        return WaitEvaluation(WaitCode.INVARIANT_VIOLATION, cycle, "cycle already complete")
    if cycle.automatic_tws_consumed >= cycle.automatic_tws_required:
        return WaitEvaluation(WaitCode.INVALID_INPUT, cycle, "automatic TW budget exhausted")
    updated = replace(
        cycle,
        automatic_tws_consumed=cycle.automatic_tws_consumed + 1,
        progress_token=cycle.progress_token + 1,
    )
    return WaitEvaluation(WaitCode.OK, updated)


def sample_external_wait(cycle: WaitCycle, active: bool | None) -> WaitEvaluation:
    """Sample external/fixture WAIT at T2 or each following TW; UNKNOWN never means inactive."""
    if cycle.state is WaitState.COMPLETE:
        return WaitEvaluation(WaitCode.INVARIANT_VIOLATION, cycle, "cycle already complete")
    if cycle.source is WaitSource.NONE:
        return WaitEvaluation(WaitCode.INVALID_INPUT, cycle, "profile declares no external WAIT source")
    if cycle.source is WaitSource.UNKNOWN or active is None:
        return WaitEvaluation(WaitCode.BLOCKED, cycle, "WAIT level is UNKNOWN at the required sample point")
    if cycle.source not in (WaitSource.EXTERNAL, WaitSource.FIXTURE) or type(active) is not bool:
        return WaitEvaluation(WaitCode.INVALID_INPUT, cycle, "invalid WAIT sample")
    if cycle.next_sample_point is None:
        return WaitEvaluation(WaitCode.INVARIANT_VIOLATION, cycle, "no WAIT sample point is pending")
    point = cycle.next_sample_point
    samples = cycle.samples + ((point, active),)
    if active:
        updated = replace(
            cycle,
            samples=samples,
            external_tws_consumed=cycle.external_tws_consumed + 1,
            state=WaitState.WAITING,
            next_sample_point="TW",
            progress_token=cycle.progress_token + 1,
        )
    else:
        updated = replace(
            cycle,
            samples=samples,
            state=WaitState.RUNNING,
            next_sample_point=None,
            progress_token=cycle.progress_token + 1,
        )
    return WaitEvaluation(WaitCode.OK, updated)


def complete_cycle(cycle: WaitCycle) -> WaitEvaluation:
    """Complete only after WAIT release and all declared automatic Z80 TWs."""
    if cycle.state is WaitState.COMPLETE:
        return WaitEvaluation(WaitCode.OK, cycle)
    if cycle.state is WaitState.WAITING or cycle.next_sample_point is not None:
        return WaitEvaluation(WaitCode.BLOCKED, cycle, "WAIT sampling/release remains pending")
    if cycle.automatic_tws_consumed != cycle.automatic_tws_required:
        return WaitEvaluation(WaitCode.BLOCKED, cycle, "automatic Z80 TWs remain")
    updated = replace(cycle, state=WaitState.COMPLETE)
    return WaitEvaluation(WaitCode.OK, updated)
