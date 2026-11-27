# CHRONOS R3 disposable prototypes

This directory is isolated research code for R3. It is not a runtime package,
is not imported from Rust or the desktop host, and is not a candidate production
crate. All experiments are disposable.

## Current sequence

1. P-A — instruction/budget, catch-up, deadline lifecycle, serial causal sync, simultaneous UNKNOWN ordering (W06), finite microstep chains (W07), transaction lifecycle/partial effects (W08) and canonical snapshot/restore/replay (W09).
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
- `p_a_snapshots.py`: canonical JSON snapshot bytes, SHA-256 verification, atomic restore and sequence-ordered deterministic replay.
- `bench_p_a_ordering.py` / `bench_p_a_microsteps.py` / `bench_p_a_transactions.py` / `bench_p_a_snapshots.py`: W06 group-order, W07 chain-depth, W08 transaction and W09 snapshot/replay matrices.
- `test_p_a_instruction_catchup.py`: 24 R2 differential tests, including 48 deterministic boundary combinations.
- `test_p_a_deadline_lifecycle.py`: 31 tests, including cancellation races, W06 phase-basis comparability, no-mutation negatives and explicit R2 oracle-gap fixtures.
- `test_p_a_causal_sync.py`: 14 R2 differential tests, including cross-domain dependencies, cycles, UNKNOWN ties, unrelated-device isolation and applied-order equivalence.
- `test_p_a_microsteps.py`: 8 microstep tests, including the W07 lower-level-pending barrier and explicit R2 oracle-gap fixture; the same module also contains 3 shared benchmark-statistics policy tests.
- `test_p_a_transactions.py`: 18 R2 differential tests for idempotency, sample/commit exactly-once, wait/resume, TW budgets, cancellation, partial failure and explicit recovery.
- `test_p_a_snapshots.py`: 27 tests for R2 snapshot equivalence, ambiguous acknowledge cuts, atomic restore, canonical digest and deterministic replay properties.
- `test_p_a_deadline_lifecycle.py`: W06 group-size/permutation coverage for 2/4/8/16 simultaneous deadlines, plus phase-basis discriminants.
- Total: **125 tests** (24 advance/catch-up, 31 deadline lifecycle, 14 causal sync, 8 microstep tests, 18 transactions, 27 snapshot/replay and 3 benchmark-statistics tests).

## Historical validation and benchmark status — CI #10902

CI #10902 completed successfully on Linux and Windows for P-A HEAD
`81cb1780a61dd5af80c52582d379497378365109`. The P-A differential suite passed
116/116 tests on each OS (24 advance/catch-up, 26 deadline lifecycle, 14 causal
sync, 7 microsteps, 18 transactions, 27 snapshot/replay). The R2 independent
reference suite passed 256/256 tests on each OS, with zero failures/errors, no
first divergence, mutation checks PASS and clean worktrees. Documentation hygiene,
Actionlint 1.7.12 and the quality supervisor passed. Fourteen unrelated
hardware/runtime/desktop jobs were SKIPPED.

Seven matrices produced 351 points per OS: advance/catch-up (75), deadline lifecycle
(180), W03/W04 causal synchronization (30), W06 simultaneous ordering (24), W07
microstep chains (12), W08 transaction lifecycle (10) and W09 snapshot/restore/
replay (20). All use five warm-ups and 30 measured repetitions. Independent
inspection confirms identical parameter sets and output digests across Linux and
Windows for every corresponding point in all seven matrices; advance counters,
deadline result/claim state, causal applied counts, W06 claim counts, W07 resolved
counts, W08 transaction/partial-effect ledgers and W09 replay digests/state counts
also match.

Linux thread CPU-clock resolutions: 3,006 ns (advance), 2,495 ns (deadlines),
2,454 ns (causal), 2,385 ns (W06), 2,534 ns (W07), 2,184 ns (W08) and 2,364 ns
(W09). Windows resolution was 15,625,000 ns for all seven matrices; CPU-time samples
are explicitly `SKIPPED_COARSE_RESOLUTION`, while wall-time samples remain
available. `tracemalloc` is a separate probe outside timed samples.

The CI #10902 environment details, 16 raw artifact IDs, verified SHA-256 digests,
cross-platform output-equivalence checks and limitations are recorded in
[P_A_BASELINE_RESULTS.md](./P_A_BASELINE_RESULTS.md).

## Corrective validation — CI #10931

**Executed HEAD:** `21480075857dc2ef6bbe951ba0f894ce630f9df7`

- P-A R3: **125/125 tests PASS on Linux and 125/125 PASS on Windows**.
- R2 independent reference: **256/256 tests PASS on Linux and 256/256 PASS on Windows**.
- All seven matrices completed on both systems: 351 points per OS (75 advance, 180 deadlines, 30 causal, 24 W06, 12 W07, 10 W08, 20 W09).
- Canonical corpus: 112 fixtures, 1,475 cuts, 1,468 differential cuts and 7 property cuts. Linux/Windows JSONs are byte-for-byte equal with digest `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.
- All 18 R2/R3 ZIP artifacts were downloaded, ZIP-tested and SHA-256 verified. Every benchmark point contains 30 wall samples; Linux also retains 30 CPU samples per point. Windows CPU samples are null because the clock resolution is too coarse.
- Quantiles use the shared Hyndman-Fan Type 7 estimator; maximum is calculated directly from raw samples. All seven drivers use the same 10,000 ns maximum accepted CPU-clock resolution.
- CI #10931 overall was **FAIL** because documentation patch hygiene detected trailing whitespace in the newly added oracle-gap proposal; the dependent quality supervisor also failed. The R3/R2 test jobs, all benchmark matrices and Actionlint passed. The trailing whitespace was removed. Final CI #10936 passed on HEAD `34f4f52865b429269ebad05a7857ac4a2e3d5ce5`, including documentation hygiene and the dependent quality supervisor.

Detailed metrics, environments, artifact IDs, hashes and limitations are in [P_A_BASELINE_RESULTS.md](./P_A_BASELINE_RESULTS.md).

## Scope boundary

W00–W09 now have tests and synthetic benchmark matrices on Linux/Windows. The declared canonical fixture/trace corpus is fully equal across platforms (112 fixtures, 1,475 cuts). P-A is still not fully closed: the W06/W07 R2 oracle-gap proposal and independent adversarial review remain pending.

P-D/P-E are not started. R4/R5 remain mandatory. No production CHRONOS/V2 code is
authorized before R5.

- `w07_pairwise_cycle.py` / `test_w07_pairwise_cycle.py`: isolated W07 experiment with explicit pairwise causal/commutative relations; missing relations remain UNKNOWN. Six discriminating tests; this is not an integration into the current P-A microstep engine.

## W07 pairwise-cycle experiment — CI #11056

The isolated candidate was validated on Linux and Windows as part of the 175-test R3 suite. It tests B→C, C→B, proven commutativity and UNKNOWN, including no-mutation behavior for indeterminate pairs. A microstep number does not establish a pair relation.

CI #11056 passed on HEAD `e562a363e171ec017584b1eb8052c0759d0135d5`: R3 Linux/Windows, R2 Linux/Windows, all W00–W09 matrices and byte-identical canonical/replayable corpus artifacts passed. Rust workspace regression and unrelated hardware/runtime jobs were SKIPPED.

This experiment does not prove any physical B/C relation, does not alter `p_a_microsteps.py` or R2, and does not close W07. For the original case without a declared/evidenced B/C relation, the result remains `IndeterminateCausalOrder`. P-A closeout and P-D/P-E remain blocked; R4/R5 are pending.
