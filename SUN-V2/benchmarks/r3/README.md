# CHRONOS R3 disposable prototypes

This directory is isolated research code for R3. It is not a runtime package,
is not imported from Rust or the desktop host, and is not a candidate production
crate. All experiments are disposable.

## Current sequence

1. P-A — instruction/budget plus catch-up baseline (mandatory).
2. P-D — central discrete-deadline agenda (after P-A equivalence and baseline).
3. P-E — sync-on-demand (after the same fixtures are frozen).
4. P-F — selective hybrid only if a measured functional/cost gap remains.
5. P-B/P-C are conditional and are not in the initial cut.

The exact candidate definitions, workloads, equivalence rules, benchmark protocol,
and gates are in [R3_PROTOTYPE_PLAN.md](../../docs/temporal-core/R3_PROTOTYPE_PLAN.md).

## Oracle boundary

The R2 reference model under `SUN-V2/verification/r2_reference/` is the normative
test oracle. Prototype implementations must use their own data structures and
algorithm paths. The oracle may be invoked outside the timed region to compare
fixtures, traces, outcome classes, no-mutation guarantees and final digests.
Do not import R2 model classes into the timed prototype implementation.

## Result status

P-A smoke benchmark completed on Linux and Windows in CI #10821 (two domains,
four obligations, chunk size eight, one warm-up and three timed samples per
density). Those temporary runner outputs were not retained as artifacts, so this
smoke run is not a reproducible baseline and no comparative throughput/latency
claim is made. A retained full baseline matrix is pending.


## Retained baseline

The first retained P-A advance and deadline lifecycle matrices, environment details, raw artifact IDs/SHA-256, selected p50/p95 observations and remaining limitations are recorded in [P_A_BASELINE_RESULTS.md](./P_A_BASELINE_RESULTS.md). The baseline is tied to exact HEAD `555c3d034c0d0b7f2013e48fad80ae78d75c1106` and CI #10844; it is historical evidence for that prototype revision, not a claim about later revisions.

## P-A first-slice boundary

The P-A suite currently has 44 tests across advance/catch-up and deadline lifecycle. It compares result classes and immutable book/state outputs with the R2 oracle, including 48 deterministic advancement boundary comparisons, watermark frontier, incomplete participant sets with one-sided invariant checks, rational common offsets, deadline publication, C<D/C=D cancellation, admission frontier A(E), cancellation arbitration before W closure, commutative/UNKNOWN ties, dispatch exactly-once, completion and generation replacement. Detailed per-transition trace comparison is still not implemented; no trace-coverage claim is made.
