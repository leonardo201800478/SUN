# P-A baseline results — first retained matrix

**Classification:** `OBSERVATION` — synthetic Python benchmark only
**Repository HEAD:** `555c3d034c0d0b7f2013e48fad80ae78d75c1106`
**CI:** [run #10844](https://github.com/leonardo201800478/SUN/actions/runs/45692690856) — SUCCESS
**Date:** 2026-11-25

## Validation

- P-A differential suite: 44/44 tests passed on Linux and Windows (24 advance/catch-up; 20 deadline lifecycle).
- R2 reference suite: PASS on Linux and Windows.
- P-A advance matrix: 75 parameter points per OS, 5 warm-ups and 30 measured repetitions per point.
- P-A deadline lifecycle matrix: 180 parameter points per OS, 5 warm-ups and 30 measured repetitions per point.
- Actionlint, documentation patch hygiene and quality supervisor: PASS.
- Unrelated hardware/runtime jobs were SKIPPED and are not counted as validation.

## Retained raw artifacts

All four artifacts were produced from the exact HEAD above and retained by CI for 30 days.

| Environment | Artifact | SHA-256 |
|---|---|---|
| Linux advance matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45692690856/artifacts/15058613650) | `a6c9dac60f4a1302ee5d29594ffd701fa20042ad55a8737aeb858004e0a290dc` |
| Windows advance matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45692690856/artifacts/15059197797) | `c723cc6200a9d1c00fc7cf30aaf8907079e120b64c5677a95f1a3b542fc2896a` |
| Linux deadline matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45692690856/artifacts/15058653944) | `dedff9bcdfb85adcae5f3fb202c72e906e054b3919a688913a77958cea484128` |
| Windows deadline matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45692690856/artifacts/15058554169) | `f52add901c8172b912a2c5ecc64ff350748b6eb0450b848f7c8a3811b96d1ba7` |

## Measurement environment and limitations

- Both environments used Python 3.14.4 and the same benchmark parameters.
- Linux thread CPU clock empirical resolution: 2,635 ns (advance) and 2,535 ns (deadline); CPU samples retained.
- Windows thread CPU clock empirical resolution: 15,625,000 ns; CPU samples and summaries were explicitly omitted as `SKIPPED_COARSE_RESOLUTION`. Wall-clock samples remain valid observations.
- tracemalloc peak is a separate probe outside timed samples.
- Fixtures, state digest, allocation probe and JSON serialization are outside the timed region.
- These runners are different machines. Their wall-time numbers are reported separately, not treated as a controlled cross-platform A/B.
- The results do not represent Rust native performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility.

## Advance/catch-up observations

Selected points (p50/p95 wall time):

| Workload | Linux | Windows | P-A counters |
|---|---:|---:|---|
| 16 domains, 0 obligations, chunk 64 | 64.690 / 65.805 ms | 63.093 / 66.284 ms | 782 chunks; 12,512 participant inspections |
| 16 domains, 0 obligations, chunk 8192 | 0.620 / 0.626 ms | 0.594 / 0.660 ms | 7 chunks; 112 participant inspections |
| 16 domains, 128 sparse obligations, chunk 64 | 2.201 / 2.261 ms | 2.072 / 2.143 ms | 16 chunks; 256 participant and 2,048 obligation inspections |
| 16 domains, 128 sparse obligations, chunk 8192 | 0.240 / 0.254 ms | 0.222 / 0.242 ms | 1 chunk; 16 participant and 128 obligation inspections |
| 16 domains, 128 dense obligations, chunk 64 | 0.312 / 0.324 ms | 0.293 / 0.314 ms | 1 chunk; 16 participant and 128 obligation inspections |

**Observation:** chunk size materially changes the number of repeated participant/obligation scans. In the no-event, 16-domain case, chunk 64 produces 782 chunks versus 7 at chunk 8192. This is the cost of the P-A bounded-chunk/full-scan design, not a recommendation to weaken temporal semantics or select a production chunk size.

## Deadline lifecycle observations

Selected points at 16 domains and 128 deadlines (p50/p95 wall time):

| Operation | Linux sparse | Windows sparse |
|---|---:|---:|
| Publish one deadline | 0.183 / 0.196 ms | 0.164 / 0.191 ms |
| Cancel one deadline | 0.328 / 0.344 ms | 0.313 / 0.496 ms |
| Arbitrate 128 pending cancellations | 3.538 / 3.592 ms | 3.544 / 3.645 ms |
| Close watermark — 128 sparse instants | 11.636 / 11.677 ms | 11.920 / 13.950 ms |
| Dispatch one claimed deadline | 0.200 / 0.202 ms | 0.186 / 0.223 ms |
| Complete one dispatched deadline | 0.202 / 0.214 ms | 0.185 / 0.230 ms |

For 128 dense deadlines grouped into eight instants, close-watermark p50/p95 was 3.083/3.133 ms on Linux and 3.053/3.150 ms on Windows.

**Observation:** the current `close_watermark` implementation repeatedly filters the due list for each distinct timestamp. Its sparse 128-instant workload is materially more expensive than the dense eight-instant workload. This is an identified algorithmic hotspot in the disposable P-A implementation. Any single-pass grouping refinement must preserve the same R2 tie/UNKNOWN and claim ordering tests and be benchmarked as a new candidate.

## W03/W04 causal synchronization — first retained matrix

**Prototype HEAD:** `aab24c457f424099d29475bc135f532537e488d0`
**CI:** [run #10859](https://github.com/leonardo201800478/SUN/actions/runs/45694821658) — SUCCESS

- P-A differential suite: 58/58 tests passed on Linux and Windows (24 advance/catch-up, 21 deadline lifecycle, 13 causal synchronization).
- R2 reference suite: PASS on Linux and Windows.
- Advance matrix: 75 points per OS.
- Deadline lifecycle matrix: 180 points per OS.
- W03/W04 causal matrix: 30 points per OS (dependency-chain fan-in/depth and repeated observations).
- Actionlint, documentation patch hygiene and quality supervisor: PASS.
- Unrelated hardware/runtime jobs: SKIPPED.

Selected causal matrix observations (p50 wall time):

| Workload | Linux | Windows |
|---|---:|---:|
| Dependency chain, fan-in 1, depth 16 | 73.9 µs | 140.8 µs |
| Dependency chain, fan-in 4, depth 16 | 80.5 µs | 157.2 µs |
| Dependency chain, fan-in 16, depth 16 | 118.1 µs | 224.3 µs |
| 32 repeated observations, fan-in 1 | 2.241 ms total | 4.248 ms total |
| 32 repeated observations, fan-in 4 | 7.955 ms total | 14.956 ms total |
| 32 repeated observations, fan-in 16 | 32.006 ms total | 59.211 ms total |

Repeated-observation rows measure the entire sequence of 32 sync calls, not one call. The matrix confirms that the serial full-scan resolver's work increases with fan-in and observation frequency. These are synthetic Python observations on different runners, not a controlled cross-platform A/B or a native-runtime prediction.

Final W03/W04 artifacts:

| Environment | Artifact | SHA-256 |
|---|---|---|
| Linux causal matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45694821658/artifacts/15060066683) | `7f4b8adabfb68a31d86738a2f796af5568ebde5f8619539ffebce6a2a9a20598` |
| Windows causal matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45694821658/artifacts/15059238450) | `36caa7c025981f802e6b81bda20c93015f3941252bfa12f737f32013a88e01a6` |

Linux CPU thread clock resolution was 2,584 ns (measured); Windows was 15,625,000 ns (CPU samples skipped as coarse resolution).

## Scope still incomplete

This baseline does **not** close P-A or R3. Remaining work includes:
- detailed fixture/trace equivalence consolidation beyond current per-operation state and applied-order comparisons;
- transaction observe/sample/commit and partial-effect ledger (W08);
- snapshot/restore/replay (W09);
- deeper W06/W07 boundary depth and workload coverage;
- adversarial independent review (Codex review requests were blocked by account usage limits).

P-D/P-E are not started. R4/R5 remain blocked. No production CHRONOS/V2 code is authorized before R5.


## Multi-operation fixture/trace equivalence follow-up

**Prototype HEAD:** `02a9bf17c8bb70fb5e9599754389e8d65210bf98`
**Validation:** CI #10868 SUCCESS on Linux/Windows; 60/60 P-A tests per OS. See the final closeout section below.

Two sequence tests were added:
- Deadline trace: publish generation 0 → admit cancellation → verify watermark close is blocked → arbitrate → publish generation 1 → close W → dispatch → complete. P-A and R2 book/result state are compared after each operation.
- Repeated observation trace: four successive observation cuts across CPU/VDP/PSG. P-A and R2 applied order and full causal state are compared at every cut.

Current P-A suite count: **60 tests** (24 advance/catch-up, 22 deadline lifecycle, 14 causal sync). These tests strengthen fixture/trace equivalence for the selected W00–W05 scenarios; they do not claim internal hardware microstep trace equivalence or close W08/W09.


## Final P-A CI closeout — CI #10868

- **Classification:** `OBSERVATION` — CI and synthetic Python prototype evidence
- **Workflow:** [run #10868](https://github.com/leonardo201800478/SUN/actions/runs/45695617409) — `SUCCESS`
- **Run ID:** `45695617409`
- **Executed HEAD:** `7351798b61971baad6d25133ce9616b3f5a33b9e`
- **Branch:** `research/chronos-r3-disposable-prototypes`
- **Date:** 2026-11-25

The run is associated with the exact HEAD above. The branch exists and the PR remains open and unmerged. The latest PR metadata update observed was 2026-11-25T20:30:27Z, immediately after the HEAD commit at 20:30:25Z. No separate branch-ref SHA was exposed by the available GitHub connector; therefore the SHA above is stated as the exact CI-tested HEAD, not as an independently queried ref response.

### Job summary

- 22 jobs total: 8 `SUCCESS`, 14 `SKIPPED`, 0 `FAILURE`, 0 `CANCELLED`.
- `SUCCESS`: Change impact classification; CHRONOS R2 Windows; documentation patch hygiene; Actionlint 1.7.12; CHRONOS R3 Windows; CHRONOS R3 Linux; CHRONOS R2 Linux; Rust quality/unit regression supervisor.
- The 14 skipped jobs concern unrelated hardware/runtime/desktop validations. They are not counted as PASS.

### Test evidence

- P-A R3: **60/60 tests PASS on Linux and 60/60 PASS on Windows** (24 advance/catch-up, 22 deadline lifecycle, 14 causal synchronization).
- Both R3 jobs executed all three benchmark matrices and uploaded their JSON artifacts.
- R2 independent reference: **256 tests PASS on Linux and 256 PASS on Windows**, zero failures/errors, no first divergence, mutation checks PASS, clean worktree before/after.
- Documentation patch hygiene and Actionlint: PASS.
- The quality supervisor completed successfully.

### Benchmark matrices

All matrices use 5 warm-ups and 30 measured repetitions per point.

| Matrix | Linux points | Windows points |
|---|---:|---:|
| Instruction/budget + catch-up | 75 | 75 |
| Deadline lifecycle | 180 | 180 |
| W03/W04 causal synchronization | 30 | 30 |

Selected observations from the current run (wall time, p50/p95; different hosted runners, not a controlled cross-platform A/B):

| Workload | Linux | Windows |
|---|---:|---:|
| Advance: 16 domains, 128 dense obligations, chunk 8192 | 311.771 / 325.575 µs | 448.150 / 538.090 µs |
| Deadline complete: 16 domains, 128 dense deadlines | 205.344 / 219.219 µs | 284.450 / 331.170 µs |
| Causal: 16 fan-in, 32 repeated observations (whole sequence) | 42.982 / 44.756 ms | 78.850 / 106.667 ms |

Windows `time.thread_time_ns` empirical resolution was 15,625,000 ns. CPU-time measurements are explicitly `SKIPPED_COARSE_RESOLUTION`; wall-time measurements and the test/benchmark jobs themselves passed. Linux thread CPU-clock resolutions were 2,575 ns (advance), 2,244 ns (deadlines) and 2,715 ns (causal), so CPU-time samples were retained there.

These are synthetic Python measurements, not Rust runtime performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility. The new run's numbers are retained as a separate observation; they must not be presented as a controlled speedup against earlier runs or across different hosted runners.

### Artifacts retained from run #10868

GitHub artifact digests were independently recalculated from the downloaded ZIP files and matched exactly.

| Artifact | Artifact ID | SHA-256 |
|---|---:|---|
| R2 Windows | 15061610581 | `1212055df3e0f383e71652716c58ddb33db34e66cc80ca11c566db3bee1ac447` |
| R3 causal Windows | 15061555663 | `2b39304937925ad577feb79d37455c6c74c312a3797f35c0e5401e5e7b6bb63e` |
| R3 advance Linux | 15061500503 | `e230ee8b41c4d885bafd58a48f3835958a2a398baee9d7b635a5e59af6e31e4d` |
| R3 deadlines Linux | 15060601442 | `a39f8184079994726667c152f88cbc82be204a08d6faaf03b091ecadeeebc576` |
| R3 deadlines Windows | 15060426806 | `ced5a7f6760be07d54a68551320b46e4cf2e3edc6e26890df13dea026e966f70` |
| R3 advance Windows | 15060247070 | `d8b740dae7725ad80405e42ba9e022bcc373e60ee43e5eb5699bc7b76b69ae4c` |
| R3 causal Linux | 15060246943 | `92fcb2acdf617809f504cfb10d562c818f7db818f01e09196eee8ad5f51edd49` |
| R2 Linux | 15060063376 | `9fabf6aa7b539c37e4231dc6812090e666411c059c7232577b59378a1fe53fcd` |

All eight artifacts were downloaded and inspected. R2 JSON reports 256 tests per OS with zero failures/errors and mutation checks PASS. R3 JSON files identify the same executed HEAD.

### Independent review status

- PR review submissions: none.
- Inline review threads: none.
- Codex review: **BLOCKED** by account code-review usage limits. The visible failed attempt targeted older commit `555c3d034c0d0b7f2013e48fad80ae78d75c1106`; later requests were also blocked by the usage limit.
- No independent adversarial review exists for the current HEAD `7351798b61971baad6d25133ce9616b3f5a33b9e`.

This remains a review gap. Do not mark R4 as passed.

### Remaining P-A / R3 gaps

- W06/W07: deeper simultaneous-UNKNOWN and microstep boundary/workload coverage.
- W08: transaction observe/sample/commit, resumability and partial-effect ledger.
- W09: snapshot/restore/replay.
- Complete canonical fixture/trace equivalence beyond the current selected state/order comparisons.
- Independent adversarial review.

**Disposition:** CI #10868 is PASS for the declared P-A W00–W05 prototype scope and its current test/benchmark jobs. P-A is not fully closed; R3 remains in progress. P-D/P-E remain blocked until the documented P-A equivalence and review gates are satisfied. R4/R5 remain pending. No production CHRONOS/V2 implementation is authorized before R5.


## Latest retained run — CI #10871

**Classification:** `OBSERVATION` — independent reference tests and synthetic Python prototype evidence
**Workflow:** [run #10871](https://github.com/leonardo201800478/SUN/actions/runs/45705184737) — `SUCCESS`
**Run ID:** `45705184737`
**Executed HEAD:** `75f8f048f248325850796ddc30d97c38b11dff86`
**Branch:** `research/chronos-r3-disposable-prototypes`
**Date:** 2026-11-25

### Job disposition

- 22 jobs total: 8 `SUCCESS`, 14 `SKIPPED`, 0 `FAILURE`, 0 `CANCELLED`.
- Successful jobs: change impact classification; CHRONOS R3 P-A Linux; CHRONOS R3 P-A Windows; CHRONOS R2 independent reference Linux; CHRONOS R2 reference Windows; documentation patch hygiene; Actionlint 1.7.12; Rust quality/unit regression supervisor.
- The 14 skipped jobs are unrelated hardware/runtime/desktop validations and are not counted as PASS.

### Test and reference evidence

- P-A R3: **60/60 tests PASS on Linux and 60/60 PASS on Windows** (24 advance/catch-up, 22 deadline lifecycle, 14 causal synchronization).
- R2 independent reference: **256/256 tests PASS on Linux and 256/256 PASS on Windows**; zero failures/errors, no first divergence, mutation checks PASS, worktree CLEAN before and after.
- Documentation patch hygiene: PASS after removing the five trailing-whitespace lines found by CI #10870.
- Actionlint 1.7.12: PASS.
- Quality supervisor: PASS.

### Benchmark matrix and cross-platform equivalence

All measurements are synthetic Python prototype experiments, with five warm-ups and 30 measured repetitions per point, on the exact HEAD above.

| Matrix | Linux points | Windows points | Result |
|---|---:|---:|---|
| Instruction/budget + catch-up | 75 | 75 | PASS |
| Deadline lifecycle | 180 | 180 | PASS |
| W03/W04 causal synchronization | 30 | 30 | PASS |

Independent inspection of the six JSON artifacts confirms, for every corresponding point across Linux and Windows:
- parameter sets are identical;
- output digests are identical;
- advance counters are identical;
- deadline result/claimed state is identical;
- causal applied counts are identical.

This is cross-platform equivalence of the declared synthetic prototype outputs for these matrices. It is not hardware timing evidence, physical SMS timing, Rust runtime performance, frame pacing, audio/video fidelity or ROM compatibility.

Linux thread CPU-clock resolutions were 2,263 ns (advance), 2,503 ns (deadlines) and 2,593 ns (causal); CPU samples were retained. Windows `time.thread_time_ns` resolution was 15,625,000 ns for all three matrices, so CPU-time samples are explicitly `SKIPPED_COARSE_RESOLUTION`; wall-time samples remain valid. `tracemalloc` is a separate probe outside timed samples.

### CI #10871 artifacts — all downloaded and SHA-256 verified

| Matrix | Artifact ID | SHA-256 |
|---|---:|---|
| R2 Linux | 15064299006 | `5ec8f08cfafe9acc36c229bfcf424822508fe70a102907ce25b027ef3f29bfce` |
| R2 Windows | 15063429315 | `7cf385180e24c2b3fd605c6741f006332b25cfde02af9c957fd58e3333c6cea8` |
| R3 advance Linux | 15065495955 | `23941966200175a5d9c34b3281acbc1787862dbdad80e86927b92fd6ef15ca3a` |
| R3 advance Windows | 15063529100 | `044c40f67889cc76d781ca4dd6af46c58b059112b7ba92d5583b489dbf0b5fbb` |
| R3 deadlines Linux | 15065765231 | `53c4819db3e7612978c57dc096ae7af7cbd837b5a1b4f9840790df2a7c814ac6` |
| R3 deadlines Windows | 15064842670 | `d14bbaaeae6c27b4b199c2f8245946f0ac9704d05cde13d79683090987eafa73` |
| R3 causal Linux | 15065635600 | `fd5db0b3efbebaccbfbc976fe62daa4dab589ee4f686b3b029b0e88f3e356d90` |
| R3 causal Windows | 15065331253 | `e89ae65d75518d930e723b27fe7934ff5c62eeed87318e206cbc9a3888b1f661` |

The SHA-256 values above were recalculated from the downloaded ZIP bytes and match the GitHub artifact digests. All eight artifacts identify executed HEAD `75f8f048f248325850796ddc30d97c38b11dff86`.

### Review and gate status

- PR review submissions: none.
- Inline review threads: none.
- Independent adversarial review: still BLOCKED / not performed for this HEAD.
- W06/W07 deeper UNKNOWN/microstep coverage, W08 transaction/partial-effect ledger, W09 snapshot/restore/replay, and complete canonical fixture/trace equivalence remain open.

**Disposition:** CI #10871 is PASS for the declared P-A W00–W05 prototype scope, including its cross-platform synthetic matrices. This does not close P-A's remaining equivalence/review requirements or R3. P-D/P-E remain blocked. R4/R5 remain pending. No production CHRONOS/V2 implementation is authorized before R5.


## Latest retained run — CI #10882 (W00–W07)

**Classification:** `OBSERVATION` — synthetic Python prototypes and independent R2 reference tests
**Workflow:** [run #10882](https://github.com/leonardo201800478/SUN/actions/runs/45708088537) — `SUCCESS`
**Run ID:** `45708088537`
**Executed HEAD:** `eb11debda2e13cb7b60c8721627e03a99d711880`
**Branch:** `research/chronos-r3-disposable-prototypes`
**Date:** 2026-11-25

### Job disposition

- 22 jobs total: 8 `SUCCESS`, 14 `SKIPPED`, 0 `FAILURE`, 0 `CANCELLED`.
- Successful jobs: change impact classification; CHRONOS R3 P-A Linux; CHRONOS R3 P-A Windows; CHRONOS R2 independent reference Linux; CHRONOS R2 reference Windows; documentation patch hygiene; Actionlint 1.7.12; Rust quality/unit regression supervisor.
- The 14 skipped jobs are unrelated hardware/runtime/desktop validations and are not counted as PASS.

### Test evidence

- P-A R3: **71/71 tests PASS on Linux and 71/71 PASS on Windows** (24 advance/catch-up, 26 deadline lifecycle including W06, 14 causal synchronization, 7 W07 microsteps).
- R2 independent reference: **256/256 tests PASS on Linux and 256/256 PASS on Windows**; zero failures/errors, no first divergence, mutation checks PASS (both deliberate boundary mutants KILLED), worktrees CLEAN before/after.
- Documentation patch hygiene: PASS.
- Actionlint 1.7.12: PASS.
- Quality supervisor: PASS.

### Five benchmark matrices — 321 points per OS

All matrices use five warm-ups and 30 measured repetitions per point.

| Matrix | Linux | Windows |
|---|---:|---:|
| Instruction/budget + catch-up | 75 | 75 |
| Deadline lifecycle | 180 | 180 |
| W03/W04 causal synchronization | 30 | 30 |
| W06 simultaneous UNKNOWN/order groups | 24 | 24 |
| W07 finite microstep chains | 12 | 12 |
| **Total** | **321** | **321** |

Independent inspection of all ten R3 JSON artifacts confirms identical parameter sets and output digests across Linux and Windows for all 321 corresponding points. The relevant semantic counters/state also match: advance counters, deadline result/claimed state, causal applied counts, W06 claim counts and W07 resolved counts. W06 output digests are invariant under forward/reverse insertion for every group-size/relation pair.

Linux thread CPU-clock resolutions: 2,815 ns (advance), 2,604 ns (deadlines), 2,935 ns (causal), 2,604 ns (W06) and 2,895 ns (W07); CPU samples retained. Windows resolution: 15,625,000 ns for all five matrices; CPU samples explicitly `SKIPPED_COARSE_RESOLUTION`, wall-time samples retained. `tracemalloc` is a separate probe outside timed samples.

These are synthetic Python observations, not Rust runtime performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility.

### CI #10882 artifacts — all 12 downloaded and SHA-256 verified

| Matrix | Artifact ID | SHA-256 |
|---|---:|---|
| R2 Linux | 15066057644 | `8e74fcc81a8c93c5eb46e8b06a54f90fdc4e28057a0398dbec600a362f6e418d` |
| R2 Windows | 15066746099 | `3699a0ea8d8f55ffc240b0a4514ac1b7b50cdc95829945cfd773cb82bfbb8bf9` |
| R3 advance Linux | 15066846121 | `c83d0d9f722dc589d6935f88d166a21f58a99dbcc5b178085fd84400175451cb` |
| R3 advance Windows | 15066421362 | `8229c9e72f51dc74e4cead4a3b5ba23519a2840cc449b2a3f4ca1a89de167edd` |
| R3 deadlines Linux | 15065468581 | `92b9f054813dfbce825ff328ef40f6599947eec505ee78d0298fbb4a548e0031` |
| R3 deadlines Windows | 15066072696 | `3e3e0d4ef44579d1642708f5679b5e0bc209cc840fefcd95aa9e8c1ea0fe773c` |
| R3 causal Linux | 15065468590 | `c1ec4e5248b83c1f6bf9949dba620c8bdd293bb9369d0e7bee942c94ab60dc5e` |
| R3 causal Windows | 15066381540 | `e9a6e56e0d55e2153347e03ae5a381c2fa757c5c6963631f7bc22928ed7181cf` |
| R3 W06 Linux | 15065438606 | `1556893f6018e14f026ba56c90dcefc5be0aa5ea1c61115438f403f998a1b1f7` |
| R3 W06 Windows | 15066621537 | `c69bb6b6a9d5eba4575dd21dea2cf857ff1543f9ab38da8a5409c1f691b06f2c` |
| R3 W07 Linux | 15067205166 | `00f8eabef14c830d286a718efe04b0c974182d84de8d4fe25cc997a5c0740793` |
| R3 W07 Windows | 15065254118 | `16812c15b1033a7f4458a695a582da48d7e1f5167a6dfa881f33d8f6741fdfa2` |

All 12 SHA-256 values were recalculated from downloaded ZIP bytes and matched GitHub artifact digests. All R3 artifacts identify executed HEAD `eb11debda2e13cb7b60c8721627e03a99d711880`.

### Remaining P-A / R3 gaps

- W08 transaction lifecycle: observe/sample/commit, suspend/resume, exactly-once effects and partial-effect ledger.
- W09 snapshot/restore/replay and deterministic digest.
- Complete canonical fixture/trace equivalence across the full declared P-A scope.
- Independent adversarial review of R3.

**Disposition:** CI #10882 is PASS for the declared P-A W00–W07 synthetic prototype scope. P-A is not fully closed. P-D/P-E remain unstarted until canonical equivalence and P-A closeout. R4/R5 remain pending. No production CHRONOS/V2 implementation is authorized before R5.


## Latest retained run — CI #10892 (W00–W08)

**Classification:** `OBSERVATION` — synthetic Python prototypes and independent R2 reference tests
**Workflow:** [run #10892](https://github.com/leonardo201800478/SUN/actions/runs/45709649888) — `SUCCESS`
**Run ID:** `45709649888`
**Executed HEAD:** `3b54226c472489092c1a2a3b9d29685cc6dac3df`
**Branch:** `research/chronos-r3-disposable-prototypes`
**Date:** 2026-11-25

### Job disposition

- 22 jobs total: 8 `SUCCESS`, 14 `SKIPPED`, 0 `FAILURE`, 0 `CANCELLED`.
- Successful jobs: change impact classification; CHRONOS R3 P-A Linux; CHRONOS R3 P-A Windows; CHRONOS R2 reference Linux; CHRONOS R2 reference Windows; documentation patch hygiene; Actionlint 1.7.12; Rust quality/unit regression supervisor.
- The 14 skipped jobs are unrelated hardware/runtime/desktop validations and are not counted as PASS.

### Test evidence

- P-A R3: **89/89 tests PASS on Linux and 89/89 PASS on Windows** (24 advance/catch-up, 26 deadline lifecycle including W06, 14 causal synchronization, 7 W07 microsteps, 18 W08 transaction lifecycle).
- R2 independent reference: **256/256 tests PASS on Linux and 256/256 PASS on Windows**; zero failures/errors, no first divergence, mutation checks PASS (both deliberate boundary mutants KILLED), worktrees CLEAN before/after.
- Documentation patch hygiene: PASS.
- Actionlint 1.7.12: PASS.
- Quality supervisor: PASS.

### Six benchmark matrices — 331 points per OS

All matrices use five warm-ups and 30 measured repetitions per point.

| Matrix | Linux | Windows |
|---|---:|---:|
| Instruction/budget + catch-up | 75 | 75 |
| Deadline lifecycle | 180 | 180 |
| W03/W04 causal synchronization | 30 | 30 |
| W06 simultaneous UNKNOWN/order groups | 24 | 24 |
| W07 finite microstep chains | 12 | 12 |
| W08 transaction lifecycle / partial effects | 10 | 10 |
| **Total** | **331** | **331** |

Independent inspection of all twelve R3 JSON artifacts confirms identical parameter sets and output digests across Linux and Windows for all 331 corresponding points. Semantic state also matches: advance counters, deadline result/claimed state, causal applied counts, W06 claim counts, W07 resolved counts and W08 transaction state/sample/commit/pending ledgers.

W08 partial-failure cases preserve the sampled marker and exact committed prefix: sizes 1/2/4/8/16 produce 0/1/2/4/8 committed effects and 1/1/2/4/8 pending markers, respectively. The size-1 case fails after sampling and before the first commit.

Linux thread CPU-clock resolutions: 1,609 ns (advance), 1,629 ns (deadlines), 1,786 ns (causal), 1,735 ns (W06), 22,642 ns (W07) and 1,766 ns (W08). Windows resolution was 15,625,000 ns for all six matrices; CPU-time samples are explicitly `SKIPPED_COARSE_RESOLUTION`, while wall-time samples remain available. `tracemalloc` is a separate probe outside timed samples.

These are synthetic Python observations, not Rust runtime performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility.

### CI #10892 artifacts — all 14 downloaded and SHA-256 verified

| Matrix | Artifact ID | SHA-256 |
|---|---:|---|
| R2 Linux | 15066098030 | `4b03c4ace804eae1ed33434cf32d1661d6a2737b72464db1f70c950029cf2e98` |
| R2 Windows | 15067505689 | `6f3ecc703c756a8390c120befa3fcd84c696bceb4e68539aa4e1be298f9064c9` |
| R3 advance Linux | 15066932001 | `b5cf4e03360591ce691930820ba2f7e8bb55f544567825d923a50a3511f9ae31` |
| R3 advance Windows | 15066643036 | `1b5438c41f1866319b1e81fd8dfabcb84043cd740d1e126feb4e5069a026e451` |
| R3 deadlines Linux | 15066477440 | `c98e1b3f63933089920d3ac032c7232a250487d90041428078be64a8410c3137` |
| R3 deadlines Windows | 15067052039 | `dfaed989cffd4a7d1c6b60185139114f6b838c0787204aeebe628ce1ce138b1f` |
| R3 causal Linux | 15066063838 | `e27f0f718cebfbaa5a2d526d1a3b8c9e87ed50c168b6c01b8d4fd2741dab6657` |
| R3 causal Windows | 15067655407 | `330c9414d2ac4d8deaee4085e8caed181eb6e72faa5da7109fe1f7405330682b` |
| R3 W06 Linux | 15066497562 | `9d9efc394c46fec3544054b04e35a12354594fe92147a182fab3b504e8a87386` |
| R3 W06 Windows | 15066732791 | `6d99820196e7ca44a8e5e9320fedd826aa7d9fc3edf1562250f70cabb81c1a3c` |
| R3 W07 Linux | 15065104543 | `319757b8ca3ee7ef9a5ad07dfdda5bf7c310654310b6a06f341a9954c191df9a` |
| R3 W07 Windows | 15066777805 | `124a14b259d51d9327f4b2813f06e676944a1fde2b300b74fbbfee7c75625ab4` |
| R3 W08 Linux | 15066503327 | `7d55fdf15b24e01c200f0913df2ad77761d2e2bd7a94e61e99e59ba678ac222a` |
| R3 W08 Windows | 15067530746 | `dcb01985316c59653da782713e3bbacb54457e0932856f493bce5ed18d5697b7` |

All 14 SHA-256 values were recalculated from downloaded ZIP bytes and matched GitHub artifact digests. All R3 artifacts identify executed HEAD `3b54226c472489092c1a2a3b9d29685cc6dac3df`.

### Remaining P-A / R3 gaps

- W09 snapshot/restore/replay and deterministic digest.
- Complete canonical fixture/trace equivalence across the full declared P-A scope.
- Independent adversarial review of R3.

**Disposition:** CI #10892 is PASS for the declared P-A W00–W08 synthetic prototype scope. P-A is not fully closed. P-D/P-E remain unstarted until W09, canonical equivalence and P-A closeout. R4/R5 remain pending. No production CHRONOS/V2 implementation is authorized before R5.


## Latest retained run — CI #10902 (W00–W09)

**Classification:** `OBSERVATION` — synthetic Python prototypes and independent R2 reference tests
**Workflow:** [run #10902](https://github.com/leonardo201800478/SUN/actions/runs/45710697470) — `SUCCESS`
**Run ID:** `45710697470`
**Executed HEAD:** `81cb1780a61dd5af80c52582d379497378365109`
**Branch:** `research/chronos-r3-disposable-prototypes`
**Date:** 2026-11-25

### Job disposition

- 22 jobs total: 8 `SUCCESS`, 14 `SKIPPED`, 0 `FAILURE`, 0 `CANCELLED`.
- Successful jobs: change impact classification; CHRONOS R3 P-A Linux; CHRONOS R3 P-A Windows; CHRONOS R2 reference Linux; CHRONOS R2 reference Windows; documentation patch hygiene; Actionlint 1.7.12; Rust quality/unit regression supervisor.
- The 14 skipped jobs are unrelated hardware/runtime/desktop validations and are not counted as PASS.

### Test evidence

- P-A R3: **116/116 tests PASS on Linux and 116/116 PASS on Windows** (24 advance/catch-up, 26 deadline lifecycle including W06, 14 causal synchronization, 7 W07 microsteps, 18 W08 transactions, 27 W09 snapshot/replay).
- R2 independent reference: **256/256 tests PASS on Linux and 256/256 PASS on Windows**; zero failures/errors, no first divergence, mutation checks PASS (both deliberate boundary mutants KILLED), worktrees CLEAN before/after.
- Documentation patch hygiene: PASS.
- Actionlint 1.7.12: PASS.
- Quality supervisor: PASS.

### Seven benchmark matrices — 351 points per OS

All matrices use five warm-ups and 30 measured repetitions per point.

| Matrix | Linux | Windows |
|---|---:|---:|
| Instruction/budget + catch-up | 75 | 75 |
| Deadline lifecycle | 180 | 180 |
| W03/W04 causal synchronization | 30 | 30 |
| W06 simultaneous UNKNOWN/order groups | 24 | 24 |
| W07 finite microstep chains | 12 | 12 |
| W08 transaction lifecycle / partial effects | 10 | 10 |
| W09 snapshot/restore/replay | 20 | 20 |
| **Total** | **351** | **351** |

Independent inspection of all fourteen R3 JSON artifacts confirms identical parameter sets and output digests across Linux and Windows for all 351 corresponding points. Semantic state also matches: advance counters, deadline result/claimed state, causal applied counts, W06 claim counts, W07 resolved counts, W08 transaction/sample/commit/pending ledgers and W09 replay digest/cursor/domain/deadline/transaction counts.

W08 partial-failure cases preserve the sampled marker and exact committed prefix: sizes 1/2/4/8/16 produce 0/1/2/4/8 committed effects and 1/1/2/4/8 pending markers, respectively. The size-1 case fails after sampling and before the first commit.

W09 uses canonical JSON bytes and SHA-256. Restore validates profile/model version, digest and state invariants before exposing the restored state. Replay sorts accepted input records by sequence, validates contiguous cursor progression, monotonic common time and per-domain progress, then produces a deterministic state digest. Arrival-order permutations and repeated replay produce identical results; gaps, duplicates, unknown domains and regressions are rejected without partial application.

Linux thread CPU-clock resolutions: 3,006 ns (advance), 2,495 ns (deadlines), 2,454 ns (causal), 2,385 ns (W06), 2,534 ns (W07), 2,184 ns (W08) and 2,364 ns (W09). Windows resolution was 15,625,000 ns for all seven matrices; CPU-time samples are explicitly `SKIPPED_COARSE_RESOLUTION`, while wall-time samples remain available. `tracemalloc` is a separate probe outside timed samples.

These are synthetic Python observations, not Rust runtime performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility.

### CI #10902 artifacts — all 16 downloaded and SHA-256 verified

| Matrix | Artifact ID | SHA-256 |
|---|---:|---|
| R2 Linux | 15065874160 | `5b80cacca01c339590c4de391e076712b05967a4eaaf2901efd93f1a153c92d1` |
| R2 Windows | 15066904419 | `582d80de84ea83c2a58776a419f6004df0f05aa8c42de7cbd5985fc35ce3254c` |
| R3 advance Linux | 15066158801 | `d4a092e6407e3c73aea5419aa38042dafdc15f0459c66ede6f99f04d76cdd4a7` |
| R3 advance Windows | 15066794649 | `900c9b9935c5274f9a3664c0e89ccc68ead289fe13941162c358b5d4b93afe20` |
| R3 deadlines Linux | 15067990476 | `75157d25921ac261c8bd7b8b5f664549b84d703dd17d7fc732cb0dc638db28c4` |
| R3 deadlines Windows | 15067436850 | `c04212801bc5129cdbc71f2579408d9281311d2d731a8d3d51173e9d8124f65f` |
| R3 causal Linux | 15066534259 | `985c2d3f3931e7f52bba77bdd7909be41f984d583b8e49dddd8aa3104058c22d` |
| R3 causal Windows | 15067203692 | `c6b6c467231def0785f6b7926406bc981fe8597b6dfb542ed41edf049a4dcdb2` |
| R3 W06 Linux | 15066378583 | `17dcb96ecc73935eba7cb0a60dc1fc091d9ec38d76571f3edbbe9475b134e70c` |
| R3 W06 Windows | 15067776321 | `97d0199cfbba28f7af574af8c737a80490a1b87ff307b67ce0e66a5ac8071ae8` |
| R3 W07 Linux | 15066298703 | `669a334a5d26052291f63aa83a7d7efaffcbbd3337e13639c36bb014435b2c19` |
| R3 W07 Windows | 15067477410 | `34ea81ac0bfe940d325c46570445f409833aed31d5bc3b066d008019433c43aa` |
| R3 W08 Linux | 15066188956 | `16cf68998cd8519fb11739d8a7c94bbb56134cec8670cb7d6e354bd9d51de0ac` |
| R3 W08 Windows | 15066794698 | `6956bf5800cf50dc34e0513ed2ce679bd4a281eb489590bbc49b9745797194d9` |
| R3 W09 Linux | 15066364432 | `9f68f264540ec3b084f8fe9802a5972509a26e5186a7edd475c0eb043de9f14f` |
| R3 W09 Windows | 15067382657 | `559adb29a5cfaac97b9b9f47ca9e3251e96a0dfae9283f6a949bf7c92e70a287` |

All 16 SHA-256 values were recalculated from downloaded ZIP bytes and matched GitHub artifact digests. All R3 artifacts identify executed HEAD `81cb1780a61dd5af80c52582d379497378365109`.

### Remaining P-A / R3 gaps

- Complete canonical fixture/trace equivalence across the full declared P-A scope.
- Independent adversarial review of R3 and reconciliation of any findings.

**Disposition:** CI #10902 is PASS for the declared P-A W00–W09 synthetic prototype scope. P-A is not fully closed until canonical fixture/trace equivalence and adversarial review are completed. P-D/P-E remain unstarted. R4/R5 remain pending. No production CHRONOS/V2 implementation is authorized before R5.


## Corrective P-A run — CI #10931 (findings W06/W07 + benchmark statistics)

**Classification:** `OBSERVATION` — synthetic Python prototypes and independent R2 tests
**Workflow:** [CI #10931](https://github.com/leonardo201800478/SUN/actions/runs/45747493838) — overall `FAILURE` due to documentation whitespace; R3/R2 tests and all benchmark matrix jobs succeeded.
**Executed HEAD:** `21480075857dc2ef6bbe951ba0f894ce630f9df7`
**Branch:** `research/chronos-r3-disposable-prototypes`
**Date:** 2026-11-26
**Python:** 3.14.4
**Platforms:** Linux `Linux-7.0.0-1020-azure-x86_64-with-glibc2.43`; Windows `Windows-2025Server-10.0.26100-SP0`.

### Test and corpus evidence

- P-A R3: **125/125 PASS Linux; 125/125 PASS Windows**.
- R2 independent reference: **256/256 PASS Linux; 256/256 PASS Windows**, no first divergence; deliberate boundary mutation checks PASS.
- W06/W07 discriminants passed on both platforms, including rejection without mutation, valid processing after prior obligations resolve, phase-basis equality/inequality, missing/invalid basis, equal phase values and commutative events.
- Canonical corpus: 112 fixtures, 1,475 cuts, 1,468 differential cuts, 7 property cuts. Linux and Windows JSON documents are fully equal.
- Canonical trace digest: `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.

### Benchmark protocol

- Seven matrices, **351 points per OS**: advance 75; deadline lifecycle 180; causal W03/W04 30; W06 24; W07 12; W08 10; W09 20.
- Five warm-ups and 30 measured repetitions per point; each point records `sample_count=30`.
- Units: nanoseconds. Every point preserves all 30 raw wall-time samples. Linux points also preserve 30 thread-CPU samples. Windows CPU samples are explicitly null/omitted from summaries because clock resolution is coarse.
- Estimator: Hyndman-Fan Type 7, `h=(n-1)p`, linear interpolation. The maximum is the maximum raw observation, not a quantile estimate.
- Shared policy: `SUN-V2/benchmarks/r3/benchmark_statistics.py`; CPU clock is accepted only at empirical resolution <=10,000 ns.
- Linux thread CPU clock resolutions (advance/deadlines/causal/W06/W07/W08/W09): 2,298 / 1,911 / 1,926 / 1,931 / 1,867 / 2,083 / 2,184 ns.
- Windows resolution: 15,625,000 ns for all seven matrices; CPU time is `SKIPPED_COARSE_RESOLUTION`. Wall-time remains available.
- `tracemalloc` remains a separate probe outside timed samples. Runners differ; Linux/Windows wall-time values are reported separately, never as a controlled A/B.

### Wall-time summary by matrix

The p50/p95/p99 columns are **ranges across parameter points** (minimum to maximum of each point's quantile). `Max` is the largest raw sample observed over all points in that matrix. Values are microseconds.

| Matrix | Points | Linux p50 | Linux p95 | Linux p99 | Linux max | Windows p50 | Windows p95 | Windows p99 | Windows max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Advance/catch-up | 75 | 23.9–52,522.7 | 26.7–52,992.1 | 32.3–53,035.3 | 53,051.3 | 31.8–75,249.1 | 34.3–76,366.6 | 35.3–81,695.8 | 83,796.0 |
| Deadline lifecycle | 180 | 10.0–2,856.3 | 10.5–4,968.9 | 11.1–5,030.5 | 5,038.9 | 12.4–4,296.1 | 13.1–6,169.5 | 13.3–7,082.6 | 7,173.1 |
| Causal W03/W04 | 30 | 19.5–36,535.9 | 24.1–37,459.4 | 24.5–37,617.7 | 37,621.4 | 23.9–52,485.7 | 28.3–54,121.7 | 38.1–59,279.4 | 61,236.6 |
| W06 ordering | 24 | 6.6–214.1 | 7.4–235.0 | 8.0–255.4 | 261.8 | 9.1–326.2 | 10.0–354.1 | 14.7–356.3 | 356.4 |
| W07 microsteps | 12 | 12.0–242.2 | 16.9–251.9 | 19.9–271.2 | 278.9 | 16.6–339.7 | 17.1–369.3 | 17.2–372.0 | 372.8 |
| W08 transactions | 10 | 32.7–287.1 | 37.4–296.8 | 55.2–309.9 | 315.0 | 44.6–631.3 | 46.5–668.8 | 56.1–750.8 | 783.6 |
| W09 snapshot/replay | 20 | 138.0–1,035.4 | 156.5–1,241.4 | 162.1–1,457.0 | 1,497.5 | 203.1–1,370.7 | 228.8–1,638.6 | 233.5–2,013.5 | 2,130.7 |

### CPU-time summary — Linux only

The same range convention applies; values are microseconds.

| Matrix | p50 range | p95 range | p99 range | Maximum |
|---|---:|---:|---:|---:|
| Advance/catch-up | 23.3–52,521.1 | 26.0–52,985.2 | 31.7–53,030.5 | 53,048.8 |
| Deadline lifecycle | 9.4–2,855.6 | 10.0–4,967.4 | 10.4–5,028.4 | 5,036.6 |
| Causal W03/W04 | 18.9–36,535.4 | 23.5–37,455.2 | 23.9–37,611.3 | 37,614.9 |
| W06 ordering | 7.1–214.8 | 8.5–235.9 | 9.3–256.3 | 262.7 |
| W07 microsteps | 12.5–242.8 | 17.8–252.6 | 21.3–272.2 | 280.0 |
| W08 transactions | 33.3–287.7 | 38.1–297.7 | 55.8–310.6 | 315.7 |
| W09 snapshot/replay | 138.7–1,036.2 | 157.2–1,243.2 | 162.8–1,458.6 | 1,499.0 |

### Cross-platform comparison

- All 351 corresponding parameter sets match.
- All seven matrix point-level semantic payloads and output digests match after excluding host timing, CPU timing, allocation probes and wall-derived throughput.
- The canonical corpus is fully equal, including every fixture and cut.
- This is semantic/output equivalence of the declared synthetic workloads, not equality of wall-time performance or proof of physical SMS behavior.

### CI artifacts — 18 ZIPs downloaded, integrity-tested and SHA-256 verified

| Matrix | Linux artifact ID | Linux ZIP SHA-256 | Windows artifact ID | Windows ZIP SHA-256 |
|---|---:|---|---:|---|
| R2 reference | 15083564732 | `f105838c6b73c1a194f37c790c59bf4a649f01ce8ce4e1a80f33bbde899de4c8` | 15084051519 | `f518ad98342099af201a31087d01f282387f99fde7cae7afae62205c8af8bd4d` |
| Canonical trace | 15083902211 | `a75e01c6067a01ebf7d0d6f8e13ab7c3cb2fddf9e31e2e2b4e16122cf3c02730` | 15084470441 | `b1a84c5964d2a63e43f61106c52fd19558e6d366c76f300737d50ed04f8dc20e` |
| Advance | 15083599556 | `12a457c0ef8b76e0bbd762f8309c47d35a9482984fd63defdb7f02016a326faa` | 15084021782 | `4f192e5f4c27a4a649238d4605cad42a01c69390e7a77f015542e2bcb94220f1` |
| Deadlines | 15083782995 | `0d680f8d3da6525d33ba9e44b318597c93f1152c1c753fae81dda6841d6a73f5` | 15084071607 | `bff44b3dd1b4bd39e658da8556896914fd64fa6f84786e0d06174f0852f1f3ef` |
| Causal | 15083728212 | `58562b57668f01fbd6d7d2a46a0ffc2ed23cee8f4e01d85415f23f92509d1249` | 15083294233 | `68b87cbd01ced9dad9c29d5b9ef9e22a3e57600a52339b6f08e204d5f8543ee6` |
| W06 | 15083783013 | `6cd44f426f08ebd8e5c0639180429b8fdbf634a5bb15bd43830ea4aa716f0d84` | 15083079335 | `0ee95ab63db174984ae1ee0d286d33d14f4ecb678b07d9dc737b46fd8615d408` |
| W07 | 15083619523 | `5a4e6f5e19f2f429350b826ee0d314fab452f06781a9faacc5e7338d5adc047b` | 15083294240 | `2e43e27b0291584c5856e380e1fda536307576a6524274277c7ac5374042c7c6` |
| W08 | 15083718393 | `a3350a40d8f0f82dff978757237f5ef11f9de152ae35f49d87c5c817bf077933` | 15082974795 | `bc17d0a30ccb0e0ab2e04f604e378348367383e1594f001891837afd6c6066d6` |
| W09 | 15083673832 | `bd36b3102786e23aaf7ffafa5cb23001885ee550480f9317d88eb7ceb8259ac5` | 15083498761 | `991545b63f61938788c9fff0d2a42df7fc66ef2566d8d039f055e0ea9f31f0b2` |

All listed ZIP SHA-256 digests were recalculated from downloaded bytes and matched the GitHub digests; all ZIP integrity tests passed. R2 oracle SHA-256 was identical on both platforms: `3e27b3d387b0af3fa30fd78626e8fd3cc00e1ef289e7b68192c5b3d838a494fb`.

### Disposition and limitations

- Benchmark and test jobs in CI #10931 passed. The overall workflow failed only because `git diff --check` found trailing spaces in `R2_R3_ORACLE_GAP_PROPOSAL.md`; the quality supervisor failed because it requires documentation success.
- The trailing whitespace was removed in the next revision. A subsequent CI run on the corrected documentation must complete all required jobs before this revision is classified fully green.
- These results are synthetic Python observations, not Rust runtime performance, physical SMS timing, FPS, frame pacing, audio/video fidelity or ROM compatibility.
- R2 source and tests were not modified. The W06/W07 oracle gaps are documented separately; differential equality is not used as proof for those two scenarios.


## Final CI revalidation — CI #10936

**Disposition:** `SUCCESS`
**Workflow:** [CI #10936](https://github.com/leonardo201800478/SUN/actions/runs/45748159166)
**Executed HEAD:** `34f4f52865b429269ebad05a7857ac4a2e3d5ce5`

- P-A R3: 125/125 PASS on Linux and 125/125 PASS on Windows.
- R2 independent reference: 256/256 PASS on Linux and 256/256 PASS on Windows.
- All seven matrices passed on both platforms (351 points per OS).
- Corpus digest remains `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`; complete Linux/Windows JSON equality PASS.
- Actionlint, documentation patch hygiene and Rust quality/unit regression supervisor: PASS.
- Fourteen unrelated hardware/runtime/desktop jobs: SKIPPED, not counted as PASS.
- All 18 CI #10936 ZIP artifacts were downloaded, ZIP-integrity tested and SHA-256 recalculated; all matched the digests published by GitHub. Every benchmark point retained 30 wall-time samples; Linux also retained 30 CPU-time samples, while Windows correctly marked CPU time `SKIPPED_COARSE_RESOLUTION`.

CI #10931 remains in the historical record because it produced the detailed benchmark metric ranges and artifact table; its overall failure was caused by trailing whitespace in the oracle-gap proposal and the dependent quality supervisor. That formatting issue was corrected, and CI #10936 is the final successful validation of the corrected HEAD.

**P-A disposition remains IN PROGRESS, not CLOSED:** independent adversarial review and formal consideration of the R2 oracle-gap proposal remain pending. No gate promotion follows from this CI success.


## Atualização de métricas — CI #10962

**HEAD:** `c6d9f71f989cf9c1ff61fde8a36ea883b82a35ff`  
**Run:** [CI #10962](https://github.com/leonardo201800478/SUN/actions/runs/45753632859)  
**Matrizes:** sete, 351 pontos por plataforma; cinco warm-ups e 30 amostras medidas por ponto.

A tabela publica, para cada matriz, o **maior p99 entre os pontos de parâmetros** e o **maior valor bruto observado** entre todas as amostras. Quantis usam Hyndman–Fan Type 7. Valores em microssegundos (µs).

| Matriz | Linux p99 máx. | Linux máximo observado | Windows p99 máx. | Windows máximo observado |
|---|---:|---:|---:|---:|
| Advance/catch-up | 35.539,3 | 35.595,1 | 76.519,9 | 77.079,0 |
| Deadline lifecycle | 3.246,3 | 3.253,4 | 20.295,4 | 25.184,9 |
| Causal W03/W04 | 25.210,0 | 25.436,8 | 58.454,1 | 59.083,1 |
| W06 ordering | 160,7 | 162,7 | 344,5 | 353,8 |
| W07 microsteps | 203,6 | 210,2 | 353,5 | 361,9 |
| W08 transactions | 191,7 | 191,7 | 380,4 | 380,5 |
| W09 snapshot/replay | 905,4 | 1.006,6 | 1.191,6 | 1.192,8 |

O p99 com 30 amostras é uma estatística descritiva do lote, não um limite robusto de cauda. Linux teve resolução de CPU clock entre 2.204 ns e 2.784 ns e manteve métricas de CPU. Windows apresentou resolução de 15.625.000 ns em todas as matrizes; métricas de CPU foram corretamente classificadas como `SKIPPED_COARSE_RESOLUTION`. Wall-time continua disponível.

Essas medições são observações sintéticas de Python e não constituem benchmark do runtime Rust, FPS, frame pacing ou evidência de hardware SMS.
