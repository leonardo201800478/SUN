# CHRONOS R3 — plano de protótipos temporais descartáveis

**Gate:** R3 — prototipagem e comparação
**Estado:** primeiro corte P-A implementado; CI #10821 passou em Linux/Windows; smoke executado sem retenção dos dados; baseline completa pendente
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

A equivalência é verificada por trace canônico: operações aceitas, estado resultante por domínio, lifecycle/generation dos deadlines, watermark, ledger de efeitos, código de retorno, first divergence e digest final. A ordenação determinística só pode ordenar membros comutativos/declaradamente ordenados; UNKNOWN permanece UNKNOWN.

## 4. Primeiro corte: baseline P-A

O primeiro incremento P-A implementa o recorte de avanço limitado:
- construção de cenário e estado de domínio;
- bounded advance até target sob budget/chunk configurável;
- varredura completa de obrigações em cada fronteira de budget;
- catch-up sem ultrapassar safe horizon;
- digest canônico de progresso por participante.

O P-A atual ainda NÃO implementa publicação/cancelamento de deadline, lifecycle de dispatch, observação/commit transacional nem snapshot/replay. Esses casos são pendências explícitas, não cobertura alegada. A suíte atual compara classe de resultado, target solicitado, corte alcançado, obrigações preservadas e progresso/horizonte/razão de relógio de cada participante com R2. Inclui 48 combinações determinísticas de fronteira, invariância de partição e casos de obrigações comutativas/resolvidas/vazias. Captura detalhada de trace ainda não está implementada neste primeiro corte. Os contadores de inspeções/chunks são métricas internas experimentais, não saídas normativas do oráculo.

A baseline será medida primeiro com um único chunk até safe horizon e depois com chunks menores. O tamanho do chunk é parâmetro de workload, não constante de semântica. O benchmark registra chunks, participantes tocados/inspecionados e obrigações varridas. Isso evidencia trabalho interno, sem afirmar que essas contagens representam ciclos físicos.

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

O primeiro corte P-A implementará o subconjunto W00–W05; W06–W09 entram assim que seus workloads puderem ser representados de forma fiel sem ampliar premissas físicas. P-D/P-E não devem ser implementados antes de P-A validar a fixture e o trace de comparação.

## 6. Protocolo de benchmark

- Gerar fixtures e entradas antes da região cronometrada.
- Warm-up 5 execuções; medir 30 repetições independentes por ponto de parâmetro (revisável se variância exigir mais). A coleta inclui wall time e CPU time por amostra; pico tracemalloc é medido em probe separado, fora das amostras cronometradas.
- Fixar seed, versão de Python/toolchain, CPU, sistema, governor/power plan, frequência efetiva se disponível, commit SHA, configuração, tracing e flags.
- Executar cada variante em processos/ordens alternadas para reduzir viés de aquecimento e drift; guardar amostras brutas, não só resumo.
- Medir wall-clock e CPU time; separar geração de workload, validação do trace, serialização JSON e hashing do benchmark principal.
- Reportar throughput, ns/op e ns por classe; p50/p95/p99 e máximo observado; peak/retained memory (tracemalloc ou ferramenta mais apropriada); alocações/bytes quando a instrumentação não distorcer a medida.
- Repetir com tracing ligado/desligado; tracing não pode alterar resultado funcional.
- Guardar medição por operação e dados brutos, ambiente e hashes como artefatos CI com retenção; não comparar números de máquinas diferentes como se fossem A/B. O throughput deve derivar de contadores reais de inspeção, nunca de soma arbitrária de entidades do fixture.
- Nenhum limite de overhead é escolhido antes da primeira baseline P-A; benchmark é evidência de custo, não critério para enfraquecer contrato.

## 7. Critérios de equivalência e descarte

Uma execução é funcionalmente comparável somente se:
1. todas as pré-condições e semântica R1 forem implementadas no escopo declarado;
2. trace/digest dos resultados coincidirem com o oracle R2 para cenários suportados;
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
- validar W00–W05, trace e JSON de resultados;
- publicar baseline A reprodutível com SHA/ambiente e dados brutos.

P-E só começa após o mesmo conjunto de fixtures e baseline A congelados. P-F exige gap demonstrado que não possa ser resolvido em P-A/P-D/P-E individualmente.

R3 só conclui com matriz comparativa equivalente, custos e limitações explícitos, decisão sustentada (incluindo possibilidade de rejeitar todos e revisar hipótese) e revisão adversarial R4. R5 decide a arquitetura de produção. Não há promoção automática de protótipo a runtime.
