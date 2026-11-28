# SUN V2 — revisão adversarial do P-A (CHRONOS R3)

**Data:** 2026-11-26
**Classificação:** revisão técnica independente do protótipo; não é aprovação de gate
**HEAD auditado nesta revisão:** `f2e0e83735284d5137c5755b27f8a82d75f6a9d6`
**HEAD da revisão original:** `09f71c8aa059c44855c358b614c965f92900a173`
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

### Evidência histórica da revisão original

Na revisão original do HEAD `09f71c8aa059c44855c358b614c965f92900a173`, o histórico remoto reportava CI #10937 SUCCESS:

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

**Fronteira de confiança ainda não implementada (UNKNOWN):** registry/resolver de registros, autoridade ou trust anchor, codificação canônica dos bytes do registro, política de assinatura/autenticação, revogação/expiração, vínculo de perfil/revisão e validação da origem da medição. Não escolher algoritmo, formato de assinatura ou autoridade por inferência.

Critérios mínimos para uma futura alegação verificável:
1. Resolver o identificador exato do registro em uma fonte/registry identificada.
2. Recuperar os bytes canônicos exatos e verificar o SHA-256 declarado sobre esses bytes.
3. Validar o vínculo do registro com perfil/revisão, coordenada/unidade, escala racional e identidade/revisão da base.
4. Aplicar uma política de autoridade/autenticação aprovada e documentada, incluindo validade/revogação quando aplicável.
5. Preservar URI/identificador da fonte, versão, método de aquisição, SHA, perfil, ambiente, resultado e limitações.

Até esses critérios serem definidos e satisfeitos por evidência externa real, qualquer registro ausente, não resolvido, não autenticado ou incompatível não pode sustentar ordenação física. Os testes positivos usam fixtures sintéticas. Nenhum deles prova fase elétrica real do SMS.

### 4.3 Decisão W06

**LIMITADO A MECÂNICA SINTÉTICA; evidência física permanece UNKNOWN.**

A exigência de igualdade estrutural de bases é uma melhoria experimental útil. Para declarar evidência verificável, será necessário definir e testar a resolução/autenticação do registro externo ou manter explicitamente o resultado como UNKNOWN no perfil real. Não criar fixtures sintéticas com aparência de prova física.

## 5. Corpus canônico — digests não equivalem a trace reexecutável

### 5.1 FACT — implementação observada

`emit_p_a_trace_corpus.py` executa dez módulos de teste e chama `build_document` em `trace_fixture_support.py`. O fixture W07 declarativo/reexecutável é executado à parte pelo módulo de teste e não deve ser confundido com os cortes diferenciais agregados abaixo.

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

No documento agregado de cortes, all_equal continua sendo true por construção: _record aborta imediatamente se qualquer par divergir. Isso é distinto do campo all_equal do fixture reexecutável, que agora é derivado dos cortes, incluído no digest do artefato e revalidado antes do replay.

O digest de cada corte agregado é útil para detectar divergência durante a execução do teste. Contudo, sem entradas e transições completas para todos os workloads W00–W09, o artefato agregado não é suficiente para reproduzir a mesma sequência em P-D/P-E nem para uma auditoria independente do estado divergente.

### 5.3 Decisão sobre o corpus

**INCOMPLETO para o gate de equivalência canônica P-A/P-D/P-E.**

O próximo corpus deve, no mínimo, permitir reexecução independente da mesma fixture/seed/operação e inspeção do corte divergente. Deve armazenar a entrada canônica, operação/argumentos, resultado/status, estado observável relevante por domínio e digests por lado; deve identificar explicitamente divergências e manter versionamento de schema e hashes dos artefatos.

Alternativamente, o requisito de R3 pode ser revisado formalmente antes de P-D/P-E, mas não pode ser considerado atendido apenas pelo digest agregado atual.

### 5.8 Resultado do primeiro gate de equivalência multiplataforma

O CI #10973 passou em Linux e Windows para os testes R3, em ambos os conjuntos independentes R2, em Actionlint e em higiene documental. O job de comparação do corpus falhou no `cmp` byte a byte.

**Diagnóstico dos artefatos reais do run:** os JSONs descompactados eram semanticamente idênticos (`json.loads(linux) == json.loads(windows)`), e os bytes tornavam-se idênticos após substituir CRLF por LF no artefato Windows. O Linux escreveu 9.313 LF; o Windows escreveu 9.313 CRLF. Portanto, a diferença era a tradução de newline feita por `Path.write_text` no Windows, não divergência de estado, digest, fixture ou ordem de registros. Os SHA-256 originais diferentes não indicam divergência semântica nesse caso.

**Correção e validação:** `emit_p_a_trace_corpus.py` passa `newline="\\n"` explicitamente a `Path.write_text`, definindo bytes LF em todas as plataformas. CI #10975 no HEAD `c2446410526e08f729ce4c1703969aba64cd776c` confirmou PASS em R3 Linux/Windows, R2 Linux/Windows, higiene documental, Actionlint e equivalência byte a byte do corpus. O job `cmp` permaneceu estrito e passou sem normalização. O commit de correção do newline é `43c45cf1883fb4b90c3ec2e7e2b3f127581919a8`.

### 5.9 Fixture reexecutável W08 — commit exatamente uma vez

Foi adicionado `test_w08_replayable_fixture.py` e o módulo foi incluído em `emit_p_a_trace_corpus.py`. A fixture declarativa executa a mesma sequência de operações nos dois lados, usando exclusivamente as transições P-A no adaptador esquerdo e as transições independentes do oráculo R2 no adaptador direito:

1. Submeter o descriptor de transação com um marco COMMIT em tempo racional exato.
2. Iniciar a transação.3. Avançar até o marco declarado.
4. Aplicar o commit.
5. Repetir o mesmo commit para verificar idempotência e ledger exatamente uma vez.

O artefato registra estado completo do book e da transação, status, ledger de efeitos, argumentos e cada corte; replay dos dois lados valida todos os estados capturados. O teste exige igualdade integral P-A/R2 e um único identificador no ledger final.

**Classificação e limite:** OBSERVATION de equivalência entre dois modelos sintéticos. CI #10978 confirmou COMMIT/retry; CI #10987 confirmou WAIT/release e SAMPLE/retry; CI #10989 confirmou falha/retry/recuperação explícita. As três fixtures passaram em Linux e Windows, com replay independente P-A/R2 e equivalência byte a byte do corpus. Não são traces de barramento físico SMS, medições de hardware, nem prova de comportamento exatamente-uma-vez no dispositivo real. W08 permanece parcialmente coberto; consumo separado de TW, cancelamento e combinações mais amplas de ledger parcial ainda não foram migrados para fixtures declarativas independentes.

### 5.10 Fixture reexecutável W09 — snapshot capture/restore

Foi adicionada `test_w09_replayable_fixture.py` e o módulo foi incluído em `emit_p_a_trace_corpus.py`. A fixture usa um estado de origem SMS-NTSC com coordenadas racionais exatas em dois domínios, watermark e input cursor, além de um estado ativo deliberadamente diferente. A sequência declarativa executa:

1. Captura do estado de origem.
2. Restore sobre o estado ativo, com validação explícita de perfil e versão.

O adaptador P-A usa o snapshot canônico em bytes e valida o digest do artefato antes de restaurar. O adaptador R2 usa seu artefato de estado independente. Ambos são projetados para a mesma representação semântica neutra; o fixture registra estados completos em cada corte e exige que o estado ativo permaneça intacto e o estado restaurado corresponda exatamente à origem. O replay reexecuta separadamente os dois lados.

**Classificação e limite:** OBSERVATION sintética de equivalência capture/restore. O CI #10982 encontrou primeiro uma asserção que comparava `Fraction` bruto com estado normalizado; ao corrigi-la, o CI #10984 revelou um defeito do envelope do adaptador: o estado-fonte retornado por capture estava sendo reutilizado incorretamente como alvo ativo de restore. O adaptador agora mantém separados estado-fonte e estado-alvo e verifica que capture/restore não mutam seus respectivos inputs. CI #10985 confirmou PASS em Linux e Windows, incluindo emissão do corpus e equivalência byte a byte. Correção final: commit `0a12cb79806d4e74cf463bbcf6b6fb17276b5996`. Não cobre ainda replay de eventos após restore, deadlines/gerações, ledger parcial de transações ou cortes de acknowledge INT; esses cenários permanecem em testes diferenciais/digest-only ou UNKNOWN e exigem fixtures reexecutáveis próprias. Não é evidência física SMS nem snapshot de runtime de produção.

### 5.7 Achados adicionais da revisão independente — correções de harness e CI

A revisão independente do HEAD 25ff40b registrou quatro achados P2 no harness/corpus e uma ressalva de benchmark. Na branch atual foram aplicadas as seguintes correções experimentais:

1. **Replay bool/int:** replay_fixture_side compara estados por bytes canônicos, não pela igualdade permissiva do Python; foi adicionado teste que diferencia true de 1.
2. **Colisão de chaves:** normalize rejeita chaves de mapping que não sejam strings, eliminando colisões como 1 versus "1"; teste de regressão incluído.
3. **Integridade de all_equal:** o campo integra o digest do fixture; o replay recalcula a igualdade a partir dos cortes, valida os digests de estado e confere first_divergence. Um artefato com flag adulterada é rejeitado mesmo se o digest for recalculado.
4. **Equivalência Linux/Windows:** o job agregador baixa os dois artefatos e exige igualdade byte a byte. **PASS no CI #10975**, após normalização explícita de LF no emissor.
5. **Comparação de quantis:** os drivers R3 utilizam benchmark_statistics.percentile_type7 (Hyndman–Fan Type 7), com a convenção declarada no JSON. A ressalva de 30 amostras permanece: p99 é quantil descritivo da amostra, não limite robusto de cauda.
6. **Base de fase:** _causal_order_known exige APhaseBasis válido e igualdade estrutural da base entre eventos do mesmo nível; perfis/bases divergentes e ausência de base permanecem indeterminados. Isso resolve a comparabilidade estrutural do protótipo, mas **não** autentica o registro externo de W06.

Essas correções não modificam R1 v0.3.18, o oráculo R2 aprovado ou código de produção. A correção do harness não fecha o requisito de corpus completo nem altera W06/W07 para PASS.
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

### 5.4 Schema mínimo de corpus reexecutável — especificação e cobertura parcial

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

Este schema é uma **especificação de artefato de verificação**, não uma decisão sobre API, tipos de runtime ou estrutura de produção CHRONOS. O harness genérico e as fixtures W07, W08 e W09 foram parcialmente implementados em `replayable_fixture_corpus.py`; a migração integral dos workloads W00–W09, incluindo variantes de estado parcial, entradas independentes e os demais cenários de lifecycle, ainda não foi concluída. O corpus completo continua BLOCKED.


### 5.5 Implementação experimental do corpus e fixture W07 (em validação)

No HEAD experimental posterior à revisão, foi adicionado o harness descartável `replayable_fixture_corpus.py`, schema `sun.chronos.r3.replayable-fixture.v0.1`. Ele registra estado inicial, provenance, operações/argumentos ordenados, estado completo por corte, digest independente por lado e primeira divergência. O replay verifica o digest do artefato e cada estado de corte.

Também foi adicionado `test_w07_replayable_fixture.py`, que adapta a mesma fixture declarativa aos modelos P-A e R2 sem compartilhar a lógica de transição. A sequência registra A/B no microstep 0, A emitindo C no microstep 1 e a tentativa de C antes de B. A expectativa observada é divergência no corte da operação 1: P-A retorna `Blocked`; R2 retorna `Advanced`. O fixture então continua até o fim e exige replay independente dos dois lados.

**Limite:** isto é uma fixture sintética executável do caso W07, não evidência física SMS nem prova de que a ordem escolhida por P-A ou R2 seja fisicamente correta. A divergência continua sendo um blocker semântico. O adaptador de replay foi corrigido no commit `bc40d435d818e624ed7b8f2607a3e32fcebeaac4`; CI #10958 confirmou a fixture em Linux/Windows no SHA `25ff40bdfc817185dabcf9065afb9b6c32e293c2`. A matriz de relações par-a-par foi validada no CI #10962 em Linux/Windows no SHA `c6d9f71f989cf9c1ff61fde8a36ea883b82a35ff`. Isso valida os protótipos executáveis, não a relação física B/C; W07 continua BLOCKED.

O emissor do corpus agregado permanece separado: a fixture de divergência não é misturada aos cortes diferenciais que exigem igualdade. Ela é capturada e validada pelo próprio teste de replay, preservando a distinção entre igualdade diferencial e divergência intencional.


### 5.6 Matriz W07 de relações par-a-par — protótipo isolado (não altera P-A/R2)

Após a validação do replay W07 em CI #10958, foi criado o módulo descartável `w07_pairwise_matrix.py` e sua suíte `test_w07_pairwise_matrix.py`. A matriz implementa somente a adjudicação de uma relação **explicitamente fornecida** entre B e C:

| Relação informada para (B,C) | Solicitar B | Solicitar C |
|---|---|---|
| B causa C | Advance | Blocked |
| C causa B | Blocked | Advance |
| B/C comutativos | Advance | Advance |
| Relação UNKNOWN | IndeterminateCausalOrder | IndeterminateCausalOrder |

O módulo não infere a relação a partir de microstep, owner, ordem de inserção ou flags unárias. Ele também não prova que uma relação declarada é verdadeira: a origem e a justificativa da evidência continuam sendo responsabilidade de uma revisão independente. A matriz é um teste de decisão sobre relações já estabelecidas, não um mecanismo de descoberta de causalidade ou prova de comutatividade.

**Isolamento preservado:** `p_a_microsteps.py`, o oráculo aprovado `verification/r2_reference/microstep_reference.py`, R1 v0.3.18 e R2 merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0` não foram alterados. O caso original W07 permanece como observação da divergência P-A/R2. A nova matriz não autoriza substituir a barreira de P-A nem alterar o resultado do oráculo.

**Validação executável:** os seis testes da matriz passaram no CI #10962 em Linux e Windows. Mesmo com PASS, W07 permanece **BLOCKED** até que uma revisão independente defina quais relações podem ser admitidas e qual evidência as sustenta. Em particular, o caso original B/C sem relação demonstrada deve permanecer indeterminado; não se escolhe B<C nem C<B por conveniência de implementação.

## 8. Aditivo de revalidação — HEAD atual e W09 replay após restore

**Data da revalidação:** 2026-11-26
**HEAD revalidado:** `f2e0e83735284d5137c5755b27f8a82d75f6a9d6`
**CI:** [#11012](https://github.com/leonardo201800478/SUN/actions/runs/45813531583) — Run ID `45813531583`, conclusão **SUCCESS**.
**Escopo:** inspeção do parecer, fixture W09 atual, metadados da PR e jobs do CI. Este aditivo atualiza a base de evidência; não aprova gate nem altera R1/R2.

### 8.1 CI no HEAD revalidado

Jobs relevantes concluídos com sucesso:

- CHRONOS R3 disposable prototype tests — Linux e Windows.
- CHRONOS R2 independent reference tests — Linux.
- CHRONOS R2 Python reference tests — Windows.
- CHRONOS R3 canonical and replayable corpus Linux/Windows equivalence.
- GitHub Actions workflow validation.
- Documentation patch hygiene.
- Rust quality and unit regression supervisor.
- Change impact classification.

Foram **9 jobs SUCCESS e 14 SKIPPED**. Os jobs de hardware/runtime/desktop, incluindo SMS1 VDP, scheduler stress, PSG, CPAL, SDL3, Slint/WGPU, Z80 e YM2413, permanecem SKIPPED e não são contabilizados como PASS. Os testes R3 reportam 162 testes por sistema operacional. Esta execução comprova os testes e artefatos declarados no workflow, não a validação dos subsistemas ignorados.

### 8.2 Corpus reexecutável atualizado

O PR registra no HEAD revalidado:

- Corpus canônico: 112 fixtures e 1.475 cortes (1.468 diferenciais + 7 propriedades), digest lógico `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`, SHA-256 idêntico Linux/Windows `2539b0e5115e9d14bca13392405cda729e48471b2e0acaa63f06af1df9e87b9f`.
- Corpus replayável independente: 17 fixtures e 88 operações/cortes, digest lógico `e15bb0dc410b01dd1484b79c80d9918010dd86db665a269fec2bc394ea698f1f`, SHA-256 idêntico Linux/Windows `36407339579b7a35bb88619a26a5cdcbe2f0146b463bb8fc41afcbceaf0b0aaa`.

A igualdade multiplataforma dos artefatos é PASS. Isso não transforma o corpus replayável parcial em cobertura completa de todos os workloads W00–W09. A migração integral permanece pendente.

### 8.3 W09 — replay de eventos após restore

A fixture `W09_REPLAY_EVENTS_AFTER_RESTORE_PRESERVES_ACCEPTED_SEQUENCE` foi inspecionada no arquivo `test_w09_replayable_fixture.py`. A sequência declarativa executa capture, restore e replay sobre um estado sintético SMS-NTSC:

1. O snapshot capturado contém `input_cursor=4`.
2. Após restore, são fornecidos eventos em ordem de tupla 6, 5.
3. O replay deve aplicar a sequência aceita 5, 6, preservando o cursor e avançando-o até 6.
4. CPU e VDP terminam com progresso comum 12.
5. A fixture compara o adaptador P-A com o oráculo independente R2, exige igualdade de todos os cortes e executa replay independente de ambos os lados.

O teste também declara explicitamente que o oráculo R2 constrói um histórico sintético de entradas aceitas alinhado ao cursor do snapshot. Portanto, **não** demonstra que um log histórico físico é armazenado pelo snapshot nem que houve replay de um dispositivo SMS real. Classificação: **OBSERVATION de equivalência sintética P-A/R2**.

O mesmo arquivo mantém a fixture de gerações de deadlines e ledgers parciais de efeitos após restore. Isso amplia a cobertura sintética de W09, mas não substitui traces de runtime nem fecha a cobertura reexecutável integral W00–W09.

### 8.4 Disposição atualizada dos achados

| Achado | Estado no HEAD revalidado | Efeito no gate |
|---|---|---|
| W07 — relação B/C | **BLOCKED**; a matriz só adjudica relações fornecidas e não prova a relação física/lógica | Não fechar W07 |
| W06 — registro externo de fase | **UNKNOWN** como evidência física; validação estrutural não resolve nem autentica o registro | Não alegar fase física SMS |
| Corpus reexecutável W00–W09 | **PARTIAL**; 17 fixtures/88 operações, incluindo W07/W08/W09 | Não fechar requisito integral |
| W09 replay após restore | **PASS** como fixture sintética P-A/R2, com histórico anterior sintético | Evidência limitada; não prova hardware |
| CI #11012 | **PASS** nos jobs executados; 14 jobs SKIPPED | Valida somente o escopo executado |
| P-A closeout | **NOT READY** | R3 continua em andamento |
| P-D/P-E | **NOT RUN / BLOCKED** | Não iniciar sem critérios de entrada resolvidos/revisados |
| R4/R5 | **PENDING** | Sem aprovação arquitetural final |
### 8.5 Conclusão da revalidação

A revisão adversarial foi atualizada para o HEAD `f2e0e83735284d5137c5755b27f8a82d75f6a9d6`. As correções e ampliações recentes do harness, a fixture W09 de replay após restore e o CI #11012 fortalecem a evidência experimental. **Nenhuma delas resolve W07, autentica W06 ou completa o corpus reexecutável W00–W09.**

A decisão permanece: manter a PR #1284 **OPEN/DRAFT**, não declarar P-A fechado, não iniciar P-D/P-E e não introduzir código de produção CHRONOS/V2 antes de R5. R1 v0.3.18 e o oráculo R2 aprovado permanecem preservados.


## 9. Experimento W07 — adjudicação de relações par-a-par no ciclo (27/11/2026)

### 9.1 Objetivo e isolamento

Foi criado o módulo descartável `w07_pairwise_cycle.py`, separado de `p_a_microsteps.py` e de `verification/r2_reference/microstep_reference.py`. O objetivo é testar se a matriz de relações explícitas pode ser aplicada ao ciclo completo, e não somente a uma função isolada de adjudicação.

Cada par não ordenado de obrigações pode declarar uma das relações: primeira causa segunda, segunda causa primeira, comutativas ou UNKNOWN. Um par sem declaração é tratado como UNKNOWN. O número do microstep não preenche nem substitui essa relação.

### 9.2 OBSERVATION — resultados discriminantes

A suíte `test_w07_pairwise_cycle.py` contém seis testes:
1. Após A emitir C enquanto B permanece pendente, B/C UNKNOWN faz qualquer tentativa (B ou C) retornar `IndeterminateCausalOrder`, preservando exatamente o estado.
2. B→C bloqueia C e permite B.
3. C→B bloqueia B e permite C, mesmo C estando no microstep numericamente maior.
4. Comutatividade(B,C) demonstrada permite qualquer uma das duas ordens.
5. Irmãos iniciais sem relação explícita também permanecem indeterminados.
6. Relação duplicada para o mesmo par não ordenado é rejeitada como `InvalidInput`, sem mutação.

A implementação não infere relações a partir de owner, ordem de inserção, microstep ou flags unárias. Relações declaradas para uma obrigação filha podem existir antes de sua emissão; a validação exige IDs não vazios, pares distintos e unicidade do par, mas não exige que a obrigação futura já esteja pendente.

### 9.3 Validação e limite de evidência

**CI #11056:** [run 46032873448](https://github.com/leonardo201800478/SUN/actions/runs/46032873448), HEAD `e562a363e171ec017584b1eb8052c0759d0135d5`, conclusão **SUCCESS**:
- R3 Linux: suite completa, **175 testes PASS**, incluindo os seis novos testes.
- R3 Windows: suite completa, **175 testes PASS**, incluindo os seis novos testes.
- Matrizes W00–W09 Linux/Windows: PASS.
- R2 independente Linux e R2 Python Windows: PASS.
- Equivalência byte a byte dos corpora canônico e replayável: PASS.
- Workflow validation, documentation patch hygiene e Rust quality supervisor: PASS.
- Rust workspace regression e jobs não relacionados: SKIPPED, não contabilizados como PASS.

A primeira execução CI #11054 encontrou quatro falhas de validação no protótipo: relações B/C pré-declaradas eram rejeitadas antes da emissão de C. A validação foi corrigida para aceitar referências declarativas a obrigações futuras; o CI #11056 passou. A falha inicial e sua correção permanecem no histórico.

**Limites:** isto é OBSERVATION sintética de um modelo experimental isolado. O módulo não substitui nem altera P-A; não foi integrado ao fixture replayável W07; não demonstra que uma relação causal ou comutativa real entre B/C exista. No cenário original sem evidência, o resultado correto continua UNKNOWN/Indeterminate. A evidência física SMS permanece UNKNOWN.

### 9.4 Disposição

O experimento fornece uma alternativa concreta para revisão, mas **não fecha W07**. Manter a divergência histórica P-A/R2, não modificar R1 v0.3.18 nem o oráculo R2 aprovado, não declarar P-A fechado e não iniciar P-D/P-E. W06, corpus reexecutável completo, R4/R5 e autorização de produção continuam pendentes.


## 10. W07 — hardening de identidade futura e consistência causal/microstep (27/11/2026)

### 10.1 Achados da revisão do próprio experimento

A inspeção do módulo `w07_pairwise_cycle.py` após CI #11057 encontrou duas lacunas do protótipo:

1. A validação aceitava `PairConstraint` com endpoint que não existia em obrigações pendentes/resolvidas/futuras. Isso permitia typo de identidade passar como estado válido e ser interpretado depois como relação ausente/UNKNOWN.
2. A primeira versão permitia declarar C→B mesmo quando C estava atribuído a microstep maior que B, e o teste anterior chegava a solicitar C antes de B. Isso não demonstrava conformidade com R1 §16.6.2: a sequência de microsteps não pode inverter uma dependência causal estabelecida.

### 10.2 Correção experimental

O estado descartável agora possui `future: tuple[Obligation, ...]`:
- Cada endpoint de relação precisa pertencer a uma obrigação pendente, resolvida ou explicitamente declarada como futura.
- Identidades futuras são únicas e não podem colidir com pendentes/resolvidas.
- Uma obrigação emitida precisa ter sido declarada previamente como futura e corresponder exatamente em identidade, proprietário e microstep.
- Ao emitir, a declaração futura é consumida e a obrigação passa para `pending`.
- Uma relação causal que contradiz a ordem dos microsteps declarados é rejeitada como `InvalidInput`, sem mutação.
- O caso C→B continua testável quando C está em microstep anterior a B; nesse caso, C pode avançar primeiro. A relação é premissa sintética fornecida ao protótipo, não inferência nem evidência de hardware.

Foram acrescentados testes negativos para endpoint desconhecido, emissão não declarada, divergência entre emissão e declaração, colisão de identidade futura e relação causal incompatível com microsteps. O teste C→B foi substituído por dois casos: atribuição contraditória rejeitada e atribuição consistente permitida.

### 10.3 Evidência e estado

CI #11057 — [run 46033234761](https://github.com/leonardo201800478/SUN/actions/runs/46033234761) — **SUCCESS** no HEAD documental anterior `0792315e6276d3912ecb70a72f669c416e721002`. Validou 175 testes R3 em Linux/Windows, R2 Linux/Windows, matrizes, corpora byte-idênticos, workflow validation e higiene documental. Esse resultado antecede o hardening descrito nesta seção e **não** valida os novos commits.

Commits de hardening ainda sujeitos a CI próprio:
- `b08b266dd219c480924761e4e2deda77a92875a5` — registro explícito de obrigações futuras e validação de endpoints.
- `42a9ec410207f6089d584c60d27b95e3614332af` — testes negativos de identidade futura.
- `c1b0baf38f1e5f68d5c2c5d726fc3befdd8b07a8` — rejeição de relação causal incompatível com microsteps.
- `4c58c748f7e7688f4cc31645ca219cb50a8aba26` — testes discriminantes para consistência causal/microstep.

### 10.4 Disposição

O experimento está mais estrito, mas W07 continua **BLOCKED** no caso real B/C sem relação demonstrada. A matriz só decide a partir de premissas explícitas; não estabelece a verdade dessas premissas. Nenhuma alteração em R1 v0.3.18, P-A original, R2 aprovado ou código de produção. Não fechar P-A nem iniciar P-D/P-E. A validação dos commits acima aguarda CI próprio.


## 11. W07 — aderência adicional à sequência de microsteps (27/11/2026)

A segunda revisão do ciclo encontrou mais duas condições que precisavam ser explícitas para respeitar R1 §§16.6.2–16.6.3:

1. Mesmo quando B/C são comutativos, uma obrigação em microstep posterior não pode ultrapassar uma obrigação ainda pendente em microstep anterior neste candidato. A relação comutativa autoriza a sequência lógica; não apaga o cursor de microstep declarado.
2. Uma obrigação emitida durante o despacho não pode ser inserida no mesmo microstep ou em microstep anterior ao da obrigação que a emitiu. Isso impediria a regra explícita de criação no mesmo instante e permitiria retroatividade.

O candidato foi ajustado:
- Ao solicitar obrigação de microstep posterior, relação UNKNOWN com qualquer obrigação anterior retorna `IndeterminateCausalOrder`, sem mutação.
- Se a relação for comutativa, a obrigação anterior é processada primeiro (`Blocked` para a posterior); comutatividade no mesmo microstep permite qualquer membro.
- Relação causal que aponta a obrigação posterior como predecessora de uma obrigação anterior é inválida em relação aos microsteps declarados.
- Obrigações emitidas precisam ter microstep estritamente maior que o da obrigação despachada, além de constar do registro `future` com metadados idênticos.

Foram adicionados testes para: comutatividade entre microsteps diferentes respeitando a sequência; comutatividade no mesmo microstep; e rejeição de filho emitido no mesmo microstep. A suíte isolada `test_w07_pairwise_cycle.py` agora contém 13 testes. Esses resultados ainda aguardam CI próprio no HEAD final desta atualização.

**Leitura normativa:** isso aplica a regra de que microstep é ordem lógica de software, só pode ordenar dependências estabelecidas ou efeitos comutativos, e um evento criado durante despacho segue regra explícita sem inserção retroativa. Não escolhe fase física nem prova que B/C sejam causalmente relacionados/comutativos no SMS.

O caso W07 original, B/C sem relação demonstrada, continua `IndeterminateCausalOrder` em qualquer tentativa, preservando o estado. W07 permanece **BLOCKED** para o perfil físico; P-A não fechado e P-D/P-E não iniciados.


## 12. Validação do hardening W07 — CI #11067 (27/11/2026)

**HEAD validado:** `926812ba7a4d442dbadb36df83791d4028a80702`.
**CI:** [#11067 / run 46034036821](https://github.com/leonardo201800478/SUN/actions/runs/46034036821) — **SUCCESS**.

- CHRONOS R3 disposable prototype tests: **182/182 PASS em Linux e 182/182 PASS em Windows**.
- Os 13 testes de `test_w07_pairwise_cycle.py` passaram nos dois sistemas.
- R2 independente Linux e R2 Python Windows: PASS.
- Matrizes sintéticas W00–W09: PASS em Linux e Windows.
- Emissão do corpus canônico e replayável: PASS.
- Equivalência byte a byte dos corpora canônico e replayável Linux/Windows: PASS.
- Workflow validation, documentation patch hygiene e Rust quality supervisor: PASS.
- Rust workspace regression e jobs de hardware/runtime/desktop não relacionados: SKIPPED, não contabilizados como PASS.

**Artefatos emitidos pelo harness:**
- Corpus canônico: 112 fixtures, 1.475 cortes (1.468 diferenciais + 7 propriedades), digest lógico `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.
- Corpus replayável: 24 fixtures, 131 operações/cortes, duas divergências preservadas (W06 e W07), digest lógico `07551ec877a58597180470045ad24bd51444124d469b2dc2efdcc4e3d9dcba7e`.
- O emissor/replay do corpus executou 169 testes. Os 13 testes adicionais do candidato par-a-par não são incorporados ao corpus W07 replayável nem alteram os 24 fixtures/131 cortes.

**Limite:** CI #11067 comprova os testes sintéticos e a igualdade dos artefatos gerados. Não valida relação física B/C, não autentica W06, não integra o novo ciclo ao P-A, não fecha o corpus reexecutável W00–W09 e não aprova P-A/R3 como gate. W07 permanece BLOCKED para o perfil físico; W06 físico permanece UNKNOWN; P-D/P-E não iniciados; R4/R5 pendentes.


## 13. W06 — isolamento da variável de comparabilidade (27/11/2026)

A fixture W06 foi refinada para não variar simultaneamente a base de fase e o rótulo de evidência por evento. Os dois deadlines agora:
- mantêm `phase_order` distintos;
- usam exatamente o mesmo `phase_evidence` sintético (`shared-synthetic-capture`);
- declaram `APhaseBasis` estruturalmente válida, porém diferente (perfil/base A versus perfil/base B).

Assim, a divergência observada isola a comparabilidade da base: P-A retorna `IndeterminateCausalOrder` sem fechar W nem reivindicar deadlines; R2 continua aceitando a ordem numérica. O R2 aprovado não possui campo `phase_basis` e seu predicado de ordenação não compara uma base estruturada compartilhada.

A alteração é uma melhoria do desenho experimental, não uma mudança de semântica. Os placeholders continuam sintéticos e não autenticam qualquer medição ou perfil SMS. A nova fixture/corpus precisa de CI próprio; o CI #11067 validou a versão anterior da fixture e não valida esta alteração.


## 14. W06 — controle positivo com base compartilhada e CI #11072 (27/11/2026)

Foi adicionada à fixture replayável W06 uma segunda condição, positiva, para complementar o caso negativo de bases distintas:

- dois deadlines simultâneos e não comutativos;
- `phase_order` distinto;
- `phase_evidence` distinto por evento;
- uma única `APhaseBasis` sintética compartilhada;
- fechamento do watermark em 10.

O teste exige que P-A e R2 aceitem a ordenação, reivindiquem ambos os deadlines, avancem o watermark até 10 e possam reproduzir independentemente os três cortes. Os rótulos de evidência por evento são deliberadamente distintos: a comparabilidade vem da base compartilhada, não da igualdade textual dos rótulos.

### Validação

**HEAD:** `afbdf337da80243d05202163fcc7d8412dfe0db8`
**CI #11072:** [run 46060833047](https://github.com/leonardo201800478/SUN/actions/runs/46060833047) — **SUCCESS**.

- R3 descartável: **183/183 testes PASS em Linux e 183/183 em Windows**.
- O teste W06 negativo (bases distintas/evidência compartilhada) e o controle positivo (base compartilhada/rótulos distintos) passaram em ambos os sistemas.
- Emissor do corpus: **170 testes PASS** em cada sistema.
- Corpus canônico: **112 fixtures / 1.475 cortes** (1.468 diferenciais + 7 propriedades), digest lógico `fefc49aaf5a61f655991214008dbc84073ac4c8709cf4ac1db573bdcf95909bc`.
- Corpus replayável: **25 fixtures / 134 operações e cortes**, duas divergências preservadas (W06 negativo e W07), digest lógico `2dcf09eeaf6ffe03e6bcf1cfda087aeedeb865d351b6ac081a0dc12ff28bc28f`.
- Os JSONs dos corpora canônico e replayável foram comparados byte a byte entre Linux e Windows: **PASS**.
- R2 independente Linux e R2 Python Windows: PASS.
- Matrizes W00–W09: PASS em Linux e Windows.
- Workflow validation, documentation patch hygiene e Rust quality supervisor: PASS.
- Rust workspace regression e jobs de hardware/runtime/desktop/áudio/vídeo/Z80 não relacionados: SKIPPED, não contabilizados como PASS.

**Limite de benchmark:** em Windows, as medições de CPU time que não têm resolução suficiente foram marcadas `SKIPPED_COARSE_RESOLUTION`; os pontos de teste/matriz ainda passaram, mas isso não constitui medida de performance comparável. Os resultados são de protótipos Python sintéticos, não de CHRONOS em Rust nem de hardware SMS.

### Interpretação e disposição

O par de fixtures distingue dois casos do predicado de comparabilidade:
1. Bases sintéticas diferentes, mesmo rótulo de evidência e ordens distintas: P-A mantém `IndeterminateCausalOrder`; R2 aceita pela ordenação numérica, preservando a divergência.
2. Base sintética comum, rótulos de evidência por evento distintos e ordens distintas: P-A/R2 concordam e aceitam.

Isso aumenta a discriminação do experimento e fecha a lacuna de cobertura positiva da fixture; **não fecha a lacuna semântica do oráculo R2**, não valida a autenticidade/validade física da base e não decide qual ordenação ocorre no SMS. Todos os identificadores, registros e hashes são sintéticos. Evidência física/externa W06 permanece **UNKNOWN**. R2 aprovado permanece imutável; P-A não está fechado; P-D/P-E NOT RUN; R4/R5 pendentes.

## 15. W07 — matriz negativa estrutural de PairwiseCycle (27/11/2026)

### 15.1 Escopo e resultado

A revisão parcial publicada no comentário `7228796915` recomendou sistematizar entradas estruturalmente inválidas para `_validate`. A suíte isolada foi ampliada de 15 para **17 testes** em `test_w07_pairwise_cycle.py`, sem alterar `w07_pairwise_cycle.py` nem introduzir novas regras temporais.

A matriz acrescentada cobre:
- `pending`, `constraints`, `resolved` e `future` com tipos de coleção incorretos;
- itens inválidos em coleções e ledger `resolved` malformado;
- identidades repetidas e conflitos entre `pending`, `resolved` e `future`;
- declarações futuras com microstep booleano/negativo;
- enumeração de relação inválida;
- endpoints autorreferentes, não declarados ou com tipo inválido;
- tipos inválidos de `emitted` e identidade solicitada malformada.

Cada entrada negativa exige `InvalidInput` e preservação por identidade (`assertIs`) do estado original. Os testes não impõem restrições novas além da estrutura já exigida pelo próprio protótipo e não inferem causalidade a partir de microstep.

### 15.2 Validação vinculada ao SHA

**Commit de testes:** `d36c0966a6a9f5abf94872c769c62b29453e8517`.
**CI #11079:** [run 46074483099](https://github.com/leonardo201800478/SUN/actions/runs/46074483099) — **SUCCESS**.

- R3 disposable suite: **187/187 PASS Linux e 187/187 PASS Windows**.
- W07 pairwise-cycle: **17/17 PASS** em ambos os sistemas.
- R2 independente Linux e R2 Python Windows: PASS.
- Corpus canônico/replayável: equivalência byte a byte Linux/Windows PASS.
- Documentation patch hygiene, workflow validation, change impact classification e Rust quality supervisor: PASS.
- Rust workspace regression e jobs de hardware/runtime/desktop não relacionados: SKIPPED, não contabilizados como PASS.

### 15.3 Disposição

A recomendação de cobertura estrutural foi atendida e validada no SHA indicado. Nenhum defeito acionável no `_validate` foi demonstrado pela matriz; as entradas inválidas são rejeitadas sem mutação. Isso não prova relação física B/C, não fecha W07 físico, não fecha P-A e não autoriza P-D/P-E. W06 físico permanece UNKNOWN; P-D/P-E NOT RUN; R4/R5 pendentes. R1 v0.3.18, R2 aprovado e P-A baseline original permanecem preservados. Nenhum código de produção CHRONOS/V2 foi introduzido.


## 16. W07 — conclusão da matriz negativa estrutural e limite da política de emissão (27/11/2026)

### 16.1 Validação final

A matriz estrutural foi ampliada de 17 para **18 testes** no módulo `test_w07_pairwise_cycle.py`. A última adição cobre instância de ciclo inválida, tipo inválido de `at_common`, descritores pendentes/futuros com campos malformados e endpoint de relação com tipo inválido. Todos os casos negativos exigem `InvalidInput` e preservação por identidade do estado original.

**HEAD validado:** `0f917ed9b5b26c1862623b9c14a6812e82183d0c`.
**CI #11083:** [run 46074762710](https://github.com/leonardo201800478/SUN/actions/runs/46074762710) — **SUCCESS**.

- R3 disposable suite: **188/188 PASS Linux e 188/188 PASS Windows**.
- W07 pairwise-cycle: **18/18 PASS** em ambos os sistemas.
- Emissor/replay do corpus: 170 testes PASS em ambos.
- R2 independente Linux e R2 Python Windows: PASS.
- Equivalência byte a byte dos corpora canônico/replayável Linux/Windows: PASS.
- Documentation patch hygiene, workflow validation, change impact classification e Rust quality supervisor: PASS.
- Rust workspace regression e jobs não relacionados de hardware/runtime/desktop: SKIPPED, não contabilizados como PASS.

### 16.2 Classificação da regra de emissão

O protótipo exige que um filho emitido use `microstep` estritamente maior que o da obrigação emissora. Essa condição é uma **política candidata do experimento**, escolhida para evitar inserção retroativa. R1 §16.6.3 exige uma regra explícita para processar eventos criados no mesmo instante e proíbe inserção retroativa antes de efeito confirmado; o texto não determina que avanço numérico estrito seja a única regra admissível. Uma política que anexasse o filho depois do emissor no mesmo microstep não foi testada.

Portanto, a exigência de microstep estritamente maior deve permanecer classificada como **HYPOTHESIS / INFERENCE (política do candidato)**, não como FACT normativo de R1. O teste atual valida a política escolhida, não prova exclusividade nem generalidade. Nenhum resultado W07 físico pode depender dessa escolha sem cenário discriminante e revisão próprios.

### 16.3 Disposição

A recomendação de cobertura negativa estrutural está atendida e validada no HEAD indicado; nenhum defeito acionável em `_validate` foi demonstrado pela matriz. A política de emissão estritamente posterior permanece uma escolha experimental explicitamente limitada. W07 físico continua BLOCKED porque a relação B/C real não foi demonstrada; W06 físico permanece UNKNOWN. P-A não está fechado, P-D/P-E NOT RUN, R4/R5 pendentes. R1 v0.3.18, R2 aprovado e P-A baseline original permanecem preservados; nenhum código de produção CHRONOS/V2 foi introduzido.


## 17. Resolução dos sete achados adversariais da PR #1284 (27/11/2026)

**HEAD validado:** `6e99758af2d11d3bbe398240b294d7e28efc586a`.
**CI #11093:** [run 46083978800](https://github.com/leonardo201800478/SUN/actions/runs/46083978800) — **SUCCESS**.

### 17.1 P1 — snapshot, replay e watermark

- **Gerações no snapshot:** `_validate_state` identifica deadline por `(identity, generation)`; geração 0 cancelada e geração 1 agendada são preservadas no round-trip. Duplicidade do mesmo par segue rejeitada.
- **Fronteira de replay:** `replay_from_snapshot` calcula `A(E)` uma vez no corte inicial do lote aceito, como máximo de W (se existir) e de todos os progressos de domínio presentes no snapshot. Timestamps `<= A(E)` retornam `LateInput` sem mutação. Os registros são então aplicados em ordem de sequência; o progresso produzido pela aplicação de um registro não re-admite/rejeita outro registro do mesmo lote já aceito. A fixture W09 foi mantida concordante com o replay em lote do R2.
- **Watermark entre microsteps:** `_causal_order_known` avalia o grupo completo no mesmo instante, não cada nível isoladamente. Nível de microstep não cria causalidade. Para grupo não comutativo, todos os membros precisam de uma phase basis validada comum e `phase_order` distinto; sem isso, `close_watermark` retorna `IndeterminateCausalOrder` sem mutação. Grupos integralmente comutativos usam ordem determinística por identidade, sem alegar precedência causal.

### 17.2 P2 — fase C=D, canonicalidade, limite W07 e documentação

- **C=D:** `ACancellation` carrega `phase_basis`; comparação de `phase_order` requer base válida e igual à do deadline. A evidência textual pode ter rótulos diferentes quando a base estruturada é comparável; sem base, o resultado é indeterminado.
- **Restore:** payload decodificado precisa reserializar para os mesmos bytes recebidos. JSON reformatado e frações não reduzidas são rejeitados mesmo se o digest tiver sido recalculado.
- **W07:** `PairwiseCycle` tem limite finito experimental `max_microsteps=32` (nível máximo inclusivo). A fronteira é aceita; emissão acima do limite retorna `ResourceExhausted` sem mutação. Testes cobrem fronteira, excesso e limite inválido. Esse protótipo continua restrito à adjudicação de relações explícitas; não é um ciclo completo de hardware, não prova relação B/C e não torna o valor 32 normativo para R1.
- **Documentação:** a fonte de módulos replayáveis W04–W06 foi corrigida; 131 testes enumerados são subconjunto, enquanto o runner completo executa 197 (176 no emissor + 21 W07 pairwise).

### 17.3 Evidência de validação

- R3: **197/197 PASS Linux e 197/197 PASS Windows**.
- Emissor de corpus: **176 testes PASS**; W07 pairwise isolado: **21 testes PASS**.
- R2 independente Linux e R2 Python Windows: PASS.
- Sete matrizes sintéticas P-A (75 + 180 + 30 + 24 + 12 + 10 + 20 pontos por sistema): PASS.
- Corpus canônico: **117 fixtures / 1.481 cortes** (1.470 diferenciais + 11 propriedades), digest lógico `ca73c8720f6b9b81454ff5e5f7e67f0aab1c49b71e24032ab4f6edcf74465181`.
- Corpus replayável: **25 fixtures / 134 operações-cortes**, divergências W06/W07 preservadas, digest lógico `2dcf09eeaf6ffe03e6bcf1cfda087aeedeb865d351b6ac081a0dc12ff28bc28f`.
- Artefatos canônico/replayável byte a byte idênticos Linux/Windows; workflow validation, documentation hygiene, change-impact classification e Rust quality supervisor: PASS.
- CPU time Windows: `SKIPPED_COARSE_RESOLUTION`; hardware/runtime/desktop jobs: SKIPPED. Nenhum resultado de benchmark Python é desempenho Rust ou evidência de hardware.

### 17.4 Disposição

Os sete achados P1/P2 foram corrigidos no escopo R3. R1 v0.3.18 e R2 aprovado permanecem imutáveis; nenhum crate Rust de produção/runtime/desktop foi alterado. W06 físico permanece UNKNOWN e W07 físico B/C permanece BLOCKED. P-A continua sem closeout, P-D/P-E NOT RUN/BLOCKED, R3 em andamento e R4/R5 pendentes. O CI não aprova o gate nem valida 60 FPS. A revisão independente formal da PR ainda é condição separada para merge.


### 17.5 Follow-up to the independent final review (new HEAD; CI pending)

The three P2 findings from the final review were addressed, and the review's P3 reproducibility observation was also corrected:

1. **W07 limit continuity:** `process()` carries `state.max_microsteps` into the returned `PairwiseCycle`. The regression test now performs a real sequence across returned states with a non-default limit of 2: one transition emits microstep 1, the next reaches microstep 2, and a later attempt to emit microstep 3 returns `ResourceExhausted` atomically.
2. **Closed watermark:** snapshot validation rejects a `Scheduled` deadline whenever `D <= W`; capture and restore tests cover both `D < W` and `D = W` and verify restore atomicity.
3. **Single effect-capable generation:** only one generation per identity may remain capable of producing an effect. This includes `Scheduled`, `Claimed`, and `Dispatched`; `Failed` is included while pending subeffects remain recoverable. `Cancelled`, `Superseded`, and `Completed` remain valid historical records. Capture/restore tests exercise a newer scheduled generation while the previous generation is claimed, dispatched, or failed with pending subeffects.
4. **Python reproducibility:** the Linux R3 job now pins Python 3.14.4 through the same setup-python action/version used by Windows, and invokes that interpreter consistently. This removes the previously reported runner-default version mismatch; runner/OS performance differences remain.

**Validation status:** the earlier CI #11095 and its 197/197 result are historical and do not validate these follow-up commits. The new PR HEAD must complete the full applicable CI before the corrections are marked validated. R1 v0.3.18 and R2 remain unchanged; no production code is touched. W06 physical UNKNOWN; W07 physical B/C BLOCKED; P-A no closeout; P-D/P-E NOT RUN/BLOCKED; R3 in progress; R4/R5 pending.
