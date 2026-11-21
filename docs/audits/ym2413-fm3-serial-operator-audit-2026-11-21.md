# YM2413 FM-3 — serial operator pipeline audi

**Date:** 2026-11-21
**Rust base:** FM-2 serial PG service, PR #1145 HEAD `45877480d824b558c0a708bd5a2a30732bff67f8`
**Implementation branch:** `feat/ym2413-fm3-serial-operator`
**Primary software oracle:** Nuked-OPLL commit `1269cf5a783b65583b50fa2464d08be75830aaa0`

## Scope and claim boundary

This increment adds a serial operator datapath and connects it to the existing core-owned slot hooks. It is **FM-3A implementation work**, not FM-3 completion. The independent operator differential has been authored but has not yet produced a CI PASS at the time of this audit.

Rhythm Mode routing, digital channel accumulation, PCM output, and physical DAC encoding remain outside this increment. No analog-fidelity claim is made.

## Source-derived OPLL_Clock order

The pinned Nuked `OPLL_Clock` executes the relevant stages in this order:

1. `OPLL_PreparePatch1`
2. `OPLL_Channel`
3. `OPLL_PhaseGenerate`
4. `OPLL_Operator`
5. `OPLL_PhaseCalcIncrement`
6. `OPLL_EnvelopeOutput`
7. `OPLL_EnvelopeKSLTL`
8. `OPLL_EnvelopeGenerate`
9. `OPLL_DoLFO`
10. `OPLL_DoRhythm`
11. `OPLL_PreparePatch2`
12. register-write / I/O service

The implementation maps these stages as follows:

| Nuked stage | Rust component / owner | Status in FM-3A |
|---|---|---|
| PreparePatch1 | `Ym2413PgSerialService::before_eg_update` + `decode_channel_synthesis` + `Ym2413SerialOperator::prepare_patch1` | Connected for melodic KSL/TL |
| Channel | Existing channel/register decoder and slot event | Reused; no new channel mixer |
| PhaseGenerate | Existing `generate_phase` and FM-2 PG state | Preserved |
| Operator | `Ym2413SerialOperator::tick_operator` | Implemented; differential pending |
| PhaseCalcIncrement | Existing `calculate_increment` | Preserved, after Operator |
| EnvelopeOutput | `advance_envelope_output` reading core EG state | Connected; EG remains core-owned |
| EnvelopeKSLTL | `advance_envelope_output` updates the delayed KSL/TL latch after output calculation | Connected |
| EnvelopeGenerate | `Ym2413Core` / `EnvelopeSequencer` | Sole authority; no second EG |
| DoLFO | Existing PM LFO plus `Ym2413SerialAmLfo::clock` | Serial AM shift/carry state implemented; differential pending |
| DoRhythm | Rhythm pipeline | Explicitly excluded; FM-5 |
| PreparePatch2 | `latch_next_parameters` and `prepare_patch2` | Connected for melodic PG/FB/DC/DM/AM latches |

The service call order is therefore:

`PreparePatch1 → PhaseGenerate → Operator → PhaseCalcIncrement → EnvelopeOutput → EnvelopeKSLTL → core EG update → DoLFO → PreparePatch2`.

The core's EG update occurs between the service's pre- and post-EG hooks. It is not invoked by the synthesis service.

## Operator state correspondence

The new `Ym2413SerialOperatorState` retains:

- `op_fb1[9]`, `op_fb2[9]`, and `op_fbsum`;
- `op_mod`, `op_neg`, and `op_logsin`;
- `op_exp_m` and `op_exp_s`;
- `eg_out`, `eg_silent`, and `eg_ksltl`;
- PreparePatch1 KSL/TL latches;
- PreparePatch2 FB, DC, DM, and AM latches;
- last operator/channel output and the last phase/feedback contributions.

The feedback ring index and delayed feedback-sum selection follow the serial cycle:
`op_fbsum ← (op_fb1[(cycle+3)%9] + op_fb2[(cycle+3)%9]) >> 1`, with the two-sample history updated only on the carrier-selection cycles defined by Nuked's `ismod1` expression.

Nuked's PhaseGenerate output is stored in a `uint16_t` PG bus (`phase >> 9`) and can carry values above bit 10. OPLL_Operator adds it to the modulation value and masks the sum to ten bits. The Rust input therefore preserves the full `u16` PG bus and applies the ten-bit mask only at the operator phase boundary. The phase path uses Nuked's ten-bit effective phase:
`phase = (op_mod + pg_out) & 0x3ff`, followed by the half-wave mirror when bit 8 is set. The 32-bit addition is wrapping before the ten-bit mask.

## ROM transform audi

The Rust log-sine table contains 256 values and was compared against the pinned Nuked `logsinrom`: **256/256 values equal**.

The Rust EXP table is stored in a transformed mathematical domain. Its serial operator reconstruction is:

`exprom[index] = 1024 + YM2413_EXP_TABLE[255 - index]`.

This expression was compared against the pinned Nuked `exprom`: **256/256 reconstructed values equal**. The production module performs this conversion when latching `op_exp_m`; it does not copy Nuked's ROM table into the Rust source.

## Differential harness

The external Nuked adapter now records the pre-call operator inputs consumed by `OPLL_Operator` and `OPLL_DoLFO`:

- `pg_out`;
- `eg_out` and `eg_silent`;
- `c_fb`, `c_dc`, and `c_dm`;
- the pre-increment `lfo_counter` and `testmode` consumed by the AM LFO.

The post-call trace exposes the internal operator state, channel output, and AM shift/carry state. The new `compare_operator.py` feeds only the oracle's pre-call operator latches into the production Rust operator and compares:

- `op_exp_m`, `op_exp_s`, `op_logsin`, `op_neg`, and `op_mod`;
- `op_fbsum`;
- all 18 feedback-history entries;
- `ch_out`.

The new `compare_am_lfo.py` replays the serial AM network and compares `lfo_am_counter`, `lfo_am_dir`, `lfo_am_car`, `lfo_am_step`, and `lfo_am_out` after every call. The CI fixture programs all nine melodic channels, a user patch with nonzero feedback, AM/VIB and both waveform bits, KEY-ON/KEY-OFF, live parameter changes, and Test Mode bits 0–3. This isolates the operator datapath from independent PG/EG generation. A later integrated differential must additionally prove that Rust itself produces the same pre-call inputs.

## Known open work

1. Run the new external operator differential and fix every mismatch.
2. Compare the integrated Rust PG + operator + core-owned EG pipeline against Nuked without oracle-injected latches.
3. Run the new serial AM LFO differential and prove its integration with the PM counter and Test Mode. The serial slot service now uses a nine-bit shift/carry implementation; the older measured sample-domain AM candidate remains separate and is not consumed by the serial operator.
4. Add parameter-latch timing cases for FNUM/BLOCK/MUL/FB/KSL/TL/AM/VIB/DC/DM writes during active notes.
5. Validate continuous versus partitioned service execution, clone independence, snapshot/restore, rollback, and all nine channel mappings.
6. Resolve any P1/P2 review findings before merge.

## CI provenance

FM-2 PR #1145, code HEAD `56dfe6262d6e98ccae1bd32a3e622050a16ebf20`, has the previously recorded PG differential PASS (run `44856572960`). The later documentation-only commit `45877480d824b558c0a708bd5a2a30732bff67f8` fixed scorecard trailing whitespace; CI run `44859302456` passed Rust workspace regression, Rust quality/unit regression, workflow validation and documentation patch hygiene. The PG differential (run `44859302507`) and EG characterization (run `44859302452`) also passed. Windows/desktop, Scheduler stress, SDL3, PSG, VDP and unrelated console gates were SKIPPED and are not approvals.

No FM-3 differential PASS is recorded by this audit.
