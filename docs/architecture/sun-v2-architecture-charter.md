# SUN V2 — Architecture Charter

- **Status:** PROPOSED — execution direction approved by project owner; becomes normative after PR merge.
- **Created:** 2026-11-22.
- **Repository baseline:** `main` at `30420fde1510d84834ed4e776f1a1bd1a5503b78`.
- **Scope:** architecture, engineering constraints and delivery gates for the Sega 8-bit family, then Mega Drive, Sega CD and 32X.
- **Primary behavioral oracle:** BlastEm. Other emulators are secondary references used to investigate gaps and disagreements.

## 1. Mission

Build a fast, deterministic, game-focused Sega emulator. Hardware fidelity is required only to the extent that behavior is observable and necessary for supported games, cartridges and explicitly supported peripherals. Do not implement chip features merely to complete a datasheet.

The project is not a universal chip laboratory. A feature requires a demonstrated relationship to a supported console/game, a reproducible compatibility defect, or a required platform contract.

## 2. Non-negotiable engineering rules

1. **Performance is a functional requirement.** No component is accepted without a reproducible benchmark and a defined performance budget.
2. **Correctness is observable.** No speed optimization may change game-visible CPU results, bus side effects, interrupt timing, video output, audio behavior or deterministic state.
3. **Measure before changing.** Identify the hot path with a release build and profiler; retain before/after artifacts.
4. **No speculative precision.** Implement only behavior supported by games, conformance ROMs, hardware documentation or reproducible oracle comparisons.
5. **No speculative abstraction.** Share a component only when hardware semantics and ownership genuinely match.
6. **No hot-path allocation or dynamic dispatch without measured justification.** Prefer fixed-capacity/preallocated buffers, static dispatch and contiguous state.
7. **One temporal authority per machine.** MachineScheduler owns causal ordering. Host rendering, audio callbacks, input polling and UI pacing never advance hardware time.
8. **Preserve validated code.** No rewrite or parallel implementation without a defect, measured performance problem, or explicit ADR.
9. **Evidence belongs to an exact commit.** PASS/FAIL/BLOCKED/NOT RUN are distinct; SKIPPED is never PASS.
10. **No game-specific hardware hacks.** Cart metadata may describe actual cartridge hardware; it must not encode game-specific behavioral exceptions.

## 3. Target platform sequence

1. SMS1/SMS2 — current primary 8-bit reference machine.
2. SG-1000 — finish the temporal machine and gameplay acceptance using existing shared components.
3. Game Gear — own machine profile with its display, memory, CRAM, stereo audio and I/O behavior.
4. 8-bit media and accessories — only explicitly supported hardware, after the three machines are stable.
5. Mega Drive/Genesis.
6. Sega CD.
7. 32X.
8. Cross-platform compatibility and product acceptance.

Do not begin 16-bit hardware until the 8-bit family and the approved 8-bit accessory scope are accepted.

## 4. Logical architecture and existing repository mapping

The target is a set of independent machine compositions over carefully selected shared components. It is not a mandate to create a second workspace or duplicate existing crates.

| Responsibility | Existing/target ownership |
|---|---|
| CPU Z80 | Existing shared Z80 implementation; one source of truth |
| Time and causal ordering | Existing MachineScheduler / temporal core; one authority |
| Bus and memory | Machine-specific address maps and bus adapters; common primitives only where semantics match |
| PSG | Existing SN76489 implementation where chip variant and console wiring match |
| SMS VDP | SMS/Mark III VDP profile, including model/revision distinctions where game-visible |
| SG-1000 VDP | TMS9918A-compatible SG-1000 profile; never alias it to the SMS VDP |
| Game Gear video | GG-specific VDP/LCD profile; reuse low-level logic only after semantic equivalence is demonstrated |
| FM | YM2413 only for the supported integrated Japanese SMS profile; optional platform configuration |
| Audio pipeline | Shared PCM/mixer/resampler infrastructure; chip timing remains owned by the machine scheduler |
| Runtime sessions | One machine runtime per console profile, exposed through coarse-grained lifecycle/frame APIs |
| Desktop | Slint + WGPU 30/Vulkan + CPAL 0.18.2 + SDL3 input integration |

The existing Cargo workspace and crates are the starting point. M0 must map each target responsibility to actual crates, modules, tests and consumers. Do not create proposed `sun-*` crates in parallel with existing `native/*` or `rust/runtime/*` crates unless an ADR demonstrates why migration is necessary.

### 4.1 Machine composition

Each console has an explicit machine profile and composition root:

- SMS: Z80 + SMS bus/memory/mapper + SMS VDP + SN76489 + optional integrated YM2413 + SMS I/O.
- SG-1000: shared Z80 + SG-1000 bus/memory/cartridge + TMS9918A profile + SN76489 + SG-1000 I/O.
- Game Gear: shared Z80 + GG bus/memory/cartridge + GG VDP/LCD + stereo audio routing + GG I/O.

A shared chip crate must not contain console-specific policy that belongs in a machine composition. A shared implementation must expose only the operations and state required by its actual consumers.

### 4.2 Temporal model

MachineScheduler is the only authority for machine time and event ordering. M0 audits the current scheduler before changing time representation or dispatch.

Preferred optimization direction, subject to differential validation:

- run a CPU until a safe observable horizon rather than synchronizing every peripheral at every CPU tick;
- catch up a peripheral before a CPU-visible access or at the next event horizon;
- preserve interrupt, DMA, WAIT, bus contention and write/read side effects at every observable boundary;
- use integer clock relationships and bounded event horizons;
- avoid global lockstep when lazy synchronization is provably equivalent;
- never skip an event to gain throughput.

Do not assume a generic `catch_up` interface is sufficient. Every component must document which events it can generate independently and which accesses force synchronization.

Time representation (`u64`, machine master ticks, divisors and wrap policy) must be decided after auditing existing types, overflow invariants and generated code. Do not migrate to a new clock type by aesthetic preference.

## 5. Performance engineering policy

Every component must have:
- a release-mode benchmark representative of real gameplay;
- targets recorded in [SUN V2 performance targets](sun-v2-performance-targets.md), separating SMS 400 FPS, Megazord 100 FPS and the product's 60 FPS real-time requirement;
- a microbenchmark for its measured hot path where useful;
- a profiler capture identifying top costs;
- an allocation/copy/dispatch review;
- a before/after comparison;
- a differential correctness test;
- a stated budget and regression threshold.

Initial CPU strategy — owner-approved direction:
1. Start the SMS implementation at **F2: predecoded instruction/block cache**, retaining the existing interpreter as the semantic executor and fallback.
2. F2 caches decoded metadata; it does not generate native machine code and is not a JIT.
3. Cache validity must account for writes to executable RAM, mapper/bank changes, reset and state restoration. Blocks must exit at every scheduler/event horizon or observable bus boundary.
4. Preserve instruction results, flags, cycle accounting, bus ordering, WAIT/IRQ behavior, suspend/resume and deterministic save-state/replay semantics.
5. **F3: dynamic native-code compilation** is the contingency if the validated F2 SMS benchmark misses its 400 FPS throughput target and profiling confirms CPU decode/dispatch/execution is a material bottleneck. If another subsystem is the bottleneck, fix that subsystem rather than adding a JIT that cannot address it.
6. F3 must retain the interpreter as fallback/oracle and be isolated behind a feature/architecture boundary. Its implementation requires a separate task specification and differential validation.
7. The detailed decision and acceptance protocol are recorded in [ADR-0017](adr/0017-f2-decoded-block-cache-and-f3-contingency.md).

No global fast-math, clock approximation, frame skipping, reduced hardware semantics, target-cpu=native distribution, or unvalidated SIMD.

Audio may be processed in blocks only when timestamped writes and chip state transitions remain equivalent. Do not replace chip clocks with arbitrary sample-rate updates. Preserve serial dependencies in FM operator pipelines.

## 6. Oracle and evidence hierarchy

1. **Primary behavioral oracle:** BlastEm, pinned to a specific source revision/build configuration for every campaign.
2. **Secondary emulators:** Ares, Gearsystem, Genesis Plus GX, MAME and other maintained implementations, selected per subsystem and platform.
3. **Conformance ROMs and hardware documentation:** preferred for resolving disagreement.
4. **Physical hardware captures/tests:** strongest evidence when the setup and method are reproducible.

When references disagree, record the exact ROM/hash, region/model, emulator revision, settings, trace/frame/audio artifact and first divergence. Do not choose a winner by popularity. A hardware test or authoritative conformance test may override BlastEm.

BlastEm source is GPL-3.0-or-later. The SUN's current repository license is MIT. Use BlastEm as a behavioral and engineering reference; do not translate/copy its implementation code into SUN. Any proposal to change project licensing requires an explicit owner-approved legal ADR.

## 7. Definition of done for a hardware change

A change is not complete because it compiles. Its PR must include:
- scope and affected machine profiles;
- hardware evidence tagged CONFIRMED / PROBABLE / UNKNOWN;
- tests written or updated before/with implementation;
- differential result against BlastEm or a documented reason it is unavailable;
- relevant secondary reference when BlastEm has a known gap;
- gameplay corpus result;
- release benchmark before/after;
- profiler evidence for performance work;
- affected frontend/runtime integration validation;
- known limitations and exact tested commit SHA.

## 8. M0 exit gates

- Repository and documentation authority reconciled.
- Existing code classified KEEP / REFACTOR / ORACLE / REMOVE, with evidence and consumer map.
- Existing performance baseline reproduced on the reference Windows x64 machine.
- BlastEm source revision and build configuration pinned.
- First independent differential harness produces a useful report for one 8-bit ROM/test.
- Machine/clock/event ownership diagram approved.
- Shared-vs-machine-specific component matrix approved.
- Agent task contracts and PR gates published.
- No production hardware rewritten during M0.

## 9. Explicit non-goals

- Universal Z80/VDP/PSG/FM compliance beyond supported game needs.
- SC-3000 as a platform.
- Copying or translating BlastEm source.
- JIT before the F2 benchmark gate and profiling evidence;
- Parallel CPUs, schedulers, workers or duplicate pipelines for convenience.
- Rewriting already validated subsystems without evidence.
- Implementing debugger-only chip behavior as a prerequisite for gameplay.

## 10. References

- Project master prompt: `GUIA_PROJETO_SUN_V2.md` (project attachment; proposal to reconcile with repository reality).
- Repository rules: `AGENTS.md`.
- Current platform scope: `docs/project-scope.md`.
- Current roadmap: `docs/project-roadmap.md`.
- Current component status: `docs/component-status.md`.
- BlastEm reference policy: `docs/architecture/blastem-reference-and-performance-policy.md`.
- Agent workflow: `docs/ai/agent-roles-and-workflow.md`.
