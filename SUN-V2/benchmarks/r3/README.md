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

No prototype has been benchmarked yet. No throughput, latency, allocation,
memory, or hardware-fidelity result is claimed. The first deliverable is P-A
with reproducible fixtures and a deterministic trace comparison.


## P-A first-slice boundary

The current P-A test harness checks result class, stop coordinate and every participant's common-time progress against R2. It also checks deterministic final-state digest under chunk repartitioning. A detailed per-transition trace is not implemented in this first slice; no trace-coverage claim is made. The current suite contains 12 discriminating tests, including unequal participant progress, zero-length frontier catch-up, UNKNOWN ties, and R2 invariant-violation boundaries.
