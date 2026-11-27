# CHRONOS R3 — auditoria de cobertura do corpus reexecutável

**Data:** 2026-11-27
**Branch:** `research/chronos-r3-disposable-prototypes`
**PR:** #1284
**HEAD validado para W04:** `5eac391350f82cdbaa6b15e333142eb4f9e4b12e`
**Atualização W04:** fixtures W03/W04 e emitter presentes no HEAD `5eac391350f82cdbaa6b15e333142eb4f9e4b12e`; CI #11041 PASS em Linux/Windows. Os corpora canônico e replayável foram comparados byte a byte entre plataformas.
**Classificação:** OBSERVATION do inventário de testes e fixtures; não é closeout de R3.

## 1. Resultado executivo

CI #11041 validou o HEAD `5eac391350f82cdbaa6b15e333142eb4f9e4b12e`; Linux e Windows executaram **167 testes PASS**. O corpus replayável contém **22 fixtures, 118 operações/cortes** e uma divergência intencional W07. Digest lógico `613d154e92160ac758c740da82d5338e964822ffbc9881e15e6f143ccf18f8e4`; SHA-256 do JSON do corpus, idêntico nas duas plataformas: `1987d6f46df4e6919a147065636962fb5a9533d6189b3e7987b8fb7efd2e1aeb`. O corpus canônico contém **112 fixtures/1.475 cortes**, digest `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`; SHA-256 do JSON, idêntico Linux/Windows: `175258c86464ceb1b9ce3d877bf97b413974d60ee6d67e42aeb09dd6faf148b1`. W03 e W04 foram aprovadas em ambas as plataformas.

- Corpus replayável validado no CI #11041: **22 fixtures / 118 operações e cortes**.
- W00: **1 fixture / 2 operações e cortes**, validada em Linux e Windows.
- W01: **1 fixture / 16 operações e cortes**, validada em Linux e Windows.
- W02: **1 fixture / 6 operações e cortes**, validada em Linux e Windows.
- W03: **1 fixture / 2 operações e cortes**, validada no CI #11037 em Linux e Windows.
- W04: **1 fixture / 4 operações e cortes**, validada no CI #11041 em Linux e Windows.
- W07: 1 fixture, mantendo a divergência P-A/R2.
- W08: 12 fixtures de lifecycle de transação.
- W09: 4 fixtures de snapshot/restore/replay.
- W00–W04: existem fixtures declarativas e replayáveis; W00/W01/W02/W03/W04 validadas em CI. W05–W06 continuam sem fixtures replayáveis.

Portanto, não classificar a cobertura de teste unitário de W00–W06 como cobertura replayável. O corpus permanece parcial e ainda não atende à migração completa dos workloads aplicáveis W00–W09.

## 2. Matriz de cobertura observada

| Workload | Definição no plano R3 | Evidência presente | Fixture replayável | Lacuna para o corpus |
|---|---|---|---|---|
| W00 | Avanço sem eventos/obrigações | `test_p_a_instruction_catchup.py`, incluindo `test_sparse_budgeted_advance_matches_r2`; `test_w00_replayable_fixture.py` | **Sim — 1 (2 operações/cortes; CI #11023 PASS)** | Fixture usa CPU/VDP abstratos, clock racional 3/2, duas operações advance e replay independente P-A/R2; sintética, sem evidência SMS. |
| W01 | Deadlines autônomos esparsos | `test_p_a_deadline_lifecycle.py`, publicação futura, lifecycle e dispatch; `test_w01_replayable_fixture.py` | **Sim — 1 (16 operações/cortes; CI #11027 PASS)** | Fixture sintética com quatro deadlines em instantes esparsos, publish → close watermark → dispatch → complete, preservando geração, estado integral e replay P-A/R2. |
| W02 | Deadlines densos/simultâneos | `test_p_a_deadline_lifecycle.py`, grupos simultâneos, claims e watermark; `test_w02_replayable_fixture.py` | **Sim — 1 (6 operações/cortes; CI #11032 PASS)** | Fixture cobre grupo comutativo de dois membros com claim estável por identidade e grupo UNKNOWN de dois membros sem claim/mutação nem avanço do watermark. |
| W03 | Sincronização dirigida por observação | `test_p_a_causal_sync.py`, observação, dependências e bloqueios; `test_w03_replayable_fixture.py` | **Sim — 1 (2 operações/cortes; CI #11037 PASS)** | Primeira operação resolve `vdp-status` antes de `cpu-read`; segunda retorna BLOCKED por dependência ausente, sem progresso parcial nem mutação do ledger. P-A/R2 concordam em cada corte. OBSERVATION sintética; não estabelece ordem física SMS. |
| W04 | Observações frequentes | `test_p_a_causal_sync.py`, incluindo `test_repeated_observation_trace_matches_r2_at_each_cut`; `test_w04_replayable_fixture.py` | **Sim — 1 (4 operações/cortes; CI #11041 PASS)** | Quatro chamadas independentes de `synchronize`, uma por corte racional; cada corte registra CPU/VDP/PSG, progresso completo e ledgers cumulativos. Replay independente P-A/R2. OBSERVATION sintética; não estabelece cadence ou clock físico SMS. |
| W05 | Corridas de cancelamento | `test_p_a_deadline_lifecycle.py`, C<D, C=D, C>D, arbitragem e fechamento de watermark | **Não** | Capturar publish/cancel/arbitrate/close/dispatch com fronteiras exatas e estados sem mutação nos resultados indeterminados. |
| W06 | Simultaneidade UNKNOWN | `test_p_a_deadline_lifecycle.py`, grupos UNKNOWN e comparabilidade estrutural de fase | **Não** | Fixture sintética pode demonstrar somente a decisão UNKNOWN e a ausência de mutação; não pode autenticar fase física nem substituir o registro externo. |
| W07 | Microsteps | `test_w07_replayable_fixture.py` e `test_w07_pairwise_matrix.py` | **Sim — 1** | A relação B/C permanece não demonstrada; preservar a divergência e o primeiro corte divergente. |
| W08 | Lifecycle de transação | `test_w08_replayable_fixture.py` | **Sim — 12** | Cobertura registrada para retry, sample/commit, WAIT, TW, cancelamento, falha e ledger parcial. Confirmar se workloads adicionais aplicáveis exigem novos cenários. |
| W09 | Snapshot/restore/replay | `test_w09_replayable_fixture.py` | **Sim — 4** | Fixtures cobrem replay de entradas, gerações/ledgers, acknowledge e round-trip; continuam sintéticas e não demonstram hardware SMS. |

## 3. Fontes de código verificadas

- `SUN-V2/benchmarks/r3/emit_p_a_trace_corpus.py`: a lista `TEST_MODULES` inclui os módulos P-A, o harness e os testes replayáveis W00/W01/W02/W03/W07/W08/W09; W04–W06 ainda não possuem módulos replayáveis.
- `SUN-V2/benchmarks/r3/replayable_fixture_corpus.py`: schema `sun.chronos.r3.replayable-fixture.v0.1`; guarda estado inicial, operações, estados completos por corte, digests por lado e primeira divergência.
- `test_p_a_instruction_catchup.py`: 24 testes unitários/diferenciais, sem registro próprio de fixture replayável; W00 possui módulo declarativo separado.
- `test_p_a_deadline_lifecycle.py`: 31 testes unitários/diferenciais; `test_w01_replayable_fixture.py` e `test_w02_replayable_fixture.py` registram sequências declarativas próprias.
- `test_p_a_causal_sync.py`: 14 testes unitários/diferenciais; `test_w03_replayable_fixture.py` e `test_w04_replayable_fixture.py` registram sequências declarativas próprias, validadas nos CI #11037/#11041.
- `test_p_a_microsteps.py`: 11 testes, com matriz e divergência tratadas separadamente.
- `test_w07_replayable_fixture.py`: 1 fixture registrada.
- `test_w08_replayable_fixture.py`: 12 fixtures registradas.
- `test_w09_replayable_fixture.py`: 4 fixtures registradas.
- `test_w00_replayable_fixture.py`: 1 fixture registrada no emitter; CI #11023 PASS, com replay P-A/R2 e equivalência Linux/Windows.

## 4. Plano incremental de preenchimento

A ordem abaixo reduz acoplamento entre os adapters e mantém o corpus verificável em fatias:

1. **W00 — VALIDADA:** fixture mínima de avanço sem obrigações, CPU/VDP abstratos, clock racional 3/2 e duas operações. CI #11023 confirmou replay e equivalência Linux/Windows.
2. **W01 — VALIDADA:** quatro deadlines esparsos em instantes 10/30/50/80; cada deadline é publicado, reivindicado por watermark fechado, despachado e concluído. 16 operações/cortes, estado integral por corte e replay independente P-A/R2; CI #11027 PASS Linux/Windows.
3. **W02 — VALIDADA:** grupo comutativo de dois membros publicado em ordem inversa à identidade e grupo UNKNOWN de dois membros. O primeiro é reivindicado em ordem estável; o segundo preserva estado, status e watermark sem mutação. 6 operações/cortes; CI #11032 PASS Linux/Windows.
4. **W03 — VALIDADA no CI #11037:** uma observação com dependência cross-device declarada e uma negativa por dependência ausente, com bloqueio atômico. 2 operações/cortes; replay independente P-A/R2. 166 testes PASS em Linux e Windows; corpus replayável 21/114, digest `4dfd24e30bd0871826dcf393b83d017b2e2afe7dee39c1a76ed4c0b38d2566d8`; SHA-256 do JSON idêntico `51d3a9db3dd084c5f15c1a321aeadb95a05eda766e7b00db55768c696f6c62bc`.
5. **W04 — VALIDADA no CI #11041:** quatro chamadas de observação, quatro estados completos por corte, replay independente P-A/R2. 4 operações/cortes. 167 testes PASS em Linux e Windows; corpus replayável 22/118, digest `613d154e92160ac758c740da82d5338e964822ffbc9881e15e6f143ccf18f8e4`; SHA-256 do JSON idêntico `1987d6f46df4e6919a147065636962fb5a9533d6189b3e7987b8fb7efd2e1aeb`.
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

- [x] W00: fixture e replay validados em CI #11023.
- [x] W01: fixture e replay validados em CI #11027 (16 operações/cortes).
- [x] W02: fixture e replay validados em CI #11032 (6 operações/cortes).
- [x] W03: fixture validada no CI #11037 (2 operações/cortes).
- [x] W04: fixture validada no CI #11041 (4 operações/cortes); 167 testes PASS em Linux/Windows e artefatos canônicos/replayáveis byte a byte idênticos. W05–W06 ainda sem fixtures replayáveis aplicáveis ou justificativa de não aplicabilidade/duplicação.
- [ ] W07/W08/W09 permanecem registrados e suas divergências conhecidas são preservadas.
- [ ] Em cada fixture, P-A e R2 são reexecutados de forma independente a partir das mesmas entradas.
- [ ] O corpus agregado registra contagem por workload, operações/cortes e divergências.
- [ ] Linux e Windows produzem artefatos byte a byte idênticos.
- [ ] Os resultados não são apresentados como evidência física SMS nem como desempenho Rust.

## 6. Disposição dos gates

**R3 permanece EM ANDAMENTO.** Esta auditoria identifica a lacuna de cobertura; não a fecha. W06 permanece UNKNOWN como evidência física; W07 permanece BLOCKED pela relação B/C não demonstrada. P-A não está pronto para closeout, P-D/P-E permanecem NOT RUN/BLOCKED, R4/R5 estão pendentes e nenhum código de produção CHRONOS/V2 é autorizado antes de R5.
