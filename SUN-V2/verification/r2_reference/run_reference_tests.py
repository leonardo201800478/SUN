"""Run the R2 reference tests and print pinned reproducibility metadata."""
from __future__ import annotations

import hashlib
import platform
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPOSITORY_ROOT = HERE.parents[2]
ORACLE = HERE / "chronos_reference.py"


def repository_head() -> str:
    """Return the exact Git HEAD or an explicit marker outside a Git checkout."""
    try:
        completed = subprocess.run(
            ["git", "-C", str(REPOSITORY_ROOT), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return "UNAVAILABLE"
    return completed.stdout.strip()


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
    print(f"repository_head={repository_head()}")
    print(f"oracle_path={ORACLE.relative_to(REPOSITORY_ROOT)}")
    print(f"oracle_sha256={sha256_file(ORACLE)}")
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
