"""P-A W07 finite microstep-chain synthetic benchmark matrix."""
from __future__ import annotations

import argparse
import hashlib
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

from p_a_microsteps import AStepCode, AStepCycle, close_cycle, make_cycle, process_obligation


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
            ["git", "rev-parse", "HEAD"],
            check=False, capture_output=True, text=True,
        )
    except OSError:
        return "UNKNOWN"
    return result.stdout.strip() if result.returncode == 0 else "UNKNOWN"


def percentile(values: list[int], fraction: float) -> float:
    """Use the shared Hyndman-Fan Type 7 estimator."""
    return percentile_type7(values, fraction)


def digest(result) -> str:
    """Digest logical cycle state and result, excluding host timing."""
    state = result.state
    payload = (
        result.code.value,
        state.at_common,
        state.max_microsteps,
        state.next_microstep,
        tuple(
            (item.identity, item.owner, item.microstep, item.commutative)
            for item in state.pending
        ),
        state.resolved,
    )
    return hashlib.sha256(repr(payload).encode("utf-8")).hexdigest()


def execute_chain(depth: int, mode: str):
    """Run a child-emission chain to completion or its finite limit."""
    state = make_cycle(
        Fraction(10), (("root", "cpu", False),), max_microsteps=depth
    ).state
    current = "root"
    for index in range(depth):
        child = f"child-{index + 1}"
        result = process_obligation(
            state, current, resolves_obligation=True,
            emitted=((child, "cpu", False),),
        )
        if result.code is not AStepCode.ADVANCED:
            raise AssertionError(f"chain did not advance: {result.code.value}")
        state = result.state
        current = child

    if mode == "complete":
        result = process_obligation(state, current, resolves_obligation=True)
        if result.code is not AStepCode.COMPLETE:
            raise AssertionError(f"chain did not complete: {result.code.value}")
        closed = close_cycle(result.state)
        if closed.code is not AStepCode.COMPLETE:
            raise AssertionError(f"completed chain did not close: {closed.code.value}")
        return closed
    if mode == "exhaust":
        before = state
        result = process_obligation(
            before, current, resolves_obligation=True,
            emitted=((f"overflow-{depth}", "cpu", False),),
        )
        if result.code is not AStepCode.RESOURCE_EXHAUSTED:
            raise AssertionError(f"expected ResourceExhausted, got {result.code.value}")
        if result.state != before:
            raise AssertionError("microstep exhaustion mutated the cycle")
        return result
    raise ValueError(f"unsupported mode: {mode}")


def run_point(depth: int, mode: str, warmup: int, repeats: int, cpu_enabled: bool) -> dict:
    """Measure a complete same-instant chain or its atomic exhaustion boundary."""
    for _ in range(warmup):
        execute_chain(depth, mode)

    wall_samples: list[int] = []
    cpu_samples: list[int] = []
    clock = getattr(time, "thread_time_ns", None)
    for _ in range(repeats):
        cpu_start = clock() if cpu_enabled and clock is not None else None
        wall_start = time.perf_counter_ns()
        result = execute_chain(depth, mode)
        wall_samples.append(time.perf_counter_ns() - wall_start)
        if cpu_start is not None and clock is not None:
            cpu_samples.append(clock() - cpu_start)

    tracemalloc.start()
    execute_chain(depth, mode)
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    output = {
        "sample_count": len(wall_samples),
        "parameters": {"depth": depth, "mode": mode, "max_microsteps": depth},
        "result": result.code.value,
        "resolved_count": len(result.state.resolved),
        "pending_count": len(result.state.pending),
        "output_digest": digest(result),
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
    parser.add_argument("--depths", default="1,2,4,8,16,32")
    parser.add_argument("--modes", default="complete,exhaust")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_w07_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")
    depths = tuple(int(item) for item in args.depths.split(","))
    modes = tuple(args.modes.split(","))
    if any(depth not in {1, 2, 4, 8, 16, 32} for depth in depths):
        parser.error("W07 depths must be selected from 1,2,4,8,16,32")
    if any(mode not in {"complete", "exhaust"} for mode in modes):
        parser.error("modes must be complete,exhaust")

    resolution = cpu_clock_resolution_ns()
    cpu_enabled = cpu_clock_is_usable(resolution)
    points = [
        run_point(depth, mode, args.warmup, args.repeats, cpu_enabled)
        for depth in depths for mode in modes
    ]
    document = {
        "schema": "sun.chronos.r3.p-a-w07-microsteps.v1",
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
            "synthetic Python benchmark, not native Rust runtime",
            "logical same-instant obligations, not physical chip phases",
            "tracemalloc is a separate probe outside timed samples",
        ],
    }
    args.output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": "PASS", "suite": document["schema"],
        "points": len(points), "output": str(args.output),
        "git_head": document["git_head"],
        "cpu_time_status": document["cpu_time_status"],
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
