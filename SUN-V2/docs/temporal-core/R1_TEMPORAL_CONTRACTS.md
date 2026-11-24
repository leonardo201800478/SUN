# CHRONOS — contratos temporais R1

**Projeto:** SUN V2  
**Sistema inicial:** Sega Master System (SMS)  
**Gate:** R1 — especificação verificável  
**Status:** rascunho técnico v0.1; sujeito à revisão e aos testes de referência R2

**Issue:** [#1270 — contratos temporais R1](https://github.com/leonardo201800478/SUN/issues/1270)  
**Pré-requisito:** [R0 aprovado](R0_CLOSEOUT.md) em 2026-11-24  
**Escopo:** contratos normativos, independentes da estrutura de dados e da representação temporal final

> Este documento define o que o CHRONOS deve garantir. Não escolhe heap, vetor, timing wheel, corrotinas, quantum, tipo inteiro, largura de timestamp, epoch ou fase elétrica. Nenhum código de produção é autorizado por este rascunho.

## 1. Objetivo e fronteiras

O CHRONOS é a autoridade única do tempo virtual da máquina. Ele coordena avanço limitado, sincronização sob demanda e deadlines autônomos seletivos. CPU, barramento, RAM, VDP, PSG e futuramente YM2413 mantêm propriedade do próprio estado.

O contrato deve permitir que uma operação observável seja suspensa e retomada sem repetir efeitos. Deve também permitir que um dispositivo avance em blocos quando não há fronteira observável intermediária.

O CHRONOS não:
- implementa instruções Z80 ou lógica interna dos chips;
- chama callbacks por instrução ou por T-state como política geral;
- usa VSync, WGPU, Slint, CPAL, threads do host ou tempo de parede como relógio virtual;
- transforma toda transição interna de dispositivo em evento global;
- presume fase elétrica ou precedência física que não tenha evidência;
- exige uma thread por dispositivo.

## 2. Vocabulário normativo

- **Tempo virtual:** coordenada temporal da máquina, monotônica e independente do host.
- **Domínio de clock:** sequência local de ciclos de um componente, com frequência/razão, fase e perfil próprios.
- **Instante comum:** posição comparável entre domínios dentro de um perfil de máquina.
- **Fase:** posição relativa de um evento dentro de um instante comum, somente quando definida por evidência ou contrato validado.
- **Fronteira observável:** instante em que uma leitura, escrita, IRQ, saída de porta, pixel, amostra ou outra interação suportada pode mudar o resultado de gameplay.
- **Deadline autônomo:** próxima transição futura que um dispositivo precisa publicar mesmo sem acesso externo.
- **Horizonte seguro:** maior instante até o qual um dispositivo pode avançar sem ocultar uma fronteira que outro componente precise observar.
- **Sincronização sob demanda:** avanço de um dispositivo até o instante necessário imediatamente antes de uma observação ou efeito causal.
- **Transação pendente:** operação de barramento iniciada, mas ainda não amostrada, comprometida ou concluída.
- **Resto/fase residual:** informação preservada na conversão entre domínios para evitar deriva por arredondamento repetido.

Os termos **MUST**, **MUST NOT**, **SHOULD** e **MAY** são requisitos normativos equivalentes a obrigatório, proibido, recomendado e opcional.

## 3. Contrato de tempo e domínios

### 3.1 Monotonicidade e exatidão

1. O tempo virtual de uma máquina MUST ser monotônico.
2. Nenhum domínio pode observar o tempo retroceder.
3. Conversões entre domínios MUST preservar resto/fase residual; não podem arredondar cada bloco de avanço de forma independente.
4. A aritmética temporal autoritativa MUST ser determinística e exata para as razões declaradas pelo perfil. Ponto flutuante não pode ser a fonte de verdade do relógio.
5. Overflow, razão inválida, conversão não representável ou perda de fase MUST produzir erro explícito; wrap silencioso é proibido.
6. O contrato não escolhe ainda inteiro de referência, inteiro amplo, razão racional, ponto fixo ou composição dessas opções.
7. A frequência nominal, a tolerância, a fase inicial, o reset phase e a revisão de hardware são campos conceitualmente distintos. Uma frequência não implica fase.

### 3.2 Avanço limitado

Cada dispositivo temporal MUST declarar o próximo limite até o qual pode avançar sem ocultar uma fronteira observável. O CHRONOS solicita um alvo e o dispositivo avança somente até o menor entre:
- o alvo solicitado;
- seu próximo deadline autônomo;
- a próxima fronteira externa que precise ser observada;
- o limite causal imposto por uma transação ou dependência.

O resultado de um avanço deve distinguir pelo menos:
- **alvo alcançado:** o dispositivo chegou ao instante solicitado;
- **parada em fronteira:** o dispositivo parou antes do alvo e informou a fronteira;
- **bloqueado:** uma dependência, transação ou condição externa impede progresso;
- **falha:** estado temporal inválido ou violação de contrato.

O dispositivo MUST informar o instante realmente alcançado. O chamador MUST NOT assumir que o alvo foi alcançado quando o resultado indicar parada ou bloqueio.

### 3.3 Invariância de chunk

Para o mesmo estado inicial, perfil e sequência de entradas, avançar de A até C em uma chamada deve produzir o mesmo estado observável que avançar A→B e B→C, desde que B não altere entradas nem introduza uma observação externa.

Essa propriedade inclui:
- contadores e fase residual;
- estado digital de áudio;
- posição de raster e flags;
- deadlines futuros;
- linhas de interrupção;
- resultados de leitura/escrita.

A propriedade não exige que cada dispositivo execute internamente a mesma quantidade de trabalho em cada partição, apenas que o estado observável final seja equivalente.

## 4. Sincronização sob demanda

Antes de uma operação observar ou alterar outro dispositivo, o CHRONOS MUST garantir que o dispositivo de destino esteja sincronizado até o instante causal definido para aquela operação.

Exemplos:
- leitura de VCounter/HCounter sincroniza o VDP até o ponto de amostragem da leitura;
- leitura de status sincroniza flags e colisões até o instante em que a CPU observa o barramento;
- escrita de porta sincroniza o destino até o ponto de commit definido pelo contrato da transação;
- mudança de linha IRQ é publicada antes do ponto em que a CPU pode amostrá-la.

O instante de início da instrução, o instante de amostragem do dado, o instante de commit do efeito e o instante de conclusão do ciclo não são intercambiáveis. Cada operação que dependa deles MUST nomear o ponto relevante.

A sincronização não deve avançar componentes sem relação causal com a observação. Ela também não deve fazer um dispositivo ultrapassar um deadline ou uma fronteira que ainda precise ser processada.

## 5. Deadlines e propriedade de eventos

1. Cada deadline tem um único proprietário semântico.
2. O proprietário é responsável por agendar, cancelar, substituir e restaurar o deadline.
3. Todo deadline possui instante, identidade estável, geração ou mecanismo equivalente para invalidar agendamentos antigos, e efeito observável definido.
4. Um dispositivo agenda somente transições que possam alterar outro componente ou uma observação suportada. Contadores internos que podem ser avançados em bloco permanecem locais.
5. Cancelar ou substituir um deadline não pode deixar uma geração antiga capaz de produzir efeito.
6. O CHRONOS despacha deadlines em ordem causal determinística.
7. Eventos novos no instante corrente são permitidos apenas sob uma regra explícita de microstep/delta-cycle. O mecanismo deve impedir ciclos infinitos de eventos sem progresso temporal e reportar limite excedido.
8. O algoritmo de agenda permanece em aberto até R3 e comparação de custo.

## 6. Eventos simultâneos e causalidade

A ordenação de eventos deve distinguir:
- instantes diferentes;
- mesmo instante com fases comprovadamente diferentes;
- mesmo instante e mesma fase;
- relação de fase desconhecida.

A chave conceitual pode conter instante, fase e sequência estável, mas essa forma não aprova uma representação concreta.

Regras:
1. Evento posterior não pode ser observado antes de evento causalmente anterior.
2. Fases físicas só podem ser atribuídas a partir de evidência do perfil ou de uma decisão validada por R2/R4.
3. Sequência de inserção é um desempate determinístico de software; ela não prova precedência elétrica.
4. Se dois efeitos no mesmo instante forem comutativos, o modelo pode usar uma sequência estável sem alterar o resultado observável.
5. Se forem não comutativos e a relação física for UNKNOWN, o perfil não pode silenciosamente escolher uma ordem e declarar precisão. O caso permanece explicitamente não resolvido e exige teste discriminante ou restrição do perfil.
6. A ordem de eventos não pode depender do endereço de memória, ordem de alocação, hash randomizado, escalonamento do host ou thread que publicou o evento.
7. Eventos que criem novos eventos no mesmo instante devem ter progresso de microstep bem definido e limite de segurança.

## 7. Transações de barramento suspensíveis e retomáveis

### 7.1 Conteúdo mínimo da transação

Uma transação precisa preservar informação suficiente para ser retomada sem repetir ou perder efeitos:
- identidade estável da requisição;
- origem e destino;
- tipo de operação (leitura, escrita, I/O, acknowledge ou outra operação suportada);
- endereço e dado de escrita, quando aplicável;
- instante de início;
- instante ou regra de amostragem;
- instante ou regra de commit;
- duração base e estados de espera aplicados;
- resultado amostrado, se já ocorreu;
- efeitos já comprometidos e efeitos ainda pendentes;
- motivo de bloqueio/espera e estado de conclusão.

### 7.2 Estados conceituais

A implementação pode usar outra representação, mas deve preservar transições equivalentes:
1. **Criada:** requisição identificada, ainda sem efeitos.
2. **Em andamento:** tempo base do ciclo está sendo consumido.
3. **Aguardando:** progresso suspenso por WAIT ou dependência.
4. **Pronta para amostragem/commit:** a fronteira relevante foi alcançada.
5. **Amostrada:** dado de leitura ou acknowledge foi capturado exatamente uma vez.
6. **Comprometida:** efeito de escrita foi aplicado exatamente uma vez.
7. **Concluída:** todos os efeitos e tempos da operação foram finalizados.
8. **Cancelada:** cancelamento explícito antes de efeito irreversível, com motivo rastreável.

Amostragem e commit são eventos distintos quando o hardware assim exigir. A implementação não pode fundi-los apenas para simplificar o código.

### 7.3 Exactly-once

- Retomar uma transação não pode repetir leitura amostrada, escrita comprometida, acknowledge, alteração de registrador ou publicação de IRQ.
- Uma requisição duplicada com o mesmo ID deve ser rejeitada ou reconhecida como a mesma operação, nunca executada duas vezes.
- Conclusão com ID ou geração incompatível deve falhar explicitamente.
- Snapshot no meio de uma transação deve restaurar o mesmo ponto lógico e o mesmo conjunto de efeitos pendentes.

## 8. WAIT e ciclos Z80

O contrato deve distinguir três fontes, sem as tratar como equivalentes:

1. **TW automático do Z80:** parte da semântica do ciclo CPU. O manual Zilog documenta um TW automático em I/O e dois TW automáticos no acknowledge de INT.
2. **WAIT externo:** sinal ativo baixo, amostrado pelo Z80 na borda de descida durante T2 e em cada TW subsequente. Um WAIT ativo prolonga o ciclo.
3. **WAIT de fixture:** injeção determinística para teste; não é evidência de um sinal físico.

O contrato genérico deve permitir espera externa sem assumir que VDP ou PSG integrado SMS a produz. A fonte física de WAIT do cartridge port permanece UNKNOWN para os perfis sem esquema/medição.

O perfil SG-1000/SC-3000 com PSG discreta é separado e não pode alterar implicitamente o perfil SMS.

## 9. IRQ, NMI e acknowledge

1. Cada dispositivo proprietário de uma linha de interrupção publica transições com instante causal.
2. A linha observada pela CPU deve refletir todas as transições anteriores ao ponto de amostragem da CPU.
3. INT Z80 é amostrada no limite de instrução documentado; NMI é negativa-edge-triggered e reconhecida no fim da instrução.
4. O atraso de uma instrução após EI pertence à semântica Z80, não a uma prioridade arbitrária do Scheduler.
5. O acknowledge de INT é uma transação de barramento própria e contém os dois TW automáticos documentados.
6. O CHRONOS não escolhe precedência entre IRQ de VDP e outras fontes sem contrato do perfil.
7. Snapshot deve preservar nível de linha, bordas pendentes, acknowledge em andamento e estado de aceitação da CPU.

## 10. Áudio e vídeo independentes do host

- VDP e chips de áudio avançam pelo tempo virtual, não pela taxa de apresentação nem pelo consumo de PCM.
- CPAL, WGPU, Slint, VSync, áudio callback, filas do host e SO nunca avançam, pausam ou retrocedem o relógio da máquina.
- A drenagem de áudio é uma leitura/consumo de dados já produzidos; underrun do host não altera fase ou contadores do PSG/YM2413.
- Um dispositivo pode gerar dados em blocos, mas a divisão em blocos deve respeitar chunk invariance.
- O contrato temporal deve permitir fontes de áudio com domínios e taxas diferentes, preservando timestamp/fase antes da mistura.
- A primeira fatia SMS inclui PSG; YM2413 é previsto como domínio separado posterior, sem exigir sua implementação para fechar o contrato inicial.

## 11. Snapshot, restore e replay

Snapshot deve representar um corte causal coerente da máquina e conter, no mínimo:
- tempo virtual confirmado;
- estado de cada domínio e resto/fase residual;
- deadlines pendentes, identidades, gerações e sequência estável;
- eventos ainda não despachados;
- CPU e microsequência pendente;
- transações de barramento, estados de WAIT, amostragens e commits;
- linhas IRQ/NMI e bordas pendentes;
- estado temporal de VDP, PSG e demais dispositivos;
- entradas externas já aplicadas e cursor de replay.

Regras:
1. Snapshot não pode capturar metade de uma transição global sem registrar a pendência que a completa.
2. Restore valida versão, perfil, domínios, deadlines, IDs e invariantes antes de publicar a máquina restaurada.
3. Restore seguido do mesmo replay deve produzir os mesmos eventos, estados observáveis, frames e sequência digital de áudio que execução contínua.
4. Estado de host (device handle, buffer CPAL, swapchain, janela, VSync) não é estado do tempo virtual; é reconstituído por camada própria.
5. DTO em memória e codec persistente versionado são entregas distintas. R1 define a semântica; R2 prova o modelo; implementação persistente fica em fase posterior.

## 12. Determinismo, concorrência e erros

Para mesmo perfil, estado inicial, ROM, entradas e sequência de operações, o resultado deve ser bit-determinístico nos estados e saídas digitais definidos como observáveis.

O contrato não exige execução single-thread nem paralelismo. Qualquer execução concorrente deve publicar eventos por uma ordem lógica determinística e não pode expor data race ou dependência de escalonamento do host.

Erros obrigatoriamente detectáveis:
- avanço retroativo;
- avanço além do horizonte seguro;
- conversão temporal inválida ou overflow;
- deadline com proprietário ausente, ID/generation inválida ou duplicidade;
- evento não comutativo com fase UNKNOWN usado como se a precedência fosse conhecida;
- request duplicado ou conclusão de transação incompatível;
- amostragem/commit repetido;
- restore incompleto ou incompatível com o perfil;
- ciclo infinito de eventos no mesmo instante;
- estado de dispositivo que viola monotonicidade ou chunk invariance.

Erros não podem ser silenciosamente convertidos em avanço, descarte de evento, prioridade fixa ou atraso arbitrário.

## 13. Propriedades exigidas do modelo de referência R2

R2 deve construir um oráculo independente da futura implementação. O modelo pode ser simples e lento; não deve compartilhar o algoritmo de agenda ou a representação temporal candidata que será comparada.

| ID | Propriedade | Cenário mínimo |
|---|---|---|
| R2-T01 | Monotonicidade | Sequências de avanços crescentes, repetidos e inválidos |
| R2-T02 | Chunk invariance | Avanço único contra partições aleatórias do mesmo intervalo |
| R2-T03 | Conversão sem deriva | Domínios com razão inteira e não inteira, incluindo sessões longas |
| R2-T04 | Limite seguro | Deadline interno antes do alvo solicitado |
| R2-T05 | Sincronização causal | Leitura após evento de dispositivo e escrita com commit definido |
| R2-T06 | Ordenação determinística | Eventos em instantes diferentes, fases distintas e mesma fase |
| R2-T07 | UNKNOWN não vira precedência | Eventos simultâneos não comutativos sem fase validada |
| R2-T08 | Exactly-once | Suspensão/retomada antes e depois de amostragem/commit |
| R2-T09 | WAIT | TW automático, WAIT externo modelado e fixture separados |
| R2-T10 | IRQ/NMI | Linha antes/depois do ponto de amostragem, EI delay e acknowledge |
| R2-T11 | Snapshot/restore | Eventos e transações pendentes em cada estado intermediário |
| R2-T12 | Replay | Execução contínua contra restore + mesma sequência de entradas |
| R2-T13 | Host independence | Alterar chunk de áudio/apresentação sem alterar estado virtual |
| R2-T14 | Microstep finito | Evento cria outro evento no mesmo instante, incluindo ciclo infinito |
| R2-T15 | Overflow/erro | Razões, timestamps, IDs e snapshots inválidos |

R2 deve registrar sementes, perfil, sequência de operações, primeiro ponto divergente e estado mínimo para reprodução.

## 14. Critérios de aprovação R1

R1 só pode ser aprovado quando:
- cada contrato deste documento estiver marcado como obrigatório, recomendado ou fora de escopo;
- não houver contradição entre tempo comum, avanço limitado, sincronização sob demanda e agenda de deadlines;
- cada estado de transação tiver regra de entrada, saída, amostragem, commit, cancelamento e restore;
- simultaneidade e fase UNKNOWN não dependerem de ordem acidental;
- todos os erros listados tiverem comportamento definido;
- as propriedades R2 estiverem ligadas a contratos específicos;
- revisão arquitetural confirmar que não foi escolhida prematuramente uma estrutura de dados ou granularidade física;
- os limites de perfil NTSC/PAL/PAL-M e SMS/SG-1000 estiverem preservados.

A aprovação de R1 não aprova R2, protótipos R3 ou código de produção. R3 só começa depois de R1 e R2 aprovados.

## 15. Questões que R1 deve resolver sem inventar hardware

- Como representar relação de fase conhecida, desconhecida e comutativa sem impor ordem física?
- Qual é a menor interface de avanço que permite reportar a próxima fronteira observável?
- Como a CPU informa safe points sem callbacks por instrução?
- Como representar instante de amostragem e instante de commit em transações?
- Como deadlines são invalidados sem cancelamento caro ou evento zumbi?
- Como impedir starvation e ciclos de microstep sem impor custo fixo no hot path?
- Como compor domínios de clock sem perder resto/fase e sem depender de ponto flutuante?
- Como restaurar transações e eventos pendentes sem expor detalhes internos dos dispositivos ao Scheduler?

As respostas devem ser avaliadas por prova, rastreabilidade e custo, não por preferência de implementação.

## 16. Próximos passos

1. Revisão cruzada deste contrato contra CHRONOS_MINIMAL_HYBRID_SPEC.md, CLOCK_DOMAIN_RESEARCH.md e VDP_OBSERVABLE_BOUNDARIES.md.
2. Revisar cada requisito com cenários de gameplay SMS e classificar os casos que realmente exigem sub-T-state.
3. Construir o modelo de referência R2 e propriedades independentes.
4. Só depois comparar estruturas de agenda e protótipos descartáveis R3.
5. Não iniciar crate de produção antes da aprovação R5.
