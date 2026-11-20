# YM2413 EG differential — Nuked-OPLL characterization (2026-11-20)

## Scope

This note records a single-channel, user-patch EG trace against the pinned
Nuked-OPLL reference:

- Reference repository: `nukeykt/Nuked-OPLL`
- Reference revision: `1269cf5a783b65583b50fa2464d08be75830aaa0`
- Oracle timebase: one `OPLL_Clock` call per trace row
- Capture length: 30,000 calls
- Active per-operator samples: 1,744
- Fixture: `tools/ym2413-nuked-oracle/fixtures/eg-single-channel.csv`

The fixture programs the custom user patch on channel 0, then asserts KEY-ON
and KEY-OFF. Its operator parameters are AR=10, DR=5, RR=7, SL=4,
EG-TYPE=1, KSR=0. FNUM=128 and block=4. EG-TYPE is bit 5 of each operator
mode register; bit 3 is part of the multiplier field, not EG-TYPE.

## Serial-slot and event mapping

The adapter captures the state after each `OPLL_Clock`. During that call,
Nuked updates:

```c
eg_level[(cycles + 16) % 18]
eg_state[(cycles + 16) % 18]
```

The state array index is associated with the operator whose key-on/patch data
was shifted into the two-cycle pipeline. Using the pinned `ch_offset[18]`
and `mcsel=((cycles+1)/3)&1` mapping:

- EG array index 2 is channel 0 carrier
- EG array index 17 is channel 0 modulator

Raw `kon[0]` is not the effective per-slot EG key-on. The EG pipeline shifts
`eg_kon` during the call; the post-call bit 2 (`eg_kon & 0x04`) identifies
the bit-1 key-on value consumed by the EG update in that call. The extractor
uses this effective per-slot event, avoiding a one-slot-early KEY-ON for the
modulator.

For the selected update rows, `global_counter = call_index / 18`. Its low
eight bits match Nuked's `eg_counter_state` at every selected row. This
establishes the phase mapping for this fixture; it does **not** imply that the
Rust counter should be truncated to eight bits.

## Differential candidates

The Rust example replays the production `EnvelopeGenerator` API for both
channel-0 operators. It is a diagnostic harness; no production component was
changed. The comparison uses identical call indices and per-slot counter
values, with no correlation-based alignment.

| Candidate | Level mismatches / 1,744 | Maximum absolute error | Stage mismatches |
|---|---:|---:|---:|
| Initial level 0, immediate transition | 1,726 | 127 | 886 |
| Initial level 127, immediate DAMP-to-ATTACK update, unwrapped counter | 254 | 8 | 16 |
| Initial level 127, immediate update, counter truncated to 8 bits | 1,499 | 8 | 16 |
| Initial level 127, deferred attack update, counter truncated to 8 bits | 1,263 | 3 | 2 |
| **Initial level 127, deferred attack update, unwrapped counter** | **18** | **1** | **2** |

The best candidate is the final row. Its diagnostic wrapper preserves the
DAMP-to-ATTACK state transition but restores the pre-update level when the
transition requests a phase reset, thereby not consuming the ATTACK increment
on that same operator update. This is a test candidate only; it is not a
production patch.

### Residual level mismatches

All 18 residual level mismatches are one attenuation step, with the Rust
candidate one level higher than Nuked. They occur at these global-counter
values, for both operator slots:

- 256, 257, 258
- 512, 513, 514
- 768, 769, 770

There are no other level mismatches in this fixture for the deferred,
unwrapped candidate. The pattern is consistent with a remaining EG rate
selector / timer-phase difference near these counter boundaries. It does not
justify changing the static rate table or truncating the global counter.

### Residual stage mismatches

Two stage mismatches remain:

- call 2452, carrier slot 2: Nuked remains ATTACK at level 0; Rust has already
  entered DECAY at level 0
- call 2467, modulator slot 17: same boundary one operator update later

This suggests that the Nuked ATTACK-to-DECAY state transition is observed one
operator update after the level first reaches zero. The current Rust candidate
changes stage on the same update that reaches zero. This is a separate
boundary from DAMP-to-ATTACK and must be tested independently.

## Findings and limits

1. The correct KEY-ON stimulus for an operator is the pipelined EG event, not
   the raw channel register bit.
2. Starting this candidate at level 127 is substantially closer to the
   Nuked reset/key-on trajectory than starting at level 0.
3. Deferring the first ATTACK increment after DAMP completion reduces the
   level mismatch count from 254 to 18 in this fixture.
4. Simply truncating the Rust global counter to eight bits is not supported;
   it increases the mismatch count sharply.
5. The residuals are tightly localized: 18 one-step level mismatches at three
   256-counter boundaries and two ATTACK-to-DECAY stage-boundary mismatches.

This is one user-patch configuration on one channel and one pinned software
reference. It is not proof of physical YM2413 behavior, and it does not cover
the full AR/DR/RR/KSR/SL/EG-type/SUS parameter space, all nine channels,
rhythm mode, or every chip revision.

## Next validation

Before changing production code:

1. Add isolated fixtures for effective EG rate low-bit rows 0, 1, 2 and 3,
   varying KSR and block while holding the other operator parameters fixed.
2. Repeat the attack-to-decay boundary with multiple AR values, including
   rates below and above the fast-rate boundary.
3. Add independent EG-type and SUS/key-off release vectors.
4. Compare the same vectors against a second independently implemented
   reference where internal EG state is available.
5. Only then decide whether the production API needs an explicit deferred
   transition policy, a different EG rate phase source, or both.

**Production status:** unchanged. No YM2413 envelope behavior was modified.
