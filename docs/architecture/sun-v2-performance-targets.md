# SUN V2 — Performance targets and acceptance protocol

- **Status:** APPROVED BY OWNER — effective; published in PR #1254.
- **Scope:** 8-bit SMS performance and the future integrated Mega Drive + Sega CD + 32X Megazord.
- **Principle:** throughput targets provide engineering headroom; stable real-time gameplay remains the product requirement.

## 1. Targets

| Machine | Uncapped emulation-throughput target | Functional product requirement |
|---|---:|---|
| SMS, with YM2413/FM actively processed and mixed | **>=400 emulated frames/s** | Stable 60 FPS real-time execution |
| Megazord: Mega Drive + Sega CD + 32X operating together and synchronized | **>=100 emulated frames/s** | Stable 60 FPS real-time execution |

These throughput targets are engineering goals, not presentation refresh rates, frame caps, or promises that every low-end device will reach the same uncapped number.

The 60 FPS requirement is separate: the integrated product must sustain the console's real-time cadence without recurring missed frames, audio underruns, drift or causal-order violations. The target device class and minimum reference hardware must be recorded before claiming support for "modest devices".

## 2. Existing performance evidence and its limits

The latest available historical ABBA campaign is tied to commit `331fc7e97bf67964b0db6ad84ed7e27d171705d3`, not the current main HEAD. Its ThinLTO FM-mixed scenario measured 56.7438 guest frames/s. A separate diagnostic window reported approximately 5.412 ms/frame around `SmsTemporalBus::advance_ym2413`, against approximately 19.162 ms total frame processing.

This is evidence that the FM temporal-advance path is a significant measured cost, but it is not a complete function-level attribution of the remaining frame time and is not a current-main baseline. Re-run the same campaign on the current HEAD and obtain a CPU profile before deciding how much of the 400 FPS gap can be addressed by F2 or F3.

## 3. SMS benchmark definition

The primary SMS throughput campaign must:
- use a fixed, identified ROM and SHA-256, console profile/revision and region;
- keep YM2413 enabled, clocked, generating and mixing FM output together with SN76489; muting the host output is not equivalent to disabling chip processing;
- use a repeatable gameplay scene and fixed emulation configuration;
- measure completed guest frames per wall-clock second with host pacing/vsync disabled;
- separate core throughput from Slint/WGPU presentation and physical CPAL output;
- include an integrated run with the desktop and physical audio path as a separate acceptance gate.

Use the existing PERF-01 corpus as the starting point: OutRun (World) as the primary candidate, Kenseiden and Phantasy Star Japan Rev 1 as secondary continuity/correctness cases. Reconfirm each title's FM relevance and exact configuration before treating it as a full-FM workload.

## 4. Megazord benchmark definition

The Megazord campaign begins only after the combined machine exists. All hardware required by the selected game must run in one causally synchronized machine:
- Mega Drive main CPU, Z80, VDP and audio;
- Sega CD subsystem, including its required CPUs, CD/media and audio paths;
- 32X Master/Slave SH-2, video composition and required buses;
- shared buses, arbitration, DMA, interrupts and cross-system communication.

Do not add isolated emulator FPS values, sum per-chip throughput, or benchmark the three systems independently and call it Megazord performance. Select a reproducible game/scene that exercises the combined hardware, identify its ROM/disc/media hashes and configuration, and measure the composed machine end to end.

## 5. Ares comparison protocol

Ares is a comparative reference, not a substitute for a defined SUN benchmark. A comparison is valid only when the workload, console profile, region, game/scene, host machine, pacing policy and measured quantity are equivalent.

- Normalize the metric to **guest frames completed per wall-clock second** whenever both systems expose enough information.
- Do not treat Ares VPS and SUN FPS as interchangeable labels without verifying their definitions and measurement windows.
- Preserve raw outputs and report the exact emulator versions/builds, host CPU/OS, ROM/media hashes, settings, warm-up and measurement duration.
- If exact equivalence cannot be established, report the results side by side as non-equivalent observations, not as a speedup ratio.

## 6. Measurement and reporting

Every performance report must include:
- exact SUN commit SHA, build profile, compiler/toolchain and target;
- host CPU, OS, memory, power mode and relevant background-load conditions;
- ROM/media SHA-256, console profile/revision, region, game scene and enabled hardware;
- warm-up and measurement durations, repeated runs and run ordering (ABBA where practical);
- completed guest frames, elapsed time, throughput, p50/p95/p99 frame time, mean, standard deviation and maximum;
- separate core-only, integrated desktop and physical-output results;
- profiler/flamegraph for the measured hot paths;
- allocations, copies, queue/FIFO occupancy and audio underruns where relevant;
- frame/audio/determinism checks and first divergence against the selected oracle/reference.

Use Release builds. Do not use host pacing to inflate or cap the throughput measurement. Instrumented profiler runs and uninstrumented throughput runs must be reported separately.

## 7. F2 and F3 performance gate

F2 starts on SMS using the existing Rust interpreter plus a predecoded instruction/block cache. The acceptance target is >=400 guest frames/s in the defined full-FM workload.

If F2 misses the target:
1. verify the benchmark and compare it with the no-cache interpreter baseline;
2. inspect the profile and determine whether CPU decode/dispatch/execution is a material limiter;
3. if it is, proceed to F3 (dynamic native-code compilation) under ADR-0017, with interpreter fallback and differential tests;
4. if the dominant limiter is YM2413, Scheduler, VDP, mixer, memory/bus or host integration, fix that measured limiter first. JIT does not accelerate unrelated subsystems.

The later Megazord target is >=100 guest frames/s for the complete combined machine. Its performance campaign must profile the whole composition; it does not inherit the SMS benchmark or assume a CPU-only JIT will solve every bottleneck.

## 8. Acceptance rule

A throughput target is accepted only when the exact workload and measurement are reproducible and the optimization preserves gameplay, CPU/bus semantics, video, audio, deterministic state and temporal causality.

A throughput target does not replace the real-time product gate. Both must pass:
- **Headroom gate:** SMS >=400 frames/s with FM active; future Megazord >=100 frames/s with all required subsystems active.
- **Product gate:** stable 60 FPS real-time execution on the declared modest reference device, with integrated video/audio/input and no recurring underruns or timing drift.

No speed target authorizes frame skipping, inaccurate clocks, omitted FM processing, disabled peripherals required by the game, reduced fidelity or game-specific hacks.
