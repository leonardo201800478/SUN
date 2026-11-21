# YM2413 FM-4/FM-5 Integration Plan

**Date:** 2026-11-21  
**Base `main`:** `94949c680e973c889580588686503c575dbc5c5b` (scorecard PR #1151 merged)  
**YM2413 implementation baseline:** `624bb3dc956080b18b5b924f94f3ab4ce49c83e4` (FM-3A merge)  
**Working branch:** `feat/ym2413-fm4-fm5-slot-dispatcher`

This document defines the bounded engineering scope for FM-4 and FM-5. It does not authorize SMS bus integration, PCM/mixer work, or changes to the already validated PG/EG/operator semantics without differential evidence.

## 1. Current implementation facts

- `Ym2413Core` owns the 64-byte register image, delayed address/data write pipeline, serial EG, current zero-based slot index, input-clock phase and completed rotations.
- The Rust timing contract currently uses 4 input clocks per slot and 18 slots per rotation (72 input clocks per complete rotation). This is the repository's current core contract; it must not be restated as one slot per 72 input clocks.
- The core already exposes an internal transactional `Ym2413SlotService` boundary: `before_eg_update`, exactly one core-owned EG update, then `after_eg_update`. Service and core are cloned/staged and committed together.
- `advance_input_clocks_with_pg_service` is the current public integrated path. Its service owns PG pipeline latches, the synthesis bank, serial operator and serial AM LFO.
- `Ym2413PgSerialService` currently rejects Rhythm Mode in both pre-EG and post-EG parameter-latch paths. Its documented order is pre-EG `PreparePatch1 -> Channel -> PhaseGenerate -> Operator -> PhaseCalcIncrement -> EnvelopeOutput/KSL-TL`, then the core EG update, then post-EG `DoLFO -> DoRhythm -> PreparePatch2`. Rhythm is explicitly rejected at the DoRhythm boundary.
- `YM2413_SLOT_MAP` already describes all 18 zero-based slot positions. Slots 0..11 cover the six ordinary melodic channels; slots 12..17 are the channel 7..9 operator positions that are reassigned when Rhythm Mode is active.
- `channel.rs` already decodes nine melodic channel register sets and resolves a channel's patch and static parameters. Instrument code 0 selects the live user patch stored in registers `0x00..=0x07`; codes 1..15 select the repository's candidate preset bank. The eight user-patch register bytes are not eight instrument IDs.
- `decode_envelope_slot_input_for_cycle` already diverts cycles 11..16 to rhythm EG inputs when Rhythm Mode is enabled. The PG/operator service does not yet implement the matching complete rhythm path.
- `rhythm.rs`, `rhythm_channel.rs`, `rhythm_phase.rs`, `rhythm_output.rs` provide independent rhythm configuration, phase/noise, voice evaluation and delayed output components. Their comments correctly retain unknown physical seed/reset/cadence and candidate patch status.
- `rhythm_output.rs` models a constrained MO/RO measurement-domain level and slot retention. `Ym2413DacCode` is not a raw physical DAC word and must not be silently treated as PCM.
- The pinned Nuked adapter emits one trace row per `OPLL_Clock` call. Its 18-call serial software traversal is not automatically equivalent to 18 physical XIN clocks. The oracle README explicitly says the 72-XIN-clock rotation comes from separate hardware timing evidence and must be mapped independently before combining timebases.

## 2. Timing terminology that must remain unambiguous

Use three separate terms in code, tests and documentation:

1. **Input clock:** one clock supplied to `Ym2413Core::advance_input_clocks`.
2. **Rust slot boundary:** reached after four input clocks in the current core contract; one of 18 slot indices is serviced.
3. **Nuked oracle call:** one `OPLL_Clock` invocation and one step in Nuked's 18-step software serial traversal.

Do not equate these units merely because both models expose an 18-position serial index. Establish and test the mapping from the pinned Nuked trace to Rust input-clock boundaries. Do not infer four physical subphases inside a Rust slot.

The user-proposed `CLK/72` notation may describe the complete rotation/output cadence, not the rate at which an individual Rust slot advances. Hardware frequency and output-sample rate must be sourced and reconciled separately before they become constants.

## 3. FM-4 — nine melodic channels

### Required work

- Preserve the existing 64-register delayed-write pipeline and its exact application order.
- Resolve the channel/operator descriptor for every serial slot using the existing table, not a second hand-maintained channel map.
- Resolve F-number, block, KEY-ON, KEY-OFF, sustain, instrument code and carrier volume from the register image visible at that serial boundary.
- Preserve the distinction between the custom user patch bytes (`0x00..=0x07`) and the instrument selector value 0.
- Preserve parameter-latch latency: a register write must not retroactively change an operator already evaluated in the current slot.
- Ensure each channel's modulator and carrier update their own PG/EG/operator/feedback state in the correct serial order.
- Keep preset data explicitly candidate until its provenance/hardware status is independently established.

### Required tests

- All nine channels, all 18 operator positions, and exact slot-to-channel/operator mapping.
- Custom patch and each instrument selector class, including invalid/edge selector handling.
- F-number low/high, block, KEY-ON/OFF, sustain, instrument and volume writes around slot boundaries.
- Writes queued before, during and after the relevant channel/operator slots; prove the old/new values are consumed on the correct cycle.
- Continuous advancement versus arbitrarily partitioned input-clock blocks.
- Snapshot/restore and retry after a deliberately injected service failure.

## 4. FM-5 — unified deterministic 18-slot dispatcher

### Required work

- Reuse the existing core-owned slot-service boundary. Do not add a second slot clock, duplicate EG sequencer, or host-side per-slot callbacks.
- Extend/refactor the current PG serial service so one transactional service owns the complete FM serial state: PG latches/accumulators, operator/feedback, AM/PM state, rhythm LFSR/sequencer, rhythm voice state and MO/RO retention state as applicable.
- Keep the core as the sole owner of the serial EG update. The service must consume the core's pre/post EG snapshots and must never tick a second EG.
- Implement the full pre-EG and post-EG order from the pinned Nuked source trace. In particular, Rhythm Mode work belongs at the documented post-EG DoRhythm position; do not move it to a convenient slot callback.
- Handle the mode transition and channel 7..9 reassignment at the correct delayed register-write boundary. Do not switch ownership immediately when the host queues a write to register `0x0E`.
- Integrate the five rhythm voices, their channel 7..9 source state, LFSR samples, candidate phase logic, and MO/RO retention in the same serial transaction.
- Retain explicit candidate status for unmeasured LFSR seed/reset/cadence and for candidate rhythm patch values.

### Required tests

- Rhythm disabled/enabled transitions at different points in the 18-slot rotation, including writes to `0x0E` adjacent to cycles 11..16.
- All five percussion triggers and volume changes, separately and in simultaneous combinations.
- Correct reuse/reassignment of slots 12..17 with no double evaluation of melodic channels 7..9 while rhythm mode owns those resources.
- LFSR, rhythm voice, phase, output-retention and core state snapshot/restore.
- Continuous-vs-partitioned equivalence across multiple complete serial rotations and mode transitions.
- Transactional failure/retry equivalence for the complete composite service.

## 5. End-to-end differential validation

The existing pinned oracle is Nuked-OPLL commit `1269cf5a783b65583b50fa2464d08be75830aaa0`. Keep it external and use the existing adapter; do not vendor or port Nuked implementation code.

Extend the independent Rust trace/comparator so a deterministic register-event stream is submitted through the Rust production write path and through Nuked `OPLL_Write` at explicitly mapped call boundaries. Compare, where the source trace exposes them:

- serial cycle and selected channel/operator;
- active register-decoded channel/patch parameters and their delayed visibility;
- PG phase array, PG increment and output latches;
- EG state/level and key/sustain pipeline;
- operator log/EXP/feedback latches and channel output;
- AM/PM LFO state;
- rhythm mode/voice selection, noise/LFSR state and phase candidates;
- MO/RO output pins and retained output state.

The run must cover multiple complete synthesis rotations and several independent event sequences: all nine melodic channels, custom patch, key transitions, rhythm enable/disable, each percussion voice and combined percussion. Define the tested duration in oracle calls and mapped Rust input clocks; do not call it a video frame unless a separately verified SMS frame mapping is part of the harness.

A PASS requires exact equality for fields whose semantics and sampling boundaries are mapped. Any intentionally candidate field must be reported separately, not hidden by excluding it from the trace. The differential is behavioral software validation, not physical-chip certification.

## 6. Explicitly out of scope for this branch

- SMS ports `0xF0`, `0xF1`, `0xF2`, bus busy-cycle WAIT states and hardware profile detection (FM-7).
- PCM quantization, physical DAC word claims, analog RC model, PSG+FM gain calibration, resampling and CPAL ring-buffer integration (FM-6/FM-8).
- Changes to Z80, MachineScheduler, SMS VDP, Slint/WGPU or CPAL.
- Reopening validated FM-1/FM-2/FM-3A behavior without a reproducible regression or a new oracle discrepancy.

## 7. Merge gate

FM-4/FM-5 are not complete when the crate compiles. Completion requires:
- the full service is the only temporal owner and has no injected phase inputs in production;
- all state needed for deterministic continuation is snapshotted and validated;
- block partitioning and transactional retry are equivalent;
- the multi-rotation Nuked differential passes on the exact PR HEAD;
- CI runs the YM2413-specific differential and Rust regression gates on that exact HEAD;
- no SMS bus or PCM/mixer work is smuggled into the FM-4/FM-5 PR.
