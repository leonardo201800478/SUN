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
