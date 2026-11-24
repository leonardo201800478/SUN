# CHRONOS — contratos temporais R1

**Projeto:** SUN V2

**Sistema inicial:** Sega Master System (SMS)

**Gate:** R1 — especificação verificável

**Status:** rascunho técnico v0.3.9; corrige R1-EXIT-01 (taxonomia de avanço retroativo) e R1-EXIT-03 (ciclo de vida transacional e mapeamento de marcos); ainda não aprovado

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

Cada dispositivo temporal MUST declarar o próximo limite até o qual pode avançar sem ocultar uma fronteira observável. O progresso local e os limites locais são expressos no domínio do dispositivo; antes de compará-los com o alvo solicitado ou com limites de outros domínios, o perfil deve fornecer uma conversão válida para o instante comum. O CHRONOS solicita um alvo e o dispositivo avança somente até o menor entre:
- o alvo solicitado;
- seu horizonte seguro H_d, convertido ao instante comum por regra válida do perfil;
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
| Solicitar avanço até T | T pertence ao perfil ativo e é expresso como instante comum; P_d/H_d só podem ser comparados a T após conversão válida do domínio d para o instante comum; T não é anterior a W (se existir) nem ao P_d convertido | O dispositivo avança até T ou para na primeira fronteira/dependência anterior; retorna o instante realmente alcançado e a razão da parada |
| Sincronizar dispositivo em T | T pertence ao perfil ativo; P_d e T são comparados somente após conversão válida para o instante comum; T não é anterior a W (se existir) nem ao P_d convertido | Todas as transições próprias com instante causal até T foram processadas; nenhuma transição posterior a T foi aplicada |
| Publicar deadline D | Proprietário válido; D é expresso no domínio do proprietário ou como instante comum identificado; comparação com P_d/H_d usa conversão válida; comparação com W ocorre somente no instante comum; D não é anterior ao P_d do proprietário; se W existir, D deve ser estritamente maior que W; igualdade D=P_d exige microstep explícito e W ausente ou W<D; identidade/geração é única | O deadline passa a ser responsabilidade do proprietário e pode ser despachado uma única vez enquanto a geração for vigente. Se D=P_d, só pode ser aceito sob microstep explícito quando W não existir ou W<D, isto é, antes de o corte global fechar esse instante. D<=W é sempre LateInput, inclusive D=W e independentemente de microstep; W é um corte já fechado e microstep não o reabre. D<P_d também é LateInput; igualdade D=P_d sem microstep explícito é IndeterminateCausalOrder quando D>W (ou W não existe) |
| Cancelar/substituir deadline | Identidade e geração correspondem ao agendamento ativo; instante causal C e D são convertidos para a mesma linha temporal comum antes de comparação | A geração anterior não pode mais produzir efeito; cancelamento de evento já despachado não desfaz efeito aplicado |
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

### 7.1 Descritor obrigatório da transação

Uma transação MUST preservar informação suficiente para ser retomada sem repetir, omitir ou deslocar temporalmente efeitos. O descritor normativo contém:

- identidade estável da requisição e geração;
- origem, destino e classe da operação (leitura/escrita de memória, leitura/escrita de I/O, acknowledge ou operação de dispositivo identificada);
- perfil ativo, endereço e payload de escrita, quando aplicáveis;
- instante comum de admissão e instante de início do ciclo;
- sequência ordenada dos marcos exigidos pela operação, cada qual associado a uma fronteira de barramento nomeada e à sua classe de evidência;
- duração base consumida, TWs automáticos, TWs externos e esperas de dependência já observados;
- estado de controle atual e próxima fronteira ainda não alcançada;
- origem/condição de WAIT e ponto documentado da próxima amostragem, quando aplicável;
- marcadores de amostragem e commit, cada um com identidade estável, estado pendente/aplicado e resultado, quando aplicável;
- conjunto exato de efeitos irreversíveis já aplicados e efeitos ainda pendentes;
- erro, motivo de bloqueio, cancelamento ou recuperação explícita, se existentes.

O descritor não escolhe a representação de runtime nem a granularidade universal do barramento. Ele exige que a operação declare seus marcos antes de produzir efeitos.

### 7.2 Estados de controle e transições normativas

Os estados de controle abaixo são normativos. A implementação MAY usar outra representação, mas MUST preservar as mesmas condições, transições e efeitos observáveis.

| Estado | Condição de entrada | Transições permitidas e condição de saída | Efeitos permitidos |
|---|---|---|---|
| **Criada** | Requisição validada, identificada e admitida; nenhum ciclo iniciado e nenhum efeito aplicado | **Em andamento** quando o proprietário inicia o ciclo; **Cancelada** se o cancelamento for admitido antes de qualquer efeito irreversível; **Falha** se uma falha posterior à admissão impedir a execução | Nenhuma amostragem ou commit |
| **Em andamento** | Ciclo iniciado na fronteira de início declarada | **Aguardando** somente em ponto de amostragem de WAIT/dependência declarado; **Pronta** quando a próxima fronteira requerida for alcançada; **Cancelada** se ainda não houver efeito irreversível e o cancelamento for admitido; **Falha** diante de erro terminal | Consome apenas o tempo base e os TWs previstos; não aplica marco antes de sua fronteira |
| **Aguardando** | WAIT ativo observado em ponto documentado ou dependência externa identificada ainda não satisfeita | **Em andamento** quando a condição for observada como liberada no próximo ponto permitido; **Cancelada** antes de efeito irreversível e após admissão; **Falha** diante de erro terminal | WAIT externo ativo consome os TWs exigidos pelo Z80 e reamostra nos pontos documentados; dependência sem progresso não autoriza busy-wait nem avanço fictício |
| **Pronta** | A próxima fronteira nomeada foi alcançada; estado transitório sem consumo adicional de tempo | **Em andamento** após processar o marco e existir trabalho temporal restante; **Aguardando** se a próxima fronteira depender de WAIT/dependência; **Concluída** quando todos os marcos e tempos requeridos terminarem; **Cancelada** somente antes de efeito irreversível; **Falha** em erro terminal | Pode aplicar somente o marco cuja fronteira foi alcançada, uma única vez; não pode antecipar marcos posteriores |
| **Concluída** | Todos os marcos requeridos foram aplicados e toda a duração/espera da operação foi finalizada | Terminal; nenhuma transição para execução, cancelamento ou repetição | Nenhum novo efeito; duplicata idêntica apenas reconhece o resultado já concluído |
| **Cancelada** | Cancelamento causalmente admitido antes do primeiro efeito irreversível | Terminal; não pode ser reaberta nem restaurada como ativa | Preserva identidade, motivo e histórico; nenhum efeito futuro da transação pode ocorrer |
| **Falha** | Erro terminal durante a execução, com ou sem efeitos anteriores | Terminal por padrão. Só pode retornar a **Em andamento** por recuperação explícita do proprietário, com identidade/geração verificadas e retomada exclusiva dos marcos pendentes | Preserva erro e ledger parcial; restore não inicia retry automático |

Transições não listadas são proibidas. Uma transição interna impossível é **InvariantViolation**; comando externo com identidade/perfil/geração inválidos é **InvalidInput**; comando temporalmente tardio é **LateInput**. Uma dependência ainda não satisfeita é **Blocked**, não falha terminal.

### 7.3 Marcadores independentes de amostragem e commit

**Amostrada** e **Comprometida** não são estados de controle mutuamente exclusivos. São marcadores monotônicos no ledger da transação, pois uma operação pode exigir ambos, em ordens distintas, ou mais de um subefeito. Se a implementação os representar como estados, deverá preservar o produto equivalente entre estado de controle e marcadores.

- **Amostragem:** cada marco de leitura/acknowledge tem identidade e fronteira próprias. Ao alcançar a fronteira, captura o valor/condição exatamente uma vez e registra o resultado. Retomada ou restore reutiliza o valor capturado; nunca consulta novamente o barramento para o mesmo marco.
- **Commit:** cada efeito de escrita/acknowledge ou subefeito declarado tem identidade e fronteira próprias. Ao alcançar a fronteira, aplica o efeito exatamente uma vez e registra o commit. Efeitos indivisíveis usam um único marco atômico; efeitos múltiplos mantêm registro individual.
- Um marcador aplicado é monotônico e não pode voltar a pendente. O par estado do dispositivo/efeito e marcador de commit pertence ao mesmo corte atômico. Se isso não puder ser garantido, o cenário não pode ser declarado suportado, salvo idempotência comprovada por identidade estável.
- Um mesmo ID com payload/descritor diferente é **InvalidInput**. Repetição idêntica reconhece a mesma operação e não reaplica efeitos.
- Cancelamento é permitido somente antes do primeiro efeito marcado como irreversível pelo contrato da operação. Se o timestamp não satisfizer A(E), ou se a fronteira de cancelamento já tiver sido ultrapassada por efeito irreversível, rejeitar como **LateInput**, sem rollback e sem alterar a transação. Amostragem só encerra a janela de cancelamento quando o contrato da operação a classificar como irreversível.
- Falha antes de qualquer efeito preserva o estado anterior e registra a falha. Falha após efeitos irreversíveis entra em **Falha**, preserva o conjunto exato aplicado/pendente e não simula rollback.

### 7.4 Restore e retomada por estado

Snapshot MUST preservar o estado de controle, a próxima fronteira, o progresso temporal consumido, a condição/origem de WAIT, os marcadores e resultados de amostragem, o ledger de commits, o conjunto pendente, a identidade/geração e o erro/motivo.

| Estado/condição no snapshot | Resultado obrigatório do restore |
|---|---|
| Criada | Continua não iniciada; não consome tempo nem aplica marco |
| Em andamento | Retoma do mesmo progresso e da próxima fronteira; não reinicia duração já consumida |
| Aguardando | Preserva a condição e a fonte de espera; não presume WAIT inativo nem inventa TW; retoma somente após a condição ser observada no ponto permitido |
| Pronta | Retoma o processamento do marco pendente já alcançado, sem avançar tempo novamente; cada marcador é aplicado no máximo uma vez |
| Marcador Amostrada | Preserva o resultado capturado; não faz nova leitura/acknowledge |
| Marcador Comprometida | Preserva o efeito aplicado; não o reaplica |
| Concluída | Permanece terminal e retorna o resultado concluído |
| Cancelada | Permanece terminal; não reativa a transação nem a geração |
| Falha | Preserva erro e ledger parcial; não executa retry automático. Recuperação explícita do proprietário retoma somente marcos pendentes |

Restore incompatível com perfil, identidade, geração ou formato é **IncompatibleSnapshot** e não pode alterar a máquina ativa parcialmente.

### 7.5 Mapeamento normativo dos marcos às fronteiras de barramento

Todo marco exigido por uma operação MUST apontar para uma fronteira nomeada. A fronteira semântica fica definida nesta seção; quando a fonte não determina a borda física exata, essa coordenada permanece **UNKNOWN** e não pode ser substituída por fase presumida. UNKNOWN limita a alegação de fidelidade do perfil/operação, mas não deixa o marco sem contrato.

| Classe/operação | Marco | Fronteira de barramento exigida | Evidência e limite |
|---|---|---|---|
| Todas | Admissão da requisição | Antes do início do primeiro ciclo e antes de qualquer efeito da transação; sujeita à fronteira temporal A(E) quando for entrada/cancelamento causal | Fronteira lógica do contrato; não é uma borda elétrica |
| Memória — leitura | Início | Início do M-cycle de leitura declarado pela operação Z80/perfil | M-cycles/T-states são FACT conforme Zilog UM008011-0816; duração exata depende da operação |
| Memória — leitura | Amostragem | Fronteira em que o dado de leitura está válido e é capturado pelo consumidor | A fonte R0 não estabelece uma borda física universal para todas as operações/perfis; manter UNKNOWN até evidência específica |
| Memória — escrita | Início | Início do M-cycle de escrita declarado pela operação Z80/perfil | M-cycles/T-states são FACT; não inferir commit pelo início do ciclo |
| Memória — escrita | Commit | Fronteira em que o destino aceita o dado e o efeito se torna irreversível | Específica do destino/operação; UNKNOWN sem fonte/medição aplicável. Nenhum efeito antes dela |
| I/O — leitura/escrita | Início e duração | M-cycle de I/O, incluindo o TW automático documentado; WAIT externo, se presente, é amostrado separadamente | Um TW automático em I/O é FACT no manual Zilog; não equivale a WAIT externo |
| I/O — leitura/escrita | Amostragem/commit | Fronteira de validade/captura do dado ou aceitação da escrita pelo destino | A fonte R0 não fixa borda universal de dado/commit para todos os dispositivos; UNKNOWN por operação/perfil até evidência |
| WAIT externo Z80 | Amostragem de WAIT | Borda de descida durante T2 e em cada TW subsequente | FACT no Zilog UM008011-0816; WAIT ativo solicita TW adicional. Fonte física de WAIT no cartridge SMS permanece UNKNOWN |
| INT Z80 | Amostragem de INT | Borda de subida do último clock ao final da instrução | FACT no Zilog UM008011-0816; não confundir com o commit/efeito do acknowledge |
| INT acknowledge | Ciclo e espera automática | Ciclo M1/IORQ de acknowledge, com dois TW automáticos | FACT no Zilog UM008011-0816; a captura de vetor e efeitos do dispositivo seguem fronteira específica, não presumida |
| Todas | Conclusão | Após a última fronteira de ciclo exigida, todos os marcadores obrigatórios aplicados e toda espera consumida | Condição lógica verificável; não pode concluir com marco pendente |
| Todas | Cancelamento | Ponto lógico anterior ao primeiro efeito irreversível, após validação temporal C>A(E) | Não cria fase física; cancelamento após a fronteira irreversível é LateInput e não desfaz efeito |
| Todas | Snapshot/restore | Corte causal coerente que registra estado, próxima fronteira e ledger de efeitos | Não exige que o corte coincida com uma borda física; pendências e efeitos parciais precisam estar serializados |

O contrato não afirma uma borda de amostragem/commit universal para memória, I/O, VDP ou PSG. Para cada operação usada em compatibilidade, o perfil MUST fornecer fonte e fronteira específica; se permanecer UNKNOWN, a operação não pode ser apresentada como temporalmente validada naquele perfil. Isso não autoriza inserir WAIT, fase ou microtiming presumido.

### 7.6 Exactly-once — invariantes transacionais

- Retomar ou restaurar uma transação não pode repetir leitura amostrada, escrita comprometida, acknowledge, alteração de registrador, publicação de IRQ ou subefeito.
- Uma requisição duplicada com o mesmo ID e descritor idêntico é reconhecida como a mesma operação; mesmo ID com descritor divergente é rejeitado antes de mutação.
- Conclusão com ID ou geração incompatível falha explicitamente e preserva a transação original para diagnóstico.
- Snapshot em qualquer estado de controle ou entre quaisquer dois marcadores deve restaurar o mesmo ponto lógico e o mesmo conjunto de efeitos pendentes.
- Nenhuma transição terminal é reaberta por restore. Recuperação de Falha é explícita e nunca repete marcadores aplicados.

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
- deadlines e seus estados de ciclo de vida (Scheduled, Superseded/Cancelled, Claimed, Dispatched, Completed ou Failed), identidades, gerações e sequência estável;
- eventos ainda não despachados operacionalmente, incluindo os que já estão em estado Claimed;
- deadlines em estado Dispatched com o registro exato dos subefeitos já comprometidos e ainda pendentes;
- deadlines Failed com classe/contexto do erro e conjunto exato de subefeitos comprometidos e pendentes;
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
| Tempo retroativo | Pedido externo de avanço com T<F_d, sendo F_d=max(W se existir, P_d convertido do domínio avançado), entrada/cancelamento além da fronteira admissível ou cancelamento após o primeiro efeito irreversível: LateInput antes de mutação. Regressão interna de P_d/W: InvariantViolation. InvalidInput não classifica timestamp válido apenas por ser retroativo |
| Horizonte seguro ultrapassado | Parar antes da fronteira; se um componente já a ultrapassou, sinalizar violação de invariante e invalidar a execução de validação |
| Razão/conversão inválida ou overflow | Rejeitar perfil/operação antes de publicar avanço parcial; nunca aplicar wrap ou saturação silenciosa |
| ID de deadline duplicado | Rejeitar o novo agendamento, salvo se for explicitamente a mesma geração/atualização idempotente |
| Geração de deadline obsoleta | Descartar antes de qualquer efeito; não despachar o evento antigo |
| Conflito não comutativo com fase UNKNOWN | IndeterminateCausalOrder; rejeitar a ativação do perfil/caso ou marcar a execução como não aprovada; nunca escolher ordem por sequência incidental |
| ID/conclusão de transação incompatível | Rejeitar a conclusão e preservar a transação original para diagnóstico |
| Amostragem/commit repetido | Falha de invariante determinística; não tentar compensar ou repetir o efeito |
| Snapshot inválido/incompatível | Rejeitar restore atomicamente; a máquina ativa não pode ficar parcialmente restaurada |
| Limite de microsteps excedido | Interromper com erro reproduzível, instante, sequência de eventos e proprietário do ciclo |
| Ordem dependente do host | Falha de determinismo; registrar primeiro evento divergente e sequência causal |

## 13. Propriedades exigidas do modelo de referência R2

R2 deve construir um oráculo independente da futura implementação. O modelo pode ser simples e lento; não deve compartilhar o algoritmo de agenda ou a representação temporal candidata que será comparada.

| ID | Propriedade | Cenário mínimo |
|---|---|---|
| R2-T01 | Monotonicidade | Sequências de avanços crescentes, repetidos e inválidos; progresso local, H_d e W; pedido externo T abaixo de F_d=max(W se existir, P_d convertido) rejeitado como LateInput sem mutação; regressão interna de P_d/W como InvariantViolation; igualdade permitida apenas conforme idempotência contratual |
| R2-T02 | Chunk invariance | Avanço único contra partições aleatórias do mesmo intervalo |
| R2-T03 | Conversão sem deriva | Domínios com razão inteira e não inteira, composição direta/encadeada quando matematicamente equivalente e sessões longas |
| R2-T04 | Limite seguro | Deadline interno antes do alvo solicitado; alvo posterior a H_d convertido deve parar em H_d; domínio bloqueado; fronteira de dependência; W não pode cruzar evento/efeito ainda não resolvido |
| R2-T05 | Sincronização causal | Leitura após evento de dispositivo e escrita com commit definido |
| R2-T06 | Ordenação determinística e corrida deadline/cancelamento | Eventos em instantes diferentes, fases distintas e mesma fase; D<P_d do proprietário rejeitado como LateInput; D<=W rejeitado como LateInput, inclusive D=W e mesmo com microstep; D=P_d com microstep explícito aceito somente quando W não existe ou W<D; D=P_d sem microstep, nesse mesmo corte ainda não fechado, resulta em IndeterminateCausalOrder e nenhuma mutação; cancelamento C<D admitido somente após A(E) e antes de W fechar D; cancelamento C<=A(E) rejeitado como LateInput antes da regra de empate, inclusive C=D; cancelamento retroativo C<D recebido após W>=D e rejeitado; cancelar depois de D com despacho operacional atrasado; cancelamento e claim em D após admissão, respeitando relação causal/microstep; permutar ordem de publicação/execução mantendo a mesma ordem causal admitida |
| R2-T07 | UNKNOWN não vira precedência | Eventos simultâneos não comutativos sem fase validada; assertiva de que W não fecha sobre o conflito |
| R2-T08 | Exactly-once e efeitos parciais | Todas as transições permitidas/proibidas da máquina de estados; suspensão/retomada em Criada, Em andamento, Aguardando e Pronta; amostragem/commit independentes; cancelamento antes/depois do primeiro efeito irreversível; falha antes/depois de cada efeito; retry idêntico e ID duplicado com descritor divergente; ledger exato comprometido/pendente |
| R2-T09 | WAIT | TW automático, WAIT externo modelado e fixture separados; WAIT ativo consome TW e reamostra em cada ponto; UNKNOWN não é tratado como inativo e bloqueia sem TW especulativo |
| R2-T10 | IRQ/NMI | Transições/níveis antes e depois dos pontos de amostragem, borda NMI, EI delay, acknowledge e preservação das bordas/latches efetivamente definidos pelo dispositivo |
| R2-T11 | Snapshot/restore e geração obsoleta | Deadlines em todos os estados Scheduled, Cancelled, Claimed, Dispatched, Completed e Failed; transações de barramento em Criada, Em andamento, Aguardando, Pronta, Concluída, Cancelada e Falha; snapshots entre cada marco de amostragem/commit; restore preserva próxima fronteira, WAIT, resultado amostrado, ledger de commits e erro parcial; sem retry automático, reabertura de estado terminal ou repetição de efeito |
| R2-T12 | Replay e limite de admissão | Execução contínua contra restore + mesma sequência de entradas; classificar obrigatoriamente como LateInput e rejeitar sem mutação T<=W ou T<=P_d convertido de qualquer participante mesmo quando T>W; distinguir InvalidInput para parâmetro/perfil/ID/geração inválidos; aceitar T>A(E); aceitar entrada dentro de H_d previamente publicado e exigir recálculo dos horizontes participantes antes do próximo avanço; reproduzir a mesma ordem lógica de aceitação |
| R2-T13 | Host independence | Variar tamanho de bloco, atrasar/drenar/descartar saída e simular underrun sem alterar estado virtual |
| R2-T14 | Microstep finito | Evento cria outro evento no mesmo instante, incluindo ciclo infinito |
| R2-T15 | Overflow/erro e atomicidade parcial | Razões e timestamps; avanço solicitado T<F_d rejeitado como LateInput sem mutação; regressão interna como InvariantViolation; InvalidInput reservado a parâmetro/perfil/identidade/ID/geração/descritor inválidos; cancelamento após primeiro efeito irreversível como LateInput; D<=W como LateInput inclusive D=W com microstep; regras de D=P_d; transições inválidas; IDs/snapshots incompatíveis; capacidade esgotada; falha antes/depois de cada marco; ledger parcial exato, restore e recuperação explícita sem repetição |

R2 deve registrar sementes, perfil, sequência de operações, primeiro ponto divergente e estado mínimo para reprodução.

## 14. Critérios de aprovação R1

R1 só pode ser aprovado quando:
- **R1-EXIT-01 — Taxonomia temporal:** C01, §12.1 e §16.12 MUST classificar de forma consistente avanço externo retroativo como LateInput antes de mutação; regressão interna como InvariantViolation; InvalidInput fica restrito a parâmetro/perfil/identidade/ID/geração inválidos.
- **R1-EXIT-03 — Ciclo de vida transacional:** cada estado de controle e cada marcador de amostragem/commit MUST ter condições de entrada/saída, transições permitidas/proibidas, regras de cancelamento, falha, snapshot/restore e retomada exactly-once; todos os marcos MUST estar mapeados a fronteiras de barramento nomeadas, com evidência FACT/OBSERVATION ou UNKNOWN explícito e efeito dessa incerteza sobre a alegação de compatibilidade.
- cada contrato C01–C16 estiver classificado explicitamente como MUST, SHOULD ou MAY, com proprietário semântico, pré-condições, pós-condições, bloqueios/erros e rastreabilidade; itens fora de escopo de R1 estiverem identificados separadamente;
- não houver contradição entre tempo comum, avanço limitado, sincronização sob demanda e agenda de deadlines;
- simultaneidade e fase UNKNOWN não dependerem de ordem acidental;
- todos os erros listados tiverem comportamento definido;
- as propriedades R2 estiverem ligadas a contratos específicos;
- a revisão independente da versão exata candidata estiver concluída, sem blockers não resolvidos; comentários devem ser respondidos ou registrados como riscos aceitos explicitamente;
- revisão arquitetural confirmar que não foi escolhida prematuramente uma estrutura de dados ou granularidade física;
- os limites de perfil NTSC/PAL/PAL-M e SMS/SG-1000 estiverem preservados.

A aprovação de R1 não aprova R2, protótipos R3 ou código de produção. R3 só começa depois de R1 e R2 aprovados.

## 15. Questões de desenho deliberadamente abertas após R1 v0.3.9

Os contratos normativos das seções 3–16 definem as garantias exigidas. As questões abaixo permanecem abertas para R2/R3 porque dependem de representação, prova independente, custo ou evidência adicional; não suspendem a semântica já fixada:

- Qual representação expressa fase conhecida, desconhecida e comutatividade sem atribuir fase física presumida?
- Qual forma de interface reporta progresso local, horizonte seguro, watermark e próxima fronteira sem impor callbacks por instrução/T-state?
- Quais safe points da CPU Z80 são suficientes para os pontos de observação comprovados por gameplay SMS?
- Quais fontes primárias ou medições discriminantes poderão resolver, por perfil e operação, as fronteiras físicas de amostragem/commit atualmente classificadas UNKNOWN, sem presumir uma borda universal?
- Qual mecanismo de agenda/cancelamento satisfaz identidade, geração e linearização com custo mensurável?
- Qual mecanismo detecta dependência sem progresso, starvation ou ciclo de microstep sem custo fixo indevido no hot path?
- Qual representação implementa conversões exatas, resíduos e composição sem assumir associatividade/inversibilidade?
- Qual DTO/codec e política de compatibilidade persistente preservam corte causal, eventos e transações pendentes?

As respostas devem ser avaliadas por prova, rastreabilidade e custo, não por preferência de implementação.


## 16. Clarificações normativas para tornar os contratos verificáveis

Esta seção resolve ambiguidades sem selecionar representação numérica, estrutura de agenda, modelo de threads, fase física ou granularidade de hardware. Se uma relação física continuar UNKNOWN, a regra abaixo limita a alegação de fidelidade; não inventa uma ordem substituta.

### 16.1 Progresso local, horizonte seguro e watermark confirmado

Os termos abaixo são distintos e MUST NOT ser usados como sinônimos:

- **Progresso local (P_d):** instante até o qual o estado próprio do domínio d foi efetivamente atualizado. P_d é expresso no domínio local; só pode ser comparado ao instante comum por uma conversão declarada e válida no perfil.
- **Horizonte seguro (H_d):** maior instante local do domínio d até o qual ele pode avançar sem atravessar uma fronteira que ainda possa exigir observação, publicação ou arbitragem causal externa. H_d é expresso no domínio local e só pode ser comparado ao instante comum por uma conversão declarada e válida no perfil.
- **Watermark confirmado da máquina (W):** corte causal global fechado: todos os efeitos e eventos despacháveis pertencentes ao perfil ativo com instante causal menor ou igual a W foram processados, cancelados ou demonstrados como sem efeito. Uma transação ainda pendente pode atravessar W somente se todos os seus efeitos devidos até W estiverem concluídos e os efeitos restantes tiverem instante causal posterior a W. Um conflito não resolvido que possa mudar o resultado impede fechar W através daquele instante.

Regras normativas:

1. P_d e H_d pertencem ao domínio e podem diferir entre dispositivos. W pertence à máquina/perfil; um dispositivo não pode declarar W global por conta própria.
2. W MUST ser monotônico. Um domínio não pode publicar progresso local superior ao seu H_d vigente.
3. O coordenador só pode confirmar W depois de obter evidência de fechamento causal de todos os participantes relevantes do perfil até W. A regra de participação é parte do perfil e não pode ser inferida da ordem de iteração.
4. W não é necessariamente o mínimo aritmético de todos os P_d: domínios sem dependência causal no intervalo podem não limitar o corte, mas essa independência precisa estar declarada e validada. R1 não escolhe o algoritmo que calcula W.
5. Um evento despachável ou efeito ainda não resolvido com instante menor ou igual a W impede fechar W. Uma transação pode permanecer pendente apenas sob a condição definida na própria definição de W: nenhum efeito devido até W pode continuar pendente. O snapshot deve preservar a continuação posterior a W.
6. Para um pedido de avanço do domínio d até o instante comum T, defina a fronteira inferior F_d=max(W, P_d convertido para o instante comum), omitindo W quando ainda não existir. Se T<F_d, o pedido é retroativo e MUST falhar como LateInput antes de mutar estado. T=F_d só é idempotente quando não houver nova entrada causal e as demais condições do contrato forem satisfeitas. Se W existir, T<W é portanto retroativo; sem W, P_d convertido continua sendo o limite inferior. Um pedido T>F_d continua sujeito a H_d, dependências e demais limites de avanço.
7. O instante comum é uma relação comparável definida pelo perfil. Esta definição não exige epoch, tick mestre, inteiro específico, relógio de parede ou fase elétrica comum.
8. Se um conflito não comutativo com fase UNKNOWN impedir fechar o corte, o resultado deve indicar bloqueio/indeterminação; o coordenador não pode avançar W escolhendo desempate incidental.
9. Cada classe de entrada externa MUST declarar no perfil o conjunto de domínios participantes cujo estado ou decisões causais ela pode afetar. O conjunto inclui, no mínimo, o proprietário/consumidor direto; se não puder ser determinado, a admissão fica Blocked/indeterminada e não pode ser inferida por ausência de eventos. Uma entrada só integra a linha causal depois de admitida/registrada com identidade, instante e ordem lógica. Seu instante T deve ser expresso como instante comum do perfil, ou convertido a ele por conversão válida; coordenadas locais brutas não são comparáveis entre si nem com W.
10. Para uma entrada E, defina conceitualmente sua **fronteira de admissão A(E)** como o maior entre W vigente (se já existir) e os P_d convertidos ao instante comum de todos os domínios participantes de E. A entrada só pode ser admitida se T > A(E). Se ainda não existir W confirmado, ele é omitido do cálculo; os P_d dos participantes continuam limitando a admissão. A(E) é uma regra semântica, não escolhe representação ou algoritmo.
11. Uma nova entrada com T <= W vigente ou T <= P_d convertido de qualquer participante MUST ser rejeitada como LateInput antes de mutar estado, mesmo que T > W. InvalidInput fica reservado a parâmetro, perfil, identidade, ID ou geração inválidos; não pode substituir LateInput para um timestamp temporalmente inadmissível. O fechamento global ou o progresso local do participante não pode ser reaberto implicitamente. Isso inclui igualdade exata com W ou P_d.
12. Uma entrada com T > A(E) pode ser admitida se perfil, identidade e ordem lógica forem válidos. A admissão invalida qualquer H_d previamente declarado pelos participantes que pressupunha ausência dessa entrada; esses participantes MUST recalcular/publicar seus limites seguros antes de qualquer novo avanço. Entradas futuras podem ser registradas antecipadamente, desde que sua admissão e ordem sejam determinísticas.
13. Restore de snapshot restaura o limite de admissão, os P_d relevantes, o cursor e as identidades de entradas aceitas. Replay reproduz a sequência já admitida; não transforma entrada tardia em válida nem reordena silenciosamente entradas em torno de W/P_d.
14. Uma entrada em T <= A(E) só pode participar do estado se tiver sido admitida e incluída antes de a fronteira correspondente avançar. Não existe rollback/reabertura implícita; rollback explícito exige replay a partir de snapshot anterior e nova linha causal identificada.

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
- **Superseded/Cancelled:** geração invalidada antes do claim lógico em D; não pode mais produzir efeito.
- **Claimed:** no instante causal D, a decisão lógica de reivindicar o deadline vigente foi tomada; esta é a fronteira que fecha a possibilidade de cancelamento.
- **Dispatched:** a execução operacional do efeito foi iniciada depois ou no instante do claim; atraso do dispatcher não altera a decisão causal já tomada.
- **Completed:** o efeito definido foi aplicado exatamente uma vez.
- **Failed:** a tentativa de efeito não pôde ser concluída; o erro e o estado pendente são preservados. Restore mantém Failed e não dispara retry automático. Uma recuperação só pode ser iniciada por operação explícita do proprietário e retoma a partir dos subefeitos pendentes, sem repetir os comprometidos.

Regras:

1. O ponto de linearização entre cancelamento e deadline é o **claim lógico no instante causal D**, não o início operacional do dispatcher. Um cancelamento causalmente admitido antes de D pode suprimir o claim; depois do claim em D, cancelar não suprime nem desfaz o efeito, mesmo que o dispatcher ainda não tenha executado. Se cancelamento e claim coincidirem em D, aplica-se exclusivamente a relação causal/microstep declarada pelo perfil; sem ela, o caso permanece indeterminado. A arbitragem é serializada e reproduzível, nunca determinada por corrida de threads ou escalonamento do host.
2. Substituir um deadline incrementa/renova a geração lógica. Qualquer referência antiga deve ser rejeitada antes de seu efeito, mesmo após restore.
3. Identidade não pode ser reutilizada de forma que uma referência antiga se torne válida para outro evento. Se IDs forem reciclados, a geração deve impedir ABA.
4. O proprietário é responsável por reconstruir os deadlines próprios a partir do snapshot; o coordenador não pode inventar eventos ausentes.
5. Um deadline concluído não pode ser despachado novamente por retry, restore ou duplicação de mensagem.
6. Um deadline bloqueado durante despacho deve manter estado suficiente para retomar sem duplicar seu efeito.
7. Cancelar um evento concluído não reverte estado. Compensação, se existir em algum dispositivo, é uma nova operação causal, não cancelamento retroativo.
8. Eventos obsoletos podem ser descartados sem efeito, mas a decisão deve ser determinística e auditável.
9. O cancelamento possui instante causal de aceitação C, registrado no domínio declarado pela entrada; C e D MUST ser convertidos para o mesmo instante comum por conversão válida do perfil antes de comparação. Comparar coordenadas locais brutas de domínios diferentes é proibido. Não basta registrar quando uma thread executou a chamada ou quando o dispatcher recebeu a mensagem.
10. Um cancelamento só suprime o efeito se sua posição causal for anterior ao claim lógico do deadline em D. Se C for posterior a D, o deadline continua devido, mesmo que o dispatcher ainda não o tenha processado por atraso operacional; o atraso de despacho não reabre a janela causal de cancelamento.
11. Se C e D coincidirem, a supressão só pode ser decidida por relação causal/microstep explicitamente definida pelo perfil. Sem essa relação, o resultado é indeterminado e não aprovado para o caso; sequência de chegada, thread ou ordem de execução do host não escolhem o vencedor.
12. A arbitragem lógica entre cancelamento e claim da regra 1 é aplicada no corte causal correspondente ao instante D, enquanto a execução operacional do efeito pode ocorrer depois. O registro deve preservar ambos os fatos: decisão causal e instante operacional de conclusão. O estado Claimed é persistido separadamente de Dispatched para que atraso, falha ou restore do dispatcher não reabra a janela de cancelamento. Restore de um evento Claimed MUST retomar o despacho operacional exatamente uma vez; não pode descartá-lo, reivindicá-lo novamente nem torná-lo elegível a cancelamento. Se o efeito de um deadline possuir vários subefeitos irreversíveis, cada subefeito MUST ter identidade/registro de commit próprio conforme §16.7; se for indivisível, seu commit deve ser representado como um único marco atômico. O estado do core e o marcador de commit de cada subefeito MUST pertencer ao mesmo corte atômico; quando isso não puder ser garantido, o efeito deve ser idempotente por identidade estável ou o cenário não pode ser declarado suportado. Snapshot de um deadline Dispatched MUST preservar esse progresso: restore retoma somente subefeitos ainda pendentes, sem redispatch do evento nem repetição de subefeito comprometido. Snapshot/restore de um deadline Failed preserva o erro e o conjunto parcial, sem retry automático; uma recuperação explícita do proprietário pode retomar apenas os subefeitos pendentes sob a mesma identidade idempotente.
13. Todo cancelamento é uma entrada causal E e MUST satisfazer a fronteira de admissão A(E) definida em 16.1, independentemente da relação entre C e D: C > P_d_admissão convertido para cada domínio participante e C > W_admissão quando W existir. Todas as comparações ocorrem no instante comum do perfil. Se C <= A(E), o cancelamento MUST ser rejeitado como LateInput antes de qualquer avaliação de empate/microstep ou mutação; essa precedência também vale para C = D e não pode ser contornada pela regra 11. Depois de admitido: (a) para C < D, o cancelamento só suprime o deadline se for registrado antes de W fechar D; se W >= D, o deadline já está devido e o cancelamento não o suprime; (b) para C = D, aplica-se a regra 11 de relação causal/microstep explícita somente se W não existir ou W<D; se W>=D, o corte já está fechado e o cancelamento não suprime; sem relação causal/microstep explícita, o resultado é IndeterminateCausalOrder; (c) para C > D, o cancelamento não suprime o deadline.
14. A admissão de publicação de deadline é distinta da arbitragem de claim/cancelamento: se W existir, D MUST ser estritamente maior que W. Portanto D<=W, inclusive D=W, é LateInput antes de qualquer regra de microstep; W já representa um corte global confirmado e fechado, que não pode ser reaberto. Para a igualdade local D=P_d, a cadeia causal do instante D é considerada globalmente aberta somente quando W não existe ou W<D; nessa condição e somente com microstep explícito a publicação pode ser admitida. Se D=P_d e W>=D, a publicação é LateInput; se D=P_d e W<D (ou W não existe), mas faltar relação causal/microstep explícita, o resultado é IndeterminateCausalOrder sem mutação. A condição W ausente ou W<D é verificável sem introduzir estado candidato adicional nem presumir fase física.

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
5. Se a fonte de WAIT físico não for conhecida no perfil, a interface genérica deve representá-la como UNKNOWN. Um fixture pode injetar WAIT explicitamente para testar a semântica, mas UNKNOWN não pode ser convertido silenciosamente em sinal inativo para declarar fidelidade do perfil.
6. A quantidade de TW automáticos permanece vinculada ao tipo de ciclo Z80 documentado; não é uma regra universal para todos os ciclos ou dispositivos.
7. Quando WAIT externo estiver ativo em um ponto documentado de amostragem, a CPU não pode concluir/retirar a operação como se o ciclo tivesse terminado; o ciclo permanece na sequência Z80 e consome o TW correspondente.
8. Cada TW adicional consome tempo virtual do ciclo e exige nova amostragem de WAIT no ponto documentado. Portanto, “CPU sem avançar a instrução” não significa “tempo virtual congelado”: o domínio CPU avança pelo TW e reavalia o sinal.
9. Se o valor de WAIT estiver disponível e ativo, a operação permanece em andamento/aguardando TW, com progresso temporal do ciclo. Se a própria condição de entrada ainda não puder ser determinada no ponto de amostragem, o resultado é Blocked, preservando o último P_d efetivo e a identidade da dependência; não se inventa nível inativo nem se consome TW especulativo.
10. WAIT de fixture, quando ativo, segue a mesma semântica temporal de amostragem/TW para validar a transação, mas permanece marcado como entrada de teste. A distinção entre espera que consome TW e Blocked por ausência de condição conhecida deve sobreviver a snapshot/restore/replay.

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
2. O snapshot contém identidade do perfil/revisão, estado de cada domínio, resíduo, watermark, progresso local relevante, deadlines, gerações e estados de ciclo de vida; para deadlines Dispatched, inclui o registro de cada subefeito comprometido/pendente ou o único marco atômico do efeito indivisível; para Failed, inclui erro e estado parcial sem autorizar retry automático; inclui também microsteps pendentes, CPU, barramento, WAIT, IRQ/NMI, dispositivos e cursor de entradas externas.
3. Filas, buffers e handles do host (CPAL, WGPU, janela, VSync) não integram o snapshot temporal e são reconstruídos pela camada host. O snapshot inclui apenas estado de geração pertencente ao core/dispositivo (por exemplo, divisores, fase, acumuladores ou resampler se fizer parte do contrato digital). Blocos já entregues/consumidos pelo host não são reintroduzidos no restore.
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
- **LateInput:** pedido externo de avanço até T<F_d, com F_d=max(W, P_d convertido do domínio avançado, omitindo W quando ausente); nova entrada/cancelamento com timestamp T/C menor ou igual à fronteira de admissão A(E); cancelamento após a fronteira irreversível da transação; publicação de deadline D<P_d do proprietário ou publicação com D<=W (se W existir). Rejeitar antes de mutação e sem rollback/reabertura implícita. T=F_d só pode ser idempotente quando não houver nova entrada causal e as demais condições do contrato forem satisfeitas. A igualdade D=P_d só pode ser admitida com microstep explícito quando W não existir ou W<D. D=P_d sem microstep explícito, nesse mesmo caso ainda não fechado globalmente, usa IndeterminateCausalOrder. D=W é sempre LateInput, mesmo com microstep, pois W representa corte já confirmado e fechado. O diagnóstico preserva timestamp solicitado, fronteira aplicável, T/D, A(E), participantes, progresso e primeiro efeito irreversível relevante. Se participantes/conversões não puderem ser determinados, usar Blocked/IndeterminateCausalOrder, não LateInput.
- **InvalidInput:** parâmetro, perfil, identidade, ID, geração ou descritor inválido; inclui reutilização de ID com payload/descritor divergente. Rejeitar antes de mutação. Não usar para um timestamp válido cujo único defeito seja estar atrás do progresso confirmado.
- **InvariantViolation:** regressão interna de P_d/W, monotonicidade, causalidade, exactly-once, chunk invariance, transição de estado proibida ou estado interno violado; execução de validação inválida.
- **ResourceExhausted:** limite de eventos, microsteps, memória ou capacidade atingido; não descartar silenciosamente nem avançar parcialmente além do último estado consistente.
- **IncompatibleSnapshot:** versão, perfil ou estado restaurado incompatível; rejeição atômica.
- **IndeterminateCausalOrder:** relação causal/fase necessária não estabelecida no perfil; não declarar compatibilidade para o cenário nem resolver por ordem incidental do software.
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
3. O caminho comum SHOULD evitar alocação dinâmica por T-state ou por instrução. Qualquer exceção deve ser identificada e justificada no desenho; sua medição comparativa é uma entrega de R3, não pré-condição documental de aprovação R1.
4. R3 MUST medir separadamente o custo de avanço, sincronização, publicação, cancelamento, despacho, microsteps, snapshot/restore e replay antes de selecionar a alternativa candidata.
5. Os relatórios de R3 MUST incluir p50/p95/p99 e pior caso, volume de eventos/sincronizações, alocações/bytes, memória máxima e comportamento em carga esparsa/densa. Nenhuma dessas medições é exigida para fechar documentalmente R1.
6. Instrumentação detalhada é opt-in. Ativar/desativar tracing não pode mudar resultados, ordem lógica ou estado temporal.
7. Resultados de benchmark devem fixar SHA, perfil, carga, estado inicial, entradas, ambiente, configuração de tracing e método. Headless, frame pacing real, áudio físico e input-to-photon são métricas distintas.
8. O objetivo de 60 FPS é requisito de produto posterior; não é critério de aprovação isolado do R1 e não pode ser inferido de microbenchmark.

### 16.15 Classificação de evidência e proveniência

As afirmações técnicas e os limites de hardware devem usar as seguintes classes:

- **FACT:** afirmação sobre o hardware ou documento técnico diretamente sustentada por fonte primária identificada, dentro do escopo declarado. Registrar fonte, revisão/página/seção e limites de aplicabilidade.
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

### 16.16 Proveniência interna e limites das fontes

A revisão deste contrato deve manter a seguinte cadeia de proveniência. Os documentos internos orientam o escopo; não substituem as fontes primárias citadas por eles.

| Assunto | Fonte de autoridade interna | O que sustenta | O que NÃO sustenta |
|---|---|---|---|
| Semântica Z80, M-cycles, T-states, WAIT, INT/NMI e acknowledge | R0_CLOSEOUT.md, seção 2; Zilog Z80 CPU User Manual UM008011-0816 | Pontos documentados de amostragem e TW automáticos citados no R0 | WAIT externo de cartridge, fase elétrica entre chips ou timing não documentado |
| Relações e tolerâncias de clock | CLOCK_DOMAIN_RESEARCH.md; R0_CLOSEOUT.md, seção 3 | Valores nominais/derivados por perfil, convenções e divergências registradas | Epoch comum, fase absoluta, clock PAL-M físico universal ou precisão além da fonte |
| PSG/YM2413 e saída de áudio | AUDIO_CLOCK_RESEARCH.md; R0_CLOSEOUT.md, seção 5 | Domínios candidatos, divisores publicados e incertezas de fase/analógico | Qualidade analógica, filtro físico, fase de reset ou equivalência universal de placas |
| Fronteiras observáveis VDP | VDP_OBSERVABLE_BOUNDARIES.md; R0_CLOSEOUT.md, seção 4 | Fronteiras e efeitos de software relevantes para gameplay, com confiança/limites | Pipeline universal por dot, arbitration física ou resultados históricos como PASS no SHA congelado |
| Implementação e limites conhecidos da V1 | V1_TO_V2_INVENTORY.md; V1_BASELINE_EVIDENCE_MATRIX.md | OBSERVATION de implementação no SHA congelado e cenários a investigar | Prova física, API a copiar, dependência de runtime ou aprovação automática de comportamento |
| Estratégia temporal candidata | CHRONOS_MINIMAL_HYBRID_SPEC.md | Proposta de três mecanismos: avanço limitado, sincronização sob demanda e deadlines seletivos | Decisão final de algoritmo, estrutura de agenda, fase física, autorização de R2 antes de R1 ou produção antes de R5 |

Regras adicionais de proveniência:
1. A sequência mandatória R0→R1→R2→R3→R4→R5 e as condições de passagem em `PROJECT_INSTRUCTIONS.md` prevalecem sobre planos antigos ou propostas. Menções a R2 em R0/CHRONOS_MINIMAL_HYBRID_SPEC identificam o gate futuro; não autorizam iniciar R2 antes da aprovação formal de R1. Nenhum código de produção antes de R5.
2. Uma cláusula normativa é um requisito/decisão de projeto, não um FACT físico. Registrar sua justificativa e marcar separadamente as premissas subjacentes como FACT, OBSERVATION, HYPOTHESIS/INFERENCE ou UNKNOWN.
3. Quando uma afirmação depender de hardware, registrar fonte primária, revisão, página/seção, perfil e limite de aplicabilidade.
4. Quando depender de V1, registrar o SHA congelado 207ccc4cebbc4f8c0aed45c68262eae963491689 e identificar se é código, teste ou documentação.
5. Quando depender de ROM/teste, registrar hash da ROM, perfil, estado inicial, entrada, ambiente, executável/SHA e artefato de resultado. Relatórios sem esses elementos permanecem OBSERVATION histórica limitada.
6. Referências MAME, ares, Gearsystem, JGenesis, BlastEm, MEKA e Genesis Plus GX são comparações de implementação; registrar revisão/commit e nunca usá-las como prova física isolada.
7. O fato de uma fonte não mencionar um comportamento não prova sua ausência física; classificar como UNKNOWN quando a pesquisa não o resolver.

### 16.17 Mapeamento de lacunas normativas para validação R2

| Lacuna fechada | Contratos | Propriedade R2 associada |
|---|---|---|
| Progresso local, horizonte e watermark global | C01, C04, C05 | T01, T04, T05 |
| Fronteira de admissão A(E), progresso dos participantes e invalidação de H_d | C04, C06, C13 | T04, T06, T12 |
| Bloqueio, dependência circular e ausência de progresso | C04, C05, C14, C15 | T04, T05, T14, T15 |
| Composição de conversões e resíduo | C02, C03, C11 | T02, T03, T11, T12 |
| Ponto de linearização e geração de deadline | C06, C11, C14 | T04, T06, T11, T14 |
| Microstep e progresso finito | C06, C07, C14, C15 | T06, T07, T14, T15 |
| Efeitos parciais e idempotência por marco | C08, C11, C15 | T08, T11, T12, T15 |
| WAIT e ciclos Z80 | C09 | T09 |
| Linha, borda, latch e acknowledge | C10, C11 | T10, T11 |
| Corte causal e restore atômico | C11, C13, C15 | T11, T12, T15 |
| Ordem de ingestão e concorrência | C07, C13 | T06, T12, T13 |
| Saturação e observabilidade sem perturbação | C14, C15, C16 | T14, T15 e benchmarks R3 |
| Independência do host de áudio/vídeo | C12 | T13 |

## 17. Matriz de rastreabilidade dos contratos

| ID | Contrato | Propriedade R2 / validação posterior |
|---|---|---|
| R1-C01 | Tempo monotônico e sem wrap silencioso | R2-T01, R2-T15 |
| R1-C02 | Conversão exata entre domínios e conservação de resto/fase | R2-T03, R2-T15 |
| R1-C03 | Chunk invariance | R2-T02 |
| R1-C04 | Avanço limitado e horizonte seguro | R2-T01, R2-T04 |
| R1-C05 | Sincronização antes de observação/efeito causal | R2-T05 |
| R1-C06 | Propriedade, identidade, geração e cancelamento de deadlines | R2-T04, R2-T06, R2-T11, R2-T14 |
| R1-C07 | Ordenação causal e tratamento de fase UNKNOWN | R2-T06, R2-T07 |
| R1-C08 | Transação suspensível com amostragem/commit exactly-once | R2-T08, R2-T11, R2-T15 |
| R1-C09 | TW automático, WAIT externo e fixture separados | R2-T09 |
| R1-C10 | Amostragem INT/NMI e acknowledge Z80 | R2-T10, R2-T11 |
| R1-C11 | Snapshot/restore de estados pendentes | R2-T11, R2-T12, R2-T15 |
| R1-C12 | Independência do host de áudio/vídeo | R2-T13 |
| R1-C13 | Determinismo sob concorrência e replay | R2-T06, R2-T12, R2-T13 |
| R1-C14 | Progresso finito de eventos no mesmo instante | R2-T14, R2-T15 |
| R1-C15 | Erros temporais e estado inválido explícitos | R2-T15 |
| R1-C16 | Custo previsível e contenção de alocações em hot paths | Benchmarks R3; medição integrada da primeira fatia (F6) |


### 17.1 Classificação normativa, proprietário e condições verificáveis

A classificação abaixo é parte do contrato, não uma escolha de implementação. **MUST** é obrigatório; **SHOULD** é recomendado e qualquer exceção precisa ser justificada; **MAY** é opcional e não pode alterar a semântica. Nenhum C01–C16 fica fora de escopo. Métricas de apresentação física do host (frame pacing, input-to-photon e underrun físico) ficam **OUT OF SCOPE para R1** e pertencem à validação integrada posterior (R3/F6).

| ID | Força | Proprietário semântico | Pré-condições | Pós-condições / invariante | Bloqueio ou falha verificável | Base de evidência / proveniência | R2 |
|---|---|---|---|---|---|---|---|
| C01 | MUST | CHRONOS + domínio | Perfil válido; instante comum comparável; alvo T não inferior a F_d=max(W se existir, P_d convertido do domínio avançado) | Progresso monotônico; overflow nunca silencioso | Pedido externo T<F_d: LateInput sem mutação; regressão interna de P_d/W: InvariantViolation; parâmetro/perfil inválido: InvalidInput; overflow detectado antes de mutação (InvalidInput se o valor solicitado estiver fora do domínio válido, InvariantViolation se for falha aritmética interna) | R0 §3; CLOCK_DOMAIN_RESEARCH; razões/frequências são FACT dentro dos limites citados; epoch/unidade final UNKNOWN | T01, T15 |
| C02 | MUST | Perfil de clock + conversor | Conversão declarada e válida; resíduo compatível | Conversão exata, resíduo preservado e composição apenas quando provada | Razão inválida, resíduo inconsistente ou overflow: rejeitar sem avanço parcial | R0 §3; CLOCK_DOMAIN_RESEARCH; PAL/PAL-M e fase mantidos conforme UNKNOWN registrado | T03, T15 |
| C03 | MUST | Cada domínio avançável | Mesmo perfil, estado inicial e sequência de entradas | Particionar avanço não muda estado/saída observável | Divergência de chunk: InvariantViolation reproduzível | R1 §3.3; R0 §5; V1 PSG/VDP apenas OBSERVATION limitada | T02 |
| C04 | MUST | Domínio + coordenador | T comum válido; P_d/H_d convertidos por regra do perfil antes de comparar; T >= P_d convertido e, se W existir, T >= W; avanço limitado também por H_d convertido | Alcança T ou retorna fronteira/bloqueio com progresso real; nunca excede H_d | Dependência sem progresso: Blocked; H_d violado: InvariantViolation | R0 §2–4; VDP_OBSERVABLE_BOUNDARIES; limites físicos não extrapolados | T01, T04 |
| C05 | MUST | Dispositivo observado + coordenador | Operação cross-device declara estados e instante causal | Estados relevantes sincronizados até T; nenhum efeito posterior aplicado | Dependência recíproca: Blocked; observação fora de corte: falha causal | R0 §2–4; VDP_OBSERVABLE_BOUNDARIES; boundaries são por perfil/confiança | T05 |
| C06 | MUST | Proprietário do deadline + coordenador | ID, geração e proprietário válidos; D e C comparados no instante comum; D>P_d do proprietário ou igualdade D=P_d com microstep explícito e W ausente/W<D; D>W quando W existir; todo cancelamento, inclusive C=D, exige C>A(E) antes da arbitragem de empate | Só geração vigente produz efeito; cancelamento C<D admitido após A(E) e antes de W fechar D suprime; C<=A(E) é LateInput independentemente da relação C/D; posterior a D não suprime | LateInput se D<P_d ou D<=W, inclusive D=W com microstep; igualdade D=P_d sem microstep e com D>W: IndeterminateCausalOrder; geração antiga: descarte sem efeito | R1 §§5, 16.5; CHRONOS_MINIMAL_HYBRID_SPEC; política lógica é contrato, não FACT físico | T04, T06, T11, T14 |
| C07 | MUST | Perfil + coordenador de eventos | Instantes/fases/comutatividade conhecidos ou explicitamente UNKNOWN | Ordem causal determinística; sequência de software nunca inventa fase física | Conflito não comutativo UNKNOWN: bloqueia fechamento W/indeterminação | R0 §6; CHRONOS_MINIMAL_HYBRID_SPEC; referências são OBSERVATION de implementação | T06, T07 |
| C08 | MUST | Dono da transação/barramento | Request/ID único; marcos e efeitos identificados | Amostragem/commit por marco exactly-once; conjunto parcial preservado | Payload divergente, repetição ou falha parcial: rejeitar/preservar efeitos realizados | R0 §2; Zilog UM008011-0816 para M/T/WAIT; V1 bus é OBSERVATION no SHA congelado | T08, T11, T15 |
| C09 | MUST | CPU Z80 + contrato de ciclo | Tipo de ciclo conhecido; fonte WAIT identificada (FACT, fixture ou UNKNOWN) | TW automático separado; WAIT amostrado em T2/TWs; ciclo consome tempo sem aposentar instrução enquanto espera | Sinal indisponível no ponto: Blocked; UNKNOWN não vira inativo | R0 §2 e §5; Zilog UM008011-0816; WAIT físico SMS permanece UNKNOWN | T09 |
| C10 | MUST | CPU Z80 + donos de IRQ | Ponto de amostragem e fonte/perfil identificados | INT/NMI/acknowledge observados segundo contrato Z80; bordas/latches reais preservados | Fase/fonte sem relação causal definida: IndeterminateCausalOrder; borda omitida: InvariantViolation | R0 §2; Zilog UM008011-0816; VDP boundary sources são limitadas por perfil | T10, T11 |
| C11 | MUST | Máquina + componentes | Corte W fechado ou pendências integralmente serializadas; perfil compatível | Restore atômico; eventos/efeitos pendentes preservados; sem duplicar commit | Snapshot incompatível: IncompatibleSnapshot; falha de restore não altera máquina ativa | R0 §7; V1 inventory §3.7 e SNAP-01 são OBSERVATION, não prova de codec persistente | T11, T12, T15 |
| C12 | MUST | Core + adaptador host | Bloco de saída identificado por intervalo/geração | Drenagem/atraso/descarte/underrun host não altera tempo virtual | Reordenação/duplicação de bloco: falha do pipeline, sem alterar relógio | R0 §5 e §7; V1 PSG-02/AUD-01 são OBSERVATION; saída física não é relógio | T13 |
| C13 | MUST | Ponto de ingestão + coordenador | Entrada com ID, instante comum (ou conversão válida), ordem lógica e conjunto de participantes causais declarados | Replay reproduz ordem admitida; T deve ser posterior a W (se existir) e a P_d convertido de todos os participantes; H_d afetado é recalculado após admissão | LateInput se T<=W ou T<=P_d de participante; conjunto de participantes desconhecido: Blocked/indeterminado; corrida sem serialização: execução não determinística/não aprovada | R1 §§16.1, 16.11; R0 §6; ordem de ingestão é decisão normativa, não fato elétrico | T06, T12, T13 |
| C14 | MUST | Dispatcher + dono do evento | Evento no instante atual tem regra de microstep e limite | Cada microstep resolve obrigação identificável; termina, bloqueia ou falha | Limite excedido/ciclo sem progresso: ResourceExhausted ou InvariantViolation | CHRONOS_MINIMAL_HYBRID_SPEC; estratégia candidata não é decisão de algoritmo | T14, T15 |
| C15 | MUST | Todas as operações temporais | Erro/classe e ponto de falha identificáveis | Falha pré-efeito preserva estado; pós-efeito informa conjunto parcial exato | LateInput, InvalidInput, InvariantViolation, ResourceExhausted, IncompatibleSnapshot, IndeterminateCausalOrder ou InternalFailure | R0 §2–7; regras normativas de atomicidade não alegam rollback físico | T15 |
| C16 | SHOULD | Core temporal / responsável por medição | Hot path e carga-alvo identificados antes da comparação | Evitar alocação por T-state/instrução; custos/capacidade observáveis e comparáveis em R3 | Exceção deve ser justificada; saturação segue C15 MUST | R0 §7; V1 PERF-01 é NOT RUN e não fornece baseline transferível | Benchmarks R3; medição integrada da primeira fatia (F6) |

Regras de leitura:
1. A classificação MUST/SHOULD/MAY aplica-se ao contrato, não transforma uma hipótese de hardware em FACT.
2. Pré-condições e pós-condições acima resumem os contratos detalhados nas seções 3–16; em caso de divergência, a cláusula detalhada deve ser corrigida, não ignorada.
3. **MAY** é reservado a mecanismos opcionais de instrumentação/tracing descritos em 16.14; sua ativação não pode alterar estado, ordem ou saída.
4. Um requisito que dependa de evidência UNKNOWN deve continuar aplicável como regra de segurança/indeterminação, mas não autoriza alegar compatibilidade física do perfil afetado.
5. R2 IDs indicam propriedades futuras, não testes executados nem resultados PASS.


## 18. Requisitos de desempenho e observabilidade

R1 não escolhe estrutura de agenda, mas define os requisitos de capacidade e observabilidade que deverão ser demonstrados em R3. Esta seção não exige benchmark nem medição integrada para a aprovação documental de R1:
- como recomendação C16 (SHOULD), evitar alocação dinâmica por T-state/instrução no caminho comum; justificar exceções e medi-las em R3;
- publicação e despacho de deadlines devem ter custo limitado e mensurável; crescimento de agenda deve possuir capacidade/limite e política explícita de falha;
- o estado temporal deve permitir contadores de diagnóstico sem instrumentação obrigatória no hot path;
- tracing detalhado deve ser opt-in e não alterar a semântica;
- benchmark de protótipo deve separar avanço de dispositivo, sincronização, publicação/dispatch, snapshot, alocações e pior caso;
- nenhuma medição headless será tratada como prova de 60 FPS apresentados; frame pacing, CPAL e input-to-photon são métricas posteriores e separadas.

O objetivo de produto continua 60 FPS estáveis (16,67 ms/frame), mas R1 não declara esse objetivo validado. R3/F6 deve medir o runtime e a apresentação reais e interromper expansão de features se houver regressão abaixo da meta.

## 19. Resposta à revisão adversarial da linha de base

Esta seção preserva o histórico das revisões anteriores. A revisão corretiva v0.3.9 também registra os dois blockers identificados no parecer formal de 24/11/2026; sua correção proposta não equivale à aprovação do gate. As revisões v0.3.4, v0.3.5 e v0.3.6 fecharam lacunas de admissão de entradas/cancelamentos em relação a W e P_d, invalidação de H_d após nova entrada, restauração de efeitos parciais de deadlines Dispatched, limite explícito por H_d, precedência dos gates e taxonomia LateInput. A revisão Codex da v0.3.6 não encontrou major issues no commit 74529fccfacb1d43cf1861d993aac0863f8b5e41. A revisão Codex da v0.3.7 identificou que a expressão 'cadeia causal aberta' não tinha critério verificável e era incompatível com W já definido como corte fechado. A v0.3.8 remove a admissão de D=W: deadline publicado com D<=W é sempre LateInput, independentemente de microstep. A igualdade D=P_d só é admitida com microstep explícito se W não existir ou W<D; assim, o instante ainda não foi fechado pelo watermark global. Esta versão continua sendo rascunho e não constitui aprovação formal do gate.

| Achado da linha de base / auditoria complementar | Resultado anterior | Tratamento nas revisões v0.3.4–v0.3.8 | Estado documental |
|---|---|---|---|
| BLOCKER — classificação normativa C01–C16 | Não havia força normativa, proprietário e condições por contrato | Nova matriz 17.1 classifica C01–C15 como MUST e C16 como SHOULD, com pré/pós-condições, falhas, proveniência e R2 | Corrigido no texto; aguarda revisão independente |
| MAJOR — WAIT/TW vs Blocked | 16.2 e 16.8 distinguiam bloqueio e fonte, mas não explicitavam consumo temporal de TW | 16.8 define ciclo Z80 sem aposentadoria enquanto espera, consumo de tempo em cada TW e reamostragem; Blocked reservado à condição ainda indisponível | Corrigido no texto, limitado ao manual Z80 citado |
| MAJOR — entrada tardia em W | 16.11 registrava admissão, mas não proibia timestamp <= W fechado | 16.1/16.11 rejeitam LateInput <= W, inclusive T=W, sem reabrir o corte; replay restaura cursor/limite | Corrigido no texto |
| MAJOR — cancelamento após vencimento | 16.5 definia linearização operacional, não validade temporal C versus D | 16.5 separa instante causal de cancelamento, deadline D e despacho operacional atrasado; cancelamento posterior a D não suprime | Corrigido no texto; empate em D sem causalidade permanece indeterminado |
| MAJOR — propriedades R2 incompletas | T06/T08/T11/T12/T15 não enumeravam todas as corridas e estados parciais | Cenários ampliados para antes/depois de D, despacho atrasado, permutação, efeitos parciais, geração obsoleta, restore e LateInput | Especificado; não executado, pois R2 continua bloqueado |
| MINOR — grupos de rastreabilidade | C09 e C12 estavam agrupados em linhas incompatíveis com seus contratos | 16.17 separa WAIT (C09), IRQ/ack (C10/C11), concorrência (C07/C13) e host (C12) | Corrigido no texto |
| MINOR — medição como pré-requisito R1 | 16.14.3 sugeria medição antes de R3 | 16.14/18 tornam medição comparativa entrega R3; R1 exige apenas contrato de capacidade/observabilidade | Corrigido no texto |
| MAJOR — admissão de cancelamento retroativo em relação a W/P_d | v0.3.3 exigia C<D e admissão antes de W fechar D, mas não explicitava que C também precisava estar além do W vigente e do progresso dos participantes | v0.3.4 exige C>A(E), incluindo W se existir e P_d de todos os participantes; C<=W ou C<=P_d é LateInput mesmo quando W<D | Corrigido no texto; propriedade ampliada em R2-T06 |
| MAJOR — publicação de deadline retroativo | A tabela de operações exigia conversão, mas não explicitava rejeição de D anterior ao progresso do proprietário/W | v0.3.4 exige D>=P_owner e, se W existir, D>=W; igualdade requer microstep explícito; caso retroativo gera LateInput | Corrigido no contrato; cenário incluído em R2-T06 |
| MAJOR — entrada posterior a W mas retroativa a P_d | v0.3.3 rejeitava T<=W, mas poderia admitir T>W e T<=P_d de um domínio participante que já avançou | v0.3.4 define A(E)=max(W se existir, P_d convertido dos participantes), rejeita T<=A(E) e invalida H_d afetado após entrada admitida | Corrigido no contrato; propriedade ampliada em R2-T12 |
| MAJOR — snapshot de deadline Dispatched com efeitos parciais | v0.3.3 preservava Claimed, mas não explicitava o registro de subefeitos de um deadline cujo despacho já começou | v0.3.4 exige registro de commit por subefeito ou marco atômico; restore de Dispatched retoma pendências e restore de Failed preserva erro/efeitos parciais sem retry automático | Corrigido no contrato; cenário ampliado em R2-T11 |
| MAJOR — precedência de gates divergente em documentos de planejamento | CHRONOS_MINIMAL_HYBRID_SPEC.md e ROADMAP.md continham planos antigos que agrupavam R1/R2, classificavam a primeira fatia como R3 e/ou colocavam integração antes de R5 | v0.3.4 alinha ambos à sequência estrita R0→R1→R2→R3→R4→R5; R3 fica restrito a protótipos descartáveis e a fatia de produção ocorre somente pós-R5 | Corrigido nos documentos de planejamento; R1 continua em revisão |
| MINOR — H_d não explícito no limite de avanço | v0.3.3 descrevia o próximo limite, mas não listava H_d convertido entre os limites mínimos da solicitação | v0.3.4 inclui H_d convertido explicitamente no menor limite de avanço e amplia R2-T04 | Corrigido no texto |
| MINOR — LateInput usado sem entrada na taxonomia de erros | v0.3.4 usava LateInput em admissão, cancelamento e publicação retroativa, mas §16.12 não o definia explicitamente | v0.3.5 definiu LateInput, diagnóstico e rejeição pré-mutação; C15 e R2-T15 incluem a classe | Corrigido na v0.3.5 |

| P2 — empate de cancelamento versus fronteira de admissão | v0.3.5 permitia que C=D fosse tratado apenas pela regra de microstep, sem explicitar a precedência de C<=A(E) | v0.3.6 exige admissão C>A(E) para todo cancelamento; C<=A(E) é LateInput antes da arbitragem de empate | Codex review da v0.3.6 concluída sem major issues; correção preservada na v0.3.7 |
| P2 — classe obrigatória para timestamp tardio | §16.1.11 permitia LateInput/InvalidInput para o mesmo caso | v0.3.6 torna LateInput obrigatório para timestamp temporalmente inadmissível e reserva InvalidInput a parâmetros/perfil/IDs/gerações inválidos | Codex review da v0.3.6 concluída sem major issues; correção preservada na v0.3.7 |
| P2 — igualdade causal indeterminada sem cenário R2 | §16.12/C06 definiam IndeterminateCausalOrder, mas R2-T06/T15 não exigiam o caso D=P_d sem microstep | v0.3.6 inclui o cenário; v0.3.8 mantém IndeterminateCausalOrder para D=P_d sem microstep somente quando W<D (ou W não existe), enquanto D<=W é LateInput | Codex review da v0.3.6 sem major issues; semântica refinada na v0.3.8 |

| MAJOR — deadline publicado no instante de watermark já fechado | v0.3.7 tentou limitar igualdade D=P_d/W a uma cadeia causal 'aberta', mas não definiu um critério verificável e contradisse W como corte global já fechado | v0.3.8 proíbe D<=W sempre, inclusive D=W com microstep; define a condição objetiva para D=P_d: W ausente ou W<D; atualiza C06 e R2-T06/T15 | Correção proposta; aguarda nova revisão independente |
| BLOCKER — R1-EXIT-03: ciclo de vida transacional e fronteiras de barramento | §7 descrevia estados conceituais sem regras completas de entrada/saída, transições, amostragem, commit, cancelamento e restore; marcos não estavam mapeados a fronteiras nomeadas | v0.3.9 define estados de controle, transições permitidas, marcadores independentes, cancelamento/falha/restore e matriz de fronteiras por classe de operação; fronteiras físicas sem fonte permanecem UNKNOWN e limitam alegação de compatibilidade | Correção proposta; requer revisão independente do SHA candidato |
| BLOCKER — R1-EXIT-01: avanço retroativo e taxonomia | C01 permitia InvalidInput ou InvariantViolation para retrocesso, sem reconciliar a taxonomia de §16.12 | v0.3.9 classifica pedido externo T<F_d como LateInput, regressão interna como InvariantViolation e reserva InvalidInput a parâmetros/perfil/identidade/ID/geração inválidos; atualiza §12.1, §16.12 e R2-T01/T15 | Correção proposta; requer revisão independente do SHA candidato |

Limites da revisão:
- A base v0.3.1 fornecida pelo parecer é histórica; o confronto foi feito contra o conteúdo atual v0.3.2 e R0 aprovado.
- Os itens “corrigido” significam que a lacuna normativa foi endereçada no texto. Não significam que as propriedades R2 foram executadas, que o hardware foi medido ou que o gate R1 foi aprovado.
- Uma entrada E só é admitida quando T>A(E), sendo A(E) o maior entre W (se existir) e P_d convertido de todos os domínios participantes. Portanto T<=W ou T<=P_d de qualquer participante é LateInput, mesmo se T>W; para cancelamento, vale igualmente C<=W ou C<=P_d mesmo quando W<D. Se não houver W, o progresso local dos participantes continua limitando a admissão. A regra é de admissão/replay, não afirmação física sobre hardware. Uma entrada aceita invalida H_d que pressupunha sua ausência e exige recálculo antes do próximo avanço.
- Um deadline Dispatched com múltiplos efeitos precisa preservar cada commit; o estado do core e o marcador devem pertencer ao mesmo corte atômico. Quando isso não puder ser garantido, o efeito deve ser idempotente por identidade estável ou o cenário não pode ser declarado suportado. Um efeito indivisível precisa preservar seu único marco atômico. O contrato não autoriza repetir efeitos para “recuperar” dispatch.
- A relação C/D em empate continua explicitamente UNKNOWN quando o perfil não fornece relação causal/microstep; não se inventa prioridade física.
- A semântica WAIT deriva do Zilog Z80 User Manual UM008011-0816, R0 §2. WAIT externo de cartridge continua UNKNOWN conforme R0 §5.
- A regra LateInput é uma decisão normativa do contrato de replay/watermark, não uma afirmação física de hardware.

## 20. Próximos passos

A v0.3.9 é uma revisão corretiva motivada por dois blockers de saída: R1-EXIT-01 (classificação de avanço retroativo) e R1-EXIT-03 (máquina de estados transacional, regras de restore e mapeamento dos marcos às fronteiras de barramento). A correção documental não constitui aprovação; o novo SHA exige revisão independente e reavaliação explícita dos critérios.

1. Revisar adversarialmente a v0.3.9 no SHA exato, conferindo C01, §7, §12.1, §14, §16.1, §16.12, R2-T01/T08/T11/T15 e a matriz C01–C16.
2. Confirmar que cada classe de operação tem marcos nomeados e que UNKNOWN físico permanece explícito, sem fase/WAIT/microtiming inventado.
3. Atualizar a issue #1270 com o SHA candidato, revisão, checks e estado de cada critério; manter R1 não aprovado enquanto houver blocker.
4. Aprovar R1 formalmente somente após todos os critérios da seção 14 serem atendidos na versão exata e a revisão independente não encontrar blockers.
5. Iniciar R2 apenas após aprovação formal de R1; R2 deve ser independente deste contrato e do algoritmo candidato.
6. R3 só começa após R1 e R2 aprovados. Nenhum crate de produção antes de R5.
