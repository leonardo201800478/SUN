# Rust workspace

O workspace Rust na raiz do repositório é a autoridade de build atual. Os crates
de hardware permanecem em `native/`; os runtimes de produto ficam em
`rust/runtime/`. A migração física de todos os crates para uma única árvore
ainda não foi concluída.

## Layout ativo

```text
native/
├── sg1000_core/          # SG-1000 temporal core, shared Z80 + MachineScheduler
├── sms_core/             # SMS temporal/atomic modules under staged consolidation
├── z80_cpu/              # shared Z80 micro-sequencer and architectural state
├── machine_scheduler/    # sole machine-time authority
├── audio_runtime/        # shared DSP, PCM and AudioSession contract
├── video_runtime/        # FrameMailbox and immutable VideoFrame
├── audio_output/         # CPAL host backend (standalone workspace)
└── sega_desktop/         # Slint 1.18 + WGPU 30/Vulkan host

rust/runtime/
├── sms-runtime/          # SMS worker and session lifecycle
└── sg1000-runtime/       # SG-1000 worker candidate; desktop handoff follows
```

## Product boundary

- Hardware, CPU execution, clocks, Scheduler, VDP, PSG, memory and bus remain in Rust.
- The shared Z80 micro-sequencer is the only SG-1000 CPU path in product builds.
- The former atomic SG-1000 CPU/ALU/bus/machine is compiled only under `cfg(test)`
  as a functional regression oracle.
- `Sg1000TemporalWorker` owns the SG-1000 machine on a dedicated Rust thread.
  The host communicates through coarse-grained commands, immutable frames and
  the shared `AudioSession` contract.
- Slint owns presentation and operating-system integration only. WGPU/Vulkan
  presents frames; CPAL consumes PCM. Neither changes emulated time.
- No Python implementation, PyO3 binding, Python test runner or Python package
  is part of the active SG-1000 runtime path.

## Migration rules

1. Keep the root Cargo workspace authoritative while native crates are moved
   incrementally; update paths, lockfiles and CI together.
2. Do not duplicate the Z80, MachineScheduler, PSG, VDP, FrameMailbox or PCM
   output pipeline for a new console.
3. Machine workers own mutable machine state and run on dedicated Rust threads.
   UI and host callbacks submit coarse-grained snapshots/commands only.
4. Preserve the legacy atomic SG-1000 implementation only as a test oracle;
   do not expose it as a second runtime.
5. Save states must bind to ROM and hardware profile and preserve all resumable
   Scheduler, CPU, bus, VDP, PSG and input state.
6. Separate headless Rust CI from physical Vulkan/CPAL acceptance. A successful
   build is not evidence of physical presentation or audible output.

## Current status

- SMS: `SmsTemporalWorker` remains the active SMS worker.
- SG-1000 P2.2: temporal machine, shared `Z80MicroSequencer`, M1/WAIT scheduling,
  TMS9918A, PSG, I/O and composite in-memory snapshots are integrated in main
  by PR #988.
- SG-1000 P2.3: dedicated worker, FrameMailbox publication, shared audio session,
  controller snapshots and ROM-bound save-state envelope are under validation
  in the current implementation branch.
- SG-1000 P2.4: Slint system selection, ROM routing and CPAL/WGPU host handoff
  remain pending.
- Remaining workspace consolidation: SMS temporal/atomic split, standalone
  audio-output workspace boundary, and legacy Python/Qt/PyO3 tooling inventory.
