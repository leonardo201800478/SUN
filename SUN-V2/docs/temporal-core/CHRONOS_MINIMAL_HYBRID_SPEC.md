# CHRONOS — proposta arquitetural do híbrido mínimo

**Projeto:** SUN V2

**Componente:** CHRONOS — Temporal Core

**Máquina inicial:** Sega Master System (SMS)

**Status:** proposta arquitetural consolidada para especificação e prototipagem; **não aprovada para produção**

**Issue:** [#1267 — SUN V2 — R0: pesquisa e especificação do Temporal Core](https://github.com/leonardo201800478/SUN/issues/1267)

> Este documento consolida a direção recomendada após a avaliação arquitetural e estratégica. Ele não declara comprovadas as fases de hardware ainda em investigação e não autoriza a implementação do crate de produção.

## 1. Decisão arquitetural proposta

O SUN V2 deve investigar e, se os gates forem aprovados, implementar um **núcleo temporal determinístico de híbrido mínimo**.

A proposta não é criar um scheduler que execute cada chip a cada tick, nem uma fila global que represente toda mudança interna do hardware. Também não é permitir que cada dispositivo avance livremente e corrigir a defasagem depois.

O CHRONOS deve ser a autoridade única sobre:

- o tempo virtual confirmado da máquina;
- os limites temporais seguros de avanço;
- a coordenação causal entre componentes;
- a ordenação determinística de eventos que precisam de despacho;
- os pontos em que a máquina pode pausar, capturar estado ou retomar.

A propriedade do estado permanece distribuída por responsabilidade:

- a CPU possui seu estado arquitetural e sua microsequência;
- o barramento possui as transações em andamento e seus efeitos pendentes;
- VDP, PSG e YM2413 possuem registradores, contadores, fase local e regras internas;
- o host possui filas e relógios físicos, mas não determina o tempo virtual.

**Princípio de projeto:** centralizar a autoridade temporal sem centralizar artificialmente o estado dos dispositivos.

## 2. O problema que o CHRONOS precisa resolver

Uma máquina SMS não é uma coleção de componentes independentes que podem avançar por quantidades arbitrárias de tempo e sincronizar apenas no fim de cada frame. Uma operação da CPU pode depender de um valor atualizado do VDP; uma transação de barramento pode ser prolongada por WAIT; um evento de vídeo pode alterar uma IRQ; os chips de áudio precisam preservar divisores e fase mesmo quando o host atrasa a reprodução.

O núcleo temporal deve, portanto, responder a quatro perguntas distintas:

1. **Que horas são na máquina?** Representação exata e monotônica do tempo virtual.
2. **Até quando cada componente pode avançar?** Limite seguro determinado pelas dependências e pelos próximos deadlines relevantes.
3. **Quando um estado precisa estar atualizado?** Pontos de observação e sincronização causal.
4. **O que precisa acordar a execução?** Eventos autônomos que não podem esperar pelo próximo acesso ou avanço em lote.

Essas perguntas não devem ser fundidas em uma única abstração. A representação do tempo não determina a estrutura da agenda; a agenda não determina a granularidade de execução; e a granularidade não substitui a semântica de causalidade.

## 3. Os três mecanismos temporais

O híbrido mínimo contém somente três mecanismos de coordenação. Cada mecanismo precisa ter uma responsabilidade exclusiva e uma justificativa verificável.

### 3.1 Avanço limitado até um timestamp seguro

Um componente pode processar um intervalo em lote até um limite autorizado pelo CHRONOS.

O limite seguro é o menor instante imposto pelas restrições ativas, por exemplo:

- próximo deadline autônomo conhecido;
- próxima fronteira de barramento que possa alterar uma observação;
- próximo instante em que outro componente precise observar um estado atualizado;
- limite solicitado por uma operação suspensa ou pela máquina.

O avanço em lote reduz a necessidade de coordenação por unidade mínima. Ele só é válido quando o componente consegue processar internamente o intervalo sem ocultar uma transição que outro componente deveria observar antes do limite.

O CHRONOS não deve escolher automaticamente frame ou scanline como limite universal. A granularidade decorre da menor dependência causal ativa.

**Regra de segurança:** nenhum dispositivo avança além de um instante no qual uma mudança sua possa ser necessária para decidir uma operação de outro componente.

### 3.2 Sincronização sob demanda nos pontos de observação

Quando um componente precisa observar ou modificar um estado pertencente a outro dispositivo, a dependência deve ser sincronizada até o timestamp causal da operação.

Exemplos de pontos que podem exigir sincronização:

- leitura ou escrita de registrador do VDP;
- acesso a portas de I/O;
- leitura de contador ou flag dependente do instante;
- acesso a memória compartilhada ou sujeita a contenção;
- consulta de linha de interrupção;
- operação que dependa do estado atualizado de um chip de áudio ou vídeo.

A sincronização não pode ser uma chamada opcional espalhada pelo código. Cada caminho de observação deve declarar sua dependência temporal e passar por um contrato auditável.

O mecanismo sob demanda não substitui deadlines: ele só atualiza um dispositivo quando alguém precisa observar seu estado. Eventos que precisam ocorrer mesmo sem observador exigem o terceiro mecanismo.

**Regra de segurança:** antes de uma leitura, o estado observado deve estar atualizado até o instante definido pelo contrato do barramento; antes de uma escrita, o efeito deve ser aplicado na fase temporal correta e exatamente uma vez.

### 3.3 Agenda seletiva de deadlines autônomos

Uma agenda explícita deve existir somente para eventos que precisam interromper, acordar ou limitar o avanço sem depender de uma observação externa.

Exemplos candidatos, sujeitos à especificação do SMS:

- vencimento de um deadline de interrupção;
- transição autônoma que altera uma linha de IRQ;
- limite de temporização que não pode ser atravessado por um avanço em lote;
- evento de dispositivo que exige ação causal em um instante definido.

A agenda não deve receber cada dot, cada T-state ou cada transição interna de áudio por padrão. Eventos periódicos internos podem permanecer no estado e no algoritmo local do dispositivo quando isso preservar a mesma semântica e for mais eficiente.

A estrutura de dados — heap, vetor ordenado, timing wheel, agenda local ou outra — permanece em aberto até benchmark. A decisão deve considerar densidade de eventos, inserção, cancelamento, despacho, snapshot, memória e pior caso.

**Regra de segurança:** todo evento agendado tem um único proprietário semântico, timestamp definido e política determinística de desempate.

## 4. CPU e barramento: transações suspensíveis, não um quarto scheduler

A capacidade de suspender e retomar uma operação Z80/barramento é um contrato de execução, não um mecanismo temporal independente.

Uma instrução pode precisar continuar em andamento enquanto a transação aguarda uma fase de barramento, WAIT ou outro limite observável. A CPU deve preservar sua microsequência e o barramento deve preservar a transação pendente.

O contrato precisa representar, no mínimo:

- identidade e tipo da operação;
- instante de início;
- fase atual e próxima fronteira relevante;
- instante de amostragem de dados, quando aplicável;
- estado de WAIT e condição de retomada;
- efeitos já realizados e efeitos ainda pendentes;
- conclusão exatamente uma vez.

A suspensão não pode repetir leitura, escrita, acknowledge, alteração de registrador ou qualquer outro efeito parcial. O restore de uma operação pendente deve produzir o mesmo resultado de uma execução contínua.

A granularidade exata — T-state, M-cycle, fase de ciclo ou outra fronteira — depende da evidência de hardware e dos testes discriminantes. Não deve ser fixada por conveniência da implementação.

## 5. Propriedade dos dispositivos e fronteiras de módulo

O CHRONOS não deve conhecer os detalhes internos de registradores ou algoritmos de cada chip. Sua interface deve tratar de tempo, limites, dependências, deadlines e ordenação.

| Subsistema | Responsabilidade própria | Relação com CHRONOS |
|---|---|---|
| Z80 | Estado arquitetural, execução e microsequência | Solicita avanço e pode suspender em fronteira temporal |
| Barramento | Transações, amostragem e efeitos | Expõe fases e conclusão retomável |
| RAM/VRAM | Dados e regras de acesso | Participa dos pontos de observação e contenção aplicável |
| VDP | Counters, raster, registradores, IRQ e evolução de vídeo | Avança até limite seguro e expõe deadlines relevantes |
| PSG | Contadores, divisores, ruído e fase | Avança intervalos preservando continuidade |
| YM2413 | Slots, divisores, estado FM e fase | Avança intervalos preservando continuidade |
| Mixer/resampler | Conversão de eventos/estado em blocos PCM | Consome dados temporizados sem comandar o clock virtual |
| Vídeo/host | Composição, filas e apresentação | Consome frames; não altera o tempo da máquina |
| Áudio/host | Buffering e saída física | Consome PCM; underrun não acelera nem pausa o clock virtual |

Um dispositivo deve poder ser testado isoladamente com um relógio/limite controlado. A autoridade global não deve se transformar em singleton que torna todos os testes dependentes da máquina inteira.

## 6. Tempo virtual e domínios de clock

A representação numérica ainda não está decidida.

As alternativas incluem inteiro de referência, inteiro amplo, razão racional, contadores locais com fase residual ou uma combinação. A seleção depende da pesquisa das frequências, relações e fases reais do SMS NTSC/PAL, além de requisitos de sessões longas e save states.

A solução escolhida precisa garantir:

- monotonicidade do tempo confirmado;
- conversão entre domínios sem deriva acumulada;
- conservação de fase ou de resto de conversão;
- overflow detectado ou impossibilidade demonstrada dentro do intervalo suportado;
- serialização e restauração do estado temporal;
- custo compatível com o hot path.

Frequências nominais relacionadas matematicamente não provam, por si só, alinhamento de fase física. Frequência, divisor, fase inicial, revisão de hardware e incerteza devem ser registrados separadamente.

Nenhuma frequência ou relação ainda classificada como hipótese deve virar constante imutável da API.

## 7. Causalidade e eventos simultâneos

A ordenação de eventos no mesmo timestamp precisa ser explícita. Ordem incidental de iteração, endereço de memória ou ordem de inserção não documentada não é aceitável.

A especificação deverá avaliar uma chave lógica do tipo:

**(timestamp, phase, stable_sequence)**

- `timestamp`: instante no tempo virtual;
- `phase`: fase definida pelo contrato de hardware, ainda a investigar;
- `stable_sequence`: desempate determinístico dentro da mesma fase.

Essa tupla é uma hipótese de representação, não uma decisão final sobre as fases do SMS.

Também é necessário definir:

- se um evento pode agendar outro no mesmo timestamp;
- quando o novo evento pode ser processado;
- como evitar ciclos infinitos de eventos no mesmo instante;
- como impedir que um evento retroceda o tempo confirmado;
- como lidar com cancelamento e substituição;
- como garantir que o resultado não dependa da ordem incidental de execução dos módulos.

Não presumir que CPU, VDP e áudio tenham uma prioridade universal. A ordem deve refletir a fase real de hardware quando conhecida; quando desconhecida, deve permanecer explicitamente incerta e ser alvo de teste.

## 8. Host, paralelismo e experiência em tempo real

O tempo virtual deve ser independente de VSync, WGPU, Slint, CPAL, agendamento do sistema operacional e atrasos de apresentação.

A apresentação estável a 60 FPS é um requisito do pipeline completo, não uma propriedade isolada do CHRONOS. A meta de throughput headless de 120 FPS com YM2413 ativo em OutRun, registrada em PERF-01, também deve ser medida separadamente. FPS apresentado, throughput headless e input-to-photon não são métricas intercambiáveis.

O CHRONOS pode fornecer timestamps e pontos de aplicação de entrada, mas a latência ponta a ponta precisa medir captura, aplicação, execução, publicação, fila e apresentação.

A execução causal entre componentes deve começar serial e determinística. Paralelismo fica restrito a tarefas que possam demonstrar independência temporal e troca de dados imutáveis ou versionados, como processamento de blocos de áudio, preparação de vídeo e I/O. Nenhum worker pode alterar retroativamente o estado virtual nem bloquear de forma imprevisível o caminho causal.

Corrotinas podem ser avaliadas como técnica de implementação. Não são requisito arquitetural nem sinônimo de paralelismo.

## 9. Save state, restore e replay

O estado necessário para continuar a execução precisa incluir, conforme o modelo final:

- tempo virtual confirmado e fase/conversões dos domínios;
- deadlines e eventos pendentes;
- sequência estável de ordenação;
- estado interno de cada dispositivo;
- microsequência da CPU;
- transações de barramento pendentes e efeitos parciais;
- estado de entrada necessário para replay.

O formato persistido deve ser separado das estruturas internas de runtime. Deve conter versão, perfil/modelo da máquina e política explícita de migração ou rejeição.

Um snapshot não pode ser aceito se omitir informação necessária para reproduzir a execução. Restore seguido de replay deve ser comparado com execução contínua, inclusive quando há eventos e transações pendentes.

## 10. Invariantes de aceitação

Os invariantes abaixo são condições obrigatórias para qualquer protótipo ou implementação:

| ID | Invariante | Evidência exigida |
|---|---|---|
| T-01 | Tempo confirmado nunca retrocede | Testes de monotonicidade |
| T-02 | Nenhum efeito observa causa futura | Testes causais e contraexemplos |
| T-03 | Mesma entrada/estado produz resultado observável idêntico | Replay determinístico |
| T-04 | Conversão preserva fase/resto | Testes de aritmética temporal |
| T-05 | Overflow não ocorre silenciosamente | Testes de limite e sessão longa |
| T-06 | Eventos simultâneos têm ordenação estável | Testes de permutação e desempate |
| T-07 | Suspensão/retomada não duplica efeitos | Testes de transação parcial |
| T-08 | Nenhum dispositivo ultrapassa limite inseguro | Testes de deadlines |
| T-09 | Particionamento permitido preserva resultado | Comparação bloco único vs blocos menores |
| T-10 | Snapshot contém estado pendente necessário | Restore em pontos intermediários |
| T-11 | Host não controla tempo virtual | Testes com pacing e atrasos artificiais |
| T-12 | Eventos e dados críticos não são descartados silenciosamente | Testes de saturação/backpressure |

Os testes de referência devem ser independentes do algoritmo que será escolhido. O protótipo final não pode servir como seu próprio oráculo.

## 11. Estratégia de prototipagem: funil, não catálogo

Não é necessário implementar seis schedulers completos. Cada protótipo deve responder uma pergunta discriminante.

### P-A — Baseline simples

Avanço por instrução/orçamento, com sincronização explicitamente instrumentada. Serve para estabelecer custo mínimo e revelar quais dependências impedem a abordagem.

### P-D — Agenda de eventos

Avalia deadlines e ordenação explícita. Deve medir agenda esparsa e densa, cancelamento, simultaneidade, snapshot e pior caso.

### P-E — Sincronização sob demanda

Avalia custo e correção dos pontos de observação. Deve demonstrar cobertura completa das dependências e provar que catch-up não atravessa evento que deveria ser observado.

### P-F — Híbrido mínimo

Só deve ser prototipado se P-A, P-D e P-E demonstrarem que a combinação resolve uma lacuna real. Deve ser comparado com os modelos simples sob a mesma semântica e a mesma carga.

### P-B e P-C — Condicionais

- P-B (quanta/interleave) só entra se quantum permanecer uma alternativa real para algum subsistema.
- P-C (corrotinas) só entra se houver benefício claro de expressão/retomada que justifique custo, serialização e manutenção.

Cada protótipo exige hipótese, cenário discriminante, métricas e regra de descarte definidos antes de ser implementado. Protótipos são descartáveis e não devem ser promovidos diretamente a código de produção.

## 12. Plano de execução e gates

### Começo — fechar evidência e contratos

1. Criar matriz de evidências por requisito: afirmação, fonte primária, revisão de hardware, confiança, divergência, impacto e teste discriminante.
2. Resolver ou classificar as relações e fases dos clocks SMS NTSC/PAL.
3. Mapear observações Z80/barramento, WAIT, IRQ/NMI e efeitos exatamente uma vez.
4. Identificar eventos autônomos reais de VDP e áudio.
5. Definir semântica provisória de avanço, observação e deadline.
6. Construir modelo de referência independente.
7. Estabelecer baseline headless com ROM, hash, perfil e ambiente fixos.
8. Submeter a especificação a revisão adversarial.

**Saída:** R0–R2 aprovados. Nenhum crate de produção nesta fase.

### Meio — comparar e integrar

1. Executar P-A, P-D e P-E.
2. Implementar P-F somente se os resultados justificarem.
3. Validar causalidade, eventos simultâneos, WAIT e restore.
4. Comparar custo por avanço, sincronização e evento, alocações, memória, p50/p95/p99 e pior caso.
5. Integrar CPU/barramento com transações retomáveis.
6. Integrar VDP e depois PSG/YM2413, mantendo testes por subsistema.
7. Validar NTSC/PAL e jogos reais.
8. Medir 60 FPS de apresentação, throughput headless e input-to-photon separadamente.
9. Obter revisão independente e registrar ADR de aprovação.

**Saída:** R3–R5 aprovados. Só então iniciar ou consolidar a implementação de produção.

### Fim — estabilizar e tornar sustentável

1. Testar sessões longas, limites numéricos e overflow.
2. Validar snapshot/restore com eventos e transações pendentes.
3. Executar replay determinístico e regressões.
4. Testar catálogo representativo de jogos e ROMs de diagnóstico.
5. Validar estabilidade de áudio, vídeo e frame pacing em carga real.
6. Documentar contratos, limites, diagnóstico e política de versionamento.
7. Fazer piloto de generalização para SG-1000 somente após SMS estável.
8. Manter política de regressão proporcional a cada subsistema alterado.

**Saída:** núcleo temporal explicável, testável, recuperável e sustentável. Compilação ou microbenchmark isolado não constituem conclusão.

## 13. Orçamento de complexidade e performance

A baseline deve preceder qualquer limite numérico arbitrário. Depois dela, cada mecanismo terá custo atribuído e orçamento justificável.

Métricas mínimas:

- tempo de CPU do CHRONOS separado do tempo dos dispositivos;
- custo por avanço, sincronização e despacho;
- eventos e sincronizações por frame;
- alocações e bytes alocados no hot path;
- memória por domínio e evento pendente;
- p50, p95, p99, pior caso e jitter;
- desempenho com agenda esparsa e densa;
- custo de snapshot/restore;
- impacto de tracing ligado/desligado;
- desempenho headless e frame pacing em integração real.

Se a complexidade adicional não produzir ganho de fidelidade, correção ou desempenho mensurável, deve ser removida. Se houver regressão de desempenho em carga antes sustentável, novas features ficam bloqueadas até identificação e correção do gargalo.

## 14. Decisões deliberadamente abertas

Este documento **não** escolhe antecipadamente:

- u64, u128, racional ou representação híbrida;
- frequência-base e fase física definitiva;
- heap, vetor, árvore, timing wheel ou agenda local;
- fases exatas de eventos simultâneos;
- granularidade mínima de suspensão do Z80;
- fronteiras subscanline do VDP;
- quais deadlines merecem agenda explícita;
- corrotinas ou execução explícita;
- paralelismo por dispositivo;
- formato final e migração de save states;
- limites numéricos de overhead.

Cada decisão será fechada por evidência, teste discriminante, benchmark e revisão. Manter uma decisão aberta é preferível a transformar uma hipótese em contrato permanente.

## 15. Riscos e critérios de veto

Bloquear avanço de fase quando ocorrer qualquer um dos seguintes:

- perda de causalidade ou efeito duplicado;
- divergência entre execução contínua e restore/replay;
- fase de hardware assumida sem evidência adequada;
- dependência de timing do host para a máquina virtual funcionar;
- regressão inexplicada em ROM de referência;
- underruns recorrentes atribuíveis à arquitetura temporal;
- overhead que elimine a margem necessária de desempenho;
- mecanismo redundante sem benefício mensurável;
- impossibilidade de explicar ou testar a propriedade de um evento.

## 16. Veredito

A direção recomendada é **prosseguir com CHRONOS como híbrido mínimo, ainda sob validação**.

A formulação arquitetural passa a ser:

> Construir o menor mecanismo temporal determinístico que consiga preservar a causalidade do SMS, sincronizar somente onde existe dependência observável, despachar somente os deadlines autônomos necessários e demonstrar fidelidade, desempenho e recuperabilidade por testes independentes.

A decisão não é “usar um scheduler híbrido porque parece flexível”. É provar, com o SMS como primeiro caso, que os três mecanismos são necessários, não redundantes e mais eficientes em custo total do que as alternativas simples.

**R0:** aprovado no escopo documental em 2026-11-24; ver R0_CLOSEOUT.md.

**Próximo passo autorizado:** formalizar os contratos R1 e preparar o modelo de referência independente R2.

**Próximo passo não autorizado:** implementar o CHRONOS de produção antes de R5.
