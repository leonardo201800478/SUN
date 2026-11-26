"""Disposable W07 pair-relation adjudicator for R1 §16.6 review.

This module evaluates an explicitly supplied relation between two obligations.
It does not infer causality or commutativity, assign physical phases, or change
P-A/R2 behavior. The caller must justify the relation independently.
"""
from __future__ import annotations

from enum import Enum


class PairRelation(str, Enum):
    """Explicit relation supplied for one unordered obligation pair."""

    FIRST_CAUSES_SECOND = "first_causes_second"
    SECOND_CAUSES_FIRST = "second_causes_first"
    COMMUTATIVE = "commutative"
    UNKNOWN = "unknown"


class PairDecision(str, Enum):
    """Admissibility of attempting one member of the pair next."""

    ADVANCE = "Advance"
    BLOCK = "Blocked"
    INDETERMINATE = "IndeterminateCausalOrder"


def adjudicate_pair(
    *,
    first_id: str,
    second_id: str,
    relation: PairRelation,
    requested_id: str,
) -> PairDecision:
    """Evaluate one requested obligation against an explicit pair relation.

    Causal relations require their predecessor first. A declared commutative
    pair permits either order. An unknown relation remains indeterminate for
    either requested side; no microstep number or insertion order is inferred.
    """
    if (
        not isinstance(first_id, str)
        or not first_id.strip()
        or not isinstance(second_id, str)
        or not second_id.strip()
        or first_id == second_id
        or not isinstance(requested_id, str)
        or requested_id not in (first_id, second_id)
        or not isinstance(relation, PairRelation)
    ):
        raise ValueError("invalid W07 pair relation descriptor")

    if relation is PairRelation.UNKNOWN:
        return PairDecision.INDETERMINATE
    if relation is PairRelation.COMMUTATIVE:
        return PairDecision.ADVANCE
    if relation is PairRelation.FIRST_CAUSES_SECOND:
        return (
            PairDecision.ADVANCE
            if requested_id == first_id
            else PairDecision.BLOCK
        )
    return (
        PairDecision.ADVANCE
        if requested_id == second_id
        else PairDecision.BLOCK
    )
