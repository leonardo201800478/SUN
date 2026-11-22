# YM2413 DAC capture audit

This dependency-free tool reproduces the ADC histogram, triangular smoothing,
parabolic peak detection, missing-peak insertion and integer-bin boundary
construction in the pinned Andete reverse-engineering source:

- Repository: andete/ym2413
- Revision: 4893d1c09faa0450b5d3355f19b7682d72822501
- Calibration: analyze/calibrate.cc
- Capture transport: analyze/common.cc

## Input format

The input is the whitespace-separated .raw stream emitted by the original
analysis program. common.cc::handleRead(uint16_t) stores
uint16_t(data + 0x8000), so values are unsigned 16-bit offset-binary ADC
codes. They are not signed integers and must be in 0..65535.

## Run

    python3 tools/ym2413-dac-capture-audit/audit_capture.py capture.raw --json capture-audit.json

The report distinguishes peaks detected in the histogram from midpoint peaks
inserted by the original insertMissing() heuristic (one midpoint whenever a
gap is strictly greater than 160 ADC codes). --require-expected-bins makes a
non-512 result return exit status 2; without that flag, incomplete or unusual
captures are still reported for investigation.

Run the deterministic tests with:

    python3 -m unittest discover -s tools/ym2413-dac-capture-audit -v

## Interpretation boundary

The generated bin boundaries classify measured ADC voltages. A decoded value
0..511 is a calibrated analog-bin index, not the YM2413's internal 9-bit
DAC word. The tool does not infer the operator i16 to DAC conversion, the
sub-clock latch phase, or the MO/RO analog transfer function.

The committed volume*.raw captures are not substitutes for the missing
60,000-sample calibrate.raw and its .peaks/.bins outputs. A 512-bin
reconstruction from a new capture is only a calibration reproduction result;
it does not by itself prove direct observation of all 512 levels because the
upstream algorithm inserts midpoint peaks. Hardware capture validation must
separately report ADC overflow, expected versus delivered sample count, DMA
loss/reordering and known trigger phase.

Synthetic unit fixtures validate the ported algorithm and error handling only.
They do not validate the YM2413 hardware, DAC encoding, analog response, PCM
conversion, or game playback.
