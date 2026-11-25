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
