"""Experimental declarative, replayable differential fixture corpus.

This module is a disposable R3 harness prototype. It does not change R1/R2
semantics and is not yet wired into the existing P-A workload fixtures.
"""
from __future__ import annotations

import hashlib
from copy import deepcopy
from typing import Callable

from trace_fixture_support import canonical_bytes, normalize


SCHEMA = "sun.chronos.r3.replayable-fixture.v0.1"


def _sha256(value) -> str:
    """Hash a value using the canonical exact-value serializer."""
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _validate_operations(operations: tuple[dict, ...]) -> None:
    """Require stable, contiguous operation sequence numbers and JSON data."""
    if type(operations) is not tuple:
        raise TypeError("operations must be an immutable tuple")
    for expected_sequence, operation in enumerate(operations):
        if type(operation) is not dict:
            raise TypeError("each operation must be a dictionary")
        if operation.get("sequence") != expected_sequence:
            raise ValueError("operation sequence must be contiguous from zero")
        if not isinstance(operation.get("name"), str) or not operation["name"].strip():
            raise ValueError("operation name must be a non-empty string")
        if "args" not in operation:
            raise ValueError("operation args are required")
        normalize(operation["args"])


def capture_differential_fixture(
    *,
    fixture_id: str,
    provenance: dict,
    initial_state,
    operations: tuple[dict, ...],
    left_step: Callable,
    right_step: Callable,
) -> dict:
    """Capture replay inputs and every full differential state cut.

    Divergence is recorded as data and does not abort capture. The first
    divergent cut is identified explicitly so the artifact remains diagnosable.
    """
    if not isinstance(fixture_id, str) or not fixture_id.strip():
        raise ValueError("fixture_id must be a non-empty string")
    if type(provenance) is not dict:
        raise TypeError("provenance must be a dictionary")
    _validate_operations(operations)
    normalized_initial = normalize(initial_state)
    normalized_provenance = normalize(provenance)

    left_state = deepcopy(normalized_initial)
    right_state = deepcopy(normalized_initial)
    cuts = []
    first_divergence = None

    for operation in operations:
        normalized_args = normalize(operation["args"])
        left_state = left_step(deepcopy(left_state), operation["name"], deepcopy(normalized_args))
        right_state = right_step(deepcopy(right_state), operation["name"], deepcopy(normalized_args))
        left_observed = normalize(left_state)
        right_observed = normalize(right_state)
        left_digest = _sha256(left_observed)
        right_digest = _sha256(right_observed)
        equal = left_digest == right_digest
        cut = {
            "sequence": operation["sequence"],
            "operation": operation["name"],
            "left_state": left_observed,
            "right_state": right_observed,
            "left_digest": left_digest,
            "right_digest": right_digest,
            "equal": equal,
        }
        cuts.append(cut)
        if not equal and first_divergence is None:
            first_divergence = {
                "sequence": operation["sequence"],
                "operation": operation["name"],
                "left_state": left_observed,
                "right_state": right_observed,
                "left_digest": left_digest,
                "right_digest": right_digest,
            }

    return {
        "schema": SCHEMA,
        "fixture_id": fixture_id,
        "provenance": normalized_provenance,
        "initial_state": normalized_initial,
        "operations": normalize(operations),
        "cuts": cuts,
        "first_divergence": first_divergence,
        "all_equal": first_divergence is None,
        "fixture_digest": _sha256({
            "fixture_id": fixture_id,
            "provenance": normalized_provenance,
            "initial_state": normalized_initial,
            "operations": normalize(operations),
            "cuts": cuts,
            "first_divergence": first_divergence,
        }),
    }


def replay_fixture_side(
    fixture: dict,
    *,
    step: Callable,
    side: str,
) -> tuple:
    """Replay the stored inputs and verify each recorded full-state cut."""
    if side not in ("left", "right"):
        raise ValueError("side must be 'left' or 'right'")
    if fixture.get("schema") != SCHEMA:
        raise ValueError("unsupported replay fixture schema")

    state = fixture["initial_state"]
    observed = []
    for operation, expected_cut in zip(
        fixture["operations"], fixture["cuts"], strict=True
    ):
        state = step(state, operation["name"], normalize(operation["args"]))
        normalized = normalize(state)
        expected = expected_cut[f"{side}_state"]
        if normalized != expected:
            raise AssertionError(
                f"{side} replay divergence at operation {operation['sequence']}: "
                f"expected={_sha256(expected)} actual={_sha256(normalized)}"
            )
        observed.append(normalized)
    return tuple(observed)
