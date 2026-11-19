# YM2413 operator-to-DAC boundary: competing output encodings

**Date:** 2026-11-19  
**Related gate:** GitHub Issue #1031  
**Scope:** evidence analysis only; no conversion is approved by this document.

## 1. Problem statement

The current Rust operator candidate returns a signed one's-complement intermediate after the EXP lookup and arithmetic shift. The reproducible edge case in Issue #1031 reaches -256. The measured output-level abstraction used by the project is -255..=+255, with two distinct zero encodings reported in the reverse-engineering notes.

These are different domains:

1. operator arithmetic intermediate (i16, current candidate range includes -256);
2. reconstructed signed YM2413 output levels from the measurement pipeline;
3. physical 9-bit DAC input word;
4. analog MO/RO pin voltage after the DAC and output circuitry.

No direct physical correlation between domain 1 and domain 3 has been established.

## 2. Two published model expressions

### 2015-12-03 model

The model shown in Andete's operator-pair investigation emits:

    lookupExp(carrier) + 256

The note explicitly warns that the measured waveforms appear upside down and suggests the test setup may invert signal polarity. The expression is part of a model that reproduces the sampled output waveform; it is not a direct observation of the internal DAC input latch.

### 2015-12-12 feedback model

The later feedback model emits:

    255 - c

where c is the signed result of the carrier EXP path after the model's shift. This model reproduces the captured feedback waveforms, subject to the stated start-condition and measurement-noise limitations. It likewise does not expose the chip's internal DAC input word.

## 3. Exact arithmetic comparison

For a candidate operator intermediate x in the current representable range -256..=255, the two expressions produce:

| Intermediate x | x + 256 | 255 - x |
|---:|---:|---:|
| -256 | 0 | 511 |
| -255 | 1 | 510 |
| -1 (one's-complement -0) | 255 | 256 |
| 0 (+0) | 256 | 255 |
| 1 | 257 | 254 |
| 255 | 511 | 0 |

The two candidate expressions are exact 9-bit complements:

    (x + 256) + (255 - x) = 511

They therefore reverse the output polarity and exchange the two zero codes. This is not a one-bit rounding discrepancy. It is a complete inversion of the 9-bit code ordering.

## 4. What the measurements establish

- Andete's 2015-03-11 volume investigation reports 511 observed levels represented as signed values -255..=+255, including distinct -0 and +0 levels. It also notes that the ADC measurement has non-uniform spacing/noise and that the two center levels ideally coincide in the reconstructed analog level domain.
- The 2015-12-03 operator-pair investigation reports a test setup whose captured sine waveform appears inverted, and its matching model uses +256.
- The 2015-12-12 feedback investigation uses 255-c to match the later captured waveforms and reports that initial feedback-state conditions can strongly affect the match.
- The YM2413 application manual specifies a 9-bit DAC but does not document either integer mapping from the Rust intermediate to its input word.

These observations explain why two model expressions can appear in the same reverse-engineering record without proving which expression is the physical DAC encoding. Neither equation is promoted to hardware truth here.

## 5. Implementation decision

Do not implement either conversion as the default. Do not clamp -256 to -255, saturate, wrap, bias, truncate, or reinterpret the existing Ym2413DacCode measurement-domain type.

Keep the following boundaries separate:

- operator candidate output remains a signed one's-complement intermediate;
- Ym2413DacCode remains the signed reconstructed measurement-level domain;
- a future Ym2413RawDacWord must represent the physical nine-bit input only after a source vector or physical capture correlates an isolated operator configuration with the DAC word.

## 6. Evidence needed to close Issue #1031

1. A capture that controls YM2413 input clocks and register writes, isolates one carrier path, and reads or reconstructs the digital DAC input independently of the board analog response.
2. A documented correlation for both positive and negative peaks and both zero encodings.
3. A specific explanation for the current candidate's -256 result and why the physical output has 511 rather than 512 observed levels.
4. Reproducible vectors that distinguish x+256 from 255-x, including x=-256,-1,0,255.
5. Only then, a typed conversion API and tests for the proven mapping.

## References

- Yamaha YM2413 Application Manual: https://www.waveguide.se/?article=46&file=ym2413am.pdf
- Andete, 2015-03-11: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-11
- Andete, 2015-12-03: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-03
- Andete, 2015-12-12: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-12
- Project gate: https://github.com/leonardo201800478/sega-emulator/issues/1031
