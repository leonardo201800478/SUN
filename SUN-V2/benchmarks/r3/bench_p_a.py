"""P-A synthetic microbenchmark driver.

The driver emits raw samples and an environment manifest. It does not claim
SMS gameplay, frame pacing, audio, or production Rust performance.
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

from p_a_instruction_catchup import (
    AObligation, AParticipant, AState, advance_budgeted, state_digest,
)


def git_head() -> str:
    try:
        root = Path(__file__).resolve().parents[3]
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"


def build_state(domains: int, obligations: int, density: str) -> AState:
    participants = tuple(
        AParticipant(f"d{i}", Fraction(0), Fraction(100_000), Fraction(1 + (i % 3), 1))
        for i in range(domains)
    )
    events = []
    for i in range(obligations):
        # Sparse: events are distant and evenly spaced.
        # Dense: events are clustered in 32 common-time buckets.
        at = Fraction((i + 1) * 1_000 if density == "sparse" else (i % 32) + 1)
        events.append(AObligation(f"e{i}", f"d{i % domains}", at, False, True))
    return AState(participants, tuple(events))


def percentile(values: list[float], p: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    position = (len(ordered) - 1) * p
    lo = int(position)
    hi = min(lo + 1, len(ordered) - 1)
    fraction = position - lo
    return ordered[lo] * (1 - fraction) + ordered[hi] * fraction


def run_point(domains: int, obligations: int, density: str, chunk: int, warmup: int, repeats: int):
    state = build_state(domains, obligations, density)
    target = Fraction(50_000)
    chunk_value = Fraction(chunk)

    for _ in range(warmup):
        advance_budgeted(state, target=target, chunk=chunk_value)

    samples_wall_ns = []
    samples_cpu_ns = []
    final_digest = None
    final_counters = None
    for _ in range(repeats):
        wall_start = time.perf_counter_ns()
        cpu_start = time.process_time_ns()
        result = advance_budgeted(state, target=target, chunk=chunk_value)
        samples_cpu_ns.append(time.process_time_ns() - cpu_start)
        samples_wall_ns.append(time.perf_counter_ns() - wall_start)
        digest = state_digest(result.next_state)
        if final_digest is None:
            final_digest = digest
            final_counters = {
                "result": result.code,
                "reason": result.reason,
                "reached": [result.reached.numerator, result.reached.denominator],
                "chunks": result.chunks,
                "participants_inspected": result.participants_inspected,
                "participants_touched": result.touched_participants,
                "obligations_inspected": result.obligations_inspected,
            }
        elif digest != final_digest:
            raise RuntimeError("P-A produced non-deterministic state digest")

    # Allocation measurement is deliberately outside the timed samples.
    tracemalloc.start()
    allocation_result = advance_budgeted(state, target=target, chunk=chunk_value)
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    if state_digest(allocation_result.next_state) != final_digest:
        raise RuntimeError("allocation probe changed the deterministic output")

    mean_wall_ns = statistics.fmean(samples_wall_ns)
    mean_cpu_ns = statistics.fmean(samples_cpu_ns)
    scan_entries = (
        final_counters["participants_inspected"]
        + final_counters["obligations_inspected"]
    )
    return {
        "parameters": {
            "domains": domains, "obligations": obligations,
            "density": density, "chunk": chunk,
            "target": [target.numerator, target.denominator],
        },
        "samples_wall_ns": samples_wall_ns,
        "summary_wall_ns": {
            "min": min(samples_wall_ns),
            "p50": percentile(samples_wall_ns, 0.50),
            "p95": percentile(samples_wall_ns, 0.95),
            "p99": percentile(samples_wall_ns, 0.99),
            "max": max(samples_wall_ns),
            "mean": mean_wall_ns,
        },
        "samples_cpu_ns": samples_cpu_ns,
        "summary_cpu_ns": {
            "min": min(samples_cpu_ns),
            "p50": percentile(samples_cpu_ns, 0.50),
            "p95": percentile(samples_cpu_ns, 0.95),
            "p99": percentile(samples_cpu_ns, 0.99),
            "max": max(samples_cpu_ns),
            "mean": mean_cpu_ns,
        },
        "peak_tracemalloc_bytes_separate_probe": peak_bytes,
        "inspected_entries_per_second": (
            scan_entries * 1_000_000_000 / mean_wall_ns if mean_wall_ns > 0 else 0
        ),
        "mean_ns_per_inspected_entry": (
            mean_wall_ns / scan_entries if scan_entries > 0 else None
        ),
        "output_digest": repr(final_digest),
        "counters": final_counters,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--domains", default="1,2,4,8,16")
    parser.add_argument("--obligations", default="0,8,128")
    parser.add_argument("--chunks", default="64,1024,8192")
    parser.add_argument("--densities", default="sparse,dense")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")

    points = []
    for domains in map(int, args.domains.split(",")):
        for obligations in map(int, args.obligations.split(",")):
            for chunk in map(int, args.chunks.split(",")):
                densities = ["none"] if obligations == 0 else args.densities.split(",")
                for density in densities:
                    if domains <= 0 or obligations < 0 or chunk <= 0:
                        parser.error("domains/chunk must be positive; obligations cannot be negative")
                    if density not in {"none", "sparse", "dense"}:
                        parser.error("density must be none, sparse or dense")
                    points.append(run_point(
                        domains, obligations, density, chunk,
                        args.warmup, args.repeats,
                    ))

    report = {
        "schema": "sun.chronos.r3.p-a.benchmark.v1",
        "status": "SYNTHETIC_EXPERIMENT",
        "prototype": "P-A instruction/budget plus catch-up",
        "git_head": git_head(),
        "python": sys.version,
        "platform": platform.platform(),
        "processor": platform.processor(),
        "pythonhashseed": os.environ.get("PYTHONHASHSEED", "unset"),
        "warmup": args.warmup,
        "repeats": args.repeats,
        "timing_scope": "advance_budgeted only; fixture construction, digest, allocation probe and JSON serialization are outside wall/cpu samples",
        "points": points,
        "limitations": [
            "synthetic Python benchmark, not native Rust runtime",
            "no SMS hardware, ROM, audio, video or host frame-pacing claims",
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
