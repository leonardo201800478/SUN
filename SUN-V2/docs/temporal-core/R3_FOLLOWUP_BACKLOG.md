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


### Atualização FUP-01/FUP-02 — diferenciais W05 e W03/W04 (28/11/2026)

- **FUP-01 / PR #1290:** CI #11283 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46169692644), SHA `a3a3704cec15dcc77b5a1d6f8caaf41f6d4210f9`. R3 283/283 Linux e Windows; P-A corpus 186/186 por OS; R2 independente 256/256 Linux; R2 Python 256/256 Windows; mutation checks, corpora byte-identical e Rust quality/unit regression SUCCESS.
- Nova matriz W05 diferencial adiciona descritores inválidos de cancelamento: owner mismatch, identity ausente, generation bool, coordenada float, microstep negativo e phase order/evidence incompletos. Todos devem retornar InvalidInput e preservar exatamente os estados originais P-A/P-D.
- **FUP-01 permanece PARCIAL:** a matriz cobre substituição atômica/fencing, C/A(E)/D/W, arbitragem, contextos incompletos e rejeições selecionadas; ainda falta rastreabilidade exaustiva dos casos R2-T06/T15 aplicáveis a W05. Não há promoção de fase sintética a FACT físico.
- **FUP-02 / PR #1291:** CI #11284 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46169747181), SHA `a729acf77205fcef7ff5a2291c3d164b98b1c8fb`. R3 278/278 Linux e Windows; P-A corpus 186/186 por OS; R2 independente 256/256 Linux; R2 Python 256/256 Windows; mutation checks, corpora byte-identical e Rust quality/unit regression SUCCESS.
- A matriz P-E/P-A W03/W04 agora inclui dependência explícita não comutativa entre transições no mesmo instante e repetição idempotente de observação sem avanço de domínio não solicitado, além dos casos já cobertos de empate desconhecido/comutativo, participante ausente, observação retroativa, cadeia transitiva e observações frequentes.
- **FUP-02 permanece PARCIAL:** cobertura causal ampliada, mas W06 fase física segue UNKNOWN e W07 relação B/C UNKNOWN/BLOCKED. Divergências sintéticas e metadata de commutativity continuam sem autoridade de hardware.
- PRs #1290 e #1291 seguem abertas, sem merge. Jobs de produção SMS/Z80/VDP/PSG/YM2413/CPAL/SDL3/Slint/WGPU/scheduler SKIPPED; nenhum resultado desses jobs é contado como PASS.


### Atualização FUP-03 — composição W08 TransactionBook/ResourceLedger (28/11/2026)

- **PR #1293 / CI #11289:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46170082575), SHA `7d4eb4dbaa729a49a588fe4f7bbbcf8e4742efa5`. R3 282/282 Linux e Windows; P-A corpus 186/186 por OS; R2 independente 256/256 Linux; R2 Python 256/256 Windows; mutation checks, corpora byte-identical e Rust quality/unit regression SUCCESS.
- Nova composição experimental `transaction_resource_bridge.py`: reserva + submit atômicos com amount e associação marker→effect fornecidos explicitamente pela fixture; commit marker + commit effect atômicos/idempotentes; validação de igualdade dos ledgers committed/pending; falha preserva prefixo committed e sufixo pending.
- A ponte é restrita a uma transação por estado experimental. Não define capacidade de hardware, custo por transação, bytes, ciclos ou limite de armazenamento do TransactionBook.
- Recovery foi auditado como **IncomparableRecovery**: TransactionBook.recover retoma InProgress preservando markers pendentes; ResourceLedger.recover termina o ledger e commita efeitos pendentes. A auditoria não muta o estado e não aplica ambos os recovery paths.
- **FUP-03 permanece PARCIAL:** ainda falta decidir se um limite de armazenamento do TransactionBook é necessário como orçamento interno de implementação, separado de hardware; recovery composto não tem semântica equivalente demonstrada; generation replacement do TransactionBook não é inferido.


### Consolidação FUP-01 — matriz de rastreabilidade W05 (28/11/2026)

- Criada `SUN-V2/docs/temporal-core/R3_FUP01_W05_TRACEABILITY.md`, com crosswalk dos 78 métodos R2-T06 e dos 10 casos R2-T15 de atomicidade, classificando cada grupo como Direct, Partial, Absent ou Out of scope.
- A matriz separa correspondência P-A/R2 explicitamente testada de comparação P-D↔P-A; não trata o P-A como prova física nem promove phase metadata sintética a evidência de hardware.
- Gaps explícitos incluem validação estrutural de livros/containers malformados, owner/participant e watermark mismatch em alguns caminhos, invalid clock conversion precedence, preconstructed pending cancellation, lifecycle invariants em todas as operações e fronteiras combinadas W/microstep.
- Casos T15 de ResourceLedger, TransactionBook e snapshot/IRQ acknowledge permanecem em W08/W09/T11, sem serem artificialmente incorporados a W05.
- **FUP-01 continua PARCIAL.** A matriz organiza a cobertura existente; não afirma equivalência integral dos 78 casos T06 nem fecha os gaps identificados.
- Documento criado na branch `research/chronos-r3-follow-up`, commit `f45fd226b445a91ce4a9c458c864ce5ce45e7312`. A atualização deste backlog registra o link; testes Rust não são aplicáveis a esta alteração documental.

### Atualização de validação W08 — PR #1293 (28/11/2026)

- Corrigida a ordem de validação em `transaction_resource_bridge.py`: `_validate_spec(spec)` agora ocorre antes do acesso a `spec.identity` quando já existe transação.
- Adicionado `test_malformed_retry_descriptor_is_rejected_before_identity_access`: retry com `spec=None` deve retornar `InvalidInput` e preservar exatamente o estado composto.
- Commit: `354d2a5da7b5d12885ade6f045de8387f309acde`; CI #46173011966 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46173011966). R2 independente Linux, R2 Python Windows, R3 Linux/Windows, corpora canônico/replayável equivalentes e Rust quality/unit regression: PASS.
- Jobs de produção SMS/Z80/VDP/PSG/YM2413/CPAL/SDL3/Slint/WGPU/scheduler: SKIPPED, não PASS.
- Finding P2 da revisão continua aberto no GitHub até confirmação/revisão; a correção está validada pela CI, mas não foi marcado como resolvido por automação.


### Consolidação FUP-02 — matriz de rastreabilidade W03/W04 (28/11/2026)

- Criado `SUN-V2/docs/temporal-core/R3_FUP02_W03_W04_TRACEABILITY.md`, com crosswalk dos 13 casos R2 causal synchronization, 6 casos R2 partial-order e fixtures replayáveis W03/W04.
- A matriz separa P-A↔R2, P-E↔P-A e fixtures P-A↔R2; não afirma que as duas arestas transitivas equivalem a um replay triplo idêntico.
- P-E tem seis diferenciais expandidos P-E/P-A, 19 testes próprios e fixtures W03/W04 separados P-A/R2. Os fixtures W03/W04 não executam P-E.
- A cobertura de P-E não reproduz as 120 permutações de cinco eventos para todos os grupos R2 partial-order; pares e cadeias selecionadas não são substitutos.
- `Blocked` para dependência ausente em R2 e `InvalidInput` para descritor de dependência desconhecido em P-E permanecem classificações distintas até prova de que as pré-condições são idênticas.
- **W06 permanece UNKNOWN; W07 permanece UNKNOWN/BLOCKED.** Positive control com phase basis sintética não prova fase física; microstep não resolve relação B/C.
- **FUP-02 permanece PARCIAL.** Documento criado na branch `research/chronos-r3-follow-up`, commit `610e7018aab8bcd53ecf6583c6859f2671f8cdca`. Sem alteração de código de produção.


### Consolidação FUP-03 — decisão de fronteira TransactionBook/ResourceLedger (28/11/2026)

- Criado `SUN-V2/docs/temporal-core/R3_FUP03_W08_BOUNDARY_AUDIT.md`, com crosswalk R2-T08/T15, composição W08, atomicidade, exactly-once, falha parcial, recovery e generation/identity.
- **Decisão:** não adicionar `capacity`/`max_transactions` ao TransactionBook no R3. R2 e R3 mantêm coleção de registros sem limite lógico; não existe contrato de capacidade, custo por transação ou eviction.
- O ResourceLedger mantém sua capacidade abstrata caller-supplied, separada do TransactionBook. Não representa contagem de transações, bytes, ciclos, slots de hardware ou limite físico SMS.
- Um eventual orçamento de memória em runtime será política interna de implementação, separada do contrato de hardware, e depende de medição e da política de retenção necessária a snapshot/replay. Nenhum valor fixo foi inventado.
- Recovery TransactionBook/ResourceLedger permanece **IncomparableRecovery**; os caminhos não devem ser aplicados sequencialmente como se fossem uma transação composta.
- Generation do TransactionBook permanece parte do descritor estável; divergência de generation sob a mesma identity é retry inválido, não replacement.
- Documento criado na branch `research/chronos-r3-follow-up`, commit `6c5d1403619e3b8a1fbf8184f11e19f0eed0f412`. FUP-03 continua PARCIAL.


### Revalidação FUP-01 e consolidação documental FUP-01..03 (28/11/2026)

- **FUP-01 / PR #1290:** seis regressões adicionais em W05 cobrem conjunto esperado vazio, clock conversion inválido, watermark divergente, limite de microstep em cancelamento, request C>D pré-construído e cancelamento após W fechado.
- HEAD validado: `0ba1de3994e59e289be840e3a0b5535cdc22f34b`; CI #46174170146 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46174170146). R3 Linux/Windows, R2 independente Linux, R2 Python Windows, equivalência de corpora canônico/replayável e Rust quality/unit regression: PASS.
- A matriz W05 foi atualizada no documento `R3_FUP01_W05_TRACEABILITY.md`, commit `8823bb90316fac32f78085adb2fe0d120bbc2c3d`. Os gaps estruturais restantes e casos não comparáveis continuam explícitos; FUP-01 permanece PARCIAL.
- O CI confirmou R3 prototype suite 289 tests por plataforma; canonical P-A report 125 fixtures/1.496 cuts (1.485 differential + 11 property cuts); replayable corpus 30 fixtures/143 operations/cuts. Os IDs divergentes W06/W07 continuam registrados como limitações, sem inferência física.
- **FUP-02:** matriz `R3_FUP02_W03_W04_TRACEABILITY.md`, versão sem trailing whitespace no commit `bbf6dbde27cdf1db2e19ea62a113c85cee0a720d`; CI documental #46174219914 SUCCESS. P-E↔P-A e P-A↔R2 continuam como arestas distintas; W06 UNKNOWN e W07 UNKNOWN/BLOCKED.
- **FUP-03:** auditoria `R3_FUP03_W08_BOUNDARY_AUDIT.md`, versão sem trailing whitespace no commit `334a65d608ab8ebfff623e40910507f4d852c7d5`. Decisão registrada: sem capacidade lógica/max_transactions no TransactionBook; eventual budget de memória é política interna futura, sem valor fixo e sem equivalência com capacidade física. Recovery continua IncomparableRecovery.
- Todos os documentos e alterações permanecem em branches de pesquisa. Nenhum merge executado; nenhuma implementação de produção CHRONOS/V2 iniciada.


### Estado reconciliado final desta rodada (28/11/2026)

#### FUP-01 / W05 — PR #1290

- HEAD: `572e64740409782ecf06fb0b58091c3ef13e8007`.
- CI #46174471762 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46174471762): R3 293 testes Linux/Windows; R2 independente Linux e R2 Python Windows; corpus canônico/replayável byte-equivalent; Rust quality/unit regression PASS.
- A rodada adicionou dez regressões P-A/R2 (seis na primeira extensão e quatro na segunda) para A(E), watermark, owner, lifecycle, microstep e coordenadas não exatas.
- Canonical P-A report: 129 fixtures / 1.508 cuts (1.497 differential + 11 property cuts); replayable corpus: 30 fixtures / 143 operações/cortes.
- Matriz atual: `R3_FUP01_W05_TRACEABILITY.md`, commit `b38c7b2308cf1d55cefb4d049e09ae0432cc0b00`.
- **FUP-01 permanece PARCIAL:** validação estrutural completa de livros/containers malformados e alguns invariantes de lifecycle ainda não têm crosswalk um-a-um.

#### FUP-02 / W03-W04 — PR #1291

- HEAD: `c8e86be79acba313c9b7abf882ceed415c3a44b5`.
- CI #46174772392 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46174772392): R3 282 testes Linux/Windows; R2 independente Linux e R2 Python Windows; corpora byte-equivalent; Rust quality/unit regression PASS.
- Adicionados quatro testes P-E com 120 permutações cada: cinco eventos comutativos, cinco UNKNOWN não comutativos, cadeia explícita de dependências e DAG parcial com irmãos comutativos.
- Um teste inicialmente falhou porque P-E considerava apenas dependência direta. A correção calcula alcance transitivo no grafo explícito `depends_on`; a cadeia e o DAG alinhado à fixture R2 passam.
- Matriz atual: `R3_FUP02_W03_W04_TRACEABILITY.md`, commit `6cc50054036af757b6fbf5c8ff19fc9667b267a5`.
- **FUP-02 permanece PARCIAL:** os novos 120-permutation tests são propriedades independentes P-E, não um diferencial triplo P-E↔P-A↔R2. W06 continua UNKNOWN e W07 UNKNOWN/BLOCKED.

#### FUP-03 / W08 — PR #1293

- HEAD: `f5179cd1383ec9734e781de281211be669023cd2`.
- CI #46174810003 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46174810003): R3 284 testes Linux/Windows; R2 independente Linux e R2 Python Windows; corpora byte-equivalent; Rust quality/unit regression PASS.
- Além da correção do descritor inválido `spec=None`, adicionada regressão com cinco subcasos de estado composto inválido: mapping incompleto/divergente, reservation identity divergente, pending-effect mismatch e failure state sem correspondência. Todas as rejeições preservam o estado original.
- Auditoria atual: `R3_FUP03_W08_BOUNDARY_AUDIT.md`, commit `5b8c658731e4f03c8f25d3cfb9cc3edf70af24d7`.
- Decisão mantida: sem capacidade lógica no TransactionBook; budget de memória, se necessário, é política interna futura. Recovery segue IncomparableRecovery.
- O finding P2 de validação do descritor da PR #1293 foi corrigido, validado e respondido; thread de revisão resolvida.

**Situação global:** FUP-01, FUP-02 e FUP-03 continuam PARCIAIS, com limites e lacunas identificados. FUP-04 permanece tecnicamente concluído com limites explícitos. FUP-05, R4 e R5 continuam bloqueados. Nenhum merge foi executado e nenhum código de produção CHRONOS/V2 foi criado.


### Atualização final FUP-01 — fronteira racional exata (28/11/2026)

- A matriz `R3_FUP01_W05_TRACEABILITY.md` foi atualizada para enumerar explicitamente os 78 métodos R2-T06 (17 publication/admission + 13 cancellation frontier + 12 C/D + 5 replacement + 3 watermark/arbitration + 4 ties + 6 microsteps + 9 structural + 9 lifecycle).
- Adicionado o 11º teste diferencial W05: `test_cancel_frontier_accepts_exact_rational_participant_conversions_matches_r2`. Ele compara cancelamento com conversões racionais exatas em dois participantes e verifica aceitação/estado pendente contra R2.
- HEAD final PR #1290: `8ad22e3cdf8d4c97f4c557f061a9fd4e8f96013e`; CI #46175104624 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46175104624): R3 294 testes Linux/Windows; R2 independente Linux; R2 Python Windows; equivalência de corpora; Rust quality/unit regression PASS.
- Canonical P-A report: 130 fixtures / 1.510 cuts (1.499 differential + 11 property cuts); replayable corpus: 30 fixtures / 143 operações/cortes.
- Matriz atualizada no commit `4b4e9476c77a773dfaceb08c96d4fbd304858435`. **FUP-01 permanece PARCIAL** devido aos gaps estruturais/lifecycle listados na matriz.

### Confirmação de limites FUP-02/FUP-03

- **FUP-02 / PR #1291:** HEAD `c8e86be79acba313c9b7abf882ceed415c3a44b5`, CI #46174772392 SUCCESS. Quatro propriedades P-E com 120 permutações cada passam em Linux/Windows; a resolução transitiva de `depends_on` foi corrigida e comparada à topologia R2. Não há ainda diferencial triplo P-E/P-A/R2 para todos os 120 casos. W06 UNKNOWN; W07 UNKNOWN/BLOCKED.
- **FUP-03 / PR #1293:** HEAD `f5179cd1383ec9734e781de281211be669023cd2`, CI #46174810003 SUCCESS. Teste de integridade da composição acrescentou cinco subcasos de estado inválido e rejeição atômica. Finding P2 resolvido na revisão. TransactionBook sem limite lógico; ResourceLedger capacity separada; recovery IncomparableRecovery.
- PRs #1289–#1294 permanecem abertas; nenhuma foi mergeada. R3 continua em andamento; FUP-01/02/03 parciais; FUP-04 tecnicamente concluído com limites. FUP-05, R4 e R5 não iniciados.


### FUP-02 — fechamento da rodada de permutações triplas (28/11/2026)

- HEAD PR #1291: `009ab3d8b375c382dad7fecc7ee1df2a573351e2`.
- CI #46175394407 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46175394407): R3 286 testes Linux/Windows; R2 independente 256/256 Linux; R2 Python 256/256 Windows; equivalência dos corpora canônico/replayável e Rust quality/unit regression: PASS.
- Quatro diferenciais P-E↔P-A↔R2, cada um com 120 permutações: comutatividade, UNKNOWN não comutativo, cadeia explícita de dependências e DAG parcial com irmãos comutativos. As fixtures de cadeia e DAG reproduzem a topologia R2.
- O finding de dependência transitiva foi corrigido no P-E e permanece coberto pelos testes de ciclo e pelas 120 permutações. Não há promoção de phase metadata a evidência física.
- Matriz W03/W04 atualizada no commit `45634029bad00491090aa8cba81f9c0f2cd26ebf`.
- **FUP-02 permanece PARCIAL:** W03/W04 replay fixtures P-A/R2 não executam P-E; faltam ainda alguns crosswalks para observation/alters e pre-applied causal ledger. W06 UNKNOWN; W07 UNKNOWN/BLOCKED.

### Referências finais das matrizes R3

- FUP-01 W05: `R3_FUP01_W05_TRACEABILITY.md`, HEAD documental `4b4e9476c77a773dfaceb08c96d4fbd304858435`; evidência de código/CI mais recente PR #1290 HEAD `8ad22e3cdf8d4c97f4c557f061a9fd4e8f96013e`, CI #46175104624 SUCCESS.
- FUP-02 W03/W04: `R3_FUP02_W03_W04_TRACEABILITY.md`, HEAD documental `45634029bad00491090aa8cba81f9c0f2cd26ebf`; evidência de código/CI PR #1291 HEAD `009ab3d8b375c382dad7fecc7ee1df2a573351e2`, CI #46175394407 SUCCESS.
- FUP-03 W08: `R3_FUP03_W08_BOUNDARY_AUDIT.md`, HEAD documental `5b8c658731e4f03c8f25d3cfb9cc3edf70af24d7`; evidência de código/CI PR #1293 HEAD `f5179cd1383ec9734e781de281211be669023cd2`, CI #46174810003 SUCCESS.


### FUP-03 — fechamento dos dois findings P2 da ponte W08 (28/11/2026)

- PR #1293 HEAD: `fcafd5f7901a091db2d4cd087795b88c3f32f9fb`.
- CI #46175667275 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46175667275): R3 285 testes Linux/Windows; R2 independente Linux; R2 Python Windows; corpora canônico/replayável byte-equivalent; Rust quality/unit regression PASS.
- Segundo finding P2 resolvido: SAMPLE marker agora é executado pela operação coarse-grained de ponte `sample_transaction_marker()`; retorna READY→InProgress, registra o valor, mantém ResourceLedger inalterado e repete idempotentemente o primeiro valor capturado. COMMIT seguinte continua pareado atomicamente ao efeito do ResourceLedger.
- Regressão: `test_sample_marker_returns_to_in_progress_before_commit_and_is_idempotent`.
- Ambos os findings P2 da PR #1293 foram respondidos e as duas threads estão resolvidas.
- Auditoria W08 atualizada em `R3_FUP03_W08_BOUNDARY_AUDIT.md`, commit `fd385c2ee09ac2ac644f7d774d57cf56b135e1f5`.
- **FUP-03 permanece PARCIAL** exclusivamente quanto aos limites de recovery composto e à necessidade de ampliar a matriz/invariantes além dos cenários já cobertos. A correção SAMPLE não resolve IncomparableRecovery nem altera a decisão de capacidade do TransactionBook.


### FUP-01 — global horizon e precedência de obrigações validados (28/11/2026)

- HEAD PR #1290: `80416f457478f0564ebd3a3d07f21c14648d7a8a`.
- CI #46176113812 [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/46176113812): R3 299 testes Linux/Windows; R2 independente 256/256 Linux; R2 Python 256/256 Windows; corpus canônico/replayável byte-equivalent; Rust quality/unit regression PASS. Rust workspace regression SKIPPED (alteração apenas Python).
- Cinco novos diferenciais P-A/R2 cobrem horizonte regressivo/não exato, horizonte global atrás de W na publicação e cancelamento, violação de horizonte conhecida antes do bloqueio por conjunto incompleto e obrigação pendente inconsistente em publicação/cancelamento.
- Canonical P-A report: 135 fixtures / 1.525 cuts (1.514 differential + 11 property cuts); replayable corpus: 30 fixtures / 143 operações/cortes. W06/W07 continuam como divergências declaradas e UNKNOWN.
- Matriz atual: `R3_FUP01_W05_TRACEABILITY.md`, commit `713263f38c91be3df3ba2b014e51a816db8c164f`. O inventário R2-T06 soma 78 métodos em nove grupos.
- FUP-01 permanece PARCIAL apenas para as lacunas estruturais/malformed-book e alguns invariantes de lifecycle explicitamente listados na matriz.

### FUP-03 — auditoria SAMPLE evidence clarification

- A matriz W08 distingue o diferencial P-A/P-D/P-E do TransactionBook compartilhado e o teste de integração específico da ponte SAMPLE. O wrapper da ponte não altera a evidência de hardware nem a semântica do ResourceLedger.
- Documento atualizado: `R3_FUP03_W08_BOUNDARY_AUDIT.md`, commit `1806a153a5364ab6746610578ec0555e3c636c62`; CI documental #46175970726 SUCCESS.


### FUP-01 — geração ativa, cancelamento pendente e lifecycle crosswalk (28/11/2026)

- PR #1290 adicionou cobertura P-A/P-D/R2 para rejeitar publicação sucessora quando a geração anterior ainda está `Scheduled` ou `CancelPending`. A API contextual P-D agora retorna `InvalidInput` sem mutação; a substituição explícita continua em `replace_with_context`.
- O finding P2 sobre publicação com cancelamento antigo pendente foi corrigido e a thread correspondente foi resolvida após CI #46183529839 SUCCESS no SHA `ec74ea861f8a823d9495cbc6368e14d1e7d0a38c`.
- Evidência nesse SHA: R2 257/257 Linux e Windows; R3 302/302 Linux e Windows; corpus emitter 202/202 por OS; corpus canônico 135 fixtures / 1.525 cuts, digest `4486046a13980e26e3e491bc7e5457cc52159dc5560230421fb9357f41b492b3`; replayable 30 fixtures / 143 operações, digest `ed039c1985d1a0d1dee518a33bfcbbefd3713368e040e113f4b8d856c255e215`; Rust quality/unit regression PASS. Jobs de produção/hardware SKIPPED.
- Nova validação após a cobertura de predecessor Scheduled está em CI #46183873117, candidata SHA `6949b074096bad237cf4413ad75fc5762a6d9218`; no momento deste registro está PENDING. A tentativa anterior SHA `513e825c9f0464dc19cfa7e9ddf16c1038ca32d3` falhou por erro no helper de teste R2 (argumento `at_common` duplicado), corrigido na candidata atual.
- O crosswalk foi atualizado em `R3_FUP01_W05_TRACEABILITY.md`: inventário R2-T06 agora tem 80 métodos; P-D não representa explicitamente `Dispatched` ou `Failed` e contém `Superseded` candidato-local. Esses estados permanecem NOT COMPARABLE/NOT REPRESENTED, sem mapeamento artificial.
- FUP-01 continua PARCIAL; R3 continua em andamento. Nenhuma aprovação R4/R5, merge ou implementação de produção foi realizada.
