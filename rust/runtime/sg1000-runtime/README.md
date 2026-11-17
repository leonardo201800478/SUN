# SG-1000 native runtime

This crate provides the Rust-owned execution worker for the temporal SG-1000
machine in `sega-sg1000-core`.

## Ownership and boundaries

- `Sg1000TemporalWorker` owns `Sg1000Machine` on a dedicated Rust thread.
- The machine's shared `Z80MicroSequencer` and `MachineScheduler` remain the
  only CPU and time authorities.
- Host commands are coarse-grained: start, pause, stop, reset, load ROM,
  step frame, controller snapshot, NMI, sprite-limit setting and save-state
  capture/restore.
- Completed 256×192 RGB888 frames are published through the externally owned
  `FrameMailbox`; the worker never owns WGPU devices, textures or windows.
- Audio samples are passed to the shared `AudioSession` contract. The host can
  construct a CPAL-backed session inside the worker using
  `configure_audio_output`; CPAL callbacks do not access the machine or
  MachineScheduler.
- Save states are versioned in-memory snapshots bound to the exact ROM and
  `MemoryConfig`. File encoding and slot management belong to the host.

## Current limits

- The first worker profile uses the SG-1000 baseline memory map and the
  provisional 3.579545 MHz CPU/PSG clock profile already defined by the core.
- The worker does not add cartridge mappers, BIOS boot, SG-1000 desktop menu
  routing or physical-device acceptance. Those are separate follow-up gates.
- The legacy atomic SG-1000 machine is test-only and is not used by this crate.

The desktop host has not yet selected this worker. The next integration phase
will add SG-1000 ROM routing to Slint and connect its frames/audio through the
existing WGPU/CPAL host paths without moving hardware logic into the GUI.
