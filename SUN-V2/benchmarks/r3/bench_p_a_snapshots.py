"""P-A W09 canonical snapshot/restore/replay synthetic benchmark matrix."""
from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import sys
import time
import tracemalloc
from fractions import Fraction
from pathlib import Path

from benchmark_statistics import (
    MAX_CPU_CLOCK_RESOLUTION_NS,
    QUANTILE_ESTIMATOR,
    TIME_UNIT,
    cpu_clock_is_usable,
    percentile_type7,
)

from p_a_snapshots import (
    PADeadlineRecord, PADeadlineState, PADomainState, PAReplayEvent,
    PAInterruptRecord, PAMachineState, PASnapshotCode,
    PATransactionRecord, PATransactionState,
    capture, replay_from_snapshot, restore,
)


def cpu_clock_resolution_ns() -> int | None:
    """Measure empirical thread CPU clock resolution over a bounded 100 ms window."""
    clock = getattr(time, "thread_time_ns", None)
    if clock is None:
        return None
    start_wall = time.perf_counter_ns()
    start_cpu = clock()
    while time.perf_counter_ns() - start_wall < 100_000_000:
        current = clock()
        if current > start_cpu:
            return current - start_cpu
    return None


def git_head() -> str:
    """Return the checked-out source revision, or UNKNOWN outside a Git worktree."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"], check=False,
            capture_output=True, text=True,
        )
    except OSError:
        return "UNKNOWN"
    return result.stdout.strip() if result.returncode == 0 else "UNKNOWN"


def percentile(values: list[int], fraction: float) -> float:
    """Use the shared Hyndman-Fan Type 7 estimator."""
    return percentile_type7(values, fraction)


def build_state(size: int) -> PAMachineState:
    """Create a deterministic machine cut with domains, deadlines and transactions."""
    domains = tuple(
        PADomainState(
            f"domain-{index:02d}", Fraction(10),
            Fraction(index + 1, 17),
        )
        for index in range(size)
    )
    deadlines = tuple(
        PADeadlineRecord(
            f"deadline-{index:02d}", index, Fraction(10 + index),
            PADeadlineState.SCHEDULED,
        )
        for index in range(size)
    )
    transactions = tuple(
        PATransactionRecord(
            f"transaction-{index:02d}", index, PATransactionState.IN_PROGRESS,
            Fraction(10), next_marker=f"commit-{index:02d}",
            sampled=((f"sample-{index:02d}", f"0x{index:02x}"),),
            committed_effects=(f"effect-done-{index:02d}",),
            pending_effects=(f"effect-pending-{index:02d}",),
        )
        for index in range(size)
    )
    return PAMachineState(
        "SMS-NTSC", "R1-0.3.18", domains, Fraction(8),
        deadlines, transactions, PAInterruptRecord(), 100,
    )


def build_events(state: PAMachineState, count: int) -> tuple[PAReplayEvent, ...]:
    """Create contiguous input records with monotonic time/progress per domain."""
    progress = {domain.name: domain.progress_common for domain in state.domains}
    events = []
    for index in range(count):
        domain = state.domains[index % len(state.domains)].name
        progress[domain] += 1
        events.append(PAReplayEvent(
            state.input_cursor + index + 1,
            Fraction(11 + index),
            domain,
            progress[domain],
            Fraction((index % 13) + 1, 19),
        ))
    return tuple(events)


def execute_roundtrip(size: int, event_count: int):
    """Capture, restore and replay a deterministic input suffix."""
    state = build_state(size)
    captured = capture(state)
    if captured.code is not PASnapshotCode.OK:
        raise AssertionError(f"snapshot capture failed: {captured.code.value}")
    active = PAMachineState(
        state.profile, state.model_version, state.domains,
        Fraction(9), state.deadlines, state.transactions,
        state.interrupts, state.input_cursor,
    )
    restored = restore(
        active, captured.artifact,
        expected_profile=state.profile, expected_version=state.model_version,
    )
    if restored.code is not PASnapshotCode.OK or restored.restored_state != state:
        raise AssertionError("snapshot restore did not reproduce the source state")
    replayed = replay_from_snapshot(
        captured.artifact, build_events(state, event_count)
    )
    if replayed.code is not PASnapshotCode.OK or replayed.state is None:
        raise AssertionError(f"replay failed: {replayed.code.value}")
    return replayed


def run_point(size: int, event_count: int, warmup: int, repeats: int, cpu_enabled: bool) -> dict:
    """Measure snapshot round-trip and a deterministic replay suffix."""
    for _ in range(warmup):
        execute_roundtrip(size, event_count)

    wall_samples: list[int] = []
    cpu_samples: list[int] = []
    clock = getattr(time, "thread_time_ns", None)
    for _ in range(repeats):
        cpu_start = clock() if cpu_enabled and clock is not None else None
        wall_start = time.perf_counter_ns()
        result = execute_roundtrip(size, event_count)
        wall_samples.append(time.perf_counter_ns() - wall_start)
        if cpu_start is not None and clock is not None:
            cpu_samples.append(clock() - cpu_start)

    tracemalloc.start()
    execute_roundtrip(size, event_count)
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    output = {
        "sample_count": len(wall_samples),
        "parameters": {
            "domains": size,
            "deadlines": size,
            "transactions": size,
            "replay_events": event_count,
        },
        "result": result.code.value,
        "input_cursor": result.state.input_cursor,
        "domain_count": len(result.state.domains),
        "deadline_count": len(result.state.deadlines),
        "transaction_count": len(result.state.transactions),
        "snapshot_replay_digest": result.digest,
        "samples_wall_ns": wall_samples,
        "samples_cpu_ns": cpu_samples if cpu_samples else None,
        "wall_ns": {
            "p50": percentile(wall_samples, 0.50),
            "p95": percentile(wall_samples, 0.95),
            "p99": percentile(wall_samples, 0.99),
            "min": min(wall_samples),
            "max": max(wall_samples),
            "mean": statistics.fmean(wall_samples),
        },
        "peak_tracemalloc_bytes_separate_probe": peak_bytes,
    }
    if cpu_samples:
        output["cpu_ns"] = {
            "p50": percentile(cpu_samples, 0.50),
            "p95": percentile(cpu_samples, 0.95),
            "p99": percentile(cpu_samples, 0.99),
            "min": min(cpu_samples),
            "max": max(cpu_samples),
            "mean": statistics.fmean(cpu_samples),
        }
    else:
        output["cpu_ns"] = None
    return output


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", default="1,2,4,8,16")
    parser.add_argument("--replay-events", default="0,1,4,16")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_w09_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")
    sizes = tuple(int(item) for item in args.sizes.split(","))
    event_counts = tuple(int(item) for item in args.replay_events.split(","))
    if any(size not in {1, 2, 4, 8, 16} for size in sizes):
        parser.error("W09 sizes must be selected from 1,2,4,8,16")
    if any(count not in {0, 1, 4, 16} for count in event_counts):
        parser.error("replay event counts must be selected from 0,1,4,16")

    resolution = cpu_clock_resolution_ns()
    cpu_enabled = cpu_clock_is_usable(resolution)
    points = [
        run_point(size, count, args.warmup, args.repeats, cpu_enabled)
        for size in sizes for count in event_counts
    ]
    document = {
        "schema": "sun.chronos.r3.p-a-w09-snapshot-replay.v1",
        "git_head": git_head(),
        "python_version": sys.version,
        "platform": platform.platform(),
        "cpu_clock_resolution_ns": resolution,
        "cpu_time_status": "AVAILABLE" if cpu_enabled else "SKIPPED_COARSE_RESOLUTION",
        "warmup": args.warmup,
        "repeats": args.repeats,
        "timing_unit": TIME_UNIT,
        "quantile_estimator": QUANTILE_ESTIMATOR,
        "cpu_clock_max_resolution_ns": MAX_CPU_CLOCK_RESOLUTION_NS,
        "points": points,
        "limitations": [
            "synthetic Python JSON snapshot candidate, not native Rust serialization",
            "replay events are accepted input records, not hardware event simulation",
            "tracemalloc is a separate probe outside timed samples",
        ],
    }
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps({
        "status": "PASS", "suite": document["schema"],
        "points": len(points), "output": str(args.output),
        "git_head": document["git_head"],
        "cpu_time_status": document["cpu_time_status"],
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
