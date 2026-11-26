"""Shared canonical trace recorder for P-A versus independent R2 fixtures."""
from __future__ import annotations

import dataclasses
import hashlib
import json
from enum import Enum
from fractions import Fraction


_RECORDS: list[dict] = []
_COUNTS: dict[str, int] = {}


def normalize(value):
    """Convert model values to stable JSON primitives without losing exact time."""
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, Fraction):
        return {"$fraction": [value.numerator, value.denominator]}
    if dataclasses.is_dataclass(value):
        return {
            field.name: normalize(getattr(value, field.name))
            for field in dataclasses.fields(value)
        }
    if isinstance(value, dict):
        return {
            str(key): normalize(item)
            for key, item in sorted(value.items(), key=lambda pair: str(pair[0]))
        }
    if isinstance(value, (tuple, list)):
        return [normalize(item) for item in value]
    if isinstance(value, (set, frozenset)):
        normalized = [normalize(item) for item in value]
        return sorted(normalized, key=lambda item: json.dumps(item, sort_keys=True))
    if isinstance(value, bytes):
        return {"$bytes": value.hex()}
    if type(value) is float:
        raise TypeError("floating-point values are forbidden in canonical traces")
    if value is None or type(value) in (str, int, bool):
        return value
    raise TypeError(f"unsupported canonical trace value: {type(value).__name__}")


def canonical_bytes(value) -> bytes:
    """Serialize one normalized cut with deterministic key and collection order."""
    return json.dumps(
        normalize(value), sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def digest(value) -> str:
    """Return SHA-256 for one canonical state/result cut."""
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _record(fixture_id: str, basis: str, left, right) -> None:
    """Record one cut and fail immediately if the two canonical sides diverge."""
    left_digest = digest(left)
    right_digest = digest(right)
    if left_digest != right_digest:
        raise AssertionError(
            f"canonical trace divergence at {fixture_id}: "
            f"P-A={left_digest} R2/property={right_digest}"
        )
    cut_index = _COUNTS.get(fixture_id, 0)
    _COUNTS[fixture_id] = cut_index + 1
    _RECORDS.append({
        "fixture": fixture_id,
        "cut_index": cut_index,
        "basis": basis,
        "digest": left_digest,
    })


def record_differential(fixture_id: str, p_a_state, r2_state) -> None:
    """Record one operation boundary compared against the independent R2 model."""
    _record(fixture_id, "R2_DIFFERENTIAL", p_a_state, r2_state)


def record_property(fixture_id: str, property_name: str, left, right) -> None:
    """Record a deterministic replay/property cut without claiming an R2 oracle."""
    _record(f"{fixture_id}:{property_name}", "DETERMINISTIC_PROPERTY", left, right)


def reset() -> None:
    """Clear the trace corpus before one complete test-suite execution."""
    _RECORDS.clear()
    _COUNTS.clear()


def build_document(*, git_head: str, test_count: int) -> dict:
    """Build a deterministic trace-corpus document suitable for CI artifacts."""
    fixtures = []
    for fixture_id in sorted(_COUNTS):
        fixtures.append({
            "fixture": fixture_id,
            "cuts": _COUNTS[fixture_id],
        })
    trace_digest = digest(tuple(
        (item["fixture"], item["cut_index"], item["basis"], item["digest"])
        for item in _RECORDS
    ))
    return {
        "schema": "sun.chronos.r3.p-a-canonical-trace-corpus.v1",
        "git_head": git_head,
        "test_count": test_count,
        "fixture_count": len(fixtures),
        "cut_count": len(_RECORDS),
        "differential_cut_count": sum(
            item["basis"] == "R2_DIFFERENTIAL" for item in _RECORDS
        ),
        "property_cut_count": sum(
            item["basis"] == "DETERMINISTIC_PROPERTY" for item in _RECORDS
        ),
        "all_equal": True,
        "trace_digest": trace_digest,
        "fixtures": fixtures,
        "cuts": list(_RECORDS),
    }
