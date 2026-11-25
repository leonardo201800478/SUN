# CHRONOS R3 disposable prototypes

This directory is isolated research code for R3. It is not a runtime package,
is not imported from Rust or the desktop host, and is not a candidate production
crate. All experiments are disposable.

## Current sequence

1. P-A — instruction/budget, catch-up, deadline lifecycle, serial causal sync, simultaneous UNKNOWN ordering (W06), finite microstep chains (W07) and transaction lifecycle/partial effects (W08).
2. P-D — central discrete-deadline agenda (after P-A equivalence and baseline).
3. P-E — sync-on-demand candidate (after the same fixtures are frozen).
4. P-F — selective hybrid only if a measured functional/cost gap remains.
5. P-B/P-C are conditional and are not in the initial cut.

The exact candidate definitions, workloads, equivalence rules, benchmark protocol,
and gates are in [R3_PROTOTYPE_PLAN.md](../../docs/temporal-core/R3_PROTOTYPE_PLAN.md).

## Oracle boundary

The R2 reference model under `SUN-V2/verification/r2_reference/` is the normative
test oracle. Prototype implementations use their own data structures and algorithm
paths. The oracle may be invoked outside timed regions to compare fixtures,
outcome classes, applied transition order, no-mutation guarantees and final state.
Do not import R2 model classes into timed prototype implementations.

## Current P-A inventory

- `p_a_instruction_catchup.py`: bounded advance, chunk catch-up, expected participants, common watermark and rational common offsets.
- `p_a_deadline_lifecycle.py`: publish/cancel/arbitrate/close W/claim/dispatch/complete/replace, with linear scans and temporary timestamp grouping.
- `p_a_causal_sync.py`: serial observation-driven synchronization and DFS dependency resolution for W03/W04; no persistent agenda.
- `p_a_microsteps.py`: immutable same-instant obligation cycle with child admission at the next microstep and finite limit.
- `p_a_transactions.py`: immutable transaction lifecycle with sample/commit ledgers, wait ownership, distinct TW budgets, failure and explicit recovery.
- `bench_p_a_ordering.py` / `bench_p_a_microsteps.py` / `bench_p_a_transactions.py`: W06 group-order, W07 chain-depth and W08 transaction matrices.
- `test_p_a_instruction_catchup.py`: 24 R2 differential tests, including 48 deterministic boundary combinations.
- `test_p_a_deadline_lifecycle.py`: 22 R2 differential tests, including cancellation races, causal ties and generation fencing.
- `test_p_a_causal_sync.py`: 14 R2 differential tests, including cross-domain dependencies, cycles, UNKNOWN ties, unrelated-device isolation and applied-order equivalence.
- `test_p_a_microsteps.py`: 7 R2 differential tests for child admission, no-op repetition, finite exhaustion, commutative permutations, UNKNOWN ties and depths 1–32.
- `test_p_a_transactions.py`: 18 R2 differential tests for idempotency, sample/commit exactly-once, wait/resume, TW budgets, cancellation, partial failure and explicit recovery.
- `test_p_a_deadline_lifecycle.py`: 4 additional W06 group-size/permutation tests for 2/4/8/16 simultaneous deadlines.
- Total: **89 tests** (24 advance/catch-up, 26 deadline lifecycle, 14 causal sync, 7 microsteps, 18 transactions).

## Validation and benchmark status

CI #10892 completed successfully on Linux and Windows for P-A HEAD
`3b54226c472489092c1a2a3b9d29685cc6dac3df`. The P-A differential suite passed
89/89 tests on each OS (24 advance/catch-up, 26 deadline lifecycle, 14 causal sync,
7 microsteps, 18 transactions). The R2 independent reference suite passed 256/256
tests on each OS, with zero failures/errors, no first divergence, mutation checks
PASS and clean worktrees. Documentation hygiene, Actionlint 1.7.12 and the quality
supervisor passed. Fourteen unrelated hardware/runtime/desktop jobs were SKIPPED.

Six matrices produced 331 points per OS: advance/catch-up (75), deadline lifecycle
(180), W03/W04 causal synchronization (30), W06 simultaneous ordering (24), W07
microstep chains (12) and W08 transaction lifecycle (10). All use five warm-ups
and 30 measured repetitions. Independent inspection confirms identical parameter
sets and output digests across Linux and Windows for every corresponding point in
all six matrices; advance counters, deadline result/claim state, causal applied
counts, W06 claim counts, W07 resolved counts and W08 transaction/partial-effect
ledgers also match.

Linux thread CPU-clock resolutions: 1,609 ns (advance), 1,629 ns (deadlines),
1,786 ns (causal), 1,735 ns (W06), 22,642 ns (W07) and 1,766 ns (W08). Windows
resolution was 15,625,000 ns for all six matrices; CPU-time samples are explicitly
`SKIPPED_COARSE_RESOLUTION`, while wall-time samples remain available.
`tracemalloc` is a separate probe outside timed samples.

The CI #10892 environment details, 14 raw artifact IDs, verified SHA-256 digests,
cross-platform output-equivalence checks and limitations are recorded in
[P_A_BASELINE_RESULTS.md](./P_A_BASELINE_RESULTS.md).

## Scope boundary

W00–W08 now have differential tests and synthetic benchmark matrices on Linux/Windows. P-A is still not complete: detailed canonical fixture/trace equivalence needs consolidation, and snapshot/restore/replay (W09) remains unimplemented.

P-D/P-E are not started. R4/R5 remain mandatory. No production CHRONOS/V2 code is
authorized before R5.
