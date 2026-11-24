"""Run the R2 reference tests and print pinned reproducibility metadata."""
from __future__ import annotations

import hashlib
import platform
import subprocess
import sys
import unittest
from pathlib import Path

# Test discovery imports local modules; never let that create untracked __pycache__
# files that would make the next reproducibility run fail its clean-tree gate.
sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
REPOSITORY_ROOT = HERE.parents[2]
ORACLE = HERE / "chronos_reference.py"


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


def main() -> int:
    """Print run identity, execute the suite, and return a process status."""
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
    print("---- unittest ----")

    suite = unittest.defaultTestLoader.discover(
        start_dir=str(HERE),
        pattern="test_*.py",
        top_level_dir=str(HERE),
    )
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    print(f"tests_run={result.testsRun}")
    print(f"tests_failures={len(result.failures)}")
    print(f"tests_errors={len(result.errors)}")
    print(f"test_status={'PASS' if result.wasSuccessful() else 'FAIL'}")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
