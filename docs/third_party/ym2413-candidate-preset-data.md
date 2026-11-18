# YM2413 candidate preset patch data — provenance

## Included material

`native/ym2413/src/instrument.rs` contains only the 15 melody-preset byte patterns (instrument codes 1..15) from the first YM2413 bank in `default_inst[0]` in `emu2413.c`.

- Project: [digital-sound-antiques/emu2413](https://github.com/digital-sound-antiques/emu2413)
- Source revision: `cef17c32f67a88df381e08168e5820984de7e936`
- Source file: `emu2413.c`, first `default_inst` bank
- License: MIT
- License text retained at [third_party/licenses/emu2413-MIT-LICENSE.txt](../../third_party/licenses/emu2413-MIT-LICENSE.txt)
- Original license copyright notice: Mitsutaka Okazaki, 2001–2019. The cited source revision also contains its own 2026 copyright header.

No C implementation, synthesis code, or other emulator logic was copied. The numeric patch bytes were adapted into a Rust-only data table, and the MIT notice is retained with the repository.

## Scope exclusions

- The VRC7 and YMF281B banks in emu2413 are not included.
- The three rhythm patch patterns in the YM2413 bank are not included because the source labels them as VRC7-derived.
- Instrument code 0 is not part of this ROM table; it resolves to the live user patch in registers $00–$07.

## Accuracy status

These values are a **candidate preset bank**, not a verified dump of the physical YM2413 ROM. In the 2018-05-13 reverse-engineering report, andete states that reconstructed instrument settings were often close but not always identical to the built-in instruments.

Therefore:

1. The values are usable for initial audible FM tests and register-decoder validation.
2. They must not be presented as physically exact Yamaha ROM contents.
3. Differential PCM or register-level comparisons against a physical YM2413 remain necessary.
4. If a verified hardware dump becomes available, it should replace this candidate bank with new provenance and tests.

Reference: [andete — YM2413 Reverse Engineering Notes, 2018-05-13](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13).
