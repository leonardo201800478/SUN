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

## Scope still incomplete

This baseline does **not** close P-A or R3. Remaining work includes:
- W03/W04 observation-driven and frequent-observation workloads;
- detailed trace/fixture equivalence beyond current result/state differential checks;
- transaction observe/sample/commit and partial-effect ledger (W08);
- snapshot/restore/replay (W09);
- additional W06/W07 boundary depth and workload coverage;
- adversarial independent review.

P-D/P-E are not started. R4/R5 remain blocked. No production CHRONOS/V2 code is authorized before R5.


## P-A v2 — single-pass deadline grouping follow-up

**Prototype source commit:** `292547d6f46a0a950036d46ba90d1e3540727423`  
**Final validation HEAD:** `dc9b9ab7b00c1b956f025e3cc9dee40b29c5b24e` (report formatting-only commit after the prototype source)  
**Final CI:** [run #10851](https://github.com/leonardo201800478/SUN/actions/runs/45693814913) — SUCCESS

P-A v2 changes `close_watermark` to collect due deadlines into a timestamp map in one pass, then sort instants and apply the same causal tie rules. It remains a disposable full-scan/temporary-grouping implementation; no persistent agenda was introduced.

The v2 p50 for 16 participants / 128 deadlines was:

| Environment | Distribution | v1 p50 (run #10844) | v2 p50 (run #10851) | v2 p95 |
|---|---|---:|---:|---:|
| Linux | 128 sparse instants | 11.636 ms | 1.295 ms | 1.373 ms |
| Linux | 8 dense instants | 3.083 ms | 1.230 ms | 1.236 ms |
| Windows | 128 sparse instants | 11.920 ms | 3.279 ms | 3.415 ms |
| Windows | 8 dense instants | 3.053 ms | 3.150 ms | 3.262 ms |

A first v2 run (#10849) measured 2.763 ms sparse / 2.667 ms dense on Linux and 1.949 ms sparse / 1.856 ms dense on Windows. The second run (#10851) produced materially different values, especially on Windows. These are separate CI runs and not an interleaved same-process A/B; runner load/variation is visible. Therefore no fixed percentage speedup is claimed. The structural change removes repeated filtering of the entire due list once per distinct timestamp; future comparison should use paired/interleaved samples on a controlled runner.

Final CI #10851 validated **45 P-A tests** (24 advance/catch-up; 21 deadline lifecycle) on Linux and Windows, both R2 suites, both benchmark matrices, Actionlint, documentation patch hygiene and the quality supervisor. Hardware/runtime jobs remained SKIPPED.

### Final v2 artifacts

All artifacts below were produced from validation HEAD `dc9b9ab7b00c1b956f025e3cc9dee40b29c5b24e` and retained by CI for 30 days.

| Environment | Artifact | SHA-256 |
|---|---|---|
| Linux advance matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45693814913/artifacts/15058819679) | `274552f65f4d9c7a08953bc75538315642f0f8cadd2bd8992d2b026fceb23ea0` |
| Windows advance matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45693814913/artifacts/15058934714) | `c203b71d461f697d1a038b13ab977968a22e95981b7f2d15d468521d56b02a57` |
| Linux deadline matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45693814913/artifacts/15059343325) | `cda7968472cedc21289b222e31696a9336788f98c685bbec52f36c56d9be588e` |
| Windows deadline matrix | [JSON artifact](https://github.com/leonardo201800478/SUN/actions/runs/45693814913/artifacts/15059787973) | `afb8fcb171ca5707e75c37821cb3b3ab601488f2fbd75a59006099090f27d347` |

- Linux CPU thread clock resolution: 2,003 ns (advance) and 2,233 ns (deadlines); CPU samples retained.
- Windows CPU thread clock resolution: 15,625,000 ns; CPU samples explicitly skipped as `SKIPPED_COARSE_RESOLUTION`.
- tracemalloc remains a separate allocation probe outside timed samples.

## Scope still incomplete

This baseline does **not** close P-A or R3. Remaining work includes:
- W03/W04 observation-driven and frequent-observation workloads;
- detailed trace/fixture equivalence beyond current result/state differential checks;
- transaction observe/sample/commit and partial-effect ledger (W08);
- snapshot/restore/replay (W09);
- additional W06/W07 boundary depth and workload coverage;
- adversarial independent review (Codex review requests were blocked by account usage limits).

P-D/P-E are not started. R4/R5 remain blocked. No production CHRONOS/V2 code is authorized before R5.
