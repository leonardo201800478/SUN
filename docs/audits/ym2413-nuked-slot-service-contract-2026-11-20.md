# YM2413 serial slot-service contract — Nuked-OPLL compatibility (2026-11-20)

## Scope and provenance

This document records the serial call ordering and array offsets used to design FM-1B. It is based on the external Nuked-OPLL source pinned at commit `1269cf5a783b65583b50fa2464d08be75830aaa0`:

- Repository: https://github.com/nukeykt/Nuked-OPLL
- Source: `opll.c`
- Reference role: primary software compatibility oracle, not a physical-chip measurement.

No Nuked source, tables, or implementation structures are copied into the Rust backend. The contract records observable source-level ordering and index relationships only.

## 1. OPLL_Clock stage order

One Nuked `OPLL_Clock` call executes the following order:

1. Capture the previously retained MO/RO output values.
2. At cycle 0, latch the AM output from the current AM counter.
3. Shift/apply mode writes and advance the rhythm selector.
4. `OPLL_PreparePatch1`.
5. `OPLL_Channel`.
6. `OPLL_PhaseGenerate`.
7. `OPLL_Operator`.
8. `OPLL_PhaseCalcIncrement`.
9. `OPLL_EnvelopeOutput`.
10. `OPLL_EnvelopeKSLTL`.
11. `OPLL_EnvelopeGenerate`.
12. `OPLL_DoLFO`.
13. `OPLL_DoRhythm`.
14. `OPLL_PreparePatch2`.
15. `OPLL_DoRegWrite`.
16. `OPLL_DoIO`.
17. Increment the serial cycle modulo 18.

This order matters. In particular, Operator runs before the current call's EG update and before the next PG increment is calculated. The current call's PG increment is consumed by a later PhaseGenerate call. The operator therefore consumes retained pipeline state; it must not be modeled as an unordered combination of freshly advanced PG and EG values.

## 2. Nuked cycle to canonical Yamaha slot map

Nuked selects the current channel from:

```
ch_offset = [1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8, 0]
mcsel = ((cycle + 1) / 3) & 1
```

For every cycle 0..17, the pair `(ch_offset[cycle], mcsel)` matches:

```
YM2413_SLOT_MAP[(cycle + 1) % 18]
```

Thus the selected logical operator is a one-position rotation of the canonical physical slot map. The mapping is bijective across all eighteen slots.

The new Rust helpers `nuked_cycle_selected_slot_index` and `nuked_cycle_selected_slot_descriptor` codify this compatibility mapping. The existing `Ym2413Core::next_slot_descriptor` behavior is intentionally unchanged: it continues to expose the physical slot-clock position and is not silently reinterpreted as a Nuked internal-cycle descriptor.

## 3. Stage-specific array offsets

The following expressions are distinct and must remain explicit:

| Nuked stage | Array entry |
|---|---|
| PhaseGenerate writes the delayed PG accumulator | `(cycle + 17) % 18` |
| PhaseGenerate reads the current PG phase for `pg_out` | `cycle` |
| EnvelopeGenerate updates one EG operator | `(cycle + 16) % 18` |
| EnvelopeOutput reads one EG level | `(cycle + 17) % 18` |

Rust exposes these as `nuked_pg_phase_write_index`, `nuked_pg_phase_output_index`, `nuked_eg_update_index`, and `nuked_eg_output_index`.

The current call's Operator consumes the retained `eg_out` produced by the prior call. Its source entry is the current call's EG update index `(cycle + 16) % 18`; that value is read before the current EG update and is then advanced by EnvelopeGenerate. The current call's EnvelopeOutput samples `(cycle + 17) % 18` after Operator and prepares the `eg_out` latch for the next call. Therefore `eg_update_index` is both the current operator's pre-update EG input and the target of its post-operator EG update, while `eg_output_index` feeds the next call's EG output latch.

The EG output is a retained latch: Operator consumes the previous `eg_out`; the current call computes a new `eg_out` only after Operator. Similarly, PhaseGenerate applies the previously calculated `pg_inc` and `pg_phase_next`, while PhaseCalcIncrement computes the next increment later in the same call.

The PG next-phase reset request is selected from the pre-update EG `eg_dokon` latch. Nuked uses bit `0x8000` when its PG selector classifies the cycle as a modulator, otherwise bit `0x01`. In Rhythm Mode, cycles 13 and 14 force the selector to the carrier path when `rm_enable & 0x40` is set. The Rust helper `nuked_phase_reset_required` encodes this predicate and must be called with post-shift/post-mode-write `rm_enable` and pre-update `eg_dokon`. Its sole future consumer is the PG `pg_phase_next` latch.

The offsets do not define subphases within the four input clocks of a Rust slot. No physical sub-clock position is inferred.

## 4. LFO and register-latch implications

- PM is consumed by PhaseCalcIncrement before DoLFO advances the vibrato counter at cycle 17.
- AM output is latched at cycle 0 before the current call's DoLFO work.
- PreparePatch1 supplies the current patch fields used by the earlier pipeline stages.
- PreparePatch2 refreshes the frequency/block/multiplier/vibrato and related latches after EG and rhythm processing; these values feed a subsequent PG calculation.
- Register writes are applied near the end of OPLL_Clock, after the synthesis stages for that call.

A Rust service must preserve these causal boundaries and snapshot every pending latch needed to resume in the middle of the 18-cycle rotation.

## 5. Implemented evidence in this change

- Exhaustive 18-cycle bijection test between Nuked's channel/operator selection and `YM2413_SLOT_MAP`.
- Exhaustive offset tests for all PG and EG array indices, including wrap.
- Invalid-cycle rejection tests.
- No changes to PG/EG arithmetic, Scheduler, PSG, VDP, Z80, SMS runtime, or PCM.

## 6. FM-1B implementation constraints

The next implementation must:

1. Keep `Ym2413Core` as the sole owner of the serial `EnvelopeSequencer` and its delayed control latches.
2. Keep the synthesis bank from advancing a second EG for the same serial operator.
3. Represent the selected logical operator, PG write target, PG output source, EG update target, and EG output source as separate fields in the slot-service event.
4. Give the phase-reset event exactly one consumer in the PG pipeline, using the Nuked-compatible delay and slot mapping.
5. Preserve modulator feedback history and all pending phase/operator latches in snapshots.
6. Apply the same order for continuous and partitioned input-clock advancement.
7. Compare every event against the pinned external Nuked adapter before promoting the contract beyond software-oracle compatibility.

The immutable `Ym2413SlotServiceEvent` combines these stage-specific indices with the selected logical operator and the PG reset request. `nuked_slot_service_event` rejects cycles outside 0..17. This event is the intended hand-off object for FM-1B; it does not advance either core or synthesis state.

The immutable `Ym2413SlotServiceEvent` combines these stage-specific indices with the selected logical operator and the PG reset request. `nuked_slot_service_event` rejects cycles outside 0..17. This event is the intended hand-off object for FM-1B; it does not advance either core or synthesis state.

The exact reset origin on physical silicon, electrical MO/RO transfer, and analog board response remain unmeasured. This contract does not claim hardware corroboration.
