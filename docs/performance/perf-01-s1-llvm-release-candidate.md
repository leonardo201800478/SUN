# PERF-01 / S1 — LLVM Release candidate

## Intent

Introduce an isolated Cargo profile that exercises rustc/LLVM optimization and ThinLTO while preserving the existing **release** profile as the A/B control. This is a candidate configuration, not yet the production default.

## Candidate profiles

- **release-llvm**: inherits Cargo **release**, explicitly sets **opt-level = 3**, **lto = "thin"** and **codegen-units = 16**.
- **release-llvm-debug**: inherits the candidate and retains full debug information (**debug = 2**, **strip = "none"**) for ETW/WPA symbol attribution.

The current Cargo defaults already use **opt-level = 3** and **codegen-units = 16**; the first candidate isolates the material change (ThinLTO) and makes the selected codegen-unit count explicit. Do not assume this is the final winner. Fat LTO and alternative codegen-unit counts remain comparative experiments, not defaults.

## Build and compare on Windows x64

Keep the same commit, ROM SHA-256, target triple, game configuration and benchmark protocol. Run the stock profile first:

~~~powershell
.\scripts\benchmark\perf-baseline-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (World).sms" -CargoProfile release
~~~

Then run the candidate:

~~~powershell
.\scripts\benchmark\perf-baseline-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (World).sms" -CargoProfile release-llvm
~~~

For symbolized CPU sampling, use the diagnostic profile separately:

~~~powershell
cargo build --profile release-llvm-debug --locked --target x86_64-pc-windows-msvc
~~~

The diagnostic profile is for WPA/ETW attribution, not the headline throughput result. Its extra debug information can change code layout and performance.

The benchmark manifest records the selected Cargo profile. The verbose Cargo log records the actual rustc invocation and effective LLVM flags. Use separate output directories/campaigns and compare repeated runs; do not combine instrumented diagnostic timings with uninstrumented throughput.

## Acceptance gates

1. Confirm both profiles build on the pinned Rust 1.99.1 toolchain and Windows x64 target.
2. Compare stock **release** and **release-llvm** using repeated A/B/ABBA campaigns and report median/dispersion plus p50/p95/p99 frame-processing distributions.
3. Preserve the exact ROM SHA-256, commit, target and machine configuration.
4. Run the relevant SMS runtime tests and desktop Windows Release validation.
5. Reject the candidate if it causes functional, deterministic, audio or stability regressions, regardless of throughput.
6. Adopt the candidate as the default only after reproducible improvement is demonstrated and the PERF-01 S0/S1 gates are reviewed.

## Constraints

- No **target-cpu=native** in distributable builds.
- No global fast-math, reassociation or flush-to-zero.
- No changes to Scheduler ownership, CPU/VDP/audio timing, YM2413 sequencing, Z80 semantics or hardware behavior.
- No PGO or SIMD in this first change; those remain later PERF-01 stages.
- No dependency updates as part of this profile experiment.
