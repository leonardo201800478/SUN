# SUN PERF-01 / S0 — OutRun baseline harness

## Purpose

This harness establishes an uncapped, reproducible runtime-throughput baseline before changing Cargo Release settings, LLVM flags, LTO, PGO or hardware hot paths.

It uses one legally obtained OutRun SMS ROM, identified by SHA-256, in three SMS board/audio configurations:

| Scenario | Physical machine | FM output |
|---|---|---|
| A — no YM2413 | Generic SMS board; no YM2413 instance | Not applicable |
| B — clocked/muted | Japanese SMS board with YM2413 installed and clocked | Muted before the host mixer |
| C — FM mixed | Japanese SMS board with YM2413 installed and clocked | Enabled into the native mixer/resampler |

Muting FM output does not stop YM2413 clocks or change register state. The MachineScheduler remains the only authority for virtual hardware time.

## What the throughput number means

The throughput pass disables only host frame pacing. It does not modify Scheduler clocks, CPU cycles, WAIT states, VDP timing or audio-chip timing.

The test does not attach a WGPU presentation mailbox. It configures the production native mixer/resampler with a deterministic discard sink so that scenario C exercises the software FM mix path without requiring a physical audio device.

Consequently:

- `throughput_fps` is completed emulated frames per wall-clock second for the headless worker path.
- It is not monitor scanout FPS, displayed FPS or a CPAL playback rate.
- The discard sink proves the software audio path receives PCM; it does not prove that physical audio was heard.
- CPAL callback behavior, underruns, FIFO occupancy and actual audible continuity must be measured separately.
- The test starts the ROM from reset and does not yet automate an interactive OutRun race sequence. Treat this as the initial ROM-runtime baseline, not final gameplay acceptance.

## Execution on Windows x64

Use the same OutRun ROM file and SHA-256 for all three scenarios. The filename must contain OutRun or outrun.

From the repository root:

```powershell
.\scripts\benchmark\perf-baseline-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms"
```

Defaults:

- Warmup: 5 seconds per scenario.
- Throughput capture: 30 seconds per scenario.
- Separate diagnostics pass: 5 seconds per scenario.
- Region: Japan / NTSC.
- VDP: SMS1 / Sega 315-5124.
- Frame pacing: disabled.
- WGPU presentation: disabled.
- Audio: native mixer/resampler with headless discard sink.

For a CPU sampling trace, run PowerShell as Administrator:

```powershell
.\scripts\benchmark\perf-baseline-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms" -CaptureCpuProfile
```

The optional WPR CPU trace is written as cpu-profile.etl. The script also captures manifest.json, outrun-a-b-c.csv and benchmark.log in a timestamped directory under Documents\SUN-Benchmarks\perf-01-s0\.

For a shorter smoke run, use -WarmupSeconds 1 -CaptureSeconds 5 -DiagnosticSeconds 1; do not use that shortened run as the final baseline.

The underlying opt-in test can also be invoked directly:

```powershell
$env:SEGA_PERF_ROM_PATH = "D:\ROMS\Sega Master System\OutRun (Japan).sms"
cargo test --release --locked -p sega-sms-runtime --test perf_baseline outrun_a_b_c_uncapped_baseline -- --ignored --nocapture
```

## Recorded evidence

The manifest records repository commit, branch, working-tree state, ROM filename and SHA-256, region, VDP, scenario order, durations, host CPU, OS and Rust/Cargo versions.

The CSV records, per scenario:

- Completed frames.
- Wall-clock capture duration.
- Uncapped throughput in frames per second.
- PCM sample count submitted to the headless sink.
- Committed YM2413 address/data write counters.

The diagnostics pass reports the runtime's existing rolling p50/p95/p99 values for frame interval, worker step, total frame processing, video publication and audio submission. This pass is separate from the throughput pass because diagnostic instrumentation adds overhead.

## Interpretation and next gaps

Do not compare the headless throughput value directly to the historical 84 FPS / 43–52 FPS observations until the exact capture conditions are reconciled. The previously reported Gearsystem 123.49 FPS and Ares 395 VPS are observational references, not controlled equivalent runs.

S0 remains open until:

1. OutRun A/B/C results are captured on the target Windows x64 machine and artifacts are preserved.
2. A CPU sampling profile is available and hotspots are identified.
3. Stage distributions include p50, p95, p99, mean, standard deviation and maximum for the measured window.
4. YM2413 advancement cost is separately quantified, without moving its clock away from the Scheduler.
5. CPAL physical output, FIFO occupancy, underruns/drops and audible continuity are measured separately.
6. The same run protocol is repeated sufficiently to establish variance.

The present harness reports p50/p95/p99 for the worker's bounded 512-sample diagnostic window. It does not yet emit mean, standard deviation or maximum for each stage, nor a standalone YM2413 advancement distribution. Those are explicit remaining S0 tasks; they must not be inferred from throughput.

## Safety constraints

- Do not add target-cpu=native to distributable builds.
- Do not enable global fast-math.
- Do not alter hardware timing or Scheduler ownership to increase throughput.
- Do not modify Release settings until the unmodified baseline artifacts are preserved.
- Do not advance to S1 until the S0 evidence and remaining distributions are reviewed.
