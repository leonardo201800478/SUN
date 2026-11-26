"""Run the complete P-A differential suite and emit its canonical trace corpus."""
from __future__ import annotations

import argparse
import importlib
import json
import subprocess
import sys
import unittest
from pathlib import Path

from trace_fixture_support import build_document, reset
from replayable_fixture_corpus import (
    build_replayable_document,
    reset_replayable_corpus,
)


TEST_MODULES = (
    "test_p_a_instruction_catchup",
    "test_p_a_deadline_lifecycle",
    "test_p_a_causal_sync",
    "test_p_a_microsteps",
    "test_p_a_transactions",
    "test_p_a_snapshots",
    "test_trace_fixture_support",
    "test_replayable_fixture_corpus",
    "test_w00_replayable_fixture",
    "test_w01_replayable_fixture",
    "test_w07_replayable_fixture",
    "test_w07_pairwise_matrix",
    "test_w08_replayable_fixture",
    "test_w09_replayable_fixture",
)


def git_head() -> str:
    """Return the checked-out Git revision for trace provenance."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"], check=False,
            capture_output=True, text=True,
        )
    except OSError:
        return "UNKNOWN"
    return result.stdout.strip() if result.returncode == 0 else "UNKNOWN"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output", type=Path, default=Path("p_a_canonical_trace_corpus.json")
    )
    parser.add_argument(
        "--replayable-output",
        type=Path,
        default=Path("p_a_replayable_fixture_corpus.json"),
    )
    args = parser.parse_args()

    reset()
    reset_replayable_corpus()
    loader = unittest.defaultTestLoader
    suite = unittest.TestSuite()
    for module_name in TEST_MODULES:
        module = importlib.import_module(module_name)
        suite.addTests(loader.loadTestsFromModule(module))

    result = unittest.TextTestRunner(verbosity=1).run(suite)
    if not result.wasSuccessful():
        return 1

    document = build_document(
        git_head=git_head(),
        test_count=result.testsRun,
    )
    if document["cut_count"] == 0 or not document["all_equal"]:
        raise AssertionError("canonical trace corpus is empty or divergent")
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps({
        "status": "PASS",
        "schema": document["schema"],
        "tests": document["test_count"],
        "fixtures": document["fixture_count"],
        "cuts": document["cut_count"],
        "differential_cuts": document["differential_cut_count"],
        "property_cuts": document["property_cut_count"],
        "trace_digest": document["trace_digest"],
        "output": str(args.output),
    }, sort_keys=True))

    replayable = build_replayable_document(
        git_head=git_head(),
        test_count=result.testsRun,
    )
    if replayable["fixture_count"] == 0:
        raise AssertionError("replayable fixture corpus is empty")
    args.replayable_output.write_text(
        json.dumps(replayable, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps({
        "status": "PASS",
        "schema": replayable["schema"],
        "tests": replayable["test_count"],
        "fixtures": replayable["fixture_count"],
        "operations": replayable["operation_count"],
        "cuts": replayable["cut_count"],
        "divergent_fixtures": replayable["divergent_fixture_count"],
        "divergent_fixture_ids": replayable["divergent_fixture_ids"],
        "corpus_digest": replayable["corpus_digest"],
        "output": str(args.replayable_output),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
