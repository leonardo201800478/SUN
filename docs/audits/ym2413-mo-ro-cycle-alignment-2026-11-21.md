# YM2413 MO/RO cycle alignment audit — 2026-11-21

## Scope

This audit reconciles the output-pin routing stage in pinned Nuked-OPLL
commit `1269cf5a783b65583b50fa2464d08be75830aaa0` with the measured
MO/RO slot maps already represented by the Rust output service. It is a
software schedule analysis, not physical timing certification.

## Source execution order

In the pinned `opll.c`, `OPLL_Clock` calls:

1. `OPLL_PreparePatch1`
2. `OPLL_Channel`
3. `OPLL_PhaseGenerate`
4. `OPLL_Operator`
5. `OPLL_PhaseCalcIncrement`
6. EG output/KSL/EG generation
7. `OPLL_DoLFO`
8. `OPLL_DoRhythm`
9. `OPLL_PreparePatch2`

Therefore `OPLL_Channel` consumes the `ch_out` latch produced by the
**previous** `OPLL_Operator` invocation. The current operator result cannot
be routed to the output pins in the same pre-operator stage.

Pinned source:
https://github.com/nukeykt/Nuked-OPLL/blob/1269cf5a783b65583b50fa2464d08be75830aaa0/opll.c

## MO and RO have different cycle-to-measured-slot transforms

The two pins cannot be collapsed into one shared Nuked-cycle offset.

### MO

Nuked computes:

```c
ismod = (cycles / 3) & 1;
mute_m = ismod || ((rm_enable & 0x40) && (cycles + 15) % 18 >= 12);
```

With stable melodic mode (`rm_enable & 0x40 == 0`), MO is active on
Nuked cycles 0, 1, 2, 6, 7, 8, 12, 13 and 14. These correspond to the
measured carrier-slot positions 3, 4, 5, 9, 10, 11, 15, 16 and 17 using:

```text
mo_slot = (nuked_cycle + 3) % 18
```

With stable Rhythm Mode (`rm_enable & 0x40 != 0`), MO remains active only
on cycles 0, 1, 2, 6, 7 and 8, corresponding to measured slots 3, 4, 5,
9, 10 and 11 (channels 1..6).

### RO

Nuked's `mute_r` switch enables RO on cycles
16, 17, 0, 1, 2, 3, 4, 5, 9 and 10 when `rm_enable & 0x40` is set.
The `ch_out` source is separately gated by `rm_enable & 0x80` in
`OPLL_Operator`; a stable active RO value therefore requires both delayed
mode bits.

Those ten cycles correspond to the measured rhythm-output positions
13, 14, 15, 16, 17, 0, 1, 2, 6 and 7 using:

```text
ro_slot = (nuked_cycle + 15) % 18
```

This matches the existing measured RO occurrence map: original outputs at
13..17 and repeats at 0, 1, 2, 6 and 7.

## Sign-dependent neutral output

For the original YM2413 branch (not the YM2413B/YMF281B special branch),
`OPLL_Channel` does not simply write numeric zero when a pin is muted. It
derives a sign code from the previous `ch_out`; for nonnegative values it
increments both the active value and sign code. Consequently the neutral
output token is sign-dependent. The output service currently represents
active measured-domain pulses and `None`/no-event, but it does not expose
this per-slot neutral token.

This distinction is separate from the signed measured-level conversion
helper. `Ym2413DacCode::from_operator_channel_output` decodes the operator
one's-complement value to a signed measured amplitude; it does not reproduce
the full `OPLL_Channel` output-pin code or prove the physical DAC input word.

## Integration consequence

The existing `Ym2413RhythmOutputService::process_slot` accepts one shared
slot index and produces MO and RO results together. The Nuked compatibility
model instead requires distinct per-pin slot transforms and consumes the
previous operator output before the current operator tick. Directly calling
the current combined service once with the core's Nuked cycle would therefore
misalign at least one pin.

Do not integrate this service into `Ym2413PgSerialService` by simply calling
`process_slot(context.event.cycle, ...)` or by applying the RO transform to
both pins.

## Required adapter design before core integration

1. Consume the previous operator `ch_out` latch at the pre-operator
   `OPLL_Channel` boundary.
2. Derive MO and RO measured-slot indices independently:
   `(cycle + 3) % 18` for MO and `(cycle + 15) % 18` for RO.
3. Keep independent pin cursors/retention validation; do not force both
   schedules through one cursor unless a separately demonstrated common
   physical phase is established.
4. Apply delayed `rm_enable` bits independently: MO gating uses bit 6;
   RO gating uses bit 6 and the prior operator's bit-7-controlled `ch_out`.
5. Represent the sign-dependent neutral token explicitly, retaining +0/-0.
6. Keep the operator-to-measured-amplitude conversion separate from any raw
   nine-bit DAC-word hypothesis.
7. Add exact-cycle unit tests and a Nuked differential for the software
   `output_m/output_r` gating. Treat the measured output slot map and
   sub-clock active/neutral phase as separate evidence requirements.

## Decision

The output conversion helper is a valid bounded measured-domain API candidate,
but the current combined MO/RO service is **not ready for direct PG/core
integration**. Separate pin timing, independent cursors, and sign-dependent
neutral output must be modeled first. No core integration was made in this
audit.
