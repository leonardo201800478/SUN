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
