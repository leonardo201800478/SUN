# YM2413 Envelope Generator — Differential Investigation (2026-11-20)

## Scope and reference

This record covers the Envelope Generator (EG), its shared timer, operator-slot pipeline, and the differential work against the pinned Nuked-OPLL reference.

- Reference repository: https://github.com/nukeykt/Nuked-OPLL
- Reference revision: \`1269cf5a783b65583b50fa2464d08be75830aaa0\`
- Reference implementation: \`opll.c\`, especially the EG update and timer-latch path in \`OPLL_EnvelopeGenerate\`
- Tooling: \`tools/ym2413-nuked-oracle/\`
- Production component: \`native/ym2413/src/envelope.rs\`
- Experimental shared timer model: \`native/ym2413/src/envelope_timer.rs\`
- Validation PR: #1083 (draft; validation-only)
- Production boundary PR: #1082 (draft; not merged)
- Characterization tooling PR: #1076 (draft; not merged)
- Tracking issue: #1078

No result in this document establishes physical-hardware equivalence. The claims below are differential statements against the pinned reference and the stated captures.

## Confirmed EG transition ordering

### DAMP completion

The DAMP-to-ATTACK boundary is an update-entry decision. When the operator enters the update already at the DAMP completion threshold, the EG changes to ATTACK but does not consume an ATTACK increment in the same update.

The DAMP branch must also preserve its own decay step when that step reaches the off threshold. These are distinct cases:

- DAMP step reaches the threshold during the current update: preserve that DAMP step.
- DAMP begins the update at the completion threshold: transition to ATTACK and defer the ATTACK increment.

The production candidate in PR #1082 implements this distinction and has directed regression tests.

### ATTACK to DECAY

ATTACK that changes attenuation from 1 to 0 remains in ATTACK for that update. DECAY becomes visible at the next operator EG update. The production candidate uses the pre-update attenuation for this state decision.

### DECAY to SUSTAIN

A rate-matrix capture exposed the corresponding DECAY-to-SUSTAIN boundary. At level 31 with SL=4, an update increments the attenuation to 32, but Nuked retains DECAY for that update. SUSTAIN is selected on the following operator update, and that transition update does not also consume a SUSTAIN/RR increment.

The production candidate in PR #1082 now evaluates the SL comparator against the pre-update attenuation and defers the SUSTAIN/RR increment until the next update. Regression test:

- \`decay_to_sustain_transition_uses_pre_update_level_and_defers_sustain_rate\`

The focused YM2413 test workflow passed after this correction; the expanded Nuked differential is running separately.

## Counter and rate-latch pipeline

The trace inspection established a critical distinction between the fields visible after an OPLL clock and the fields consumed by the EG update:

- \`eg_rate\` is the newly prepared rate for the following slot.
- \`eg_rate_hi\`, \`eg_inc_hi\`, \`eg_inc_lo\`, and \`eg_maxrate\` are latches derived from the rate that was pending before the current slot.
- \`eg_zerorate\` reflects the newly prepared rate.
- The timer's phase locks use the previous slot's \`eg_counter_state_prev\`; that latch is updated later in the slot.
- The eight-bit counter increments at the slot-17 boundary.
- The timer shift register advances once per operator slot, with 18 slots per rotation.

This is a mixed one-slot pipeline. Feeding the post-call \`eg_rate\` directly into the same call's rate-latch calculation is incorrect. The timer model must retain the prior rate and compute the next phase from it.

The trace replay also distinguishes:
- the EG state before the current update,
- the state consumed by the current operator update,
- the state and latches visible after the update,
- the next rate prepared for the following operator.

These values must not be collapsed into a single post-call snapshot.

## Experimental phase-minus-three candidate

The first single-channel and four-channel captures showed that a DECAY/SUSTAIN counter candidate using \`global_counter - 3\` could eliminate the residual one-level differences after the DAMP and ATTACK boundary corrections:

- Single-channel fixture: 1,744 samples.
- Four-channel matrix fixture: 6,418 samples.

That result was fixture-specific. It was not sufficient evidence for a production counter correction.

A 38-configuration sweep was then run with:
- AR=10 and every DR nibble 0–15, for KSR=0 and KSR=1;
- additional AR values 0, 5, and 15 at DR=5, for both KSR settings;
- FNUM=128;
- block values 0, 2, 4, and 6;
- SL=4 and RR=7;
- the same user-patch setup on all four channels.

The pre-DECAY-to-SUSTAIN-fix results were:

| Candidate | Level mismatches | Stage mismatches |
|---|---:|---:|
| Deferred, unshifted counter | 56,334 | 328 |
| DECAY/SUSTAIN counter minus three | 44,064 | 143 |

The candidate matched several low/intermediate-rate configurations, but failed in other rate classes. In particular:
- failures increased for DR values 8 and above in some KSR configurations;
- the high-rate path showed mismatches even after the phase adjustment;
- AR=15 was not reproduced by the current SkipAttackToDecay candidate.

Therefore, a constant phase-minus-three correction is rejected as a universal production fix. It remains a diagnostic candidate only.

## High-rate DECAY divergence

A representative high-rate first mismatch occurs with AR=10, DR=13, KSR=0, at effective rate 55:

- Nuked pre-update state: DECAY.
- Nuked rate high field: 13.
- Nuked increment latches: \`inc_hi=0\`, \`inc_lo=0\`.
- Nuked step for that update: 0.
- Rust static fast-decay candidate: increment 1.

The Rust fast-decay table selects an increment from the global counter pattern, but the reference's fast-rate behavior is gated by the timer phase and rate latches. A constant counter offset does not reproduce those gates.

This points to missing shared timer/latch state in the current scalar-counter EG model. It is not evidence that the fast-decay table alone needs another arbitrary phase adjustment.

## AR=15 KEY-ON divergence

A representative AR=15 capture shows the Nuked pipeline consuming the delayed KEY-ON event with:
- maximum-rate condition active,
- DAMP/off pipeline active,
- resulting attenuation set to zero,
- resulting state set to ATTACK.

The current Rust diagnostic candidate configured as SkipAttackToDecay instead enters DECAY at the old attenuation level. This does not match the observed Nuked transition.

A separate EnterAttackAtZero hypothesis is required. It is not yet a production change and must be tested with the exact KEY-ON pipeline and subsequent ATTACK-to-DECAY update.

## Shared timer model

A standalone Rust component has been added experimentally at:

- \`native/ym2413/src/envelope_timer.rs\`

Its responsibilities are deliberately narrow:
- maintain the 18-bit timer shift state and carry;
- maintain the eight-bit counter and previous-counter latch;
- maintain low-phase and high-phase locks;
- preserve the one-slot rate-latch pipeline;
- produce the rate phase for the following operator update;
- export and restore the complete timer state.

It is not yet integrated into \`Ym2413Core\` or \`EnvelopeGenerator\`. It is not clocked by CPAL or the mixer. The future integration point is the YM2413 chip/slot clock, under the MachineScheduler's authority.

The timer's differential replay is being run against every fixture in the 38-configuration sweep. The acceptance condition for this component is exact agreement for all captured timer and rate-latch fields, not merely matching envelope levels.

## Remaining work

1. Complete the Rust timer replay for all 38 configurations and inspect any first mismatch.
2. Validate the DECAY-to-SUSTAIN correction against the same expanded sweep.
3. Add and test the AR=15 EnterAttackAtZero diagnostic candidate.
4. Replace the scalar-counter fast-decay approximation only after the shared timer model and rate-latch pipeline are validated.
5. Extend the fixtures to FNUM high-bit coverage and to distinct modulator/carrier patch parameters, so slot-to-parameter pipeline alignment is independently tested.
6. Integrate the timer with the YM2413 slot clock only after the standalone differential passes; include timer state in save/restore.
7. Re-run deterministic block-versus-continuous, reset, save-state, and workspace regressions.

## Merge status

PRs #1076, #1082, and #1083 remain drafts. No merge is authorized by this investigation.
