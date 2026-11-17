# SG-1000 cartridge RAM profiles: DahJee/Jumbo and Othello

## Scope and evidence

This document records distinct RAM maps for SG-1000 cartridge hardware. CRC32 identifies known dumps; ROM size alone is not used to infer a mapper.

### DahJee/Jumbo Type A

The Genesis Plus GX source (core/cart_hw/sms_cart.c) explicitly lists the fourteen CRCs below as MAPPER_RAM_8K_EXT1 and maps the adapter's 8 KiB external RAM at **2000h–3FFFh**. This is a cartridge/adapter RAM window that overlays the normal ROM read map; it is not the SG-1000's internal RAM at C000h–FFFFh.

Reference implementation (used for independent behavioral study, not copied): https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/cart_hw/sms_cart.c

| ROM | CRC32 | Profile |
|---|---:|---|
| Adventure Island [DahJee] (Taiwan) | 16f240d3 | DahJee/Jumbo Type A |
| Bomberman Special [DahJee] (Taiwan) | ce5648c3 | DahJee/Jumbo Type A |
| King's Valley (Taiwan) | 223397a1 | DahJee/Jumbo Type A |
| Knightmare (Taiwan) | 281d2888 | DahJee/Jumbo Type A |
| Rally-X [DahJee] (Taiwan) | 306d5f78 | DahJee/Jumbo Type A |
| Road Fighter (Taiwan) | 29e047cc | DahJee/Jumbo Type A |
| Tank Battalion (Taiwan) | 5cbd1163 | DahJee/Jumbo Type A |
| The Goonies [DahJee] (Taiwan) | 40414556 | DahJee/Jumbo Type A |
| The Legend of Kage (Taiwan) | 2e7166d5 | DahJee/Jumbo Type A |
| TwinBee (Taiwan) | c550b4f0 | DahJee/Jumbo Type A |
| Yie Ar Kung-Fu II (Taiwan) | fc87463c | DahJee/Jumbo Type A |
| Pippols (Taiwan) | df7cbfa5 | DahJee/Jumbo Type A |
| Star Soldier [DahJee] (Taiwan) | e0816bb7 | DahJee/Jumbo Type A |
| Zanac [DahJee] (Taiwan) | 7c55057c | DahJee/Jumbo Type A |

The MAME software list identifies several of these titles with slot dahjee_typea and reports a 9,216-byte RAM data area for those conversions. The 9,216-byte metadata is not modeled as a single contiguous 9 KiB window: the Type-A mapper behavior above describes an 8 KiB external window, while the baseline 1 KiB internal RAM remains mapped separately.

MAME metadata: https://github.com/mamedev/mame/blob/master/hash/sg1000.xml


### SG-1000 II Type-B RAM expansion

These five CRC32 entries are catalogued by Genesis Plus GX as games requiring the SG-1000 II 8 KiB Type-B RAM expansion adapter. The expansion replaces the standard 1 KiB internal RAM mapping with 8 KiB at **C000h–FFFFh**, mirrored across the 16 KiB CPU window.

| ROM | CRC32 | Profile |
|---|---:|---|
| Bomberman Special (Taiwan) | 69fc1494 | Type-B internal RAM 8 KiB |
| Magical Kid Wiz (Taiwan) | ffc4ee3f | Type-B internal RAM 8 KiB |
| The Castle [MSX] (Taiwan) | 2e366ccf | Type-B internal RAM 8 KiB |
| Rally-X (Taiwan) | aaac12cf | Type-B internal RAM 8 KiB |
| Road Fighter (Taiwan) | d2edd329 | Type-B internal RAM 8 KiB |

The English-logo Bomberman Special variant is additionally identified by the documented SHA-256 `3eff3d6f1f74041f7b732455799d0978ab49724552ff2985f34b76478cd91721`.

Reference: https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/cart_hw/sms_cart.c

### Othello

Othello uses a different 2 KiB RAM mapper. The Genesis Plus GX game list labels the Japanese and Taiwan CRCs as MAPPER_RAM_2K; its implementation maps that RAM mirrored across 8000h–BFFFh.

| ROM | CRC32 | Profile |
|---|---:|---|
| Othello (Japan) | af4f14bc | Othello 2 KiB RAM |
| Othello (Taiwan) | 1d1a0ca3 | Othello 2 KiB RAM |

The Othello map remains a separate implementation profile; its successful manual run is recorded below, but has not been differentially compared with another emulator.

## Implementation

- The Rust runtime (`rust/runtime/sg1000-runtime/src/cartridge.rs`) selects known profiles by CRC32 and the documented Bomberman English-logo SHA-256; ROM size/title are never used as heuristics.
- DahJee/Jumbo Type A: 8 KiB cartridge/adapter RAM at 2000h–3FFFh; internal RAM remains 1 KiB at C000h–FFFFh.
- SG-1000 II Type-B: 8 KiB internal RAM at C000h–FFFFh, selected by five CRC32 entries or the documented Bomberman English-logo SHA-256.
- Desktop SG-1000 sessions use `Sg1000TemporalWorker::new_for_cartridge`; unknown ROMs retain the standard 1 KiB map.
- Othello: 2 KiB cartridge RAM mirrored across 8000h–BFFFh.
- The Castle (Japan) remains a separate 8 KiB cartridge-RAM profile at 8000h–BFFFh.
- Unknown ROMs retain the standard SG-1000 memory profile.

## Integration and milestone record

- Final Type-A RAM-map fix integrated into `main` by [PR #141](https://github.com/leonardo201800478/sega-emulator/pull/141), merge commit `f51c0fac8dfd850a761e65de48709a2e7002d11d`.
- Investigation issue [#136](https://github.com/leonardo201800478/sega-emulator/issues/136) closed as completed.
- Maintainer-reported manual functional result: PASS 8/8 — King's Valley (Taiwan), Knightmare (Taiwan), The Legend of Kage (Taiwan), Othello (Japan), Pippols (Taiwan), Star Soldier (Taiwan), Tank Battalion (Taiwan), and TwinBee (Taiwan).
- The maintainer reports other previously working SG-1000 games continue to work as before. This is a report from the maintainer's test environment, not a full catalog sweep.

## Validation status

- [x] Rust profile resolver tests cover all catalogued Type-A, Type-B, Othello and The Castle CRCs, the Bomberman English-logo SHA-256 exception, and conservative fallback for unknown ROMs.
- [ ] End-to-end desktop/in-game validation of every expanded-RAM profile: PENDING.
- [x] Native Rust memory test covers Type-A RAM writes, boundaries, ROM-window separation, and unchanged 1 KiB internal RAM.
- [x] Othello has a synthetic test for its 2 KiB mirrored window.
- [x] Full local Rust workspace tests: 114 passed, 0 failed.
- [x] Full local Python pytest suite: 192 passed, 0 failed.
- [x] `cargo fmt --all -- --check`: PASS.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- [x] Native Python extension rebuilt locally with Maturin for CPython 3.14 on Windows x64.
- [x] Maintainer manual functional report: all eight issue #136 games ran correctly in the local emulator.
- [ ] Differential comparison against Genesis Plus GX/MAME or Ares: NOT RUN.
- [ ] Python Ruff, compileall, and pip check acceptance commands: NOT RUN in this local validation session.

The manual functional result is based on the maintainer's local testing; the supplied session log records eight ROM-load events and confirms `rust_native_machine` as the effective runtime. It is not an independently reproduced differential test. The merge commit's GitHub Actions runs were not confirmed in the available query. No commercial ROMs are stored in the repository.
