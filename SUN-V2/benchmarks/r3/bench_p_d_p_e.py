"""Paired synthetic benchmarks for the first P-D/P-E candidate slices.

P-D is compared with P-A for W01/W02 deadline publication/claim order.
P-E is compared with P-A for W03/W04 causal dependency/frequent observation.
Fixtures are built outside timed regions and every sample verifies equal outputs.
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
from itertools import combinations
from pathlib import Path

from benchmark_statistics import (
    MAX_CPU_CLOCK_RESOLUTION_NS,
    QUANTILE_ESTIMATOR,
    TIME_UNIT,
    cpu_clock_is_usable,
    percentile_type7,
)
from p_a_causal_sync import (
    ACausalState,
    ADevice,
    ATransition,
    synchronize as p_a_synchronize,
)
from p_a_deadline_lifecycle import (
    ADeadlineBook,
    ADeadlineCode,
    ADeadlineContext,
    close_watermark as p_a_close_watermark,
    publish_deadline as p_a_publish_deadline,
)
from p_a_instruction_catchup import AParticipant
from p_d_discrete_agenda import (
    AgendaCode,
    AgendaConstraint,
    AgendaState,
    PairRelation as DPairRelation,
    close_watermark as p_d_close_watermark,
    publish as p_d_publish,
)
from p_e_sync_on_demand import (
    PairRelation as EPairRelation,
    SyncDomain,
    SyncConstraint,
    SyncState,
    SyncTransition,
    synchronize as p_e_synchronize,
)


def cpu_clock_resolution_ns() -> int | None:
    """Estimate thread CPU clock resolution independently of timed samples."""
    wall_start = time.perf_counter_ns()
    cpu_start = time.thread_time_ns()
    while time.perf_counter_ns() - wall_start < 100_000_000:
        current = time.thread_time_ns()
        if current > cpu_start:
            return current - cpu_start
    return None


def git_head() -> str:
    """Return the exact repository HEAD used by this benchmark process."""
    try:
        root = Path(__file__).resolve().parents[3]
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"


def summary(values: list[int]) -> dict:
    """Summarize nanosecond samples using the shared Type-7 estimator."""
    return {
        "min": min(values),
        "p50": percentile_type7(values, 0.50),
        "p95": percentile_type7(values, 0.95),
        "p99": percentile_type7(values, 0.99),
        "max": max(values),
        "mean": statistics.fmean(values),
    }


def _measure(operation, cpu_enabled: bool) -> tuple[int, int | None, object]:
    """Measure one immutable-fixture operation and return its output."""
    wall_start = time.perf_counter_ns()
    cpu_start = time.thread_time_ns() if cpu_enabled else 0
    output = operation()
    cpu_elapsed = time.thread_time_ns() - cpu_start if cpu_enabled else None
    return time.perf_counter_ns() - wall_start, cpu_elapsed, output


def _allocation_probe(operation, expected) -> int:
    """Measure peak Python allocation separately from latency samples."""
    tracemalloc.start()
    output = operation()
    _, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    if output != expected:
        raise RuntimeError("allocation probe changed deterministic output")
    return peak


def _build_deadline_pair(domains: int, count: int, density: str):
    names = tuple(f"d{i}" for i in range(domains))
    participants = tuple(
        AParticipant(name, Fraction(0), Fraction(1_000_000_000), Fraction(1))
        for name in names
    )
    context = ADeadlineContext(participants, frozenset(names))
    p_a_book = ADeadlineBook()
    p_d_state = AgendaState()
    ids_by_time: dict[Fraction, list[str]] = {}
    for index in range(count):
        at = (
            Fraction(10_000 + index * 100)
            if density == "sparse"
            else Fraction(10_000 + index % 8)
        )
        identity = f"deadline-{index:05d}"
        owner = names[index % len(names)]
        reference = p_a_publish_deadline(
            p_a_book,
            context,
            owner=owner,
            identity=identity,
            generation=0,
            at_common=at,
            effect_id=f"effect-{index:05d}",
            commutative=True,
        )
        if reference.code is not ADeadlineCode.ACCEPTED:
            raise RuntimeError(f"P-A fixture publication failed: {reference.code.value}")
        p_a_book = reference.book
        candidate = p_d_publish(
            p_d_state,
            identity=identity,
            generation=0,
            owner=owner,
            at_common=at,
            effect_id=f"effect-{index:05d}",
        )
        if candidate.code is not AgendaCode.ACCEPTED:
            raise RuntimeError(f"P-D fixture publication failed: {candidate.code.value}")
        p_d_state = candidate.state
        ids_by_time.setdefault(at, []).append(identity)

    constraints = tuple(
        AgendaConstraint(first, second, DPairRelation.COMMUTATIVE)
        for group in ids_by_time.values()
        for first, second in combinations(group, 2)
    )
    p_d_state = AgendaState(
        p_d_state.heap, p_d_state.events, constraints, p_d_state.max_events
    )
    target = max(ids_by_time) + 1
    return p_a_book, context, p_d_state, target


def _p_a_close_operation(book, target):
    """Close the P-A watermark and return its exact claimed identity sequence."""
    result = p_a_close_watermark(book, target)
    if result.code is not ADeadlineCode.ACCEPTED:
        raise RuntimeError(f"P-A close failed: {result.code.value} {result.detail}")
    return result.claimed


def _p_d_claim_operation(state, target):
    """Close the P-D watermark atomically and return its claimed identity sequence."""
    result = p_d_close_watermark(state, target)
    if result.code is not AgendaCode.ACCEPTED:
        raise RuntimeError(f"P-D watermark close failed: {result.code.value} {result.detail}")
    return result.claimed


def run_p_d_point(domains: int, count: int, density: str,
                  warmup: int, repeats: int, cpu_enabled: bool) -> dict:
    """Compare P-A linear close with P-D heap claim on identical W01/W02 fixtures."""
    p_a_book, _, p_d_state, target = _build_deadline_pair(domains, count, density)
    p_a_operation = lambda: _p_a_close_operation(p_a_book, target)
    p_d_operation = lambda: _p_d_claim_operation(p_d_state, target)

    expected = p_a_operation()
    if p_d_operation() != expected:
        raise RuntimeError("P-D claim sequence differs from P-A on the same deadline fixture")
    for _ in range(warmup):
        if p_a_operation() != expected or p_d_operation() != expected:
            raise RuntimeError("P-D/P-A warm-up output mismatch")

    samples = {"p_a_wall_ns": [], "p_d_wall_ns": []}
    cpu_samples = {"p_a_cpu_ns": [], "p_d_cpu_ns": []}
    for index in range(repeats):
        operations = (
            (("p_a", p_a_operation), ("p_d", p_d_operation))
            if index % 2 == 0
            else (("p_d", p_d_operation), ("p_a", p_a_operation))
        )
        for name, operation in operations:
            wall, cpu, output = _measure(operation, cpu_enabled)
            if output != expected:
                raise RuntimeError(f"{name} output changed during timed sample")
            samples[f"{name}_wall_ns"].append(wall)
            if cpu_enabled:
                cpu_samples[f"{name}_cpu_ns"].append(cpu)

    peak_a = _allocation_probe(p_a_operation, expected)
    peak_d = _allocation_probe(p_d_operation, expected)
    return {
        "family": "P-D W01/W02 deadline claim",
        "parameters": {"domains": domains, "deadlines": count, "density": density},
        "sample_count": repeats,
        "p_a": {
            "samples_wall_ns": samples["p_a_wall_ns"],
            "summary_wall_ns": summary(samples["p_a_wall_ns"]),
            "samples_cpu_ns": cpu_samples["p_a_cpu_ns"] if cpu_enabled else None,
            "summary_cpu_ns": summary(cpu_samples["p_a_cpu_ns"]) if cpu_enabled else None,
            "peak_tracemalloc_bytes_separate_probe": peak_a,
        },
        "p_d": {
            "samples_wall_ns": samples["p_d_wall_ns"],
            "summary_wall_ns": summary(samples["p_d_wall_ns"]),
            "samples_cpu_ns": cpu_samples["p_d_cpu_ns"] if cpu_enabled else None,
            "summary_cpu_ns": summary(cpu_samples["p_d_cpu_ns"]) if cpu_enabled else None,
            "peak_tracemalloc_bytes_separate_probe": peak_d,
        },
        "claimed_count": len(expected),
        "output_digest": repr(expected),
        "equivalent_output": True,
    }


def _build_causal_pair(fan_in: int, scale: int, kind: str):
    names = tuple(f"d{i}" for i in range(fan_in))
    p_a_transitions: dict[str, list[ATransition]] = {name: [] for name in names}
    p_e_transitions: list[SyncTransition] = []
    constraints: list[SyncConstraint] = []

    if kind == "dependency_chain":
        previous = None
        for index in range(scale):
            owner = names[index % fan_in]
            identity = f"chain-{index:05d}"
            at = Fraction(index + 1)
            requires = () if previous is None else (previous,)
            p_a_transitions[owner].append(
                ATransition(identity, at, requires, False)
            )
            p_e_transitions.append(
                SyncTransition(identity, owner, at, requires)
            )
            previous = identity
        target = Fraction(scale + 1)
        p_a_cpu = (ATransition("cpu-observation", target, (previous,), False),)
        p_e_transitions.append(
            SyncTransition("cpu-observation", "cpu", target, (previous,))
        )
    elif kind == "frequent_observation":
        target = Fraction(scale)
        p_a_cpu = ()
        for owner in names:
            for tick in range(1, scale + 1):
                identity = f"{owner}-observation-{tick:05d}"
                at = Fraction(tick)
                p_a_transitions[owner].append(
                    ATransition(identity, at, (), True)
                )
                p_e_transitions.append(SyncTransition(identity, owner, at))
        for tick in range(1, scale + 1):
            group = tuple(
                f"{owner}-observation-{tick:05d}" for owner in names
            )
            constraints.extend(
                SyncConstraint(first, second, EPairRelation.COMMUTATIVE)
                for first, second in combinations(group, 2)
            )
    else:
        raise ValueError(f"unsupported causal fixture kind: {kind}")

    p_a_state = ACausalState(tuple(
        [ADevice("cpu", Fraction(0), p_a_cpu, ())]
        + [
            ADevice(name, Fraction(0), tuple(p_a_transitions[name]), ())
            for name in names
        ]
    ))
    p_e_domains = tuple(
        SyncDomain(name, Fraction(0), target + 1)
        for name in ("cpu", *names)
    )
    p_e_state = SyncState(
        p_e_domains, tuple(p_e_transitions), tuple(constraints)
    )
    requested = ("cpu", *names)
    return p_a_state, p_e_state, requested, target


def _p_a_causal_operation(state, requested, target, kind, scale):
    """Run the P-A serial resolver using the declared observation cadence."""
    current = state
    applied = []
    targets = (target,) if kind == "dependency_chain" else tuple(
        Fraction(tick) for tick in range(1, scale + 1)
    )
    for instant in targets:
        result = p_a_synchronize(
            current,
            target_device="cpu",
            at_common=instant,
            observes=tuple(name for name in requested if name != "cpu"),
        )
        if result.code.value != "Synchronized":
            raise RuntimeError(f"P-A causal sync failed: {result.code.value} {result.detail}")
        current = result.state
        applied.extend(result.applied_order)
    return tuple(applied)


def _p_e_sync_operation(state, requested, target, kind, scale):
    """Run P-E on the same causal requests and exact observation cuts."""
    current = state
    targets = (target,) if kind == "dependency_chain" else tuple(
        Fraction(tick) for tick in range(1, scale + 1)
    )
    for instant in targets:
        result = p_e_synchronize(
            current,
            requested_domains=requested,
            target_common=instant,
        )
        if result.code.value not in {"Reached", "StoppedAt"}:
            raise RuntimeError(f"P-E causal sync failed: {result.code.value} {result.detail}")
        current = result.state
    return current.observed


def run_p_e_point(fan_in: int, scale: int, kind: str,
                  warmup: int, repeats: int, cpu_enabled: bool) -> dict:
    """Compare P-E closure with P-A on identical W03/W04 causal fixtures."""
    p_a_state, p_e_state, requested, target = _build_causal_pair(fan_in, scale, kind)
    p_a_operation = lambda: _p_a_causal_operation(
        p_a_state, requested, target, kind, scale
    )
    p_e_operation = lambda: _p_e_sync_operation(
        p_e_state, requested, target, kind, scale
    )
    expected = p_a_operation()
    if p_e_operation() != expected:
        raise RuntimeError("P-E applied sequence differs from P-A on the same causal fixture")
    for _ in range(warmup):
        if p_a_operation() != expected or p_e_operation() != expected:
            raise RuntimeError("P-E/P-A warm-up output mismatch")

    samples = {"p_a_wall_ns": [], "p_e_wall_ns": []}
    cpu_samples = {"p_a_cpu_ns": [], "p_e_cpu_ns": []}
    for index in range(repeats):
        operations = (
            (("p_a", p_a_operation), ("p_e", p_e_operation))
            if index % 2 == 0
            else (("p_e", p_e_operation), ("p_a", p_a_operation))
        )
        for name, operation in operations:
            wall, cpu, output = _measure(operation, cpu_enabled)
            if output != expected:
                raise RuntimeError(f"{name} output changed during timed sample")
            samples[f"{name}_wall_ns"].append(wall)
            if cpu_enabled:
                cpu_samples[f"{name}_cpu_ns"].append(cpu)

    peak_a = _allocation_probe(p_a_operation, expected)
    peak_e = _allocation_probe(p_e_operation, expected)
    return {
        "family": "P-E W03/W04 causal synchronization",
        "parameters": {"kind": kind, "fan_in": fan_in, "depth_or_observations": scale},
        "sample_count": repeats,
        "p_a": {
            "samples_wall_ns": samples["p_a_wall_ns"],
            "summary_wall_ns": summary(samples["p_a_wall_ns"]),
            "samples_cpu_ns": cpu_samples["p_a_cpu_ns"] if cpu_enabled else None,
            "summary_cpu_ns": summary(cpu_samples["p_a_cpu_ns"]) if cpu_enabled else None,
            "peak_tracemalloc_bytes_separate_probe": peak_a,
        },
        "p_e": {
            "samples_wall_ns": samples["p_e_wall_ns"],
            "summary_wall_ns": summary(samples["p_e_wall_ns"]),
            "samples_cpu_ns": cpu_samples["p_e_cpu_ns"] if cpu_enabled else None,
            "summary_cpu_ns": summary(cpu_samples["p_e_cpu_ns"]) if cpu_enabled else None,
            "peak_tracemalloc_bytes_separate_probe": peak_e,
        },
        "applied_count": len(expected),
        "output_digest": repr(expected),
        "equivalent_output": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--domains", default="1,4,16")
    parser.add_argument("--deadlines", default="8,32,128")
    parser.add_argument("--densities", default="sparse,dense")
    parser.add_argument("--fan-in", default="1,4,16")
    parser.add_argument("--depths", default="1,4,16")
    parser.add_argument("--observation-counts", default="1,8,32")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("r3-pd-pe-results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >= 0 and repeats must be > 0")

    resolution = cpu_clock_resolution_ns()
    cpu_enabled = cpu_clock_is_usable(resolution)
    points = []
    for domains in map(int, args.domains.split(",")):
        for count in map(int, args.deadlines.split(",")):
            for density in args.densities.split(","):
                if domains <= 0 or count <= 0 or density not in {"sparse", "dense"}:
                    parser.error("invalid P-D matrix parameters")
                points.append(run_p_d_point(
                    domains, count, density, args.warmup, args.repeats, cpu_enabled
                ))
    for fan_in in map(int, args.fan_in.split(",")):
        if fan_in <= 0:
            parser.error("fan-in must be positive")
        for depth in map(int, args.depths.split(",")):
            if depth <= 0:
                parser.error("dependency depth must be positive")
            points.append(run_p_e_point(
                fan_in, depth, "dependency_chain",
                args.warmup, args.repeats, cpu_enabled,
            ))
        for observations in map(int, args.observation_counts.split(",")):
            if observations <= 0:
                parser.error("observation count must be positive")
            points.append(run_p_e_point(
                fan_in, observations, "frequent_observation",
                args.warmup, args.repeats, cpu_enabled,
            ))

    report = {
        "schema": "sun.chronos.r3.pd-pe-paired-benchmark.v1",
        "status": "SYNTHETIC_EXPERIMENT",
        "git_head": git_head(),
        "python": sys.version,
        "platform": platform.platform(),
        "processor": platform.processor(),
        "cpu_clock": "time.thread_time_ns",
        "cpu_clock_resolution_ns": resolution,
        "cpu_time_status": "MEASURED" if cpu_enabled else "SKIPPED_COARSE_RESOLUTION",
        "pythonhashseed": os.environ.get("PYTHONHASHSEED", "unset"),
        "warmup": args.warmup,
        "repeats": args.repeats,
        "timing_unit": TIME_UNIT,
        "quantile_estimator": QUANTILE_ESTIMATOR,
        "cpu_clock_max_resolution_ns": MAX_CPU_CLOCK_RESOLUTION_NS,
        "fixture_building_and_json_serialization_in_timed_region": False,
        "points": points,
        "limitations": [
            "synthetic Python measurements only; not native Rust runtime or SMS hardware",
            "P-D currently measures W01/W02 heap claim order, not cancellation admission/arbitration or full watermark participant contracts",
            "P-E currently measures W03/W04 causal closure; W06/W07 are conformance tests, not timing benchmarks",
            "W08 transaction lifecycle and W09 snapshot/replay are not yet integrated into P-D/P-E candidate state",
            "wall-time is comparable only within the same runner and execution; coarse Windows CPU clock is not measured",
        ],
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": report["status"],
        "git_head": report["git_head"],
        "points": len(points),
        "p_d_points": sum(item["family"].startswith("P-D") for item in points),
        "p_e_points": sum(item["family"].startswith("P-E") for item in points),
        "cpu_time_status": report["cpu_time_status"],
        "output": str(args.output),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
