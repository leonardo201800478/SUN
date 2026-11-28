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
CORPUS_SCHEMA = "sun.chronos.r3.replayable-fixture-corpus.v1"

_RECORDED_FIXTURES: dict[str, dict] = {}


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

    all_equal = first_divergence is None
    digest_payload = {
        "fixture_id": fixture_id,
        "provenance": normalized_provenance,
        "initial_state": normalized_initial,
        "operations": normalize(operations),
        "cuts": cuts,
        "first_divergence": first_divergence,
        "all_equal": all_equal,
    }
    return {
        "schema": SCHEMA,
        **digest_payload,
        "fixture_digest": _sha256(digest_payload),
    }


def reset_replayable_corpus() -> None:
    """Clear explicitly registered workload fixtures before one emitter run."""
    _RECORDED_FIXTURES.clear()


def record_replayable_fixture(fixture: dict) -> str:
    """Register one complete replay fixture for the separate corpus artifact.

    Registration is explicit: helper self-tests are not included unless a
    workload test deliberately records its fixture. Known divergences remain
    data and are not rejected.
    """
    if type(fixture) is not dict or fixture.get("schema") != SCHEMA:
        raise ValueError("unsupported replayable fixture")
    fixture_id = fixture.get("fixture_id")
    if not isinstance(fixture_id, str) or not fixture_id.strip():
        raise ValueError("fixture_id must be a non-empty string")
    digest_fields = (
        "fixture_id", "provenance", "initial_state", "operations",
        "cuts", "first_divergence", "all_equal",
    )
    if any(field not in fixture for field in digest_fields):
        raise ValueError("replayable fixture is missing digest fields")
    digest_payload = {key: fixture[key] for key in digest_fields}
    if fixture.get("fixture_digest") != _sha256(digest_payload):
        raise ValueError("replayable fixture digest mismatch")
    if fixture_id in _RECORDED_FIXTURES:
        raise ValueError(f"duplicate replayable fixture id: {fixture_id}")
    _RECORDED_FIXTURES[fixture_id] = deepcopy(fixture)
    return fixture_id


def build_replayable_document(*, git_head: str, test_count: int) -> dict:
    """Build a deterministic manifest containing complete replayable fixtures."""
    if not isinstance(git_head, str):
        raise TypeError("git_head must be a string")
    if type(test_count) is not int or test_count < 0:
        raise ValueError("test_count must be a non-negative integer")
    fixtures = [
        deepcopy(_RECORDED_FIXTURES[fixture_id])
        for fixture_id in sorted(_RECORDED_FIXTURES)
    ]
    manifest = tuple(
        (fixture["fixture_id"], fixture["fixture_digest"])
        for fixture in fixtures
    )
    return {
        "schema": CORPUS_SCHEMA,
        "git_head": git_head,
        "test_count": test_count,
        "fixture_count": len(fixtures),
        "operation_count": sum(len(item["operations"]) for item in fixtures),
        "cut_count": sum(len(item["cuts"]) for item in fixtures),
        "divergent_fixture_count": sum(not item["all_equal"] for item in fixtures),
        "divergent_fixture_ids": [
            item["fixture_id"] for item in fixtures if not item["all_equal"]
        ],
        "corpus_digest": _sha256(manifest),
        "fixtures": fixtures,
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

    digest_fields = (
        "fixture_id", "provenance", "initial_state", "operations",
        "cuts", "first_divergence", "all_equal",
    )
    digest_payload = {key: fixture[key] for key in digest_fields}
    if fixture.get("fixture_digest") != _sha256(digest_payload):
        raise ValueError("replay fixture digest mismatch")

    operations = fixture["operations"]
    cuts = fixture["cuts"]
    if type(operations) is not list or type(cuts) is not list:
        raise ValueError("replay operations and cuts must be arrays")
    if len(operations) != len(cuts):
        raise ValueError("replay operation/cut count mismatch")

    derived_first_divergence = None
    for operation, cut in zip(operations, cuts, strict=True):
        left_state = normalize(cut["left_state"])
        right_state = normalize(cut["right_state"])
        left_digest = _sha256(left_state)
        right_digest = _sha256(right_state)
        equal = left_digest == right_digest
        if cut.get("left_digest") != left_digest or cut.get("right_digest") != right_digest:
            raise ValueError("replay cut state digest mismatch")
        if type(cut.get("equal")) is not bool or cut["equal"] is not equal:
            raise ValueError("replay cut equality flag mismatch")
        if not equal and derived_first_divergence is None:
            derived_first_divergence = {
                "sequence": operation["sequence"],
                "operation": operation["name"],
                "left_state": left_state,
                "right_state": right_state,
                "left_digest": left_digest,
                "right_digest": right_digest,
            }

    derived_all_equal = derived_first_divergence is None
    if type(fixture.get("all_equal")) is not bool or fixture["all_equal"] is not derived_all_equal:
        raise ValueError("replay all_equal flag is inconsistent with cuts")
    if canonical_bytes(fixture.get("first_divergence")) != canonical_bytes(derived_first_divergence):
        raise ValueError("replay first_divergence is inconsistent with cuts")

    state = deepcopy(fixture["initial_state"])
    observed = []
    for operation, expected_cut in zip(operations, cuts, strict=True):
        state = step(
            deepcopy(state), operation["name"], deepcopy(normalize(operation["args"]))
        )
        normalized = normalize(state)
        expected = normalize(expected_cut[f"{side}_state"])
        if canonical_bytes(normalized) != canonical_bytes(expected):
            raise AssertionError(
                f"{side} replay divergence at operation {operation['sequence']}: "
                f"expected={_sha256(expected)} actual={_sha256(normalized)}"
            )
        observed.append(normalized)
    return tuple(observed)
