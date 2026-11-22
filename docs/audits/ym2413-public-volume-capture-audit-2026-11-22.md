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

## Reproduction

From the repository root, run the audit tool against each downloaded file:

```sh
python3 tools/ym2413-dac-capture-audit/audit_capture.py volume0.raw --json volume0-audit.json
```

The files are hosted in the pinned upstream revision under `analyze/volume/`.
