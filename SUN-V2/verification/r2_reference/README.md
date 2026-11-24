# CHRONOS R2 reference oracle — Python verification artifact

This directory contains an independent, test-only reference model for R2.

## Reproducible run

From this directory, run:

    python run_reference_tests.py

The runner records:
- Git repository HEAD SHA;
- SHA-256 of `chronos_reference.py`;
- Python version and platform;
- test-suite result.

For a plain unittest invocation (without the pinned evidence header; it runs the same test cases and does not depend on runner-specific process state):

    python -m unittest -v

No third-party dependencies are required. The model uses exact rational arithmetic and a full scan of obligations. It is intentionally not optimized and must not be imported by the Rust backend or desktop host.

## Current candidate coverage

The current candidate defines 27 tests:
- R2-T01: monotonicity, retrograde-input no-mutation, participant completeness, structural-validation precedence over `Blocked`, exact-time validation, invalid clock conversion/watermark, obligation identity/owner validation .
- R2-T02: chunk invariance, including 100 deterministic random partitions, safe-horizon boundary stops, pending-obligation stops and continuation after resolution.
- R2-T03: exact rational round-trip/composition and 10,000-step conversion sessions across three ratios.
- R2-T04: inclusive safe horizon, endpoint StoppedAt(T), earlier obligations, resume after resolution and multi-domain minimum horizon.

**Earlier executed candidate:** 24 tests PASS in CI #10619 on Git SHA `a0c0782ff4d8e9886144b20bde36e42bcc6f1eff`, with oracle SHA-256 `160489b753575f0e7fcb12892eb58f7199a1dc5828c5bb4f13c266f557ba001a`, Python 3.14.4, Linux x86_64. This is historical evidence for that exact candidate only.\n\n**Current executed candidate:** 25 tests PASS in CI #10623 on Git SHA `a8cef1bcc39e284729a7a55fc16e27deb9fb4b22`, with clean worktree. Oracle SHA-256 `de802612b198996cd6e888aa9fa8840596fa42ab8319aef0611b3841052a30b3`; runner SHA-256 `bb4798b3cc073b785a8c71e5796a24a595be0cc2ebf32501978bd5d23494bc77`; test module SHA-256 `b5a6ad833240b5f622ac313d98595bd17a079a51289dedf71c7d0f7bdd1f3d1e`. Python 3.14.4, Linux x86_64. The runner rejects dirty worktrees and records hashes for the oracle, runner and every discovered test module. Earlier 19-test results remain historical, unbound local evidence.

R2-T05–T15 remain NOT RUN until implemented and executed. This is the first R2-A slice, not completion of R2.


The runner sets `sys.dont_write_bytecode` before test discovery. CI invokes it twice from the same checkout and checks `git status --porcelain --untracked-files=all` between invocations; the second run must still begin with a clean worktree. The test suite itself does not assert this runner-specific process setting, so both the dedicated reproducibility runner and the documented plain `python -m unittest -v` invocation remain valid.
