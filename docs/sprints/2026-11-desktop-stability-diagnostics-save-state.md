# Sprint — Estabilização do Desktop, Persistência e Diagnóstico

- **ID:** SPRINT-2026-11-DESKTOP
- **Prioridade:** P0 — bloqueante para a consolidação do desktop
- **Kickoff:** 2026-11-20
- **Execução planejada:** 2026-11-23 a 2026-12-04
- **Baseline do kickoff:** `main` em `137d0859463bfbf6cdbcdde0945b0eca65eee90c` (20/11/2026).
- **Baseline desta reconciliação:** `main` em `8a80fc784c3b9b2563cea248f5c3ba1b4073dfc9` (20/11/2026).
- **Escopo:** DP0–DP7, com caminho crítico P0 e entregas P1/P2 condicionadas a evidência e capacidade.
- **Arquitetura obrigatória:** Slint 1.18 + WGPU 30/Vulkan + CPAL 0.18.2; backend integralmente Rust.

## 1. Objetivo da Sprint

Entregar uma base desktop previsível e segura para uso contínuo: diagnóstico de falhas sem impacto relevante no runtime e sem escrita contínua em disco; lifecycle de ROM sem ambiguidade; Save State/Load State utilizáveis e íntegros; persistência funcional atômica; loaders documentados pelo comportamento real; e um inventário verificável das capacidades expostas pela GUI e CLI.

A Sprint não autoriza reescrita de hardware, CPU, VDP, PSG, barramentos, mappers ou Scheduler para acomodar a interface.

## 2. Requisitos vinculantes do produto

1. **Desempenho:** diagnóstico não pode bloquear o Scheduler, CPU, VDP, áudio, worker ou GUI; nenhum lock contendido, I/O, alocação por instrução ou serialização no caminho crítico.
2. **RAM-first:** ROM carregada, estado de execução e buffers de diagnóstico permanecem em RAM. VRAM física da GPU é destinada a frames/texturas/composição, não substitui RAM geral do runtime.
3. **Trace essencial sempre ativo em RAM:** ring buffers pré-alocados, limitados e com política explícita de sobrescrita/descarte e contador de perdas. Capturar apenas eventos de valor diagnóstico; não registrar indiscriminadamente cada instrução.
4. **Coleta persistente opt-in:** logs detalhados, snapshots e exportações para disco dependem de ação explícita do usuário. Nenhuma gravação contínua ou snapshot automático no encerramento.
5. **Sem alteração temporal:** instrumentação não pode modificar ordem causal, ciclos, M-cycles/T-states, WAIT states, IRQs, estado do Scheduler ou timing de áudio/vídeo.
6. **Persistência funcional:** configurações, SRAM e Save States podem ser gravados por operações explícitas/contratos definidos; usar buffer em RAM e commit atômico quando aplicável. Não gravar a cada frame ou alteração.
7. **Lifecycle claro:** Stop pausa/interrompe a execução sem descarregar a ROM. Descarregar ROM é ação separada. Erros devem chegar à UI com resultado verdadeiro.
8. **Diretórios configuráveis:** ROMs por sistema, BIOS/firmware, Save States, SRAM, logs e exportações. Separar diretório padrão, última pasta visitada e destino de exportação.
9. **Capacidades reais:** não anunciar função inexistente. Save State e Load State são prioridade máxima; cada item do inventário deve ser marcado implementado, parcial, ausente ou não suportado.

## 3. Backlog priorizado

### P0 — caminho crítico e bloqueante

#### P0.1 — DP0: diagnóstico seguro e não intrusivo
- [x] Reconciliar PR #1107, PR #1117 e issue #1103 com os requisitos RAM-first desta Sprint; ambas as PRs foram integradas na main.
- [x] Verificar os gates aplicáveis da PR #1117 e a CI pós-merge; jobs SKIPPED permanecem explicitamente fora do PASS.
- [x] Ring essencial de 2.048 eventos implementado em RAM, fixo/pré-alocado, overwrite-oldest e contador saturante.
- [x] Trace essencial, trace detalhado opt-in e exportação persistente estão separados; exportação exige consentimento e ação explícita.
- [x] F6: auditoria dos 43 callsites Slint e 15 módulos de produção com `tracing`, além de CLI/tools, examples/tests/build, sinks e persistência; limites da auditoria estática documentados.
- [x] Revogação interrompe trace detalhado/coleta e não limpa o ring essencial; filas destinadas à persistência são descartadas.
- [x] Auditoria de bootstrap não encontrou callsite fatal elegível; hook padrão preservado e nenhum reporter/arquivo automático foi instalado.
- [x] Testes de capacidade/overwrite/snapshot/revogação e quatro execuções A/B Release concluídos; benchmark físico Windows x64 permanece NOT RUN.

**Aceite:** diagnóstico essencial permanece em RAM e limitado; coleta persistente só ocorre por ação explícita; nenhum I/O no caminho de hardware; revogação efetiva; nenhuma mudança temporal; evidência de overhead em Release.

#### P0.2 — DP4: lifecycle da sessão e GUI
- [ ] Separar Stop, Reset, Descarregar ROM e Fechar aplicação.
- [ ] Stop não pode limpar ROM, SRAM ou estado de sessão por efeito colateral.
- [ ] Exibir falhas de load/save/restore/renderer/audio/input com mensagem e estado coerentes.
- [ ] Auditar cada comando e ação do menu para não apresentar sucesso fictício.
- [ ] Garantir que a GUI só invoque operações coarse-grained do runtime.

**Aceite:** testes de transição de estado e integração demonstram Stop ≠ Unload; falhas são observáveis e recuperáveis; GUI não controla ciclos/instruções.

#### P0.3 — Save State / Load State
- [ ] Inventariar o formato atual, versões, componentes incluídos e compatibilidade.
- [ ] Salvar primeiro em buffer RAM e gravar somente após solicitação explícita.
- [ ] Restaurar apenas após leitura/validação integral do arquivo em RAM.
- [ ] Validar round-trip, corrupção, truncamento, versão incompatível e restauração determinística.
- [ ] Confirmar inclusão de todo estado necessário à retomada (CPU, Scheduler, VDP, áudio, barramentos, mappers, memória e workers aplicáveis).
- [ ] Verificar que execução contínua e execução em blocos continuam equivalentes após save/load.

**Aceite:** Save/Load State funcional de ponta a ponta, sem estado parcial silencioso, com testes de round-trip e retomada determinística.

### P1 — persistência e contratos de entrada

#### P1.1 — DP2: persistência e diretórios
- [ ] Inventariar todos os dados persistidos e seus proprietários.
- [ ] Implementar preferências atômicas, recuperação de arquivo inválido e estado de janela.
- [ ] Definir diretórios configuráveis por sistema/categoria: ROM, BIOS, Save States, SRAM, logs e exports.
- [ ] Separar diretório padrão de última pasta visitada.
- [ ] Definir política SRAM dirty-state/flush agrupado e documentar a janela de perda em crash.
- [ ] Testar caminho inexistente, sem permissão, disco cheio, arquivo truncado e rename/replace.

**Aceite:** gravações funcionais atômicas quando suportadas; sem persistência diagnóstica implícita; erros reportados; preferências recuperáveis.

#### P1.2 — DP3: loaders e formatos
- [ ] Enumerar formatos realmente aceitos por cada loader e sistema.
- [ ] Validar extensão versus conteúdo, tamanho, cabeçalhos, região, mapper e limites.
- [ ] Confirmar que ROM validada é mantida em RAM durante a sessão.
- [ ] Definir mensagens para formato inválido, BIOS ausente, mapper não suportado e ROM incompatível.
- [ ] Criar matriz de fixtures válidas/inválidas por sistema.

**Aceite:** documentação e testes refletem somente formatos efetivamente aceitos; nenhum formato é anunciado por inferência de extensão.

### P2 — consolidação e prova

#### P2.1 — DP1: inventário de capacidades
- [ ] Criar matriz única de capacidades runtime/GUI/CLI, status, dependências e testes.
- [ ] Marcar implementado, parcial, ausente ou não suportado.
- [ ] Relacionar comandos de GUI às APIs reais do runtime.

#### P2.2 — DP5: desempenho e diagnóstico
- [ ] Medir baseline e cenário com trace essencial ativo em Release.
- [ ] Medir coleta detalhada habilitada e desabilitada.
- [ ] Verificar alocações, contenção, filas e drops.
- [ ] Demonstrar ausência de I/O contínuo em disco.

#### P2.3 — DP6: CLI
- [ ] Inventariar comandos e ferramentas de produto.
- [ ] Reutilizar serviços compartilhados sem iniciar coleta/gravação implícita.
- [ ] Distinguir ferramentas de desenvolvimento, testes e artefatos de produto.

#### P2.4 — DP7: fechamento
- [ ] Executar gates de formatação, Clippy, testes workspace e testes focados.
- [ ] Validar Windows x64 Release, Slint/WGPU/Vulkan e CPAL quando aplicável.
- [ ] Anexar SHA, workflows, logs de teste e resultados A/B.
- [ ] Registrar limitações restantes e bloquear qualquer declaração não demonstrada.

## 4. Sequência e checkpoints

| Janela | Foco | Saída verificável |
|---|---|---|
| 20/11 — kickoff | Congelar requisitos, revisar baseline, abrir rastreamento | Sprint documentada e backlog priorizado |
| 23–24/11 | DP4 — lifecycle da sessão | Stop/Reset/Descarregar ROM/Fechar com transições e erros verificáveis |
| 25–26/11 | Save State/Load State | Inventário, round-trip, corrupção e retomada determinística |
| 27–30/11 | DP2 | Persistência atômica e diretórios |
| 01–02/12 | DP3 + DP1 | Matriz de loaders e capacidades |
| 03/12 | DP5 + DP6 | Medições, auditoria CLI e sinks |
| 04/12 | DP7 | Relatório final por SHA e decisão de aceite |

A ordem é uma sequência-alvo, não autorização para declarar etapas concluídas por prazo. Se um gate bloquear outro, registrar BLOCKED e preservar a integridade técnica.

## 5. Regras de execução

- Trabalhar em branches curtas e PRs focadas; não fazer commits diretamente em `main`.
- Não iniciar reescrita de componentes validados.
- Toda alteração em runtime exige testes direcionados e evidência de preservação temporal.
- Nenhum benchmark sem build Release e SHA identificado.
- Não considerar check pendente, job ignorado ou teste não executado como PASS.
- Não remover estado serializado sem análise de compatibilidade/migração.
- Não persistir consentimento entre sessões, salvo decisão posterior explícita e documentada.
- Preferir mudanças pequenas, reversíveis e auditáveis.

## 6. Riscos e dependências

- **DP0:** PR #1107 e PR #1117 estão integradas; F6–F10 foram registrados como PASS na auditoria DP0. Não reabrir sem regressão reproduzível.
- **Issue #1103:** política RAM-first atualizada e implementada pelas PRs #1107/#1117; manter a separação entre ring essencial em RAM, coleta detalhada opt-in e exportação explícita.
- **Save State:** depende de inventário real do estado serializado e dos contratos de workers; não presumir cobertura completa.
- **SRAM:** persistência agrupada reduz desgaste, mas uma queda abrupta pode perder alterações ainda não descarregadas; a janela deve ser explícita.
- **Hardware timing:** qualquer regressão temporal bloqueia aceite mesmo que GUI e build funcionem.

## 7. Definição de pronto da Sprint

A Sprint só pode ser encerrada quando:
1. Todos os itens P0 estiverem PASS ou houver bloqueio externo documentado e aceito explicitamente.
2. Save State/Load State tiverem validação de round-trip, corrupção e retomada determinística.
3. DP0 demonstrar buffers limitados em RAM, coleta persistente opt-in, revogação e overhead medido.
4. Stop e Descarregar ROM forem operações independentes e testadas.
5. Persistência e loaders tiverem contratos documentados e testes.
6. CI aplicável estiver verde no SHA final; checks não executados permanecem NOT RUN.
7. DP1/DP5/DP6/DP7 tiverem inventário, evidências e limitações documentadas.

**Não é critério de pronto:** apenas compilar, abrir a janela, carregar uma ROM ou produzir uma imagem.

## 8. Registro de execução

Atualizar este bloco a cada checkpoint com PRs, commits, workflows, evidências, status e impedimentos. Não substituir histórico: acrescentar entradas datadas.

| Data | Item | SHA/PR/Workflow | Status | Evidência/impedimento |
|---|---|---|---|---|
| 2026-11-20 | Kickoff e baseline histórica | `137d0859463bfbf6cdbcdde0945b0eca65eee90c` | RECORDED | Baseline consultada no kickoff. A leitura inicial do trace como `Vec` opt-in foi supersedida pela integração DP0 registrada abaixo. |


### 8.1 Evidências iniciais — 2026-11-20

- **Sprint tracking:** issue [#1108](https://github.com/leonardo201800478/sega-emulator/issues/1108).
- **Documento original:** PR [#1109](https://github.com/leonardo201800478/sega-emulator/pull/1109), baseada no kickoff.
- **Reconciliação sobre a main atual:** PR [#1134](https://github.com/leonardo201800478/sega-emulator/pull/1134).
- **Primeiro item técnico:** issue [#1110](https://github.com/leonardo201800478/sega-emulator/issues/1110).
- **DP0 policy:** issue [#1103](https://github.com/leonardo201800478/sega-emulator/issues/1103) atualizada para RAM-first; o trace essencial não é revogado junto com coleta detalhada.
- **PR #1107:** integrada em `bc4664d9f44f6791ad8f8520a57c8744dd647dbf` (20/11/2026), estabelecendo a base de consentimento e sinks.
- **PR #1117:** integrada em `eac1d2463637a65d40621066d4ec889d2e062884` (20/11/2026), adicionando o ring essencial RAM-only e concluindo a fatia DP0.
- **Estado de código após #1117:** `SmsTemporalMachine` possui ring essencial fixo de 2.048 eventos, pré-alocado e sempre ativo em RAM, separado do trace detalhado limitado e opt-in. O ring essencial não integra Save States.
- **Limite da evidência:** os benchmarks A/B Release foram executados em CI Linux e preservaram instruções/ticks; benchmark físico Windows x64 e smoke test interativo local permanecem NOT RUN. Não declarar overhead físico zero.


### 8.2 Reconciliação DP0 na main — 2026-11-20

**Baseline consultada nesta revisão:** `main` `8a80fc784c3b9b2563cea248f5c3ba1b4073dfc9`. A main contém seis commits posteriores ao merge DP0 `eac1d2463637a65d40621066d4ec889d2e062884`; o código atual de `native/sms_core/src/temporal_machine.rs` mantém `ESSENTIAL_TRACE_CAPACITY = 2_048` e o ring essencial.

| Marco | Estado | Evidência |
|---|---|---|
| F6 — consentimento e inventário | PASS | PR #1107, merge `bc4664d9f44f6791ad8f8520a57c8744dd647dbf`; ADR 0013; inventário de 43 callsites e 15 módulos |
| F7 — desempenho | PASS | Runs A/B #31, #36, #37 e #38; quatro pares alternados; medianas -0,02% (bus-heavy) e -0,08% (relative-loop); instruções e ticks invariantes |
| F8 — regressão | PASS | CI push #9431, run [44837777090](https://github.com/leonardo201800478/sega-emulator/actions/runs/44837777090); SMS Runtime push #1716, run [44837777063](https://github.com/leonardo201800478/sega-emulator/actions/runs/44837777063); CPAL validado separadamente na CI #9418, run [44833816014](https://github.com/leonardo201800478/sega-emulator/actions/runs/44833816014) |
| F9 — integração | PASS | PR #1117 MERGED; merge `eac1d2463637a65d40621066d4ec889d2e062884`; CI pós-merge executada |
| F10 — documentação | PASS | Auditoria DP0, ADR 0013, arquitetura de observabilidade e inventário complementar atualizados |

**Limitações externas:** benchmark físico Windows x64 e smoke test interativo no hardware do usuário permanecem NOT RUN. O job do debugger egui legado e outros jobs condicionais SKIPPED não são promovidos a PASS. O fechamento interno DP0 não declara validação física nem overhead zero.

**Decisão para a Sprint:** DP0 está encerrado no escopo interno F6–F10 e deixa de ser bloqueador ativo. P0.2 (lifecycle) e P0.3 (Save State/Load State persistentes e determinísticos) continuam pendentes até evidência própria. Não inferir que snapshots in-memory existentes equivalem a persistência funcional de arquivos.
