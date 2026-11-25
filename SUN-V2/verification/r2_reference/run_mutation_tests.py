"""Deliberate mutation checks for critical R2 temporal boundaries."""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


HERE = Path(__file__).resolve().parent
SOURCE = HERE / "deadline_reference.py"


def sha256(data: bytes) -> str:
    """Return a SHA-256 digest for exact source bytes."""
    return hashlib.sha256(data).hexdigest()


def run_mutant(
    *,
    name: str,
    old: str,
    new: str,
    target_test: str,
) -> dict[str, object]:
    """Run one targeted regression against an isolated, deliberately mutated oracle."""
    original = SOURCE.read_text(encoding="utf-8")
    occurrences = original.count(old)
    if occurrences != 1:
        return {
            "name": name,
            "status": "BLOCKED",
            "reason": f"expected one mutation anchor, found {occurrences}",
            "target_test": target_test,
        }
    mutated = original.replace(old, new, 1)
    environment = os.environ.copy()
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    environment["PYTHONHASHSEED"] = "0"
    with tempfile.TemporaryDirectory(prefix="sun-r2-mutant-") as temporary:
        destination = Path(temporary)
        for source in HERE.glob("*.py"):
            shutil.copy2(source, destination / source.name)
        mutant_path = destination / SOURCE.name
        mutant_path.write_text(mutated, encoding="utf-8", newline="\n")
        completed = subprocess.run(
            [sys.executable, "-m", "unittest", "-v", target_test],
            cwd=destination,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
    output = completed.stdout + completed.stderr
    killed = completed.returncode != 0
    diagnostic_lines = [
        line.strip()
        for line in output.splitlines()
        if "FAIL:" in line or "ERROR:" in line or "AssertionError:" in line
    ]
    return {
        "name": name,
        "status": "KILLED" if killed else "SURVIVED",
        "target_test": target_test,
        "mutant_sha256": sha256(mutated.encode("utf-8")),
        "exit_code": completed.returncode,
        "first_divergence": diagnostic_lines[:3],
        "trace": [target_test],
        "minimal_state": {
            "kind": "hand-authored boundary fixture",
            "test_id": target_test,
            "reduction": "the named test is the minimal deterministic fixture for this mutation",
        },
    }


def main() -> int:
    """Execute all required mutants and emit a machine-readable report."""
    mutants = (
        {
            "name": "C_equals_D_is_not_strictly_before_deadline",
            "old": "if request.at_common < deadline.at_common:",
            "new": "if request.at_common <= deadline.at_common:",
            "target_test": (
                "test_deadlines.R2BCancellationRaceTests."
                "test_preconstructed_pending_cancellation_must_precede_deadline"
            ),
        },
        {
            "name": "C_equals_D_unknown_is_not_late_input",
            "old": (
                'book, DeadlineCode.INDETERMINATE,\n'
                '                "C=D requires an explicit profile causal-order or microstep relation",'
            ),
            "new": (
                'book, DeadlineCode.LATE_INPUT,\n'
                '                "C=D requires an explicit profile causal-order or microstep relation",'
            ),
            "target_test": (
                "test_deadlines.R2BCancellationRaceTests."
                "test_cancel_at_deadline_without_causal_relation_is_indeterminate"
            ),
        },
    )
    results = [run_mutant(**mutant) for mutant in mutants]
    report = {
        "schema_version": 1,
        "suite": "SUN CHRONOS R2 deliberate boundary mutation checks",
        "mutants": results,
        "status": (
            "PASS"
            if all(item["status"] == "KILLED" for item in results)
            else "FAIL"
        ),
    }
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
