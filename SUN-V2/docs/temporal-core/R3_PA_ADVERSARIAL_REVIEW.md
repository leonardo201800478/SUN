# SUN V2 — revisão adversarial do P-A (CHRONOS R3)

**Data:** 2026-11-26
**Classificação:** revisão técnica independente do protótipo; não é aprovação de gate
**HEAD auditado:** `09f71c8aa059c44855c358b614c965f92900a173`
**Branch:** `research/chronos-r3-disposable-prototypes`
**PR:** [#1284 — Begin disposable CHRONOS R3 prototypes](https://github.com/leonardo201800478/SUN/pull/1284)
**R1 normativo:** v0.3.18, SHA de conteúdo `ac912f342ff303b3fdc554b90d00e85b23d86c76`
**R2 aprovado:** merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0`

## 1. Decisão executiva

**P-A permanece uma baseline experimental útil, mas não está pronto para closeout. P-D/P-E permanecem bloqueados.**

A revisão encontrou três limitações que impedem declarar o primeiro corte completo:

1. W07 impõe uma ordem entre obrigações de microsteps sem demonstrar que a relação é causal ou que os efeitos são comutativos par-a-par, como exige R1 §16.6.2.
2. W06 compara descritores de base estruturalmente iguais, mas não resolve nem autentica o registro externo de validação referenciado.
3. O artefato chamado canonical trace corpus contém digests por corte, não as entradas, operações e estados completos necessários para replay e comparação independente de P-D/P-E.

Nenhum destes resultados invalida automaticamente R1/R2. Nenhuma alteração normativa é aprovada por este relatório.

## 2. Base factual e evidência CI

No HEAD auditado, o histórico remoto reporta CI #10937 SUCCESS:

- P-A: 125/125 testes em Linux e 125/125 em Windows.
- R2 independente: 256/256 testes em cada sistema operacional.
- Matrizes sintéticas: 351 pontos por ambiente (75 avanço, 180 deadlines, 30 causal, 24 W06, 12 W07, 10 W08 e 20 W09).
- Cinco warm-ups e trinta repetições medidas por ponto.
- Jobs de hardware/runtime não relacionados foram SKIPPED e não são contados como PASS.

Os testes P-A por arquivo são: 24 avanço/catch-up, 31 deadline lifecycle, 14 causal sync, 11 microsteps, 18 transações e 27 snapshot/replay.

O emissor de corpus registra 112 identificadores de fixture e 1.475 cortes: 1.468 diferenciais e sete propriedades determinísticas. O digest agregado reportado é idêntico entre Linux e Windows. Isso comprova igualdade dos digests efetivamente registrados, não a disponibilidade de um corpus completo de entradas e transições reexecutáveis.

As medições são **OBSERVATION de Python sintético**. Não representam desempenho Rust, clocks físicos SMS, FPS, frame pacing, fidelidade de áudio/vídeo ou compatibilidade de ROM.

## 3. W07 — ordenação entre níveis de microstep

### 3.1 FACT — contrato aprovado

R1 §6, item 1, proíbe observar um evento posterior antes de um evento causalmente anterior. R1 §16.6.2 é mais específico: a sequência de microsteps só pode ordenar dependências causais já estabelecidas ou efeitos comutativos. A numeração de microstep não pode resolver um conflito não comutativo cuja fase seja UNKNOWN.

R1 §16.6.3 permite que um evento criado durante despacho no mesmo instante seja processado segundo uma regra explícita de microstep, mas essa regra continua subordinada ao item §16.6.2.

### 3.2 FACT — comportamento observado no P-A

Em `p_a_microsteps.py`, `AStepObligation` contém:

- `identity`;
- `owner`;
- `microstep`;
- `commutative: bool`.

O processamento encontra o menor nível lógico pendente. Se a obrigação solicitada estiver em nível posterior, retorna `Blocked` até que as obrigações anteriores sejam resolvidas.

O teste `test_later_microstep_cannot_overtake_pending_commutative_sibling` executa:

1. A e B são criados no microstep 0, ambos marcados com `commutative=True`.
2. A é processado e emite C no microstep 1.
3. B permanece pendente no microstep 0.
4. Tentar processar C retorna `Blocked`.
5. B é processado.
6. C é processado e o ciclo termina.

O teste também registra que o R2 aprovado aceita o processamento de C antes de B. Essa divergência não é mascarada como equivalência diferencial.

### 3.3 Finding — a barreira impõe precedência sem relação demonstrada

Depois de A emitir C, o protótipo impõe B antes de C. Entretanto:

- Não foi declarada dependência causal B→C.
- O campo `commutative` é uma propriedade booleana individual; o modelo não registra uma relação de comutatividade entre o par B/C.
- O teste não demonstra que B e C comutam.
- O fato de B estar no microstep 0 e C no microstep 1 não estabelece, por si só, uma relação causal física ou lógica entre B e C.

A propriedade “B é comutativo” não pode ser interpretada silenciosamente como “B comuta com toda obrigação futura”, a menos que esse significado global seja definido e justificado pelo contrato do modelo. O código atual não carrega uma matriz ou relação par-a-par que permita demonstrar isso.

Consequentemente, o teste demonstra que o algoritmo aplica uma barreira de níveis, mas não demonstra que a ordenação B<C é autorizada por R1 §16.6.2.

### 3.4 HYPOTHESIS / INFERENCE — alternativas para investigação

Não escolher uma alternativa sem nova revisão. As opções a avaliar são:

- Representar relações causais explicitamente entre obrigações e permitir progressão apenas quando a dependência estiver demonstrada.
- Representar comutatividade par-a-par ou por classes formalmente definidas e verificáveis.
- Retornar `IndeterminateCausalOrder` quando a relação entre obrigações de níveis distintos não puder ser demonstrada.
- Revisar formalmente a interpretação de microstep, sem usar a numeração como substituto de causalidade.

### 3.5 Decisão W07

**BLOCKED — não fechado.** O teste atual valida uma hipótese conservadora, mas a hipótese não está demonstrada como consequência do R1 aprovado. Não promover esta regra a semântica normativa nem usá-la para selecionar arquitetura até resolver a relação B/C.

## 4. W06 — base comparável versus evidência validada

### 4.1 FACT — implementação

P-A introduz `APhaseBasis`, contendo perfil/revisão, coordenada/unidade, escala racional, identidade/revisão da base e identificador/digest do registro de validação.

O protótipo exige bases iguais para comparar ordens de fase de eventos não comutativos no mesmo microstep. Base ausente ou diferente produz `IndeterminateCausalOrder` sem fechar watermark nem reivindicar deadlines. Descritor malformado é rejeitado na publicação sem mutação.

### 4.2 Limite observado

A validação atual verifica estrutura e formato do digest. Ela não resolve o identificador do registro externo, não recupera seu conteúdo e não autentica que o conteúdo referenciado corresponde à revisão/perfil/escala declarados.

Um SHA-256 com formato válido não prova que o registro existe, é íntegro, tem autoridade ou contém uma medição aplicável.

Os testes positivos usam fixtures sintéticas. Nenhum deles prova fase elétrica real do SMS.

### 4.3 Decisão W06

**LIMITADO A MECÂNICA SINTÉTICA; evidência física permanece UNKNOWN.**

A exigência de igualdade estrutural de bases é uma melhoria experimental útil. Para declarar evidência verificável, será necessário definir e testar a resolução/autenticação do registro externo ou manter explicitamente o resultado como UNKNOWN no perfil real. Não criar fixtures sintéticas com aparência de prova física.

## 5. Corpus canônico — digests não equivalem a trace reexecutável

### 5.1 FACT — implementação observada

`emit_p_a_trace_corpus.py` executa os seis módulos de teste e chama `build_document` em `trace_fixture_support.py`.

O recorder:

- normaliza valores e frações;
- calcula SHA-256 canônico;
- compara os digests dos dois lados em `_record` e lança AssertionError quando divergem;
- grava `fixture`, `cut_index`, `basis` e um único `digest`.

O documento agregado contém contagens, `all_equal`, digest agregado, fixtures (ID e número de cortes) e registros de corte.

### 5.2 Limitações

O artefato não serializa:

- definições e entradas completas das fixtures;
- sequência de operações e argumentos;
- estado completo de cada domínio em cada corte;
- lifecycle/generation detalhado dos deadlines;
- watermark e ledger de efeitos como dados reexecutáveis;
- digest separado de cada lado ou payload suficiente para inspecionar o primeiro estado divergente.

Além disso, `all_equal` é gravado como constante `true`; a igualdade é efetivamente protegida pelas exceções em `_record`, mas o campo agregado não é calculado independentemente a partir de evidências armazenadas.

O digest de cada corte é útil para detecção de divergência durante a execução do teste. Contudo, sem entradas e transições completas, o artefato não é suficiente para reproduzir a mesma sequência em P-D/P-E nem para uma auditoria independente do estado divergente.

### 5.3 Decisão sobre o corpus

**INCOMPLETO para o gate de equivalência canônica P-A/P-D/P-E.**

O próximo corpus deve, no mínimo, permitir reexecução independente da mesma fixture/seed/operação e inspeção do corte divergente. Deve armazenar a entrada canônica, operação/argumentos, resultado/status, estado observável relevante por domínio e digests por lado; deve identificar explicitamente divergências e manter versionamento de schema e hashes dos artefatos.

Alternativamente, o requisito de R3 pode ser revisado formalmente antes de P-D/P-E, mas não pode ser considerado atendido apenas pelo digest agregado atual.

## 6. Estado do gate e ações necessárias

| Item | Resultado |
|---|---|
| P-A testes unitários/diferenciais registrados no CI | PASS |
| R2 independente no CI | PASS |
| Matrizes sintéticas W00–W09 | PASS como execução |
| W06 como mecanismo sintético | PASS limitado |
| W06 como evidência física SMS | UNKNOWN |
| W07 conformidade com R1 §16.6.2 | BLOCKED |
| Corpus canônico reexecutável completo | BLOCKED |
| P-A formal closeout | BLOCKED |
| P-D/P-E | NOT RUN |
| R4/R5 | PENDENTES |
| Código de produção CHRONOS/V2 | NÃO AUTORIZADO |

### Ordem recomendada

1. Resolver formalmente W07 e criar testes discriminantes que provem a relação causal ou a comutatividade exigida.
2. Definir a fronteira de confiança de W06: resolver/autenticar registros externos ou manter a fase física UNKNOWN.
3. Completar o corpus reexecutável com fixtures, entradas, operações e estados/cortes; validar igualdade entre Linux e Windows.
4. Reexecutar CI no novo HEAD e preservar artefatos/SHAs.
5. Só então emitir P-A closeout e decidir a entrada em P-D/P-E.

## 7. Preservação e proibições

- Preservar R1 v0.3.18 e o merge R2 `18b4352e16cf6ab275c4923a30acc3124754a1b0`.
- Não alterar `SUN-V2/verification/r2_reference/` para fazer P-A passar.
- Não transformar a divergência W07 em equivalência diferencial positiva.
- Não promover fixtures sintéticas de W06 a evidência física.
- Não iniciar P-D/P-E antes de resolver os blockers ou revisar formalmente os critérios de entrada.
- Não criar código de produção CHRONOS/V2 antes de R5.
- Nenhuma alteração de R1/R2, gate R4/R5 ou arquitetura final é aprovada por este relatório.

**Conclusão:** CI verde confirma que os testes implementados passaram. Não transforma hipóteses semânticas em contrato aprovado nem substitui o corpus reexecutável exigido para comparação entre arquiteturas.

### 5.4 Proposta de esquema mínimo para corpus reexecutável (não implementada)

Para tornar o requisito verificável sem acoplar P-D/P-E à implementação interna de P-A, cada fixture deve ser uma entrada declarativa e cada protótipo deve consumir a mesma sequência canônica de operações. O artefato deve conter, no mínimo, os seguintes blocos:

| Bloco | Campos mínimos | Finalidade |
|---|---|---|
| `schema` | `name`, `version`, `canonical_encoding` | Evolução explícita do formato e serialização determinística |
| `provenance` | `source_commit`, `generator_version`, `test_suite_version`, `profile_id`, `profile_revision` | Identificar origem e escopo do corpus |
| `fixture` | `fixture_id`, `seed` quando aplicável, `initial_state`, `external_inputs` | Reconstruir o estado inicial e todas as entradas |
| `operations` | lista ordenada de `sequence`, `operation`, `arguments`, `admission_context` | Reexecutar exatamente a mesma sequência sem inferi-la do protótipo |
| `cuts` | `cut_id`, `after_sequence`, `common_time`, `status`, `error`, `observable_state` | Inspecionar cada fronteira após a operação |
| `comparison` | `left_prototype`, `right_reference`, `left_digest`, `right_digest`, `equal`, `first_divergence` | Comparar os dois lados e localizar divergência sem depender apenas do hash |

O `observable_state` deve preservar, quando existentes no cenário: estado por domínio, progresso local e resíduo/fase, watermark, deadlines com identidade/geração/lifecycle, dependências pendentes, transações e marcos de sample/commit, ledger de efeitos, cursor de entradas e microsteps pendentes. Campos inaplicáveis devem ser omitidos por regra do schema, não preenchidos com valores fictícios.

**Regras de codificação e integridade:**

1. Tempo racional deve ser codificado exatamente (numerador e denominador inteiros, preferencialmente strings decimais para interoperabilidade); floats são proibidos.
2. Operações e coleções semanticamente ordenadas preservam sua ordem explícita. Mapas são serializados com chaves ordenadas; conjuntos são serializados por uma ordenação canônica definida pelo schema.
3. Cada lado guarda seu próprio estado canônico e digest. `equal` é calculado a partir dos estados/digests efetivamente armazenados; não pode ser constante.
4. Se houver divergência, `first_divergence` deve conter o primeiro `cut_id`, caminho do campo, valores canônicos de ambos os lados e status/erro correspondente. O artefato não deve abortar antes de persistir esse diagnóstico.
5. O corpus deve ser suficiente para executar a fixture sem importar código de geração de estado do protótipo comparado. O adaptador de cada protótipo apenas traduz a fixture para suas próprias estruturas.
6. O digest agregado deve cobrir schema, provenance relevante, fixture, operações e cortes, não apenas IDs/índices/digests dos cortes.
7. Linux e Windows devem produzir os mesmos bytes canônicos e SHA-256 para a mesma versão de schema e fixture; metadados específicos de ambiente ficam fora do digest semântico ou em bloco separado claramente excluído.

Este esquema é uma **proposta de artefato de verificação**, não uma decisão sobre API, tipos de runtime ou estrutura de produção CHRONOS. Antes de implementar, validar o schema contra W00–W09, snapshots com estado parcial e pelo menos um caso de divergência intencional, para provar que o primeiro corte divergente é recuperável. O corpus atual continua BLOCKED até essa validação e a execução cruzada em Linux/Windows.


### 5.5 Implementação experimental do corpus e fixture W07 (em validação)

No HEAD experimental posterior à revisão, foi adicionado o harness descartável `replayable_fixture_corpus.py`, schema `sun.chronos.r3.replayable-fixture.v0.1`. Ele registra estado inicial, provenance, operações/argumentos ordenados, estado completo por corte, digest independente por lado e primeira divergência. O replay verifica o digest do artefato e cada estado de corte.

Também foi adicionado `test_w07_replayable_fixture.py`, que adapta a mesma fixture declarativa aos modelos P-A e R2 sem compartilhar a lógica de transição. A sequência registra A/B no microstep 0, A emitindo C no microstep 1 e a tentativa de C antes de B. A expectativa observada é divergência no corte da operação 1: P-A retorna `Blocked`; R2 retorna `Advanced`. O fixture então continua até o fim e exige replay independente dos dois lados.

**Limite:** isto é uma fixture sintética executável do caso W07, não evidência física SMS nem prova de que a ordem escolhida por P-A ou R2 seja fisicamente correta. A divergência continua sendo um blocker semântico. O primeiro CI após a integração falhou por erro no adaptador de replay (o estado serializado retornado por uma operação não era desembrulhado antes da operação seguinte). A correção foi aplicada no commit `bc40d435d818e624ed7b8f2607a3e32fcebeaac4`, mas ainda aguarda execução CI específica neste SHA. Até CI Linux/Windows PASS, classificar a integração como **NOT VALIDATED**.

O emissor do corpus agregado permanece separado: a fixture de divergência não é misturada aos cortes diferenciais que exigem igualdade. Ela é capturada e validada pelo próprio teste de replay, preservando a distinção entre igualdade diferencial e divergência intencional.
