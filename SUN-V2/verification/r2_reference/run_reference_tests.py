"""Run R2 properties, mutation checks and emit a reproducible JSON report."""
from __future__ import annotations

import hashlib
import json
import os
import platform
import random
import re
import subprocess
import sys
import unittest
from pathlib import Path
from typing import Iterator

# Test discovery imports local modules; do not create untracked __pycache__ files.
sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
REPOSITORY_ROOT = HERE.parents[2]
ORACLE = HERE / "chronos_reference.py"
SEED = 20261125


def git_output(*arguments: str) -> str:
    """Run Git against the repository root and return stdout."""
    completed = subprocess.run(
        ["git", "-C", str(REPOSITORY_ROOT), *arguments],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def repository_head() -> str:
    """Return the exact Git HEAD or an explicit marker outside a Git checkout."""
    try:
        return git_output("rev-parse", "HEAD")
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"


def dirty_worktree() -> str:
    """Return porcelain status, including untracked files, or fail closed."""
    try:
        return git_output("status", "--porcelain", "--untracked-files=all")
    except (OSError, subprocess.CalledProcessError) as error:
        raise RuntimeError("cannot verify repository worktree state") from error


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of a file's exact bytes."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def flatten_suite(suite: unittest.TestSuite) -> Iterator[unittest.TestCase]:
    """Yield test cases from nested unittest suites in discovery order."""
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from flatten_suite(item)
        else:
            yield item


class RecordingTestResult(unittest.TextTestResult):
    """Record the exact test trace and the first failing property."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.trace: list[str] = []
        self.first_divergence: dict[str, object] | None = None

    def startTest(self, test):
        self.trace.append(test.id())
        super().startTest(test)

    def _record_divergence(self, test, err, outcome: str) -> None:
        if self.first_divergence is not None:
            return
        formatted = self._exc_info_to_string(err, test)
        reproduction = None
        match = re.search(r"R2_REPRO=(\\{.*\\})", formatted)
        if match:
            try:
                reproduction = json.loads(match.group(1))
            except json.JSONDecodeError:
                reproduction = {"parse_error": "R2_REPRO marker was not valid JSON"}
        self.first_divergence = {
            "test_id": test.id(),
            "outcome": outcome,
            "traceback": formatted,
            "reproduction": reproduction,
        }

    def addFailure(self, test, err):
        self._record_divergence(test, err, "failure")
        super().addFailure(test, err)

    def addError(self, test, err):
        self._record_divergence(test, err, "error")
        super().addError(test, err)


def run_mutation_checks() -> dict[str, object]:
    """Run the two required deliberate deadline-boundary mutants in temp copies."""
    completed = subprocess.run(
        [sys.executable, str(HERE / "run_mutation_tests.py")],
        cwd=HERE,
        text=True,
        capture_output=True,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1", "PYTHONHASHSEED": "0"},
        check=False,
    )
    lines = [line for line in completed.stdout.splitlines() if line.strip().startswith("{")]
    if completed.returncode != 0 or not lines:
        return {
            "status": "FAIL",
            "exit_code": completed.returncode,
            "stdout": completed.stdout[-4000:],
            "stderr": completed.stderr[-4000:],
        }
    try:
        return json.loads(lines[-1])
    except json.JSONDecodeError as error:
        return {
            "status": "FAIL",
            "reason": f"invalid mutation JSON: {error}",
            "stdout": completed.stdout[-4000:],
        }


def main() -> int:
    """Run pinned tests and mutation checks, then emit JSON and human metadata."""
    sys.path.insert(0, str(HERE))
    head = repository_head()
    if head == "UNAVAILABLE":
        print("test_status=BLOCKED")
        print("reason=Git HEAD unavailable; reproducibility cannot be pinned")
        return 2

    try:
        status = dirty_worktree()
    except RuntimeError as error:
        print("test_status=BLOCKED")
        print(f"reason={error}")
        return 2
    if status:
        print("test_status=BLOCKED")
        print("reason=dirty worktree; commit or discard all changes before running")
        print(status)
        return 2

    model_files = sorted(
        path for path in HERE.glob("*.py")
        if path.name != Path(__file__).name and not path.name.startswith("test_")
    )
    test_files = sorted(HERE.glob("test_*.py"))
    print(f"repository_head={head}")
    print("worktree_status=CLEAN")
    print(f"oracle_path={ORACLE.relative_to(REPOSITORY_ROOT)}")
    print(f"oracle_sha256={sha256_file(ORACLE)}")
    for model_file in model_files:
        print(f"model_sha256[{model_file.name}]={sha256_file(model_file)}")
    print(f"runner_sha256={sha256_file(Path(__file__).resolve())}")
    for test_file in test_files:
        print(f"test_sha256[{test_file.name}]={sha256_file(test_file)}")
    print(f"python_version={platform.python_version()}")
    print(f"platform={platform.platform()}")
    print("profile=SMS-NTSC (abstract reference-model profile)")
    print(f"seed={SEED}")
    print("seed_policy=deterministic test execution order; no randomized model inputs")
    print("---- unittest ----")

    discovered = unittest.defaultTestLoader.discover(
        start_dir=str(HERE),
        pattern="test_*.py",
        top_level_dir=str(HERE),
    )
    cases = list(flatten_suite(discovered))
    random.Random(SEED).shuffle(cases)
    suite = unittest.TestSuite(cases)
    runner = unittest.TextTestRunner(verbosity=2, resultclass=RecordingTestResult)
    result = runner.run(suite)

    mutation_report = run_mutation_checks()
    try:
        final_worktree = dirty_worktree()
        worktree_after = "CLEAN" if not final_worktree else "DIRTY"
    except RuntimeError as error:
        final_worktree = str(error)
        worktree_after = "UNAVAILABLE"

    unit_ok = result.wasSuccessful()
    mutation_ok = mutation_report.get("status") == "PASS"
    overall_ok = unit_ok and mutation_ok and worktree_after == "CLEAN"
    first_divergence = result.first_divergence
    minimal_state = None
    if first_divergence is not None:
        reproduction = first_divergence.get("reproduction")
        if isinstance(reproduction, dict) and "minimal_state" in reproduction:
            minimal_state = reproduction["minimal_state"]
        else:
            minimal_state = {
                "kind": "hand-authored deterministic test fixture",
                "test_id": first_divergence["test_id"],
                "source": "the named test function constructs the reproducer",
                "reduction_status": "NOT_APPLICABLE_TO_FIXED_CASE",
            }

    report = {
        "schema_version": 1,
        "suite": "SUN CHRONOS R2 independent reference model",
        "repository_head": head,
        "worktree_before": "CLEAN",
        "worktree_after": worktree_after,
        "oracle_sha256": sha256_file(ORACLE),
        "runner_sha256": sha256_file(Path(__file__).resolve()),
        "python_version": platform.python_version(),
        "platform": platform.platform(),
        "profile": "SMS-NTSC (abstract reference-model profile)",
        "seed": SEED,
        "seed_policy": "seed deterministically shuffles test execution order; model inputs are fixed/exhaustive",
        "tests_run": result.testsRun,
        "tests_failures": len(result.failures),
        "tests_errors": len(result.errors),
        "trace": result.trace,
        "first_divergence": first_divergence,
        "minimal_state": minimal_state,
        "mutation_checks": mutation_report,
        "test_status": "PASS" if unit_ok else "FAIL",
        "mutation_status": "PASS" if mutation_ok else "FAIL",
        "status": "PASS" if overall_ok else "FAIL",
    }
    encoded = json.dumps(report, sort_keys=True, indent=2)
    result_path = os.environ.get("R2_RESULT_PATH")
    if result_path:
        destination = Path(result_path)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(encoded + "\n", encoding="utf-8", newline="\n")
        print(f"json_report_path={destination}")
    print("r2_result_json=" + json.dumps(report, sort_keys=True, separators=(",", ":")))
    print(f"tests_run={result.testsRun}")
    print(f"tests_failures={len(result.failures)}")
    print(f"tests_errors={len(result.errors)}")
    print(f"mutation_status={'PASS' if mutation_ok else 'FAIL'}")
    print(f"worktree_after={worktree_after}")
    print(f"test_status={'PASS' if overall_ok else 'FAIL'}")
    return 0 if overall_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
