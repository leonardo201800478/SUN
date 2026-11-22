# PERF-01 / Z80 — steady-state CPU profiling

## Purpose

Capture separate Windows ETW CPU-sampling traces for the scheduler-driven SMS
runtime using the same OutRun ROM and board configurations as PERF-01/S0.
This is diagnostic profiling, not a throughput headline benchmark. It uses the
release-llvm-debug profile to retain symbols. Use perf-baseline-s0.ps1 with the
stock release profile for uncapped throughput measurements.

## Capture

Run elevated PowerShell from a clean checkout:

    .\scripts\benchmark\perf-z80-cpu-profile.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms"

The harness builds before ETW starts and captures three independent traces:
A = SMS without YM2413; B = Japanese SMS with YM2413 clocked and FM muted;
C = Japanese SMS with YM2413 clocked and FM mixed into the native headless sink.

For each scenario, the worker starts and warms up first. A ready/go file
handshake starts WPR only after warmup. Diagnostic telemetry is disabled during
capture. Host pacing and WGPU presentation are disabled; the MachineScheduler
and all emulated hardware clocks remain active.

Artifacts are written to Documents\SUN-Benchmarks\perf-01-z80\<timestamp>:
manifest.json, build.log, per-scenario stdout/stderr logs and three ETL files.
The manifest records the Git commit, ROM SHA-256, toolchain, target, CPU, OS
and capture conditions.

## WPA analysis

Open each ETL in Windows Performance Analyzer. In CPU Usage (Sampled):

1. Filter to the perf_z80_profile test process; exclude Cargo and system noise.
2. Group by Process, Thread, Stack and Function/Module.
3. Inspect Call Tree exclusive samples for hotspot ranking. Inclusive samples
   include descendants; do not sum nested inclusive percentages.
4. Export the filtered table for A, B and C and retain the WPA profile setup.

Classify symbols into:
- Z80: sega_z80_cpu sequencer, prefix and ALU handlers.
- SMS temporal machine: sega_sms_core begin/resume/dispatch paths.
- Scheduler and temporal bus: event queue, M1, bus transaction/completion.
- VDP and interrupt synchronization.
- YM2413, PSG, mixer and resampler.
- Worker/host overhead.

Compare A→B to estimate the installed/clocked FM hardware delta, and B→C to
estimate the additional software FM mix-path delta. These are controlled
contrasts, not proof that every difference belongs exclusively to one chip.

## Guardrails

- ETL samples do not directly provide instructions/s or M-cycles/s.
- release-llvm-debug is for attribution only; do not report its throughput as
  the stock Release headline result.
- Do not claim an FPS gain from profiling alone. Repeat uninstrumented S0
  throughput after a future optimization.
- Do not alter Scheduler ownership, M1, WAIT, IRQ/NMI, HALT or bus ordering.

No Z80 runtime optimization is included in this change. A bottleneck can only
be declared after these ETL files are captured and analyzed on the target
Windows x64 machine with the legally obtained OutRun ROM.
