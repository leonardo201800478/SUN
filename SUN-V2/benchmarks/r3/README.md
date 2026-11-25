# CHRONOS R3 disposable prototypes

This directory is isolated research code for R3. It is not a runtime package,
is not imported from Rust or the desktop host, and is not a candidate production
crate. All experiments are disposable.

## Current sequence

1. P-A — instruction/budget, catch-up, deadline lifecycle and serial causal sync baseline.
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
- `test_p_a_instruction_catchup.py`: 24 R2 differential tests, including 48 deterministic boundary combinations.
- `test_p_a_deadline_lifecycle.py`: 21 R2 differential tests, including cancellation races, causal ties and generation fencing.
- `test_p_a_causal_sync.py`: 13 R2 differential tests, including cross-domain dependencies, cycles, UNKNOWN ties, unrelated-device isolation and applied-order equivalence.
- Total: **58 tests** (24 advance/catch-up, 21 deadline lifecycle, 13 causal sync).

## Validation and benchmark status

CI #10851 passed the 45-test P-A revision on Linux and Windows, R2 on both systems,
both retained advance/deadline matrices, Actionlint, documentation hygiene and the
quality supervisor. W03/W04 causal sync was added after that run and is awaiting its
own CI validation.

The advance matrix has 75 points per OS; the deadline lifecycle matrix has 180 points
per OS. The W03/W04 causal matrix has 30 points per OS. All use five warm-ups and
30 measured repetitions. CPU samples are retained only when empirical thread CPU
clock resolution is at most 1 ms; coarse Windows CPU clocks are explicitly marked
unavailable rather than misreported. tracemalloc is a separate probe outside timed
samples.

The first retained advance/deadline matrices, environment details, raw artifact
IDs/SHA-256 and observations are recorded in
[P_A_BASELINE_RESULTS.md](./P_A_BASELINE_RESULTS.md). W03/W04 results will be appended
after their CI run completes.

## Scope boundary

P-A is not complete until the new W03/W04 differential suite and benchmark matrix pass.
Detailed fixture/trace equivalence still needs consolidation. Transaction
observe/sample/commit and partial-effect ledger (W08), snapshot/restore/replay (W09),
and deeper W06/W07 coverage remain outside this first cut.

P-D/P-E are not started. R4/R5 remain mandatory. No production CHRONOS/V2 code is
authorized before R5.
