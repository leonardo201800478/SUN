"""P-A W08 transaction lifecycle synthetic benchmark matrix."""
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

from p_a_transactions import (
    PAMarker, PAMarkerKind, PATransactionBook, PATxCode,
    cancel, commit_effect, complete, consume_base_time, consume_tw,
    fail, reach_next_marker, sample_marker, start, submit,
)
from p_a_transactions import PATransactionSpec


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


def percentile(values: list[int], fraction: float) -> int:
    """Return a nearest-rank percentile from integer nanosecond samples."""
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, round((len(ordered) - 1) * fraction)))
    return ordered[index]


def require_ok(result, expected=PATxCode.OK):
    """Fail the workload immediately if a transaction transition is unexpected."""
    if result.code is not expected:
        raise AssertionError(f"unexpected transaction result: {result.code.value}")
    return result


def execute_transaction(marker_count: int, mode: str):
    """Execute a complete transaction or fail after a measured partial-effect prefix."""
    markers = [PAMarker("sample-status", PAMarkerKind.SAMPLE, Fraction(1))]
    markers.extend(
        PAMarker(f"write-{index:02d}", PAMarkerKind.COMMIT, Fraction(index + 2))
        for index in range(marker_count)
    )
    spec = PATransactionSpec(
        "w08-tx", 0, "cpu", "vdp", "write", "SMS-NTSC", "payload",
        Fraction(0), Fraction(0), Fraction(1), 1, 1, tuple(markers),
    )
    result = require_ok(submit(PATransactionBook(), spec))
    result = require_ok(start(result.book, "w08-tx"))
    result = require_ok(reach_next_marker(result.book, "w08-tx", Fraction(1)))
    result = require_ok(
        sample_marker(result.book, "w08-tx", "sample-status", "0x81")
    )

    commit_target = marker_count
    if mode == "partial-failure":
        commit_target = max(1, marker_count // 2)
    elif mode != "complete":
        raise ValueError(f"unsupported transaction mode: {mode}")

    for index in range(commit_target):
        marker_id = f"write-{index:02d}"
        result = require_ok(
            reach_next_marker(
                result.book, "w08-tx", Fraction(index + 2)
            )
        )
        result = require_ok(commit_effect(result.book, "w08-tx", marker_id))

    if mode == "partial-failure":
        return require_ok(fail(result.book, "w08-tx", "synthetic partial failure"))

    result = require_ok(
        consume_base_time(result.book, "w08-tx", Fraction(1))
    )
    result = require_ok(consume_tw(result.book, "w08-tx", automatic=True))
    result = require_ok(consume_tw(result.book, "w08-tx", automatic=False))
    return require_ok(complete(result.book, "w08-tx"))


def result_digest(result) -> str:
    """Digest the complete logical transaction state, including partial effects."""
    tx = result.transaction
    payload = (
        result.code.value,
        tx.spec,
        tx.state.value,
        tx.progress_common,
        tx.base_consumed,
        tx.automatic_tw_consumed,
        tx.external_tw_consumed,
        tx.ready_marker,
        tx.wait_source,
        tx.sampled,
        tx.committed,
        tx.failure,
        tx.recoveries,
    )
    return hashlib.sha256(repr(payload).encode("utf-8")).hexdigest()


def run_point(size: int, mode: str, warmup: int, repeats: int, cpu_enabled: bool) -> dict:
    """Measure one transaction size and terminal/partial-effect scenario."""
    expected = PATxCode.OK
    for _ in range(warmup):
        result = execute_transaction(size, mode)
        if result.code is not expected:
            raise AssertionError("warm-up transaction failed")

    wall_samples: list[int] = []
    cpu_samples: list[int] = []
    clock = getattr(time, "thread_time_ns", None)
    for _ in range(repeats):
        cpu_start = clock() if cpu_enabled and clock is not None else None
        wall_start = time.perf_counter_ns()
        result = execute_transaction(size, mode)
        wall_samples.append(time.perf_counter_ns() - wall_start)
        if cpu_start is not None and clock is not None:
            cpu_samples.append(clock() - cpu_start)
        if result.code is not expected:
            raise AssertionError("measured transaction failed")

    tracemalloc.start()
    execute_transaction(size, mode)
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    tx = result.transaction
    output = {
        "parameters": {"commit_markers": size, "mode": mode},
        "result": result.code.value,
        "transaction_state": tx.state.value,
        "committed_count": len(tx.committed),
        "sampled_count": len(tx.sampled),
        "pending_marker_count": len(tx.spec.markers) - len(tx.committed) - len(tx.sampled),
        "output_digest": result_digest(result),
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
    parser.add_argument("--modes", default="complete,partial-failure")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_w08_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")
    sizes = tuple(int(item) for item in args.sizes.split(","))
    modes = tuple(args.modes.split(","))
    if any(size not in {1, 2, 4, 8, 16} for size in sizes):
        parser.error("W08 sizes must be selected from 1,2,4,8,16")
    if any(mode not in {"complete", "partial-failure"} for mode in modes):
        parser.error("modes must be complete,partial-failure")

    resolution = cpu_clock_resolution_ns()
    cpu_enabled = resolution is not None and resolution <= 1_000_000
    points = [
        run_point(size, mode, args.warmup, args.repeats, cpu_enabled)
        for size in sizes for mode in modes
    ]
    document = {
        "schema": "sun.chronos.r3.p-a-w08-transactions.v1",
        "git_head": git_head(),
        "python_version": sys.version,
        "platform": platform.platform(),
        "cpu_clock_resolution_ns": resolution,
        "cpu_time_status": "AVAILABLE" if cpu_enabled else "SKIPPED_COARSE_RESOLUTION",
        "warmup": args.warmup,
        "repeats": args.repeats,
        "points": points,
        "limitations": [
            "synthetic Python transaction model, not native Rust bus runtime",
            "logical sample/commit frontiers, not physical device timing",
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
