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
- `p_a_snapshots.py`: canonical JSON snapshot bytes, SHA-256 verification, atomic restore, `(identity, generation)` deadline history and sequence-ordered replay. Replay checks every event against the snapshot admission frontier `max(W, P_d for all snapshot domains)` once for the accepted batch; events at/behind it return `LateInput` without mutation.
- `bench_p_a_ordering.py` / `bench_p_a_microsteps.py` / `bench_p_a_transactions.py` / `bench_p_a_snapshots.py`: W06 group-order, W07 chain-depth, W08 transaction and W09 snapshot/replay matrices.
- `test_p_a_instruction_catchup.py`: 24 R2 differential tests, including 48 deterministic boundary combinations.
- `test_p_a_deadline_lifecycle.py`: 34 tests, including cancellation races, C=D phase-basis comparability, cross-microstep watermark ambiguity, no-mutation negatives and explicit R2 oracle-gap fixtures.
- `test_p_a_causal_sync.py`: 14 R2 differential tests, including cross-domain dependencies, cycles, UNKNOWN ties, unrelated-device isolation and applied-order equivalence.
- `test_p_a_microsteps.py`: 8 microstep tests, including the W07 lower-level-pending barrier and explicit R2 oracle-gap fixture; the same module also contains 3 shared benchmark-statistics policy tests.
- `test_p_a_transactions.py`: 18 R2 differential tests for idempotency, sample/commit exactly-once, wait/resume, TW budgets, cancellation, partial failure and explicit recovery.
- `test_p_a_snapshots.py`: 30 tests for R2 snapshot equivalence, multi-generation deadline history, ambiguous acknowledge cuts, atomic restore, canonical byte encoding, global replay admission and deterministic replay properties.
- `test_p_a_deadline_lifecycle.py`: W06 group-size/permutation coverage for 2/4/8/16 simultaneous deadlines, plus phase-basis discriminants.
- **131 testes** formam o subconjunto dos módulos de núcleo enumerados acima (24 advance/catch-up, 34 deadline lifecycle, 14 causal sync, 8 microstep tests, 18 transactions, 30 snapshot/replay e 3 benchmark-statistics). Não é a suíte R3 completa: o runner completo no HEAD `6e99758af2d11d3bbe398240b294d7e28efc586a` executou **197 testes**; o emissor de corpus executou 176 e o módulo isolado W07 pairwise acrescenta 21 testes fora do emissor.

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

- `w07_pairwise_cycle.py` / `test_w07_pairwise_cycle.py`: isolated W07 relation-adjudication experiment with explicit pairwise causal/commutative relations; missing relations remain UNKNOWN. It has a finite `max_microsteps=32` (maximum level inclusive): the boundary is accepted and an emitted child above the limit returns `ResourceExhausted` without mutation. The isolated module contains 21 tests and is not integrated into the P-A microstep engine or evidence of physical B/C order.

## W07 pairwise-cycle experiment — CI #11056

The isolated candidate was validated on Linux and Windows as part of the 175-test R3 suite. It tests B→C, C→B, proven commutativity and UNKNOWN, including no-mutation behavior for indeterminate pairs. A microstep number does not establish a pair relation.

CI #11056 passed on HEAD `e562a363e171ec017584b1eb8052c0759d0135d5`: R3 Linux/Windows, R2 Linux/Windows, all W00–W09 matrices and byte-identical canonical/replayable corpus artifacts passed. Rust workspace regression and unrelated hardware/runtime jobs were SKIPPED.

This experiment does not prove any physical B/C relation, does not alter `p_a_microsteps.py` or R2, and does not close W07. For the original case without a declared/evidenced B/C relation, the result remains `IndeterminateCausalOrder`. P-A closeout and P-D/P-E remain blocked; R4/R5 are pending.


## W07 pairwise-cycle hardening — 27/11/2026

A code review found that the initial candidate accepted undeclared relation endpoints and permitted a causal relation that contradicted the declared microstep order. The disposable model now carries an explicit `future` obligation registry. Relation endpoints must resolve to pending/resolved/future identities; emitted obligations must match their exact prior declaration; duplicate/conflicting future identities are rejected. A causal relation inconsistent with the assigned microstep order returns `InvalidInput` without mutation.

The C→B test was corrected: C may precede B only when its declared microstep is also earlier. A contradictory assignment (C at a later microstep causing B at an earlier one) is rejected. These are synthetic input contracts, not evidence that a real SMS causal relation exists.

CI #11057 passed on the previous HEAD `0792315e6276d3912ecb70a72f669c416e721002` (175 R3 tests on Linux/Windows, R2, matrices, byte-identical corpora and documentation checks). It predates this hardening. The new commits require their own CI; do not count CI #11057 as validation of the latest code.


### W07 microstep-sequence constraints

The pairwise-cycle candidate now also preserves the declared microstep sequence in accordance with R1 §§16.6.2–16.6.3:
- A later microstep cannot overtake an earlier pending obligation. If their relation is UNKNOWN, the result is `IndeterminateCausalOrder`; if commutative, the earlier microstep resolves first. Commutative obligations in the same microstep may be processed in either order.
- A causal relation that places a later microstep before an earlier one is rejected as an invalid descriptor.
- An emitted child must be predeclared in `future`, match its identity/owner/microstep exactly, and use a strictly later microstep than its emitter; same/earlier microstep emission is rejected without mutation.

At the time of CI #11067, the isolated `test_w07_pairwise_cycle.py` had 13 tests. The later structural-negative expansion is recorded below and supersedes that count.


### CI #11067 — W07 pairwise-cycle hardening validated

Validated HEAD: `926812ba7a4d442dbadb36df83791d4028a80702`; [run 46034036821](https://github.com/leonardo201800478/SUN/actions/runs/46034036821) **SUCCESS**.

- R3: 182/182 tests PASS on Linux and Windows; all 13 W07 pairwise-cycle tests pass on both.
- R2 independent Linux and R2 Python Windows: PASS.
- W00–W09 synthetic benchmark matrices: PASS on both OS.
- Canonical and replayable corpus generation and byte-identical Linux/Windows comparison: PASS.
- Canonical corpus: 112 fixtures / 1,475 cuts, digest `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.
- Replayable corpus: 24 fixtures / 131 operations-cuts, two preserved divergences W06/W07, digest `07551ec877a58597180470045ad24bd51444124d469b2dc2efdcc4e3d9dcba7e`.
- Documentation hygiene, workflow validation and Rust quality supervisor: PASS. Rust workspace regression and unrelated hardware/runtime jobs: SKIPPED.

The corpus emitter runs 169 tests; the 13 pairwise-cycle tests are separate and are not yet represented as a replayable W07 fixture. CI proves the synthetic tests and artifact equivalence, not a physical B/C relation, W06 phase evidence, full W00–W09 replayable coverage, P-A closeout or architecture approval. W07 remains BLOCKED for the physical profile; P-D/P-E remain NOT RUN.


## W06 paired positive/negative replayable controls — CI #11072

Validated HEAD: `afbdf337da80243d05202163fcc7d8412dfe0db8`; [CI #11072 / run 46060833047](https://github.com/leonardo201800478/SUN/actions/runs/46060833047) **SUCCESS**.

- R3 disposable tests: **183/183 PASS** on Linux and Windows.
- Corpus emitter/replay suite: **170 tests PASS** on both.
- Canonical corpus: 112 fixtures / 1,475 cuts; digest `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.
- Replayable corpus: **25 fixtures / 134 operations-cuts**, two divergences preserved (W06 negative control and W07); digest `2dcf09eeaf6ffe03e6bcf1cfda087aeedeb865d351b6ac081a0dc12ff28bc28f`.
- Canonical and replayable JSON artifacts are byte-identical between Linux and Windows.
- R2 independent Linux and R2 Python Windows: PASS; W00–W09 matrices PASS; workflow validation, documentation hygiene and Rust quality supervisor PASS.
- Unrelated hardware/runtime/desktop/audio/video/Z80/Rust workspace jobs: SKIPPED.

W06 now has two replayable cases: distinct synthetic phase bases with the same event evidence label preserve the P-A/R2 divergence; a shared synthetic basis with distinct per-event evidence labels is a positive control where both models accept and agree. This confirms the fixture's discrimination around basis comparability, not physical SMS timing or evidence authenticity.

Windows CPU-time fields with coarse timer resolution are marked `SKIPPED_COARSE_RESOLUTION`; matrix correctness still passed, but those timings must not be used as cross-platform performance evidence. All fixtures remain synthetic. W06 physical evidence remains UNKNOWN; W07 physical B/C remains BLOCKED; P-A is not closed; P-D/P-E NOT RUN; R4/R5 pending.


## W07 structural negative matrix — CI #11079

Validated HEAD: `d36c0966a6a9f5abf94872c769c62b29453e8517`; [CI #11079 / run 46074483099](https://github.com/leonardo201800478/SUN/actions/runs/46074483099) **SUCCESS**.

- `test_w07_pairwise_cycle.py`: **17/17 tests PASS** on Linux and Windows.
- Complete CHRONOS R3 disposable suite: **187/187 PASS** on Linux and Windows.
- R2 independent Linux and R2 Python Windows: PASS.
- Canonical and replayable corpus Linux/Windows byte-equivalence: PASS.
- Documentation patch hygiene, workflow validation, change-impact classification and Rust quality supervisor: PASS.
- Rust workspace regression and unrelated hardware/runtime/desktop jobs: SKIPPED, not counted as PASS.

The added matrix rejects malformed collection shapes, malformed/duplicate/conflicting `resolved` and `future` ledgers, invalid relation enums, self/unknown/unhashable endpoints and invalid operation collection shapes. Every negative case asserts `InvalidInput` and preservation of the original state object. This adds structural validation coverage only; it does not introduce temporal ordering rules or establish a physical B/C relation. W07 physical remains BLOCKED; W06 physical remains UNKNOWN; P-A is not closed; P-D/P-E remain NOT RUN; R4/R5 remain pending.


## W07 malformed-state completion — CI #11083

Validated HEAD: `0f917ed9b5b26c1862623b9c14a6812e82183d0c`; [CI #11083 / run 46074762710](https://github.com/leonardo201800478/SUN/actions/runs/46074762710) **SUCCESS**.

- `test_w07_pairwise_cycle.py`: **18/18 PASS** on Linux and Windows.
- Complete R3 disposable suite: **188/188 PASS** on Linux and Windows.
- R2 independent Linux and R2 Python Windows: PASS.
- Canonical/replayable corpus byte-equivalence Linux/Windows: PASS.
- Documentation patch hygiene, workflow validation, change-impact classification and Rust quality supervisor: PASS.
- Rust workspace regression and unrelated hardware/runtime/desktop jobs: SKIPPED, not counted as PASS.

The final negative case matrix also covers malformed cycle instances, invalid `at_common` type, pending/future descriptor fields and malformed relation endpoints. Each negative input returns `InvalidInput` and preserves the exact original state object. This validates structure, not physical B/C causality.

**Interpretive limitation:** the prototype's requirement that an emitted child use a strictly greater microstep than its emitter is a candidate policy chosen to avoid retroactive insertion. R1 §16.6.3 requires an explicit rule and prohibits retroactive insertion, but does not specify that this is the only admissible rule. Same-microstep append-after-emitter semantics are not covered. Treat strict advancement as an experimental policy/scope limitation, not a new normative R1 requirement.

W07 physical remains BLOCKED; W06 physical remains UNKNOWN; P-A is not closed; P-D/P-E remain NOT RUN; R4/R5 remain pending.


## Adversarial corrections — CI #11093

**Validated HEAD:** `6e99758af2d11d3bbe398240b294d7e28efc586a`; [CI #11093 / run 46083978800](https://github.com/leonardo201800478/SUN/actions/runs/46083978800) — **SUCCESS**.

- R3 disposable suite: **197/197 PASS Linux and 197/197 PASS Windows**.
- Corpus emitter: **176 tests PASS** on both systems; isolated W07 pairwise suite: **21 tests PASS**.
- R2 independent Linux and R2 Python Windows: PASS.
- All seven P-A synthetic matrices (advance, deadlines, causal sync, W06, W07, W08, W09): PASS on both systems.
- Canonical corpus: **117 fixtures / 1,481 cuts** (1,470 differential + 11 property); logical digest `ca73c8720f6b9b81454ff5e5f7e67f0aab1c49b71e24032ab4f6edcf74465181`.
- Replayable corpus: **25 fixtures / 134 operations-cuts**, with W06/W07 divergences preserved; logical digest `2dcf09eeaf6ffe03e6bcf1cfda087aeedeb865d351b6ac081a0dc12ff28bc28f`.
- Canonical and replayable artifacts are byte-identical between Linux and Windows. Documentation patch hygiene, workflow validation, change-impact classification and Rust quality supervisor: PASS.
- Windows CPU-time samples with coarse timer resolution: `SKIPPED_COARSE_RESOLUTION`; do not treat as performance evidence. Hardware/runtime/desktop jobs: SKIPPED, not PASS.

The seven adversarial findings were addressed in the disposable candidates/tests/docs: snapshot generation identity is keyed by (identity, generation); replay admission uses the snapshot-wide frontier once for the accepted batch; watermark closure no longer infers causality from microstep level; C=D cancellation requires a comparable phase basis; restore requires byte-canonical payloads; W07 pairwise emission has a finite resource limit with boundary/excess tests; and the inventory/source statements were corrected. R1 v0.3.18 and the R2 oracle remain unchanged.

**Limitations unchanged:** W06 physical evidence remains UNKNOWN; W07 physical B/C remains BLOCKED. The W07 candidate is bounded experimental relation adjudication, not a complete hardware cycle model. P-A has no closeout; P-D/P-E remain NOT RUN/BLOCKED; R4/R5 are pending. No production CHRONOS/V2 code or 60 FPS/hardware claim is validated by this Python CI.
