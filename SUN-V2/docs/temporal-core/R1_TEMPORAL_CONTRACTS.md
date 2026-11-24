# CHRONOS — contratos temporais R1

**Projeto:** SUN V2

**Sistema inicial:** Sega Master System (SMS)

**Gate:** R1 — especificação verificável

**Status:** rascunho técnico v0.3; revisão normativa ampliada, ainda não aprovado

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

### 3.4 Contratos das operações temporais

| Operação conceitual | Pré-condições | Pós-condições obrigatórias |
|---|---|---|
| Solicitar avanço até T | T não é anterior ao watermark já confirmado do dispositivo | O dispositivo avança até T ou para na primeira fronteira/dependência anterior; retorna o instante realmente alcançado e a razão da parada |
| Sincronizar dispositivo em T | T pertence ao perfil ativo e não viola causalidade já confirmada | Todas as transições próprias com instante causal até T foram processadas; nenhuma transição posterior a T foi aplicada |
| Publicar deadline D | Proprietário válido; D não é anterior ao tempo confirmado; identidade/geração é única | O deadline passa a ser responsabilidade do proprietário e pode ser despachado uma única vez enquanto a geração for vigente |
| Cancelar/substituir deadline | Identidade e geração correspondem ao agendamento ativo | A geração anterior não pode mais produzir efeito; cancelamento de evento já despachado não desfaz efeito aplicado |
| Iniciar transação | ID não está ativo; request e perfil são válidos | A transação existe em estado pendente, sem aplicar prematuramente leitura/commit |
| Amostrar transação | A fronteira de amostragem foi alcançada e a amostragem ainda não ocorreu | O valor observado é capturado uma única vez e fica associado ao ID da transação |
| Comprometer transação | A fronteira de commit foi alcançada e o efeito ainda não ocorreu | O efeito de escrita/acknowledge é aplicado uma única vez e registrado como comprometido |
| Retomar transação | Existe estado pendente válido e compatível com o request original | A execução continua do primeiro efeito ainda não realizado; nenhum estágio anterior é repetido |
| Capturar snapshot | O corte causal está estável ou todas as operações incompletas estão representadas | O snapshot contém dados suficientes para continuação equivalente |
| Restaurar snapshot | Versão e perfil são compatíveis; invariantes e referências internas são válidas | A máquina restaurada continua de forma equivalente, sem publicar eventos duplicados ou descartar pendências |

A nomenclatura e a forma das APIs permanecem para o desenho posterior. Esta tabela fixa semântica observável, não assinatura de função.

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
8. Dois efeitos não comutativos na mesma fase exigem uma política semântica explícita do perfil, baseada em evidência ou propriedade do dispositivo. A sequência estável só desempata depois que essa semântica estiver definida; ela não pode inventar a ordem física.

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

### 12.1 Comportamento obrigatório dos erros

| Classe de erro | Comportamento exigido |
|---|---|
| Tempo retroativo | Rejeitar a operação antes de mutar estado; não corrigir silenciosamente o timestamp |
| Horizonte seguro ultrapassado | Parar antes da fronteira; se um componente já a ultrapassou, sinalizar violação de invariante e invalidar a execução de validação |
| Razão/conversão inválida ou overflow | Rejeitar perfil/operação antes de publicar avanço parcial; nunca aplicar wrap ou saturação silenciosa |
| ID de deadline duplicado | Rejeitar o novo agendamento, salvo se for explicitamente a mesma geração/atualização idempotente |
| Geração de deadline obsoleta | Descartar antes de qualquer efeito; não despachar o evento antigo |
| Conflito não comutativo com fase UNKNOWN | Rejeitar a ativação do perfil/caso ou marcar a execução como não aprovada; nunca escolher ordem por sequência incidental |
| ID/conclusão de transação incompatível | Rejeitar a conclusão e preservar a transação original para diagnóstico |
| Amostragem/commit repetido | Falha de invariante determinística; não tentar compensar ou repetir o efeito |
| Snapshot inválido/incompatível | Rejeitar restore atomicamente; a máquina ativa não pode ficar parcialmente restaurada |
| Limite de microsteps excedido | Interromper com erro reproduzível, instante, sequência de eventos e proprietário do ciclo |
| Ordem dependente do host | Falha de determinismo; registrar primeiro evento divergente e sequência causal |

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


## 16. Clarificações normativas para tornar os contratos verificáveis

Esta seção resolve ambiguidades sem selecionar representação numérica, estrutura de agenda, modelo de threads, fase física ou granularidade de hardware. Se uma relação física continuar UNKNOWN, a regra abaixo limita a alegação de fidelidade; não inventa uma ordem substituta.

### 16.1 Progresso local, horizonte seguro e watermark confirmado

Os termos abaixo são distintos e MUST NOT ser usados como sinônimos:

- **Progresso local (P_d):** instante até o qual o estado próprio do domínio d foi efetivamente atualizado.
- **Horizonte seguro (H_d):** maior instante até o qual o domínio d pode avançar sem atravessar uma fronteira que ainda possa exigir observação, publicação ou arbitragem causal externa.
- **Watermark confirmado da máquina (W):** corte causal global fechado: todos os efeitos e eventos pertencentes ao perfil ativo com instante causal menor ou igual a W foram processados, ou estão representados como pendências explícitas cuja existência não altera a validade do corte. Um conflito não resolvido que possa mudar o resultado impede fechar W através daquele instante.

Regras normativas:

1. P_d e H_d pertencem ao domínio e podem diferir entre dispositivos. W pertence à máquina/perfil; um dispositivo não pode declarar W global por conta própria.
2. W MUST ser monotônico. Um domínio não pode publicar progresso local superior ao seu H_d vigente.
3. O coordenador só pode confirmar W depois de obter evidência de fechamento causal de todos os participantes relevantes do perfil até W. A regra de participação é parte do perfil e não pode ser inferida da ordem de iteração.
4. W não é necessariamente o mínimo aritmético de todos os P_d: domínios sem dependência causal no intervalo podem não limitar o corte, mas essa independência precisa estar declarada e validada. R1 não escolhe o algoritmo que calcula W.
5. Um evento pendente com instante menor ou igual a W é permitido somente se sua pendência estiver incluída no próprio estado lógico do corte e não puder produzir retroativamente um efeito que deveria ter precedido um efeito já confirmado. Caso contrário, W não pode atravessá-lo.
6. Um pedido de avanço até T com T < W é retroativo e MUST falhar antes de mutar estado. T = W é operação idempotente se não houver nova entrada causal.
7. O instante comum é uma relação comparável definida pelo perfil. Esta definição não exige epoch, tick mestre, inteiro específico, relógio de parede ou fase elétrica comum.
8. Se um conflito não comutativo com fase UNKNOWN impedir fechar o corte, o resultado deve indicar bloqueio/indeterminação; o coordenador não pode avançar W escolhendo desempate incidental.

### 16.2 Contrato de avanço, parada e ausência de progresso

Toda solicitação de avanço MUST retornar uma das classes semânticas abaixo, independentemente da forma da API:

- **Reached(T):** o domínio alcançou T e satisfez as obrigações causais até T.
- **StoppedAt(B, reason):** alcançou B, com B < T, e B é uma fronteira ou deadline que exige coordenação.
- **Blocked(dependency, condition):** não pode avançar até que uma dependência identificada ou condição externa seja satisfeita. O resultado informa o último progresso efetivo.
- **Failed(error):** uma precondição ou invariante foi violada; não é um estado normal de espera.

Regras:

1. Blocked não é falha por si só e não autoriza busy-wait. O chamador deve devolver controle, resolver outra dependência ou propagar o bloqueio.
2. Blocked MUST identificar a dependência/condição que impede progresso e não pode afirmar que T foi alcançado.
3. Repetir a mesma solicitação sem mudança de estado, entrada ou condição MUST produzir o mesmo bloqueio observável; não pode avançar o tempo virtual artificialmente.
4. Se uma cadeia de dependências não possuir evento, proprietário ou condição capaz de liberá-la, o modelo deve reportar deadlock/no-progress como falha de contrato reproduzível. R1 não prescreve algoritmo de detecção.
5. O limite de segurança não pode ser ultrapassado para evitar starvation ou desbloquear outro componente.
6. Nenhum timeout de relógio de parede pode converter um bloqueio temporal em avanço, WAIT ou evento virtual.

### 16.3 Dependências causais e sincronização

1. Toda operação cross-device MUST declarar o conjunto de estados que observa ou altera e o instante causal de observação/commit.
2. A sincronização de um destino em T só é concluída quando suas transições próprias relevantes até T foram aplicadas e nenhuma transição posterior a T foi aplicada.
3. Uma dependência recíproca não autoriza avanço especulativo. Os participantes devem convergir a uma fronteira comum segura, resolver a transação que cria a dependência ou retornar Blocked.
4. A ausência de dependência causal deve ser explícita para o intervalo e os estados envolvidos; não pode ser inferida apenas porque nenhum evento foi publicado.
5. A sincronização não deve avançar componentes não relacionados, salvo se o perfil declarar uma dependência que os torne participantes do mesmo corte.
6. O chamador não pode manter uma transação em estado ambíguo enquanto aguarda sincronização: a transação deve permanecer identificável, retomável e com seus efeitos já realizados registrados.

### 16.4 Conversões entre domínios e fase residual

1. Cada conversão permitida pertence a um perfil e declara origem, destino, razão/frequência aplicável, estado residual necessário e condições de validade.
2. Converter um intervalo em blocos MUST produzir o mesmo estado temporal que converter o intervalo acumulado, desde que perfil, razão e sequência de entradas permaneçam iguais.
3. O resto/resíduo pertence à continuidade da conversão; não pode ser descartado ao encerrar um bloco, trocar o tamanho do bloco ou serializar estado.
4. Uma cadeia A→B→C e uma conversão direta A→C MUST ser equivalentes somente quando a relação declarada pelo perfil e a composição exata das conversões assim garantirem. R1 não presume associatividade, inversibilidade ou equivalência entre caminhos distintos.
5. Conversão reversa só é permitida quando o contrato do perfil a define sem ambiguidade e sem perda de informação necessária. Não se pode reconstruir fase desconhecida por arredondamento.
6. Mudança de perfil, frequência, tolerância ou fase inicial exige fronteira de reconfiguração explícita. Nenhum domínio pode trocar silenciosamente sua razão enquanto conserva resíduo incompatível.
7. Se a informação física de fase inicial/reset for UNKNOWN, o estado pode conservar uma fase configurada/abstrata, mas não pode rotulá-la como fase elétrica medida.
8. Overflow, denominador inválido, conversão fora do domínio declarado ou resíduo inconsistente MUST falhar antes de publicar avanço parcial.

### 16.5 Ciclo de vida de deadlines e ponto de linearização

Cada deadline lógico tem identidade, proprietário, geração vigente, instante causal e efeito. A implementação pode usar outra estrutura, desde que preserve o seguinte ciclo de vida:

- **Scheduled:** aceito pelo proprietário e elegível para despacho quando o instante causal for alcançado.
- **Superseded/Cancelled:** geração invalidada antes do ponto de despacho; não pode mais produzir efeito.
- **Claimed/Dispatched:** o despacho foi iniciado e sua identidade/generation foi validada.
- **Completed:** o efeito definido foi aplicado exatamente uma vez.
- **Failed:** o efeito não pôde ser concluído; o erro e o estado pendente são preservados.

Regras:

1. O ponto de linearização entre cancelamento e despacho é único e observável no contrato: se o cancelamento vencer antes de o evento ser reivindicado para despacho, o efeito não ocorre; depois desse ponto, cancelar não desfaz o efeito.
2. Substituir um deadline incrementa/renova a geração lógica. Qualquer referência antiga deve ser rejeitada antes de seu efeito, mesmo após restore.
3. Identidade não pode ser reutilizada de forma que uma referência antiga se torne válida para outro evento. Se IDs forem reciclados, a geração deve impedir ABA.
4. O proprietário é responsável por reconstruir os deadlines próprios a partir do snapshot; o coordenador não pode inventar eventos ausentes.
5. Um deadline concluído não pode ser despachado novamente por retry, restore ou duplicação de mensagem.
6. Um deadline bloqueado durante despacho deve manter estado suficiente para retomar sem duplicar seu efeito.
7. Cancelar um evento concluído não reverte estado. Compensação, se existir em algum dispositivo, é uma nova operação causal, não cancelamento retroativo.
8. Eventos obsoletos podem ser descartados sem efeito, mas a decisão deve ser determinística e auditável.

### 16.6 Eventos simultâneos, microsteps e progresso

1. Microstep/delta-cycle é ordem lógica de software no mesmo instante, não uma fase física do chip.
2. A sequência de microsteps só pode ordenar dependências causais já estabelecidas ou efeitos comutativos. Não pode resolver conflito físico não comutativo cuja fase seja UNKNOWN.
3. Um evento criado durante o despacho no mesmo instante só pode ser processado segundo uma regra explícita de microstep; não pode ser inserido retroativamente antes de um efeito já confirmado.
4. Cada microstep deve consumir ou resolver uma obrigação identificável: evento despachado, dependência liberada, transação avançada ou erro declarado. Repetir o mesmo estado sem resolver obrigação não conta como progresso.
5. A execução no mesmo instante MUST terminar, bloquear em dependência identificada ou falhar com diagnóstico. Não pode permanecer em ciclo infinito nem avançar o tempo para escapar dele.
6. O limite de recursos/microsteps é um mecanismo de proteção, não uma propriedade física. Seu valor e estrutura permanecem para R2/R3; excedê-lo produz falha determinística, nunca descarte silencioso.
7. O diagnóstico deve incluir instante, proprietário, identidade/generation do evento e cadeia causal mínima que levou ao ciclo.
8. Permutar a ordem de inserção de eventos independentes/comutativos não pode alterar o resultado observável. Para conflitos não comutativos sem ordem validada, o caso permanece não aprovado.

### 16.7 Transações: amostragem, commit e efeitos parciais

A transação é uma operação lógica; amostragem, commit e conclusão são marcos distintos quando o contrato do dispositivo/barramento assim os exigir.

1. A transação mantém estado independente para: tempo base consumido, WAIT acumulado, amostragem realizada, resultado amostrado, cada efeito irreversível comprometido, efeitos pendentes e conclusão.
2. Amostragem e commit MUST ser idempotentes por identidade da transação e identidade do marco/efeito. Repetir o pedido para marco já realizado retorna o resultado registrado ou erro de duplicidade; nunca executa o efeito novamente.
3. Se uma operação possuir vários efeitos observáveis, cada efeito deve ter identidade e estado de conclusão próprios, ou a especificação do dispositivo deve demonstrar que formam um único efeito atômico. R1 não presume atomicidade de múltiplos efeitos.
4. A retomada começa no primeiro marco ainda não realizado. Não pode repetir leitura amostrada, escrita, acknowledge, publicação de IRQ ou qualquer efeito já comprometido.
5. Antes do primeiro efeito irreversível, cancelamento pode encerrar a transação se a operação de hardware permitir. Depois do primeiro efeito irreversível, cancelamento não pode fingir rollback: a transação deve concluir, permanecer retomável ou falhar preservando os efeitos já aplicados.
6. O commit de escrita e a amostragem de leitura não podem ser fundidos quando o contrato de barramento ou a evidência exigir instantes diferentes.
7. Um erro entre efeitos parciais deve preservar a lista exata de efeitos comprometidos e pendentes. O restore retoma apenas os pendentes.
8. Um ID duplicado com conteúdo/request diferente é conflito e MUST falhar. O mesmo ID com conteúdo idêntico só pode ser tratado como retry idempotente se isso estiver definido pelo contrato da operação.
9. O acknowledge de interrupção é uma transação com estado próprio; os TW automáticos documentados pertencem à semântica do Z80 e não são acrescentados genericamente pelo coordenador.

### 16.8 WAIT e fronteiras de amostragem

1. TW automático é duração/comportamento interno definido pela operação Z80 correspondente. Não é um sinal externo e não é sintetizado genericamente pelo CHRONOS.
2. WAIT externo é uma condição de entrada amostrada nos pontos documentados pelo Z80; a transação preserva cada amostragem e cada TW adicional aplicado.
3. WAIT de fixture é entrada do modelo de teste, identificada como tal e reproduzida no log/replay. Não pode ser promovida a evidência de hardware.
4. O contrato deve identificar se o bloqueio está aguardando sinal externo, fixture ou dependência interna. A origem não pode ser perdida no snapshot.
5. Se a fonte de WAIT físico não for conhecida no perfil, a interface genérica pode representá-la como UNKNOWN/inativa para fins de teste, mas o perfil não pode declarar que existe ou que está conectada sem evidência.
6. A quantidade de TW automáticos permanece vinculada ao tipo de ciclo Z80 documentado; não é uma regra universal para todos os ciclos ou dispositivos.

### 16.9 IRQ, NMI e acknowledge

1. Estado de linha, transição/borda pendente, estado de aceitação da CPU e transação de acknowledge são estados distintos.
2. O proprietário da fonte publica transições com identidade e instante causal; a CPU observa a linha apenas no ponto de amostragem definido pela semântica Z80/perfil.
3. Uma transição anterior ao ponto de amostragem não pode ser omitida. Um pulso/borda cuja retenção dependa de latch do dispositivo deve ser preservado segundo o contrato desse dispositivo; R1 não inventa um latch universal.
4. A regra de borda da NMI e a regra de amostragem de INT são responsabilidade semântica da CPU Z80, enquanto a disponibilidade temporal das linhas é coordenada pelo CHRONOS.
5. EI delay, IFF, prioridade NMI sobre INT e acknowledge pertencem ao contrato CPU/barramento documentado; o CHRONOS não os reimplementa.
6. Fontes simultâneas sem precedência comprovada permanecem sem ordenação física presumida. O perfil precisa declarar a política observável ou permanecer não aprovado para o caso.
7. Snapshot preserva nível, bordas/latches efetivamente existentes, pendências de publicação, estado de aceitação e acknowledge em andamento.

### 16.10 Snapshot e restore como operação transacional

1. O snapshot representa um corte causal fechado conforme 16.1. Se o corte não puder ser fechado, a captura deve incluir explicitamente todas as transações, eventos e efeitos parciais que cruzam o corte; não pode omiti-los.
2. O snapshot contém identidade do perfil/revisão, estado de cada domínio, resíduo, watermark, progresso local relevante, deadlines e gerações, microsteps pendentes, CPU, barramento, WAIT, IRQ/NMI, dispositivos e cursor de entradas externas.
3. O estado de apresentação/host não integra o tempo virtual. Buffers de saída já consumidos pelo host só entram no snapshot se forem necessários para reproduzir uma saída digital definida como observável; essa necessidade deve ser especificada separadamente.
4. Restore MUST validar versão, perfil, referências, identidades, gerações e invariantes antes de tornar o estado restaurado visível. Falha de validação não pode deixar a máquina ativa parcialmente modificada.
5. O estado restaurado só é publicado como nova máquina quando todos os componentes necessários aceitaram o mesmo snapshot. A implementação pode preparar estado temporário, mas a forma do mecanismo fica em aberto.
6. Restore não pode revalidar evento cancelado/superseded, repetir efeito comprometido ou perder efeito pendente.
7. Snapshot em memória e formato persistente são contratos de compatibilidade diferentes. Migração de versão só é permitida por regra explícita; na ausência dela, rejeitar é correto.
8. O snapshot deve registrar a versão do modelo/perfil que dá significado aos timestamps e estados; não se presume que snapshots de perfis SMS/SG-1000 sejam intercambiáveis.

### 16.11 Replay, entradas e concorrência

1. O replay identifica ROM/hash, perfil, estado inicial, versão de contrato/modelo, sequência de entradas externas e ordem de aceitação dessas entradas.
2. Uma entrada externa aceita passa a integrar a linha causal. Seu instante e identidade devem ser registrados antes de poder influenciar o estado virtual.
3. Se produtores concorrentes publicarem entradas sem uma ordem causal definida, a ordem de chegada do host não pode ser tratada como verdade física. O ponto de ingestão deve serializar e registrar uma ordem lógica estável, ou a execução deve ser marcada como não determinística/não aprovada.
4. O resultado do replay deve ser independente do escalonamento de threads, ordem de conclusão de tarefas assíncronas e disponibilidade de CPU do host.
5. Segurança de memória (ausência de data race) e determinismo temporal são propriedades distintas; ambas precisam de validação própria.
6. Paralelismo só é admissível quando os dados e resultados publicados têm dependências e versão definidas. Um worker não pode alterar estado virtual confirmado retroativamente.
7. O log de replay deve registrar a primeira divergência com instante, origem, sequência lógica, estado esperado/obtido e identidade da operação causal.
8. Para a mesma sequência de entradas aceita, diferentes particionamentos de avanço e diferentes escalonamentos permitidos devem produzir os mesmos estados e saídas digitais observáveis.

### 16.12 Erros, bloqueios e atomicidade de falha

O resultado de uma operação temporal deve distinguir sucesso, parada normal, bloqueio e falha. As classes abaixo não são intercambiáveis:

- **Blocked:** dependência/condição externa ainda não satisfeita; estado preservado e retomável.
- **InvalidInput:** parâmetro, perfil, ID ou geração inválida; rejeitar antes de mutação.
- **InvariantViolation:** monotonicidade, causalidade, exactly-once, chunk invariance ou estado interno violado; execução de validação inválida.
- **ResourceExhausted:** limite de eventos, microsteps, memória ou capacidade atingido; não descartar silenciosamente nem avançar parcialmente além do último estado consistente.
- **IncompatibleSnapshot:** versão, perfil ou estado restaurado incompatível; rejeição atômica.
- **IndeterminateHardwareOrder:** resultado depende de ordem física UNKNOWN; não declarar compatibilidade para o cenário.
- **InternalFailure:** falha inesperada com preservação do máximo de contexto diagnóstico; não converter em estado válido.

Uma operação que falhe antes de qualquer efeito deve deixar o estado anterior intacto. Se falhar depois de efeitos irreversíveis, deve retornar o estado parcial explicitamente, mantendo a transação retomável ou inválida conforme o contrato; não pode alegar rollback inexistente.

### 16.13 Independência de áudio, vídeo e host

1. O tempo virtual e os estados de PSG/VDP evoluem por contratos internos, não por callback, buffer, VSync ou consumo do host.
2. Drenar, atrasar, descartar ou reprocessar um bloco de saída do host não altera divisor, fase, contador, deadline ou watermark virtual.
3. Se a produção de PCM for assíncrona, cada bloco deve carregar identidade/intervalo temporal suficiente para impedir duplicação, lacuna silenciosa ou reordenação não detectada.
4. Underrun/overrun é condição do pipeline de saída, não instrução para acelerar, pausar ou retroceder a máquina virtual.
5. Diferentes tamanhos de bloco de áudio/vídeo devem preservar chunk invariance para o estado virtual e para as saídas digitais definidas pelo contrato.
6. A relação de clocks de PSG/YM2413 e a fase entre domínios seguem o perfil e a evidência; a independência do host não prova fase física entre chips.

### 16.14 Desempenho, capacidade e observabilidade

1. R1 exige que recursos temporais tenham limite/capacidade e resultado explícito ao esgotá-los; não escolhe a estrutura de dados nem a política concreta de dimensionamento.
2. Nenhum evento, deadline, transação ou entrada causal pode ser descartado silenciosamente por saturação.
3. O caminho comum não deve exigir alocação dinâmica por T-state ou por instrução. Qualquer exceção deve ser identificada, justificada e medida antes de R3.
4. O custo deve ser medido separadamente para avanço, sincronização, publicação, cancelamento, despacho, microsteps, snapshot/restore e replay.
5. Relatar pelo menos p50/p95/p99 e pior caso, volume de eventos/sincronizações, alocações/bytes, memória máxima e comportamento em carga esparsa/densa.
6. Instrumentação detalhada é opt-in. Ativar/desativar tracing não pode mudar resultados, ordem lógica ou estado temporal.
7. Resultados de benchmark devem fixar SHA, perfil, carga, estado inicial, entradas, ambiente, configuração de tracing e método. Headless, frame pacing real, áudio físico e input-to-photon são métricas distintas.
8. O objetivo de 60 FPS é requisito de produto posterior; não é critério de aprovação isolado do R1 e não pode ser inferido de microbenchmark.

### 16.15 Classificação de evidência e proveniência

As afirmações técnicas e os limites de hardware devem usar as seguintes classes:

- **FACT:** afirmação diretamente sustentada por fonte primária identificada ou por contrato normativo já aprovado. Registrar fonte, revisão/página/seção e escopo.
- **OBSERVATION:** comportamento observado em implementação, teste, ROM, medição ou comparação. Registrar SHA, perfil, ambiente, entrada, método e artefato; não generalizar além do cenário.
- **HYPOTHESIS / INFERENCE:** interpretação ou modelo candidato derivado de fatos/observações. Explicitar premissas, alternativas e teste discriminante.
- **UNKNOWN:** informação não estabelecida ou contraditória. Preservar como desconhecida; declarar impacto, bloqueio e evidência necessária.

Regras:

1. Um requisito normativo R1 é uma decisão de contrato, não uma afirmação de que o hardware já foi medido dessa forma.
2. Uma propriedade demonstrada no modelo R2 não se converte automaticamente em FACT físico.
3. Resultado de outro emulador é OBSERVATION de implementação/referência, não prova isolada do hardware.
4. UNKNOWN não pode ser preenchido por default, conveniência de implementação ou ordem incidental.
5. Toda linha da matriz de rastreabilidade deve apontar para a origem do requisito e para a propriedade R2 que poderá falsificá-lo.
6. Se fontes discordarem, registrar a divergência e restringir o perfil/alegação até existir evidência discriminante.

### 16.16 Mapeamento de lacunas normativas para validação R2

| Lacuna fechada | Contratos | Propriedade R2 associada |
|---|---|---|
| Progresso local, horizonte e watermark global | C01, C04, C05 | T01, T04, T05 |
| Bloqueio, dependência circular e ausência de progresso | C04, C05, C14, C15 | T04, T05, T14, T15 |
| Composição de conversões e resíduo | C02, C03, C11 | T02, T03, T11, T12 |
| Ponto de linearização e geração de deadline | C06, C11, C14 | T04, T06, T11, T14 |
| Microstep e progresso finito | C06, C07, C14, C15 | T06, T07, T14, T15 |
| Efeitos parciais e idempotência por marco | C08, C11, C15 | T08, T11, T12, T15 |
| Linha, borda, latch e acknowledge | C09, C10, C11 | T09, T10, T11 |
| Corte causal e restore atômico | C11, C13, C15 | T11, T12, T15 |
| Ordem de ingestão e concorrência | C07, C12, C13 | T06, T12, T13 |
| Saturação e observabilidade sem perturbação | C14, C15, C16 | T14, T15 e benchmarks R3 |

## 17. Matriz de rastreabilidade dos contratos

| ID | Contrato | Propriedade R2 / validação posterior |
|---|---|---|
| R1-C01 | Tempo monotônico e sem wrap silencioso | R2-T01, R2-T15 |
| R1-C02 | Conversão exata entre domínios e conservação de resto/fase | R2-T03 |
| R1-C03 | Chunk invariance | R2-T02 |
| R1-C04 | Avanço limitado e horizonte seguro | R2-T04 |
| R1-C05 | Sincronização antes de observação/efeito causal | R2-T05 |
| R1-C06 | Propriedade, identidade, geração e cancelamento de deadlines | R2-T04, R2-T06, R2-T14 |
| R1-C07 | Ordenação causal e tratamento de fase UNKNOWN | R2-T06, R2-T07 |
| R1-C08 | Transação suspensível com amostragem/commit exactly-once | R2-T08 |
| R1-C09 | TW automático, WAIT externo e fixture separados | R2-T09 |
| R1-C10 | Amostragem INT/NMI e acknowledge Z80 | R2-T10 |
| R1-C11 | Snapshot/restore de estados pendentes | R2-T11, R2-T12 |
| R1-C12 | Independência do host de áudio/vídeo | R2-T13 |
| R1-C13 | Determinismo sob concorrência e replay | R2-T06, R2-T12 |
| R1-C14 | Progresso finito de eventos no mesmo instante | R2-T14 |
| R1-C15 | Erros temporais e estado inválido explícitos | R2-T15 |
| R1-C16 | Custo previsível e hot paths sem alocação por evento/instrução | Benchmark R3; medição integrada da primeira fatia |

## 18. Requisitos de desempenho e observabilidade

R1 não escolhe estrutura de agenda, mas não pode deixar custo e falhas sem contrato:
- o caminho comum de avanço não deve exigir alocação dinâmica por T-state ou por instrução;
- publicação e despacho de deadlines devem ter custo limitado e mensurável; crescimento de agenda deve possuir capacidade/limite e política explícita de falha;
- o estado temporal deve permitir contadores de diagnóstico sem instrumentação obrigatória no hot path;
- tracing detalhado deve ser opt-in e não alterar a semântica;
- benchmark de protótipo deve separar avanço de dispositivo, sincronização, publicação/dispatch, snapshot, alocações e pior caso;
- nenhuma medição headless será tratada como prova de 60 FPS apresentados; frame pacing, CPAL e input-to-photon são métricas posteriores e separadas.

O objetivo de produto continua 60 FPS estáveis (16,67 ms/frame), mas R1 não declara esse objetivo validado. R3/F6 deve medir o runtime e a apresentação reais e interromper expansão de features se houver regressão abaixo da meta.

## 19. Próximos passos

1. Revisão cruzada deste contrato contra CHRONOS_MINIMAL_HYBRID_SPEC.md, CLOCK_DOMAIN_RESEARCH.md e VDP_OBSERVABLE_BOUNDARIES.md.
2. Revisar cada requisito com cenários de gameplay SMS e classificar os casos que realmente exigem sub-T-state.
3. Construir o modelo de referência R2 e propriedades independentes.
4. Só depois comparar estruturas de agenda e protótipos descartáveis R3.
5. Não iniciar crate de produção antes da aprovação R5.
