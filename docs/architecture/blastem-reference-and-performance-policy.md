# BlastEm — Oracle and Performance Engineering Policy

- **Status:** PROPOSED.
- **Date:** 2026-11-22.
- **Primary reference:** BlastEm by Michael Pavone.
- **SUN implementation language/license:** Rust / MIT (repository declaration at the current baseline).

## 1. What “follow BlastEm” means

SUN adopts BlastEm as the primary behavioral oracle and studies its performance architecture. This means reproducing independently the useful engineering principles, not copying source code or assuming every implementation detail is correct for every Sega console.

BlastEm is not infallible. A reproducible hardware test, authoritative platform documentation or a conformance ROM can supersede it. Secondary emulators are used to cover platform support gaps and investigate disagreements.

## 2. Reference pinning

Every oracle campaign must record:
- upstream/mirror URL and exact revision;
- build flags and target architecture;
- console model/revision and region;
- ROM name and SHA-256;
- configuration affecting timing/audio/video;
- command line and environment;
- output artifacts and comparison tool version.

Do not compare SUN against an unpinned “latest BlastEm” binary.

## 3. Performance principles to investigate and independently implement

### 3.1 Lazy synchronization / sync-on-access

Let a CPU execute until a safe synchronization horizon. Advance dependent peripherals to the CPU timestamp when the CPU accesses them or when an independently generated event must be serviced.

This is valid only if:
- all events that can affect the CPU are represented in the horizon;
- interrupt lines are sampled at the correct boundary;
- I/O reads/writes observe the same state as the oracle;
- DMA, WAIT and bus arbitration are not bypassed;
- instruction execution can suspend/resume without losing partial bus state.

Do not synchronize every device at every T-state merely for implementation convenience. Do not use lazy synchronization where it loses observable causal behavior.

### 3.2 Bounded execution horizons

A CPU block must stop before the earliest event that can affect its execution or an observable bus operation that requires catch-up. The bound is a correctness constraint, not a tuning knob that may be increased without tests.

### 3.3 Hot-path reduction

Profile real games; then remove measured costs such as redundant dispatch, repeated address translation, avoidable copies, unnecessary bounds/variant checks and per-frame allocations. Preserve safe Rust unless a specific unsafe optimization has a measured benefit and an explicit safety review.

### 3.4 Precomputation and data locality

Use compile-time/static lookup tables for stable mappings and arithmetic where this reduces measured cost. Keep hot state compact and contiguous. Avoid hash maps, heap allocation and pointer-heavy structures in per-instruction/per-pixel/per-sample loops unless benchmarks justify them.

### 3.5 CPU execution strategy

Start with an optimized instruction interpreter. Benchmark realistic instruction mixes, bus activity and interrupts. Consider decoded-block caching or JIT only after profiling proves the interpreter is the bottleneck and an ADR approves:
- code invalidation for writable/executable memory;
- exits on MMIO, interrupts, WAIT and timing boundaries;
- precise cycle accounting;
- save-state/replay compatibility;
- x86-64 baseline and portable fallback;
- security and maintenance implications.

### 3.6 Audio and video

Use batching, precomputed tile/palette data, scanline work and SIMD only where they preserve observable output and improve measured gameplay. Chip write timestamps, event ordering, raster-visible register writes and serial operator dependencies must remain intact.

## 4. Benchmark acceptance

Each optimization report includes:
- exact commit SHA and build profile;
- fixed machine, OS, CPU, memory and power configuration;
- ROM SHA-256, region, console profile and game scene;
- warm-up and measurement lengths;
- at least repeated A/B runs with order control (ABBA where appropriate);
- p50/p95/p99 and mean frame time or throughput;
- CPU profile/flamegraph;
- allocation and copy observations when relevant;
- output correctness (trace, frame hash/diff, PCM/VGM or gameplay);
- rollback decision if the result is neutral, noisy or negative.

A microbenchmark win is insufficient if whole-game throughput does not improve. A throughput win is rejected if it changes game-visible behavior.

## 5. Licensing boundary

BlastEm is distributed under GPL-3.0-or-later. The current SUN repository declares MIT. Therefore:
- source may be inspected to understand architecture and observable behavior;
- do not copy, translate, mechanically port or adapt BlastEm implementation code;
- independently derive behavior from documentation, tests, traces and hardware observations;
- retain provenance for every external reference;
- request a dedicated license ADR and owner decision before considering any license change.

This document is an engineering policy, not legal advice.

## 6. Oracle gaps

When BlastEm does not support a required platform behavior, has experimental support, or disagrees with another reference:
1. reproduce the difference with a minimal ROM/test;
2. capture traces and outputs from all available references;
3. consult primary hardware documentation;
4. classify the result CONFIRMED / PROBABLE / UNKNOWN;
5. implement the smallest behavior needed by supported games;
6. preserve the unresolved difference in the compatibility ledger.

## 7. Required output

Every hardware PR states: **Oracle revision / ROM hash / first divergence / gameplay impact / performance before-after / unresolved differences.**
