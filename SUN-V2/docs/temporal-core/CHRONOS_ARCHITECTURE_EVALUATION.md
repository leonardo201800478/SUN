# CHRONOS — SUN Temporal Core

**Nome arquitetural:** CHRONOS

**Componente:** SUN V2 Temporal Core

**Sistema inicial:** Sega Master System (SMS)

**Status:** avaliação arquitetural e hipótese de prototipagem; **não é uma decisão final de implementação**

**Branch:** architecture/sun-v2-foundation

**Issue:** [#1267 — SUN V2 — R0: pesquisa e especificação do Temporal Core](https://github.com/leonardo201800478/SUN/issues/1267)

---

## 1. Missão

CHRONOS é a autoridade sobre o **tempo virtual** da máquina emulada. Sua responsabilidade é coordenar quando cada componente pode avançar, estabelecer a ordem causal dos efeitos observáveis e permitir que a execução seja interrompida e retomada sem perda de estado temporal.

O Chronos existe para que o SMS execute jogos com compatibilidade de gameplay, temporização coerente entre CPU, barramento, VDP e áudio, baixa latência de entrada e resposta visual, frame pacing estável, continuidade de PSG e YM2413 e determinismo reproduzível.

O Chronos não é um chip, não implementa instruções Z80, não renderiza pixels, não sintetiza áudio e não controla a apresentação do host. Ele fornece os contratos temporais usados por esses subsistemas.

### 1.1 Princípio central

> Nenhum componente pode produzir um efeito observável em um instante virtual anterior ao instante em que suas causas já foram processadas.

A regra vale para leituras e escritas de barramento, alterações de registradores, interrupções, mudanças de estado do VDP, transições de áudio, snapshots e replay.

### 1.2 O que o nome não determina

O nome CHRONOS não implica uma fila global de eventos, um heap como estrutura de dados, uma thread por dispositivo, execução obrigatória por T-state, execução obrigatória por scanline, um modelo de corrotinas ou uso de ponto flutuante para representar o tempo.

Essas são decisões técnicas a serem demonstradas por especificação, protótipos e medições.

---

## 2. Escopo e limites de responsabilidade

### 2.1 Responsabilidades

1. Representar o tempo virtual sem deriva acumulada.
2. Coordenar domínios de clock e suas relações de fase.
3. Definir limites seguros até os quais cada componente pode avançar.
4. Ordenar efeitos simultâneos de forma determinística.
5. Permitir que a CPU pare em fronteiras temporais observáveis.
6. Preservar operações de barramento pendentes e WAIT states.
7. Controlar deadlines, eventos e sincronizações entre dispositivos.
8. Fornecer pontos seguros para pause, reset, snapshot e restore.
9. Permitir replay determinístico.
10. Expor métricas de custo e latência sem contaminar o hot path.

### 2.2 Fora do Chronos

- Decodificação e execução de instruções Z80.
- Implementação de registradores, portas, mapeadores ou memória.
- Algoritmos internos do VDP, PSG ou YM2413.
- Renderização, composição WGPU ou apresentação Slint.
- Callback CPAL, mixer ou resampler.
- Polling de teclado/gamepad e políticas do sistema operacional.

Os dispositivos continuam responsáveis por sua semântica interna. O Chronos coordena o instante em que essa semântica pode avançar ou produzir efeitos.

### 2.3 Fronteira de integração

A integração com a máquina deve ser coarse-grained no ciclo de vida e suficientemente fina no contrato temporal:
- ciclo de vida: iniciar, pausar, parar, resetar e carregar ROM;
- execução: avançar até um limite temporal seguro;
- barramento: iniciar, suspender, concluir ou retomar uma transação;
- dispositivos: avançar até um timestamp autorizado;
- observabilidade: coletar métricas fora do hot path sempre que possível.

Não haverá callback do frontend por instrução, M-cycle ou T-state.

---

## 3. Quatro problemas independentes

O desenho do Chronos será avaliado em quatro dimensões que não podem ser confundidas.

### 3.1 Representação do tempo

Define como instantes e durações são armazenados: inteiro, racional, ticks de referência, contadores locais com fase residual ou uma combinação.

### 3.2 Política de avanço e agendamento

Define quem escolhe o próximo limite de execução: agenda central de eventos, deadlines anunciados pelos dispositivos, quanta, sincronização sob demanda, corrotinas ou combinação.

### 3.3 Granularidade de observação

Define em que ponto a execução pode ser interrompida ou sincronizada: frame, scanline, dot, T-state, fase de M-cycle ou operação parcial de barramento.

### 3.4 Ordenação causal

Define a ordem de efeitos que compartilham um timestamp ou dependem uns dos outros: CPU, barramento, VDP, IRQ, PSG e YM2413.

Uma solução pode usar clock mestre inteiro sem possuir fila global. Pode usar uma agenda de eventos e ainda ser imprecisa nos acessos de barramento. Pode executar em corrotinas sem criar paralelismo nativo. A avaliação não usará esses conceitos como sinônimos.

---

## 4. Requisitos temporais do SMS

Os requisitos abaixo são entradas para a especificação. Valores e fases ainda em investigação não devem ser convertidos em constantes definitivas da API.

### 4.1 Domínios de clock

O modelo deve representar pelo menos:
- Z80: T-states, ciclos M e fases de barramento;
- VDP: clocks de chip, dots, linhas e limites de frame;
- PSG SN76489: clock de entrada, divisor interno e fase;
- YM2413: clock de entrada, divisores internos e temporizações de síntese;
- barramento: instante de início, amostragem e conclusão de transações.

A pesquisa atual registra valores nominais NTSC e PAL e uma possível relação inteira entre os clocks de CPU e VDP. A relação candidata de 53.693175 MHz para NTSC e seus múltiplos é **hipótese matemática de representação**, não comprovação da fase física entre todos os sinais.

O perfil de máquina deverá declarar frequência nominal por domínio, fonte documental, divisor ou razão de conversão, fase inicial conhecida, tolerância e incerteza, e revisão/modelo de hardware a que se aplica.

Não se deve presumir que PSG, Z80, VDP e YM2413 compartilham fase apenas porque suas frequências são relacionadas.

### 4.2 NTSC e PAL

A representação precisa cobrir os dois perfis sem deriva acumulada. Não usar arredondamento por frame ou por scanline que acumule erro ao longo do tempo.

A conversão entre domínios deverá preservar resto/fase. Se houver clocks não comensuráveis no modelo adotado, a conversão precisa usar razão racional ou acumulador de fase com erro limitado e demonstrável.

### 4.3 Z80 e barramento

O contrato não pode tratar uma instrução completa como unidade temporal sempre atômica.

A integração deve permitir execução suspensível em fronteiras relevantes de M-cycle/T-state, leitura de dados no ponto definido pelo ciclo, escrita com efeito aplicado uma única vez, WAIT amostrado na fase correta, extensão por estados TW, I/O com estados de espera aplicáveis, amostragem de INT/NMI nos limites do processador e retomada sem repetir leituras, escritas ou efeitos parciais.

O Chronos não implementará o Z80. Ele deve oferecer um contrato capaz de preservar a sequência temporal que o Z80 e o barramento exigem.

### 4.4 VDP

O VDP precisa poder ser atualizado até os limites necessários para leituras de H/V counter, latch e mudanças de estado, IRQs horizontais e verticais, acesso a VRAM/CRAM e contenção quando aplicável, efeitos de raster, scroll e sprites, leitura de flags e registradores e mudanças que possam afetar a imagem exibida.

A documentação disponível não resolve todos os pontos subscanline. Onde a fase exata ainda não estiver confirmada, o contrato deve preservar a possibilidade de refinamento, sem inventar precisão.

### 4.5 Áudio

PSG e YM2413 devem manter contadores, divisores e fase internos contínuos. O Chronos não pode deslocar transições de áudio para limites convenientes de frame se isso alterar a sequência temporal do chip.

A geração de amostras PCM, a conversão para a taxa do host e a saída CPAL pertencem a outros componentes. O relógio virtual não pode acelerar, desacelerar ou saltar porque o callback de áudio atrasou.

---

## 5. Invariantes obrigatórios

Estes invariantes são requisitos de aceitação para qualquer modelo candidato.

| ID | Invariante | Consequência verificável |
|---|---|---|
| C-01 | Monotonicidade | O tempo confirmado da máquina nunca retrocede. |
| C-02 | Causalidade | Um efeito não observa causas futuras nem deixa de observar causas já efetivadas. |
| C-03 | Determinismo | Mesma ROM, perfil, estado inicial e entradas produzem a mesma sequência observável. |
| C-04 | Fase preservada | Conversões entre domínios não descartam silenciosamente resto ou fase residual. |
| C-05 | Sem wrap silencioso | Overflow é detectado ou impossível dentro de limites demonstrados. |
| C-06 | Ordem estável | Eventos simultâneos têm regra de ordenação explícita e reproduzível. |
| C-07 | Sem efeito duplicado | Suspender/retomar não repete leituras, escritas, IRQs ou transições. |
| C-08 | Sem avanço além do limite | Um dispositivo não ultrapassa deadline que possa alterar o resultado de outro componente. |
| C-09 | Particionamento equivalente | Dividir um avanço em blocos menores preserva o resultado quando não há observações intermediárias diferentes. |
| C-10 | Snapshot completo | Estado temporal pendente necessário para continuar é serializável. |
| C-11 | Host desacoplado | Pacing, VSync, WGPU e CPAL não alteram o tempo virtual. |
| C-12 | Sem perda silenciosa | Eventos, transações ou amostras críticas não são descartados para cumprir orçamento de tempo real. |

A especificação R1 deverá converter cada invariante em propriedades executáveis, incluindo contraexemplos.

---

## 6. Modelos concorrentes a avaliar

Nenhum modelo abaixo é aprovado nesta etapa. Os nomes descrevem padrões observados, não implementações universais.

### A — Avanço por instrução / orçamento da CPU

A CPU executa uma instrução ou unidade mínima permitida; os dispositivos recebem depois o tempo consumido e avançam até o timestamp correspondente.

**Pontos fortes:** fluxo simples, baixo custo estrutural potencial, fácil instrumentação e baseline útil para medir sincronização mais fina.

**Riscos:** uma instrução pode atravessar evento relevante; acessos de barramento podem exigir sincronização dentro da instrução; WAIT e operações parciais exigem estado de retomada; catch-up posterior pode ser insuficiente para efeitos que precisam ocorrer antes de uma leitura/escrita.

**Condição de aceitação:** demonstrar que toda fronteira observável pode ser respeitada por interrupção da execução ou sincronização no acesso, sem efeitos retroativos.

### B — Quanta por scanline / interleave

A CPU e os dispositivos avançam em orçamentos temporais curtos, frequentemente delimitados por scanline ou quantum ajustável.

**Pontos fortes:** custo de coordenação potencialmente baixo, estrutura adequada a workloads dominados por linhas e deadlines periódicos simples.

**Riscos:** scanline não é fronteira suficiente para todos os efeitos; correções locais de fase podem proliferar; quantum grande aumenta latência; quantum pequeno aumenta overhead.

**Condição de aceitação:** comprovar tratamento de eventos intra-scanline e acessos de barramento, medindo vários tamanhos de quantum.

### C — Corrotinas cooperativas

Cada componente mantém seu contexto de execução e cede o controle quando precisa sincronizar com outro componente ou quando alcança seu limite temporal.

**Pontos fortes:** fluxo local próximo ao modelo do hardware, pontos de sincronização explícitos e possível clareza para sequências temporais complexas.

**Riscos:** custo de troca de contexto, snapshot complexo, dependência da ordem de yield, ausência de paralelismo nativo implícito e granularidade excessiva.

**Condição de aceitação:** comparar custo por sincronização, complexidade de serialização e clareza dos contratos frente a uma implementação sem corrotinas.

### D — Agenda central de eventos discretos

Os componentes anunciam eventos futuros. O Chronos seleciona o próximo timestamp e despacha os eventos vencidos segundo uma ordem estável.

**Pontos fortes:** deadlines explícitos, ordenação central de eventos simultâneos, boa representação de eventos esparsos e redução de polling.

**Riscos:** custo de inserir, cancelar e despachar; eventos periódicos podem gerar overhead; fila global pode centralizar contenção; agenda correta não resolve automaticamente microtiming de barramento; eventos no mesmo timestamp exigem semântica rigorosa.

**Condição de aceitação:** medir agenda densa e esparsa, cancelamento, ordenação, delta-cycles, snapshot e pior caso.

### E — Sincronização sob demanda / catch-up

Um dispositivo avança apenas quando outro componente precisa observar seu estado atualizado. Acesso a registrador, porta ou memória força a atualização até o timestamp da operação.

**Pontos fortes:** evita trabalho quando estado não é observado, pode reduzir dispatches e alinha sincronização a dependências reais.

**Riscos:** caminhos de acesso podem esquecer a sincronização; dependências implícitas são difíceis de auditar; eventos autônomos exigem deadlines; catch-up não pode atravessar eventos que deveriam ter sido observados antes.

**Condição de aceitação:** mapear todas as dependências observáveis e provar que nenhum acesso lê estado atrasado ou aplica efeitos fora de ordem.

### F — Modelo híbrido: limites + sincronização causal + eventos seletivos

Combina execução em blocos seguros, sincronização sob demanda nos pontos de dependência, deadlines para eventos autônomos e execução suspensível do Z80 nas fases de barramento relevantes.

**Pontos fortes:** adapta granularidade à necessidade de cada dispositivo; evita obrigar todos a gerar eventos a cada tick; pode preservar precisão de barramento sem lockstep global; usa agenda somente onde deadlines explícitos são necessários.

**Riscos:** maior complexidade; possível duplicação de mecanismos; requer contrato claro de propriedade e validade de timestamps; pode tornar-se difícil de entender sem limites rígidos.

**Condição de aceitação:** provar que mecanismos são ortogonais, cada evento tem um único responsável e o custo total é justificável frente aos modelos simples.

---

## 7. Matriz qualitativa inicial

A matriz indica adequação esperada para investigação, não resultado de benchmark nem nota final.

| Modelo | Precisão sub-instrução | Eventos autônomos | Custo potencial | Complexidade | Snapshot/replay |
|---|---|---|---|---|---|
| A — instrução | Baixa a média, salvo refinamentos | Fraca sem deadlines | Baixo | Baixa a média | Média |
| B — quantum | Média, dependente do quantum | Média | Baixo a médio | Média | Média |
| C — corrotinas | Alta, se yields corretos | Boa | Médio, a medir | Alta | Alta |
| D — eventos | Alta para eventos modelados | Forte | Variável | Alta | Alta |
| E — sob demanda | Alta nos pontos sincronizados | Exige mecanismo adicional | Potencialmente baixo | Média a alta | Média a alta |
| F — híbrido | Potencialmente alta | Forte com deadlines seletivos | Variável | Alta | Alta |

**Conclusão:** não há vencedor demonstrado. F é a hipótese de maior interesse porque atende conceitualmente às diferentes granularidades do SMS, mas carrega risco de complexidade. D e E devem ser prototipados como comparadores essenciais; A/B oferecem baselines de custo; C deve ser incluído se custo e serialização puderem ser avaliados de forma comparável.

---

## 8. Hipótese de arquitetura a prototipar primeiro

A hipótese prioritária para R3 é um **coordenador temporal determinístico, com propriedade única do estado causal da máquina**, que combina:

1. **Tempo virtual monotônico:** representação inteira/racional ainda a decidir, com conversão de clock que preserve fase.
2. **Avanço limitado por deadline:** nenhum componente avança além do próximo limite que possa alterar uma observação de outro componente.
3. **CPU suspensível:** execução do Z80 pode parar em fronteiras de barramento necessárias para WAIT, amostragem de dados, IRQ e efeitos de leitura/escrita.
4. **Dispositivos com avanço até timestamp:** VDP, PSG e YM2413 processam intervalos sem exigir dispatch global a cada tick, desde que não atravessem eventos observáveis pendentes.
5. **Sincronização sob demanda:** acessos a estado de dispositivo sincronizam o dispositivo até o instante causal da operação.
6. **Eventos seletivos:** IRQs, deadlines e transições autônomas entram em agenda somente quando precisam acordar ou limitar a execução.
7. **Execução determinística serial no caminho causal:** não introduzir threads nativas entre componentes que compartilham estado temporal sem prova de independência.
8. **Workers desacoplados:** DSP por blocos, conversão de áudio, preparação de vídeo e I/O podem ser paralelizados depois que dados e timestamps forem publicados sob contrato explícito.

Essa hipótese não define ainda a estrutura da agenda, tipo inteiro, uso de corrotinas, ticks por T-state/dot, ordem exata de fases simultâneas ou quais eventos do VDP precisam de dispatch explícito.

### 8.1 Autoridade temporal e propriedade do estado

O Chronos é a autoridade única do tempo virtual confirmado, dos limites de avanço e da política de ordenação causal. Isso **não** significa que ele seja proprietário de todo o estado dos dispositivos.

- CPU possui estado arquitetural e microsequência.
- Barramento possui transações pendentes e efeitos ainda não concluídos.
- VDP, PSG e YM2413 possuem registradores, contadores, fase local e regras internas.
- Host possui filas de apresentação e relógios físicos, sem autoridade sobre o tempo virtual.

O Chronos coordena e valida limites; cada dispositivo mantém a propriedade do próprio estado. Efeitos entre componentes passam por contratos explícitos e não podem alterar timestamps retroativamente.

Workers recebem entradas imutáveis e intervalos/timestamps definidos; publicam resultados identificados por sequência e intervalo. O proprietário do dispositivo valida e integra o resultado em ponto permitido. Se a tarefa não cumprir esse contrato sem bloquear o caminho causal, permanece serial.

### 8.2 Política de avanço

O Chronos calcula um limite seguro a partir do conjunto de restrições ativas:
- próximo deadline autônomo;
- próxima fronteira de barramento;
- próximo ponto em que outro dispositivo pode observar uma mudança;
- próximo limite solicitado pela máquina ou por uma operação suspensa.

O limite não deve ser simplesmente o próximo frame ou a próxima scanline. A granularidade é determinada pela menor dependência causal ativa.

---

## 9. Semântica de eventos simultâneos

A ordem de eventos com o mesmo timestamp é uma das decisões de maior risco. Não se deve usar ordem incidental de iteração, endereço de memória ou ordem de inserção não documentada como regra implícita.

A especificação R1 deverá avaliar uma chave lógica semelhante a:

**(timestamp, phase, stable_sequence)**

Onde timestamp identifica o instante virtual, phase identifica a fase temporal definida pelo contrato de hardware e stable_sequence desempata eventos da mesma fase de maneira determinística.

A tupla é uma **forma candidata de modelagem**, não uma API aprovada.

Perguntas obrigatórias:
1. Um evento pode agendar outro no mesmo timestamp?
2. O evento recém-agendado executa nesta rodada ou na próxima fase?
3. Como impedir ciclos infinitos de eventos de tempo zero?
4. Qual componente tem prioridade quando CPU e VDP mudam estado no mesmo instante?
5. Como uma escrita de CPU coincide com uma transição interna do VDP?
6. Em qual fase uma IRQ passa a ser visível para o Z80?
7. Como restaurar a ordem após snapshot?
8. A ordem é definida pelo hardware, por uma abstração necessária ou por uma aproximação documentada?

Nenhuma prioridade universal CPU > VDP ou VDP > CPU será adotada sem evidência por evento e por fase.

---

## 10. Contrato de transação suspensível

A CPU e o barramento precisam compartilhar um protocolo que preserve progresso parcial.

Uma transação pendente deverá representar conceitualmente:
- identificador/seqüência estável;
- tipo de operação (leitura, escrita, acknowledge ou I/O);
- endereço/porta e dados envolvidos;
- timestamp de início;
- fase/ciclo atual;
- ponto de amostragem;
- estado de WAIT e quantidade de TW;
- efeitos já aplicados;
- condição de conclusão;
- informação suficiente para snapshot e retomada.

O contrato deverá distinguir:
- **solicitação:** a CPU inicia uma operação;
- **espera:** a operação não pode avançar ou concluir ainda;
- **amostragem:** o dado ou sinal é observado na fase correta;
- **efeito:** leitura/escrita produz seu efeito uma única vez;
- **conclusão:** a CPU pode prosseguir;
- **suspensão:** o estado parcial é preservado;
- **retomada:** a operação continua do ponto exato.

Não se deve representar WAIT como sleep da thread do host. WAIT é estado do hardware virtual e precisa consumir tempo virtual de forma determinística.

---

## 11. Tempo virtual versus tempo do host

O Chronos controla exclusivamente o tempo virtual. A camada de host observa e apresenta os resultados.

| Domínio | Responsável | Pode alterar o tempo virtual? |
|---|---|---|
| Clock da máquina | Chronos | Sim, é a autoridade |
| CPU/VDP/PSG/YM2413 | Máquina sob contrato Chronos | Apenas por avanço autorizado |
| Captura de input | Host/input pipeline | Não; fornece entradas em fronteiras definidas |
| Renderização WGPU | Host/video pipeline | Não |
| Apresentação/VSync | Host/window system | Não |
| Callback CPAL | Host/audio pipeline | Não |
| Pacing e sleep do host | Host/runtime | Não |

A latência de input não depende somente do Chronos. Também depende de quando o host captura a entrada, quando ela é aplicada à máquina virtual, se a execução bloqueia esperando apresentação, quantos frames ficam em voo, quando o frame é publicado e da fila de apresentação.

Medir somente o tempo interno do scheduler não prova baixa latência de ponta a ponta.

### 11.1 Métricas obrigatórias de latência

- captura do input até aplicação na máquina;
- aplicação do input até primeira mudança observável de estado;
- aplicação do input até publicação do frame;
- publicação até apresentação;
- latência ponta a ponta input-to-photon, quando houver instrumentação adequada;
- profundidade de frames em voo e política de fila.

### 11.2 Desempenho

A meta de frame estável exige orçamento de 16,67 ms por frame a 60 Hz, mas esse número não é orçamento exclusivo do Chronos. Medir separadamente throughput headless, tempo de CPU do Chronos, tempo de cada dispositivo, tempo de áudio e vídeo, frame time do frontend, p50/p95/p99 e pior caso, jitter, underruns, alocações por frame e latência de input.

Não confundir 60 FPS apresentados com 60 frames/s de throughput de emulação. Um teste headless acelerado e um teste com apresentação em tempo real respondem perguntas diferentes.

---

## 12. Estratégia de clock e representação numérica

A representação final permanece em aberto. Os protótipos deverão comparar:
1. ticks inteiros em uma base comum derivada dos clocks;
2. contadores locais inteiros com conversão racional e fase residual;
3. timestamp inteiro amplo com razões racionais por domínio;
4. alternativa de ponto fixo, somente se houver vantagem mensurável e erro formalmente limitado.

Critérios: ausência de deriva acumulada, custo de conversão no hot path, largura e intervalo, overflow e detecção de erro, simplicidade de snapshot, suporte a NTSC/PAL e futuras variantes e capacidade de representar eventos sub-T-state se forem comprovadamente necessários.

Não escolher u64 ou u128 por preferência. O intervalo deverá ser calculado a partir da frequência, duração máxima de sessão, unidades mínimas e margem para futuras máquinas. Overflow precisa ser impossível dentro do contrato ou tratado explicitamente.

---

## 13. Agenda e estruturas de dados a comparar

A estrutura de dados deve ser escolhida depois de conhecer quantidade média/máxima de eventos, distribuição temporal, frequência de inserção/cancelamento, eventos recorrentes, proporção de deadlines, necessidade de ordenação estável e custo de snapshot/restore.

Protótipos candidatos:
- vetor ordenado;
- heap binário com sequência estável;
- árvore ordenada;
- timing wheel ou estrutura hierárquica;
- agendas locais por subsistema mais coordenador de deadlines;
- ausência de agenda global, com sincronização sob demanda e limites explícitos.

Não assumir que heap é melhor por complexidade assintótica. Para conjuntos pequenos e previsíveis, vetor ou agenda especializada podem ser mais rápidos e simples. A decisão depende do perfil real.

---

## 14. Snapshot, restore e replay

O estado persistente do Chronos deverá incluir tudo o que pode alterar o futuro:
- instante virtual confirmado;
- contadores e fases residuais dos domínios;
- sequência estável de desempate;
- eventos/deadlines pendentes;
- estado de delta-cycle, se existir;
- operações de barramento suspensas;
- fase de execução da CPU, se mantida pelo contrato;
- estado de sincronização necessário para evitar avanço duplicado;
- versão do formato e perfil de hardware.

O formato de save state não deve depender de ponteiros, endereços de memória ou layout interno de estruturas de dados.

Testes obrigatórios:
1. snapshot em repouso;
2. snapshot com agenda não vazia;
3. snapshot durante WAIT;
4. snapshot entre amostragem e conclusão de barramento;
5. restore e continuação comparados à execução contínua;
6. replay da mesma sequência de entradas;
7. snapshots em fronteiras simultâneas;
8. restore em outra execução do processo, quando suportado.

---

## 15. Plano de protótipos comparáveis

R3 não deve comparar implementações com semânticas diferentes. Todos os protótipos receberão os mesmos cenários, modelo de referência e carga de eventos. A execução será em funil: P-A como baseline; P-D e P-E como alternativas centrais; P-F somente se a composição híbrida se justificar. P-B e P-C serão implementados apenas se permanecerem candidatos relevantes após R1/R2.

| ID | Modelo | Objetivo |
|---|---|---|
| P-A | Instrução + catch-up | Baseline simples de custo (obrigatório) |
| P-D | Agenda discreta central | Alternativa central: custo de eventos e ordenação |
| P-E | Sync-on-demand | Alternativa central: economia e custo por acesso |
| P-F | Híbrido seletivo | Executar se a composição demonstrar necessidade |
| P-B | Quantum/deadline por linha | Condicional: somente se quantum continuar candidato real |
| P-C | Corrotinas cooperativas | Condicional: somente se houver hipótese concreta de benefício |

### 15.1 Cargas de benchmark

- agenda vazia ou quase vazia;
- poucos eventos esparsos;
- eventos densos e simultâneos;
- muitos eventos periódicos;
- acessos frequentes CPU→VDP;
- WAIT states repetidos;
- IRQs próximas a fronteiras de instrução;
- PSG e YM2413 ativos;
- carga representativa de jogos SMS;
- execução longa NTSC e PAL;
- snapshot/restore com estado pendente.

### 15.2 Métricas

- ns por avanço;
- ns por evento inserido, cancelado e despachado;
- custo de sincronização por acesso;
- custo por T-state e por transação de barramento;
- p50/p95/p99 e pior caso;
- ciclos de CPU e branch misses, quando disponíveis;
- alocações e bytes alocados;
- memória por evento e por domínio;
- tempo de snapshot/restore;
- throughput headless;
- impacto no frame pacing e na latência de input;
- equivalência funcional/temporal frente ao modelo de referência.

Microbenchmarks não substituem carga integrada. A arquitetura será escolhida pelo custo total e pela correção, não pelo melhor número isolado.

---

## 16. Organização da revisão especializada

A avaliação será conduzida por sete frentes técnicas. São **papéis de análise**, não declaração de que sete agentes independentes foram executados. A revisão adversarial final deverá ser feita por revisor externo/independente quando disponível.

| Frente | Responsabilidade | Pergunta principal |
|---|---|---|
| 1. Arquitetura temporal | Relógio, agenda, causalidade, estado e decisões | O modelo é coerente e mínimo? |
| 2. Z80 e barramento | M-cycles, T-states, WAIT, IRQ/NMI, suspensão | É possível parar e retomar sem alterar semântica? |
| 3. VDP e vídeo | Dots, scanlines, counters, IRQs, VRAM/CRAM | Os eventos observáveis cabem no modelo? |
| 4. Áudio | PSG, YM2413, divisores, fase e continuidade | O tempo de áudio permanece coerente sem acoplar ao host? |
| 5. Desempenho e latência | Hot paths, sincronização, jitter e input-to-photon | O custo é compatível com a meta do produto? |
| 6. Integração e determinismo | Contratos, replay, snapshot e workers | O sistema inteiro continua reproduzível? |
| 7. Revisão adversarial | Contraexemplos, falhas de fase, overflow e ambiguidades | O desenho pode ser refutado por um caso concreto? |

Cada frente deve produzir fatos confirmados e fontes, inferências e hipóteses separadas, riscos e contraexemplos, testes discriminantes, custo provável, recomendação e nível de confiança.

A frente adversarial não deve apenas aprovar o desenho. Deve tentar invalidá-lo.

---

## 17. Riscos arquiteturais prioritários

| Risco | Impacto | Mitigação exigida |
|---|---|---|
| Fase de clock incorreta | Deriva, IRQs ou áudio fora de tempo | Modelo matemático, fonte primária e teste longo |
| Ordem simultânea implícita | Resultados dependentes de ordem incidental | Fases e desempate formalizados |
| Instrução ultrapassa deadline | Leitura/escrita observa estado futuro | CPU suspensível e transações parciais |
| WAIT modelado como bloqueio do host | Não determinismo e jitter | WAIT como estado virtual serializável |
| Catch-up atravessa evento autônomo | IRQ ou transição perdida | Deadline obrigatório para eventos autônomos |
| Agenda excessivamente granular | Overhead e baixa performance | Eventos seletivos e benchmark de densidade |
| Híbrido sem fronteiras claras | Duplicidade e bugs difíceis | Um único dono por evento e contrato de avanço |
| Worker altera estado causal | Corrida e não determinismo | Propriedade única e publicação por timestamp |
| Snapshot incompleto | Divergência após restore | Snapshot no meio de transação e comparação diferencial |
| Métrica confundida com FPS apresentado | Falsa conclusão de performance | Medir throughput, pacing e latência separadamente |

---

## 18. Portões de conclusão

O Chronos só poderá receber implementação de produção após aprovação de todos os portões.

### R0 — Pesquisa
- [ ] Fontes primárias e limitações registradas.
- [ ] Projetos de referência inspecionados em código.
- [ ] Fatos, inferências e hipóteses separados.
- [ ] Divergências de clock e microtiming registradas.
- [ ] Modelos concorrentes documentados.

### R1 — Especificação
- [ ] Unidade e representação temporal justificadas.
- [ ] Clock domains e fases NTSC/PAL definidos.
- [ ] Semântica de eventos simultâneos definida.
- [ ] Contrato de avanço e deadlines definido.
- [ ] Contrato de barramento suspensível definido.
- [ ] WAIT, IRQ e NMI mapeados para fases.
- [ ] Overflow e limites de sessão demonstrados.
- [ ] Snapshot/restore/replay especificados.
- [ ] Propriedades executáveis associadas aos invariantes.

### R2 — Modelo de referência
- [ ] Modelo independente dos protótipos.
- [ ] Testes de aritmética e fase.
- [ ] Testes de simultaneidade e ordenação.
- [ ] Testes de causalidade e particionamento.
- [ ] Testes de suspensão/retomada.
- [ ] Testes de snapshot/restore.
- [ ] Testes de replay determinístico.

### R3 — Protótipos e benchmarks
- [ ] P-A, P-D e P-E implementados como protótipos isolados; P-F, P-B e P-C somente quando justificados por evidência.
- [ ] Mesma semântica e mesma carga nos comparativos.
- [ ] Custo de CPU, memória e alocações medido.
- [ ] Carga densa e esparsa medida.
- [ ] Benchmark integrado SMS executado.
- [ ] Impacto em áudio, vídeo e input medido.
- [ ] Resultados e ambiente reproduzíveis.

### R4 — Revisão adversarial independente
- [ ] Revisor independente recebeu especificação e evidências.
- [ ] Contraexemplos e falhas potenciais foram registrados.
- [ ] Achados críticos resolvidos ou explicitamente bloqueantes.
- [ ] Alternativas e trade-offs reavaliados.
- [ ] Parecer independente anexado.

### R5 — Aprovação arquitetural
- [ ] Modelo vencedor justificado por evidência.
- [ ] Alternativas rejeitadas e razões documentadas.
- [ ] ADR de decisão criado.
- [ ] Escopo do primeiro crate de produção aprovado.
- [ ] Critérios de integração e regressão aprovados.

**Até R5, não criar implementação de produção do Chronos.** Protótipos de pesquisa são descartáveis e não devem se tornar dependência do runtime por acidente.

---

## 19. Fontes de referência e limites conhecidos

As fontes abaixo sustentam a pesquisa comparativa atual. Código de emulador demonstra uma estratégia de implementação, não prova sozinho o comportamento físico do hardware.

### Projetos de emulação

- [MAME — scheduler](https://github.com/mamedev/mame/blob/master/src/emu/schedule.cpp)
- [MAME — driver SMS](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp)
- [MAME — VDP 315-5124](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp)
- [ares — scheduler](https://github.com/ares-emulator/ares/tree/master/ares/ares/scheduler)
- [ares — Master System](https://github.com/ares-emulator/ares/tree/master/ares/ms)
- [Gearsystem — Processor.cpp](https://github.com/drhelius/Gearsystem/blob/master/src/Processor.cpp)
- [Gearsystem — GearsystemCore.cpp](https://github.com/drhelius/Gearsystem/blob/master/src/GearsystemCore.cpp)
- [BlastEm — SMS](https://github.com/bachimusprime/BlastEm/blob/master/sms.c)
- [BlastEm — VDP](https://github.com/bachimusprime/BlastEm/blob/master/vdp.c)
- [Genesis Plus GX — system.cpp](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/system.c)
- [jgenesis — SMS core](https://github.com/jsgroth/jgenesis/tree/master/backend/smsgg-core)
- [Clock Signal — MasterSystem.cpp](https://github.com/TomHarte/CLK/blob/master/Machines/MasterSystem/MasterSystem.cpp)
- [BizHawk — SMS emulator](https://github.com/TASEmulators/BizHawk/blob/master/src/BizHawk.Emulation.Cores/Consoles/Sega/SMS/SMS.IEmulator.cs)

### Documentação técnica

- [Sega Master System Service Manual](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf)
- [Zilog Z80 CPU User Manual UM008011-0816](https://www.homecomputermuseum.nl/wp-content/uploads/2025/02/Z80_CPU_Users_Manual_2016.pdf)
- [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate)
- [SMS Power — SN76489](https://www.smspower.org/Development/SN76489)
- [SMS Power — YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [Charles MacDonald — SMS VDP documentation](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt)

### Documentos internos do SUN V2

- [Plano de pesquisa e especificação](RESEARCH_AND_DESIGN_PLAN.md)
- [Pesquisa comparativa inicial](REFERENCE_RESEARCH.md)
- [Pesquisa de clock domains](CLOCK_DOMAIN_RESEARCH.md)
- [Pesquisa de clocks de áudio](AUDIO_CLOCK_RESEARCH.md)
- [Divergências e qualidade das fontes](SOURCE_DISCREPANCIES.md)
- [Política de validação e revisão](../quality/VALIDATION_AND_REVIEW_POLICY.md)
- [Fundação arquitetural](../architecture/FOUNDATION.md)
- [ADR-0002 — pesquisa antes da implementação](../decisions/ADR-0002-temporal-core-research-first.md)

---

## 20. Estado e próxima ação

**Estado atual:** avaliação arquitetural criada. Nenhum modelo está aprovado. Nenhum código de produção foi autorizado ou implementado.

**Próxima ação técnica:** fechar R0 com auditoria de evidências e transformar os requisitos desta avaliação em uma especificação temporal R1. Em particular, resolver as incertezas de fase dos clocks SMS, os instantes de observação do barramento Z80 e a ordenação de eventos simultâneos antes de escrever o modelo de referência.

A hipótese F (híbrida) é a primeira candidata a prototipar, não uma decisão antecipada. Se os protótipos demonstrarem que um modelo mais simples satisfaz os invariantes com menor custo e complexidade, o SUN deverá escolher o modelo mais simples.
