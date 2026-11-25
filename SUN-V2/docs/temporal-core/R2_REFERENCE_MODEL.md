# CHRONOS — modelo de referência independente R2

**Projeto:** SUN V2
**Sistema inicial:** Sega Master System (SMS)
**Gate:** R2 — modelo de referência independente
**Status:** R2-A em implementação inicial; núcleo de referência e testes R2-T01–T04 parciais; R2 em andamento; não aprovado
**Issue:** [#1282 — modelo de referência independente do CHRONOS](https://github.com/leonardo201800478/SUN/issues/1282)
**Contrato normativo:** [R1 v0.3.18](R1_TEMPORAL_CONTRACTS.md), conteúdo aprovado no SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76`, publicado em `main` pelo merge `88e105d6495f7f5105502e53666a7b0d29f77572`

> Este documento define o oráculo de verificação do R2. Não escolhe nem implementa o algoritmo de produção CHRONOS, não cria crate de runtime e não autoriza código de produção antes de R5.

---

## 1. Objetivo e fronteira do gate

R2 transforma os contratos normativos R1 em um **modelo de referência executável, simples, lento e independente**. O oráculo será usado para verificar propriedades, transições, fronteiras e resultados sem compartilhar o algoritmo de agenda ou a representação temporal que poderão ser avaliados em R3.

R2 não escolhe o desenho final do CHRONOS. A proposta de híbrido mínimo permanece uma hipótese arquitetural documentada; a comparação de alternativas pertence a R3, e a aprovação arquitetural final pertence a R5.

### Entradas

- Estado inicial explícito e serializável do modelo.
- Perfil temporal identificado (inicialmente SMS NTSC; outros perfis somente quando cada cenário os nomear).
- Conjunto completo de domínios participantes, quando exigido.
- Sequência ordenada de comandos externos e ações internas.
- Relações causais/fases apenas quando declaradas pelo contrato ou pela evidência do perfil.
- Seed fixa para geração determinística de sequências.

### Saídas

- Resultado normativo da operação: `Reached`, `StoppedAt`, `Blocked`, `Failed`, `LateInput`, `InvalidInput`, `InvariantViolation`, `IndeterminateCausalOrder` ou `UnsupportedSnapshot`, conforme a operação.
- Novo estado lógico, ou prova de ausência de mutação nos erros que assim exigem.
- Ledger de eventos, deadlines, transações e efeitos.
- Primeiro ponto de divergência, se houver.
- Trace mínimo reproduzível com perfil, seed, comandos e estado inicial.

## 2. Princípio de independência

O oráculo MUST ser independente do algoritmo/representação candidata de produção e dos protótipos R3.

A independência será demonstrada por estas decisões:

1. **Sem agenda heap/priority queue:** o modelo mantém uma coleção declarativa de obrigações e procura candidatos por varredura completa. A complexidade pode ser alta; desempenho não é objetivo do R2.
2. **Sem dispatch concorrente:** cada transição é uma função lógica serial e determinística. Threads, locks, atomics, callbacks e scheduling do host não participam.
3. **Sem representação de runtime compartilhada:** o modelo usa aritmética exata de referência e estado explícito, sem reutilizar tipos, crates, codecs ou APIs da futura implementação.
4. **Sem cópia da V1:** nenhuma dependência de path, estrutura, scheduler ou implementação V1 será importada.
5. **Sem inferência física:** fase UNKNOWN, WAIT externo UNKNOWN, presença/commit UNKNOWN e relações simultâneas não demonstradas permanecem indeterminadas ou restringem a operação.
6. **Sem otimização prematura:** clareza, auditabilidade e capacidade de produzir contraexemplos prevalecem sobre throughput.

### 2.1 Tecnologia do oráculo

A implementação de referência será um artefato de verificação isolado em Python, usando somente a biblioteca padrão:

- `fractions.Fraction` para razões e coordenadas racionais exatas;
- inteiros arbitrários para contadores e identidades;
- `dataclasses`/tuplas imutáveis para estados e resultados;
- `enum` para classes de resultado e estados normativos;
- `unittest` para asserções executáveis;
- `random.Random(seed)` para geração determinística reproduzível.

O modelo Python **não é backend, runtime, dependência de build ou código de produção**. Ele existe para ser estruturalmente independente do backend Rust futuro. Não será ligado ao desktop, GUI, áudio ou vídeo. Não introduzir dependências de terceiros no primeiro incremento.

A escolha de Python é uma decisão de implementação do oráculo, não uma escolha de representação temporal de produção. Se uma limitação demonstrável impedir a prova de alguma propriedade, documentar a lacuna antes de adicionar dependência ou trocar o modelo.

## 3. Modelo matemático de estado

O estado de referência é um valor lógico explícito. A implementação pode usar records imutáveis ou cópias completas; não deve esconder transições em callbacks.

### 3.1 Domínio temporal

Cada domínio (d) declara:

- razão exata entre unidades locais e tempo comum;
- offset/fase inicial somente quando conhecido;
- progresso local (P_d);
- horizonte seguro (H_d);
- resíduo de conversão necessário para preservar equivalência;
- validade da conversão e perfil a que pertence.

A conversão para tempo comum é uma função exata. O oráculo não usa float, arredondamento binário nem tolerância epsilon. Uma razão não conversível, participante ausente ou perfil incompatível não pode ser substituído por estimativa.

### 3.2 Watermark e conjunto de participantes

(W) é o corte causal global confirmado. Ele é monotônico e só fecha quando todas as obrigações até o corte foram processadas, canceladas validamente ou demonstradas sem efeito.

O conjunto de participantes é explícito por cenário. Para calcular (A(E)), o modelo primeiro valida a completude do conjunto e todas as conversões (P_d) para a linha temporal comum; somente depois calcula o máximo. É proibido produzir máximo parcial.

- conjunto incompleto: `Blocked`;
- conversão inválida: `IndeterminateCausalOrder`;
- nenhum estado parcial é publicado como se fosse um máximo válido.

### 3.3 Relação causal parcial

A comparação de duas obrigações retorna uma relação explícita:

- `Before`;
- `After`;
- `SameKnownPhase` ou relação equivalente comprovada;
- `CommutativeAtSameInstant`;
- `Unknown`.

O modelo não totaliza uma relação UNKNOWN usando ordem de inserção. Para efeitos não comutativos simultâneos sem relação física demonstrada, não escolhe vencedor, não fecha (W) através do conflito e retorna a indeterminação normativa aplicável.

Uma sequência estável pode ordenar a apresentação do trace ou operações comprovadamente comutativas; nunca pode ser interpretada como precedência elétrica.

### 3.4 Obrigações e deadlines

Cada obrigação contém proprietário, identidade estável, geração, instante causal, classe de efeito e estado. Cancelamento/substituição invalida a geração anterior. A varredura integral do conjunto de obrigações substitui qualquer agenda eficiente: o oráculo escolhe somente obrigações cuja precedência esteja demonstrada.

A corrida cancelamento (C<D) segue o corte causal aprovado em R1: cancelamento admitido deve ser arbitrado antes de (W) fechar (D); atraso operacional de dispatch não muda o resultado lógico. O modelo registra separadamente admissão, arbitragem, claim e aplicação do efeito.

### 3.5 Transações

A máquina de estados segue literalmente R1 §7.2–§7.4:

- Criada;
- Em andamento;
- Aguardando;
- Pronta;
- Concluída;
- Cancelada;
- Falha.

Amostragem e commit são marcadores independentes e monotônicos. Cada efeito irreversível possui identidade e ledger. Retry idêntico devolve o estado/resultado registrado; não repete efeito. Recuperação de Falha é comando explícito distinto. Cancelada permanece terminal.

### 3.6 WAIT e interrupções

WAIT automático Z80, WAIT externo e WAIT de fixture são classes distintas. O oráculo só consome TW e reamostra nos pontos documentados do cenário. WAIT externo UNKNOWN não vira inativo, não injeta TW e não inventa ponto de amostragem.

IRQ/NMI são modeladas como níveis/transições e pontos lógicos de observação declarados. EI delay, borda NMI, acknowledge, vetor e efeito de dispositivo são cenários separados. `ACK_DEVICE_COMMIT(profile, device)` permanece específico do perfil/dispositivo. Se presença ou fronteira de commit for UNKNOWN na janela ambígua, snapshot retorna `UnsupportedSnapshot`, sem artefato e sem mutação.

### 3.7 Snapshot e replay

Snapshot é permitido somente em corte suportado e serializável. O estado de referência inclui obrigações, deadlines e gerações, transações, marcadores, valores amostrados, ledger de efeitos e condições de WAIT.

Em corte ambíguo não suportado:
- resultado `UnsupportedSnapshot`;
- nenhum artefato parcial;
- estado anterior byte/logicamente equivalente após a chamada;
- nenhuma mutação ou efeito colateral.

Replay compara execução contínua com restauração de snapshot suportado e repetição da mesma sequência de entradas. O modelo exige equivalência de estado observável e ledger, não igualdade de custo interno.

## 4. Operações abstratas do oráculo

As operações são comandos de um interpretador de referência, não APIs CHRONOS:

| Comando abstrato | Efeito de referência |
|---|---|
| `Advance(target)` | Avalia progresso, horizonte, watermark e obrigações; aplica somente transições causais autorizadas e devolve classe de resultado/progresso real |
| `Synchronize(domain, target)` | Resolve obrigações próprias/causais do domínio até o alvo permitido, sem ultrapassar fronteira pendente |
| `PublishDeadline(owner, id, generation, instant)` | Valida proprietário, geração, (P_d), (W), microstep e causalidade antes de admitir |
| `CancelDeadline(id, generation, causal_instant)` | Valida identidade/geração e corte de admissão; arbitra antes do fechamento de (W) quando (C<D) |
| `BeginTransaction(descriptor)` | Admite descritor e cria estado sem amostragem/commit prematuros |
| `Sample(marker)` | Captura o valor uma vez na fronteira declarada e grava o resultado |
| `Commit(marker)` | Aplica o efeito uma vez na fronteira declarada e grava o ledger |
| `Resume(transaction_id)` | Continua do primeiro marco pendente, sem repetir os anteriores |
| `Snapshot(cut)` | Produz snapshot completo somente para corte suportado |
| `Restore(snapshot)` | Restaura o corte exato, incluindo pendências e gerações |
| `Replay(inputs)` | Reexecuta a mesma sequência lógica e compara estado/ledger |
| `ObserveHost(action)` | Simula atraso, descarte, drain ou underrun de saída sem tocar no estado virtual |

Cada comando retorna o estado anterior e posterior (ou hash canônico), resultado normativo, eventos/efeitos adicionados e motivo de rejeição. Operações que R1 exige rejeitar sem mutação MUST comprovar igualdade do estado antes/depois.

## 5. Matriz de propriedades executáveis

A matriz R1 §13 é a fonte normativa detalhada. Este plano define o que o oráculo deverá provar em cada propriedade:

| Propriedade | Núcleo de verificação | Contraexemplos/famílias obrigatórias |
|---|---|---|
| R2-T01 Monotonicidade | (P_d), (H_d), (W), avanço e rejeição retroativa | avanço crescente/repetido/retroativo; regressão interna; igualdade idempotente; ausência de mutação |
| R2-T02 Chunk invariance | Equivalência entre avanço único e partições permitidas | cortes aleatórios; partição na fronteira; partição sem nova entrada/observação |
| R2-T03 Conversão sem deriva | Conversão exata e conservação de resto/fase | razões inteiras/não inteiras; conversão encadeada; sessões longas; overflow |
| R2-T04 Limite seguro | `Reached`/ `StoppedAt` / `Blocked` e H_d inclusivo | deadline anterior; T=H_d com obrigação pendente; B=T; B<T; domínio bloqueado; W não cruza efeito pendente |
| R2-T05 Sincronização causal | Observação só após sincronizar destino ao ponto de amostragem/commit | leitura de contador/status; escrita de porta; IRQ publicada antes da observação |
| R2-T06 Ordenação e cancelamento | Relação causal parcial, admissão e arbitragem | D<P_d; D<=W; D=P_d com/sem microstep; conjunto incompleto; conversão inválida; C<D; empate; permutações de publicação/dispatch |
| R2-T07 UNKNOWN | Nenhuma precedência inventada | eventos não comutativos simultâneos sem fase; W não fecha; resultado indeterminado |
| R2-T08 Exactly-once | Máquina de estados, amostragem, commit e ledger | toda transição válida/inválida; retry idêntico; ID divergente; falha antes/depois de cada efeito; recuperação explícita |
| R2-T09 WAIT | Separação de TW automático, WAIT externo e fixture | ativo/inativo conhecido; reamostragem; UNKNOWN sem TW especulativo |
| R2-T10 IRQ/NMI | Pontos de observação, EI delay, borda NMI e acknowledge | níveis antes/depois; perfil NONE; efeito/commit conhecido; snapshot UNKNOWN -> UnsupportedSnapshot |
| R2-T11 Snapshot/restore | Continuação equivalente e invalidação de geração obsoleta | todos os estados de deadline/transação; entre amostragem e commit; INT acknowledge; UnsupportedSnapshot sem mutação |
| R2-T12 Replay/admissão | Replay contínuo vs restore; cálculo integral de (A(E)) | (T<=A(E)), (T>A(E)), igualdade idempotente vs nova entrada; conjunto incompleto; conversão inválida; recálculo de H_d |
| R2-T13 Host independence | Ação do host não altera tempo/estado virtual | atraso, drain, descarte, underrun, tamanhos de bloco diferentes |
| R2-T14 Microstep finito | Progresso lógico sem loop infinito | evento gera evento no mesmo instante; ciclo sem progresso; limite excedido; permutação comutativa |
| R2-T15 Overflow/erro/atomicidade | Classes de erro e ausência de mutação parcial | overflow, IDs/perfis inválidos, precedência de validação, capacidade, transição inválida, cancelamento tardio, snapshot UNKNOWN, falha parcial |

Cada teste terá identificador estável, contrato(s) R1 coberto(s), pré-condição, sequência de comandos, resultado esperado, invariantes pós-condição e evidência produzida. Não basta ter um teste nominal por propriedade.

## 6. Estratégia de geração e redução de cenários

1. Manter um catálogo explícito de casos de fronteira determinísticos antes da geração aleatória.
2. Cada gerador recebe seed obrigatória e parâmetros limitados; a mesma seed deve gerar a mesma sequência.
3. Ao falhar, registrar seed, perfil, sequência completa, primeiro passo divergente e estados anterior/posterior.
4. Aplicar redução determinística por remoção de comandos e simplificação de valores, preservando a falha.
5. Nunca reduzir removendo a condição que torna a relação causal UNKNOWN ou que caracteriza a fronteira testada.
6. Separar falhas do oráculo (contradição/incompletude do contrato) de falhas do gerador e de violações esperadas da propriedade.

## 7. Evidência e relatório de execução

Cada execução deve registrar:

- commit SHA do repositório e hash do oráculo;
- versão do Python e sistema operacional;
- perfil de hardware/clock explicitamente modelado;
- ID da propriedade, caso, seed e parâmetros;
- resultado esperado/observado;
- primeiro ponto divergente;
- estado mínimo reproduzível;
- limitações, UNKNOWNs e qualquer hipótese usada.

Resultados possíveis: `PASS`, `FAIL`, `BLOCKED`, `NOT RUN`. `SKIPPED` nunca é convertido em `PASS`. Uma propriedade só pode ser marcada PASS quando todos os seus casos obrigatórios aplicáveis forem executados e seus resultados preservarem o contrato.

## 8. Sequência de implementação do R2

### R2-A — núcleo matemático mínimo
- tipos de valor/resultado e estado imutável;
- relógios/domínios com conversão exata;
- relação causal parcial;
- cálculo de (A(E)) com conjunto completo;
- varredura linear de obrigações, sem heap;
- operações Advance/Synchronize;
- R2-T01–T04.

### R2-B — eventos e deadlines
- proprietário, identidade, geração;
- publicação, cancelamento e substituição;
- arbitragem C<D antes de fechamento de W;
- simultaneidade, UNKNOWN e microsteps;
- R2-T06, T07 e T14.

### R2-C — transações, WAIT e interrupções
- máquina de estados e ledger exactly-once;
- WAIT em classes separadas;
- IRQ/NMI, acknowledge e efeito de dispositivo por perfil;
- R2-T05, T08, T09 e T10.

### R2-D — persistência e robustez
- snapshot/restore dos cortes suportados;
- UnsupportedSnapshot sem artefato/mutação;
- replay, overflow, erro e atomicidade;
- host independence;
- R2-T11–T13 e T15.

### R2-E — fechamento e revisão
- executar a matriz R2-T01–T15;
- gerar pacote de reprodução e matriz de cobertura;
- revisão independente adversarial;
- registrar limitações e critérios não satisfeitos;
- solicitar aprovação formal R2.

A ordem pode ser refinada se uma dependência lógica demonstrada exigir, mas nenhuma propriedade pode ser silenciosamente omitida.

## 8.1 Primeiro incremento executável — R2-A

Foi criado o oráculo isolado em `SUN-V2/verification/r2_reference/`:

- `chronos_reference.py`: estado imutável, domínios de clock, conversão racional exata, conjunto explícito de participantes, obrigações e avanço por varredura integral;
- `test_temporal_core.py`: testes iniciais de monotonicidade, chunk invariance, conversão exata e limite seguro;
- `README.md`: instruções de execução e limites do artefato.

**Evidência histórica anterior (não vinculada):** foram reportados 19 testes PASS em Python 3.13.5 / Linux x86_64, mas a execução original não registrou o SHA do repositório nem o SHA-256 do oráculo. Conforme revisão Codex, essa execução é evidência local não auditável e **não pode ser atribuída ao HEAD atual**.

**Execução reproduzível vinculada — PASS:** o runner foi executado no CI #10619 sobre o candidato abaixo. Os arquivos do oráculo, dos testes e do runner não foram alterados após essa execução.

- SHA Git executado: `a0c0782ff4d8e9886144b20bde36e42bcc6f1eff`.
- SHA-256 de `chronos_reference.py`: `160489b753575f0e7fcb12892eb58f7199a1dc5828c5bb4f13c266f557ba001a`.
- Python: `3.14.4`.
- Ambiente: `Linux-7.0.0-1020-azure-x86_64-with-glibc2.43`.
- Perfil abstrato: `SMS-NTSC`.
- Comando: `python3 run_reference_tests.py`.
- Resultado agregado: **24 tests PASS**, execução em 0.159 s.

| Propriedade | Casos executados | Resultado |
|---|---:|---|
| R2-T01 | 9 | PASS — inclui conversão inválida, progresso float e proprietário obrigatório |
| R2-T02 | 4 | PASS — inclui paridade em H_d e obrigação pendente, com continuação após resolução, além de 100 seeds |
| R2-T03 | 3 | PASS — round-trip, composição e sessões de 10.000 conversões por razão |
| R2-T04 | 8 | PASS — fronteiras, endpoint inclusivo, obrigações pendentes e horizonte global |
| R2-T05–T15 | 0 | NOT RUN |

Esta evidência valida somente os casos implementados em T01–T04. Não aprova integralmente essas propriedades além da cobertura listada e não conclui R2.

**Limitações conhecidas do primeiro incremento:**
- Ainda não implementa relação causal parcial completa nem fases/eventos concorrentes.
- Ainda não implementa publicação/cancelamento/dispatch de deadlines, ledger de efeitos de transação, WAIT, IRQ/NMI, snapshot/restore ou replay.
- A propriedade T01–T04 está coberta por casos iniciais, não por geração aleatória/longa duração.
- O modelo não representa hardware nem afirma qualquer fase física; todos os tempos são abstratos e exatos.

A implementação não pertence ao backend e não pode ser importada pelo runtime Rust. Nenhum crate de produção foi criado.

## 9. Critérios de saída R2

R2 só pode ser aprovado quando:

1. R2-T01–T15 tiverem resultados reproduzíveis, com casos positivos, negativos e de fronteira aplicáveis.
2. O oráculo estiver demonstravelmente independente de algoritmo/representação candidata e da V1.
3. A matriz de rastreabilidade ligar cada teste a contratos R1 e aos resultados normativos.
4. UNKNOWN não for convertido em fase, precedência, WAIT, commit ou compatibilidade presumida.
5. Snapshot/restore/replay e ledger exactly-once forem verificados em cortes suportados e não suportados.
6. Falhas produzirem contraexemplo mínimo reproduzível.
7. A revisão independente não encontrar blockers.
8. O responsável aprovar explicitamente R2 antes de R3.

**Proibição mantida:** R2 não autoriza R3, não escolhe a arquitetura final e não autoriza qualquer código de produção CHRONOS/V2. R3 só começa após aprovação explícita de R2; código de produção somente após R5.

---

## 10. Registro de decisões e revisões

| Data | Versão/SHA | Evento | Estado |
|---|---|---|---|
| 2026-11-24 | R1 v0.3.18; aprovado em `ac912f342ff303b3fdc554b90d00e85b23d86c76`; merge `88e105d6495f7f5105502e53666a7b0d29f77572` | Responsável aprovou publicação da PR #1281 e autorizou início de R2 | R1 APROVADO; R2 iniciado |
| 2026-11-24 | R2 v0.1 | Estrutura inicial do oráculo, independência, matriz de propriedades e sequência R2-A–R2-E | Rascunho; requer revisão e execução |

Este documento é um plano executável de verificação em evolução. Não altera retroativamente os contratos normativos aprovados em R1.


### 8.1.1 Reexecução após findings Codex — HEAD `a8cef1bcc39e284729a7a55fc16e27deb9fb4b22`

Após a revisão independente do HEAD anterior, foram corrigidos dois findings P2: o runner agora bloqueia a execução quando o worktree Git está sujo ou o HEAD não pode ser obtido, e registra hashes SHA-256 do oráculo, do runner e de cada módulo de teste descoberto; a validação do watermark ocorre antes de qualquer resultado de progresso bloqueado por conjunto incompleto, impedindo a exposição de um valor não exato.

- CI: [run #10623](https://github.com/leonardo201800478/SUN/actions/runs/45505844730).
- SHA Git executado: `a8cef1bcc39e284729a7a55fc16e27deb9fb4b22`.
- Worktree: `CLEAN`.
- SHA-256 de `chronos_reference.py`: `de802612b198996cd6e888aa9fa8840596fa42ab8319aef0611b3841052a30b3`.
- SHA-256 de `run_reference_tests.py`: `bb4798b3cc073b785a8c71e5796a24a595be0cc2ebf32501978bd5d23494bc77`.
- SHA-256 de `test_temporal_core.py`: `b5a6ad833240b5f622ac313d98595bd17a079a51289dedf71c7d0f7bdd1f3d1e`.
- Ambiente: Python 3.14.4 / Linux x86_64; perfil abstrato SMS-NTSC.
- Resultado do runner: **25 tests PASS** em 0.090 s; `test_status=PASS`.
- Contagem atual: R2-T01 = 10; R2-T02 = 4; R2-T03 = 3; R2-T04 = 8. R2-T05–T15 = **NOT RUN**.
- CI também reportou PASS para validação de workflow (actionlint), higiene documental e supervisor de qualidade Rust. Jobs específicos de hardware/Windows permaneceram SKIPPED, não PASS.
- Esta execução confirma somente os 25 casos identificados; não conclui R2 e não substitui revisão independente do novo HEAD.


### 8.1.2 Reexecução após finding sobre o comando plain unittest — HEAD `52b6dd127dcb9f301c8caaa23813f6d2648fae00`

A revisão Codex do HEAD `1ad80f41f52a5a1f676114fc66a242c3bae49089` identificou que um teste da suíte verificava `sys.dont_write_bytecode` no processo chamador. Essa condição pertence ao runner dedicado e não pode ser pré-requisito para o comando `python -m unittest -v` documentado.

Correção aplicada:
- removido o teste de estado do processo e o import `sys` de `test_temporal_core.py`;
- mantido `sys.dont_write_bytecode=True` dentro de `run_reference_tests.py`, antes da descoberta;
- README ajustado para 27 testes e para manter o comando plain unittest documentado;
- CI ampliado para executar o comando literal `python3 -m unittest -v`, remover somente os diretórios `__pycache__` produzidos por essa execução, verificar worktree limpo e executar novamente o runner dedicado.

Evidência vinculada:
- CI: [run #10634](https://github.com/leonardo201800478/SUN/actions/runs/45511594153) — **SUCCESS**.
- SHA Git executado: `52b6dd127dcb9f301c8caaa23813f6d2648fae00`.
- Worktree: `CLEAN` na primeira e na segunda execução do runner.
- SHA-256 de `chronos_reference.py`: `41ee0d615996c8003c289eb113623873ecaa8b1a6e852a33a47f1510c354ccd1`.
- SHA-256 de `run_reference_tests.py`: `244c61e977f03f4ccad7edc17381dd60335b12406014f08b4dd29fc16c6de8c7`.
- SHA-256 de `test_temporal_core.py`: `ad4dbc4c7df135db34cea71cecbeae30149a1bca87f51ca7c5704b95212c8eef`.
- Ambiente: Python 3.14.4; Linux x86_64; perfil abstrato SMS-NTSC.
- Runner dedicado: **27/27 PASS** na primeira execução e **27/27 PASS** na segunda execução.
- Comando plain `python3 -m unittest -v`: **27/27 PASS**.
- Actionlint, higiene documental e supervisor de qualidade: **PASS**.
- Jobs de Rust, hardware e Windows não aplicáveis a este diff: **SKIPPED**, não PASS.
- Contagem coberta: R2-T01 = 12; R2-T02 = 4; R2-T03 = 3; R2-T04 = 8. R2-T05–T15 = **NOT RUN**.

Esta evidência substitui a contagem de 28 testes do HEAD anterior para o candidato atual. A revisão Codex do HEAD `52b6dd127dcb9f301c8caaa23813f6d2648fae00` foi solicitada e ainda estava em andamento no momento deste registro. R2 continua EM ANDAMENTO; nenhuma expansão R2-B, R3, produção ou merge é autorizada por esta execução.


### 8.1.3 Correção do finding P2 — cálculo integral de A(E) para cancelamentos

A revisão Codex do candidato `068f56dd9ad459cef84dc30797224d6f9c21ef62` identificou que `cancel_deadline` recebia `admission_frontier` como argumento e confiava nesse valor sem recalcular a fronteira a partir do estado completo. Isso permitia aceitar incorretamente `C=2` com `W=3`, `D=10` e fronteira fornecida igual a zero.

Correção implementada no candidato subsequente:
- `cancel_deadline` agora recebe o `ReferenceState` e deriva internamente `A(E)` a partir do progresso convertido de todos os participantes esperados e de `W`.
- O conjunto incompleto retorna `Blocked`; conversões temporais inválidas ou divergência entre os watermarks do estado e do livro retornam `IndeterminateCausalOrder`.
- A rejeição `C<=A(E)` ocorre antes da avaliação de `C<D` e não altera o livro de deadlines.
- `replace_deadline` usa a mesma validação derivada, preservando a atomicidade da substituição.
- Foram acrescentados seis testes discriminantes: watermark que excede o progresso local, progresso de participante não proprietário, conjunto incompleto, conversão inválida, watermarks divergentes e conversões racionais exatas.

**Estado desta evidência:** correção e testes foram incorporados ao candidato, mas ainda não há resultado de CI nem revisão independente vinculados ao novo SHA. Não classificar a correção como PASS até essas execuções terminarem.


### 8.1.4 Correção do finding P2 — empate C=D sem relação causal explícita

A revisão Codex do SHA `eeda6c9f9a379c89cec79dd2743748e041338506` identificou que o caminho de cancelamento classificava `C=D` como `LateInput` apenas por satisfazer `C>=D`. Isso não implementava R1 §16.5.13(13)(b)/C06: quando `C=D>A(E)` e `W` não existe ou `W<D`, a supressão depende da relação causal ou do microstep explícito do perfil. A API atual não recebe essa relação; portanto, não pode inferi-la.

Correção candidata:
- Preservar a precedência normativa: `C<=A(E)` continua retornando `LateInput` antes de qualquer avaliação do empate.
- Separar `C=D` de `C>D`: sem relação causal/microstep fornecida, o empate retorna `IndeterminateCausalOrder`, sem mutação; `C>D` retorna `LateInput`.
- Substituir o teste que aceitava a classificação incorreta por um teste de empate indeterminado e sem mutação; adicionar teste independente para `C>D` tardio.

**Cobertura adicional:** o empate sem `W` e com `W<D` possui testes discriminantes separados; ambos exigem `IndeterminateCausalOrder` sem mutação.

**Estado da correção:** CI #10656 terminou SUCCESS no SHA `50d6cc64c9fa071d867ac5fa9db747116f4e32f6`; revisão independente desse SHA permanece pendente. R2 permanece EM ANDAMENTO; T06/T07/T14 continuam parciais. Não autoriza R3, produção ou merge.


A suíte candidata foi ampliada para 57 testes (30 casos parciais no slice R2-B) para cobrir o empate C=D tanto sem watermark quanto com W<D. Essa contagem ainda requer CI vinculado ao novo HEAD.


#### Evidência vinculada — CI #10656

- HEAD executado: `50d6cc64c9fa071d867ac5fa9db747116f4e32f6`.
- [CI #10656](https://github.com/leonardo201800478/SUN/actions/runs/45518319020): **SUCCESS**.
- Job CHRONOS R2: 57/57 PASS; runner dedicado executado duas vezes e plain `python -m unittest -v`; worktree CLEAN nas duas execuções.
- Ambiente: Python 3.14.4 / Linux x86_64 / perfil abstrato SMS-NTSC.
- SHA-256: `chronos_reference.py` = `41ee0d615996c8003c289eb113623873ecaa8b1a6e852a33a47f1510c354ccd1`; `deadline_reference.py` = `8ac62f8985c8fa56d448c21e6a2f8199151b8807b07fa2fb79c209ba034e7f99`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `1bff8aab0661e392017166b50165e77b8a4840b473e4f9a69565830b9e84c5ca`; `test_temporal_core.py` = `ad4dbc4c7df135db34cea71cecbeae30149a1bca87f51ca7c5704b95212c8eef`.
- Actionlint, higiene documental e Rust quality/unit regression: PASS. Jobs de hardware/Windows: SKIPPED, não PASS.
- Esta evidência está vinculada somente ao SHA executado. A atualização documental subsequente precisa de sua própria CI/revisão; nenhuma conclusão de gate é inferida.


### 8.1.5 Precedência explícita de W fechado sobre empate e cancelamento anterior

A auditoria complementar da matriz R1 §16.5.13(13) observou que os testes existentes cobriam `C=D) com `W) ausente e `W<D), mas não isolavam os ramos `W=D), `W>D) e `C<D) recebido depois de `W>=D). O código já rejeita esses casos pela precedência `C<=A(E)), pois `A(E)` inclui `W); foram acrescentados testes discriminantes para tornar a regra verificável e impedir regressão.

- `C=D) com `W=D) ou `W>D` -> `LateInput`, sem mutação.
- `C<D) recebido quando `W>=D` -> `LateInput`, sem mutação.
- Os casos `C=D) sem `W) e com `W<D` continuam `IndeterminateCausalOrder` sem relação causal/microstep explícita.
- `C>D) continua sem suprimir o deadline.

**Evidência:** CI #10658 terminou SUCCESS no SHA `7ac41b64e3c11ae85d579ff0123f55e821b4bc89`; a suíte executou 59/59 PASS, com runner dedicado duas vezes e `python -m unittest -v`, worktree CLEAN nas duas execuções. A revisão independente do SHA executado permanece pendente. Os testes não alteram contratos R1, não completam T06/T07/T14 e não autorizam R3, produção ou merge.


#### Evidência vinculada — CI #10658

- HEAD executado: `7ac41b64e3c11ae85d579ff0123f55e821b4bc89`.
- [CI #10658](https://github.com/leonardo201800478/SUN/actions/runs/45518807530): **SUCCESS**.
- Job CHRONOS R2: 59/59 PASS; runner dedicado duas vezes e plain `python -m unittest -v`; worktree CLEAN nas duas execuções.
- `deadline_reference.py` SHA-256: `8ac62f8985c8fa56d448c21e6a2f8199151b8807b07fa2fb79c209ba034e7f99`.
- `test_deadlines.py` SHA-256: `f0050b18082758d078aabcb1c8735dfeb18bae5174bfb28b8f907e8debc8ba68`.
- Esta evidência está vinculada somente ao SHA executado. A atualização documental subsequente precisa de sua própria CI/revisão; nenhuma conclusão de gate é inferida.


### 8.1.6 Consistência do watermark na publicação de deadlines

A auditoria do caminho `publish_deadline()` identificou que o modelo validava a exatidão de `DeadlineBook.watermark_common`, mas não comparava esse valor com `ReferenceState.watermark_common`. Como ambos representam o mesmo corte global no oráculo, aceitar divergência poderia admitir um deadline usando um watermark diferente daquele presente no estado temporal.

Correção candidata:
- validar a exatidão dos dois watermarks;
- exigir igualdade entre o watermark do livro de deadlines e o watermark do estado de referência antes da comparação temporal;
- retornar `IndeterminateCausalOrder` sem mutação quando houver divergência;
- ajustar o caso de teste de watermark fechado para fornecer o mesmo valor nas duas estruturas;
- adicionar teste para divergência em ambos os sentidos (watermark apenas no estado e watermark apenas no livro), verificando ausência de mutação.

**Estado:** implementação no commit `ebb4c937fc6b9205b87c639d8e9515e0c7643a39`; teste no commit `14bc6b96d228c1b9b696422ee1f6ed0340c3981f`. CI #10660 foi disparado para o HEAD de código/testes e aguarda conclusão. A contagem candidata passa a 60 testes e 33 casos no slice R2-B, ainda sem classificação PASS até o CI. T06/T07/T14 continuam parciais; R2 segue EM ANDAMENTO; R3–R5 BLOQUEADOS.


### 8.1.7 Preservação de W ao agendar microsteps

A validação de consistência introduzida em `publish_deadline()` revelou uma interação no helper `schedule_microstep()`: o estado sintético usado para admitir o evento filho era criado sem copiar o watermark já fechado do `DeadlineBook`. Com isso, um microstep futuro poderia ser rejeitado por divergência artificial entre o estado sintético e o livro.

Correção candidata:
- propagar `book.watermark_common` para o `ReferenceState` sintético usado por `schedule_microstep()`;
- testar admissão de microstep em instante posterior a W, preservando W;
- testar rejeição `LateInput`, sem mutação, quando o microstep tenta ocorrer exatamente em W.

**Estado:** implementação no commit `c6ee43d8172c227ee3571464807b0997bfecc10a`; testes no commit `fbb340f837062d12e327686f1e9964fc5cf691c7`. O candidato passa a 62 testes (35 casos R2-B). CI para o incremento de código/testes ainda pendente; não classificar como PASS até execução vinculada. T06/T07/T14 continuam parciais; R2 EM ANDAMENTO; R3–R5 BLOQUEADOS.


### 8.1.8 Validação estrutural dos participantes na publicação

A comparação entre `publish_deadline()` e `_cancellation_admission_frontier()` revelou uma assimetria: o cancelamento rejeitava identidades de participante duplicadas ou inválidas, mas a publicação convertia a lista de domínios em um conjunto antes de verificar duplicidade. Um estado com dois domínios de mesmo ID poderia, portanto, aparentar um conjunto completo e prosseguir usando apenas o primeiro domínio encontrado. A publicação também não rejeitava perfil vazio.

Correção candidata:
- validar que o estado e o perfil são válidos antes de consultar os domínios;
- rejeitar identidade de participante vazia, duplicada ou fora do conjunto esperado;
- rejeitar identidade inválida no conjunto de participantes esperados;
- retornar `InvalidInput` sem mutação;
- adicionar testes para participante duplicado e perfil vazio.

**Estado:** implementação no commit `4074b78d528b97d15677138421753fd983d74d05`; testes no commit `fa59b6f3ad092656ae9cbbd95173afc71922adf3`. O candidato passa a 64 testes e 37 casos no slice R2-B. CI vinculado ainda pendente; não classificar como PASS antes da execução. T06/T07/T14 permanecem parciais; R2 EM ANDAMENTO; R3–R5 BLOQUEADOS.


### 8.1.9 Validação estrutural defensiva do oráculo de deadlines

A revisão independente Codex do HEAD `91dbb4344ecd24699ef88719e4c65b872dd72393` confirmou três findings P2 na fronteira de entrada do slice R2-B:

- `ReferenceState.domains` podia conter um objeto que não fosse `ClockDomain`; a leitura antecipada de `domain_id` provocava `AttributeError`.
- `expected_participants` podia ter um contêiner/elementos incompatíveis com o contrato; construir `set(...)` antes da validação podia provocar `TypeError`.
- `DeadlineBook` podia ser um objeto de tipo incorreto ou conter coleções/entradas malformadas; publicação/cancelamento podiam dereferenciá-lo antes de produzir um resultado do oráculo.

Correção candidata:
- validar o tipo dos contêineres e cada objeto `ClockDomain` antes de ler seus campos;
- validar o tipo `frozenset` e as identidades de participantes antes de qualquer conversão/conjunto;
- validar `DeadlineBook`, watermark, limite de microsteps, contêineres e campos de cada `Deadline`/`Cancellation` antes de percorrer ou alterar o livro;
- aplicar a validação do livro também às operações públicas de arbitragem, fechamento de W, dispatch, completion e microstep;
- preservar `IndeterminateCausalOrder` para conversões de clock não comparáveis; usar `InvalidInput` para estrutura/tipo/descritor inválido;
- manter todas as rejeições atômicas, retornando o mesmo livro sem mutação;
- adicionar onze testes de regressão cobrindo domínios inválidos, conjunto esperado malformado, livro ausente, deadlines/cancelamentos malformados e validação nas operações de fechamento/microstep.

**Evidência vinculada — CI #10672:** executada no HEAD `6bb6b24bf5551379af13bc104b2ab900dcee6544`; [run](https://github.com/leonardo201800478/SUN/actions/runs/45524036173) — **SUCCESS**. O job CHRONOS R2 executou 75/75 PASS no runner dedicado, em duas execuções consecutivas, e no comando plain `python -m unittest -v`; worktree CLEAN. Ambiente: Python 3.14.4 / Linux x86_64. SHA-256: `deadline_reference.py` = `27d1d859a77f976187cf8fad1eac8090e879a5a9eea71defe119b60a48a34498`; `test_deadlines.py` = `dec6b21314b2f3011fa0d701f99dbd6db3dd5fe86d3c922d35b4bbb2ba36681f`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `chronos_reference.py` = `41ee0d615996c8003c289eb113623873ecaa8b1a6e852a33a47f1510c354ccd1`. Actionlint, higiene documental e supervisor de qualidade: PASS; jobs Rust/hardware/Windows e subsistemas não relacionados: SKIPPED, não PASS.

R2 continua EM ANDAMENTO; T05, T08–T13 e T15 permanecem NOT RUN; T06/T07/T14 permanecem parciais; R3–R5 continuam BLOQUEADOS. Nenhum código de produção CHRONOS/V2 ou merge é autorizado.


### 8.1.10 Correções dos contraexemplos de watermark, microsteps e horizonte — 25/11/2026

Foram reproduzidos quatro contraexemplos no HEAD anterior e corrigidos na própria branch da PR #1283:

- **P1 — consistência entre frontier e horizonte global:** após validar todos os participantes, o oráculo calcula `H = min(H_d)` e `F = max(W, P_d)`. Se `H < F`, retorna `InvariantViolation` sem alterar o estado. O teste cobre `W=20`, `H=10`, avanço solicitado em 20.
- **P1 — limite finito de microsteps na API pública:** `publish_deadline()` calcula o nível efetivo (incluindo o nível 1 implícito de `microstep=True`) e compara com `DeadlineBook.max_microsteps`. Nível acima do limite retorna `ResourceExhausted` sem mutação; o limite exato continua aceito. Os testes usam limite 2 e níveis 3 e 999.
- **P2 — deadline anterior ao watermark já fechado:** `close_watermark()` rejeita estado inconsistente que contenha deadline ainda `Scheduled` com `D <= W` antes de reivindicar eventos ou avançar W. O teste usa `D=4`, `W=5` e verifica ausência de mutação.
- **P2 — validação temporal para publicação/cancelamento:** `_validate_reference_state()` exige `horizon_local` exato e `horizon_local >= progress_local` antes da admissão. Testes cobrem `0.5`, `-1` e horizonte 1 com progresso 2, tanto em publicação quanto em cancelamento; rejeições não alteram o livro.

A primeira execução do candidato de 80 testes expôs uma fixture antiga inválida: o progresso do VDP era 5 enquanto o horizonte local permanecia no default 0. A fixture foi corrigida para horizonte 100; nenhuma validação foi relaxada. Também foi preservada a precedência da validação estrutural/temporal do estado antes do retorno `ResourceExhausted` do limite de microsteps.

**Evidência vinculada — CI #10676:** HEAD `09f2ba729d70d3e2760c4844bcc4b2df06aaba2b`; [run](https://github.com/leonardo201800478/SUN/actions/runs/45529157944) — **SUCCESS**. A suíte CHRONOS R2 executou 80/80 PASS no runner dedicado duas vezes e no comando literal `python -m unittest -v`; worktree CLEAN. Ambiente: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `701ca24e4f61bcb083504c87a079b72f0f034bd4b7abc5e3ec5fd9d176986265`; `deadline_reference.py` = `b92716d4b29e0e57846679b310ba1519f72685967308c6d16206b54c0ac0ef07`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `587f43e2dde2816f70b1e59b6f73ee8fdf828f4c9ecce79d5b50a1a0fd8d2c4b`; `test_temporal_core.py` = `fdbc70d9bd8dd79092b604bcb317d7f5af29205f73aa11a9b9ef9a6648730b55`. Actionlint, higiene documental e supervisor de qualidade: PASS. Rust workspace, Windows e hardware: SKIPPED.

Naquele candidato, a cobertura executada era de 80 testes: T01=12, T02=4, T03=3, T04=9; slice R2-B=52 casos. T06/T07/T14 permanecem parciais; T05, T08–T13 e T15 continuam NOT RUN. R2 segue EM ANDAMENTO; R3–R5 BLOQUEADOS. Esta evidência não aprova o gate R2, não autoriza código de produção CHRONOS/V2 e não autoriza merge.


### 8.1.11 Revisão adversarial de horizonte global, arbitragem e substituição — 25/11/2026

A revisão do HEAD `ff57ca2d2500906cbfefe3ca43d8b35745a5a97b` identificou quatro lacunas adicionais. As correções foram implementadas na mesma branch da PR #1283:

- **P1 — horizonte global em todos os caminhos:** `chronos_reference._validate_state()` e `deadline_reference._validate_reference_state()` comparam o menor horizonte convertido conhecido com a fronteira conhecida formada por W e os progressos convertidos. Se os dados já conhecidos provam `H < max(W, P_d)`, o resultado é `InvariantViolation` antes de um retorno `Blocked`. Quando o conjunto está incompleto e não há contradição demonstrável, a operação continua `Blocked`; nenhum máximo parcial é exposto como A(E) ou frontier de avanço. Publicação e cancelamento aplicam a mesma validação global. A taxonomia separa tempo não exato (`InvalidInput`), conversão incomparável (`IndeterminateCausalOrder`) e horizonte regressivo (`InvariantViolation`).
- **P2 — arbitragem owner/identity/generation:** `_validate_book()` agora valida a relação cruzada entre cada cancelamento pendente e o deadline exato correspondente. Owner divergente, request órfão, chave duplicada, status diferente de `CancelPending` ou deadline `CancelPending` sem request correspondente invalidam o livro antes de qualquer arbitragem. As rejeições preservam o livro original.
- **P2 — fronteira antes do estado de ciclo de vida:** `cancel_deadline()` deriva A(E) antes de classificar uma geração já `Claimed`/não cancelável; assim, participantes incompletos retornam `Blocked` e conversões inválidas retornam `IndeterminateCausalOrder`, sem serem mascarados por `LateInput`. Um conjunto de participantes vazio para essa classe de entrada também retorna `Blocked`, conforme R1 §16.1.10.
- **P2 — instante da nova geração em substituição atômica:** a API `replace_deadline()` representa uma única operação causal em `cancel_at`. Após validar o cancelamento e o descritor/admissão da nova publicação, `new_at_common < cancel_at` é rejeitado como `LateInput`; `new_at_common == cancel_at` retorna `IndeterminateCausalOrder`, pois a API não fornece relação causal/microstep explícita entre a substituição e a nova geração. Ambas as rejeições devolvem o livro original, sem cancelamento parcial. Esta é uma restrição conservadora do modelo de referência para a operação atômica; não é afirmação física nem alteração do texto normativo R1.
- **P2 — igualdade explícita D=W no fechamento:** foi adicionado teste discriminante para deadline ainda `Scheduled` com `D == W) já fechado; `close_watermark()` rejeita sem reivindicar o evento nem mutar o livro.

A documentação geral foi reconciliada: o README da raiz e `SUN-V2/README.md` agora indicam R1 APROVADO em 24/11/2026, R2 EM ANDAMENTO e R3–R5 BLOQUEADOS. O README do oráculo identifica explicitamente a execução de 80 testes no HEAD anterior como histórica e registra a evidência do novo candidato.

**Evidência vinculada — CI #10681:** [run](https://github.com/leonardo201800478/SUN/actions/runs/45530977080) — **SUCCESS** no HEAD `8c466dcaa1b5d5b4d6e9f076a03a1df9f0a2f2ab`. CHRONOS R2: **89/89 PASS**, runner dedicado em duas execuções e comando literal `python -m unittest -v`; worktree CLEAN. Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `d972005df827412bed74f0d1cc655e64fdad2aae3325f1f8842e4d4b5b531edb`; `deadline_reference.py` = `4a3999f3c0fa800a0feca4e35ffa5297e34a0eea0e6562014005f6b22496e942`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `fb33bda1923ca1d026fdb70a1c4a9ef11adb6a71b0fe8357a4ff2cc46a6ca9c7`; `test_temporal_core.py` = `c9fce1e6e3d247e9811d3a0c252066a91af8e81cc2ae2e08a32cf4a13ee03770`. Actionlint, higiene documental e supervisor de qualidade Rust: PASS. Workspace Rust, Windows e hardware: SKIPPED, não PASS.

A cobertura candidata é 89 testes: R2-T01=13, T02=4, T03=3, T04=9; slice R2-B=60 casos. T06/T07/T14 continuam parciais; T05, T08–T13 e T15 permanecem NOT RUN. R2 continua EM ANDAMENTO; R3–R5 BLOQUEADOS. Nenhum resultado desta execução aprova R2, autoriza R3, código de produção CHRONOS/V2 ou merge.


### 8.1.12 Limite de microsteps em livros pré-construídos — 25/11/2026

A revisão adversarial encontrou um bypass P2 na fronteira estrutural de `DeadlineBook`: `_validate_book()` verificava que `microstep` fosse inteiro não negativo, mas não comparava o valor armazenado com `book.max_microsteps`. Assim, um livro construído externamente com `max_microsteps=2` e deadline em `microstep=999` passava pela validação central e podia chegar à reivindicação em `close_watermark()`, embora `publish_deadline()` impedisse a publicação direta acima do limite.

Correção candidata implementada:
- `_validate_book()` rejeita cada deadline cujo `microstep > max_microsteps`, com `InvalidInput`, antes de qualquer operação pública que dependa da validação central;
- a rejeição preserva o livro original, não reivindica deadlines e não avança o watermark;
- regressão constrói diretamente livros com limite 2 e níveis 0, 1, 2 e 999; níveis 0–2 continuam válidos e claimable, enquanto 999 é rejeitado atomicamente.

**Evidência vinculada — CI #10684:** [run](https://github.com/leonardo201800478/SUN/actions/runs/45537064335) — **SUCCESS** no HEAD de código/testes `62480ce2502b3783d92c39d2afe91ea064ce8f65`. CHRONOS R2: **90/90 PASS**, runner dedicado duas vezes e comando literal `python -m unittest -v`; worktree CLEAN nas duas execuções. Ambiente: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `d972005df827412bed74f0d1cc655e64fdad2aae3325f1f8842e4d4b5b531edb`; `deadline_reference.py` = `30cbe9870750958752f2a2e7338bcfb523fa903aa8066f30aaffc93081fbd0c6`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `0d594b09a18af442b7cb142a85862eb8a8f459f45d4f6b8bd17592f65111b130`; `test_temporal_core.py` = `c9fce1e6e3d247e9811d3a0c252066a91af8e81cc2ae2e08a32cf4a13ee03770`. Actionlint, higiene documental e supervisor de qualidade Rust: PASS. Workspace Rust, Windows, hardware e subsistemas não relacionados: SKIPPED, não PASS.

A revisão independente do novo HEAD foi solicitada via `@codex review` e ainda não foi concluída na última consulta. R2 permanece EM ANDAMENTO; R3–R5 BLOQUEADOS. Nenhuma alteração normativa R1, implementação de produção CHRONOS/V2, aprovação ou merge decorre deste finding.
