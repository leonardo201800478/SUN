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

At code HEAD `6aed8b627b9b84b4459da1b0e65ffeb730b9fe35`:

- Rust CI #9825: **SUCCESS**. Change-impact classification, documentation hygiene, workflow validation, Rustfmt, Clippy, workspace unit/integration tests, observability Clippy and observability tests passed. Unrelated platform/runtime gates were skipped by impact classification.
- Nuked PG differential #361: **SUCCESS**.
  - Isolated operator differential: all five rhythm output latches compared against Nuked after each serial call; effective `rm_enable` was captured immediately before `OPLL_Operator`.
  - Integrated Rhythm Mode characterization: 147,456 calls, all 18 serial cycles, individual percussion triggers, simultaneous voices, release and retrigger. The fixture uses Table III-9 frequency values and the dispatcher regression asserts the documented physical slot ownership.
  - Strict control/timing state matched: cycle order, LFO/EG global timers, key-on control, channel/slot identity, `rm_enable`, `rm_select`, rhythm control and the 23-bit LFSR state.
  - All five retained output latches (HH/TOM/BD/SD/TC) became nonzero in both traces and had no reported candidate divergence.
- Nuked Rhythm reset differential #284: **SUCCESS**.
- External Nuked oracle adapter #457: **SUCCESS**.
- Nuked EG characterization #534: the 38-configuration AR/DR/KSR sweep was still running at the time of this audit update.
- Hardware/chip validation: **NOT RUN**.

The same PG workflow also passed the integrated PG service, all 18 accumulated phases, KEY-ON phase reset and pinned MAME/ymfm arithmetic audit.

The integrated Rhythm Mode characterization deliberately separates strict serial/control/timing evidence from synthesis fields derived from the un-certified rhythm patch candidates. It does not claim full synthesis equality with Nuked or physical chip accuracy.

Candidate-dependent differences reported over the 147,456-call trace:

| Field | Rows different | First call | Nuked | Rust candidate |
|---|---:|---:|---:|---:|
| `c_fb` | 8,092 | 1,814 | 0 | 7 |
| `c_dm` | 64,732 | 1,814 | 8 | 9 |
| `op_calc_c_fb` | 8,092 | 1,815 | 0 | 7 |
| `op_calc_c_dm` | 64,731 | 1,815 | 8 | 9 |
| `c_sl` | 8,092 | 1,815 | 4 | 6 |
| `eg_sl` | 8,092 | 1,816 | 4 | 6 |
| `eg_state` | 15,984 | 79,001 | array divergence | array divergence |

These differences are reported, not normalized away. In particular, the first feedback/waveform/sustain-level differences occur as the rhythm patch profile is selected. They must be reconciled against physical patch evidence before the candidate table is certified. The zero LFSR register used in the trace matches Nuked's software reset state only; it is not evidence for the physical YM2413 power-on seed.

## Hardware reference alignment

The serial dispatcher regression now asserts the rhythm ownership documented
in Table III-9 of the [YM2413 application manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual):
one-based physical slots 13/16 = BD modulator/carrier, 14 = HH, 15 = TOM,
17 = SD and 18 = TC. The integrated oracle fixtures use the manual's rhythm
frequency setup: R#16/17/18 = 0x20/0x50/0xC0 and R#26/27/28 =
0x05/0x05/0x01, keeping melodic KEY-ON bits clear.

The [SMS Power reverse-engineering notes of 2015-12-16](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-16)
explicitly describe the six rhythm-ROM decoder lines as a best guess and state
that rhythm behavior had not yet been reverse-engineered in that work. This is
why the current patch table remains a candidate and why its differences from
Nuked are reported rather than forcibly normalized.

## Explicitly not delivered

- Full integrated synthesis equality for the rhythm patch candidates; the current oracle characterization reports candidate-dependent PG/EG/operator differences rather than treating them as a pass.
- Physical certification of the rhythm patch table, LFSR reset seed and analog behavior.
- Final MO/RO pin routing/retention and the nine-channel output accumulation.
- Conversion from the current operator-level result to a raw nine-bit DAC word.
- PCM generation, PSG+FM mixing, or resampling.
- SMS F0–F2 register/bus integration or runtime audio integration.
- Physical validation of reset seed, patch ROM values, or analog DAC behavior.

The rhythm patch values remain software candidates with documented provenance; they are not presented as a hardware-certified internal ROM dump.

## Next technical gates

1. Reconcile the reported `c_fb`, `c_dm`, `c_sl` and downstream EG-state differences using physical patch evidence. Do not rewrite the candidate table merely to force parity with Nuked.
2. Validate the rhythm patch profile and LFSR reset seed against hardware evidence; retain the explicit-seed gate until then.
3. Resolve and validate MO/RO pin timing and rhythm output routing without collapsing serial events into a sample-domain approximation.
4. Define and test the raw DAC code and PCM sample boundary.
5. Integrate the FM sample path into SMS only after the serial output is independently validated; preserve MachineScheduler as the sole timing authority.
6. Add aggregated Save State and continuous-versus-block equivalence at the SMS machine boundary.

No merge was performed or authorized by this work.


## External Nuked operator differential gate

The pinned Nuked-OPLL workflow now instruments the effective `rm_enable`
latch immediately before `OPLL_Operator` (after the per-call shift and
`DoModeWrite`). A dedicated all-five-voice register sequence drives the
external YM2413 reference. The Rust replay consumes that exact boundary latch
and compares the serial operator/feedback state plus retained HH, TOM, BD, SD
and TC outputs after every `OPLL_Clock` call.

The gate requires both rhythm override bits (0x40 and 0x80) to occur and each
of the five retained Nuked outputs to become nonzero. This is an isolated
operator/output-latch differential: it does not independently validate Rust
PG/EG/noise generation, final MO/RO mixing, DAC/PCM conversion, or physical
chip timing. The broader PG/EG and rhythm-reset workflows remain separate
gates.
