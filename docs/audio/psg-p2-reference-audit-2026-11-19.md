# P2 — Differential reference audit of the Sega SN76489

**Date:** 2026-11-19
**Status:** PARTIAL — static source comparison completed; executable external adapters and automated cross-emulator traces remain pending.
**Production changes:** none.
**Baseline:** `main` at `0e8025e476eee12135f4c0f7def9dead7dfcaa14`.
**Core blob:** `native/sn76489/src/psg.rs`, blob `cfccd43bf9178c5204464dd8057ac964d8429d4c`.

This document records source-level behavior, not physical-chip validation. A reference implementation is evidence about that implementation's model; it is not automatically a hardware oracle. No third-party production code is copied into Sega Emulator.

## 1. Existing Sega Emulator coverage

The existing A1 audit is in [psg-sn76489-a1-audit.md](psg-sn76489-a1-audit.md). PR #784 is merged and already provides:

- 32 fixed white-noise tuples for the SMS profile;
- the 16-state periodic-noise cycle;
- a partial Noise Rate = Tone 2 counter-continuity test.

The current core uses the SMS profile:

- seed `0x8000`;
- 16-bit mask `0xFFFF`;
- white-noise taps 0 and 3 (`0x0009`);
- feedback inserted at bit 15;
- output value captured from the old bit 0 before shifting;
- tone periods 0 and 1 forced to constant output 1;
- a separate noise counter and phase for Noise Rate = Tone 2.

Coverage gaps found in the current source:

1. The exhaustive tone-boundary test starts at period 2; no independent period-0/period-1 fixture exists.
2. The full 0x7FFF-state LFSR test uses the default SG-1000 profile (15-bit). The SMS profile has only a 32-state white-noise prefix fixture; no full 57,337-state cycle test exists.
3. The current white-noise test named `sms_white_noise_trace_matches_independent_gear_system_reference` stores Fibonacci-form internal states. Gearsystem uses a Galois-form recurrence. The observable bit stream is equivalent for the same seed, but the raw internal states are not equal.
4. The test `noise_mixer_outputs_the_bit_shifted_out_of_the_lfsr` fixes the pre-shift output convention, which remains disputed by reference implementations.
5. Noise Rate = Tone 2 tests preserve a partial independent divider, but do not establish absolute phase against all references.
6. Reset values for latch, tone phase and noise-divider phase have not been reconciled for a Sega integrated VDP profile.

## 2. Reference matrix

All commits below are pinned. File paths are relative to the listed repository.

| Reference | Commit | Relevant source | License observed | Independence / caveat |
|---|---|---|---|---|
| [Ares](https://github.com/ares-emulator/ares) | `8bc30c8c3d3c8081d2a66587add602df771441dd` | `ares/component/audio/sn76489/sn76489.cpp`, `sn76489.hpp`, `ares/ms/psg/psg.cpp` | ISC | Generic SN76489 component reused by Master System; not a Sega-specific period-0/1 implementation |
| [Genesis Plus GX](https://github.com/ekeeke/Genesis-Plus-GX) | `2499cd3e3d1d8b7d0130d8b2825adb4cc608b2c0` | `core/sound/psg.c` | Custom source license: non-commercial restriction | Independent implementation; integrated ASIC profile and reset comments are relevant, but do not assume every Genesis-specific behavior is identical to SMS |
| [SMS Plus GX](https://github.com/libretro/smsplus-gx) | `3844b46caa926b6494987b97da63092818c4ddef` | `source/sound/crabemu_sn76489/sn76489.c`; `source/sound/maxim_sn76489/sn76489.c`; `source/sound/mame_sn76489/sn76489.c` | Project GPLv2+; CrabEmu GPLv2; Maxim implementation has a separate problematic permission; MAME-derived engine follows its upstream lineage | Contains three distinct sound engines. Do not count its MAME engine as an independent confirmation of MAME. |
| [Gearsystem](https://github.com/drhelius/Gearsystem) | `1be9a12b82cae84ddd6f9b7006d546abe2d58bd4` | `src/audio/Sms_Apu.cpp`, `src/audio/Sms_Apu.h` | GPLv3 | Independent SMS implementation; uses Galois-form LFSR state |
| [MEKA](https://github.com/ocornut/meka) | `4c00b68ed08f2745d5ad3eeb05f5e1f258b198be` | `meka/srcs/sound/psg.cpp` | `meka/sources.txt` calls most MEKA code BSD-like, but says “everything else” is unlicensed/gray area | Auxiliary behavioral evidence only; do not reuse code or infer a clear file-level license |
| [MAME](https://github.com/mamedev/mame) | `f186445270e2c51c400d62336962c9a08b2065c9` | `src/devices/sound/sn76496.cpp`, `src/devices/sound/sn76496.h` | BSD-3-Clause in source header | Independent source with explicit `segapsg_device` variant and historical comments about hardware verification |
| [SMS Power](https://www.smspower.org/Development/SN76489) | Page retrieved 2026-11-19 | “Tone channels”, “Noise channel”, “The Linear Feedback Shift Register”, “Playing samples on the PSG” | Technical documentation; not executable code | Primary community research source; page itself contains a pre-/post-shift wording inconsistency |

### License handling

These references are used only to inspect behavior and generate test expectations. No code is copied or translated.

- Ares: ISC.
- MAME source file: BSD-3-Clause.
- Gearsystem: GPLv3 — oracle only.
- SMS Plus GX CrabEmu core: GPLv2 — oracle only.
- SMS Plus GX Maxim core: the repository explicitly flags its license as problematic; its contributor notes describe a distribution-notification condition. Oracle only.
- Genesis Plus GX: source header restricts sale/commercial use. Oracle only.
- MEKA: the PSG file has no clear standalone license in the inspected material. Oracle only.

## 3. LFSR width, seed and feedback

### 3.1 Convergence

For the SMS integrated profile, the following sources converge on a 16-bit register and the 0/3 feedback polynomial (mask `0x0009`):

- Ares: seed `0x8000`; output bit 0 is read before the shift; feedback is bit 0 XOR bit 3 in white-noise mode.
- Genesis Plus GX integrated profile: `noiseShiftWidth = 15`, `noiseBitMask = 0x9`; reset seed is `1 << 15`.
- MAME Sega VDP profile: feedback mask `0x8000`, taps `0x01` and `0x08`; Sega device is 16-bit.
- SMS Plus GX CrabEmu: caller selects `SN76489_NOISE_BITS_SMS = 16` and `SN76489_NOISE_TAPPED_SMS = 0x0009`.
- SMS Plus GX Maxim: seed `0x8000`, white-noise feedback mask `0x0009`.
- Gearsystem: seed `0x8000`; feedback mask is converted to a Galois representation.
- SMS Power documents 16 bits and taps 0/3 for Master System.

MEKA is an outlier: its PSG source sets `NoiseInitialState = 0x4000`, while its Sega path shifts a 16-bit register and uses taps 0 and 3. This does not outweigh the convergence of the other sources and is not adopted.

### 3.2 Gearsystem state representation is not directly comparable

Gearsystem converts the 0/3 polynomial to a Galois feedback mask (`0x9000`) and updates the register as a conditional XOR after shifting right. Sega Emulator uses the equivalent Fibonacci recurrence, calculating parity of bits 0 and 3 and inserting the result at bit 15.

For seed `0x8000`, the two internal state sequences diverge after the first several shifts, but their observable bit-0 streams match. Therefore:

- comparing raw state integers is invalid unless a formal state mapping is applied;
- comparing the output-bit sequence is valid for this recurrence and seed;
- the existing test fixture should be described as an output-stream-compatible vector, not a raw Gearsystem state trace.

### 3.3 SMS sequence length

The 16-bit 0/3 recurrence returns to seed after 57,337 states. It is not a maximal 65,535-state sequence.

The existing 0x7FFF-cycle test is for the default 15-bit SG-1000 profile, not SMS. Add a dedicated SMS full-cycle test and a stable digest of the output stream in a future test-only change.

## 4. Output-bit timing at the LFSR shift boundary

The references disagree about which bit is presented to the mixer at the exact shift boundary.

| Reference | Observed convention |
|---|---|
| Sega Emulator current core | Captures old bit 0 before shifting |
| Ares | Assigns output from old bit 0, then shifts |
| MAME Sega VDP | Shifts, then assigns output from the new bit 0 |
| Genesis Plus GX | Computes the output delta from new bit 0 minus old bit 0 |
| Gearsystem | Computes the output level from the updated shifter after each shift |
| SMS Plus GX CrabEmu / Maxim | Reads the current shift-register bit for the sample; the next sample observes the updated state |
| SMS Power prose | Says the bit shifted off the end is output (old bit 0) |
| SMS Power pseudocode | Shifts first, then reads `ShiftRegister & 1` (new bit 0) |

This is a real timing conflict, not merely a different state representation. The current pre-shift behavior must remain unchanged until an edge-aligned trace establishes the intended Sega profile behavior.

Required fixture: timestamp every shift, record old state, old bit 0, new state, new bit 0, and output before/after the boundary. Do not compare only a long PCM buffer or align waveforms by correlation.

## 5. Tone period 0 and 1

The source comparison produces two behavior groups.

### Constant-output group

- SMS Power explicitly says period 0 or 1 yields constant +1 and describes period 1 as useful for PSG PCM.
- SMS Plus GX Maxim uses cutoff 1; zero is normalized to 1 and the output is held at a constant level at or below the cutoff.
- SMS Plus GX CrabEmu holds tone state at +1 below cutoff 7.
- Gearsystem's output path does not modulate amplitude for very short periods; period 0 has no transition loop and period 1 remains constant at the output.
- MEKA uses cutoff 6 and forces a stuck value for low periods.

### Toggle-at-maximum group

- Ares toggles its tone output whenever its counter reaches zero; pitch 0 and 1 therefore toggle at the internal clock cadence.
- MAME's Sega VDP device leaves period 0 as zero and toggles on each counter expiration; period 1 also expires every internal tick.
- Genesis Plus GX maps integrated frequency zero to increment 1; its period-1 increment also runs at the maximum rate.

The MAME source history explicitly says the Sega zero-frequency behavior was verified on SMS hardware, while SMS Power's technical page documents constant +1 for both 0 and 1 and connects period 1 to PCM. These claims cannot be silently reconciled.

**Current core behavior:** constant output for periods 0 and 1. This agrees with SMS Power and the PCM-oriented models, but conflicts with MAME/Ares/Genesis Plus GX. Keep this as CONFLICT until the exact Sega VDP revision and observable behavior are established. Do not generalize the discrete TI SN76489 behavior to the integrated Sega PSG.

## 6. Noise Rate = Tone 2

The average rate is broadly represented as one LFSR shift per two Tone 2 half-wave periods, but the phase coupling differs:

- Core: separate noise counter/phase; period follows Tone 2; partial divider is preserved.
- Ares: separate noise counter/flip; noise pitch is copied from Tone 2.
- MAME: noise period becomes twice Tone 2 period; its noise counter remains independent.
- Gearsystem: noise uses a live pointer to Tone 2 period and retains its pending delay between calls.
- Genesis Plus GX: when Noise Rate = Tone 2 is selected, noise frequency increment and counter are initialized from Tone 2's increment/counter.
- CrabEmu: copies Tone 2's counter into the noise counter when the noise register selects Tone 2.
- Maxim/MEKA: noise counter is updated from Tone 2's counter during the sample update.

Equal average frequency does not establish equal phase after reset, a Tone 2 write, a noise-register write, or an interrupted block. Keep the independent core counter until a timestamped differential trace resolves the phase contract.

## 7. Reset state and register latch

The reset models are not converged.

| State | Sega Emulator current | MAME Sega VDP | Genesis Plus GX integrated | SMS Power |
|---|---|---|---|---|
| Tone registers | 0 | 0 | 0 | Integrated versions described as zero |
| Volume registers | 15 (silent) | 15 (silent) | 0, but output accumulators start at zero | 15 (silent) |
| Last latch | 0 | 3 | 3 | Not specified |
| Tone outputs | 1 | 0 | polarity initialized to -1; output accumulators 0 | Initial output may be arbitrary |
| Noise seed | 0x8000 | 0x8000 | 0x8000 | Hardware-dependent initial state; 16-bit SMS model |

The MAME source attributes Sega VDP latch 3 to hardware tests. Genesis Plus GX also selects latch 3 and says its integrated ASIC reset state was verified on 315-5313A/315-5660. SMS Power describes the register contents but does not specify the initial latch. The current core uses latch 0. This needs a profile-specific reset fixture; no change is made in this P2 audit.

## 8. Write and clock behavior

The sources generally agree that writes are applied at a synchronized timestamp and that register updates should not wait for an entire 10-bit tone word. They differ in how their host-facing audio loops express time:

- Ares clocks its component at the internal PSG cadence (input /16).
- Genesis Plus GX advances to a timestamp in master M-cycles and preserves fractional phase.
- SMS Plus GX and MEKA derive internal clock progress from input clock /16 and sample rate.
- Gearsystem uses timestamped oscillator runs and persistent delay.
- MAME uses a sound stream plus a clock divider; the Sega variant's effective internal cadence is input /16.

Host sample rate is not a hardware clock. These source patterns support the existing Scheduler-authoritative architecture, but do not establish the current core's exact sub-cycle write boundary.

## 9. P2 decisions

No production PSG code is changed by this audit.

| Question | P2 result | Next evidence |
|---|---|---|
| SMS LFSR width/seed/taps | CONVERGENT | Full-cycle 57,337-state test and independent digest |
| Gearsystem white-noise state trace | FIXTURE DESCRIPTION NEEDS CORRECTION | Compare output stream or formally map Galois/Fibonacci states |
| Periodic LFSR recurrence | CONVERGENT | Keep current 16-state regression |
| LFSR output bit at edge | CONFLICT | Edge-aligned trace with old/new state and output |
| Tone period 0/1 | CONFLICT | Variant-specific oracle; preserve PCM case |
| Noise Rate = Tone 2 phase | PARTIAL | Timestamp traces across reset and writes |
| Reset latch/output phase | CONFLICT | Profile-specific reset fixture |
| External differential automation | NOT IMPLEMENTED | Add reproducible adapters and CI artifact format |

### Next P2 implementation tasks

1. Correct the Gearsystem fixture description and add a test that compares the observable output-bit stream, not raw internal state integers.
2. Add the full SMS 57,337-state cycle regression.
3. Add independent traces for period 0 and 1, including output level and tone flip-flop state separately.
4. Add edge-aligned noise output traces for old/new bit conventions.
5. Add Noise Rate = Tone 2 traces for reset, tone-2 writes, noise-register writes, and arbitrary block boundaries.
6. Record reset defaults separately for SG-1000, Japanese SMS, and other Sega integrated profiles.
7. Build external reference adapters with pinned source commits and license metadata; run them in CI without linking their code into the Rust backend.

**Status at the initial static-audit commit:** P2 PARTIAL. At that point executable differential comparison and new tests had not yet been run. The subsequent adapter trial is recorded below; it does not close P2 because Ares, Genesis Plus GX and MEKA still lack executable adapters, and no physical-hardware oracle is available.

## 10. Automated differential trial — 2026-11-20

The external trace adapters were exercised by the existing PSG-focused GitHub Actions job in draft PR [#1081](https://github.com/leonardo201800478/sega-emulator/pull/1081), against core commit `e9acb3e4ac5c7a6a864fa6e3af4dd9e3960b7ff4`. The production core was unchanged.

- CrabEmu reference: `3844b46caa926b6494987b97da63092818c4ddef`.
- Gearsystem reference: `1be9a12b82cae84ddd6f9b7006d546abe2d58bd4`.
- 600 internal PSG ticks per trace, with writes at ticks 0, 540 and 560.
- Three traces were repeated and compared byte-for-byte.
- Comparator uses identical tick indices; no correlation-based alignment.
- The CI validation and adapter steps passed. Diagnostic traces were uploaded as artifact `psg-audio-ab-reference` from workflow run [44659953063](https://github.com/leonardo201800478/sega-emulator/actions/runs/44659953063).

### Observed shift schedules

| Engine | First shift | Subsequent shifts in this fixture |
|---|---:|---|
| SMS Plus GX / CrabEmu | 16 | 48, 80, 112, … |
| Sega Emulator SMS core | 15 | 47, 79, 111, … |
| Gearsystem Sms_Apu | 0 | 32, 64, 96, … |

All three traces recorded 19 shifts over the 600-tick fixture. The different first-shift phase is an implementation-level observation, not a hardware verdict.

### Differential counts

| Comparison | Mismatches | First mismatch |
|---|---:|---:|
| CrabEmu raw LFSR state vs core LFSR | 57 | tick 15 |
| CrabEmu sample bit vs core pre-tick LFSR bit | 2 | tick 464 |
| CrabEmu sample bit vs core latched `noise_output` | 60 | tick 465 |
| Gearsystem observable bit 0 vs core LFSR bit 0 | 30 | tick 448 |
| Gearsystem observable bit 0 vs core latched `noise_output` | 64 | tick 448 |

Gearsystem's Galois raw state integers were not compared to the core's Fibonacci state integers. Only observable bit 0 and timestamped shift events were compared.

The result exposes two distinct questions:

1. **Divider phase:** the initial and subsequent shift timestamps differ between implementations.
2. **Output semantics:** the core's latched `noise_output` remains different from the reference's sampled/current LFSR bit for a substantial interval.

These results justify deeper investigation of the reset divider phase and output-bit boundary. They do not establish that either reference is electrically correct. No PSG production correction is authorized by this trace alone.

### Validation boundary

The following were validated in the PSG-focused CI job: Rust formatting, Clippy, targeted PSG/audio/SMS/SG-1000 tests, both external adapter builds, deterministic repeated traces, trace schema, diagnostic schedule, historical PCM/chunk-invariance checks, and artifact upload. The complete CI run [#44659953063](https://github.com/leonardo201800478/sega-emulator/actions/runs/44659953063) also passed GitHub Actions workflow validation, documentation hygiene, Windows x64 CPAL tests/build and Rust quality/unit regression. The separate SMS Debugger Runtime workflow passed as well.

Still pending for P2:

- executable Ares, Genesis Plus GX and MEKA traces;
- tone period 0/1 differential vectors;
- reset/latch and Noise Rate = Tone 2 phase vectors;
- a formally documented mapping for Galois/Fibonacci state across every tested reset/write boundary;
- independent evidence sufficient to choose hardware behavior rather than a majority of emulator implementations.

No production hardware code was modified by this trial.
