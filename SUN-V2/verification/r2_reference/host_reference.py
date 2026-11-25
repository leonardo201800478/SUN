"""Separate deterministic core-output and nondeterministic host-pipeline oracle for R2-T13."""
from __future__ import annotations
from dataclasses import dataclass, replace
from enum import Enum


class HostCode(str, Enum):
    OK = "Ok"
    BLOCKED = "Blocked"
    INVALID_INPUT = "InvalidInput"


@dataclass(frozen=True)
class VirtualState:
    profile: str
    virtual_ticks: int = 0
    sample_cursor: int = 0
    phase: int = 0
    digital_samples: tuple[int, ...] = ()


@dataclass(frozen=True)
class CoreOutput:
    state: VirtualState
    samples: tuple[int, ...]


@dataclass(frozen=True)
class HostBlock:
    identity: str
    start_sample: int
    samples: tuple[int, ...]


@dataclass(frozen=True)
class HostPipeline:
    buffer_limit: int
    queue: tuple[HostBlock, ...] = ()
    delivered: tuple[HostBlock, ...] = ()
    dropped: tuple[str, ...] = ()
    seen_ids: tuple[str, ...] = ()
    underruns: int = 0
    overruns: int = 0
    vsync_enabled: bool = False
    device_generation: int = 0


@dataclass(frozen=True)
class HostEvaluation:
    code: HostCode
    host: HostPipeline
    blocks: tuple[HostBlock, ...] = ()
    detail: str = ""


def advance_core(state: VirtualState, sample_count: int) -> CoreOutput:
    """Advance an integer-only digital source; host block segmentation is not core state."""
    if not isinstance(state, VirtualState) or type(sample_count) is not int or sample_count < 0:
        raise ValueError("invalid core advance")
    samples = tuple(
        ((state.phase + index * 13) % 256) - 128
        for index in range(sample_count)
    )
    updated = replace(
        state,
        virtual_ticks=state.virtual_ticks + sample_count,
        sample_cursor=state.sample_cursor + sample_count,
        phase=(state.phase + sample_count * 13) % 256,
        digital_samples=state.digital_samples + samples,
    )
    return CoreOutput(updated, samples)


def make_host(buffer_limit: int) -> HostPipeline:
    """Create a host output state separate from the virtual machine."""
    if type(buffer_limit) is not int or buffer_limit < 1:
        raise ValueError("invalid host buffer limit")
    return HostPipeline(buffer_limit)


def submit_block(host: HostPipeline, block: HostBlock) -> HostEvaluation:
    """Queue a digital block; a full host queue cannot change virtual time."""
    if (
        not isinstance(block, HostBlock)
        or type(block.identity) is not str
        or not block.identity.strip()
        or type(block.start_sample) is not int
        or block.start_sample < 0
        or type(block.samples) is not tuple
        or any(type(value) is not int for value in block.samples)
    ):
        return HostEvaluation(HostCode.INVALID_INPUT, host, detail="invalid host block")
    if block.identity in host.seen_ids:
        return HostEvaluation(HostCode.BLOCKED, host, detail="duplicate host block identity")
    if len(host.queue) >= host.buffer_limit:
        updated = replace(host, overruns=host.overruns + 1)
        return HostEvaluation(HostCode.BLOCKED, updated, detail="host queue capacity exhausted")
    updated = replace(host, queue=host.queue + (block,), seen_ids=host.seen_ids + (block.identity,))
    return HostEvaluation(HostCode.OK, updated, (block,))


def drain_host(host: HostPipeline, count: int) -> HostEvaluation:
    """Deliver queued blocks to the host device; virtual state is not an input."""
    if type(count) is not int or count < 0:
        return HostEvaluation(HostCode.INVALID_INPUT, host, detail="invalid drain count")
    drained = host.queue[:count]
    updated = replace(host, queue=host.queue[len(drained):], delivered=host.delivered + drained)
    return HostEvaluation(HostCode.OK, updated, drained)


def drop_host_block(host: HostPipeline, identity: str) -> HostEvaluation:
    """Drop one queued host block without rewinding the core output ledger."""
    if type(identity) is not str or not identity.strip():
        return HostEvaluation(HostCode.INVALID_INPUT, host, detail="invalid block identity")
    if not any(block.identity == identity for block in host.queue):
        return HostEvaluation(HostCode.BLOCKED, host, detail="block is not queued")
    updated = replace(
        host,
        queue=tuple(block for block in host.queue if block.identity != identity),
        dropped=host.dropped + (identity,),
    )
    return HostEvaluation(HostCode.OK, updated)


def signal_underrun(host: HostPipeline) -> HostEvaluation:
    """Record a host underrun without pausing or advancing the virtual machine."""
    return HostEvaluation(HostCode.OK, replace(host, underruns=host.underruns + 1))


def signal_overrun(host: HostPipeline) -> HostEvaluation:
    """Record a host overrun without changing digital core state."""
    return HostEvaluation(HostCode.OK, replace(host, overruns=host.overruns + 1))


def set_vsync(host: HostPipeline, enabled: bool) -> HostEvaluation:
    """Change host presentation synchronization only."""
    if type(enabled) is not bool:
        return HostEvaluation(HostCode.INVALID_INPUT, host, detail="invalid VSync state")
    return HostEvaluation(HostCode.OK, replace(host, vsync_enabled=enabled))


def resize_host_buffer(host: HostPipeline, buffer_limit: int) -> HostEvaluation:
    """Resize host buffering without advancing or pausing virtual time."""
    if type(buffer_limit) is not int or buffer_limit < 1:
        return HostEvaluation(HostCode.INVALID_INPUT, host, detail="invalid buffer limit")
    return HostEvaluation(HostCode.OK, replace(host, buffer_limit=buffer_limit))


def recreate_host_device(host: HostPipeline) -> HostEvaluation:
    """Recreate the host device generation without resetting the core."""
    return HostEvaluation(HostCode.OK, replace(host, device_generation=host.device_generation + 1))
