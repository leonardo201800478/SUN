# YM2413 external Nuked-OPLL oracle trace

This tool records the public internal state exposed by the pinned Nuked-OPLL
`opll_t` structure at every OPLL_Clock call. It is a **separate external
reference tool**; no Nuked implementation source is copied into this repository.

## Reference pin

- Repository: https://github.com/nukeykt/Nuked-OPLL
- Commit: `1269cf5a783b65583b50fa2464d08be75830aaa0`
- API used: `opll.h` (`opll_t`, `OPLL_Reset`, `OPLL_Clock`, `OPLL_Write`)
- Chip type default: `opll_type_ym2413 = 0`

The user must independently clone/check out that exact revision. The harness
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
1,1,64
2,0,32
3,1,21
```

Port 0 queues an address-port write; port 1 queues a data-port write, using
Nuked's own `OPLL_Write` API. Rows are submitted immediately before the
corresponding `OPLL_Clock` call. Address/data timing and internal write
acceptance remain the reference implementation's behavior; this harness does
not bypass its IO state machine. Use decimal values in CSV (the example's
hexadecimal-looking values are illustrative; convert them to decimal).

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

The trace includes PG phase/increment, the active channel/operator patch parameters, EG timer and per-operator state,
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
