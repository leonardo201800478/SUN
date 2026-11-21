# YM2413 FM-5.2B — serial Rhythm Mode integration

**Data:** 2026-11-21  
**Repository:** `leonardo201800478/SUN`  
**Parent:** PR #1154 — FM-5.2A serial rhythm LFSR cadence  
**Work branch:** `feat/ym2413-fm5-2b-rhythm-operator-routing`  
**Reference implementation:** Nuked-OPLL `1269cf5a783b65583b50fa2464d08be75830aaa0`  
**Status:** implemented in a draft stacked PR; not merged to `main`.

## Scope delivered

FM-5.2B connects the existing Rhythm Mode decoder/configuration to the serial PG/operator service. It does not claim complete YM2413 audio output.

### Serial ownership and parameter routing

The existing six-cycle ownership map is reused:

| Core cycle | EG rhythm selector | Voice/operator |
|---:|---|---|
| 11 | BassDrumFirst | Bass Drum modulator |
| 12 | HiHat | Hi-Hat single operator |
| 13 | TomTom | Tom single operator |
| 14 | BassDrumSecond | Bass Drum carrier |
| 15 | SnareDrum | Snare Drum single operator |
| 16 | TopCymbal | Top Cymbal single operator |

The selector is resolved through `decode_rhythm_synthesis`; the implementation does not create a second operator bank or a second clock owner. The existing `Ym2413Core` remains the source of serial cycle, EG state, and the post-EG service boundary.

Rhythm PreparePatch1/PreparePatch2 latches use the selected voice/operator configuration, channel FNUM/BLOCK, patch multiplier/AM/PM/envelope fields, feedback, and the channel's rhythm register. HH and TOM retain the special instrument-nibble TL selection; other voices select patch TL or rhythm volume according to Nuked's `mcsel` cadence.

### PG phase capture and phase address generation

The implementation follows the pinned Nuked `OPLL_PhaseGenerate` sequence:

- Cycle 13 captures HH phase bits 2, 3, 7 and 8 from the current PG phase.
- Cycle 17 captures TC phase bits 3 and 5 when post-shift `rm_enable & 0x80` is set.
- Cycle 13 forms the HH phase address from the captured HH/TC bits and the current pre-DoRhythm LFSR bit.
- Cycle 16 forms the SD phase address from HH bit 8 and the current pre-DoRhythm LFSR bit.
- Cycle 17 forms the TC phase address from the captured phase mix and the fixed low address bit.

The boolean equations and phase-address constants are kept in one helper and covered by directed tests. The phase latches are part of the PG pipeline snapshot.

### Operator cycle overrides and output retention

The serial operator receives the actual post-shift `rm_enable` byte, not a single Rhythm Mode boolean. This preserves transition behavior:

- Bit 7 overrides `ismod1` at cycles 15 and 16 and selects the retained rhythm output path.
- Bit 6 overrides `ismod2` at cycles 13 and 14.
- Bit 6 overrides `ismod3` at cycles 16 and 17.

Five independent delayed output latches are retained in Nuked order: HH, TOM, BD, SD and TC. Source cycles are 15, 16, 17, 0 and 1; return cycles are 2, 3, 4, 8 and 9. They are included in the serial operator snapshot.

### LFSR order

The PG phase path samples the current LFSR bit before the post-EG noise step. `clock_rhythm_noise` still advances once per core-owned slot after the EG boundary, irrespective of RHY mode, matching the pinned Nuked call order. No additional clock or scheduler was introduced.

If no explicit noise seed is supplied, the service continues to gate the rhythm-owned PG/operator path rather than fabricate a reset seed or noise bit. This is a deliberate unresolved hardware-state boundary, not a claim that unseeded percussion is synthesized.

## Snapshot compatibility

The new retained state changes snapshot schema versions:

- Serial operator state: version 3 (five rhythm channel output latches).
- PG pipeline state: version 4 (HH/TC phase latches).
- PG serial service state: version 9 (nested state schema).

Restore validation continues to reject incompatible versions and invalid retained output amplitudes. The seeded 18-slot test verifies snapshot restoration and equivalence between uninterrupted execution and execution resumed from a mid-rotation snapshot.

## Validation evidence and limits

At code HEAD `a71394f294ab30717e36079252fbbe6a1c729bbf`:

- Rust workspace CI #9790: PASS, including formatting, Clippy, workspace tests and observability feature tests.
- Nuked PG differential #326: PASS for the existing PG/FM baseline.
- Nuked Rhythm reset differential #249: PASS for the LFSR/reset cadence.
- Nuked EG characterization #499: running at the time this audit was written.
- Dedicated rhythm-operator differential: NOT RUN.
- Hardware/chip validation: NOT RUN.

The PG and Rhythm reset workflows do not constitute a differential of the new five-voice operator path. The new directed tests validate the phase equations and serial-state behavior, but a per-instrument oracle trace comparison is still required.

## Explicitly not delivered

- A complete per-instrument output differential against Nuked-OPLL.
- Final MO/RO pin routing/retention and the nine-channel output accumulation.
- Conversion from the current operator-level result to a raw nine-bit DAC word.
- PCM generation, PSG+FM mixing, or resampling.
- SMS F0–F2 register/bus integration or runtime audio integration.
- Physical validation of reset seed, patch ROM values, or analog DAC behavior.

The rhythm patch values remain software candidates with documented provenance; they are not presented as a hardware-certified internal ROM dump.

## Next technical gates

1. Add a dedicated Nuked oracle trace for all five voices, comparing selected patch/operator, PG address, EG output, operator feedback/log/exp state, and each retained rhythm channel output at the same serial boundaries.
2. Resolve and validate MO/RO pin timing and rhythm output routing without collapsing serial events into a sample-domain approximation.
3. Define and test the raw DAC code and PCM sample boundary.
4. Integrate the FM sample path into SMS only after the serial output is independently validated; preserve MachineScheduler as the sole timing authority.
5. Add aggregated Save State and continuous-versus-block equivalence at the SMS machine boundary.

No merge was performed or authorized by this work.
