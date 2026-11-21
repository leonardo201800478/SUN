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

## 8. FM-5 implementation audit — 2026-11-21

The source audit of PR HEAD `3972678ac8b3a4b1e5b083ad05900d43e68cd716` confirms the following integration facts:

- `Ym2413Core::begin_slot` is the authoritative mode boundary: it shifts `rm_enable`, applies the delayed mode write, advances `rm_select`, then resolves the cycle's EG input. The service receives the resulting register image and post-preamble mode latches. A host-queued write to `$0E` is therefore not an immediate ownership switch.
- `clock_one_slot_with_service` already enforces the required sequence: pre-EG service, exactly one core-owned `EnvelopeSequencer::tick_slot`, post-EG service, then the ordinary timed register-write stage. The service receives both pre-EG and post-EG EG snapshots.
- `advance_input_clocks_with_slot_service` stages a deep clone of the service together with the staged core. A service error rolls back both and permits deterministic retry. FM-5 must preserve this contract; no externally visible side effects may occur inside either hook.
- The core's ordinary EG decoder already receives the six rhythm selectors through `rm_select` and `rhythm_bits`. FM-5 must not introduce a second EG sequencer or bypass the core's selector pipeline.
- `Ym2413PgSerialService::before_eg_update` currently rejects when RHY is active, when the shifted `rm_enable` latch is nonzero, or while `rm_select < 6`. Its `after_eg_update` currently clocks LFO state and then calls `latch_next_parameters`, which repeats the same Rhythm Mode rejection. These are the concrete FM-5 integration points; the rejection is deliberate gating, not a missing core clock.
- `decode_rhythm_synthesis` already maps BD to slots 13/16, HH to 14, TOM to 15, SD to 17 and TCY to 18. `decode_rhythm_envelope_slot_input` maps EG cycles 11..16 to BD0, HH, TOM, BD1, SD and TCY. The static mapping exists, but it is not yet connected to the PG/operator service.
- The rhythm LFSR and output-retention components require explicit caller-supplied initial state. Rhythm phase equations and rhythm patch data retain candidate status. FM-5 must preserve that uncertainty in state metadata and tests rather than selecting an undocumented seed, reset phase, or physical DAC interpretation.

### Required implementation order

1. Add a typed rhythm-mode/slot dispatch decision derived only from the service context; test all 18 slots in melodic mode, active rhythm mode, and transition boundaries.
2. Extend the serial service snapshot with the complete rhythm-owned mutable state (noise sequencer, rhythm voice/operator state, phase inputs/latches, and output retention where integrated). Validate versions and every cursor/slot relationship on restore.
3. Implement pre-EG rhythm parameter/PG/operator routing while retaining the existing single core EG update.
4. Implement post-EG order as LFO, Rhythm processing, then next-parameter latch. Keep the DoRhythm action at this post-EG boundary.
5. Add mode-transition, trigger/volume, partition-equivalence, snapshot/restore, and injected-failure/retry tests before changing the PR's merge status.

No FM-5 production behavior is claimed complete by this audit. No core timing changes are presently justified: the existing core hook already provides the required transactional boundary.

## 9. FM-5.1 — pure serial ownership dispatcher

Commit `72285bcf3ba725b26951ee73e95aa90cc29bc285` introduces `rhythm_dispatch.rs` and exports its typed API.

- `dispatch_serial_slot` resolves the zero-based core cycle through the canonical `YM2413_SLOT_MAP`; it does not maintain a second physical channel/operator table.
- With RHY clear, all eighteen positions remain melodic, including the channel 7..9 slots.
- With RHY set, only cycles 11..16 can be reassigned. Their required selector sequence is BD-first, HH, TOM, BD-second, SD, TCY.
- A selector mismatch is represented as `RhythmTransition`, never silently routed to the wrong percussion voice.
- Invalid cycles are rejected before table indexing.
- Four unit tests cover all 18 melodic positions, the six rhythm positions, all 42 selector/cycle combinations (six expected plus mismatches), and invalid cycles.

This is the first production-code FM-5.1 foundation, but it is not yet connected to `Ym2413PgSerialService`. The service still rejects Rhythm Mode. Do not mark percussion synthesis integrated until the typed dispatch is consumed by the serial PG/operator pipeline and all rhythm state is owned by the same transactional service.

### Current validation status

- EG characterization run `44875121085` completed SUCCESS on code-bearing parent `3972678ac8b3a4b1e5b083ad05900d43e68cd716`, including the 38-configuration AR/DR/KSR sweep.
- New dispatcher commits `ae09660d7a1a9efed206dabc01b8b60160b879f9` and `72285bcf3ba725b26951ee73e95aa90cc29bc285` triggered fresh CI runs. The exact current HEAD gates must complete before this code is considered validated.

## 10. FM-5.1 — PG service ownership routing

Commit `9f024e691be1c770b906db9ea36e25482c8481d0` adds `Ym2413PgSerialService::require_melodic_slot`, which consumes the pure dispatcher at both pre-EG PG/operator preparation and post-EG next-parameter latching.

- Melodic slots outside the channel 7..9 reassignment range continue through the existing PG/operator path even while RHY is active or its shift latch is transitioning.
- The six reassigned cycles (Nuked cycles 11..16) remain explicitly rejected before any PG/operator latch mutation. Stable rhythm ownership, a mode-shift transition, or a selector mismatch all retain the existing `UnsupportedRhythmMode` error boundary.
- The returned canonical descriptor is checked against the slot event descriptor. A disagreement is an invalid service state, not a fallback to another channel.
- Tests cover all unaffected melodic cycles under active RHY, all six percussion cycles, mode enable/disable transition values, selector mismatch, and unchanged service state on rejection.
- The core still performs exactly one EG update between the service hooks. No clock, scheduler or EG ownership was added.

### Pinned Nuked source observation for the next stage

At pinned Nuked-OPLL commit `1269cf5a783b65583b50fa2464d08be75830aaa0`, `OPLL_Clock` invokes `OPLL_DoLFO`, then `OPLL_DoRhythm`, then `OPLL_PreparePatch2` on every serial call. `OPLL_DoRhythm` advances the noise register unconditionally; it is not called only for cycles 11..16 or only while RHY is active. Therefore a future Rust rhythm-noise sequencer must preserve one advancement per serial slot across the full rotation, including melodic slots and periods when rhythm mode is disabled. The seed/reset phase remains a separate unknown and must stay explicitly supplied/candidate; do not invent a default seed.

The service currently does not own or advance this LFSR. FM-5.1 therefore enables unaffected melodic channels but does not claim rhythm-noise integration or complete percussion output.

### Current exact-HEAD validation

For code-bearing HEAD `9f024e691be1c770b906db9ea36e25482c8481d0`:
- Rust workspace CI `44876497128`: SUCCESS (format, Clippy, workspace tests, observability Clippy and tests).
- YM2413 PG differential `44876497115`: SUCCESS.
- Rhythm reset differential `44876497140`: SUCCESS.
- External Nuked adapter `44876497172`: SUCCESS.
- EG characterization `44876497153`: still running its 38-configuration AR/DR/KSR sweep at the time of this update.

## 11. FM-5.2A — serial rhythm-noise cadence integration

Branch `feat/ym2413-fm5-2-serial-rhythm-state`, commit `7f6a2f0223b54f1295088abaf2f8859483a97ce7`.

This increment connects the already characterized slot-rate noise sequencer to the existing transactional PG service at the post-EG DoRhythm boundary. It is deliberately a bounded FM-5.2A foundation, not a claim that the five percussion instruments are synthesized end-to-end.

### Production changes

- `Ym2413PgSerialService` now optionally owns `Ym2413RhythmSlotNoiseSequencer` and includes its complete cursor/LFSR state in the versioned service snapshot.
- The physical reset seed remains unknown. No implicit seed is introduced: callers must opt into a candidate seed through `with_rhythm_noise_seed(seed, next_slot_index)`. A service created with `new` retains an explicitly uninitialized noise model.
- When configured, the noise sequencer advances exactly once at every post-EG serial slot, independent of RHY enablement. Its cursor is checked against the core-owned cycle before mutation; mismatch returns an error and relies on the existing staged core/service transaction for rollback.
- The sequencer's measured HH and SD sample bits are retained in service state for subsequent phase routing.
- Stable rhythm-owned slots and unsettled rhythm selector/mode transitions no longer execute the melodic PG/operator path. They still pass through the core-owned EG update, serial LFO stage and DoRhythm noise step. Rhythm-specific PG/operator evaluation and PreparePatch2 remain intentionally unimplemented in this increment.
- Melodic slots continue through the existing path. The service still does not create a second clock, scheduler or EG sequencer.

### Validation added

- All 18 slots advance the explicitly seeded LFSR once in melodic mode.
- HH and SD noise samples are captured at their configured serial positions.
- Rhythm-owned cycles 11..16 skip melodic operator mutation while reaching DoRhythm.
- Mode-latch and selector-transition cycles preserve melodic PG/operator state while still advancing the configured noise state.
- Snapshot/restore after a partial slot and across multiple rotations.
- Continuous versus partitioned input-clock execution equivalence.
- Seeded-noise cursor mismatch rejects the staged service transaction without committing core or service state.

### Remaining FM-5.2 work

- Route the five instruments' patch parameters and channel 7..9 source registers into the correct serial PG/EG/operator latches.
- Implement Bass Drum's two-operator feedback path and the single-operator HH, TOM, SD and TCY paths at their measured slot boundaries.
- Integrate HH/SD sampled noise and the channel-7/channel-8 phase combiners without inventing initial PG phases.
- Integrate per-voice trigger/key transitions, attenuation and output retention MO/RO.
- Extend snapshots for every new dynamic voice/phase/output latch and validate them in the composite transaction.
- Expand the independent Nuked/hardware-capture differential before claiming any instrument complete.

The LFSR seed/reset phase, rhythm patch parameter candidates, and physical DAC interpretation remain explicitly unresolved. Nuked remains a software reference rather than proof of physical silicon behavior.

### Nuked DoRhythm arithmetic refinement

The pinned source at commit `1269cf5a783b65583b50fa2464d08be75830aaa0` was inspected directly:

- `OPLL_Reset` zeroes the complete `opll_t`, including `rm_noise`.
- `OPLL_DoRhythm` computes feedback as bit 0 XOR bit 14, ORs the zero-register escape bit, ORs Test Mode bit 1, then shifts right and inserts the feedback at bit 22.
- `OPLL_Clock` calls `OPLL_DoLFO`, then `OPLL_DoRhythm`, then `OPLL_PreparePatch2` on every call.

FM-5.2A therefore now supports an explicit 23-bit register candidate including zero, reproduces the zero-lock escape and Test Mode bit-1 feedback injection, and passes the current Test Mode register into the post-EG DoRhythm step. The nonzero-seed constructor remains available for hardware-candidate experiments. Neither zero nor any nonzero value is claimed as the physical chip's power-on seed.

A new independent differential is added to `.github/workflows/ym2413-rhythm-reset-differential.yml`: 144 consecutive Nuked `rm_noise` states from `OPLL_Reset` are compared with the Rust slot-rate LFSR trace. Unit tests separately cover zero-register escape, Test Mode bit 1, service-level Test Mode propagation, snapshot restoration and full-slot cadence.

### Unseeded cadence state

The serial PG service now also retains a core-aligned DoRhythm slot cursor and cumulative step count when no LFSR register candidate has been selected. Every post-EG slot advances this symbolic cadence, including RHY-disabled slots, while noise samples remain `None`. This preserves the known temporal behavior without fabricating an LFSR seed or output bit. When a candidate register is explicitly configured, the concrete 23-bit LFSR and symbolic cursor advance together and are checked for alignment. Both forms are included in the versioned service snapshot.
