"""P-A W06 simultaneous-order synthetic benchmark matrix."""
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

from p_a_deadline_lifecycle import (
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    AParticipant,
    close_watermark,
    publish_deadline,
)


def cpu_clock_resolution_ns() -> int | None:
    """Measure empirical thread CPU clock resolution without assuming it."""
    clock = getattr(time, "thread_time_ns", None)
    if clock is None:
        return None
    samples = []
    previous = clock()
    for _ in range(2000):
        current = clock()
        if current > previous:
            samples.append(current - previous)
            previous = current
    return min(samples) if samples else None


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


def percentile(values: list[int], fraction: float) -> int:
    """Return a nearest-rank percentile from integer nanosecond samples."""
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, round((len(ordered) - 1) * fraction)))
    return ordered[index]


def build_book(count: int, relation: str, insertion: str):
    """Build one immutable simultaneous deadline group outside timed samples."""
    names = tuple(f"device-{index:02d}" for index in range(count))
    context = ADeadlineContext(
        participants=tuple(
            AParticipant(name, Fraction(0), Fraction(100))
            for name in names
        ),
        expected_participants=frozenset(names),
    )
    if insertion == "forward":
        order = tuple(range(count))
    elif insertion == "reverse":
        order = tuple(reversed(range(count)))
    else:
        raise ValueError(f"unsupported insertion order: {insertion}")

    book = ADeadlineBook()
    for index in order:
        commutative = relation == "commutative"
        phase_order = index + 1 if relation == "phase_ordered" else None
        phase_evidence = (
            "W06-profile-capture-v1" if phase_order is not None else None
        )
        result = publish_deadline(
            book, context,
            owner=names[index],
            identity=f"event-{index:02d}",
            generation=0,
            at_common=Fraction(10),
            effect_id=f"effect-{index:02d}",
            commutative=commutative,
            phase_order=phase_order,
            phase_evidence=phase_evidence,
        )
        if result.code is not ADeadlineCode.ACCEPTED:
            raise AssertionError(f"fixture publication failed: {result.code.value}")
        book = result.book
    return book


def canonical_digest(result) -> str:
    """Digest the logical result, not wall-clock or host measurement fields."""
    payload = (
        result.code.value,
        result.claimed,
        result.book.watermark_common,
        tuple(sorted(
            (
                item.owner, item.identity, item.generation, item.at_common,
                item.effect_id, item.commutative, item.phase_order,
                item.phase_evidence, item.microstep, item.status.value,
            )
            for item in result.book.deadlines
        )),
    )
    return hashlib.sha256(repr(payload).encode("utf-8")).hexdigest()


def run_point(
    count: int, relation: str, insertion: str, warmup: int, repeats: int,
    cpu_enabled: bool,
) -> dict:
    """Measure close-watermark cost for one immutable simultaneous group."""
    book = build_book(count, relation, insertion)
    expected = (
        ADeadlineCode.INDETERMINATE
        if relation == "unknown" else ADeadlineCode.ACCEPTED
    )
    for _ in range(warmup):
        result = close_watermark(book, Fraction(10))
        if result.code is not expected:
            raise AssertionError(f"unexpected result: {result.code.value}")

    wall_samples: list[int] = []
    cpu_samples: list[int] = []
    clock = getattr(time, "thread_time_ns", None)
    for _ in range(repeats):
        cpu_start = clock() if cpu_enabled and clock is not None else None
        wall_start = time.perf_counter_ns()
        result = close_watermark(book, Fraction(10))
        wall_samples.append(time.perf_counter_ns() - wall_start)
        if cpu_start is not None and clock is not None:
            cpu_samples.append(clock() - cpu_start)
        if result.code is not expected:
            raise AssertionError(f"unexpected result: {result.code.value}")

    tracemalloc.start()
    close_watermark(book, Fraction(10))
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    output = {
        "parameters": {
            "group_size": count,
            "relation": relation,
            "insertion_order": insertion,
        },
        "result": result.code.value,
        "claimed_count": len(result.claimed),
        "output_digest": canonical_digest(result),
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
    parser.add_argument("--group-sizes", default="2,4,8,16")
    parser.add_argument("--relations", default="unknown,commutative,phase_ordered")
    parser.add_argument("--orders", default="forward,reverse")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_w06_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")

    sizes = tuple(int(item) for item in args.group_sizes.split(","))
    relations = tuple(args.relations.split(","))
    orders = tuple(args.orders.split(","))
    if any(item not in {"unknown", "commutative", "phase_ordered"} for item in relations):
        parser.error("relations must be unknown,commutative,phase_ordered")
    if any(item not in {"forward", "reverse"} for item in orders):
        parser.error("orders must be forward,reverse")
    if any(item not in {2, 4, 8, 16} for item in sizes):
        parser.error("W06 group sizes must be selected from 2,4,8,16")

    resolution = cpu_clock_resolution_ns()
    cpu_enabled = resolution is not None and resolution <= 1_000_000
    points = [
        run_point(size, relation, order, args.warmup, args.repeats, cpu_enabled)
        for size in sizes for relation in relations for order in orders
    ]
    document = {
        "schema": "sun.chronos.r3.p-a-w06-ordering.v1",
        "git_head": git_head(),
        "python_version": sys.version,
        "platform": platform.platform(),
        "cpu_clock_resolution_ns": resolution,
        "cpu_time_status": "AVAILABLE" if cpu_enabled else "SKIPPED_COARSE_RESOLUTION",
        "warmup": args.warmup,
        "repeats": args.repeats,
        "points": points,
        "limitations": [
            "synthetic Python benchmark, not native Rust runtime",
            "logical causal-order workload, not physical SMS phase evidence",
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
