# Sprint — Estabilização do Desktop, Persistência e Diagnóstico

- **ID:** SPRINT-2026-11-DESKTOP
- **Prioridade:** P0 — bloqueante para a consolidação do desktop
- **Kickoff:** 2026-11-20
- **Execução planejada:** 2026-11-23 a 2026-12-04
- **Baseline consultada:** `main` em `137d0859463bfbf6cdbcdde0945b0eca65eee90c`
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
- [ ] Reconciliar PR #1107, PRs relacionadas e issue #1103 com os requisitos vigentes desta Sprint.
- [ ] Verificar CI integral do HEAD relevante; não mesclar com checks pendentes/falhos.
- [ ] Formalizar ring buffer de falhas sempre ativo em RAM, limitado, pré-alocado e com drop/overwrite accounting.
- [ ] Separar trace essencial de falhas, coleta detalhada de sessão e persistência/exportação em disco.
- [ ] Auditar/classificar individualmente os 43 callsites e todos os sinks identificados (tracing/log, stdout/stderr, arquivos, snapshots, traces, métricas e panic/crash).
- [ ] Garantir que revogação descarte fila pendente e dados de sessão persistentes, sem afetar o buffer essencial em RAM definido pelo contrato.
- [ ] Definir fronteira de crash de bootstrap; preservar o hook padrão de panic e evitar payload/backtrace/dados privados automáticos.
- [ ] Testar fila cheia, descarte, revogação/reconsentimento, ausência de I/O sem opt-in e overhead A/B.

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
| 23–24/11 | DP0 + CI/PR #1107 | Contrato de diagnóstico e correções bloqueantes |
| 25–26/11 | DP4 + Save/Load State | Lifecycle e round-trip demonstrados |
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

- **PR #1107:** CI e revisão precisam ser reavaliados no HEAD atual antes de merge.
- **Issue #1103:** contém uma política de diagnóstico anterior que precisa ser reconciliada com a política RAM-first desta Sprint; não aplicar literalmente critérios que desabilitem o trace essencial em RAM.
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
| 2026-11-20 | Kickoff e baseline | `137d0859463bfbf6cdbcdde0945b0eca65eee90c` | IN PROGRESS | Baseline main consultada; PR #1107 e issue #1103 identificadas para reconciliação |
