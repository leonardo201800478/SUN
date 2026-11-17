# Sega Emulator — Rust / Slint + WGPU + CPAL

Emulador Sega multissistema com runtime de hardware integralmente em Rust. A arquitetura desktop ativa utiliza **Slint 1.18.x + WGPU 30.x/Vulkan + CPAL 0.18.2**, com SDL3/input runtime para gamepads.

**Baseline remota consultada:** `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434` (2026-11-16).

## Estado resumido

- SMS1/SMS2: runtime integrado; Road Rash e alinhamento de sprites reportados pelo usuário como resolvidos. BIOS boot ainda aguarda validação física.
- Z80 compartilhado: encerrado com limitações explícitas no perfil NMOS praticável; E8/ZEXDOC/ZEXALL é complementar.
- SG-1000: P2 iniciada; a PR #981 é candidata P2.1 para PortRead/PortWrite no barramento temporal, ainda não integrada. Na main, Sg1000Machine continua usando cpu.step() e avançando VDP/PSG em fronteiras de instrução.
- Game Gear: após SG-1000, como sistema próprio.
- Após Game Gear: somente YM2413 integrado do Master System japonês; Sega Card/My Card como mídia; 3D Glasses/SegaScope e periféricos compatíveis. Mark III FM Sound Unit externo e Card Catcher C-1000 estão fora do escopo de implementação.
- 16-bit: Mega Drive/Genesis → Sega CD → 32X, somente após a fase 8-bit.
- Debugger: melhoria contínua e independente, sem bloquear hardware.
- Host gráfico anterior: não está em uso ativo; remanescentes são legado a inventariar.

## Arquitetura

MachineScheduler é a autoridade temporal. Runtime, CPU, VDP, PSG/FM, barramentos, mappers, periféricos e estados são Rust. GUI Slint, vídeo WGPU/Vulkan, áudio físico CPAL e input SDL3. A GUI não executa instruções nem avança clocks. Não duplicar componentes validados.

Consulte:
- [Escopo](docs/project-scope.md)
- [Roadmap mestre](docs/project-roadmap.md)
- [Status dos componentes](docs/component-status.md)
- [Reorganização e reconciliação de 2026-11-16](docs/project-reconciliation-2026-11-16.md)
- [Expansões e acessórios 8-bit](docs/hardware/sms-expansion-accessories-roadmap.md)
- [YM2413/FM model](docs/architecture/ym2413-fm-model.md)
- [Documentação principal](docs/README.md)

## Build e validação

Builds e benchmarks de aplicação devem usar Release. Validar workspaces/targets específicos separadamente. Gates de referência:
- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Todo resultado pertence ao SHA exato. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos. Projeto independente, sem afiliação à SEGA.
