# Roadmap mestre — Sega Emulator

**Revisão:** 2026-11-17
**Baseline consultada:** `main` `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`.
**Autoridade temporal:** MachineScheduler.
**Arquitetura desktop ativa:** Slint 1.18.x + WGPU 30.x/Vulkan + CPAL 0.18.2 + SDL3/input runtime.

O roadmap de plataformas é a sequência normativa. As trilhas de desktop, debugger, áudio host e validação são transversais e não autorizam reescrever hardware.

## 1. Diretrizes

- Runtime e hardware integralmente em Rust.
- MachineScheduler é a autoridade única de clocks, eventos e ordem causal.
- Preservar Z80 compartilhado, SmsTemporalWorker, FrameMailbox, VDPs, PSG, barramentos e componentes validados.
- GUI Slint; WGPU para renderização; Vulkan backend inicial; CPAL para áudio físico; SDL3 para gamepads.
- Host gráfico anterior fora de uso ativo; código remanescente é legado, não arquitetura-alvo.
- Sem CPU/Scheduler/PSG/VDP/worker paralelo, JIT, frame skipping, hacks temporais ou callbacks por instrução.
- Save States/replays devem preservar estado suficiente e determinismo; renderização/áudio/input não comandam ciclos.

## 2. Roadmap de plataformas

| Fase | Sistema/escopo | Critério de saída |
|---|---|---|
| P0 | Fundação Rust, toolchain, CI, contratos e hosts | Workspaces e boundaries reproduzíveis; host Slint/WGPU ativo |
| P1 | SMS1/SMS2 | Perfis 315-5124/315-5246, runtime, BIOS/cartucho, VDP, PSG, I/O, mappers e regressões consolidados |
| P2 | SG-1000 | Máquina temporal, worker Rust e sessão desktop integrados sobre Z80 compartilhado, MachineScheduler, TMS9918A, SN76489, I/O, snapshots, FrameMailbox e AudioSession | Gameplay dos perfis de cartucho, lifecycle completo e validação física do host Windows (vídeo/áudio/input) |
| P3 | Game Gear | Máquina própria com LCD 160×144, VDP/CRAM, áudio estéreo, memória/cartucho, portas e snapshots específicos |
| P4 | Expansões e acessórios 8-bit | YM2413 integrado do Master System japonês (opcional, independente de região); Sega Card/My Card como mídia; 3D Glasses/SegaScope com saídas 2D e estéreo; Paddle, Sports Pad, Light Phaser, Rapid Fire e expansões conforme compatibilidade |
| P5 | Mega Drive / Genesis | M68k, VDP, áudio, barramentos, cartuchos e Scheduler próprios, reutilizando somente contratos realmente comuns |
| P6 | Sega CD | Subsystem CD, CPUs/co-processadores e mídia conforme hardware documentado |
| P7 | 32X | SH-2 e expansão 32X após Mega Drive/Sega CD |
| P8 | Consolidação do produto | Biblioteca, launcher, configurações, HUD, telemetria, input e remoção segura de código legado |
| P9 | Compatibilidade e aceite final | Campanhas por sistema, regressões, documentação, release e limites declarados |

**Ordem obrigatória:** SMS1/SMS2 → SG-1000 → Game Gear → expansões/acessórios 8-bit → Mega Drive/Genesis → Sega CD → 32X. Não antecipar M68k ou SH-2.

### P4 — escopo inicial de acessórios

1. **YM2413:** emular o perfil de Master System japonês com YM2413 integrado e mistura simultânea com SN76489. Chip opcional por configuração Auto/On/Off, independente da região; sem emular Mark III FM Sound Unit como módulo externo.
2. **Sega Card/My Card:** mídia de software; preservar carregamento de cartões e slots nativos SMS/Mark III, com seleção/arbitragem via $3E conforme a geração. Não confundir mídia com adaptador.
3. **Card Catcher C-1000:** adaptador físico histórico para SG-1000/SC-3000, excluído da implementação. Não transportar lógica do adaptador para o core SG-1000.
4. **SegaScope/3D Glasses:** emular o protocolo $FFF8–$FFFB e os obturadores sincronizados ao VDP. A apresentação deve permitir jogar sem óculos em `Only Left Eye`, `Only Right Eye` ou `Side-by-Side`; `Both Eyes / OFF` permanece como saída 2D combinada. O modo `Anaglyph` (vermelho/ciano) é opcional e exige óculos coloridos. Essas opções são somente de apresentação: os obturadores seguem o tempo do VDP e não usam temporizadores da GUI.
5. **Periféricos DE-9:** Paddle, Sports Pad e Light Phaser, além de Rapid Fire quando suportado pelo perfil de hardware.
6. **Expansões e interfaces:** auditar pinouts, sinais, arbitragem e compatibilidade por console/revisão antes de implementar.

Cada item terá especificação própria, proveniência DOCUMENTADO/MEDIDO/INFERIDO, modelo temporal, snapshots, testes sintéticos e regressões. Não presumir que um acessório funciona em todos os consoles ou regiões.

## 3. Trilhas transversais — desktop R1–R5

| Trilha | Estado consolidado | Limites |
|---|---|---|
| R1 Slint/WGPU/Vulkan | POC e integração de software presentes | Revalidar execução física Vulkan, resize/lifecycle e apresentação sustentada no HEAD atual |
| R2 Frames SMS | Integrada via SmsTemporalWorker/FrameMailbox | Não atribuir evidência de snapshot antigo ao HEAD atual |
| R3 CPAL 0.18.2 | Integrada em software | Reprodução física prolongada, hotplug/reconexão, underruns e formatos reais ainda exigem evidência |
| R4 SDL3 input | Primeira integração SDL3/Slint presente | Controles físicos, hotplug, identidade persistente e remapeamento avançado pendentes |
| R5 Produto desktop | Parcial | Biblioteca, configurações consolidadas, HUD/telemetria e remoção segura dos componentes legados |

R5 é transversal; não deve bloquear implementação de hardware quando a integração coarse-grained necessária já existe.

## 4. Trilhas transversais adicionais

### Z80 compartilhado

Estado: `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e o escopo praticável de SMS/SG-1000. As famílias E4C/E5/E6/E7 e as extensões de F-021/F-022/F-023/F-028 possuem testes internos, diferenciais e integrações registradas. Permanecem limites: CMOS/clones fora do perfil, WAIT físico, DMA, validação elétrica e cobertura universal não demonstrados. ZEXDOC/ZEXALL e outras suítes são complementares futuras. Não reabrir E7 sem bug/regressão ou evidência concreta.

### Debugger

Implementado parcialmente e em evolução. Melhorar captura, retenção/exportação, snapshots CPU/VDP, janela de diagnóstico e usabilidade em trilha separada. Não é bloqueio para fechar SMS/SG-1000.

### BIOS

O boot por BIOS convencional de 8 KiB foi integrado pela PR #928. Validação física com firmware real, handoff e primeiro código do cartucho permanece pendente. Não declarar boot físico validado.

## 5. Estado operacional da baseline

- **Main SHA:** `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315` (17/11/2026). A main está 6 commits à frente e 0 atrás do merge documental #980 (`f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434`).
- **CI atual da main:** push run [#44135959914](https://github.com/leonardo201800478/sega-emulator/actions/runs/44135959914), no SHA exato acima. PASS: Change impact classification, Rust quality and unit regression, Rust workspace regression, Windows x64 Slint/WGPU 30/Vulkan POC, Windows SDL3 input validation, Windows PowerShell tooling syntax e Documentation patch hygiene. SKIPPED: CPAL output, PSG, Scheduler stress, Z80 focused, SMS1 VDP, input core e demais jobs não selecionados pelo classificador. SKIPPED não é PASS; este run não comprova saída física de áudio nem gameplay.
- **Reconciliação documental #980:** merged em `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434`; compare da branch: 12 commits ahead / 0 behind; 40 arquivos, todos documentação/README/AGENTS. CI no HEAD da PR [#43959831347](https://github.com/leonardo201800478/sega-emulator/actions/runs/43959831347) e no merge commit [#43962737265](https://github.com/leonardo201800478/sega-emulator/actions/runs/43962737265): Change impact, Documentation patch hygiene e Rust quality/unit PASS; jobs de runtime SKIPPED. Não houve review submetida nem review thread; comentários do Codex registram limite de uso, não uma review.
- **SG-1000 P2.2 — PR #988:** merged em `be883792025de3db7eaf4e7b8243293ea90b7e06`. A máquina ativa usa `Z80MicroSequencer`, M1/WAIT retomável, transações de memória/portas e domínios CPU/VDP/PSG no mesmo MachineScheduler. CI PR [#44090848768](https://github.com/leonardo201800478/sega-emulator/actions/runs/44090848768) e push pós-merge [#44091248478](https://github.com/leonardo201800478/sega-emulator/actions/runs/44091248478) concluíram SUCCESS; os gates aplicáveis de Scheduler, Z80, SMS1 VDP, input, SDL3, workspace e Rust quality passaram. Os demais jobs foram SKIPPED.
- **SG-1000 P2.3 — PR #989:** merged em `a728c448bb656f19650e88e214a9a723bbcc2713`. O `Sg1000TemporalWorker` possui a máquina em thread Rust dedicada, publica frames via FrameMailbox, encaminha PCM por AudioSession e oferece save-state vinculado à ROM/perfil. CI PR [#44096800272](https://github.com/leonardo201800478/sega-emulator/actions/runs/44096800272) passou em workspace, CPAL output, Slint/WGPU/Vulkan, SDL3, PowerShell, docs hygiene e Rust quality; os jobs de hardware não impactados foram SKIPPED. Os pushes pós-merge [#44103751036](https://github.com/leonardo201800478/sega-emulator/actions/runs/44103751036) e [#44103751135](https://github.com/leonardo201800478/sega-emulator/actions/runs/44103751135) concluíram SUCCESS.
- **SG-1000 P2.4 — PR #990:** merged no SHA atual `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`. `DesktopSession` unifica SMS/SG-1000; seleção de sistema é explícita; ROM/ZIP é roteada ao runtime selecionado; FrameMailbox/WGPU, AudioSession/CPAL e snapshots SDL3 são reutilizados. A identificação de cartuchos aplica hashes conhecidos aos perfis Type-A, Type-B, clone SG-1000 II 2 KiB, Othello 2 KiB e The Castle 8 KiB; ROM desconhecida conserva o mapa padrão. CI da PR [#44114863234](https://github.com/leonardo201800478/sega-emulator/actions/runs/44114863234) e push da main [#44135959914](https://github.com/leonardo201800478/sega-emulator/actions/runs/44135959914) concluíram SUCCESS. No SHA final, CPAL output, PSG, Scheduler, Z80 e SMS1 VDP foram SKIPPED.
- **Estado P2:** P2.2, P2.3 e P2.4 estão integradas em software, mas P2 ainda NÃO está fechado. Faltam execução in-game dos perfis de RAM expandidos, campanha com ROMs legais/homebrew identificadas por hash e validação física da sessão Windows (vídeo, áudio CPAL, controles SDL3, hotplug, underruns e lifecycle). A auditoria P2.4 já registra que não há evidência in-game dos cartuchos expandidos.
- **Limites temporais:** o perfil padrão CPU/PSG permanece nominal/provisório; a razão racional do PSG evita arredondamento, mas não prova o oscilador físico da placa. O domínio VDP usa T-states do modelo TMS9918A, não uma medição física de fetch/pixel. WAIT é fixture de software, não WAIT físico. Save/restore e equivalência contínua/em blocos têm testes; não foi localizado um harness dedicado de replay de log de eventos/inputs.
- **PR #981:** fechada sem merge; o trabalho de portas/I/O temporal foi incorporado à sequência P2.2 pela PR #988. Não reabrir nem cherry-pickar a branch antiga.
- **Road Rash:** usuário reportou teste físico OK; não reabrir sem regressão. **Alinhamento de sprites SMS:** reportado corrigido; não reabrir sem regressão.
- **BIOS SMS:** implementação da PR #928 integrada; validação física com BIOS real e handoff permanece pendente.
- **Host gráfico anterior:** fora de uso ativo; inventariar consumidores antes de remover código legado.
- **Documentação:** esta reconciliação cobre os artefatos P2 relevantes; não representa leitura integral dos 285 documentos Markdown.

A rastreabilidade detalhada documento → módulo/símbolo → teste → workflow → SHA está em [SG-1000 P2 — reconciliação pós P2.4](audits/sg1000-p2-post-p24-reconciliation-2026-11-17.md).

## 6. Método, evidência e gates

Antes de alterar código: auditar main/PRs/branches/workflows; consultar documentação técnica primária; identificar consumidor e efeito observável; modelar ownership, clocks, eventos, estados e snapshots; implementar mudanças pequenas.

Gates Rust conforme workspace/target afetado:
- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Executar também os workspaces standalone e os testes focados pertinentes. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Vincular cada resultado ao SHA exato. Não transferir resultados de PR para a main sem validação pós-merge.

## 7. GitHub e integração

Toda alteração em branch/PR. Não fazer commit direto na main, force-push, reset destrutivo ou merge cego. Portar seletivamente branches divergentes. Merge somente após diff revisado, CI aplicável verde no HEAD final e autorização explícita.
