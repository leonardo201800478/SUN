"""P-A W03/W04 synthetic causal synchronization benchmark."""
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

from benchmark_statistics import (
    MAX_CPU_CLOCK_RESOLUTION_NS,
    QUANTILE_ESTIMATOR,
    TIME_UNIT,
    cpu_clock_is_usable,
    percentile_type7,
)

from p_a_causal_sync import (
    ACausalCode,
    ACausalState,
    ADevice,
    ATransition,
    synchronize,
)


def cpu_clock_resolution_ns() -> int | None:
    start_wall = time.perf_counter_ns()
    start_cpu = time.thread_time_ns()
    while time.perf_counter_ns() - start_wall < 100_000_000:
        current = time.thread_time_ns()
        if current > start_cpu:
            return current - start_cpu
    return None


def git_head() -> str:
    try:
        root = Path(__file__).resolve().parents[3]
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"


def percentile(values: list[float], p: float) -> float:
    """Use the shared Hyndman-Fan Type 7 estimator."""
    return percentile_type7(values, p)


def chain_fixture(fan_in: int, depth: int):
    names = tuple(f"d{i}" for i in range(fan_in))
    transitions = {name: [] for name in names}
    previous = None
    for index in range(depth):
        owner = names[index % fan_in]
        identity = f"chain-{index:04d}"
        transitions[owner].append(ATransition(
            identity,
            Fraction(index + 1),
            () if previous is None else (previous,),
            False,
        ))
        previous = identity
    cpu_transition = ATransition(
        "cpu-observation", Fraction(depth + 1), (previous,), False
    )
    devices = [ADevice("cpu", Fraction(0), (cpu_transition,), ())]
    devices.extend(
        ADevice(name, Fraction(0), tuple(transitions[name]), ())
        for name in names
    )
    return ACausalState(tuple(devices)), names, Fraction(depth + 1)


def frequent_fixture(fan_in: int, observations: int):
    names = tuple(f"d{i}" for i in range(fan_in))
    devices = [ADevice("cpu", Fraction(0), (), ())]
    for name in names:
        transitions = tuple(
            ATransition(
                f"{name}-observation-{tick:04d}",
                Fraction(tick),
                (),
                True,
            )
            for tick in range(1, observations + 1)
        )
        devices.append(ADevice(name, Fraction(0), transitions, ()))
    return ACausalState(tuple(devices)), names


def digest(state: ACausalState):
    return tuple(
        (
            device.name,
            device.progress_common,
            tuple(
                (item.identity, item.at_common, item.requires, item.commutative)
                for item in device.transitions
            ),
            device.applied,
        )
        for device in state.devices
    )


def invoke_chain(state: ACausalState, names: tuple[str, ...], target: Fraction):
    return synchronize(
        state, target_device="cpu", at_common=target, observes=names
    )


def invoke_frequent(state: ACausalState, names: tuple[str, ...], observations: int):
    applied_order = []
    current = state
    for tick in range(1, observations + 1):
        result = synchronize(
            current,
            target_device="cpu",
            at_common=Fraction(tick),
            observes=names,
        )
        if result.code is not ACausalCode.SYNCHRONIZED:
            raise RuntimeError(f"frequent observation failed: {result.code.value} {result.detail}")
        current = result.state
        applied_order.extend(result.applied_order)
    return current, tuple(applied_order)


def run_point(kind: str, fan_in: int, scale: int, warmup: int,
              repeats: int, cpu_time_enabled: bool):
    if kind == "dependency_chain":
        state, names, target = chain_fixture(fan_in, scale)

        def operation():
            result = invoke_chain(state, names, target)
            if result.code is not ACausalCode.SYNCHRONIZED:
                raise RuntimeError(f"dependency chain failed: {result.code.value} {result.detail}")
            return result.state, result.applied_order
        calls_per_sample = 1
        parameters = {"kind": kind, "fan_in": fan_in, "dependency_depth": scale}
    else:
        state, names = frequent_fixture(fan_in, scale)

        def operation():
            return invoke_frequent(state, names, scale)
        calls_per_sample = scale
        parameters = {"kind": kind, "fan_in": fan_in, "observation_count": scale}

    for _ in range(warmup):
        operation()

    samples_wall_ns = []
    samples_cpu_ns = []
    final_digest = None
    final_order = None
    for _ in range(repeats):
        wall_start = time.perf_counter_ns()
        cpu_start = time.thread_time_ns() if cpu_time_enabled else 0
        result_state, order = operation()
        samples_cpu_ns.append(time.thread_time_ns() - cpu_start if cpu_time_enabled else None)
        samples_wall_ns.append(time.perf_counter_ns() - wall_start)
        current_digest = digest(result_state)
        if final_digest is None:
            final_digest = current_digest
            final_order = order
        elif current_digest != final_digest or order != final_order:
            raise RuntimeError("causal synchronization result is not deterministic")

    tracemalloc.start()
    allocation_state, allocation_order = operation()
    _, peak_bytes = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    if digest(allocation_state) != final_digest or allocation_order != final_order:
        raise RuntimeError("allocation probe changed causal result")

    cpu_summary = None
    if cpu_time_enabled:
        cpu_summary = {
            "min": min(samples_cpu_ns),
            "p50": percentile(samples_cpu_ns, 0.50),
            "p95": percentile(samples_cpu_ns, 0.95),
            "p99": percentile(samples_cpu_ns, 0.99),
            "max": max(samples_cpu_ns),
            "mean": statistics.fmean(samples_cpu_ns),
        }
    return {
        "sample_count": len(samples_wall_ns),
        "parameters": parameters,
        "calls_per_sample": calls_per_sample,
        "samples_wall_ns": samples_wall_ns,
        "summary_wall_ns": {
            "min": min(samples_wall_ns),
            "p50": percentile(samples_wall_ns, 0.50),
            "p95": percentile(samples_wall_ns, 0.95),
            "p99": percentile(samples_wall_ns, 0.99),
            "max": max(samples_wall_ns),
            "mean": statistics.fmean(samples_wall_ns),
        },
        "samples_cpu_ns": samples_cpu_ns if cpu_time_enabled else None,
        "summary_cpu_ns": cpu_summary,
        "peak_tracemalloc_bytes_separate_probe": peak_bytes,
        "applied_count": len(final_order),
        "output_digest": repr(final_digest),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fan-in", default="1,2,4,8,16")
    parser.add_argument("--depths", default="1,4,16")
    parser.add_argument("--observation-counts", default="1,8,32")
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--repeats", type=int, default=30)
    parser.add_argument("--output", type=Path, default=Path("p_a_causal_results.json"))
    args = parser.parse_args()
    if args.warmup < 0 or args.repeats <= 0:
        parser.error("warmup must be >=0 and repeats must be >0")
    cpu_resolution = cpu_clock_resolution_ns()
    cpu_enabled = cpu_clock_is_usable(cpu_resolution)
    fan_ins = tuple(map(int, args.fan_in.split(",")))
    points = []
    for fan_in in fan_ins:
        if fan_in <= 0:
            parser.error("fan-in must be positive")
        for depth in map(int, args.depths.split(",")):
            if depth <= 0:
                parser.error("dependency depth must be positive")
            points.append(run_point(
                "dependency_chain", fan_in, depth,
                args.warmup, args.repeats, cpu_enabled,
            ))
        for observations in map(int, args.observation_counts.split(",")):
            if observations <= 0:
                parser.error("observation count must be positive")
            points.append(run_point(
                "frequent_observation", fan_in, observations,
                args.warmup, args.repeats, cpu_enabled,
            ))
    report = {
        "schema": "sun.chronos.r3.p-a-causal.benchmark.v1",
        "status": "SYNTHETIC_EXPERIMENT",
        "prototype": "P-A serial full-scan causal synchronization",
        "git_head": git_head(),
        "python": sys.version,
        "platform": platform.platform(),
        "processor": platform.processor(),
        "cpu_clock": "time.thread_time_ns",
        "cpu_clock_resolution_ns": cpu_resolution,
        "cpu_time_status": "MEASURED" if cpu_enabled else "SKIPPED_COARSE_RESOLUTION",
        "pythonhashseed": os.environ.get("PYTHONHASHSEED", "unset"),
        "warmup": args.warmup,
        "repeats": args.repeats,
        "timing_unit": TIME_UNIT,
        "quantile_estimator": QUANTILE_ESTIMATOR,
        "cpu_clock_max_resolution_ns": MAX_CPU_CLOCK_RESOLUTION_NS,
        "timing_scope": "one dependency synchronization or a sequence of frequent observations; fixture construction, digest, allocation probe and JSON serialization excluded",
        "points": points,
        "limitations": [
            "synthetic Python benchmark, not native Rust runtime",
            "no physical SMS clock, ROM, audio, video or frame-pacing claims",
            "serial full-scan causal resolution; no persistent agenda",
        ],
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": report["status"],
        "git_head": report["git_head"],
        "points": len(points),
        "output": str(args.output),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
