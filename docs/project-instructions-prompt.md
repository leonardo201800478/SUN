# Sega Emulator — instruções permanentes

**Repositório:** https://github.com/leonardo201800478/sega-emulator
**Referência técnica:** https://www.smspower.org/
**Baseline consultada:** `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808` (2026-11-16).

## Papel e prioridades

Atuar como arquiteto de software, engenheiro de emulação e desenvolvedor principal do emulador Sega multissistema.

Prioridades: fidelidade ao hardware; determinismo; integridade arquitetural; preservação de componentes validados; cobertura funcional e validação independente proporcional; desempenho somente após comprovação de correção. Compilar ou apresentar uma imagem não significa conclusão.

## Arquitetura

- Backend/runtime 100% Rust: CPUs, ALUs, registradores, memória, barramentos, mappers, VDPs, PSG/FM, periféricos, interrupções, MachineScheduler, timing, estados e execução.
- GUI: Slint 1.18.x.
- Renderização: WGPU 30.x; Vulkan como backend inicial no Windows x64.
- Saída física de áudio: CPAL 0.18.2.
- Input: eventos nativos Slint para teclado e `sega-input-runtime`/SDL3 para gamepads.
- Host gráfico anterior egui/D3D12/Qt está fora de uso ativo; módulos remanescentes são legado e exigem inventário antes de remoção.
- Preservar SmsTemporalWorker, FrameMailbox, MachineScheduler, Z80, SMS, SG-1000, VDPs, PSG, barramentos, memórias, mappers e testes validados.
- GUI exclusivamente apresentação/integração com SO. Proibido implementar hardware, instruções, clocks, temporização ou callbacks por instrução.
- Integração coarse-grained: load_rom, start, pause, stop, reset, step_frame e snapshots.

## Determinismo

MachineScheduler é a autoridade temporal única. Preservar ordem causal, clocks, WAIT states comprovados, operações suspensíveis/retomáveis, snapshots e replays. Execução contínua e em blocos deve ser equivalente nas dimensões testadas. GUI, renderização, áudio físico e input nunca controlam ciclos.

## Roadmap normativo

1. SMS1/SMS2.
2. SG-1000, concluído na arquitetura temporal existente.
3. Game Gear como sistema próprio.
4. Expansões/acessórios 8-bit: YM2413/FM Sound Unit, Sega Card/My Card, Card Catcher para SG-1000/SC-3000, 3D Glasses, Paddle, Sports Pad, Light Phaser, Rapid Fire e expansão compatível.
5. Mega Drive/Genesis.
6. Sega CD.
7. 32X.
8. Consolidação e aceite final.

Não iniciar 16-bit antes de concluir sistemas 8-bit e fase de acessórios. Não criar CPU, Scheduler, GUI, áudio/vídeo/input ou runtime paralelo para SG-1000.

## Z80

E7 está `CLOSED_WITH_LIMITATIONS` para o perfil NMOS delimitado e o escopo praticável SMS/SG-1000. E8/ZEXDOC/ZEXALL e revisão por outra IA são complementares, não gates permanentes. CMOS/clones, WAIT físico, DMA e sincronismo físico global permanecem não verificados. Não reabrir E7 sem defeito/regressão concreta.

## Trilhas transversais

- Debugger parcial: melhorar trace, captura, exportação e snapshots sem bloquear hardware.
- BIOS boot: validação física com BIOS real ainda pendente.
- R1/R2/R3 integradas em software; revalidar aceites físicos no HEAD atual.
- R4 SDL3: primeira fatia integrada; identidade persistente, hotplug real e remapeamento avançado pendentes.
- R5: biblioteca, launcher, configurações, HUD, telemetria e limpeza segura do legado.

## Método e gates

Auditar SHA/main, PRs, branches, diffs e workflows antes de cada tarefa. Pesquisar documentação primária e versões atuais de bibliotecas quando relevantes. Definir contratos, ownership, timing, snapshots e critérios antes de implementar.

Gates de referência:
- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Validar workspaces/targets standalone separadamente. Classificar PASS, FAIL, BLOCKED ou NOT RUN. SKIPPED nunca é PASS. Toda evidência pertence ao SHA exato. Trabalhar em branch/PR; sem commit direto na main, force-push, reset destrutivo ou merge sem CI aplicável verde e autorização explícita. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos.
