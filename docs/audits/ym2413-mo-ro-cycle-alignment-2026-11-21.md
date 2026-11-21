# YM2413 MO/RO cycle alignment audit — corrected 2026-11-21

## Scope and evidence boundary

This audit reconciles the measured MO/RO output-slot maps with the pinned
Nuked-OPLL software schedule at commit
`1269cf5a783b65583b50fa2464d08be75830aaa0`. It corrects an earlier
interpretation that incorrectly assigned different cycle offsets to MO and
RO and treated `rm_enable & 0x40` as a persistent Rhythm Mode flag.

The measured slot map comes from the project's reverse-engineering evidence.
Nuked's `output_m/output_r` behavior is a software model and is not itself
physical timing evidence. The RO switch in this source is explicitly marked
`TODO: This might be incorrect`.

## Clock order and pipeline

At the beginning of `OPLL_Clock`, Nuked copies the previously computed
`output_m/output_r` into the caller's buffer. It then shifts
`rm_enable >>= 1`, applies a pending mode-register write, calls
`OPLL_Channel`, and only afterwards calls `OPLL_Operator`. The operator
updates `ch_out` after the channel stage.

Consequences:

1. The output buffer is one `OPLL_Clock` call behind the pin values just
   computed by `OPLL_Channel`.
2. `OPLL_Channel` consumes the previous operator's `ch_out` latch.
3. An adapter must preserve both pipeline boundaries; routing the current
   operator result in the same pre-operator channel stage is incorrect.

Pinned source:
https://github.com/nukeykt/Nuked-OPLL/blob/1269cf5a783b65583b50fa2464d08be75830aaa0/opll.c

## Shared Nuked-cycle to measured-slot coordinate

The measured 18-slot rotation uses one coordinate transform for both pins:

```text
measured_slot = (nuked_cycle + 15) % 18
```

The full cycle map is:

| Nuked cycle | Measured slot | Nuked cycle | Measured slot |
|---:|---:|---:|---:|
| 0 | 15 | 9 | 6 |
| 1 | 16 | 10 | 7 |
| 2 | 17 | 11 | 8 |
| 3 | 0 | 12 | 9 |
| 4 | 1 | 13 | 10 |
| 5 | 2 | 14 | 11 |
| 6 | 3 | 15 | 12 |
| 7 | 4 | 16 | 13 |
| 8 | 5 | 17 | 14 |

This coordinate transform aligns the existing measured slot maps:

- MO in Rhythm Mode: measured slots 3, 4, 5, 9, 10 and 11. These
  correspond to Nuked cycles 6, 7, 8, 12, 13 and 14.
- MO in melodic mode: slots 3..5, 9..11 and 15..17.
- RO measured occurrences: original slots 13..17 and delayed repeats at
  slots 0, 1, 2, 6 and 7. These correspond to Nuked cycles 16, 17, 0, 1,
  2, 3, 4, 5, 9 and 10.

The earlier claim that MO used `(cycle + 3) % 18` was an inverse-offset
error. MO and RO share the `+15` coordinate transform; their active windows,
source selection and retained state remain distinct.

## What the pinned Nuked gating does — and does not — establish

Nuked computes:

```c
ismod = (cycles / 3) & 1;
mute_m = ismod || ((rm_enable & 0x40) && (cycles + 15) % 18 >= 12);
```

The base `ismod` schedule is active on cycles 0..2, 6..8 and 12..14,
which maps to measured slots 15..17, 3..5 and 9..11 respectively.

However, `rm_enable & 0x40` must not be treated as a persistent Rhythm Mode
boolean. In `OPLL_Clock`, `rm_enable` is shifted right every call; the
mode-write handler loads its upper pipeline bit when register 0x0e is
written. Bit 6 is therefore a delayed write-pipeline state. It can affect
the transition schedule but does not, by itself, establish steady-state
Rhythm Mode MO gating.

For RO, Nuked enables `mute_r = 0` on cycles 16, 17, 0..5, 9 and 10 only
when `rm_enable & 0x40` is set. Under the shared `+15` transform those
cycles line up with the ten measured RO occurrence positions. This is a
useful software schedule correspondence, but the conditional is tied to
the delayed write pipeline and the source itself marks the RO switch as
possibly incorrect. It is not proof of steady-state physical pin gating.

The operator's rhythm-source selection is separately affected by
`rm_enable & 0x80`; this is another delayed pipeline bit, not a substitute
for the persistent rhythm-register state. The adapter must consume the
actual delayed state and the correct previously latched source rather than
synthesizing a constant "stable rm_enable" value.

## Sign-dependent neutral output

For the original YM2413 branch (excluding the YM2413B/YMF281B special
branch), `OPLL_Channel` derives:

```c
sign = ch_out >> 8;
if (ch_out >= 0) {
    ch_out++;
    sign++;
}
output_pin = muted ? sign : ch_out;
```

Thus the software neutral token is `-1` when the prior `ch_out` is
negative and `+1` when it is nonnegative. Active positive values are also
incremented before being assigned to the pin. This is a signed software
representation; it must not be confused with the signed measurement-domain
amplitude token or claimed as a proven physical raw DAC input word.

The current `Ym2413DacCode::from_operator_channel_output` helper performs
only a one's-complement-to-signed-measured-amplitude conversion. It does
not implement the `OPLL_Channel` positive increment, the sign-dependent
neutral token, or the delayed output buffer. It remains an isolated
conversion helper and is not a pin adapter.

## Integration consequence

A single shared 18-slot cursor is structurally compatible with the measured
MO and RO maps because both use the same `(cycle + 15) % 18` coordinate.
The existing `Ym2413RhythmOutputService::process_slot` therefore does not
need separate per-pin slot cursors solely to compensate for different
offsets.

It is nevertheless **not ready for direct PG/core integration**. The service
currently emits active pulse events and no-event results; it does not model
the per-clock sign-dependent neutral pin value, the previous-`ch_out`
latch, the output-buffer delay, or the transient `rm_enable` pipeline.
Its MO/RO inputs must be sourced from the correct prior operator latch and
delayed rhythm values at the mapped slot.

## Required adapter design

1. Convert the 0..17 Nuked/core cycle to the shared measured slot with
   `output_slot_for_nuked_cycle(cycle)`.
2. Preserve the `OPLL_Clock` ordering: consume the prior `ch_out` at the
   channel stage; update the next `ch_out` only after the current operator
   stage; preserve the additional output-buffer delay where the host-facing
   pin sample is captured.
3. Keep MO and RO source selection and active/muted gating independent,
   while using the same slot coordinate.
4. Model `rm_enable` as a shift-register/write-pipeline state. Do not pass
   a persistent Rhythm Mode boolean in its place.
5. Represent the original YM2413 sign-dependent neutral token and the
   positive active-value increment explicitly. Keep this software pin-code
   representation separate from `Ym2413DacCode` measured amplitudes and
   from any raw nine-bit DAC-word hypothesis.
6. Differentially compare the complete `output_m/output_r` traces across
   register writes, mode transitions, all 18 cycles and both output-buffer
   phases. Treat Nuked agreement as software-model compatibility, not
   physical timing certification.
7. Preserve the measured MO/RO slot evidence independently of Nuked's
   uncertain RO gating implementation.

## Decision

The prior "different MO/RO cycle transforms and independent cursors" claim
is withdrawn. The corrected shared coordinate is `(cycle + 15) % 18`.
The previous audit's interpretation of `rm_enable & 0x40` as a stable
mode flag is also withdrawn. A shared cursor may be used, but the adapter
must still model prior-`ch_out`, pin-specific gating/source, sign-dependent
neutral values and output-buffer delay. No core integration is made by this
audit.
