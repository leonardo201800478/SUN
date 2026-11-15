# SMS1 VDP — scorecard de conclusão

**Sistema-alvo:** Sega Master System 1
**ASIC prioritário:** Sega 315-5124
**Backend:** Rust / `sega-video`
**Baseline integrada:** `main` após PR #916 (`e8b6f877f9904deabda5495baa461304deb4e16c`)
**Baseline de comparação:** `202cdfc695d0c2346c86da4144b6e8a3316e0c4d`
**HEAD de validação da PR #843:** `0f2415f9382887c94d9214abe06cb6c295ed2154`
**Estado:** EM DESENVOLVIMENTO — compatibilidade funcional SMS1 ainda em fechamento

Este scorecard registra somente capacidades presentes na `main` e evidências executadas no SHA correspondente. PR aberta, branch de trabalho ou teste ainda em execução não contam como implementação integrada.

Matriz independente de evidências: [SMS1 VDP temporal evidence matrix](audits/sms1-vdp-temporal-evidence-matrix-2026-11-14.md).

Escopo funcional atualizado: [SMS1 software compatibility scope](audits/sms1-vdp-software-scope-2026-11-14.md).

## 1. Critérios funcionais

| Área | Estado | Evidência / limite |
|---|---|---|
| Perfil SMS1 / 315-5124 | IMPLEMENTED | Perfil explícito separado de SMS2, SG-1000 e Game Gear |
| DAC / paleta SMS1 | VALIDATED_FUNCTIONAL | PR #825; níveis digitais RGB específicos do SMS1. Não é calibração analógica completa |
| VRAM / CRAM / portas | IMPLEMENTED_FUNCTIONAL | Portas, command latch, read-ahead, wrap e endereçamento separados |
| Background Mode 4 | IMPLEMENTED_SCANLINE | Tiles, planos, paletas, prioridade, scroll e máscaras funcionais |
| Escritas CRAM intralinha | SELECTIVE_SPANS | PR #827; prefixo/sufixo por boundary de escrita. CRAM dot e latch físico da paleta não modelados |
| Escritas R7 intralinha | SELECTIVE_SPANS | PR #827; commit anterior à alteração do backdrop |
| Escritas VRAM intralinha | SELECTIVE_SPANS | PR #828; commit antes da escrita. Fetch-ahead de tiles/bitplanes/sprites não modelado |
| Registradores R1-R7 intralinha | SELECTIVE_SPANS_APPROXIMATE | PR #829 integrada; boundary de escrita é modelo coarse, não latch físico medido por registrador |
| R0 e transição Mode 4/legacy | SELECTIVE_SPANS_APPROXIMATE | PR #831 integrada; commit antes de R0 e renderer legacy/TMS por spans. Boundary coarse, não timestamp físico de latch |
| R8 horizontal scroll | IMPLEMENTED_APPROXIMATE | Latch em HPos 21; fase absoluta ainda sem validação física |
| R9 vertical scroll | IMPLEMENTED_APPROXIMATE | Valor pendente aplicado na fronteira de scanline conforme modelo atual |
| R10 / HINT counter | IMPLEMENTED_TSTATE_QUANTIZED | Decremento/reload em T14 (boundary HPos≈21); testes confirmam boundary, snapshot/restore e propagação INT ao Z80. PR #916 adiciona amostragem CPU-visible pela ISR para R10 `$00`, `$61`, `$80`, `$AF` e `$C0`, após sincronização VINT→HINT. Isso valida os valores lidos pelo Z80 no fixture temporal, mas não equivale à execução integral da ROM SmsVdpTest v1.31. HPos MAME 26 não é comparável sem normalizar a origem |
| SAT / avaliação de sprites | IMPLEMENTED_SCANLINE | PR #832/#833: Mode 4 e legacy/TMS capturam metadados X/tile/pattern/color e controles R0/R6 durante avaliação por scanline; bytes de padrão continuam lidos na composição; timestamp físico não medido |
| Sprite overflow | IMPLEMENTED_TSTATE_QUANTIZED | Mode 4 e legacy/TMS: índice associado latched em HPos 24 / T16, usando o buffer do modo ativo. PR #916 valida pelo caminho Z80→Scheduler→VDP a flag de overflow e o índice `$08` do nono sprite |
| Sprite collision | IMPLEMENTED_PIXEL_COMMIT_APPROXIMATE | Sobreposição de pixels opacos publicada quando um commit de span cruza o pixel; leitura de status comita o prefixo até o feixe; não há pipeline de colisão por dot. PR #916 valida a flag collision por leitura Z80 de status após programação da SAT/padrões via OUT |
| HCounter / porta $7F | COMPATIBILITY_ONLY | Leitura e latch sintético por software permanecem disponíveis. Captura elétrica por TH/Light Phaser não faz parte do escopo atual; a entrada será ponteiro/mira de software, sem pistola física |
| Entradas externas TH1/TH2 | OUT_OF_SCOPE_FOR_POINTER_INPUT | A integração genérica de TH permanece no core, mas captura física de Light Phaser não é requisito para a mira controlada por mouse/analógico/giroscópio |
| HBlank | IMPLEMENTED_TSTATE_QUANTIZED | T213/HPos319 fecha os 256 pixels ativos; T212/HPos318 não fecha o último pixel. Teste de colisão no pixel 255 valida o boundary lógico, não a fase elétrica física |
| VBlank / IRQ | IMPLEMENTED_TSTATE_QUANTIZED | VINT pendente em T13 (HPos≈20); frame publicado em T16; VBlank status em T17/HCount `$F5` (PR #910). VINT IRQ permanece separada da flag. PR #916 valida a leitura CPU-visible de VCounter `$C1` na ISR IM1/HALT (RunTest31). Chunking e snapshot/restore validados; HCount observável após HALT/ISR ainda em auditoria |
| Scheduler / determinismo | IMPLEMENTED_INTEGRATED | VDP e entradas TH externas avançam pelo Scheduler; eventos TH têm timestamp absoluto, ordem de empate explícita, snapshot/restore e trace |
| Save State | RUNTIME_IN_MEMORY_SNAPSHOT_RESTORE | PR #845: captura/restaura CPU, Scheduler, bus, mapper, VDP, PSG e eventos pendentes; envelope valida versão, ROM, região e revisão/padrão de vídeo. Restore exige pausa. Persistência binária em arquivo/slots ainda pendente; não confundir com SRAM `.srm` |
| Overscan / borda esquerda | RUNTIME_PRESENTATION_CROP | PR #845: `SmsOverscanMode::CropLeft8` publica imagem 248×192 removendo apenas os oito pixels da esquerda; framebuffer canônico 256×192 e temporização permanecem intactos. Não é uma simulação de overscan elétrico. O mask de R0 bit 5 continua sendo o comportamento VDP original |
| Limite de sprites | IMPLEMENTED_UI_AND_RUNTIME | `HardwareLimit` e `UnlockedCompatibility` já estão ligados ao worker e à preferência persistida da UI; o modo desbloqueado altera composição, não o clock nem o estado de overflow do hardware |
| SMS2 / outros perfis | REGRESSION_GATED | Renderer e perfis não SMS1 preservados pelos testes direcionados; não generalizar spans SMS1 |

## 2. Política de granularidade

- Scanline permanece a unidade predominante de processamento.
- Spans são commitados somente em boundaries de escrita com efeito visual observável e implementado.
- A janela ativa usa HPos 63 como início e HPos 319 como fim exclusivo, no modelo comparativo 228 T-states / 342 HPos.
- O Scheduler continua sendo a autoridade temporal; o VDP não possui relógio independente.
- Não declarar dot-accuracy, fetch timing ou latch físico sem evidência independente.
- MAME é referência comparativa, mas seu código 315-5124 declara mid-scanline changes como trabalho pendente; não é oráculo para a semântica CRAM/R7/VRAM.

## 3. Lacunas de software que bloqueiam a liberação

1. Fechar a reprodução CPU-visible de `Frame IRQ HCount` (RunTest32) e `Line IRQ HCount` (RunTest34) com o fluxo Z80/Scheduler/TH da SmsVdpTest. A PR #912 mantém os asserts de conformance `#[ignore]`: a fixture isolada passa a primeira amostra (`$82` / `$88`), mas a segunda observa `$81` / `$87`, em vez de `$84` / `$8A`; phase sweep inicial de 0–7 NOPs não alterou os valores. O fixture ainda não reproduz a fase de entrada do ROM completo. PR #916 fechou a cobertura CPU-visible de RunTest25, RunTest31 e RunTest33, mas não substitui RunTest32/34. Normalizar a origem HPos antes de comparar MAME HINT_HPOS26/VINT_HPOS24; não mover constantes sem teste discriminante.
2. Expandir testes automatizados de escritas CRAM durante a linha, mudanças de backdrop e spans antes/depois do boundary, usando ROMs diagnósticas quando disponíveis.
3. Modelar ou justificar por evidência o fetch-ahead de tiles, bitplanes e padrões de sprites.
4. Modelar fetches de padrão por sprite/dot e separar seleção SAT de fetch real; Mode 4 e legacy/TMS já capturam metadados por scanline, mas bitplanes continuam consultados durante a composição.
5. Executar ou reproduzir fielmente os testes `RunTest32` (Frame IRQ HCount) e `RunTest34` (Line IRQ HCount) da SmsVdpTest, sem ignorar os asserts de conformance. Registrar a primeira divergência em timestamps de evento, amostragem T4 do HALT, acknowledge, ISR ou captura TH; preservar VINT T13, HINT T14, VCounter T15, overflow/frame publish T16 e VBlank flag T17 até haver evidência contrária.
6. Confirmar o instante de colisão pixel a pixel: o modelo publica a colisão no commit do pixel, mas não possui pipeline dot-level do ASIC.
7. Expandir a demonstração de equivalência contínua/em blocos/save-load para todos os eventos subline. VINT já possui equivalência contínuo/blocos com snapshot em T12; TH software possui invariância 3+5 T-states.
8. Executar regressões SMS1/SMS2 e build Windows x64 Release no candidato final.
9. Integrar a mira de software ao host: coordenadas do mouse, analógico e giroscópio devem ser transformadas para coordenadas lógicas do frame após aspect ratio/crop; desenhar mira branca somente na composição de apresentação, sem gravá-la no framebuffer do VDP.
10. Implementar serialização persistente dos snapshots versionados em arquivos/slots, com identidade da ROM, checksum, gravação atômica e rejeição de versões incompatíveis. A API de snapshot/restore em memória já está disponível.
11. Registrar os comportamentos ainda aproximados antes da primeira campanha in-game. Medições físicas e pistola Light Phaser não são gates deste escopo.

## 4. Evidências integradas

- PR #825 — DAC SMS1: níveis RGB e regressões de framebuffer.
- PR #826 — colisão de sprites: pixels opacos, limpeza por leitura de status e máscaras/deslocamentos.
- PR #827 — spans CRAM/R7: 79 testes `sega-video`, 14 testes SMS de integração, runtime Release e build Windows x64 Release aprovados no HEAD validado.
- PR #828 — spans VRAM: 81 testes `sega-video`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, runtime Release e build Windows x64 Release aprovados no HEAD validado.
- PR #829 — spans R1-R7: 83 testes `sega-video`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, runtime Release e build Windows x64 Release aprovados no HEAD validado.
- PR #830 — R0 em Mode 4: 84 testes `sega-video`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, runtime Release e build Windows x64 Release aprovados no HEAD validado.
- PR #831 — spans Mode 4/legacy e R0: 91 testes `sega-video`, 149 testes `sega-sms-core`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, validação Z80/DD-FD, runtime Release (45 + 1 testes) e build Windows x64 Release aprovados no HEAD `19c34080757f84aec5d7fb24a5dd3355badf5d41`.
- PR #832 — metadados SAT Mode 4: 92 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, validação Z80/DD-FD, runtime Release e build Windows x64 Release aprovados no HEAD `1e0a1191f7bdc37c57520a3cc6b78e142206b1c6`.
- PR #833 — buffer SAT legacy/TMS: 93 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, validação Z80/DD-FD, runtime Release e build Windows x64 Release aprovados no HEAD `ab37d5ae69a7e754fb46a0b7d98883ccf8089719`.
- PR #834 — boundaries HBlank/HINT/VBlank/overflow (primeira aproximação): 95 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes SMS de integração, Rustfmt, Clippy, documentação, validação Z80/DD-FD, runtime Release e build Windows x64 Release aprovados no HEAD `c5e59b6a40e2158546c1d0a5bd857add34b42d08`.
- PR #836 — correção temporal baseada na SmsVdpTest: evidência histórica do HEAD `6449b7085c019a95d2cd4f0ca1c364a0bf929962`.
- PR #837 — atualização do scorecard temporal.
- PR #838 — leitura estável do H-counter por `$7F` e portas espelhadas; o primeiro workflow classificou incorretamente o caminho temporal e não executou os testes do SMS core.
- PR #839 — corrige a classificação de impacto temporal; adiciona invariância de captura TH para avanço contínuo versus blocos e boundary HBlank T212/T213 com colisão no último pixel. No HEAD `c7afd54c22d68dc3a7e9d94e26bbd780bd1564f0`: 98 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes de integração SMS; Rustfmt, Clippy, Z80 focused, input focused, Scheduler stress, workspace regression, runtime Release (45 + 1 testes) e build Windows x64 Release aprovados.
- PR #841 — integra transições externas TH1/TH2 no Scheduler, níveis active-low em $DD, snapshot/restore, desempate phase 0/phase 1 e registro no trace. No HEAD `82742d3c1b5fef3a45d0185529d5b9330f0d4180`: 98 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes de integração SMS; input/console, Z80, Scheduler stress, workspace regression, Clippy, Rustfmt, runtime Release (45 + 1 testes) e build Windows x64 Release aprovados.
- PR #843 — valida HINT/R10 e VINT nos boundaries do Scheduler: VINT pendente T13, VBlank flag T16, HINT T14, snapshot/restore e equivalência contínuo/blocos; confirma propagação HINT à linha INT do Z80. No HEAD `0f2415f9382887c94d9214abe06cb6c295ed2154`: 98 testes `sega-video`, 131 testes `sega-sms-core`, 14 testes de integração SMS; Z80 focused, input focused, Scheduler stress, workspace regression, Clippy, Rustfmt, runtime Release (45 + 1 testes) e build Windows x64 Release aprovados.
- PR #909 — colisão de sprites SMS1 no border vertical; restrita ao ASIC 315-5124, sem desenhar pixels fora da área visível ou gerar overflow indevido. Merge commit `355bf02b111d2d691762071360dbe502b64a6aba`.
- PR #910 — separa publicação do frame em T16 da flag VBlank em T17/HCount `$F5`; VINT IRQ permanece inalterada. HEAD validado `3fb41e8184a08ab07e5a258ae9d1239fbe40749a`, merge commit `a41c7706c2527858fb4454837afca21b4f0ae108`; CI, SMS Debugger Runtime, Scheduler stress, workspace regression e Windows x64 Release aprovados.
- PR #914 — adiciona três testes de integração CPU-visible em `native/sms_core/tests/sms_vdp_vcounter_integration.rs`: leitura de VCounter antes/em T15 via `IN A,(n)` (T239 `$00`; T243 `$01`), serviço VINT por Z80 IM1/HALT com leitura de status e VCounter `$C1` na ISR, e HINT R10=`$BE` observado em VCounter `$C0` antes de VBlank. Merge commit `7de9d2d1dbd260a9c9ce600bf61281b2f6478443`. CI pós-merge `43758763908` aprovou classificação, validação Actions, regressão Rust do workspace e SMS1 VDP core/machine integration; SMS Runtime Release `43758763928` aprovou fmt, Clippy, testes e build Windows x64 Release. Fixtures são discriminantes CPU-visible, não reprodução integral da ROM SmsVdpTest.
- PR #916 — adiciona testes CPU-visible pelo fluxo Z80→barramento→MachineScheduler→VDP: RunTest25 (VCounter `$00` em T239, `$01` em T243 e `$01` em T247), RunTest31 (ISR VINT lê `$C1`), RunTest33 (ISR HINT lê os cinco valores de R10 `$00/$61/$80/$AF/$C0`), colisão de sprites (status bit `$20`) e overflow do nono sprite (bit `$40`, índice `$08`). Nenhum código de hardware de produção foi alterado. Merge commit `e8b6f877f9904deabda5495baa461304deb4e16c`; CI `43762412673` aprovou SMS1 VDP core/machine integration, regressão Rust e qualidade Rust; SMS Runtime Release `43762412677` aprovou fmt, Clippy e testes.
- PR #912 — branch exploratória `test/sms1-vdp-irq-hcount-20261115`, sincronizada com a main após PR #916. OUT/IN e phase diagnostic permanecem isolados. RunTest32/34 continuam `#[ignore]`: a fixture independente observa `$81/$87` na segunda amostra, enquanto a ROM espera `$84/$8A`; o phase sweep de 0–7 NOPs não resolve a diferença. PR permanece Draft e não integrada.
- PR #845 — crop de apresentação SMS1 `CropLeft8` (248×192) sem mutação do framebuffer/timing; APIs runtime de snapshot/restore em memória com validação de versão/ROM/região/perfil; teste confirma nove sprites renderizados em modo desbloqueado mantendo overflow de hardware.
  No HEAD `bfbef337a7f894911d9bc976cdcad88b3902858f`: 99 testes `sega-video`, 131 testes `sega-sms-core`, 15 testes de integração SMS, 45 testes runtime e 6 testes de API pública; Rustfmt, Clippy, workspace regression, documentação, build runtime Windows x64 Release e POC Slint/WGPU/Vulkan Windows x64 aprovados.

Referência de timing independente: [SmsVdpTest](https://github.com/FluBBaOfWard/SmsVdpTest). A ROM ainda não foi executada pelo runtime deste projeto.

Os resultados acima pertencem aos respectivos commits e workflows; não são automaticamente atribuídos a commits futuros.

## 5. Critério de liberação

**SMS1 VDP permanece EM DESENVOLVIMENTO e ainda não está liberado como candidato geral de compatibilidade.** A ausência de medição física e a pistola Light Phaser não bloqueiam este escopo; os bloqueios restantes são de cobertura funcional, validação automatizada e integração de apresentação.

A aprovação dos spans CRAM/R7/VRAM e Mode 4/legacy comprova apenas esses incrementos. A liberação exige fechar os bloqueios da seção 3, executar os testes de integração ampliados e validar o candidato final na `main`.
