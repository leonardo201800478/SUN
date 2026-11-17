# Status dos componentes — revisão 2026-11-17

**Baseline:** `main` `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`.
**Regra:** estado integrado na main separado de PRs, testes físicos e relatos manuais.

| Área | Estado na baseline | Pendência real |
|---|---|---|
| Rust/runtime | Integrado | Gates devem ser vinculados ao SHA de cada mudança |
| Desktop Slint/WGPU/Vulkan | Caminho ativo | Revalidar execução física Vulkan, resize/lifecycle e apresentação sustentada no HEAD atual |
| Host gráfico anterior | Fora de uso ativo | Código remanescente é legado; inventariar consumidores antes de remover |
| CPAL 0.18.2 | Integrado em software pela PR #770 | Reprodução física prolongada, hotplug/reconexão, underruns e negociação de formatos |
| SDL3/input runtime | Primeira integração Slint/SDL3 pela PR #867 | Controles físicos, hotplug real, identidade persistente, remapeamento e perfis |
| Z80 compartilhado | `CLOSED_WITH_LIMITATIONS` no perfil NMOS delimitado | CMOS/clones, WAIT físico, DMA e validação elétrica permanecem não verificados; E8 é complementar |
| MachineScheduler | Integrado, com testes determinísticos | Não confundir stress de software com sincronismo físico sistêmico |
| SMS1/SMS2 | Runtime integrado; correções recentes presentes | BIOS boot aguarda validação física; VDP/HCounter e raster só reabrir com evidência atual |
| SMS sprites | Alinhamento reportado pelo usuário como corrigido | Sem defeito aberto conhecido; reabrir somente por regressão reproduzível |
| Road Rash | Testado fisicamente pelo usuário e reportado OK | Removido do backlog de defeitos; sem SHA/relatório técnico atribuído |
| BIOS SMS | Implementação integrada pela PR #928 | Validação física com BIOS real, handoff e início do cartucho permanecem pendentes |
| SG-1000 | P2.2/P2.3/P2.4 integradas na main pelas PRs #988/#989/#990. `Sg1000Machine` usa `Z80MicroSequencer`; `Sg1000TemporalBus` aceita memória, PortRead e PortWrite; CPU/VDP/PSG avançam no mesmo MachineScheduler; worker Rust e `DesktopSession` reutilizam o host Slint/WGPU/CPAL/SDL3. A máquina atômica antiga está limitada a `cfg(test)` como oracle | P2 não está fechado: gameplay real dos perfis Type-A/Type-B/2 KiB/Othello/The Castle, validação física Windows de vídeo/áudio/input, hotplug/underruns/lifecycle e replay dedicado de eventos ainda não demonstrados |
| Game Gear | Scaffold/planejamento inicial | Auditoria independente e especificação própria após SG-1000 |
| YM2413 | Modelo arquitetural e documentação existentes | Após Game Gear, implementar somente YM2413 integrado do Master System japonês; FM Sound Unit externo do Mark III excluído |
| Sega Card/My Card | Mídia de software documentada | Preservar carregamento e slots nativos SMS/Mark III; modelar seleção/arbitragem conforme geração após Game Gear |
| Card Catcher C-1000 | Adaptador físico histórico documentado | Excluído da implementação; não transportar lógica para o core SG-1000 |
| 3D Glasses / outros periféricos | Referências técnicas disponíveis | Especificação e implementação na fase P4, por compatibilidade de console |
| Debugger SMS | Parcial e em evolução | Captura de duração fixa, snapshots CPU/VDP, exportação e usabilidade; não bloqueia hardware |
| R5 desktop | Parcial | Biblioteca, configurações consolidadas, HUD/telemetria e limpeza segura de legado |

## Estados e proveniência

- `VALIDATED`: gates automatizados executados e aprovados no SHA declarado, no escopo declarado.
- `INTERNAL_REVIEW`: revisão interna concluída, sem alegar confirmação externa.
- `NOT_VERIFIED`: não há evidência suficiente.
- `CLOSED_WITH_LIMITATIONS`: escopo praticável encerrado com limitações explícitas.
- Evidência física fornecida pelo usuário deve ser registrada como relato de teste físico, sem inventar SHA, log ou ambiente.
- `SKIPPED` e `NOT RUN` nunca são PASS.

## SG-1000 P2 — rastreabilidade

- P2.2: PR #988, merge `be883792025de3db7eaf4e7b8243293ea90b7e06`; máquina temporal, M1/WAIT, portas e snapshots compostos.
- P2.3: PR #989, merge `a728c448bb656f19650e88e214a9a723bbcc2713`; worker Rust, FrameMailbox, AudioSession e save-state.
- P2.4: PR #990, merge/main `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`; DesktopSession, seleção/roteamento multi-sistema e perfis de RAM por hash.
- Auditoria detalhada: [sg1000-p2-post-p24-reconciliation-2026-11-17.md](audits/sg1000-p2-post-p24-reconciliation-2026-11-17.md).
- P2 permanece aberto até validação in-game dos perfis de cartucho e validação física do host. Não iniciar P3/Game Gear antes dos gates de saída.

## CI da baseline

Main `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`: push run [#44135959914](https://github.com/leonardo201800478/sega-emulator/actions/runs/44135959914). Change impact, Rust quality/unit, Rust workspace, Slint/WGPU/Vulkan POC, SDL3 input, PowerShell tooling e documentation hygiene PASS. CPAL output, PSG, Scheduler stress, Z80 focused, SMS1 VDP e outros jobs foram SKIPPED; portanto, não há aceite físico de áudio nem validação in-game dos perfis expandidos neste SHA.

A PR #980 foi integrada em `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434`; seu CI de PR [#43959831347](https://github.com/leonardo201800478/sega-emulator/actions/runs/43959831347) e push pós-merge [#43962737265](https://github.com/leonardo201800478/sega-emulator/actions/runs/43962737265) passaram em change impact, documentação e Rust quality/unit. Os demais jobs foram SKIPPED. Esta atualização não declara auditoria integral dos 285 documentos.
