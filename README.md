# Sega Emulator — Rust / Slint + WGPU + CPAL

Emulador Sega multissistema com runtime de hardware integralmente em Rust. A arquitetura desktop de destino é **Slint 1.18.x + WGPU 30.x + CPAL 0.18.2**, com Vulkan como backend gráfico inicial preferencial no Windows x64.

**Baseline operacional:** em 2026-11-14, a branch `main` foi validada localmente em Windows x64/MSVC no commit `118b96bf411248173b6215b7bf16aebdf3a66bff`. O host Slint passou por formatação, Clippy sem warnings, 26 testes automatizados e build Release; o crate independente CPAL passou por formatação, Clippy sem warnings e 21 testes. O executável Release foi encontrado. Isso não substitui o aceite físico: inicialização, Vulkan efetivo, gameplay, apresentação sustentada e reprodução física CPAL ainda precisam ser testados. Consulte o [relatório de validação Windows x64 Release](docs/testing/windows-x64-release-validation-2026-11-14.md).

## Migração

- **R1:** POC Slint/WGPU/Vulkan integrada. A validação automatizada e a execução física inicial não dispensam os gates restantes de lifecycle/teardown.
- **R2:** frames reais SMS integrados via SmsTemporalWorker e FrameMailbox, preservando MachineScheduler; o aceite manual Windows/Vulkan permanece pendente; o problema de frame pacing da issue #755 foi encerrado como resolvido em 2026-11-14 e não é um gate aberto.
- **R3:** CPAL 0.18.2 e pipeline PCM moderno integrados em software. Os testes automatizados do crate de saída passaram; reprodução física prolongada, formatos reais do endpoint, underruns e hotplug ainda precisam de aceite.
- **R4:** auditoria e integração SDL3 para teclado/gamepads; teclado SMS básico já é tratado pelos eventos nativos do Slint.
- **R5:** consolidar launcher, biblioteca, configurações, HUD e telemetria; retirar hosts anteriores somente após paridade e gates.

Consulte [status operacional](docs/project-status-2026-11-14.md), [status histórico e auditoria anterior](docs/project-status-2026-11-12.md), [roadmap](docs/project-roadmap.md), [plano de corte](docs/architecture/migrations/legacy-host-retirement.md) e [ADR 0011](docs/adr/0011-slint-wgpu-cpal-desktop-platform.md).

## Arquitetura

**Runtime Rust preservado:** CPU Z80 compartilhada, memória, barramentos, mappers, VDP, PSG, interrupções, MachineScheduler, timing e estados. O Scheduler é a autoridade temporal. A GUI não executa instruções, avança clocks ou controla ciclos.

**Desktop alvo:** Slint 1.18.x para GUI; WGPU 30.x para renderização; Vulkan via WGPU no Windows x64; CPAL 0.18.2 para áudio; SDL3 para gamepads após auditoria.

Estrutura de referência: native/sega_desktop, native/video_output_wgpu e native/audio_output. A estrutura real prevalece; não duplicar crates/componentes.

A arquitetura anterior permanece apenas como caminho transitório durante a migração controlada. Inventário, dependências remanescentes e critérios de retirada estão em [plano de corte dos hosts anteriores](docs/architecture/migrations/legacy-host-retirement.md). Não confundir documentação histórica ou código ainda necessário ao host anterior com arquitetura-alvo.

## Build e validação

Builds do aplicativo e avaliações de desempenho devem usar Release. Não usar Debug como referência de desempenho. Conferir formatação, Clippy e testes direcionados antes de enviar ao workflow.

Os manifests do desktop e de `native/audio_output` são validados separadamente:

- `cargo fmt --manifest-path native/sega_desktop/Cargo.toml -- --check`
- `cargo clippy --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc -- -D warnings`
- `cargo test --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc`
- `cargo build --release --locked --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc`
- Repetir fmt, Clippy e testes com `--manifest-path native/audio_output/Cargo.toml`.

Validar workspaces standalone e targets específicos separadamente. SKIPPED é NOT RUN, nunca PASS. Todo resultado deve corresponder ao SHA exato.

Não usar JIT, frame skipping, hacks temporais ou otimizações especulativas. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos. Projeto independente, sem afiliação à SEGA.
