# CHRONOS R3 — auditoria de cobertura do corpus reexecutável

**Data:** 2026-11-26
**Branch:** `research/chronos-r3-disposable-prototypes`
**PR:** #1284
**HEAD de referência do inventário inicial:** `ee96d3c013d25a60757355ecf4cf600f70b8c6ca`  
**Atualização W00:** fixture adicionada em `4605dd2af122bca0b7d7f5c34cc6c509cfc6385b`; registro no emitter em `57ee467988327a50296d32d74cde52dc1031e396` (CI #11019 pendente).
**Classificação:** OBSERVATION do inventário de testes e fixtures; não é closeout de R3.

## 1. Resultado executivo

O harness declarativo de replay já está implementado e exercitado. O inventário inicial cobria W07, W08 e W09; a primeira expansão W00 foi adicionada como fixture declarativa e aguarda validação CI no HEAD atualizado.

- Corpus replayável antes da expansão W00: **17 fixtures / 88 operações e cortes**.  
- W00: **1 fixture adicional / 2 operações e cortes**, implementada; equivalência e geração multiplataforma aguardam CI #11019.
- W07: 1 fixture, mantendo a divergência P-A/R2.
- W08: 12 fixtures de lifecycle de transação.
- W09: 4 fixtures de snapshot/restore/replay.
- W00–W06: existem testes P-A e comparações unitárias/diferenciais, mas **não há fixtures registradas no corpus replayável**.

Portanto, não classificar a cobertura de teste unitário de W00–W06 como cobertura replayável. O corpus continua parcial e não atende ainda à migração completa dos workloads aplicáveis W00–W09.

## 2. Matriz de cobertura observada

| Workload | Definição no plano R3 | Evidência presente | Fixture replayável | Lacuna para o corpus |
|---|---|---|---|---|
| W00 | Avanço sem eventos/obrigações | `test_p_a_instruction_catchup.py`, incluindo `test_sparse_budgeted_advance_matches_r2`; `test_w00_replayable_fixture.py` | **Sim — 1 (2 operações/cortes; CI pendente)** | Fixture usa CPU/VDP abstratos, clock racional 3/2, duas operações advance e replay independente P-A/R2; sintética, sem evidência SMS. |
| W01 | Deadlines autônomos esparsos | `test_p_a_deadline_lifecycle.py`, publicação futura, lifecycle e dispatch | **Não** | Sequência declarativa publish → advance/watermark → claim/dispatch → complete, incluindo geração e status. |
| W02 | Deadlines densos/simultâneos | `test_p_a_deadline_lifecycle.py`, grupos simultâneos, claims e watermark | **Não** | Registrar grupos, empates, ordem/identidade, resultado e estado integral; preservar UNKNOWN sem desempate inventado. |
| W03 | Sincronização dirigida por observação | `test_p_a_causal_sync.py`, observação, dependências e bloqueios | **Não** | Capturar operação de observação, conjunto de participantes/dependências e transições aplicadas por corte. |
| W04 | Observações frequentes | `test_p_a_causal_sync.py`, incluindo `test_repeated_observation_trace_matches_r2_at_each_cut` | **Não** | Registrar sequência repetida de observações e o estado completo em cada corte, sem agregar chamadas. |
| W05 | Corridas de cancelamento | `test_p_a_deadline_lifecycle.py`, C<D, C=D, C>D, arbitragem e fechamento de watermark | **Não** | Capturar publish/cancel/arbitrate/close/dispatch com fronteiras exatas e estados sem mutação nos resultados indeterminados. |
| W06 | Simultaneidade UNKNOWN | `test_p_a_deadline_lifecycle.py`, grupos UNKNOWN e comparabilidade estrutural de fase | **Não** | Fixture sintética pode demonstrar somente a decisão UNKNOWN e a ausência de mutação; não pode autenticar fase física nem substituir o registro externo. |
| W07 | Microsteps | `test_w07_replayable_fixture.py` e `test_w07_pairwise_matrix.py` | **Sim — 1** | A relação B/C permanece não demonstrada; preservar a divergência e o primeiro corte divergente. |
| W08 | Lifecycle de transação | `test_w08_replayable_fixture.py` | **Sim — 12** | Cobertura registrada para retry, sample/commit, WAIT, TW, cancelamento, falha e ledger parcial. Confirmar se workloads adicionais aplicáveis exigem novos cenários. |
| W09 | Snapshot/restore/replay | `test_w09_replayable_fixture.py` | **Sim — 4** | Fixtures cobrem replay de entradas, gerações/ledgers, acknowledge e round-trip; continuam sintéticas e não demonstram hardware SMS. |

## 3. Fontes de código verificadas

- `SUN-V2/benchmarks/r3/emit_p_a_trace_corpus.py`: a lista `TEST_MODULES` inclui os módulos P-A, o harness e os testes replayáveis W07/W08/W09; não inclui módulos replayáveis W00–W06.
- `SUN-V2/benchmarks/r3/replayable_fixture_corpus.py`: schema `sun.chronos.r3.replayable-fixture.v0.1`; guarda estado inicial, operações, estados completos por corte, digests por lado e primeira divergência.
- `test_p_a_instruction_catchup.py`: 24 testes unitários/diferenciais, sem registro de fixture replayável.
- `test_p_a_deadline_lifecycle.py`: 31 testes unitários/diferenciais, sem registro de fixture replayável.
- `test_p_a_causal_sync.py`: 14 testes unitários/diferenciais, sem registro de fixture replayável.
- `test_p_a_microsteps.py`: 11 testes, com matriz e divergência tratadas separadamente.
- `test_w07_replayable_fixture.py`: 1 fixture registrada.
- `test_w08_replayable_fixture.py`: 12 fixtures registradas.
- `test_w09_replayable_fixture.py`: 4 fixtures registradas.  
- `test_w00_replayable_fixture.py`: 1 fixture registrada no emitter; CI #11019 ainda precisa confirmar execução e equivalência multiplataforma.

## 4. Plano incremental de preenchimento

A ordem abaixo reduz acoplamento entre os adapters e mantém o corpus verificável em fatias:

1. **W00 — IMPLEMENTADA, CI PENDENTE:** fixture mínima de avanço sem obrigações, com CPU/VDP abstratos, clock racional 3/2, duas operações e estado integral por corte. O emitter inclui o módulo; CI #11019 deve validar replay e equivalência Linux/Windows.
2. **W01:** lifecycle de deadline esparso, com geração explícita e transição terminal.
3. **W02:** grupo denso/simultâneo. Incluir ao menos um caso comutativo e um UNKNOWN; o segundo deve preservar status e estado sem mutação.
4. **W03:** uma observação com dependência cross-domain declarada e uma negativa por dependência ausente.
5. **W04:** sequência de observações repetidas, mantendo um corte por chamada.
6. **W05:** corrida C<D e C=D sem relação demonstrada; registrar arbitragem, watermark e ausência de mutação.
7. **W06:** fixture sintética de decisão UNKNOWN apenas se ela agregar cobertura reexecutável distinta dos casos W02/W05. Não criar fixture com metadados sintéticos que se pareçam com evidência física validada.

Cada fixture deve:
- usar entradas/argumentos declarativos e sequência contígua;
- serializar o estado inicial e o estado observável completo em cada operação;
- executar os dois lados independentemente (P-A e R2) com tipos próprios;
- registrar status/código, primeiro corte divergente e digests de cada estado;
- replayar os dois lados a partir do estado inicial e verificar todos os cortes;
- identificar proveniência, classificação OBSERVATION e limites do cenário.

Não converter automaticamente cada teste unitário em fixture: agrupar cenários somente quando a sequência declarativa for legível e reproduzir uma pergunta do workload. Evitar duplicar os 12 cenários W08 ou as quatro fixtures W09 já registradas.

## 5. Critérios de conclusão do corpus

- [ ] W00: fixture implementada; CI #11019 pendente. W01–W06 ainda sem fixtures replayáveis aplicáveis ou justificativa de não aplicabilidade/duplicação.
- [ ] W07/W08/W09 permanecem registrados e suas divergências conhecidas são preservadas.
- [ ] Em cada fixture, P-A e R2 são reexecutados de forma independente a partir das mesmas entradas.
- [ ] O corpus agregado registra contagem por workload, operações/cortes e divergências.
- [ ] Linux e Windows produzem artefatos byte a byte idênticos.
- [ ] Os resultados não são apresentados como evidência física SMS nem como desempenho Rust.

## 6. Disposição dos gates

**R3 permanece EM ANDAMENTO.** Esta auditoria identifica a lacuna de cobertura; não a fecha. W06 permanece UNKNOWN como evidência física; W07 permanece BLOCKED pela relação B/C não demonstrada. P-A não está pronto para closeout, P-D/P-E permanecem NOT RUN/BLOCKED, R4/R5 estão pendentes e nenhum código de produção CHRONOS/V2 é autorizado antes de R5.
