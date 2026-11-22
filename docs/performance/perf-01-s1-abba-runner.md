# PERF-01/S1 — Repeated A/B validation with ABBA

## Purpose

The ABBA runner executes four uncapped OutRun campaigns in this fixed order:

1. A1 — stock Cargo `release`
2. B1 — candidate `release-llvm` (ThinLTO)
3. B2 — candidate `release-llvm` (ThinLTO)
4. A2 — stock Cargo `release`

The reversed second half reduces monotonic bias from CPU temperature, boost behavior, and background load. It does not replace additional independent campaigns when variance remains high.

## Command

Run from a clean Windows x64 checkout of the SUN repository:

```powershell
.\scripts\benchmark\perf-abba-s1.ps1 `
  -RomPath "D:\ROMS\Sega Master System\OutRun (World).sms"
```

Optional controls:

- `-WarmupSeconds 5`
- `-CaptureSeconds 30`
- `-DiagnosticSeconds 5`
- `-OutputDirectory "C:\path\to\results"`
- `-CaptureCpuProfile` to capture a separate WPR CPU ETL for each run (requires WPR and elevation as appropriate).

## Build reuse

The runner uses two separate Cargo target directories under the campaign output:

- `cargo-target\release-control`
- `cargo-target\release-llvm`

The S0 harness accepts `-CargoTargetDirectory` so the second occurrence of each profile reuses its already-built artifacts. The two profiles remain isolated from each other. This avoids four full clean builds while preserving the compiler profile separation.

Do not run `cargo clean` between the four steps. The harness still verifies a clean Git working tree and records the same commit and ROM SHA-256 for every run.

## Outputs

The campaign directory contains:

- `abba-manifest.json`: commit, ROM hash, machine/toolchain metadata, sequence, per-run artifact paths, completion status.
- `abba-results.csv`: all twelve scenario observations with sequence and profile labels.
- `abba-summary.csv`: per-scenario/per-profile mean, sample standard deviation, min/max, and ThinLTO mean gain relative to Release.
- `abba.log`: combined console output.
- `run-XX-*/`: original S0 manifest, CSV, log, and optional ETL for each individual run.
- `cargo-target\`: isolated reusable Cargo artifacts by profile.

A failed step stops the sequence and leaves an incomplete campaign manifest with the failure reason. Partial data is retained and must not be treated as a completed ABBA result.

## Interpretation

Compare the per-profile means and sample standard deviations for scenarios A, B, and C. Scenario C (YM2413 clocked and FM mixed) is the primary PERF-01 throughput criterion. The benchmark remains headless and uncapped; it does not measure CPAL physical output or WGPU presentation.

An ABBA result is evidence for this host, toolchain, commit, ROM, and configuration only. Do not promote ThinLTO to the default profile or claim the 120 frames/s gate is met without the broader validation required by PERF-01.
