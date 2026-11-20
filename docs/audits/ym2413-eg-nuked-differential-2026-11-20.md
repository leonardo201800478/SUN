# YM2413 EG differential — Nuked-OPLL characterization (2026-11-20)

## Reproducibility record

- Sega Emulator branch: tools/ym2413-nuked-oracle-trace
- Captured HEAD: 474e32ed1d48c093c4e2965a3e3380623bc6fac3
- Production base commit: 0e8025e476eee12135f4c0f7def9dead7dfcaa14
- Production EG source blob: 1b97fb803b8bcefb1d9243dbb96f71cef5ff3503
- Rust diagnostic example blob: eeca3f3077283e719cc69e08ad69dada6bab4e5b
- Extractor blob: 724db58facbf1cbf0c35008fd10be21fcc9625a1
- Pipeline analyzer blob: dd3e9c055f10d6440414a9adf9f5f4027ed91f52
- Reference repository: nukeykt/Nuked-OPLL
- Reference revision: 1269cf5a783b65583b50fa2464d08be75830aaa0
- Reference source file: opll.c (blob 02adfe0bf32bf2771046f8729cc5a2709a96a0cb)
- EG workflow run: 44686356353
- CI workflow run: 44686356334
- Adapter workflow run: 44686356384
- Artifact ID: 14606691958
- Artifact SHA-256: 37e87a675c76d9dde59e2da8a2b85fc1a5d2909d3a4872cef7d6c2b41766381e
- Oracle timebase: one OPLL_Clock call per trace row
- Capture length: 30,000 calls per fixture

The reference is a pinned software oracle, not a physical-chip measurement.

## Critical extractor correction

The initial extractor used post-call c_fnum and c_block as though they
belonged to the EG slot updated during that call. They are pipeline fields and
can already describe a different serial slot.

The corrected extractor uses the stable channel register arrays:

- fnum[channel]
- block[channel]

For the first channel-0 carrier sample, the correct values are FNUM=128 and
BLOCK=4; the previous extraction incorrectly emitted FNUM=0 and BLOCK=0.
That incorrect parameter association inflated the comparison to 1,054 level
mismatches and 95 stage mismatches. Those figures are rejected and must not be
used as evidence against the production rate selector.

The corrected mapping is in extract_eg_samples.py. The fixture-specific
user-patch fields AR/DR/RR/SL/KSR/EG-TYPE remain sourced from the active patch
fields; FNUM/BLOCK are sourced from the selected channel.

## Serial-slot and KEY-ON mapping

The adapter records state after each OPLL_Clock. During that call Nuked
updates:

~~~c
eg_level[(cycles + 16) % 18]
eg_state[(cycles + 16) % 18]
~~~

Using the pinned ch_offset[18] and mcsel=((cycles+1)/3)&1 mapping:

- EG array index 2 is channel 0 carrier.
- EG array index 17 is channel 0 modulator.

Raw channel kon[channel] is not the effective per-slot EG event. The EG
pipeline shifts eg_kon during the call; post-call bit 2 (eg_kon & 0x04)
identifies the bit-1 KEY-ON value consumed by that slot's EG update.

For selected rows, global_counter = call_index / 18. Its low eight bits
match Nuked's eg_counter_state at every selected row. This does not imply
that Rust should truncate its counter; the full counter candidate is
substantially better.

## Reference pre-state replay validation

analyze_eg_pipeline.py reconstructs each selected Nuked EG update from the
preceding OPLL_Clock post-state and checks it against the current call's
post-state. This verifies the exact consumed state, rather than inferring it
from the current post-call fields.

| Capture | Selected samples | Reconstructed updates | Replay errors |
|---|---:|---:|---:|
| Channel 0 | 1,744 | 1,744 | 0 |
| Channels 0–3 | 6,418 | 6,418 | 0 |

The analyzer also reports the exact pre-update eg_rate_hi, eg_inc_lo,
eg_inc_hi, eg_timer_shift_lock, eg_timer_low_lock, KEY-ON and DOKON
pipeline state for the first Rust mismatch.

## Corrected differential results

The Rust example replays the production EnvelopeGenerator API. Candidate
wrappers are experimental and do not alter production code. Comparisons use
exact call_index and per-slot counters; there is no correlation-based or
automatic time alignment.

### Single-channel capture

Fixture: eg-single-channel.csv (blob
4a7436ff882586de26a3c03859ba589fe1c08be7).

User patch parameters: AR=10, DR=5, RR=7, SL=4, EG-TYPE=1, KSR=0.
Channel 0 FNUM=128 and BLOCK=4.

| Candidate | Level mismatches / 1,744 | Maximum absolute error | Stage mismatches |
|---|---:|---:|---:|
| Initial level 0, immediate transition | 1,726 | 127 | 886 |
| Initial level 127, immediate DAMP-to-ATTACK, unwrapped counter | 254 | 8 | 16 |
| Initial level 127, immediate update, counter truncated to 8 bits | 1,499 | 8 | 16 |
| Initial level 127, deferred attack update, counter truncated to 8 bits | 1,263 | 3 | 2 |
| **Initial level 127, deferred attack update, unwrapped counter** | **18** | **1** | **2** |

The best candidate preserves the DAMP-to-ATTACK transition but restores the
pre-update level when that transition requests a phase reset. This prevents
the first ATTACK increment from being consumed on the same update. It remains
a diagnostic candidate, not a production policy decision.

### Four-channel rate matrix

Fixture: eg-rate-lowbits-matrix.csv (blob
79a70c0b950e65d7a8efad3ed5ae54836bc14a53).

The matrix exercises four channels with FNUM=128, BLOCK values 0, 2, 4 and 6,
and the same user-patch rate fields.

| Candidate | Level mismatches / 6,418 | Maximum absolute error | Stage mismatches |
|---|---:|---:|---:|
| Initial level 0 | 6,315 | 127 | 4,008 |
| Initial level 127, immediate DAMP-to-ATTACK, unwrapped counter | 328 | 8 | 14 |
| Initial level 127, deferred attack update, wrapped counter | 4,629 | 3 | 8 |
| **Initial level 127, deferred attack update, unwrapped counter** | **60** | **1** | **8** |

The unwrapped/deferred candidate remains the best of the tested variants.
The matrix does not establish full YM2413 rate coverage; it exercises the
selected four-channel fixture only.

## Residual level mismatches

For the single-channel capture, all 18 residual level mismatches are one
attenuation step, with Rust one level higher than Nuked. They occur for both
operator slots at global-counter values:

- 256, 257, 258
- 512, 513, 514
- 768, 769, 770

The matrix has 60 residual level mismatches, also with maximum absolute error
1, at the same counter windows:

- 256–258: 24 mismatches
- 512–514: 12 mismatches
- 768–770: 24 mismatches

These residuals are not evidence for truncating the counter. The unwrapped
candidate has far fewer mismatches. Their exact rate-selector/timer-phase
cause remains open; do not change the static rate table or counter policy
without a targeted fixture that isolates the relevant low-rate update.

## ATTACK-to-DECAY transition: confirmed one-update boundary

The corrected capture isolates a real state-transition difference.

In Nuked OPLL_EnvelopeGenerate, zero is computed from the level before the
current increment. In ATTACK, the next-state decision therefore remains
ATTACK on the update that changes level from 1 to 0. The following operator
update sees the pre-update level already at zero and transitions to DECAY.

The production Rust implementation currently advances ATTACK and then checks
whether the resulting level is zero, transitioning in the same update. This
accounts for the two single-channel stage mismatches:

| Call | Slot | Nuked level/state | Rust candidate level/stage |
|---:|---:|---|---|
| 2452 | 2 (carrier) | 0 / ATTACK | 0 / DECAY |
| 2467 | 17 (modulator) | 0 / ATTACK | 0 / DECAY |

The matrix produces eight instances of the same one-update boundary:

- call 2473, slot 5, counter 137
- call 2476, slot 8, counter 137
- call 2613, slot 1, counter 145
- call 2616, slot 4, counter 145
- call 3029, slot 3, counter 168
- call 3044, slot 0, counter 169
- call 3619, slot 17, counter 201
- call 3622, slot 2, counter 201

An independent reverse-engineering model in Andete's
results/20170126/model5.cc performs the ATTACK && env == 0 state check before
calculating the next envelope update, supporting the same ordering.

This is sufficient evidence for a narrowly scoped production correction:
evaluate the ATTACK zero-level transition at the beginning of the EG update,
and do not run DECAY on that same transition update. The production change
must be isolated in a separate PR, with a directed regression test and a
rerun of both EG captures. No production change is included in PR #1076.

## Findings and limits

1. The selected operator's KEY-ON is the pipelined EG event, not the raw
   channel register bit.
2. FNUM/BLOCK must be read from the selected channel arrays, not the
   post-call c_fnum/c_block pipeline fields.
3. Initial level 127, deferred DAMP-to-ATTACK increment, and an unwrapped
   counter are the best tested candidate for these fixtures.
4. The remaining level residuals are one-step errors at three 256-counter
   boundaries; their cause is unresolved.
5. ATTACK-to-DECAY is one operator update later in Nuked and the independent
   Andete model than in current production Rust.
6. The analyzer reproduces all captured Nuked selected-slot EG updates with
   zero internal replay errors. This validates trace interpretation, not
   physical silicon equivalence.

This remains a software differential against one pinned reference. It does
not prove absolute hardware timing and does not cover all AR/DR/RR/KSR/SL,
EG-TYPE/SUS combinations, all nine channels, rhythm mode, every chip revision,
or the complete audio path.

**Production status at this capture:** unchanged. PR #1076 contains tooling
and diagnostics only. The ATTACK-to-DECAY correction is to be developed and
validated separately; no merge is authorized by this record.
