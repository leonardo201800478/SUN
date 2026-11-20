#!/usr/bin/env python3
"""Summarize report-only EG phase-minus-three differential sweep results."""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path


def main() -> int:
    """Aggregate each fixture's exact Rust-versus-Nuked candidate result."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    manifest_path = args.input_dir / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != "ym2413-eg-rate-sweep-v1":
        raise ValueError("unexpected EG rate-sweep manifest schema")

    results = []
    level_by_dr: Counter[int] = Counter()
    level_by_ksr: Counter[int] = Counter()
    total_samples = 0
    total_level_mismatches = 0
    total_stage_mismatches = 0
    baseline_level_mismatches = 0
    baseline_stage_mismatches = 0
    reference_replay_failures = []
    timer_replay_failures = []
    total_timer_replay_calls = 0
    total_timer_field_mismatches = 0
    external_phase_level_mismatches = 0
    external_phase_stage_mismatches = 0

    for config in manifest["configurations"]:
        stem = Path(config["fixture"]).stem
        analysis_path = args.input_dir / f"{stem}-analysis.json"
        comparison_path = args.input_dir / f"{stem}-comparison.json"
        timer_path = args.input_dir / f"{stem}-timer-validation.json"
        if (
            not analysis_path.is_file()
            or not comparison_path.is_file()
            or not timer_path.is_file()
        ):
            raise FileNotFoundError(f"missing result for {config['fixture']}")

        analysis = json.loads(analysis_path.read_text(encoding="utf-8"))
        comparison = json.loads(comparison_path.read_text(encoding="utf-8"))
        timer_validation = json.loads(timer_path.read_text(encoding="utf-8"))
        if analysis.get("candidate") != "initial_127_deferred_decay_phase_minus3":
            raise ValueError(f"wrong candidate in {analysis_path.name}")

        sample_count = int(analysis["sample_count"])
        if sample_count != 6418:
            raise ValueError(
                f"{config['fixture']} expected 6,418 selected samples, got {sample_count}"
            )
        replay_ok = (
            analysis.get("reference_internal_update_replay") == "PASS"
            and int(analysis["reference_replay_mismatches"]) == 0
        )
        if not replay_ok:
            reference_replay_failures.append(config["fixture"])

        timer_calls = int(timer_validation["calls"])
        timer_mismatches = int(timer_validation["total_field_mismatches"])
        timer_ok = timer_calls == 30000 and timer_mismatches == 0
        if not timer_ok:
            timer_replay_failures.append(config["fixture"])
        total_timer_replay_calls += timer_calls
        total_timer_field_mismatches += timer_mismatches

        total_samples += sample_count

        baseline_comparison = comparison[
            "candidate_initial_level_127_deferred_attack_unwrapped"
        ]
        candidate_comparison = comparison[
            "candidate_initial_level_127_deferred_decay_phase_minus3"
        ]
        external_phase_comparison = comparison[
            "candidate_initial_level_127_deferred_external_rate_phase"
        ]
        baseline_level = int(baseline_comparison["level_mismatches"])
        baseline_stage = int(baseline_comparison["stage_mismatches"])
        level_mismatches = int(candidate_comparison["level_mismatches"])
        stage_mismatches = int(candidate_comparison["stage_mismatches"])
        baseline_level_mismatches += baseline_level
        baseline_stage_mismatches += baseline_stage
        total_level_mismatches += level_mismatches
        total_stage_mismatches += stage_mismatches
        external_level = int(external_phase_comparison["level_mismatches"])
        external_stage = int(external_phase_comparison["stage_mismatches"])
        external_phase_level_mismatches += external_level
        external_phase_stage_mismatches += external_stage
        level_by_dr[int(config["dr"])] += level_mismatches
        level_by_ksr[int(config["ksr"])] += level_mismatches

        results.append({
            "fixture": config["fixture"],
            "ar": int(config["ar"]),
            "dr": int(config["dr"]),
            "ksr": int(config["ksr"]),
            "sample_count": sample_count,
            "baseline_level_mismatches": baseline_level,
            "baseline_stage_mismatches": baseline_stage,
            "level_mismatches": level_mismatches,
            "stage_mismatches": stage_mismatches,
            "max_absolute_level_error": int(candidate_comparison["max_absolute_level_error"]),
            "reference_replay": "PASS" if replay_ok else "FAIL",
            "timer_replay_calls": timer_calls,
            "timer_replay_field_mismatches": timer_mismatches,
            "timer_replay": "PASS" if timer_ok else "FAIL",
            "first_level_mismatch": candidate_comparison["first_level_mismatch"],
            "first_stage_mismatch": candidate_comparison["first_stage_mismatch"],
            "external_rate_phase_level_mismatches": external_level,
            "external_rate_phase_stage_mismatches": external_stage,
            "external_rate_phase_max_absolute_level_error": int(
                external_phase_comparison["max_absolute_level_error"]
            ),
            "external_rate_phase_first_level_mismatch": external_phase_comparison[
                "first_level_mismatch"
            ],
            "external_rate_phase_first_stage_mismatch": external_phase_comparison[
                "first_stage_mismatch"
            ],
        })

    summary = {
        "schema": "ym2413-eg-rate-sweep-summary-v1",
        "candidate": "initial_127_deferred_decay_phase_minus3",
        "configuration_count": len(results),
        "total_samples": total_samples,
        "baseline_total_level_mismatches": baseline_level_mismatches,
        "baseline_total_stage_mismatches": baseline_stage_mismatches,
        "total_level_mismatches": total_level_mismatches,
        "total_stage_mismatches": total_stage_mismatches,
        "external_rate_phase_total_level_mismatches": external_phase_level_mismatches,
        "external_rate_phase_total_stage_mismatches": external_phase_stage_mismatches,
        "external_rate_phase_all_levels_match": external_phase_level_mismatches == 0,
        "external_rate_phase_all_stages_match": external_phase_stage_mismatches == 0,
        "all_reference_replays_pass": not reference_replay_failures,
        "total_timer_replay_calls": total_timer_replay_calls,
        "total_timer_field_mismatches": total_timer_field_mismatches,
        "all_timer_replays_pass": not timer_replay_failures,
        "timer_replay_failures": timer_replay_failures,
        "all_phase_minus3_levels_match": total_level_mismatches == 0,
        "all_stage_transitions_match": total_stage_mismatches == 0,
        "level_mismatches_by_dr": dict(sorted(level_by_dr.items())),
        "level_mismatches_by_ksr": dict(sorted(level_by_ksr.items())),
        "reference_replay_failures": reference_replay_failures,
        "configurations": results,
    }
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, sort_keys=True))

    if reference_replay_failures or timer_replay_failures:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
