# CHRONOS R2 reference oracle — Python verification artifact

This directory contains an independent, test-only reference model for R2.

## Run

From this directory:

    python -m unittest -v

No third-party dependencies are required. The model uses exact rational arithmetic and a full scan of obligations. It is intentionally not optimized and must not be imported by the Rust backend or desktop host.

## Current coverage

- R2-T01: monotonicity, seeded monotonic sequences, retrograde-input no-mutation, participant completeness and exact-time validation.
- R2-T02: chunk invariance, including 100 deterministic random partitions.
- R2-T03: exact rational round-trip/composition and 10,000-step conversion sessions across three ratios.
- R2-T04: inclusive safe horizon, endpoint StoppedAt(T), earlier obligations, resume after resolution and multi-domain minimum horizon.

This is the first R2-A slice, not completion of R2. R2-T05–T15 remain NOT RUN until implemented and executed.
