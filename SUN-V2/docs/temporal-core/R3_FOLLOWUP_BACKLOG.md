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


### FUP-04 — auditoria de projeção semântica R2 (28/11/2026)

A inspeção dos modelos de referência e dos candidatos identificou que "equivalência integral P-D/P-E/R2" não pode significar igualdade estrutural de snapshots. O \`MachineState\` do R2 contém \`domains\`, \`watermark\`, \`deadlines\`, \`transactions\`, \`interrupts\` e \`input_cursor\`; os candidatos não representam todos esses domínios com a mesma semântica.

**R2 — referência normativa imutável**
- \`snapshot_reference.py\`: valida e captura o \`MachineState\` completo; rejeita cortes de INT acknowledge cuja presença/commit do efeito é UNKNOWN como \`UnsupportedSnapshot\`, sem artefato; restore valida o artefato completo e a compatibilidade de profile/model_version antes de expor estado restaurado.
- \`replay_reference.py\`: modela log de inputs aceitos, sequência, applied IDs, pending obligations, recalculate-required e virtual effects. Isso não é o mesmo formato nem a mesma semântica do sufixo de operações dos candidatos.

**P-D — projeção compartilhada parcial**
- O \`AgendaState\` contém eventos/deadlines, generation, watermark, heap, constraints, cancelamentos pendentes e \`TransactionBook\`.
- Há uma projeção potencialmente comparável com R2 para watermark e parte do ledger de deadlines (identity/generation/tempo/estado/efeitos), além de subconjunto de campos das transações (identity/generation/state/progress/wait/sample/committed effects).
- A projeção deve ser explícita e tipada. Não converter automaticamente heap, obligations, constraints ou \`ready_marker\` em campos R2; não sintetizar \`pending_effects\`, domínio/residual ou IRQ/ack ausentes. Diferenças sem contraparte devem ser relatadas como "não representadas", não como equivalência.

**P-E — sem projeção de máquina integral**
- \`SyncState\` contém domínios com \`progress/safe_horizon\`, transições, constraints, observed set, índices derivados e \`TransactionBook\`.
- \`safe_horizon\`, dependências e relações par-a-par não têm equivalentes diretos no \`MachineState\` R2 (\`DomainState\` usa \`progress_common/residual\`; deadlines têm outro contrato). Não mapear horizon para residual, nem transição para deadline.
- Para P-E, a comparação com R2 fica limitada a metadados do envelope, cursor fornecido pelo harness, integridade/atomicidade de capture/restore e campos transacionais que tenham correspondência semântica demonstrada. A semântica W03/W04 permanece comparada P-E/P-A.

**Limites transversais**
- P-D/P-E não modelam \`InterruptRecord\`; nenhum teste candidato pode alegar equivalência de IRQ/NMI/ack ou suporte a cortes que R2 classifica \`UnsupportedSnapshot\`.
- \`r3_candidate_snapshots.py\` captura o estado completo do candidato e do \`TransactionBook\`, não uma máquina SMS completa. O cursor é metadado do harness; não é prova de que o log de replay dos candidatos seja idêntico ao log de inputs aceitos do R2.
- R2 permanece imutável. Não preencher lacunas com campos sintéticos apenas para fazer os objetos parecerem iguais.

**Plano executável FUP-04**
1. Manter testes P-D/P-E de round-trip/replay candidato-local, tamper, incompatibilidade, sequência não contígua e falha atômica.
2. Criar adaptador de projeção P-D→R2 somente para campos com correspondência demonstrada; comparar códigos, projeção e não mutação nos mesmos cortes compartilhados.
3. Para P-E, testar separadamente o envelope e os invariantes candidatos; não exigir igualdade de \`SyncState\` com \`MachineState\`.
4. Adicionar controles R2 de snapshot suportado e \`UnsupportedSnapshot\` para INT-ack UNKNOWN, preservando ausência de artefato e estado ativo.
5. Reportar explicitamente campos cobertos, não representados e não comparáveis. FUP-04 só fecha com matriz e CI Linux/Windows no SHA exato; não basta round-trip candidato-local.


### Atualização de execução — FUP-04 (28/11/2026, 16:47 UTC)

- **PR #1294:** [projeção parcial P-D→R2](https://github.com/leonardo201800478/SUN/pull/1294), Draft, sem merge.
- **CI #11256:** PASS no SHA `14865f1e29fd8b6c34881e87125e505f6962e6d3`. R2 independente Linux, R2 Python Windows, protótipos R3 Linux/Windows, equivalência dos corpora Linux/Windows e supervisor de qualidade Rust concluíram SUCCESS. Jobs de produção não aplicáveis ficaram SKIPPED.
- A projeção P-D→R2 cobre somente watermark e identidade/generation/tempo/estado dos deadlines. `CancelPending` é UnsupportedProjection. A passagem pelo `capture/restore` R2 valida apenas o envelope compartilhado projetado; campos P-D ausentes não são tratados como equivalentes a campos R2 vazios.
- **P-E:** adicionada auditoria explícita do envelope candidato em `pe_r2_snapshot_envelope_audit.py`. Compara somente profile/model_version; mantém candidate e input_cursor como metadados de candidato/harness; declara não mapeados safe_horizon, transições, constraints, observed/indexes, TransactionBook não demonstrado e IRQ/ack. Não cria projeção de `SyncState` para `MachineState`.
- HEAD ampliado: `b21732fdea95e39cfb90460b5aad3a035495013c`; CI #11258 em execução no momento deste registro: https://github.com/leonardo201800478/SUN/actions/runs/46166975186.
- **FUP-04 continua PARCIAL.** O round-trip/replay candidato-local P-D/P-E já existe; os testes novos tornam explícitas as fronteiras de comparação R2, mas ainda falta concluir a matriz W09 compartilhada de cortes/efeitos/transações e validar todos os controles de atomicidade/replay contra o escopo realmente comparável. Não declarar equivalência integral nem fechar FUP-04 com base apenas em round-trip.


### Atualização de execução — FUP-04 (28/11/2026, CI #11266)

- **PR #1294:** [projeção parcial P-D→R2](https://github.com/leonardo201800478/SUN/pull/1294), permanece Draft, aberta e sem merge.
- **HEAD validado:** `4cbf6b4e2fa845935cd63c960f12d2866aac5ac3`.
- **CI #11266:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46167800593), associado ao HEAD exato acima.
- Protótipos descartáveis R3: Linux SUCCESS e Windows SUCCESS.
- Referência independente R2: Linux SUCCESS e Python Windows SUCCESS.
- Corpus canônico e replayável: comparação byte a byte Linux/Windows SUCCESS.
- Classificação de impacto e supervisor de qualidade/regressão Rust: SUCCESS.
- Jobs de produção Rust workspace, Z80, VDP, PSG, YM2413, CPAL, SDL3, Slint/WGPU, scheduler stress e frame pacing SMS: SKIPPED; não são evidência de validação desses subsistemas.
- A projeção P-D→R2 normaliza watermark/coordenadas em `Fraction`, mapeia explicitamente cinco estados de evento e rejeita `CancelPending` como `UnsupportedProjection`. Testes metamórficos confirmam invariância à permutação da tupla de eventos e à alteração de `effect_id`, que não faz parte da projeção.
- A auditoria P-E→R2 compara profile quando compatível e mantém namespaces de model_version distintos; input_cursor permanece metadado do harness. Campos sem correspondência semântica continuam explicitamente não mapeados.
- **FUP-04 permanece PARCIAL.** O CI comprova os testes existentes nos dois sistemas operacionais, mas não estabelece equivalência integral de snapshot/restore/replay, nem resolve IRQ/ack, log de inputs aceitos, transações não mapeadas ou os campos P-E sem contraparte.
- **Próxima fatia:** completar a matriz executável W09 apenas nos campos com correspondência semântica demonstrável, incluindo atomicidade e replay nos cortes compartilhados; manter R2 imutável e preservar as rejeições UnsupportedSnapshot/UnsupportedProjection.


### Atualização W09 — CI #11269 e nova matriz de atomicidade (28/11/2026)

- **PR #1294 / FUP-04:** CI #11269 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46168258450), SHA `0e810baa6527d93f5fff868fab363eeac23546a0`.
- R3 descartável Linux/Windows, R2 independente Linux, R2 Python Windows, equivalência byte a byte dos corpora Linux/Windows e Rust quality/unit regression: SUCCESS.
- Jobs de produção Z80, VDP, PSG, YM2413, CPAL, SDL3, Slint/WGPU, scheduler e frame pacing: SKIPPED; sem inferência de validação de hardware.
- Novos testes no HEAD seguinte `f88a5ea7a24a302d15be2124997a362ef7d11789`: restore R2 de artefato forjado contendo INT-ack UNKNOWN rejeita atomicamente e preserva estado ativo; round-trip da projeção P-D→R2 verifica watermark e coordenadas compartilhadas de cada deadline, sem declarar equivalência de ledgers de efeitos.
- CI #11270 para `f88a5ea7a24a302d15be2124997a362ef7d11789`: em execução no momento deste registro; resultado não antecipado.
- FUP-04 permanece PARCIAL até concluir a matriz W09 de todos os campos compartilhados com correspondência semântica demonstrável e documentar campos sem contraparte. R2 permanece imutável.



### Atualização W09 — projeção transacional parcial e CI #11270/#11275 (28/11/2026)

- **CI #11270:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46168442378) no SHA `f88a5ea7a24a302d15be2124997a362ef7d11789`. R3 Linux/Windows, R2 independente Linux, R2 Python Windows, equivalência byte a byte dos corpora canônico/replayável e Rust quality/unit regression: SUCCESS.
- A inspeção dos contratos confirmou correspondência parcial entre o `r3_transaction_core.TransactionBook` (compartilhado por P-D/P-E) e o `R2 TransactionRecord`: identity, generation, state, progress_common, ready_marker→next_marker, wait_source, sampled e failure→error.
- Adicionado em PR #1294 o adaptador `transaction_snapshot_projection.py` e sua matriz de testes: todos os sete estados de ciclo de vida, round-trip pelo capture/restore R2, origem P-D e P-E e rejeição de TransactionBook inválido.
- Não são projetados: descritores de operação/origem/destino/payload, duração-base, budgets de automatic/external TW, schedule de markers, base_consumed, ledgers de TW, IDs de commit marker como se fossem device effect IDs, nem committed/pending effect ledgers R2. Nenhuma equivalência de IRQ/ack, domínios ou replay é inferida.
- **CI #11274:** FAIL no SHA intermediário `bbdbecb46737145c7bfc8d2be68d054413382782`, por uma asserção de teste que procurava um fragmento textual diferente do nome efetivo do campo não representado. O adaptador e os testes funcionais de projeção/round-trip passaram; a asserção de cobertura textual foi corrigida no commit seguinte. A falha não foi ocultada nem contada como PASS.
- **CI #11275:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46169029106) no SHA exato `7a2ac8c409a711d226c0da793b77d255ddad1a80`. R3 descartável: 291 testes Linux e 291 Windows; R2 independente Linux: 256 testes; R2 Python Windows: 256 testes; corpora canônico e replayável byte-identical Linux/Windows; supervisor Rust quality/unit regression: SUCCESS.
- Jobs de produção SMS/Z80/VDP/PSG/YM2413/CPAL/SDL3/Slint/WGPU/scheduler permaneceram SKIPPED; este CI não valida esses subsistemas.
- **FUP-04 permanece PARCIAL.** A matriz agora cobre a projeção de deadlines P-D e a projeção parcial de transações compartilhadas P-D/P-E. Permanecem não comparáveis o log de inputs aceitos R2 versus o sufixo de operações dos candidatos, os estados P-E de sincronização versus MachineState, IRQ/ack e ledgers de efeitos sem contraparte. Próxima etapa: fechar a matriz de fronteiras de replay/atomicidade sem fabricar equivalência; depois consolidar FUP-01..04 para revisão adversarial R4.


### Fechamento técnico FUP-04 — matriz W09 com limites explícitos (28/11/2026)

- **CI #11281:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46169324245), SHA `a71618b7f8b838aee4010dab2d4f5f8ee3f72c79).
- R3 descartável: **295/295 testes Linux e 295/295 Windows**; corpus P-A base 186/186 em cada plataforma. R2 independente: 256/256 Linux; R2 Python: 256/256 Windows; mutation checks R2 PASS; corpora canônico e replayável byte-identical Linux/Windows; Rust quality/unit regression SUCCESS.
- Adicionado `replay_boundary_audit.py`: verifica o envelope/sufixo candidato e o estado de replay R2, mas classifica ambos como `IncomparableReplayModels` mesmo quando o profile coincide. Cursor do harness, sequência de operações, accepted-input log, applied IDs, obligations, recalculate-required e virtual effects permanecem em namespaces sem equivalência demonstrada. Testes cobrem perfil divergente, lacuna de sequência e permutação de eventos sem alterar a sequência lógica.
- **FUP-04 está TECNICAMENTE CONCLUÍDO com resultado limitado e explícito:** (a) projeção parcial P-D→R2 para watermark/deadlines; (b) projeção parcial do TransactionBook compartilhado P-D/P-E→R2 TransactionRecord; (c) auditoria P-E sem coerção de SyncState para MachineState; (d) replay candidato-local preservado e fronteira de replay R2 declarada incomparável; (e) controles R2 de snapshot suportado, UnsupportedSnapshot em INT-ack UNKNOWN e restore atômico de artefato forjado; (f) campos cobertos, ausentes e não comparáveis explicitamente inventariados.
- **Não significa equivalência integral P-D/P-E/R2.** Domínios/horizons P-E, transitions/constraints, IRQ/ack, efeitos pendentes/confirmados sem ledger correspondente e R2 accepted-input replay continuam sem mapeamento. Nenhum campo foi sintetizado e R2 permaneceu imutável.
- A PR #1294 permanece Draft e sem merge, aguardando consolidação dos FUP-01..03 e revisão adversarial R4. Não iniciar FUP-05 enquanto a equivalência funcional necessária aos benchmarks pareados não estiver demonstrada.
