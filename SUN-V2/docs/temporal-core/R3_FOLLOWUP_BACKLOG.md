# CHRONOS R3 — Backlog de fechamento pós-PR #1284

**Baseline integrada:** PR #1284, squash merge `98cd9e258ae463e32397a119d4d4e6a007346c22`
**HEAD validado da baseline:** `1ffa7c74f5d9116c58a8889694989c8e00dec993`
**CI baseline:** [#11226 — SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46157842567)
**Estado:** R3 IN PROGRESS. Este backlog não aprova R3, R4 ou R5 e não autoriza código de produção CHRONOS/V2 antes de R5.

## Objetivo da continuação

A PR #1284 integrou uma baseline de pesquisa grande (70 arquivos; aproximadamente 23 mil linhas de adições e 523 commits no histórico da branch). Para não repetir esse acoplamento, este documento funciona como índice de pendências. Cada frente de implementação deve ser entregue em PR pequena e independente, com escopo, arquivos, evidências e CI próprios. Esta PR de acompanhamento não deve acumular todas as implementações.

## Evidência de baseline — CI #11226

- R3 descartável: 272/272 PASS Linux e 272/272 PASS Windows.
- R2 independente: 256/256 PASS; mutation checks PASS.
- R2 Python Windows: 256/256 PASS.
- Emissor P-A: 186/186 PASS por OS.
- Corpus canônico: 119 fixtures / 1.483 cuts; digest `ab5e146eda37e8292159cc5440beca615c5b511383e3807b24adbacbb490056b`.
- Corpus replayable: 30 fixtures / 143 operações-cuts; digest `ed039c1985d1a0d1dee518a33bfcbbefd3713368e040e113f4b8d856c255e215`.
- Equivalência byte a byte dos corpora Linux/Windows: PASS.
- Higiene documental, validação de workflows e supervisor de qualidade Rust: PASS.
- Jobs específicos de Rust workspace, Z80, VDP, PSG, YM2413, CPAL, SDL3, Slint/WGPU, scheduler stress e frame pacing SMS: SKIPPED; não contar como PASS.

## Backlog priorizado

### FUP-01 — Completar diferenciais W05 e P-D/P-A

**Estado:** PARCIAL — os testes W05 atuais cobrem participantes, safe horizons, watermark, obrigações, cancelamento e microsteps, mas o diferencial P-D/P-A ainda não espelha sistematicamente toda a matriz R2 de deadlines/cancelamentos.

Trabalho:
- Mapear cada cenário R2-T06/R2-T15 aplicável a W05 contra P-A e P-D.
- Acrescentar comparação de estado canônico completo após cada operação, não somente códigos de retorno.
- Cobrir admissão e cancelamento em fronteiras C/D/W/P_d, cancelamentos pré-admitidos, atraso do dispatcher, arbitragem pendente, substituição de geração e fencing de geração antiga.
- Cobrir validação estrutural, conversões racionais, conjuntos de participantes incompletos, owner mismatch, watermark divergente e não mutação.
- Preservar os UNKNOWN sem inventar precedência física.
- Medir W05 separadamente depois de equivalência funcional.

Critério de saída: matriz rastreável R2 → P-A → P-D, testes diferenciais Linux/Windows e artefatos de estado equivalentes para todos os cenários selecionados.

### FUP-02 — Completar diferenciais P-E/P-A (W03/W04) e limites W06/W07

**Estado:** PARCIAL.

Trabalho:
- Expandir P-E/P-A para a matriz causal completa de W03 e W04: dependências, fechamento transitivo, participantes, safe horizons, observações, transições simultâneas, relações e atomicidade.
- Comparar estado completo P-E/P-A, inclusive índices derivados reconstruídos e estado observado.
- Manter W06 fase física UNKNOWN e W07 relação B/C UNKNOWN/BLOCKED; preservar as duas divergências P-A/R2 como exceções sintéticas explícitas.
- Não promover phase basis sintética ou metadata unilateral de commutativity a FACT de hardware.

Critério de saída: matriz de casos diferenciais completa e artefatos Linux/Windows byte-idênticos, com divergências UNKNOWN identificadas.

### FUP-03 — W08: transações, recursos e identidade/generation

**Estado:** PARCIAL.

Fatos/limites conhecidos:
- P-D/P-E carregam um `TransactionBook` imutável e os testes cobrem retries, identidade divergente, WAIT externo/fixture/dependency, sample/commit exactly-once, duração-base, automatic/external TW, cancelamento, falha parcial, recuperação e conclusão.
- O ledger de transações não possui limite de capacidade explícito.
- A reutilização da mesma identity com descriptor/generation divergente é rejeitada; substituição da geração sob a mesma identity não está implementada.
- O R2 possui um `ResourceLedger` genérico de capacidade finita, separado do `TransactionBook`; não inferir que capacidade genérica significa limite de quantidade de transações.

Trabalho:
- Mapear a capacidade finita de R2-T15 para a reserva de recursos sem inventar unidade, custo por transação ou política de eviction.
- Decidir/documentar se limite de memória/quantidade do TransactionBook é orçamento de implementação do protótipo ou semântica normativa; manter separado do recurso de hardware.
- Comparar a política de identidade/generation com as regras normativas R1/R2; testar replacement atômico e fencing somente onde a identidade estável e o contrato permitem.
- Analisar acoplamento causal da transação com agenda P-D, observed transitions P-E e pipeline lógico de barramento. Não afirmar fronteiras físicas sem fonte.
- Testar atomicidade, falhas parciais, retry, recovery e preservação de ledgers em toda a cadeia.

Critério de saída: orçamento de recurso explícito e determinístico, testes diferenciais contra o oráculo R2 sem alterar o oráculo, e semântica de generation sustentada pelo contrato.

### FUP-04 — W09: snapshot/restore/replay contra a referência

**Estado:** PARCIAL — snapshot candidato-local existe, equivalência integral contra P-A/R2 ainda NÃO ESTABELECIDA.

Trabalho:
- Criar fixtures compartilhadas para os mesmos cortes e sequências de operação P-A/P-D/P-E/R2.
- Comparar payload semântico normalizado e estado pós-restore/replay, incluindo heap/eventos, constraints, cancelamentos, observed set, transações, WAIT/TW, sampled values, committed effects, partial failures e recovery.
- Testar snapshots antes/depois de cada marcador, entradas aceitas após snapshot, cursor, duplicidade/gaps de replay e falha atômica no meio do replay.
- Preservar UnsupportedSnapshot para cortes não serializáveis/UNKNOWN conforme R1; não produzir artefato parcial.
- Não serializar CPU/IRQ/VDP/PSG ausentes dos candidatos nem chamar snapshot candidato-local de snapshot da máquina SMS.
- Manter R2 imutável.

Critério de saída: equivalência P-A/P-D/P-E nos cortes suportados, replay determinístico e rejeições de snapshots não suportados com ausência de mutação/artefato.

### FUP-05 — Matriz de performance completa e comparável

**Estado:** PARCIAL.

- A matriz atual mede 36 pontos por OS, mas cobre P-D W01/W02 e P-E W03/W04.
- W05, W08 e W09 não são cronometrados separadamente.
- CPU-time Windows continua SKIPPED_COARSE_RESOLUTION.
- Não usar os números Python como evidência de desempenho Rust, frame pacing ou 60 FPS.

Trabalho:
- Só medir operações depois de equivalência funcional.
- Adicionar W05, W08 e W09 como matrizes separadas, incluindo custo de snapshot/restore/replay e recursos.
- Manter construção de fixtures/serialização fora da região cronometrada quando apropriado.
- Reportar p50/p95/p99, pior ponto, custo CPU quando resolução permitir, e outputs equivalentes.
- Não misturar throughput headless com frame pacing.

Critério de saída: artefatos pareados reproduzíveis e relatório de limitações por plataforma.

### FUP-06 — Revisão adversarial independente / R4

**Estado:** PENDENTE.

A tentativa anterior de `@codex review` foi bloqueada pelo limite de uso do serviço; não existe aprovação formal independente registrada para o HEAD integrado.

Escopo mínimo:
- W05: admissão, cancelamento, watermark, eventos obsoletos, dispatcher atrasado, gerações e microsteps.
- W08: identidade/generation, capacity, retries, WAIT, exatamente-uma-vez, efeitos parciais, recovery e causalidade end-to-end.
- W09: integridade, compatibilidade, snapshots de cortes parciais, replay, cursor e atomicidade.
- W06/W07: tratamento explícito das exceções UNKNOWN, sem inferência física.
- Performance, recursos, complexidade e riscos de estado inválido/forjado.
- Verificar rastreabilidade e limites do experimento.

Critério de saída: revisão independente formal no SHA exato, achados classificados e todos os blockers corrigidos ou explicitamente aceitos pelo responsável.

### FUP-07 — R5: decisão arquitetural e plano de integração

**Estado:** BLOQUEADO até FUP-01..FUP-06 e R4.

- Consolidar ADRs, trade-offs e seleção P-D/P-E ou alternativa.
- Revisar evidência, riscos, determinismo, performance e custo de integração.
- Aprovar formalmente R5 antes de qualquer código de produção CHRONOS/V2.
- Após R5, seguir fatia vertical SMS conforme governança do projeto.

## Ordem de execução e estratégia de PRs

1. PR pequena FUP-01: matriz diferencial W05 P-D/P-A.
2. PR pequena FUP-02: completar W03/W04 P-E/P-A e formalizar limites W06/W07.
3. PR pequena FUP-03: W08 recursos/generation/transações.
4. PR pequena FUP-04: W09 equivalência snapshot/restore/replay.
5. PR pequena FUP-05: benchmark pareado apenas dos caminhos já validados.
6. PR FUP-06: closeout de revisão adversarial R4 e correções residuais.
7. PR FUP-07: ADRs e proposta de aprovação R5.

Cada PR deve partir da `main`, conter apenas sua frente, referenciar os IDs FUP, rodar CI proporcional e registrar SHA exato. Evitar acumular frentes independentes numa única branch. O presente PR é apenas o índice de acompanhamento e não deve incorporar implementação de produção.

## Regras invariáveis

- R1 v0.3.18 e R2 aprovado permanecem imutáveis.
- W06 e W07 continuam UNKNOWN/BLOCKED até evidência técnica apropriada.
- Não inventar fase física, WAIT interno, microtiming ou relação B/C.
- Prototipagem permanece descartável em `SUN-V2/benchmarks/r3/`.
- Não copiar V1 nem criar dependência de crates V1.
- Nenhum código de produção CHRONOS/V2 antes de R5.
- PASS, FAIL, BLOCKED, NOT RUN e SKIPPED permanecem estados distintos.


## Estado de execução — 28/11/2026

- **PR #1289:** índice de acompanhamento aberto em Draft; CI #11228 passou (higiene documental e classificação de impacto).
- **FUP-01:** primeira fatia isolada na [PR #1290](https://github.com/leonardo201800478/SUN/pull/1290), READY FOR REVIEW. Implementa substituição atômica de geração em P-D, fencing da geração antiga, rejeições atômicas de fronteiras/contexts inválidos e arbitragem de cancelamento C<D antes do fechamento de W. **CI #11236 PASS** no SHA `a97faac312755b507ba90307edaaf7c2278eabc7`: R3 275/275 Linux e Windows; R2 256/256 Linux e Windows; emissor P-A 186/186 por OS; corpus equivalence PASS. FUP-01 permanece PARCIAL até concluir a matriz diferencial W05 restante.
- **FUP-02:** primeira fatia isolada na [PR #1291](https://github.com/leonardo201800478/SUN/pull/1291), READY FOR REVIEW. Adiciona quatro diferenciais P-E/P-A W03/W04 (empate não comutativo, empate comutativo canônico, participante desconhecido e observação retroativa). **CI #11238 PASS** no SHA `24d9e898a4ec97df80c8ffcb3acf9c4c67a571ce`: R3 276/276 Linux e Windows; R2 256/256 Linux e Windows; corpora equivalentes. FUP-02 permanece PARCIAL.
- **FUP-03:** primeira fatia isolada na [PR #1292](https://github.com/leonardo201800478/SUN/pull/1292), READY FOR REVIEW. Acrescenta comparação P-A/P-D/P-E/R2 demonstrando que generation faz parte do descriptor estável: reutilizar a mesma identity com generation divergente é rejeitado como InvalidInput sem mutação. **CI #11242 PASS** no SHA `3b1980379d0bbfb23d27e9ba83b7488ad104c25f`: R3 273/273 Linux e Windows; R2 256/256 Linux e Windows; corpus equivalence PASS. A política de capacidade do TransactionBook e o acoplamento end-to-end continuam pendentes.
- FUP-04 a FUP-07 permanecem pendentes e serão entregues em PRs próprias, sem misturar implementações independentes nesta PR de índice.
