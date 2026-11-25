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
