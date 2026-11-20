# SMS Plus GX / CrabEmu SN76489 trace adapter

This directory contains a small, project-authored C adapter for the CrabEmu PSG engine shipped as an option in SMS Plus GX. It is built against an external checkout pinned to a specific commit. The upstream C source and header are not copied into Sega Emulator and are never linked into the Rust backend.

## Reference pin

- Repository: https://github.com/libretro/smsplus-gx
- Commit: `3844b46caa926b6494987b97da63092818c4ddef`
- Engine: `source/sound/crabemu_sn76489/sn76489.c`
- Header: `source/sound/crabemu_sn76489/sn76489.h`
- License: GPL-2.0 for the CrabEmu engine. The reference is built only in an ephemeral CI runner.

SMS Plus GX also contains separate Maxim and MAME-derived engines. This adapter intentionally selects CrabEmu; it does not treat those engines as independent confirmations.

## Build

Clone the exact reference commit outside this repository:

```sh
git clone https://github.com/libretro/smsplus-gx.git /tmp/smsplus-gx
git -C /tmp/smsplus-gx checkout 3844b46caa926b6494987b97da63092818c4ddef
```

Compile the adapter with a C11 compiler:

```sh
cc -std=c11 -O2 -Wall -Wextra \
  -I/tmp/smsplus-gx/source/sound/crabemu_sn76489 \
  tools/psg-crabemu-oracle/trace.c \
  /tmp/smsplus-gx/source/sound/crabemu_sn76489/sn76489.c \
  -o /tmp/psg-crabemu-trace
```

The adapter initializes the SMS profile with 16 LFSR bits and taps 0/3 (`0x0009`). It sets the reference sample rate to input clock /16, so each adapter call advances one PSG internal clock. The reference's sample is computed before its counters and LFSR are advanced; both pre- and post-update state are recorded.

## Event format

CSV rows contain an internal PSG tick index and one PSG data byte:

```csv
# tick,byte
0,0xE4
0,0xF0
540,0xE0
560,0xE4
```

Rows must be ordered by nondecreasing tick. Multiple writes at one tick are applied in file order before that tick's sample is generated. The adapter does not emulate Z80 bus cycles; it is a chip-level diagnostic input schedule.

Run:

```sh
/tmp/psg-crabemu-trace --calls 600 --events events.csv > reference.jsonl
```

## Rust core trace

The companion Rust example emits the Sega Emulator's current SMS PSG state using the same event schedule and one Scheduler-domain PSG tick per row:

```sh
cargo run --locked -p sega-sn76489 --example psg_state_trace -- \
  --calls 600 --events events.csv > core.jsonl
```

The sample rate of the Rust core affects its PCM queue/resampler state but not the PSG hardware state. The example defaults to 48 kHz.

## Compare

```sh
python3 tools/psg-crabemu-oracle/compare.py reference.jsonl core.jsonl > comparison.json
```

The comparator uses identical tick indices and never aligns traces by correlation. It reports:

- first reference and core LFSR-shift tick;
- raw LFSR state mismatch count and first mismatch;
- reference sample bit versus the Rust core's LFSR bit before the tick;
- reference sample bit versus the Rust core's latched noise output;
- SHA-256 digests of both trace streams.

Differences are diagnostic results, not automatically test failures. In particular, the output-bit boundary and initial divider phase are open hardware questions. The workflow asserts schema, determinism and the pinned reference's known trace schedule, then uploads both traces and the comparison summary.

## Interpretation constraints

- The reference sample is computed from the current LFSR bit before the reference advances the divider and LFSR.
- The Rust trace exposes the core's current LFSR and its separately latched `noise_output`.
- A mismatch between the reference sample bit and `noise_output` does not by itself prove which model is physically correct; it identifies the exact tick and state where the conventions diverge.
- This adapter does not validate analog gain, filtering, PSG-to-FM routing, or host resampling.
- No reference code is copied, translated or vendored into the production crate.
