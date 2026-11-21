# YM2413 FM-3A — serial operator pipeline audit

**Date:** 2026-11-21
**Rust base:** FM-2 serial PG service, PR #1145 HEAD `45877480d824b558c0a708bd5a2a30732bff67f8`
**Implementation branch:** `feat/ym2413-fm3-serial-operator`
**Primary software oracle:** Nuked-OPLL commit `1269cf5a783b65583b50fa2464d08be75830aaa0`

## Scope and claim boundary

This increment adds the serial operator datapath and connects it to the existing core-owned slot hooks. It is **FM-3A**, not completion of all FM-3 or the YM2413. On PR #1147 HEAD `b538a6904c344fec5e1c5dc8bb08401d92c4c220`, the isolated operator, serial AM LFO, integrated PG/EG/AM/operator differential, serial PG differential, and Rust workspace CI all pass. The PR remains open and stacked over #1145/#1133; no merge has been performed.

Rhythm Mode synthesis/routing, nine-channel digital accumulation, PCM output, mixer, physical DAC encoding, and SMS FM Unit integration remain outside FM-3A. No analog-fidelity or ROM-playability claim is made.

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
| Operator | `Ym2413SerialOperator::tick_operator` | Implemented; isolated and integrated differential PASS — 147,456 calls |
| PhaseCalcIncrement | Existing `calculate_increment` | Preserved, after Operator |
| EnvelopeOutput | `advance_envelope_output` reading core EG state | Connected; EG remains core-owned |
| EnvelopeKSLTL | `advance_envelope_output` updates the delayed KSL/TL latch after output calculation | Connected |
| EnvelopeGenerate | `Ym2413Core` / `EnvelopeSequencer` | Sole authority; no second EG |
| DoLFO | Existing PM LFO plus `Ym2413SerialAmLfo::clock` | Serial AM shift/carry state implemented; differential PASS — 147,456 calls; Test Mode captured after `OPLL_DoModeWrite` |
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

Nuked's `OPLL_PhaseGenerate` stores `pg_phase`, `pg_phase_next`, and the phase sum as `uint32_t`, then assigns `pg_out = phase >> 9` to a `uint16_t`. The raw PG bus can therefore carry values above bit 10; it must not be masked to 19 bits or 10 bits at the PG boundary. Rust now preserves the full `u32` phase ring and full `u16` `pg_out` latch. `OPLL_Operator` then performs the actual ten-bit phase selection:
`phase = (op_mod + pg_out) & 0x3ff`, followed by the half-wave mirror when bit 8 is set. The addition is wrapping before the ten-bit mask.

The PG ring index is Nuked cycle order (`ch_offset`), not the canonical Yamaha slot order. `nuked_pg_phase_slot_descriptor` explicitly maps this rotated index to the logical operator. The integrated trace uses the same mapping; the service writes and reads the same logical accumulators as the reference.

## ROM transform audi

The Rust log-sine table contains 256 values and was compared against the pinned Nuked `logsinrom`: **256/256 values equal**.

The Rust EXP table is stored in a transformed mathematical domain. Its serial operator reconstruction is:

`exprom[index] = 1024 + YM2413_EXP_TABLE[255 - index]`.

This expression was compared against the pinned Nuked `exprom`: **256/256 reconstructed values equal**. The production module performs this conversion when latching `op_exp_m`; it does not copy Nuked's ROM table into the Rust source.

## Differential harness

The external Nuked adapter captures the pre-call inputs consumed by `OPLL_Operator` and the exact Test Mode value consumed by `OPLL_DoLFO`. The operator boundary records effective instruction/operator selection after `OPLL_PreparePatch1`; it does not compare the unused/stale structural `chip.c_op` member as if it were an active latch. The AM boundary captures `chip->testmode` after `OPLL_DoModeWrite`, immediately before `OPLL_DoLFO`.

The isolated operator differential compares `op_exp_m`, `op_exp_s`, `op_logsin`, `op_neg`, `op_mod`, `op_fbsum`, both nine-entry feedback histories, `ch_out`, phase index, and operator output. The serial AM differential compares `lfo_am_counter`, `lfo_am_dir`, `lfo_am_car`, `lfo_am_step`, and `lfo_am_out`. Both passed 147,456 calls.

The integrated differential replays the production Rust PG + core-owned EG + AM + operator path without oracle-injected Rust latches. It passed 147,456 `OPLL_Clock` calls across all 18 serial cycles. The compared scalar state includes PG increment/next phase/output, EG output/silence, operator inputs and feedback, PM/AM counters and latches, effective `c_instr`/`c_op`, FNUM/BLOCK/MULTI/KSL/TL/AR/DR/RR/SUS/VIB/AM and waveform-related latches. The fixture includes all-channel programming, a user patch with nonzero feedback, AM/VIB, both waveform bits, KEY-ON/KEY-OFF, live parameter changes and Test Mode values 0, 1, 2, 4 and 8.

The serial PG differential additionally compares raw `uint32_t` phase accumulators, `pg_phase_next`, `pg_inc`, and `uint16_t pg_out`. It reports 2,191,388 phase observations above bit 18, two phase-reset events, and 35 nonzero DOKON calls. It also runs controlled KEY-ON/KEY-OFF reset checks and a pinned MAME/ymfm arithmetic audit.

## Known open work

1. FM-3A differential closure is demonstrated on the recorded HEAD; retain the full differential as a mandatory regression gate.
2. The Rust workspace CI now passes Rustfmt, Clippy, unit/integration tests, observability-feature tests, workflow validation and documentation hygiene. The integrated core + PG service snapshot/restore, irregular block partitioning, clone independence, transactional rollback and retry tests are included in that CI.
3. EG characterization is closed: run 44852382767 passed on the exact PR #1133 HEAD and run 44865109038 passed on the stacked FM-3A code HEAD, each including the complete 38-configuration AR/DR/KSR sweep. The P1 gate on #1133 was answered with these exact run/HEAD references and resolved.
4. Perform the final dependency audit of PR #1133 → #1145 → #1147. Do not merge any of them until all stacked changes and findings are reviewed together.
5. FM-3 overall still needs complete nine-channel accumulation, channel output routing and later PCM validation. Rhythm synthesis/routing remains FM-5; do not pull it into FM-3A.
6. Preserve the distinction between the 19-bit sine phase coordinate and Nuked's raw 32-bit PG ring / 16-bit PG output latch. Do not restore the old 19-bit masks in the serial service or its differential.

## CI provenance

FM-3A PR #1147, branch `feat/ym2413-fm3-serial-operator`, validated code HEAD: `b538a6904c344fec5e1c5dc8bb08401d92c4c220`; subsequent scorecard/audit-only commits are documentation follow-ups. The PR is open and stacked on #1145; no merge was made.

- YM2413 Nuked PG differential workflow run `44865109031`: **SUCCESS**. This includes the integrated FM-3 differential (147,456 calls), isolated operator/feedback and AM LFO differentials, all 18 PG accumulators, raw serial PG service, KEY-ON/KEY-OFF reset checks and MAME/ymfm arithmetic audit.
- Rust CI run `44865109035`: **SUCCESS**. Rustfmt, Clippy, workspace unit/integration tests, observability feature Clippy/tests, workflow validation and documentation hygiene passed.
- External Nuked oracle adapter run `44865109047`: **SUCCESS**.
- Rhythm reset differential run `44865109138`: **SUCCESS**; this is not evidence that Rhythm Mode synthesis is complete.
- EG characterization run `44852382767` on the exact PR #1133 HEAD `c25511a71752f475b57c47b5e73b76e84b163a9b`: **SUCCESS**, including the complete 38-configuration AR/DR/KSR sweep.
- EG characterization run `44865109038` on the stacked FM-3A code HEAD `b538a6904c344fec5e1c5dc8bb08401d92c4c220`: **SUCCESS**, including the same 38-configuration sweep.
- Follow-up CI on the documentation-updated branch HEAD: run `44865413665` **SUCCESS**; follow-up PG differential run `44865413667` **SUCCESS**.
- Two P2 review threads on #1147 (effective carrier/rhythm output scope and Test Mode capture boundary), the P1 EG-characterization gate on #1133, and the P1 PG phase-write finding on #1145 are resolved with evidence. No review thread remains open across #1133/#1145/#1147.

Windows/desktop, Scheduler stress, SDL3, PSG, VDP and unrelated console gates were SKIPPED and are not approvals. The differential PASS closes the FM-3A operator datapath only; it does not close all FM-3, PCM, Rhythm Mode or SMS integration.
