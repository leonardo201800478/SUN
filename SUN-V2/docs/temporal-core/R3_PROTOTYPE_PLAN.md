# CHRONOS R3 — plano de protótipos temporais descartáveis

**Gate:** R3 — prototipagem e comparação
**Estado:** P-A W00–W09 implementado em protótipo; CI #10902 SUCCESS Linux/Windows, 116/116 testes P-A e 256/256 testes R2 por OS; 351 pontos de benchmark por OS com parâmetros/digests equivalentes; equivalência canônica completa e revisão adversarial pendentes
**Branch:** `research/chronos-r3-disposable-prototypes`
**Base normativa:** R1 v0.3.18 aprovado; R2-T01–T15 aprovado e merged pela PR #1283
**Escopo de sistema:** perfil temporal abstrato inicialmente; SMS NTSC/PAL somente quando os parâmetros forem fixados por fonte R0/R1 e cenário identificado.

## 1. Objetivo

Comparar mecanismos alternativos de execução temporal sem alterar a semântica normativa de R1 nem o oráculo independente de R2. A decisão deve minimizar o custo total mantendo determinismo, causalidade, avanço seguro, deadlines, transactions resumíveis, snapshot/restore e replay conforme R1.

R3 não seleciona um algoritmo por elegância, familiaridade ou melhor resultado isolado. A implementação híbrida permanece hipótese. P-A é baseline obrigatória; P-D e P-E são alternativas centrais; P-F só será prototipada se resultados medidos demonstrarem lacuna funcional/custo que a combinação resolva. P-B e P-C não entram no primeiro corte.

## 2. Modelos candidatos

| ID | Modelo | Definição experimental | Hipótese a testar | Risco principal |
|---|---|---|---|---|
| P-A | Instrução + catch-up | Cada participante avança em orçamento/chunk limitado; obrigações e deadlines são verificadas na fronteira de avanço e catch-up dos participantes atrasados respeita o safe horizon. Não há agenda global como autoridade. | O modelo mais simples pode satisfazer a carga com custo baixo quando observações/sincronizações são pouco frequentes. | Custo de catch-up proporcional a participantes e trabalho repetido; resolução de deadline próximo pode exigir reduzir chunk. |
| P-D | Agenda discreta central | Eventos/deadlines autônomos são armazenados em uma agenda central com chave temporal/causal; a retirada do próximo evento respeita igualdade UNKNOWN e relações de causalidade declaradas. Os dispositivos continuam donos dos próprios estados. | Agenda reduz polling/catch-up em workloads sparse/autonomous com número relevante de deadlines. | Inserção/cancelamento, invalidação de geração, custo de fila, empates simultâneos e contenção conceitual de propriedade. |
| P-E | Sync-on-demand | Não mantém agenda global de progresso contínuo; observação/commit declara participantes e o instante necessário, resolve apenas o fecho causal declarado e avança participantes sob demanda. | Reduz trabalho ocioso e overhead de agenda em cargas access-driven. | Explosão de custo quando acessos cross-domain são frequentes; dependências longas e validação de fecho causal. |
| P-F | Híbrido seletivo | Composição restrita de sync-on-demand com agenda apenas para deadlines autónomos comprovadamente necessários. | Pode resolver um gap mensurável entre P-D e P-E sem duplicar responsabilidades. | Duplicação de mecanismos, dois caminhos de lifecycle e custo de coerência. Só fazer se houver gap demonstrado. |

**P-A/P-D/P-E são protótipos de mecanismos, não três semânticas.** Todos consomem a mesma sequência de operações e devem produzir o mesmo trace de transições, o mesmo corte temporal, os mesmos status/erros e o mesmo digest observável. Um protótipo que “passa” ao arredondar timestamps, ordenar UNKNOWN por inserção ou descartar efeitos não é equivalente.

### 2.1 Hipóteses falsificáveis

- **H-A1:** P-A apresenta custo que cresce com número de participantes/chunk boundaries mesmo quando há poucos deadlines; refutar se sparse profiles mantiverem custo estável com carga equivalente.
- **H-D1:** P-D reduz custo em agenda com deadlines autónomos esparsos; refutar se custo de agenda/invalidação superar o polling/catch-up evitado.
- **H-E1:** P-E reduz custo em workload dirigido por observação; refutar se fechamento de dependências e sincronização repetida dominar o custo.
- **H-F1:** P-F só é justificável quando nenhum P-A/P-D/P-E isolado satisfaz simultaneamente semântica e orçamento de custo em um cenário representativo, e a combinação remove o gap sem duplicar lifecycle.
- Não converter resultados sintéticos em alegação de gameplay ou fidelidade física.

## 3. Semântica invariável e oráculo

O oracle normativo é o pacote R2 em `SUN-V2/verification/r2_reference/`. Os protótipos não devem importar tipos/código R2 para o caminho medido. O harness de validação pode invocar o oráculo como processo de comparação fora da região cronometrada; os modelos experimentais terão tipos e estruturas próprias, com conversão de fixtures determinística.

Dimensões mínimas:
- tempo exato (inteiro/racional), conversão e resto/fase; floats são proibidos como verdade temporal;
- safe horizon e avanço limitado; nenhum catch-up atravessa obrigação pendente;
- conjunto participante completo e fronteira de admissão A(E);
- deadline lifecycle Scheduled/CancelPending/Cancelled/Claimed/Dispatched/Completed/Failed e generation fencing;
- C<D, C=D com relação causal/microstep explícita e C=D UNKNOWN; não fabricar precedência;
- watermark W, publicação, cancelamento, arbitragem e dispatch atrasado;
- dependências cross-domain e fecho causal apenas de participantes declarados;
- microsteps finitos, cada um consumindo obrigação identificável; limite causa ResourceExhausted sem mutação parcial;
- amostragem/commit e retomada exactly-once;
- snapshot/restore/replay com transações e efeitos parciais pendentes;
- erros e estado sem mutação onde R1 exigir.

A equivalência pretendida é verificada por trace canônico: operações aceitas, estado resultante por domínio, lifecycle/generation dos deadlines, watermark, ledger de efeitos, código de retorno, first divergence e digest final. A ordenação determinística só pode ordenar membros comutativos/declaradamente ordenados; UNKNOWN permanece UNKNOWN. O corpus atual de P-A contém somente metadados de cortes e digests; não contém as fixtures/entradas e os estados completos necessários para reexecutar a mesma sequência em P-D/P-E. Logo, não satisfaz ainda o trace/fixture equivalence canônico completo exigido pelo gate.

## 4. Primeiro corte: baseline P-A

**Estado validado no HEAD `09f71c8aa059c44855c358b614c965f92900a173` (CI #10937):** P-A cobre os workloads W00–W09 em protótipo descartável. A suíte possui 125 testes no Linux e 125 no Windows: 24 avanço/catch-up, 31 deadline lifecycle, 14 causal sync, 11 microsteps, 18 transações e 27 snapshot/replay. As suítes independentes R2 passaram com 256/256 testes por sistema operacional. Os jobs de hardware/runtime não relacionados foram SKIPPED e não são contabilizados como validação.

As sete matrizes P-A executaram 351 pontos por ambiente: 75 avanço, 180 deadlines, 30 causal, 24 W06, 12 W07, 10 W08 e 20 W09. Cada ponto utiliza cinco warm-ups e trinta repetições medidas. Os resultados são observações de Python sintético; não representam desempenho Rust nem evidência física de hardware SMS.

O corpus emitido por `emit_p_a_trace_corpus.py` registra 112 identificadores de fixture e 1.475 cortes (1.468 diferenciais R2 e sete propriedades determinísticas), com digest agregado Linux/Windows idêntico. **Limitação importante:** o artefato atual registra identificador de fixture, índice do corte, base de comparação e digest; não serializa as definições/entradas das fixtures, operações executadas nem estados completos de cada corte. Portanto, é um corpus de digests de cortes, não ainda um corpus canônico completo e reexecutável para comparar P-A/P-D/P-E. O requisito de equivalência canônica completa permanece PENDENTE.

P-A implementa avanço limitado, catch-up, deadlines, sincronização causal, microsteps, transações e snapshot/replay. Os testes W06/W07 que divergem do oráculo R2 são mantidos como casos normativos independentes e não contam como equivalência diferencial positiva. As lacunas semânticas W06/W07 e os limites do corpus estão detalhados em `R3_PA_ADVERSARIAL_REVIEW.md`.

As matrizes P-A estão registradas em `SUN-V2/benchmarks/r3/P_A_BASELINE_RESULTS.md`; cobrem 351 pontos por ambiente: 75 avanço, 180 deadlines, 30 causal, 24 W06, 12 W07, 10 W08 e 20 W09. O tamanho do chunk permanece parâmetro de workload, não constante de semântica. O benchmark registra chunks, participantes tocados/inspecionados e obrigações varridas. Isso evidencia trabalho interno, sem afirmar que essas contagens representam ciclos físicos.

Não implementar CPU Z80, VDP, PSG, clock SMS físico, ROM loader, thread scheduling, GUI, renderer ou runtime. O primeiro protótipo é uma máquina temporal sintética para estudar custo de algoritmo, não uma máquina SMS.

## 5. Workloads comparáveis

As operações serão pré-geradas com seed fixa antes da medição. Mesma fixture e sequência de operações para P-A/P-D/P-E. O gerador não pode usar o próprio algoritmo do protótipo para determinar o workload.

| ID | Perfil | Densidade | Operações principais | Parâmetros a varrer |
|---|---|---|---|---|
| W00 | Sem eventos/obrigações | zero | avanço limitado e repetido | participantes 1/2/4/8/16/32; horizonte/chunk |
| W01 | Sparse autonomous | poucos deadlines distantes | publicação, avanço, claim/dispatch/complete | deadlines por domínio; distância temporal; proporção cancelada |
| W02 | Dense deadlines | agrupados/alta densidade | publicação, avanço, watermark, despacho | eventos por instante; taxa de empate; geração |
| W03 | Observation-driven | poucos acessos cross-domain | sync-on-demand, observes/alters, dependências | fan-in 1/2/4/8; profundidade causal |
| W04 | Frequent observation | alta frequência de acesso | repetir sync e avanços curtos | acessos por mil unidades; participantes por acesso |
| W05 | Cancellation races | C<D/C=D/C>D | admit/cancel/arbitrate/close W/dispatch | taxa de cancelamento; W ausente/antes/depois |
| W06 | Simultaneous UNKNOWN | eventos simultâneos não comutativos | fecho de watermark e resultado indeterminado | grupo 2/4/8/16; comutatividade e evidência |
| W07 | Microsteps | cadeias no mesmo instante | agendamento de filho, progress, limite | profundidade até max_microsteps e tentativa excedente |
| W08 | Transaction lifecycle | suspend/resume/commit | sample, retry, commit exactly-once, falha parcial | efeitos/subefeitos e pontos de falha |
| W09 | Snapshot/replay | estado pendente | snapshot, restore, replay e digest | número de domínios, eventos e ledgers |

O primeiro corte P-A cobriu W00–W05; W06/W07/W08/W09 foram adicionados assim que seus workloads puderam ser representados sem ampliar premissas físicas. P-D/P-E não devem ser implementados antes de CI verde para W00–W09 e fixture/trace equivalence canônica completa. Nenhuma implementação P-D/P-E está autorizada nesta etapa.

## 6. Protocolo de benchmark

- Gerar fixtures e entradas antes da região cronometrada.
- Warm-up de 5 execuções e 30 repetições medidas por ponto; registrar `sample_count`, vetor bruto, unidade e ambiente. Wall time e CPU time são campos separados. CPU time só é aceito quando a resolução empírica de `thread_time_ns` é <=10.000 ns; resolução ausente ou superior resulta em `SKIPPED_COARSE_RESOLUTION` e amostras CPU nulas. Pico tracemalloc permanece em probe separado.
- Fixar seed, versão de Python/toolchain, CPU, sistema, governor/power plan, frequência efetiva se disponível, commit SHA, configuração, tracing e flags.
- Executar cada variante em processos/ordens alternadas para reduzir viés de aquecimento e drift; guardar amostras brutas, não só resumo.
- Medir wall-clock e CPU time; separar geração de workload, validação do trace, serialização JSON e hashing do benchmark principal.
- Todos os drivers comparáveis usam o estimador Hyndman-Fan Type 7 (`h=(n-1)p`, interpolação linear), implementado em `benchmark_statistics.py`. Reportar p50/p95/p99 e máximo observado diretamente dos vetores brutos, além de média/mínimo quando disponíveis. O máximo não é interpolado.
- Repetir com tracing ligado/desligado; tracing não pode alterar resultado funcional.
- Guardar medição por operação e dados brutos, ambiente e hashes como artefatos CI com retenção; não comparar números de máquinas diferentes como se fossem A/B. O throughput deve derivar de contadores reais de inspeção, nunca de soma arbitrária de entidades do fixture.
- Nenhum limite de overhead é escolhido antes da primeira baseline P-A; benchmark é evidência de custo, não critério para enfraquecer contrato.

## 7. Critérios de equivalência e descarte

Uma execução é funcionalmente comparável somente se:
1. todas as pré-condições e semântica R1 forem implementadas no escopo declarado;
2. trace/digest dos resultados coincidirem com R2 nos cenários cobertos; divergências conhecidas W06/W07 devem ser tratadas por testes normativos independentes, nunca mascaradas como equivalência diferencial (ver `R2_R3_ORACLE_GAP_PROPOSAL.md`);
3. negativos, igualdade e UNKNOWN retornarem as mesmas classes sem mutação;
4. repeat/replay produzirem mesmo digest sob mesma entrada;
5. o protótipo não usar tempos float, agenda do host, threads, relógio wall-clock ou SO como autoridade de tempo virtual.

Descartar variante se:
- exigir regra física sem fonte/medição;
- alterar status de igualdade/UNKNOWN para ganhar throughput;
- quebrar causalidade, atomicidade, exactly-once ou snapshot/replay;
- tiver custo maior que alternativa mais simples sem ganho de fidelidade/cobertura;
- sua vantagem existir somente em workload enviesado não ligado às perguntas do gate.

## 8. Limites e não objetivos

- Os resultados iniciais serão **OBSERVATION de benchmark sintético**, não FACT de hardware.
- Python prototyping oferece comparação exploratória de estruturas; não representa desempenho futuro Rust nativo.
- Headless não demonstra 60 FPS, frame pacing, áudio físico, input-to-photon ou compatibilidade de ROM.
- R3 não valida PSG/DAC/CPAL, VDP real, CPU Z80, wait states reais ou fase de clock física.
- S0/OutRun permanece encerrado como NOT RUN; não executar, recriar ou criar substituto.
- Nenhum crate, runtime, módulo de produção, API CHRONOS ou integração desktop será criado.

## 9. Gate de entrada para P-D/P-E e de saída R3

P-D só começa após P-A:
- passar equivalência funcional do primeiro recorte R2;
- validar W00–W09, trace/fixture equivalence canônica e JSON de resultados;
- publicar baseline A reprodutível com SHA/ambiente e dados brutos.

P-E só começa após o mesmo conjunto de fixtures e baseline A congelados. P-F exige gap demonstrado que não possa ser resolvido em P-A/P-D/P-E individualmente.

R3 só conclui com matriz comparativa equivalente, custos e limitações explícitos, decisão sustentada (incluindo possibilidade de rejeitar todos e revisar hipótese) e revisão adversarial R4. R5 decide a arquitetura de produção. Não há promoção automática de protótipo a runtime.


## 10. Findings independentes W06/W07 — estado de correção

- **W07 — revisão adversarial (26/11/2026):** a barreira de nível anterior continua sendo uma hipótese experimental, não uma regra demonstrada por R1. No teste B (nível 0) / C (nível 1), o modelo bloqueia C, processa B e depois C, impondo B<C. O campo `commutative` é unário e o cenário não demonstra causalidade B→C nem comutatividade do par B/C. R1 §16.6.2 só permite ordenar dependências causais estabelecidas ou efeitos comutativos. W07 permanece BLOCKED até modelar relação causal/comutatividade por par ou retornar indeterminação quando a relação for desconhecida.
- **W06:** P-A exige descritor tipado de base de comparação, com perfil/revisão, coordenada/unidade, escala, identidade/revisão da base e referência/digest do registro de validação. Base ausente ou distinta mantém `IndeterminateCausalOrder` sem reivindicar deadlines nem fechar watermark. A validação atual verifica a estrutura e o formato do digest, mas não resolve nem autentica o registro externo referenciado. Fixtures são sintéticas; evidência física de perfil SMS permanece UNKNOWN.
- R2 permanece imutável no SHA aprovado. O tratamento formal das duas lacunas está em `R2_R3_ORACLE_GAP_PROPOSAL.md`.
- Nenhum dos findings autoriza produção CHRONOS/V2, P-D/P-E ou avanço automático para R4/R5.


## 11. Revisão adversarial independente do P-A — 26/11/2026

O relatório `R3_PA_ADVERSARIAL_REVIEW.md` registra a revisão do HEAD `09f71c8aa059c44855c358b614c965f92900a173`. Resultado: P-A continua útil como baseline experimental, mas **não está pronto para closeout nem autoriza P-D/P-E**.

- **W07 — BLOCKED:** a regra de barreira de microstep impõe ordenação entre obrigações sem demonstrar causalidade ou comutatividade par-a-par, requisito de R1 §16.6.2. O teste atual prova a execução da regra proposta, não sua conformidade normativa.
- **W06 — LIMITADO A FIXTURES SINTÉTICAS:** `APhaseBasis` torna as bases comparáveis estruturalmente, mas o registro externo e seu conteúdo não são resolvidos/autenticados. A fase física SMS permanece UNKNOWN.
- **Corpus canônico — INCOMPLETO:** 112 fixtures/1.475 cortes/digest idêntico comprovam igualdade dos cortes registrados; o artefato não contém entradas, operações e estados completos para replay cruzado independente. Não satisfaz o requisito de trace/fixture equivalence completo.
- **P-A closeout — BLOCKED:** resolver W07, definir o limite de validação W06 e completar ou formalmente reduzir o requisito do corpus antes de iniciar P-D/P-E.

Nenhum finding autoriza alteração de R1/R2, código de produção CHRONOS/V2 ou avanço para R4/R5.

## 12. Current validation addendum — HEAD 28/11/2026

This section supersedes historical CI/status snapshots above where they conflict with the latest validated PR head. The normative R1 v0.3.18 contract and approved R2 oracle are unchanged.

**Validated HEAD:** `7c07a9e6f7d6ee731aa7ba3495dca444a152300c`; [CI #11112 / run 46100695208](https://github.com/leonardo201800478/SUN/actions/runs/46100695208) — SUCCESS.

- R3 disposable suite: **206/206 PASS on Linux and Windows**.
- Corpus emitter: **184 tests PASS on each platform**.
- Independent R2 reference: **256/256 PASS on Linux and Windows**, including deliberate mutation checks.
- Seven P-A synthetic matrices: **351 points per OS**, all functionally PASS.
- Canonical corpus: **119 fixtures / 1,483 cuts** (1,472 differential + 11 property), logical digest `ab5e146eda37e8292159cc5440beca615c5b511383e3807b24adbacbb490056b`.
- Replayable corpus: **25 fixtures / 134 operations-cuts**, with W06 and W07 divergences intentionally preserved; logical digest `6189653bb3aa18fc5805f7dd1ae9708ea31b349d03da7ea9fcc906350cbcfbb6`.
- Dedicated CI job confirms canonical and replayable artifacts are byte-identical across Linux/Windows. Runner-specific artifact SHA-256 values differ because output paths are embedded; logical digests and normalized comparison establish corpus equivalence.
- CI totals: **9 SUCCESS / 14 SKIPPED**. Skipped jobs do not count as PASS. Windows benchmark CPU-time fields are `SKIPPED_COARSE_RESOLUTION`.

**Remaining gates and limits:**

1. **W06 physical evidence — UNKNOWN.** Current tests validate synthetic phase-basis discrimination only. A resolvable/authenticated external phase record tied to the relevant hardware profile and scenario is still absent.
2. **W07 physical B/C relation — BLOCKED.** Pairwise causal/commutative adjudication is explicit in the disposable candidate, but the physical relation for the original B/C scenario is not established.
3. **P-A closeout — NOT READY.** The 25 replayable fixtures/134 operations-cuts and canonical 119/1,483 corpus are verified cross-platform, but complete declared W00–W09 fixture coverage, formal resolution of the recorded P-A/R2 divergence and independent review/closeout criteria are not demonstrated by these counts alone.
4. **P-D/P-E — NOT RUN/BLOCKED.** Do not begin until P-A entry criteria, frozen shared fixtures and baseline requirements are explicitly met.
5. **R4/R5 — PENDING.** R4 adversarial review and R5 architectural decision remain mandatory. No production CHRONOS/V2 code before R5.

The formal PR review query at this HEAD returned no submitted reviews and no inline review threads. This is separate from CI success. No hardware/runtime/desktop or subsystem jobs skipped by CI are to be treated as validated, and no Python synthetic benchmark establishes Rust performance, physical SMS timing, frame pacing or 60 FPS.
