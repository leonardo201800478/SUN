# YM2413 operator-to-measured-output candidate comparator

## Purpose

`compare_output_code_candidate.py` is an offline diagnostic that exports the
serial operator's raw output, its channel-output latch, the candidate 9-bit
measured-output code, and the signed measured-domain interpretation. It
consumes the pinned Nuked-OPLL JSONL trace; it does not alter production audio
or feed oracle values into the emulator.

The candidate mapping is:

```text
candidate_output_code = 255 - channel_output
```

The accepted channel latch domain is `-256..=255`, mapping bijectively to
`0..=511`. Positive zero (`0`) and one's-complement negative zero (`-1`)
remain distinct (candidate codes 255 and 256). The negative full-scale
endpoint `-256` maps to signed measured amplitude `-255`.

## Run

Without the optional published prediction artifact:

```sh
python3 tools/ym2413-nuked-oracle/compare_output_code_candidate.py \
  --nuked-trace nuked-operator.jsonl \
  --output output-code-candidate.json
```

With the published 1024-by-16 `predicted-sine.txt` artifact:

```sh
python3 tools/ym2413-nuked-oracle/compare_output_code_candidate.py \
  --nuked-trace nuked-operator.jsonl \
  --predicted-sine predicted-sine.txt \
  --volume-column 0 \
  --output output-code-candidate.json
```

Volume columns are zero-based (0 through 15). The selected published value
is looked up by the trace's `op_phase_index`; no volume is inferred from
unrelated trace fields. The report preserves `operator_output`,
`channel_output`, the corresponding Nuked `ch_out`, candidate code,
signed amplitude, negative-zero flag, and optional prediction residual for
each serial operator event.

## Validation and evidence boundary

The helper tests exhaustively verify all 512 channel-latch values, both zero
encodings, both full-scale endpoints, invalid-domain rejection, trace schema
validation, and the published artifact's 1024-by-16 shape.

A report status of PASS means only that the trace's `operator_output >> 3`
matches its `channel_output` latch for every event. Prediction exact-match
statistics are reported separately and do not gate this invariant.

The prediction input is the published data artifact, not a regenerated
executable oracle: the pinned upstream `analyze/exp.cc` and committed
`predicted-sine.txt` have a documented signed-value provenance discrepancy.
Consequently, neither this report nor a PASS establishes the physical
YM2413 DAC input word, analog transfer function, voltage calibration, or
hardware timing. The candidate conversion remains separate from the
production `Ym2413DacCode` API and output pipeline.
