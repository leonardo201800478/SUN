# PERF-Z80 / Linux synthetic frame baseline

## Objective

Provide a repeatable Linux x64 CI measurement while the target Windows PC and
the OutRun ROM are unavailable. The benchmark compares the stock Cargo
`release` profile with the existing `release-llvm` ThinLTO candidate on the
same GitHub-hosted runner, using an ABBA order to reduce simple time-drift bias.

## Workload

The benchmark builds a deterministic 16 KiB synthetic ROM containing a loop
with M1 opcode fetches, register arithmetic, memory writes and a conditional
relative branch. It executes complete frames through the production
`SmsTemporalMachine::step_frame` path.

The measured interval therefore includes the scheduler-driven Z80 sequencer,
SMS temporal bus and VDP frame progression. It deliberately excludes the SMS
worker thread, YM2413 mixing, host audio output, WGPU presentation and CPAL.

## Metrics

Each trial reports elapsed time, milliseconds per completed VDP frame,
uncapped frames per second and retired instructions per frame. The summary
reports the median and min/max frame time across five independent trials.
Warmup and measured frame counts are fixed by the workflow.

**Important:** `ms_per_frame` is the wall time of the scheduler-driven SMS
machine frame step, not exclusive Z80 function time. It is a useful comparative
proxy for the CPU-driven frame path, but it must not be described as an
isolated Z80 CPU profile. Function-level attribution remains the purpose of
the separate Windows ETW/WPA harness in PERF-01.

## Interpretation and acceptance

- Compare profiles only within the same workflow run and runner class.
- Treat the first successful run as a synthetic baseline; do not claim a 3x
  gain from one noisy run.
- A candidate is accepted only after repeated runs show a stable improvement
  in median frame time without a material increase in dispersion.
- The same workload and compiler profile must be used before and after each
  source optimization.
- A synthetic NOP/ALU/memory loop is not a substitute for real-game validation.
  OutRun A/B/C and physical Windows acceptance remain required when available.
- This benchmark does not authorize changes to Scheduler ownership, M1,
  WAIT states, interrupt boundaries, HALT behavior or emulated instruction
  semantics.

## Invocation

```bash
cargo bench --no-run --locked -p sega-sms-core --bench z80_frame_throughput --profile release
cargo bench --locked -p sega-sms-core --bench z80_frame_throughput --profile release
```

The `Z80 Synthetic Frame Benchmark` workflow runs both profiles and uploads
the raw output as a 14-day artifact.

## Initial CI evidence — 2026-11-22

Revision: `2f9c42146ee8c0390f7e4e179dd05a2da8f3fb26`

Workflow run: [45057308059](https://github.com/leonardo201800478/SUN/actions/runs/45057308059)

Runner: `ubuntu-24.04`; pinned compiler: Rust 1.99.1.

| ABBA block | Profile | Median ms/frame | Median FPS |
|---|---|---:|---:|
| A1 | `release` | 8.914969 | 112.171 |
| B1 | `release-llvm` | 8.543381 | 117.050 |
| B2 | `release-llvm` | 8.470081 | 118.063 |
| A2 | `release` | 8.904620 | 112.301 |

The mean of the two block medians is 8.909795 ms/frame for `release`
and 8.506731 ms/frame for `release-llvm`, a 4.52% reduction in this
synthetic workload. Both profiles retired 10,199.67 instructions per frame.

This result is evidence only for the synthetic scheduler-driven workload on
that CI run. It is not a 4.52% game-wide or Windows performance claim, and it
does not establish exclusive Z80 CPU time. Absolute timings from different
GitHub-hosted runner instances must not be compared as if they were the same
hardware. The `release-llvm` profile remains experimental; the default
Release profile is unchanged.
