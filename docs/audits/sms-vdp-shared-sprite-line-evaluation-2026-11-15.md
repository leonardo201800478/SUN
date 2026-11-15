# SMS VDP — shared SAT line evaluation for 315-5124 and 315-5246

**Date:** 2026-11-15
**Status:** implementation candidate; hardware capture remains pending

## 1. Evidence

SMS Power's Sprite documentation describes a common Mode 4 pipeline:

- On each scanline, the VDP parses the SAT to find sprites for the next line.
- Matching entries are placed in an internal 8-entry buffer.
- Evaluation stops after 64 SAT entries, eight selected entries, or the Y=208 terminator in 192-line mode.
- Overflow is independent of sprite X visibility and pattern transparency.

The documented SMS1/SMS2 difference is the horizontal zoom limitation: the 315-5124 does not horizontally magnify the final four selected sprites, while the 315-5246 magnifies all eight.

MAME's device model supports this separation: 315-5246 derives from the 315-5124 implementation and shares its line timing table, while the constructor changes the horizontal zoom count from 4 to 8. This is an independent emulator implementation, not a hardware oracle.

References:
- [SMS Power — Sprites](https://www.smspower.org/Development/Sprites)
- [SMS Power — SMS1/SMS2 VDPs](https://www.smspower.org/Development/SMS1SMS2VDPs)
- [MAME — 315_5124.cpp](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp)

## 2. Implementation gap

The project already had persistent Mode 4 and legacy sprite line buffers, but the lifecycle was restricted to `Sega3155124`. The 315-5246 renderer evaluated sprites through the fallback path at line rendering time.

That restriction was not supported by the documentation. It also meant the 315-5246 did not use the documented eight-entry next-line SAT buffer, despite having the same general sprite pipeline.

## 3. Changes in this candidate

- Share Mode 4 line-buffer priming and next-line SAT evaluation across 315-5124 and 315-5246.
- Keep the legacy/TMS sprite line buffer restricted to the existing SMS1 path until SMS2 legacy-mode behavior is characterized.
- Keep `evaluate_mode4_sprites` revision-aware so the 315-5124 zoom bug and 315-5246 full eight-sprite zoom remain distinct.
- Latch the selected Mode 4 line's overflow index at the existing T16 event for both revisions.
- Preserve the existing SMS1 HBlank partial-span path; SMS2 HBlank/raster-write timing remains uncharacterized.
- Preserve the SAT order in the buffered evaluation; the first opaque sprite retains priority.
- Keep the full evaluation and buffer state inside the VDP state so clone/save-state behavior remains deterministic.

## 4. Regression tests

Added SMS2-specific tests for:

- X/tile/pattern-table metadata latched for the next scanline;
- eight-entry buffer and overflow index at T16;
- all eight sprites receiving horizontal magnification on 315-5246;
- overlapping sprites preserving SAT priority and setting collision.

Existing SMS1 line-buffer, overflow, zoom and collision tests remain unchanged.

## 5. Remaining validation

- Run SmsVdpTest sprite suite against 315-5246.
- Compare The Smurfs (Brazil) sprite order with a reproducible frame/input sequence.
- Validate sprite collision/overflow phase and pattern fetch timing on real 315-5246 hardware or a documented capture.
- Confirm Cosmic Spacehead sprite behavior in 224-line mode.
- HBlank/raster writes remain a separate SMS2 timing gap; this PR does not claim to close it.
