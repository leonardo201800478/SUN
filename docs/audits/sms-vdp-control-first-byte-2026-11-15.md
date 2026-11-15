# SMS VDP control-port first-byte semantics — 2026-11-15

**Baseline:** `e122679913f8c9f9fa133f9ed92d6aeca4eb2891`
**Scope:** shared SMS VDP control/data port, both 315-5124 and 315-5246 profiles
**Implementation branch:** `fix/vdp-first-control-byte-address-only`
**Validation state:** implementation and regression tests added; Linux VDP CI pending
**Hardware validation:** NOT RUN

## Finding

The first write to control port `$BF` updates only the low eight bits of the VDP address register. It does not execute the operation selected by the second byte, including VRAM read-ahead or the associated address increment.

Two independent technical references describe this contract:

- SMS Power, [Control Port](https://www.smspower.org/Development/ControlPort): the first byte updates the lower eight address bits; actions triggered by the second byte are not performed when only the first byte is written.
- FluBBaOfWard, [SmsVdpTest README](https://github.com/FluBBaOfWard/SmsVdpTest): test group **“1byte wr sets rd/wr adr”** states that the first control write must set the low address byte. The README also distinguishes read prefetch after selecting code 0 from operations performed by the first byte alone.

## Baseline defect

In `native/video_core/src/sms/vdp.rs`, `SmsVdp::write_control` previously updated the low address byte and, when the current mode was `VramRead`, immediately loaded `read_buffer` from VRAM and incremented the address.

That made a one-byte control write observably behave like a partial read command. For example, a current address of `$3E59` followed by a first control byte `$10` became `$3E11`, rather than the required `$3E10`. It also replaced the existing read buffer before the second command byte selected an operation.

## Correction

The first-byte path now performs only:

- preserve address bits 13–8;
- replace address bits 7–0;
- retain the pending first-byte latch;
- leave the read buffer, access mode, and address increment untouched.

Read-ahead and address increment remain in the complete two-byte VRAM-read command path. The existing data-port read/write behavior and latch cancellation rules are unchanged.

## Regression coverage added/updated

- SMS2/default VDP profile: first-byte-only read-address update, preservation of the old read buffer, and read-ahead only on subsequent data-port accesses.
- SMS1/315-5124 profile: the same low-address-only contract and read-buffer preservation.
- Public SMS1 port contract: a first byte followed by a status read leaves the updated low address intact and does not increment it.
- Existing one-byte write-address and CRAM-destination tests remain in place.

## Validation classification

| Validation | Result |
|---|---|
| Static source inspection against SMS Power control-port contract | PASS |
| Regression tests authored/updated | UPDATED; first CI run found one stale reset-test assumption, corrected in the follow-up commit |
| Linux `sega-video` + `sega-sms-core` focused CI | FIRST RUN FAILED: 112 passed, 1 failed (`reset_restores_the_default_vram_read_address_mode` expected the old one-byte prefetch); test corrected to issue the complete two-byte read command; rerun pending |
| Rustfmt / Clippy in Linux CI | PASS on first run |
| SMS1/SMS2 physical hardware | NOT RUN |
| Original diagnostic ROM execution on this runtime | NOT RUN |

This correction does not claim dot-level timing, VRAM contention, or physical ASIC validation. It addresses only the documented register-side effect of a one-byte control-port write.
