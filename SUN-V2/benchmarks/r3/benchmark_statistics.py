"""Shared, explicit statistics policy for comparable R3 benchmark drivers."""
from __future__ import annotations

from collections.abc import Sequence


QUANTILE_ESTIMATOR = "Hyndman-Fan Type 7 (linear interpolation; h=(n-1)*p)"
TIME_UNIT = "ns"
MAX_CPU_CLOCK_RESOLUTION_NS = 10_000


def percentile_type7(values: Sequence[int | float], fraction: float) -> float:
    """Return Hyndman-Fan Type 7 quantile; extrema remain separately observed."""
    if not values:
        raise ValueError("quantile requires at least one sample")
    if not isinstance(fraction, (int, float)) or isinstance(fraction, bool):
        raise TypeError("quantile fraction must be numeric")
    if not 0.0 <= fraction <= 1.0:
        raise ValueError("quantile fraction must be within [0, 1]")
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return float(ordered[lower] + (ordered[upper] - ordered[lower]) * weight)


def cpu_clock_is_usable(resolution_ns: int | None) -> bool:
    """Reject absent, invalid or coarser-than-10-us thread CPU clocks."""
    return (
        isinstance(resolution_ns, int)
        and not isinstance(resolution_ns, bool)
        and 0 < resolution_ns <= MAX_CPU_CLOCK_RESOLUTION_NS
    )
