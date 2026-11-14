# SMS in-game ROM smoke campaign

## Purpose

Exercise legally obtained SMS ROMs through the public `SmsTemporalWorker` runtime API, not through a synthetic VDP fixture. This campaign is intended to expose failures during ROM loading, CPU execution, frame scheduling, VDP frame publication and sustained execution.

The test is opt-in because commercial ROMs are not stored in this repository.

## Run

PowerShell, from the repository root:

```powershell
$env:SEGA_SMS_TEST_ROM_DIR = "D:\ROMs\SMS"
$env:SEGA_SMS_TEST_REGION = "usa" # usa | brazil | europe | japan
cargo test --release -p sega-sms-runtime --test in_game_rom_smoke -- --ignored --nocapture
```

The directory is scanned non-recursively for `.sms` and `.bin` files. Each ROM is loaded through `SmsTemporalWorker::new`, connected to a real `FrameMailbox`, and stepped for 120 frames. The test checks that every step executes instructions, a 256×192 RGB888 frame is published, the runtime reports no fatal error, and at least one frame is not entirely black.

The test emits one `SMS_IN_GAME_SMOKE` line per ROM with its path, selected region, completed frame count, instruction count, number of non-black frames and first/last frame FNV-1a fingerprints. FNV-1a is only a compact regression fingerprint; it is not a cryptographic ROM identity or integrity mechanism.

## Interpretation and limitations

- A passing result demonstrates that the ROM was accepted by the SMS worker, execution advanced for 120 frames, and the runtime published non-black video frames.
- A failure identifies the ROM and the frame at which execution or publication failed.
- A passing smoke test does **not** prove that the game is playable, that its title screen or gameplay is correct, or that audio/input behavior is correct. Those require visual/manual review and game-specific expected behavior.
- A non-black image is not sufficient evidence of correct rendering. Compare captured frames against a trusted reference emulator at the same region, frame phase and input sequence.
- The harness deliberately does not infer a ROM's region from its filename. Set the correct region explicitly for each run.
- ROM files remain outside the repository.

## VDP integration evidence

The runtime path to audit alongside this campaign is:

`SmsTemporalWorker → SmsTemporalMachine → SmsTemporalBus → SmsVdp`.

The worker's region profile selects the SMS1 `Sega3155124` ASIC for USA, Brazil, Europe and Japan. The independent SG-1000 TMS9918 implementation must remain available for SG-1000; it must not be substituted for the SMS VDP in this path.

The existing `native/video_core/tests/sms1_vdp_contract.rs` and temporal-bus tests cover VDP port behavior, raster boundaries and state restoration. They are complementary to, not replacements for, this in-game campaign.

## Next validation increments

1. Run this harness on Windows x64 with a curated ROM set including titles previously reported not to start.
2. Save representative frame captures and compare them with an independently validated emulator.
3. Add deterministic input scripts and game-specific boot milestones for ROMs whose expected startup behavior is known.
4. Correlate failures with CPU PC/opcode, mapper state, VDP status/interrupts and Scheduler timestamps before changing hardware behavior.
