"""P-A synthetic deadline lifecycle microbenchmark.

Fixtures are built outside timed regions. Each operation is measured separately
against immutable books using the P-A linear-scan implementation.
"""
from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import time
import tracemalloc
from fractions import Fraction
from pathlib import Path


def cpu_clock_resolution_ns() -> int | None:
    start_wall = time.perf_counter_ns()
    start_cpu = time.thread_time_ns()
    while time.perf_counter_ns() - start_wall < 100_000_000:
        current = time.thread_time_ns()
        if current > start_cpu:
            return current - start_cpu
    return None


from p_a_instruction_catchup import AParticipant
from p_a_deadline_lifecycle import (
    ADeadlineBook,
    ADeadlineContext,
    ADeadlineCode,
    arbitrate_cancellations,
    cancel_deadline,
    close_watermark,
    complete_deadline,
    dispatch_deadline,
    publish_deadline,
)


OPERATIONS = ("publish", "cancel", "arbitrate", "close_watermark", "dispatch", "complete")


def git_head() -> str:
    try:
        root = Path(__file__).resolve().parents[3]
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"


def percentile(values: list[float], p: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * p
    lo = int(position)
    hi = min(lo + 1, len(ordered) - 1)
    fraction = position - lo
    return ordered[lo] * (1 - fraction) + ordered[hi] * fraction


def make_context(domains: int) -> ADeadlineContext:
    participants = tuple(
        AParticipant(
            f"d{i}", Fraction(0), Fraction(1_000_000_000),
            Fraction(1 + (i % 3), 1),
        )
        for i in range(domains)
    )
    return ADeadlineContext(
        participants=participants,
        expected_participants=frozenset(item.name for item in participants),
    )


def build_scheduled_book(context: ADeadlineContext, count: int, density: str) -> ADeadlineBook:
    book = ADeadlineBook()
    for i in range(count):
        at = Fraction(
            10_000 + (i * 100 if density == "sparse" else i % 8)
        )
        result = publish_deadline(
            book, context,
            owner=f"d{i % len(context.participants)}",
            identity=f"event-{i:04d}",
            generation=0,
            at_common=at,
            effect_id=f"effect-{i:04d}",
            commutative=True,
        )
        if result.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"fixture publication failed: {result.code.value} {result.detail}")
        book = result.book
    return book


def fixture_for(
    operation: str, context: ADeadlineContext, count: int, density: str
) -> tuple[ADeadlineBook, dict]:
    book = build_scheduled_book(context, count, density)
    first = book.deadlines[0]
    if operation == "publish":
        return book, {
            "owner": context.participants[0].name,
            "identity": "event-new",
            "generation": 0,
            "at_common": max(item.at_common for item in book.deadlines) + 100,
            "effect_id": "effect-new",
            "commutative": True,
        }
    if operation == "cancel":
        return book, {
            "owner": first.owner,
            "identity": first.identity,
            "generation": first.generation,
            "at_common": first.at_common - 1,
        }
    if operation == "arbitrate":
        pending = book
        for item in book.deadlines:
            result = cancel_deadline(
                pending, context,
                owner=item.owner,
                identity=item.identity,
                generation=item.generation,
                at_common=item.at_common - 1,
            )
            if result.code is not ADeadlineCode.ACCEPTED:
                raise RuntimeError(f"fixture cancellation failed: {result.code.value} {result.detail}")
            pending = result.book
        return pending, {}
    if operation == "close_watermark":
        return book, {"target_common": max(item.at_common for item in book.deadlines) + 1}
    if operation in {"dispatch", "complete"}:
        closed = close_watermark(
            book, max(item.at_common for item in book.deadlines) + 1
        )
        if closed.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"fixture watermark close failed: {closed.code.value}")
        claimed = closed.book
        if operation == "dispatch":
            return claimed, {"identity": first.identity, "generation": first.generation}
        dispatched = dispatch_deadline(
            claimed, identity=first.identity, generation=first.generation
        )
        if dispatched.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"fixture dispatch failed: {dispatched.code.value}")
        return dispatched.book, {"identity": first.identity, "generation": first.generation}
    raise ValueError(f"unknown operation: {operation}")


def invoke(operation: str, book: ADeadlineBook, context: ADeadlineContext, args: dict):
    if operation == "publish":
        return publish_deadline(book, context, **args)
    if operation == "cancel":
        return cancel_deadline(book, context, **args)
    if operation == "arbitrate":
        return arbitrate_cancellations(book)
    if operation == "close_watermark":
        return close_watermark(book, **args)
    if operation == "dispatch":
        return dispatch_deadline(book, **args)
    if operation == "complete":
        return complete_deadline(book, **args)
    raise ValueError(f"unknown operation: {operation}")


def digest(book: ADeadlineBook) -> tuple:
    deadlines = tuple(sorted(
        (
            item.owner, item.identity, item.generation, item.at_common,
            item.effect_id, item.commutative, item.phase_order,
            item.phase_evidence, item.microstep, item.status.value,
        )
        for item in book.deadlines
    ))
    cancellations = tuple(sorted(
        (
            item.owner, item.identity, item.generation, item.at_common,
            item.microstep_level, item.phase_order, item.phase_evidence,
        )
        for item in book.pending_cancellations
    ))
    return deadlines, cancellations, book.watermark_common, book.max_microsteps


def run_point(domains: int, deadlines: int, density: str, operation: str,
              warmup: int, repeats: int, cpu_time_enabled: bool) -> dict:
    context = make_context(domains)
    book, operation_args = fixture_for(operation, context, deadlines, density)

    for _ in range(warmup):
        warm = invoke(operation, book, context, operation_args)
        if warm.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"warm-up operation failed: {warm.code.value}")

    samples_wall_ns = []
    samples_cpu_ns = []
    final_digest = None
    final_code = None
    final_claimed = ()
    for _ in range(repeats):
        wall_start = time.perf_counter_ns()
        cpu_start = time.thread_time_ns() if cpu_time_enabled else 0
        result = invoke(operation, book, context, operation_args)
        samples_cpu_ns.append(time.thread_time_ns() - cpu_start if cpu_time_enabled else None)
        samples_wall_ns.append(time.perf_counter_ns() - wall_start)
        if result.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"timed operation failed: {result.code.value} {result.detail}")
        current_digest = digest(result.book)
        if final_digest is None:
            final_digest = current_digest
            final_code = result.code.value
            final_claimed = result.claimed
        elif current_digest != final_digest or result.claimed != final_claimed:
            raise RuntimeError("P-A deadline operation produced non-deterministic output")

    tracemalloc.start()
    allocation_result = invoke(operation, book, context, operation_args)
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    if digest(allocation_result.book) != final_digest:
        raise RuntimeError("allocation probe changed the deterministic deadline result")

    return {
        "parameters": {
            "domains": domains,
            "deadlines": deadlines,
            "density": density,
            "operation": operation,
        },
        "samples_wall_ns": samples_wall_ns,
        "summary_wall_ns": {
            "min": min(samples_wall_ns),
            "p50": percentile(samples_wall_ns, 0.50),
            "p95": percentile(samples_wall_ns, 0.95),
            "p99": percentile(samples_wall_ns, 0.99),
            "max": max(samples_wall_ns),
            "mean": statistics.fmean(samples_wall_ns),
        },
        "samples_cpu_ns": samples_cpu_ns,
        "summary_cpu_ns": ({
            "min": min(samples_cpu_ns),
            "p50": percentile(samples_cpu_ns, 0.50),
            "p95": percentile(samples_cpu_ns, 0.95),
            "p99": percentile(samples_cpu_ns, 0.99),
            "max": max(samples_cpu_ns),
            "mean": statistics.fmean(samples_cpu_ns),
        } if cpu_time_enabled else None),
        "peak_tracemalloc_bytes_separate_probe": peak_bytes,
        "result": final_code,
        "claimed": final_claimed,
        "output_digest": repr(final_digest),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--domains", default="1,2,4,8,16")
    parser.add_argument("--deadlines", default="1,8,128")
    parser.add_argument("--densities", default="sparse,dense")
    parser.add_argument("--operations", default=",".join(OPERATIONS))
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_deadline_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")

    cpu_resolution = cpu_clock_resolution_ns()
    cpu_time_enabled = cpu_resolution is not None and cpu_resolution <= 1_000_000
    points = []
    for domains in map(int, args.domains.split(",")):
        for deadline_count in map(int, args.deadlines.split(",")):
            for density in args.densities.split(","):
                for operation in args.operations.split(","):
                    if domains <= 0 or deadline_count <= 0:
                        parser.error("domains and deadlines must be positive")
                    if density not in {"sparse", "dense"}:
                        parser.error("density must be sparse or dense")
                    if operation not in OPERATIONS:
                        parser.error(f"unsupported operation: {operation}")
                    points.append(run_point(
                        domains, deadline_count, density, operation,
                        args.warmup, args.repeats, cpu_time_enabled,
                    ))

    report = {
        "schema": "sun.chronos.r3.p-a-deadlines.benchmark.v1",
        "status": "SYNTHETIC_EXPERIMENT",
        "prototype": "P-A linear-scan deadline lifecycle",
        "git_head": git_head(),
        "python": sys.version,
        "platform": platform.platform(),
        "processor": platform.processor(),
        "cpu_clock": "time.thread_time_ns",
        "cpu_clock_resolution_ns": cpu_resolution,
        "cpu_time_status": "MEASURED" if cpu_time_enabled else "SKIPPED_COARSE_RESOLUTION",
        "pythonhashseed": os.environ.get("PYTHONHASHSEED", "unset"),
        "warmup": args.warmup,
        "repeats": args.repeats,
        "timing_scope": "single deadline lifecycle operation; fixture construction, digest, allocation probe and JSON serialization are outside wall/cpu samples",
        "points": points,
        "limitations": [
            "synthetic Python benchmark, not native Rust runtime",
            "no SMS hardware, ROM, audio, video or host frame-pacing claims",
            "deadline lifecycle is measured with immutable full-scan tuples, not a central agenda",
        ],
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": report["status"], "git_head": report["git_head"],
        "points": len(points), "output": str(args.output),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
