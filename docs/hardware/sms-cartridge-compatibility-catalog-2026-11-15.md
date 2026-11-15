# SMS cartridge compatibility metadata — 2026-11-15

## Contract and versioning

Compatibility is a separate metadata dimension. It must not be inferred from ROM filename, release region, mapper, SRAM capacity, or the user's selected machine.

- Schema version: `SMS_CARTRIDGE_COMPATIBILITY_SCHEMA_VERSION = 1`.
- Evidence data revision: `SMS_CARTRIDGE_COMPATIBILITY_DATA_VERSION = "2026-11-15"`.
- `Unknown` is the default whenever no independent evidence-backed minimum VDP requirement exists.
- `Sms1Compatible` means the title is supported by SMS1 315-5124 and later SMS VDPs.
- `Sms2Required` means the title requires SMS2 315-5246 capabilities.
- The selected SMS model remains explicit. Compatibility metadata never changes or auto-selects the VDP revision.

Compatibility metadata is stored separately from `CartridgeHardwareProfile`, so adding a VDP requirement does not change the SRAM profile serialization/fingerprint. Mapper and SRAM remain independent profile fields.

## Evidence-backed declarations

### Cosmic Spacehead (Europe) — SMS2 required

Exact No-Intro DAT identity from the project version `20261013-064924`:

| Field | Value |
|---|---|
| Size | 262144 bytes |
| CRC32 | `29822980` |
| SHA-1 | `f46f716dd34a1a5013a2d8a59769f6ef7536a567` |
| Region metadata | EUR |
| Mapper | Codemasters |
| Minimum VDP | SMS2 / 315-5246 |

The profile is keyed by SHA-1. CRC32 and size are retained as DAT consistency checks, not as a CRC-specific compatibility exception.

Independent evidence:

1. [Genesis Plus GX SMS cartridge implementation](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/cart_hw/sms_cart.c) classifies CRC32 `0x29822980` as `MAPPER_CODIES`, `SYSTEM_SMS2`, region Europe.
2. [SMS Power forum — Cosmic Spacehead](https://www.smspower.org/forums/1484-CosmicSpacehead): Charles MacDonald identifies the title's 224-line mode and reports SMS1/Game Gear incompatibility.
3. [SMS Power forum — SMS2 VDP 224/240-line modes](https://www.smspower.org/forums/8170-SMS2VDP224240LineModes): Charles MacDonald lists Cosmic Spacehead among titles using the 224-line mode and identifies the SMS2 VDP capability.

Result: `SmsVdpRequirement::Sms2Required`.

The catalog entry is `EvidenceOnly` for executable mapper/SRAM configuration because this record does not declare a verified SRAM mapping. The runtime's existing Codemasters mapper detection remains responsible for actual bank behavior. No SRAM capacity is inferred.

The existing `native/sms_core/src/memory.rs` implementation already detects the Codemasters header and maps the three 16 KiB slots through writes in the `$0000`, `$4000`, and `$8000` windows. Existing unit tests cover header selection, slot switching, BIOS power-on gating, invalid-header fallback, and the absence of Sega-mapper SRAM on Codemasters cartridges. This metadata change does not rewrite or alter that mapper implementation.

### The Smurfs (Brazil) — unknown

Identity supplied for investigation:

- CRC32: `E14C8A1C`
- SHA-1: `15b882ad3263cdc1e78c732d96f9bb18e25f4743`

Neither the SHA-1 nor CRC32 is present in the project's No-Intro DAT `20261013-064924`, and no independent source currently establishes a minimum SMS VDP requirement for this exact Brazilian dump.

Result: no compatibility profile is declared; the loader reports `Unknown`, continues with the explicitly selected hardware, and does not infer SMS2 from the Brazilian region or filename.

The ROM is not present in the repository. No execution or compatibility validation of this dump is claimed.

## Loader enforcement

The cross-platform `SmsTemporalWorker` performs compatibility preflight:

- before creating the worker/machine for a new ROM;
- before queuing `load_rom` on an existing worker.

A known SMS2-only cartridge selected with SMS1 returns an explicit error before machine construction or command submission. The desktop replacement path creates the new worker before taking the active worker out of its slot; therefore this preflight error leaves the active session untouched. Unknown metadata is logged as `SMS_COMPATIBILITY status=unknown` and does not change the selected VDP.

## Validation classification

| Validation | State |
|---|---|
| Exact Cosmic Spacehead identity against project DAT | Evidence verified |
| Cosmic Spacehead SMS2 requirement | Independently documented; no ROM execution claimed |
| The Smurfs Brazil exact identity in project DAT | Not found |
| The Smurfs Brazil minimum VDP requirement | Unknown |
| Compatibility model unit tests | Added; Linux CI pending |
| Runtime loader preflight tests | Added; Linux CI pending |
| Windows tests | Not run by design |
| Physical SMS1/SMS2 hardware | Not tested |
