# Sega Emulator — Rust / Slint + WGPU + CPAL

Emulador Sega multissistema com runtime de hardware integralmente em Rust. A arquitetura desktop de destino é **Slint 1.18.x + WGPU 30.x + CPAL 0.18.2**, com Vulkan como backend gráfico inicial preferencial no Windows x64.

**Baseline de arquitetura:** backend Rust determinístico; desktop Slint 1.18.x; apresentação WGPU 30.x com Vulkan inicial no Windows x64; saída de áudio CPAL 0.18.2; input SDL3 via `sega-input-runtime`. R1 está integrada; os gates físicos devem ser vinculados ao HEAD atual. R2 (frames SMS) e R3 (CPAL) estão integradas em software; frame pacing foi resolvido, mas os aceites físicos sustentados não estão comprovados no relatório histórico. R4: primeira integração SDL3/Slint integrada pela PR #867 (`f93cb777`), com CI #7512 verde; aceite físico com controles reais e recursos avançados permanecem pendentes.

## Migração

- **R1:** POC Slint/WGPU/Vulkan integrada e validada no CI Windows; inicialização física, seleção efetiva do Vulkan, resize/lifecycle e apresentação sustentada precisam de evidência no HEAD atual.
- **R2:** frames SMS integrados via `SmsTemporalWorker` e `FrameMailbox`, preservando o `MachineScheduler`; frame pacing resolvido. Repetir e registrar gameplay/apresentação no HEAD atual.
- **R3:** CPAL 0.18.2 e pipeline PCM integrados; testes automatizados Windows aprovados em SHAs anteriores. Reprodução física sustentada, hotplug, recuperação de endpoint e underruns ainda exigem aceite no HEAD atual.
- **R4:** teclado SMS via Slint e primeira integração SDL3 conectada ao desktop Slint, integrada pela PR #867. CI Windows SDL3 e regressão de workspace/desktop passaram; mapeamento inicial P1/P2. Teste físico com controles, hotplug real, perfis persistentes e remapeamento ainda pendentes.
- **R5:** consolidar launcher, biblioteca, configurações, HUD e telemetria; retirar hosts anteriores somente após paridade e regressão.

Consulte o [roadmap](docs/project-roadmap.md), o [plano de corte dos hosts](docs/architecture/migrations/legacy-host-retirement.md), a [validação local Windows x64 Release (snapshot 118b96b)](docs/testing/windows-x64-release-validation-2026-11-14.md), a [auditoria histórica do caminho VDP SMS1 (mesmo snapshot)](docs/audits/sms1-vdp-runtime-path-audit-2026-11-14.md) e a [ADR 0011](docs/adr/0011-slint-wgpu-cpal-desktop-platform.md). Os dois relatórios são evidências documentais vinculadas explicitamente ao commit auditado; não devem ser interpretados como revalidação de cada commit posterior da `main`.

## Arquitetura

**Runtime Rust preservado:** CPU Z80 compartilhada, memória, barramentos, mappers, VDP, PSG, interrupções, MachineScheduler, timing e estados. O Scheduler é a autoridade temporal. A GUI não executa instruções, avança clocks ou controla ciclos.

**Desktop alvo:** Slint 1.18.x para GUI; WGPU 30.x para renderização; Vulkan via WGPU no Windows x64; CPAL 0.18.2 para áudio; teclado via eventos nativos do Slint e SDL3 para gamepads após auditoria.

Estrutura de referência: native/sega_desktop, native/video_output_wgpu e native/audio_output. A estrutura real prevalece; não duplicar crates/componentes.

A arquitetura anterior permanece apenas como caminho transitório durante a migração controlada. Inventário, dependências remanescentes e critérios de retirada estão em [plano de corte dos hosts anteriores](docs/architecture/migrations/legacy-host-retirement.md). Não confundir documentação histórica ou código ainda necessário ao host anterior com arquitetura-alvo.

## Build e validação

Builds do aplicativo e avaliações de desempenho devem usar Release. Não usar Debug como referência de desempenho. Conferir formatação, Clippy e testes direcionados antes de enviar ao workflow.

Gates:
- cargo fmt --all -- --check
- cargo clippy --release --workspace --all-targets --locked -- -D warnings
- cargo test --release --workspace --locked

Validar workspaces standalone e targets específicos separadamente. SKIPPED é NOT RUN, nunca PASS. Todo resultado deve corresponder ao SHA exato.

Não usar JIT, frame skipping, hacks temporais ou otimizações especulativas. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos. Projeto independente, sem afiliação à SEGA.
