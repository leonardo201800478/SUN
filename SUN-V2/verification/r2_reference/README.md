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

For a plain unittest invocation (without the pinned evidence header):

    python -m unittest -v

No third-party dependencies are required. The model uses exact rational arithmetic and a full scan of obligations. It is intentionally not optimized and must not be imported by the Rust backend or desktop host.

## Current candidate coverage

The current candidate defines 24 tests:
- R2-T01: monotonicity, retrograde-input no-mutation, participant completeness, exact-time validation, invalid clock conversion and obligation-owner validation.
- R2-T02: chunk invariance, including 100 deterministic random partitions, safe-horizon boundary stops, pending-obligation stops and continuation after resolution.
- R2-T03: exact rational round-trip/composition and 10,000-step conversion sessions across three ratios.
- R2-T04: inclusive safe horizon, endpoint StoppedAt(T), earlier obligations, resume after resolution and multi-domain minimum horizon.

**Execution status of this candidate: NOT RUN.** Earlier 19-test results were not pinned to a repository SHA and oracle hash, so they remain historical, unbound local evidence and are not PASS evidence for the current candidate.

R2-T05–T15 remain NOT RUN until implemented and executed. This is the first R2-A slice, not completion of R2.
