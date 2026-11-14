# Sega Desktop — R2 SMS / Slint / WGPU 30 / Vulkan

The Slint host presents real SMS VDP frames from the Rust runtime. The desktop remains a presentation and integration layer: it does not own hardware state or advance emulated time.

## Menu migration

- Sistema: SMS region selection (USA, Brasil, Europa, Japão); other Sega systems remain visible but disabled until their runtimes are integrated.
- Arquivo: load raw `.sms` / `.bin` ROMs or a `.zip` containing exactly one supported ROM entry; reopen recent ROMs and exit. O desligamento POWER OFF mantém a ROM carregada; o comando de descarregar ROM não fica exposto neste menu.
- Vídeo: 4:3, 1:1 and 16:9 presentation modes preserve the selected ratio while the window is resized; unused viewport space is letterboxed. The source frame remains 256×192. The optional sprite-limit compatibility mode is persisted separately from hardware state.
- Áudio: the Slint host exposes the CPAL 0.18.2 output status and master-volume controls. The default Windows output device is used by the current host path; endpoint enumeration/selection UI and full physical-device lifecycle acceptance remain separate validation work.
- Controles: teclado SMS ativo via eventos nativos do Slint (setas, Z/X, P e F5–F9); F11 alterna fullscreen. Gamepad SDL3 e remapeamento permanecem em R4.
- Opções: status-footer visibility. FPS size/position remain pending. Fullscreen is available from the UI and F11.
- DEBUG: request an immutable Z80/runtime snapshot and enable/disable bounded scheduler/bus trace capture. Full debugger viewport remains a follow-up.

## Menu interaction

- A barra de navegação usa itens compactos; cada seção abre um único painel contextual alinhado ao item selecionado, com reposicionamento horizontal para permanecer dentro da janela.
- Abrir ou alternar menus não pausa a emulação nem altera o layout da área de vídeo ou o estado de fullscreen.
- Clique fora do painel ou `Esc` fecha o menu. A tipografia e os controles de navegação usam escala reduzida.
- Validar visualmente cada submenu em janela normal, maximizada e fullscreen, incluindo os itens de ROM recente e os menus próximos à borda direita.

## Runtime contract

- SmsTemporalWorker owns the SMS machine and its execution thread.
- MachineScheduler remains the sole authority for emulated time.
- Start liga e executa; Pause congela preservando o estado; Stop executa POWER OFF (reinicializa a máquina sem descarregar a ROM); Reset reinicializa e retoma automaticamente; Step-frame avança um quadro enquanto pausado. Todas são operações coarse-grained do worker.
- Completed immutable VideoFrame values are published through FrameMailbox.
- The UI timer consumes only the newest pending frame; it never advances emulation.
- RGB888 is converted to tightly packed RGBA8 only at the presentation boundary.
- Slint/WGPU uses the renderer's own Vulkan device and queue.
- Audio output is provided through the CPAL 0.18.2-backed `sega-audio-output` crate. The digital PSG generation remains owned by the SMS runtime; physical output is a host concern and must not control Scheduler time.

## Run

From the repository root on Windows x64:

```powershell
cargo run --release --locked --manifest-path native/sega_desktop/Cargo.toml -- [path\\to\\game.sms] [usa|brazil|europe|japan]
```

The ROM argument is optional. Without it, use Arquivo > Procurar or enter an absolute path manually. The native Windows file-selection dialog filters `.sms`, `.bin` and `.zip`. A ZIP must contain exactly one non-directory `.sms` / `.bin` entry; archives are read in memory, with limits on archive size, entry count and decompressed ROM size. No archive-controlled path is extracted to disk. The selected region and sprite compatibility preference are applied to each newly loaded machine. Recent ROMs are persisted in the desktop settings directory. Clicking the video viewport resumes a paused machine; F11 toggles fullscreen.

## Validation commands

Run these from the repository root on Windows x64:

```powershell
cargo fmt --manifest-path native/sega_desktop/Cargo.toml -- --check
cargo clippy --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc -- -D warnings
cargo test --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc
cargo build --release --locked --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc

cargo fmt --manifest-path native/audio_output/Cargo.toml -- --check
cargo clippy --release --locked --all-targets --manifest-path native/audio_output/Cargo.toml --target x86_64-pc-windows-msvc -- -D warnings
cargo test --release --locked --all-targets --manifest-path native/audio_output/Cargo.toml --target x86_64-pc-windows-msvc
```

A successful build does not prove physical Vulkan presentation or audible output. On physical Windows/Vulkan hardware, load a legally obtained SMS ROM and verify real VDP output, menu interaction, ROM replacement, pause/resume/POWER OFF/restart/reset-auto-resume/step-frame, aspect-ratio modes during continuous resize, fullscreen, ZIP ROM selection and rejection cases, sprite-limit toggling, trace capture, clean shutdown and physical CPAL playback. Record the exact commit, GPU/driver, executable SHA-256, audio endpoint/configuration, logs and observations.

## Verified Windows x64 Release baseline — 2026-11-14

At commit `118b96bf411248173b6215b7bf16aebdf3a66bff`, the local Windows x64 report recorded:

- Desktop formatting, Clippy with `-D warnings`, all-target tests and Release build: PASS.
- Desktop automated tests: 26 passed, 0 failed.
- Independent `native/audio_output` formatting, Clippy and all-target tests: PASS.
- CPAL output crate tests: 21 passed, 0 failed.
- Release executable found at `target\\x86_64-pc-windows-msvc\\release\\sega-desktop.exe`, 22,192,640 bytes.

These results validate compilation and the automated tests only. Physical window startup, Vulkan adapter/device execution, ROM gameplay, sustained frame presentation, audible CPAL output, endpoint hotplug and long-duration stability remain NOT RUN pending the Windows in-game acceptance session. See [Windows x64 Release validation report](../../docs/testing/windows-x64-release-validation-2026-11-14.md).

## R2.3 — persistent frontend preferences

- Persists presentation-only preferences in `%APPDATA%\\Sega Emulator\\slint_ui_settings.txt`: SMS region, aspect ratio, FPS visibility, footer visibility and FPS text size.
- Persists up to ten recent ROM paths in `%APPDATA%\\Sega Emulator\\recent_roms.txt`, newest first. Selecting a recent entry uses the same safe worker handoff as manual ROM loading.
- Recent ROM history and preferences are saved after UI changes and on application shutdown. They do not contain or serialize machine state.
- FPS text size offers small, medium and large values. HUD corner placement remains pending because the current Slint composition does not yet have a separately positioned overlay surface.

## Runtime video and save-state APIs (SMS1)

- The runtime exposes `SmsOverscanMode::Off` and `CropLeft8`. The latter crops the first eight pixels from each active line at the frame-publication boundary (248×192); it does not mutate the VDP's canonical 256×192 framebuffer or emulated timing. Default is Off.
- `SmsTemporalWorker::set_overscan_mode` controls the published frame crop. Persisting and exposing this option in Slint remains pending.
- The sprite-line compatibility switch is already wired to the worker and persisted in desktop preferences. HardwareLimit is the default; UnlockedCompatibility is an opt-in compatibility mode.
- `SmsTemporalWorker::save_state` and `load_state` provide version-validated in-memory snapshots/restores. The envelope binds the state to its format version, region, ROM bytes and VDP/PSG hardware profile. Restore is permitted only while paused. Persistent state files/slots and a portable ROM SHA-256/checksum container are not implemented yet; SRAM `.srm` files are separate.
