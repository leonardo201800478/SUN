# SMS2 VDP — raster events and interrupt timing

**Date:** 2026-11-15
**ASIC:** Sega 315-5246
**Status:** implementation candidate; physical hardware validation remains open

## 1. Defect found in the previous SMS2 path

The 315-5246 path did not process horizontal raster events while advancing a scanline. `SmsVdp::advance()` advanced directly to `LINE_TSTATES` for SMS2, while only the 315-5124 path processed:

- VINT request;
- HINT line-counter decrement/reload;
- VCounter latch;
- HBlank partial-render commit;
- VBlank frame publication and status flag.

The SMS2 path instead asserted line IRQ at `complete_scanline()`, entered VBlank at the scanline boundary, and derived `interrupt_pending()` directly from the VBlank status bit. `vint_pending` was never asserted for 315-5246. This made SMS2 line IRQ scanline-late, prevented the VINT-before-status phase, and skipped the common HBlank/VCounter events.

This was not evidence of a real hardware difference; it was an implementation asymmetry.

## 2. Evidence supporting a shared event schedule

- SMS Power's SMS VDP documentation describes the same HINT/VINT register interface and status behavior for the SMS family.
- The SMS VDP Test source documents PAL SMS1 and PAL SMS2 test targets. Its raster tests include HINT/VINT/status and counter timing.
- MAME models the 315-5246 as a 315-5124-derived device and supplies the same `line_315_5124` horizontal timing table to the 315-5246 constructor. This is an independent emulator implementation, not a hardware oracle.
- No primary source consulted identifies a different HINT/VINT/VCounter/HBlank event schedule for the 315-5246.

References:
- [SMS Power — SMS1/SMS2 VDPs](https://www.smspower.org/Development/SMS1SMS2VDPs)
- [SMS Power — Official SMS docs](https://www.smspower.org/Development/SMSOfficialDocs)
- [FluBBa — SmsVdpTest](https://github.com/FluBBaOfWard/SmsVdpTest)
- [MAME — 315_5124.cpp](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp)

## 3. Implementation

The common CPU-visible event schedule now runs for both 315-5124 and 315-5246:

| Event | Current project boundary | Effect |
|---|---:|---|
| VINT request | T13 | Set `vint_pending` |
| HINT | T14 | Decrement/reload R10 and latch line IRQ |
| VCounter | T15 | Latch current scanline |
| VBlank frame publication/status | T16 | Publish completed visible frame at `height`; set VBlank status at `height + 1` |
| HBlank | T213 | Commit the remaining visible raster span |

The SMS1 sprite-overflow latch remains revision-specific at T16 and continues to consume the persistent SMS1 sprite line buffer. This change does not invent an SMS2 sprite buffer or claim that sprite fetch timing is complete.

The VINT interrupt line now uses the dedicated `vint_pending` latch for both revisions. The VBlank status flag remains a separate event and is cleared by the status-port read.

## 4. Tests added

- SMS2 HINT asserts at the same T14 boundary as the current SMS1 model.
- SMS2 VINT asserts at T13 before the VBlank status flag at T16.
- Frame publication occurs at T16 of the selected active-height boundary.
- The VINT/status sequence is tested at 192, 224 and 240 active lines.
- Existing SMS1 tests remain unchanged and run as regressions.

These tests prove consistency of the model and prevent the previous SMS2 no-VINT/late-HINT behavior from returning. They do not establish dot-level electrical accuracy.

## 5. Remaining SMS2 timing work

- Validate HINT/VINT/VCounter/HBlank against the SMS VDP Test on PAL SMS2 hardware or a documented capture.
- Characterize the 315-5246 sprite evaluation and overflow/collision phases independently; SMS1's persistent line buffer is not assumed to represent SMS2.
- Validate raster writes and framebuffer publication for 224/240 lines through WGPU/Slint.
- Run Cosmic Spacehead (CRC32 29822980) from menu through gameplay after explicit SMS2 selection is integrated.
