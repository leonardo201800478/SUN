# Roadmap mestre — Sega Emulator

**Revisão:** 2026-11-20
**Baseline consultada:** `main` `be883792025de3db7eaf4e7b8243293ea90b7e06`.
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
| P2 | SG-1000 | Máquina temporal + worker Rust, Z80 compartilhado, MachineScheduler, TMS9918A, PSG, I/O, snapshots, FrameMailbox e áudio host integrados |
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

## 4. Trilha transversal — produto desktop (R5)

A ordem normativa de produto, os requisitos de UX, persistência, catálogo de mídia, CLI e política de diagnóstico estão detalhados em [Roadmap de produto desktop](desktop-product-roadmap.md). Esta trilha é obrigatória para consolidar R5 e deve seguir os gates DP-0 a DP-7. A proibição de coleta diagnóstica sem consentimento explícito aplica-se imediatamente, inclusive a builds Release. O desenvolvimento desta trilha não altera a ordem de plataformas nem autoriza mudanças no runtime temporal.

## 5. Trilhas transversais adicionais

### Z80 compartilhado

Estado: `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e o escopo praticável de SMS/SG-1000. As famílias E4C/E5/E6/E7 e as extensões de F-021/F-022/F-023/F-028 possuem testes internos, diferenciais e integrações registradas. Permanecem limites: CMOS/clones fora do perfil, WAIT físico, DMA, validação elétrica e cobertura universal não demonstrados. ZEXDOC/ZEXALL e outras suítes são complementares futuras. Não reabrir E7 sem bug/regressão ou evidência concreta.

### Debugger

Implementado parcialmente e em evolução. Melhorar captura, retenção/exportação, snapshots CPU/VDP, janela de diagnóstico e usabilidade em trilha separada. Não é bloqueio para fechar SMS/SG-1000.

### BIOS

O boot por BIOS convencional de 8 KiB está em implementação candidata na PR #928. Validação física com firmware real, handoff e primeiro código do cartucho permanece pendente. Não declarar boot físico validado.

### RetroAchievements — integração transversal planejada

**Estado:** PLANEJADO / NÃO IMPLEMENTADO. Esta trilha não altera a ordem das plataformas nem faz parte da Sprint P0 de estabilização desktop.

A integração deve começar somente quando SMS e SG-1000 tiverem runtime de frame estável, lifecycle e contratos de Save State suficientemente consolidados. O trabalho deve ser incremental, sem introduzir callbacks por instrução, leituras concorrentes inconsistentes ou trabalho de rede no caminho crítico do MachineScheduler.

| Marco | Escopo | Critério de saída |
|---|---|---|
| RA-0 — decisão arquitetural | ADR sobre consumo do runtime oficial `rcheevos` (C) via FFI segura ou alternativa compatível com a diretriz Rust-only; licença, build, ownership, threading, User-Agent e política de credenciais | Decisão explícita aprovada; wrapper Rust isolado; nenhuma dependência de UI ou hardware em C |
| RA-1 — identidade e memória | Hash MD5 e identificação SMS (console ID 11) / SG-1000 (console ID 33); mapa de memória legível pelo serviço de achievements, incluindo ROM/banks, RAM e regiões mapeadas relevantes | Hash reproduzível e memory map validado por console, sem expor ponteiros instáveis ou memória fora do contrato |
| RA-2 — cliente e rede | Autenticação, armazenamento seguro de token, HTTP assíncrono, identificação/carregamento do jogo e tratamento de erros/eventos | Login/logout, falhas e respostas de servidor testados; nenhuma chamada de rede bloqueia Scheduler ou GUI |
| RA-3 — integração por frame | Atualização do runtime de achievements uma vez por frame emulado concluído, inclusive quando o frame não é renderizado; leitura consistente do estado daquele frame | Mesmo resultado em execução contínua, fast-forward e renderização descartada; pausa não avança achievements; leitura thread-safe e sem corrida |
| RA-4 — experiência de conquistas | Lista, progresso, notificações, leaderboards e Rich Presence na camada Slint | Eventos do serviço chegam à GUI por mensagens coarse-grained; apresentação não interfere no timing |
| RA-5 — Hardcore e estados | Regras de Hardcore para Load State, cheats, rewind, slow motion/frame advance e debugger; Save State permitido; serialização/restauração do estado de sessão RA quando suportado | Política aplicada centralmente; Load State bloqueado em Hardcore sem corromper sessão; Save State permitido; transições de modo e reset testados |
| RA-6 — aceite por sistema | Validação de SMS e SG-1000, títulos identificados, memory maps, achievements/leaderboards e User-Agent reconhecido | Evidências por título e SHA; validação oficial do cliente quando exigida pelo serviço; limitações documentadas |

**Invariantes de integração:**
- O ponto de atualização é a conclusão do frame emulado no runtime/worker, nunca o frame renderizado nem cada instrução. Pausado, usar apenas o mecanismo de idle apropriado, sem avançar o jogo.
- A leitura de memória pode ocorrer em thread de serviço; fornecer snapshot consistente ou contrato de leitura thread-safe, sem acessar estado mutável do Scheduler de forma concorrente.
- Não executar HTTP, espera de rede, UI callback ou alocação não limitada no caminho crítico temporal.
- Hardcore é política de sessão: Save State permanece permitido; Load State e demais ações proibidas devem ser bloqueadas pela camada de comandos do runtime, não apenas ocultadas na GUI.
- Credenciais/API tokens não podem ser versionados nem registrados em logs; usar armazenamento seguro do sistema operacional.
- A decisão de FFI deve respeitar a arquitetura Rust-only do produto e ser registrada em ADR antes de qualquer dependência C ser introduzida.

Referências primárias: [RetroAchievements Integration Docs](https://docs.retroachievements.org/developer-docs/), [rcheevos](https://github.com/RetroAchievements/rcheevos), [rc_client integration](https://github-wiki-see.page/m/RetroAchievements/rcheevos/wiki/rc_client-integration), [Game identification](https://docs.retroachievements.org/developer-docs/game-identification.html).

## 6. Estado operacional da baseline

- Main SHA da baseline desta revisão: `be883792025de3db7eaf4e7b8243293ea90b7e06`.
- CI push pós-merge da baseline: run [#43962737265](https://github.com/leonardo201800478/sega-emulator/actions/runs/43962737265). Change impact, Documentation patch hygiene e Rust quality/unit PASS; workspace, Z80, Scheduler, SMS1 VDP, CPAL, SDL3, Slint/WGPU/Vulkan e demais jobs de runtime foram SKIPPED. Não há validação de runtime pós-merge nesta execução.
- Árvore atual: 285 documentos Markdown sob `docs/`; inventário estrutural anterior não equivale à leitura e reconciliação semântica integral dos 285 arquivos.
- Road Rash: usuário reportou teste físico OK; retirar da lista de defeitos abertos, sem inventar SHA/relatório.
- Alinhamento de sprites SMS: usuário reportou corrigido; não manter como defeito aberto sem regressão.
- BIOS boot: validação física pendente.
- Host gráfico anterior: não está em uso ativo.
- SG-1000 P2.2: integrado na main pela PR #988, merge commit `be883792025de3db7eaf4e7b8243293ea90b7e06`. `Sg1000Machine` usa `Z80MicroSequencer` compartilhado, M1/WAIT por eventos do MachineScheduler, TMS9918A, PSG, I/O temporal e snapshots compostos. CI pós-merge #8459 (run `44090848768`) concluiu com sucesso os gates aplicáveis: Z80 focused, SMS1 VDP, input, SDL3, workspace, Scheduler stress e Rust quality/unit. O caminho atomic antigo está limitado a `cfg(test)` como oracle funcional. SG-1000 P2.3: worker Rust dedicado com FrameMailbox, áudio compartilhado, input snapshots e save states em desenvolvimento na branch `feat/sg1000-runtime-worker-p23-20261117`. P2.4 ainda deve integrar seleção SG-1000 na GUI Slint, carregamento de ROM e handoff WGPU/CPAL. Não declarar o SG-1000 disponível no desktop antes desses gates.

## 7. Método, evidência e gates

Antes de alterar código: auditar main/PRs/branches/workflows; consultar documentação técnica primária; identificar consumidor e efeito observável; modelar ownership, clocks, eventos, estados e snapshots; implementar mudanças pequenas.

Gates Rust conforme workspace/target afetado:
- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Executar também os workspaces standalone e os testes focados pertinentes. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Vincular cada resultado ao SHA exato. Não transferir resultados de PR para a main sem validação pós-merge.

## 8. GitHub e integração

Toda alteração em branch/PR. Não fazer commit direto na main, force-push, reset destrutivo ou merge cego. Portar seletivamente branches divergentes. Merge somente após diff revisado, CI aplicável verde no HEAD final e autorização explícita.
