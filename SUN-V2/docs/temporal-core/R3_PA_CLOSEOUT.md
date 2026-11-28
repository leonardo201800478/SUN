# SUN V2 — decisão de closeout da baseline P-A (R3)

**Data da decisão:** 2026-11-28
**Aprovação:** responsável pelo projeto SUN V2, decisão explícita registrada na conversa de projeto e no comentário da PR #1284.
**Classificação:** decisão de escopo para experimento R3; não altera contrato normativo, oráculo aprovado ou arquitetura de produção.
**Baseline P-A congelada:** `466f789bc9bfc73c7d6388f31b3f9142b48acf78`
**CI de validação:** [#11122 / run 46105849156](https://github.com/leonardo201800478/SUN/actions/runs/46105849156) — SUCCESS.

## 1. Decisão

O responsável pelo projeto escolheu o caminho de preservar W06 e W07 como UNKNOWN, aceitar as divergências observadas entre P-A e R2 como exceções explícitas e continuar a comparação experimental com P-D/P-E.

A decisão é deliberadamente limitada:

1. R1 v0.3.18 aprovado não é alterado.
2. O oráculo R2 aprovado no merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0` não é alterado.
3. Não se declara que P-A ou R2 reproduzem a relação física real do SMS nos casos W06/W07.
4. As divergências permanecem nos fixtures e nos corpora replayáveis, com primeira operação divergente, estados e resultados de ambos os lados.
5. P-A é aceito como **baseline experimental congelada para comparação**, não como arquitetura selecionada nem implementação apta a produção.
6. P-D/P-E ficam autorizados a iniciar somente com o corpus e os critérios desta decisão.

## 2. Exceção W06 — base de fase não comparável

**Classificação física:** UNKNOWN.

A fixture negativa W06 usa dois deadlines simultâneos, não comutativos, com `phase_order` distinto, rótulo sintético de evidência compartilhado e `APhaseBasis` sintéticas distintas.

- P-A não considera as bases comparáveis e retorna `IndeterminateCausalOrder`; preserva os deadlines em `Scheduled`, não reivindica efeitos e não fecha o watermark.
- R2 não possui o campo estruturado `phase_basis`; considera suficientes os campos `phase_order`/ `phase_evidence` presentes e a unicidade da ordem, e retorna `Accepted`.
- A fixture é uma divergência intencional e não integra os cortes diferenciais que exigem igualdade.
- O controle positivo com base sintética comum permanece obrigatório: P-A e R2 devem concordar em `Accepted`.

**Limite aceito:** o identificador/hash do registro de validação da fixture não resolve nem autentica um registro externo real. A igualdade estrutural de uma base sintética não é evidência física. Não atribuir fase, época ou precedência a qualquer perfil SMS a partir desta fixture.

**Regra para P-D/P-E:** sem base de fase comum, válida e verificável, manter a mesma indeterminação de P-A e não reivindicar deadline. O controle positivo sintético deve continuar passando. A exceção de divergência com R2 deve ser reproduzida e reportada, não mascarada.

## 3. Exceção W07 — relação par-a-par B/C desconhecida

**Classificação da relação B/C:** UNKNOWN.

A fixture replayável inicia A e B no microstep 0, com relação A/B comutativa; A emite C no microstep 1. Não existe relação declarada entre B e C.

- P-A exige relação explícita por par e retorna `IndeterminateCausalOrder` ao tentar processar C, sem alterar o estado.
- R2 aprovado não representa a relação par-a-par desta fixture e aceita a progressão para C, retornando `Advanced`.
- A primeira divergência é preservada no corte da operação 1. Os dois lados continuam sendo reexecutados independentemente.
- A matriz isolada de relações par-a-par continua cobrindo B→C, C→B, comutatividade comprovada e UNKNOWN; ela adjudica premissas fornecidas, não descobre nem prova a relação física.

**Limite aceito:** nem o número do microstep, nem a relação A/B, nem flags unárias de comutatividade estabelecem a relação B/C. Não escolher B<C ou C<B por ordem de inserção ou conveniência.

**Regra para P-D/P-E:** uma relação ausente deve ser tratada como UNKNOWN. Para efeitos potencialmente não comutativos, retornar `IndeterminateCausalOrder` sem mutação. Só causalidade explícita ou comutatividade par-a-par comprovada pode permitir progressão. Os casos sintéticos B→C, C→B e COMMUTATIVE devem continuar como controles separados.

## 4. Evidência que sustenta o congelamento P-A

No HEAD congelado `466f789bc9bfc73c7d6388f31b3f9142b48acf78`, o CI #11122 reportou:

| Item | Resultado |
|---|---:|
| Suíte R3 descartável | 206/206 Linux e 206/206 Windows |
| Emissor e testes do corpus | 184 testes por OS |
| Oráculo R2 independente | 256/256 por OS, mutation checks PASS |
| Matrizes sintéticas P-A | 351 pontos por OS |
| Corpus canônico | 119 fixtures / 1.483 cortes |
| Corpus replayável | 25 fixtures / 134 operações-cortes |
| Jobs CI | 9 SUCCESS / 14 SKIPPED |

Digests semânticos:

- Corpus canônico: `ab5e146eda37e8292159cc5440beca615c5b511383e3807b24adbacbb490056b`.
- Corpus replayável: `6189653bb3aa18fc5805f7dd1ae9708ea31b349d03da7ea9fcc906350cbcfbb6`.

O CI confirmou JSON canônico e replayável byte a byte idênticos entre Linux e Windows. A presença de fixtures cobre cada workload W00–W09, mas não significa cobertura exaustiva de todas as combinações de parâmetros. O corpus replayável compara P-A com R2; não é evidência de comparação com P-D/P-E.

Os 351 pontos são benchmarks sintéticos Python (5 warm-ups e 30 amostras medidas por ponto). CPU-time Windows permanece `SKIPPED_COARSE_RESOLUTION`; usar wall-time apenas como observação por runner, sem inferir desempenho Rust ou comparar runners diferentes como A/B controlado. Os 14 jobs SKIPPED não contam como PASS.

## 5. Critérios de aceitação do closeout P-A

P-A é fechado **somente como baseline experimental**, sob estas condições cumulativas:

- W00–W09 têm fixtures declarativas e presença no corpus replayável.
- Os cortes diferenciais não excepcionados coincidem com R2 e o corpus canônico não contém divergência.
- W06 e W07 permanecem exatamente identificados como exceções; resultados, estados e primeira divergência são preservados e reproduzíveis.
- Controles positivos W06 e a matriz pairwise W07 continuam testados.
- Repetição/replay produz os mesmos digests; artefatos Linux/Windows permanecem byte a byte equivalentes.
- O baseline, parâmetros, digests e limitações ficam congelados no SHA acima.
- Nenhuma mudança posterior em P-A, fixtures congeladas ou adapters pode ser incorporada silenciosamente: correção exige nova revisão do baseline, novos digests e CI.

Este closeout não significa equivalência integral P-A/R2. A equivalência é afirmada apenas para os casos diferenciais positivos. W06/W07 são exceções aceitas e rastreadas.

## 6. Contrato de entrada para P-D e P-E

P-D e P-E podem iniciar a partir desta decisão, usando o mesmo corpus congelado, as mesmas entradas, parâmetros e sequência de operações. Não podem criar workloads mais favoráveis nem derivar o workload do próprio algoritmo.

### Requisitos funcionais comuns

1. Usar aritmética temporal exata; floats não podem ser autoridade temporal.
2. Preservar avanço limitado, safe horizon, participantes completos, admissão, deadlines/generation fencing, cancelamento, watermark, dependências, microsteps finitos, transações sample/commit exactly-once, snapshot/restore/replay e erros/atomicidade.
3. Preservar UNKNOWN sem ordem por inserção, endereço, hash, thread ou escalonamento do host.
4. Produzir trace canônico comparável por operação: status/erro, progresso por domínio, resíduos/fase, watermark, deadlines/lifecycle/generation, dependências, transações, ledger de efeitos, cursor e estado pendente.
5. Reproduzir o resultado de P-A nos casos positivos e preservar as exceções W06/W07 descritas acima. Reportar separadamente a divergência histórica com R2.
6. Não importar a lógica de transição de P-A ou R2 para o caminho medido. Compartilhar apenas schema, fixtures declarativas, normalização/canonicalização neutra e harness de comparação.
7. Não modificar R1, o pacote R2 aprovado ou o baseline P-A congelado durante a implementação de P-D/P-E.

### P-D — agenda discreta central

Implementar agenda central de eventos/deadlines com chave temporal/causal, cancelamento e geração. A agenda não se torna dona do estado dos dispositivos. Medir custo de inserção, cancelamento, invalidação, seleção do próximo evento, empates, dispatch e carga sparse/dense.

### P-E — sincronização sob demanda

Implementar fechamento causal apenas dos participantes declarados para a observação/commit solicitados, sem agenda global de progresso contínuo. Medir custo de fan-in, profundidade causal, repetição de observações e workloads access-driven.

### Comparação e descarte

- Rodar P-A, P-D e P-E sobre a mesma fixture congelada e comparar cada corte, primeiro ponto divergente e digest.
- Executar 5 warm-ups e 30 amostras medidas por ponto; reportar p50/p95/p99/máximo, wall-time, CPU-time quando resolução válida, contadores reais de trabalho, alocações e memória.
- Não comparar números de runners diferentes como A/B controlado; não tratar CPU-time coarse como válido.
- Um candidato que falhe semântica/trace é descartado independentemente de throughput.
- P-F só poderá ser considerado se P-A/P-D/P-E isolados apresentarem um gap mensurável que a composição resolva sem duplicar lifecycle.
- Registrar custos e limitações como OBSERVATION de protótipos Python, nunca como desempenho Rust ou hardware.

## 7. Limites e gates posteriores

- O fechamento P-A aqui não fecha R3.
- P-D/P-E precisam de CI próprio, corpus congelado, equivalência de traces, resultados de benchmark e revisão adversarial.
- R4 deve revisar a metodologia, as duas exceções, os três candidatos, os riscos, o custo e a suficiência das evidências.
- R5 deve aprovar explicitamente a arquitetura, ADRs e plano de integração.
- Nenhum código de produção CHRONOS/V2 é autorizado antes de R5.
- Não há validação de timing físico SMS, Z80, VDP, PSG, YM2413, frame pacing ou 60 FPS neste closeout.

## 8. P-D/P-E — primeiro corte validado, arquitetura ainda não selecionada

CI #11145 (run 46107673712) validou 241/241 testes R3 em Linux/Windows, 184 testes de corpus por OS, R2 256/256 por OS, corpora equivalentes e 36 pontos pareados P-D/P-E. Os detalhes e limitações estão em R3_PD_PE_PROGRESS.md.

- P-D vs P-A: equivalência funcional nos pontos W01/W02 medidos; razão mediana dos p50 6,44× Linux e 7,08× Windows; pior ponto 37,78×/39,17×.
- P-E vs P-A: equivalência funcional nos pontos W03/W04 medidos; razão mediana 1,31× Linux e 1,38× Windows; pior ponto 12,83×/11,23×, fan-in 16/observações frequentes.
- CPU-time Windows permanece SKIPPED_COARSE_RESOLUTION. Resultados são OBSERVATION sintética Python, não desempenho Rust/hardware.
- P-D ainda não inclui A(E), safe horizons, watermark e C<D/C=D/C>D (W05). P-E ainda não integra transações W08 nem snapshots/replay W09. Nenhum candidato está completo em W00–W09.
- Disposição: não selecionar P-D/P-E ainda. Otimizar estruturas/índices e repetir matriz; manter P-A congelado e W06/W07 UNKNOWN. R3/R4/R5 continuam pendentes; produção proibida antes de R5.