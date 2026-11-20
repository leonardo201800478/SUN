# YM2413 external Nuked-OPLL oracle trace

This tool records the public internal state exposed by the pinned Nuked-OPLL
`opll_t` structure at every OPLL_Clock call. It is a **separate external
reference tool**; no Nuked implementation source is copied into this repository.

## Reference pin

- Repository: https://github.com/nukeykt/Nuked-OPLL
- Commit: `1269cf5a783b65583b50fa2464d08be75830aaa0`
- API used: `opll.h` (`opll_t`, `OPLL_Reset`, `OPLL_Clock`, `OPLL_Write`)
- Chip type default: `opll_type_ym2413 = 0`

Clone/check out that exact revision independently. The harness
links against its `opll.c` and includes its `opll.h` at build time. Do not
copy either file into this repository, vendor the reference, or link the
reference into the emulator backend.

## Build

From this directory, with a C11 compiler and the reference checkout at
`../../third_party/Nuked-OPLL` (or adjust the include/source paths):

```sh
cc -std=c11 -O2 -Wall -Wextra -I../../third_party/Nuked-OPLL \
  trace.c ../../third_party/Nuked-OPLL/opll.c -o ym2413-nuked-trace
```

On Windows, use an available C11 compiler such as clang or MinGW GCC and
adapt the output executable suffix as needed.

## Input events

Optional CSV, with exactly one event per strictly increasing OPLL_Clock call index:

```csv
# call_index,port,value
0,0,16
2,1,64
40,0,32
42,1,0
80,0,48
82,1,16
```

Port 0 queues an address-port write; port 1 queues a data-port write, using
Nuked's own `OPLL_Write` API. Rows are submitted immediately before the
corresponding `OPLL_Clock` call. Address/data timing and internal write
acceptance remain the reference implementation's behavior; this harness does
not bypass its IO state machine. CSV fields are decimal integers. Keep address and data requests separated by
at least two OPLL_Clock calls so the reference IO synchronizer can latch them.
The sample schedule leaves additional time for the serial register-write
pipeline to reach its channel slot.

Run:

```sh
./ym2413-nuked-trace --calls 20000 --events sequence.csv > trace.jsonl
```

Without `--events`, the chip remains in its reset register state.
`--chip-type 0` selects YM2413 explicitly.

## Trace semantics

- The first JSONL record is a schema descriptor.
- Each following record corresponds to one call to `OPLL_Clock`.
- `call_index` is the zero-based OPLL_Clock invocation index.
- `cycle_before` is Nuked's serial cycle selected before the call;
  `cycle_after` is the value after its 18-step rotation update.
- `output_m/output_r` are the two values copied to the API buffer at the
  beginning of that call.
- `output_pins_m/output_pins_r` and the internal arrays are sampled after
  `OPLL_Clock` returns.
- Arrays retain Nuked's native order. In particular, do not assume its
  `cycles` index is identical to the Rust crate's zero-based slot index
  without a separately tested mapping.

Each sample also contains `calc_fnum`, `calc_block`, `calc_mul`, `calc_vib`,
`calc_lfo_vib_counter`, and `pg_inc_after`. The `calc_*` fields are captured
immediately before the call and are the inputs consumed by
`OPLL_PhaseCalcIncrement` during that call; `pg_inc_after` is captured after
that calculation. This creates a correctly paired PG arithmetic observation
without confusing the next slot's `c_*` values with the previous increment.

The trace includes PG phase/increment, the active channel/operator patch
parameters, EG timer and per-operator state,
LFO counters, feedback histories, operator log/EXP pipeline values, channel
outputs, register-decoded channel parameters, rhythm/noise state, and MO/RO
pin values.

## Important limitations

1. This is a software-oracle trace, not a physical measurement.
2. It records Nuked's internal state and outputs; it does not by itself prove
   that Nuked is electrically exact or free of implementation defects.
3. It does not compare against Rust automatically. A later differential
   comparator must map state fields and time boundaries explicitly.
4. The harness must not infer four functional subphases from the Rust
   four-input-clock slot budget. Nuked advances one serial `cycles` step per
   `OPLL_Clock` call. Its 18-step loop represents the serial operator traversal
   in that software model; it must not be mislabeled as 18 physical XIN clocks.
   The 72-XIN-clock rotation belongs to separate hardware timing evidence and
   must be mapped explicitly before combining the traces.
5. The reference is GPL-2.0. Keep the reference checkout and linked executable
   outside the Sega Emulator distribution. This repository contains only the
   independently authored adapter source and build instructions. Review
   licensing before redistributing binaries or derivative fixtures.

The purpose is to provide repeatable, commit-pinned diagnostic traces for
investigating discrepancies. It is not permission to copy or port Nuked code.

## Automated smoke test

The repository workflow `.github/workflows/ym2413-nuked-oracle.yml` checks
out the pinned reference in the ephemeral CI runner, compiles this adapter,
parses an 80-call JSONL trace, validates array dimensions and serial-cycle
progression, and checks rejection of duplicate event indices and unsupported
chip types. It does not publish or retain the GPL reference binary.


## Compact PG-only mode

Pass `--pg-only` to emit a compact `ym2413-nuked-opll-pg-trace-v1`
JSONL stream. It retains only the per-call inputs consumed by
`OPLL_PhaseCalcIncrement`, the serial cycle index, and the calculated
`pg_inc`. This mode is intended for long LFO-period differential runs where
full EG/operator arrays add no value to the arithmetic comparison.


## Compact PG phase-array mode

Pass `--pg-phase-only` to emit a compact trace containing the 18-entry
`pg_phase` accumulator array after each `OPLL_Clock` call, plus the PG
increment inputs and serial-cycle index. This mode supports differential
validation of the 18 operator accumulators without the unrelated EG/operator
diagnostic arrays.


The PG phase-array trace also records `calc_eg_dokon`, `calc_testmode`,
`calc_rm_enable`, and `phase_next_reset`. The reset flag describes the
phase-generator reset decision made during that call; its effect is applied
to the previous slot on the following serial call, matching the
`pg_phase_next` pipeline.


## EG characterization trace

The full trace also exposes Nuked's EG timer/selector state:
`eg_counter_state`, `eg_counter_state_prev`, `eg_timer`, carry and
shift locks, `eg_kon`, `eg_dokon`, `eg_off`, `eg_rate`,
`eg_rate_hi`, increment selectors, `eg_sl`, and silence flags. Active
patch fields include `c_adrr`, `c_et`, `c_sl`, and `c_ksr_freq`.

The fixture `fixtures/eg-single-channel.csv` programs a custom user patch on
channel 0, starts KEY-ON, then KEY-OFF. It uses AR=10, DR=5, RR=7, SL=4 and
EG-TYPE=1 (bit 5 of each operator mode register). The characterization workflow captures 30,000
`OPLL_Clock` calls twice, requires byte-identical traces, and validates the
serial EG arrays and observed register/key transitions.

Nuked updates `eg_level[(cycles + 16) % 18]` and
`eg_state[(cycles + 16) % 18]` once per `OPLL_Clock` call. For the pinned
source's `ch_offset[18]` and `mcsel=((cycles+1)/3)&1`, channel 0's
carrier update maps to EG array index 2 and its modulator update maps to
index 17. These are source-derived mappings, not an assumed Rust slot map.

This is deliberately a **reference characterization**, not yet a Rust
differential pass. Nuked's 18-call serial timebase must be related to the Rust
component's shared per-output-sample counter before level curves can be
compared. The fixture does not establish physical chip correctness.


### First Rust EG differential result

The fixture currently yields 1,744 active channel-0 operator samples. The
extractor uses the post-call `eg_kon & 0x04` pipeline bit as the effective
per-slot KEY-ON event, not the raw `kon[0]` channel bit. The absolute
`call_index / 18` sample counter matches the low eight bits of Nuked's
`eg_counter_state` at each selected update; the Rust candidate is evaluated
with the unwrapped counter.

For this one patch configuration, the best diagnostic candidate starts at
level 127 and defers the first ATTACK increment on the DAMP-to-ATTACK
transition. It produces 18 level mismatches (maximum absolute error 1) and
two stage mismatches, versus 254 level / 16 stage mismatches when the
transition consumes the same update. The 18 level mismatches are confined to
global-counter values 256–258, 512–514 and 768–770, two operator slots at each
counter. The two stage mismatches occur when level reaches zero: Nuked keeps
ATTACK for one more operator update before DECAY.

These results are documented in
`docs/audits/ym2413-eg-nuked-differential-2026-11-20.md`. They are
diagnostic only and do not authorize a production change. The counter-truncate
candidate was materially worse, so the low-eight-bit equality is a phase
observation, not evidence that the Rust global counter should wrap.
