# YM2413 public volume-capture audit

**Audit date:** 2026-11-22
**Upstream repository:** [andete/ym2413](https://github.com/andete/ym2413)
**Pinned revision:** `4893d1c09faa0450b5d3355f19b7682d72822501`
**Audited files:** `analyze/volume/volume0.raw` through `volume15.raw`
**Algorithm:** project tool `tools/ym2413-dac-capture-audit/audit_capture.py`, equivalent to the pinned `analyze/calibrate.cc` pipeline (filter width 30).

## Results

| Capture | Samples | ADC min–max | Detected peaks | Inserted midpoints | Reconstructed bins |
|---|---:|---:|---:|---:|---:|
| volume0.raw | 20,032 | 6,249–60,651 | 183 | 113 | 296 |
| volume1.raw | 20,032 | 13,634–52,440 | 274 | 55 | 329 |
| volume2.raw | 20,032 | 19,194–52,447 | 240 | 15 | 255 |
| volume3.raw | 20,032 | 23,140–42,588 | 182 | 0 | 182 |
| volume4.raw | 20,032 | 25,620–39,644 | 128 | 0 | 128 |
| volume5.raw | 20,032 | 27,107–37,676 | 92 | 0 | 92 |
| volume6.raw | 20,032 | 29,470–36,155 | 64 | 0 | 64 |
| volume7.raw | 20,032 | 30,430–36,153 | 46 | 0 | 46 |
| volume8.raw | 20,096 | 31,185–34,411 | 32 | 0 | 32 |
| volume9.raw | 20,032 | 31,298–33,977 | 24 | 0 | 24 |
| volume10.raw | 20,032 | 32,048–33,538 | 16 | 0 | 16 |
| volume11.raw | 20,032 | 32,264–33,319 | 12 | 0 | 12 |
| volume12.raw | 20,031 | 32,480–33,102 | 8 | 0 | 8 |
| volume13.raw | 20,032 | 32,589–32,992 | 6 | 0 | 6 |
| volume14.raw | 20,032 | 32,599–32,880 | 4 | 0 | 4 |
| volume15.raw | 20,096 | 32,697–32,882 | 4 | 0 | 4 |

## Interpretation

- None of the 16 volume captures reaches the default 60,000-sample calibration acquisition size.
- `volume0.raw` spans the broadest ADC range, but reconstruction produces 183 detected peaks plus 113 heuristic midpoint insertions: **296 bins**, not the expected 512.
- The remaining volume captures cover progressively narrower amplitude ranges and reconstruct fewer bins.
- Inserted midpoints are algorithmic estimates, not observed DAC levels. They cannot be counted as independent physical measurements.
- These files cannot replace `calibrate.raw`, nor certify the YM2413 DAC encoding, MO/RO transfer function, or sub-clock latch phase.

This audit validates only the public files and the pinned reconstruction algorithm. It does not establish the behavior of the user's physical board or the emulator's game playback.


## Processed volume outputs and decoder provenance

The same pinned tree also contains `volume0.out` through `volume15.out`. These files are already-quantized **ADC calibration-bin indices** (0..511), not raw DAC words. Their distributions provide an independent consistency check for the published volume attenuation table in `results/20150311.txt`.

For each file, the central 98% interval was calculated using the 1st and 99th percentiles. With the two central measured levels treated as the adjacent zero bins 255 and 256, the derived amplitude is:

`max(255 - percentile_1, percentile_99 - 256)`.

| Volume | Samples | Distinct bins | 1st percentile | 99th percentile | Derived amplitude |
|---:|---:|---:|---:|---:|---:|
| 0 | 20,030 | 437 | 0 | 511 | 255 |
| 1 | 20,032 | 352 | 74 | 436 | 181 |
| 2 | 20,032 | 260 | 128 | 383 | 127 |
| 3 | 20,032 | 182 | 165 | 346 | 90 |
| 4 | 20,032 | 129 | 192 | 319 | 63 |
| 5 | 20,032 | 93 | 210 | 301 | 45 |
| 6 | 20,032 | 64 | 224 | 287 | 31 |
| 7 | 20,032 | 47 | 233 | 278 | 22 |
| 8 | 20,096 | 32 | 240 | 271 | 15 |
| 9 | 20,032 | 26 | 244 | 267 | 11 |
| 10 | 20,032 | 16 | 248 | 263 | 7 |
| 11 | 20,032 | 12 | 250 | 261 | 5 |
| 12 | 20,032 | 9 | 252 | 259 | 3 |
| 13 | 20,032 | 6 | 253 | 258 | 2 |
| 14 | 20,030 | 4 | 254 | 257 | 1 |
| 15 | 20,096 | 4 | 254 | 257 | 1 |

The derived amplitudes reproduce all 16 values in the published 2015-03-11 amplitude table exactly. This is a statistical cross-check of the **measured analog output levels** and volume attenuation. It does not validate a raw DAC input word or the operator-to-DAC conversion. The percentile calculation uses each `.out` file independently; it does not assume that the three misaligned `.raw`/`.out` pairs correspond sample by sample.

### Why the decoder comparison initially appeared inconsistent

The `.raw` and `.out` artifacts were introduced together in upstream commit `f1384d719d25b7dd170ab58eddbe7dccca993504` (2015-03-11). At that revision, `analyze/decode.cc` simply mapped every raw ADC sample through `lower_bound(bins, value)`; it did not call the later `select_channel` routine.

The decoder subsequently evolved:
- `4c92887918764bbda1822bae45fa25438dd79f84` (2015-04-11) added 64-sample averaging.
- `f278516aa069b9cf4a6f55490e4643759cc0dd58` (2015-12-17) changed the averaging factor to 512.
- `c71980b94c7623b38abd4eb9b58a64f7cec77ec0` (2019-12-31) introduced `select_channel`.

Therefore, applying the current pinned decoder to the 2015 `volume*.raw` artifacts is not a valid reproduction of the historical `volume*.out` files. The apparent mismatch was caused by mixing decoder generations. However, a direct sample-index pairing audit exposes a separate provenance issue:

| Volume files | Raw samples | Output tokens | Same-ADC conflicts | Monotonicity violations |
|---|---:|---:|---:|---:|
| 1–11, 13, 15 (13 pairs) | equal | equal | 0 | 0 |
| 0 | 20,032 | 20,030 | 4,406 | 14,306 |
| 12 | 20,031 | 20,032 | 2,424 | 16,395 |
| 14 | 20,032 | 20,030 | 594 | 11,739 |

The three apparent failures are resolved by a constant **file-sequence alignment offset**, not by discarding the pairs. The alignment was selected by testing offsets from -8 through +8 and minimizing repeated-ADC-code mapping conflicts and monotonicity violations. Pairing is defined as `raw[i]` with `out[i + offset]`:

| Volume | Raw samples | Output tokens | Alignment offset | Paired samples |
|---:|---:|---:|---:|---:|
| 0 | 20,032 | 20,030 | -2 | 20,030 |
| 1–11 | 20,032 (volume 8: 20,096) | equal | 0 | equal |
| 12 | 20,031 | 20,032 | +1 | 20,031 |
| 13 | 20,032 | 20,032 | 0 | 20,032 |
| 14 | 20,032 | 20,030 | -2 | 20,030 |
| 15 | 20,096 | 20,096 | 0 | 20,096 |

After these sequence offsets, all 16 pairs are monotonic and free of repeated-ADC-code mapping conflicts. These offsets describe alignment between two stored data files only; they are **not** YM2413 clock, slot, or subclock offsets.

Combining the 320,635 aligned samples yields observations for **511 of 512** calibrated bin indices, spanning 0..511. Only bin 57 is unobserved. For 509 adjacent-bin boundaries, the observed samples constrain the original `lower_bound` threshold to an integer interval; no interval is contradictory. Of those 509 intervals, 147 are exact single-code boundaries. The interval-width distribution is p50=32 ADC codes, p90=84, p99=92, maximum=157. The two boundaries adjacent to missing bin 57 cannot be constrained from these files.

The per-bin observations and all 509 boundary intervals are preserved in [the machine-readable reconstruction](ym2413-volume-bin-observations-2026-11-22.json). This is a near-complete **constraint reconstruction** of the historical ADC-bin table, not the original `calibrate.bins` file: most thresholds remain intervals rather than exact values, and the original `calibrate.raw`/peak histogram is still absent. The data remain analog ADC-bin indices, not raw YM2413 DAC words.

The separate `vol*.out` files also present in the directory have no corresponding `vol*.raw` files in the pinned tree and are not used in this audit.

## Evidence boundary

- `volume*.out` confirms calibrated analog-bin indices and volume attenuation behavior.
- `volume*.out` does not reveal the binary word at the internal DAC input.
- `volume*.raw` remains insufficient to reconstruct the missing 60,000-sample `calibrate.raw` capture and its original `.peaks/.bins` artifacts.
- The 16-volume data does not resolve MO/RO independently, DAC sub-clock latch phase, or the Japanese SMS analog load/transfer function.

## Reproduction

From the repository root, run the audit tool against each downloaded file:

```sh
python3 tools/ym2413-dac-capture-audit/audit_capture.py volume0.raw --json volume0-audit.json
```

The files are hosted in the pinned upstream revision under `analyze/volume/`.


## Aggregate-histogram reconstruction limitation

As a diagnostic, all 320,639 raw samples from the 16 volume captures were concatenated and passed through the same triangular-filter / peak-detection algorithm used by `audit_capture.py`. This produced 421 detected peaks plus 40 heuristic midpoint insertions: **461 bins**, not 512. None of the 509 paired-data boundary intervals contained the corresponding threshold from this aggregate-histogram reconstruction.

This does not invalidate the sample-paired ADC→bin observations: those mappings are globally monotonic and conflict-free after the file-sequence offsets. It demonstrates that the volume captures have a different, non-uniform sample distribution from the dedicated calibration sweep and cannot be concatenated as a substitute for the missing `calibrate.raw` peak acquisition. The paired labels still constrain 509 boundaries, but the exact original peak centers and two boundaries around unobserved bin 57 remain unavailable.
