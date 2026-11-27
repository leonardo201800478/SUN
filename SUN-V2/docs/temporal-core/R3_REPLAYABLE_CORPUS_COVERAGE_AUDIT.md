# CHRONOS R3 — auditoria de cobertura do corpus reexecutável

**Data:** 2026-11-27
**Branch:** `research/chronos-r3-disposable-prototypes`
**PR:** #1284
**HEAD validado para W06:** `a2e9f68bb6ebedabe9750eb227d2e94fe82408fe`
**Atualização W06:** fixtures W03–W06 e emitter presentes no HEAD `a2e9f68bb6ebedabe9750eb227d2e94fe82408fe`; CI #11050 PASS em Linux/Windows. Os corpora canônico e replayável foram comparados byte a byte entre plataformas; a divergência W06 foi preservada.
**Classificação:** OBSERVATION do inventário de testes e fixtures; não é closeout de R3.

## 1. Resultado executivo

CI #11050 validou o HEAD `a2e9f68bb6ebedabe9750eb227d2e94fe82408fe`; Linux e Windows executaram **169 testes PASS**. O corpus replayável contém **24 fixtures, 131 operações/cortes** e **duas divergências preservadas**: W06 e W07. Digest lógico `07551ec877a58597180470045ad24bd51444124d469b2dc2efdcc4e3d9dcba7e`; SHA-256 do JSON do corpus, idêntico nas duas plataformas: `fbb0940c3d8459763803193b67fd2d974241d66e3a50a98d560afd82a71c87ce`. O corpus canônico contém **112 fixtures/1.475 cortes**, digest `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`; SHA-256 do JSON, idêntico Linux/Windows: `424cc26a3452acde2a36d4954cef6348021ec76cd818f7462663af4fe5990a9b`. W03, W04, W05 e W06 foram validadas em ambas as plataformas.

- Corpus replayável validado no CI #11050: **24 fixtures / 131 operações e cortes**.
- W00: **1 fixture / 2 operações e cortes**, validada em Linux e Windows.
- W01: **1 fixture / 16 operações e cortes**, validada em Linux e Windows.
- W02: **1 fixture / 6 operações e cortes**, validada em Linux e Windows.
- W03: **1 fixture / 2 operações e cortes**, validada no CI #11037 em Linux e Windows.
- W04: **1 fixture / 4 operações e cortes**, validada no CI #11041 em Linux e Windows.
- W05: **1 fixture / 10 operações e cortes**, validada no CI #11047 em Linux e Windows.
- W06: **1 fixture / 3 operações e cortes**, validada no CI #11050 em Linux e Windows; divergência P-A/R2 preservada.
- W07: 1 fixture, mantendo a divergência P-A/R2.
- W08: 12 fixtures de lifecycle de transação.
- W09: 4 fixtures de snapshot/restore/replay.
- W00–W09: todos os workloads têm ao menos uma fixture replayável registrada; W00–W06 validados nos CI correspondentes. W06 não tem evidência física/externa autenticada (UNKNOWN); W07 mantém divergência BLOCKED. W08/W09 têm múltiplas fixtures.

Portanto, distinguir cobertura replayável sintética de cobertura semântica/arquitetural completa: há fixtures para W00–W09, mas isso não fecha W06 físico, W07 causalidade/comutatividade, P-A closeout nem equivalência cruzada P-A/P-D/P-E. O corpus não autoriza iniciar P-D/P-E.

## 2. Matriz de cobertura observada

| Workload | Definição no plano R3 | Evidência presente | Fixture replayável | Lacuna para o corpus |
|---|---|---|---|---|
| W00 | Avanço sem eventos/obrigações | `test_p_a_instruction_catchup.py`, incluindo `test_sparse_budgeted_advance_matches_r2`; `test_w00_replayable_fixture.py` | **Sim — 1 (2 operações/cortes; CI #11023 PASS)** | Fixture usa CPU/VDP abstratos, clock racional 3/2, duas operações advance e replay independente P-A/R2; sintética, sem evidência SMS. |
| W01 | Deadlines autônomos esparsos | `test_p_a_deadline_lifecycle.py`, publicação futura, lifecycle e dispatch; `test_w01_replayable_fixture.py` | **Sim — 1 (16 operações/cortes; CI #11027 PASS)** | Fixture sintética com quatro deadlines em instantes esparsos, publish → close watermark → dispatch → complete, preservando geração, estado integral e replay P-A/R2. |
| W02 | Deadlines densos/simultâneos | `test_p_a_deadline_lifecycle.py`, grupos simultâneos, claims e watermark; `test_w02_replayable_fixture.py` | **Sim — 1 (6 operações/cortes; CI #11032 PASS)** | Fixture cobre grupo comutativo de dois membros com claim estável por identidade e grupo UNKNOWN de dois membros sem claim/mutação nem avanço do watermark. |
| W03 | Sincronização dirigida por observação | `test_p_a_causal_sync.py`, observação, dependências e bloqueios; `test_w03_replayable_fixture.py` | **Sim — 1 (2 operações/cortes; CI #11037 PASS)** | Primeira operação resolve `vdp-status` antes de `cpu-read`; segunda retorna BLOCKED por dependência ausente, sem progresso parcial nem mutação do ledger. P-A/R2 concordam em cada corte. OBSERVATION sintética; não estabelece ordem física SMS. |
| W04 | Observações frequentes | `test_p_a_causal_sync.py`, incluindo `test_repeated_observation_trace_matches_r2_at_each_cut`; `test_w04_replayable_fixture.py` | **Sim — 1 (4 operações/cortes; CI #11041 PASS)** | Quatro chamadas independentes de `synchronize`, uma por corte racional; cada corte registra CPU/VDP/PSG, progresso completo e ledgers cumulativos. Replay independente P-A/R2. OBSERVATION sintética; não estabelece cadence ou clock físico SMS. |
| W05 | Corridas de cancelamento | `test_p_a_deadline_lifecycle.py`; `test_w05_replayable_fixture.py` | **Sim — 1 (10 operações/cortes; CI #11047 PASS)** | Três deadlines D=10/20/30: C=8<D é admitido; close W10 bloqueia até arbitrar; após arbitragem W10 fecha. C=D=20 sem ordem explícita retorna INDETERMINATE sem mutação; C=32>D=30 retorna LATE_INPUT sem mutação. W30 fecha e reivindica os dois deadlines remanescentes. Estados completos incluem pendências de cancelamento. OBSERVATION sintética; sem causalidade física SMS. |
| W06 | Simultaneidade UNKNOWN / comparabilidade de phase basis | `test_p_a_deadline_lifecycle.py`; `test_w06_replayable_fixture.py` | **Sim — 2 (6 operações/cortes; CI #11072 PASS; divergência negativa e controle positivo)** | Dois controles replayáveis: (1) bases sintéticas distintas, mesmo rótulo de evidência, preservando P-A INDETERMINATE versus R2 Accepted; (2) base sintética compartilhada e rótulos por evento distintos, com concordância P-A/R2 em Accepted. Total de 6 operações/cortes; divergência negativa preservada. IDs e hashes são sintéticos; evidência física/externa permanece UNKNOWN. |
| W07 | Microsteps | `test_w07_replayable_fixture.py` e `test_w07_pairwise_matrix.py` | **Sim — 1** | A relação B/C permanece não demonstrada; preservar a divergência e o primeiro corte divergente. |
| W08 | Lifecycle de transação | `test_w08_replayable_fixture.py` | **Sim — 12** | Cobertura registrada para retry, sample/commit, WAIT, TW, cancelamento, falha e ledger parcial. Confirmar se workloads adicionais aplicáveis exigem novos cenários. |
| W09 | Snapshot/restore/replay | `test_w09_replayable_fixture.py` | **Sim — 4** | Fixtures cobrem replay de entradas, gerações/ledgers, acknowledge e round-trip; continuam sintéticas e não demonstram hardware SMS. |

## 3. Fontes de código verificadas

- `SUN-V2/benchmarks/r3/emit_p_a_trace_corpus.py`: a lista `TEST_MODULES` inclui os módulos P-A, o harness e os testes replayáveis W00/W01/W02/W03/W07/W08/W09; W04–W06 ainda não possuem módulos replayáveis.
- `SUN-V2/benchmarks/r3/replayable_fixture_corpus.py`: schema `sun.chronos.r3.replayable-fixture.v0.1`; guarda estado inicial, operações, estados completos por corte, digests por lado e primeira divergência.
- `test_p_a_instruction_catchup.py`: 24 testes unitários/diferenciais, sem registro próprio de fixture replayável; W00 possui módulo declarativo separado.
- `test_p_a_deadline_lifecycle.py`: 31 testes unitários/diferenciais; `test_w01_replayable_fixture.py` e `test_w02_replayable_fixture.py` registram sequências declarativas próprias.
- `test_p_a_causal_sync.py`: 14 testes unitários/diferenciais; `test_w03_replayable_fixture.py` e `test_w04_replayable_fixture.py` registram sequências declarativas próprias, validadas nos CI #11037/#11041.
- `test_p_a_deadline_lifecycle.py`: 31 testes unitários/diferenciais; `test_w05_replayable_fixture.py` e `test_w06_replayable_fixture.py` registram sequências declarativas próprias, validadas nos CI #11047/#11050.
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
6. **W05 — VALIDADA no CI #11047:** C<D admitido, fechamento do watermark bloqueado até arbitragem e depois aceito; C=D sem ordem explícita retorna INDETERMINATE sem mutação; C>D retorna LATE_INPUT sem mutação. 10 operações/cortes e estado integral incluindo pendências de cancelamento. 168 testes PASS Linux/Windows; corpus replayável 23/128, digest `f8fcc00da65167ff7088e8ce02513a3ff0eb297774124d2a07fbb1e4d03b4fa4`; SHA-256 JSON idêntico `8c2eaa7b3ed3a1b60f52c14464e9e802568e1273732b0f10cfb42d7568e8f46a`.
7. **W06 — dois controles replayáveis validados no CI #11072:** caso negativo com bases sintéticas distintas preserva P-A INDETERMINATE versus R2 Accepted; controle positivo com base compartilhada e rótulos por evento distintos exige concordância Accepted. Total de 6 operações/cortes. CI #11072 validou 183/183 testes R3 Linux/Windows e corpus replayável de 25 fixtures/134 operações-cortes, digest `2dcf09eeaf6ffe03e6bcf1cfda087aeedeb865d351b6ac081a0dc12ff28bc28f`; equivalência byte a byte Linux/Windows PASS. A divergência demonstra diferença entre modelos, não qual ordem física é correta. Evidência física/externa continua UNKNOWN.

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
- [x] W04: fixture validada no CI #11041 (4 operações/cortes).
- [x] W05: fixture validada no CI #11047 (10 operações/cortes).
- [x] W06: dois controles validados no CI #11072 (6 operações/cortes); divergência negativa preservada, controle positivo concordante e artefatos canônicos/replayáveis byte a byte idênticos. Evidência física/externa UNKNOWN.
- [x] W07/W08/W09 permanecem registrados; divergência W07 e demais resultados conhecidos são preservados.
- [x] Em cada fixture replayável, P-A e R2 são reexecutados de forma independente a partir das mesmas entradas.
- [x] O corpus agregado registra contagem por workload, operações/cortes e divergências.
- [x] Linux e Windows produzem artefatos byte a byte idênticos.
- [x] Os resultados não são apresentados como evidência física SMS nem como desempenho Rust.

## 6. Disposição dos gates

**R3 permanece EM ANDAMENTO.** Esta auditoria identifica a lacuna de cobertura; não a fecha. W06 permanece UNKNOWN como evidência física; W07 permanece BLOCKED pela relação B/C não demonstrada. P-A não está pronto para closeout, P-D/P-E permanecem NOT RUN/BLOCKED, R4/R5 estão pendentes e nenhum código de produção CHRONOS/V2 é autorizado antes de R5.


## 7. Aditivo — W07 pairwise-cycle e atualização W06 (27/11/2026)

- W06 contém dois controles replayáveis (negativo e positivo), totalizando 6 operações/cortes, validados no CI #11072; corpus replayável agregado: 25 fixtures / 134 operações-cortes.
- O protótipo isolado `w07_pairwise_cycle.py` possui 17 testes após a ampliação da matriz negativa validada no CI #11079. Esses testes não são uma segunda fixture W07 e não alteram a contagem do corpus replayável.
- A matriz negativa cobre formas inválidas das coleções, ledger `resolved`, colisões entre `pending`/`resolved`/`future`, descritores de relação, endpoints e preservação do estado original.
- W07 físico continua BLOCKED; W06 físico continua UNKNOWN. P-A não está fechado; P-D/P-E permanecem NOT RUN/BLOCKED; R4/R5 pendentes.
