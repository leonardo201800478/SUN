# Status dos componentes — revisão 2026-11-16

**Baseline:** `main` `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`.  
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
| BIOS SMS | Implementação candidata na PR #928 | Teste físico com BIOS real, handoff e início do cartucho pendentes |
| SG-1000 | Core funcional legado e componentes temporais existentes | Conectar máquina ao Scheduler atual; fechar CPU/VDP/PSG/I/O, snapshots e execução em blocos |
| Game Gear | Scaffold/planejamento inicial | Auditoria independente e especificação própria após SG-1000 |
| YM2413/FM | Modelo arquitetural e documentação existentes | Implementar após Game Gear; validar chip, barramento, $F2, perfis, mixer e snapshots |
| Sega Card/My Card | Referência de hardware documentada | Implementar slot/media arbitration após Game Gear |
| Card Catcher | Adaptador documentado para SG-1000/SC-3000 | Tratar como adaptador de mídia desses sistemas, não como slot nativo SMS |
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

## CI da baseline

Main `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`: run #43945796678 e SMS Runtime #43945796477. Quatro checks SUCCESS e 14 SKIPPED. SUCCESS inclui Rust quality/unit, Z80 focused, change impact e SMS Runtime Release format/Clippy/tests. Os jobs de PSG, workspace completo, CPAL, frame pacing, Slint/WGPU, SDL3, Scheduler stress, SMS1 VDP, input core, documentação e workflow validation foram SKIPPED nesta execução; não inferir validação desses domínios.
