# YM2413 — auditoria de cadência PG, PM LFO e AM LFO

**Data:** 2026-11-19
**Escopo:** comparar as cadências publicadas de PG/PM/AM com os contratos atuais de `PhaseGenerator`, `PhaseModulationLfo`, `AmLfo` e `Ym2413SynthesisBank`.
**Limite:** auditoria de componentes digitais e evidência documental; não define offsets por subfase dos 18 slots nem alinhamento absoluto após reset.

## 1. Evidência de PM — Andete 2015-12-01

A medição descreve:
- oito posições da tabela PM;
- uma posição mantida por 1.024 amostras YM2413;
- repetição do padrão a cada 8.192 amostras;
- posição escolhida por `counter >> 10`;
- correção de fase dependente de FNUM[8:6] e da posição PM.

A amostra usada nessa análise é a saída interna do YM2413, com duração de 72 clocks de entrada. A frequência calculada do ciclo PM é aproximadamente 6,069 Hz.

A comparação com a captura real foi aproximada, não uma validação perfeita: Andete relata que o contador global de 13 bits provavelmente não estava sincronizado ao início da senoide, e a medição continha modulator + carrier enquanto a simulação usava um único operador.

Fonte: [Andete — YM2413 Reverse Engineering Notes 2015-12-01](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01), especialmente linhas/seções de 1.024/8.192 amostras, seleção `counter >> 10` e limitações da medição.

## 2. Evidência de AM — Andete 2015-11-28

A medição descreve:
- uma sequência triangular de 210 posições;
- avanço de uma posição a cada 64 amostras;
- período de 13.440 amostras;
- frequência calculada de aproximadamente 3,699 Hz;
- 14 níveis de atenuação (0..13), após descartar o bit menos significativo da sequência original;
- profundidade próxima de 4,8 dB.

O artigo descreve a duração da tabela e sua forma, mas não estabelece que o contador AM seja fisicamente o mesmo contador global usado pelo EG ou PM, nem fixa sua fase após reset IC.

Fonte: [Andete — YM2413 Reverse Engineering Notes 2015-11-28](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-11-28), seções de duração da tabela, medição e profundidade.

## 3. Inspeção do código atual

### 3.1 PG

`PhaseGenerator::advance`:
- atualiza um acumulador individual de 19 bits;
- calcula um incremento por chamada;
- aplica PM usando a posição recebida pelo chamador;
- executa wrap módulo (2^{19});
- não avança clock global nem decide em qual slot deve ser chamado.

`Ym2413SynthesisBank::advance_operator_phase` lê a posição PM atual do LFO e a fornece ao PG selecionado. Não existe latch da posição PM por rotação nem dispatcher de slots nesta camada.

### 3.2 PM LFO

`PhaseModulationLfo`:
- mantém contador candidato de 13 bits;
- seleciona posição pelos bits 12..10;
- avança por quantidade explícita de amostras;
- faz wrap em 8.192 amostras;
- exige fase inicial explícita e serializa o contador.

Isso corresponde à cadência/decodificação descritas por Andete. Não prova a fase inicial física nem o ponto de observação relativo ao PG.

### 3.3 AM LFO

`AmLfo`:
- mantém fase independente em unidades de amostra;
- implementa 210 posições, cada uma mantida por 64 amostras;
- faz wrap em 13.440 amostras;
- exige fase inicial explícita e serializa a fase.

A forma e a cadência correspondem às medições publicadas. O tipo de estado `u16` é uma escolha de representação do software, não uma afirmação sobre a largura do contador físico.

### 3.4 Banco de síntese

`Ym2413SynthesisBank::advance_lfo_samples(n)` avança PM e AM pelo mesmo número de amostras informado pelo chamador. O método:
- não avança input clocks, slots ou EG;
- não exige que `n` corresponda a rotações realmente concluídas;
- não define se o avanço acontece antes ou depois dos updates PG de uma rotação;
- não comprova que os dois LFOs compartilhem o mesmo contador físico.

A API é determinística e útil para testes isolados, mas a chamada correta depende de um owner temporal futuro.

## 4. Relação entre EG, PM e AM — evidência e incerteza

Andete 2015-03-20 descreve o contador EG global compartilhado pelos 18 operadores e observa que ele também é usado por outras funções, sem ainda identificar todas elas. Andete 2015-12-01 descreve o contador global de 13 bits usado para PM. A proximidade entre as descrições é compatível com um recurso temporal compartilhado, mas os textos não demonstram inequivocamente que o contador de EG e o contador PM sejam o mesmo registrador físico, com a mesma largura e o mesmo reset.

A nota AM de 2015-11-28 confirma a cadência em amostras, mas não identifica a origem física do contador AM. O período de 13.440 amostras não pode ser reconstruído apenas do contador PM de 13 bits módulo 8.192; seria necessário estado adicional ou outra divisão temporal.

**Não assumir:** EG=PM=AM como um único contador físico; também não assumir três osciladores fisicamente independentes. A arquitetura atual mantém estados separados, todos avançados por unidades de amostra explícitas, o que preserva a incerteza sem perder determinismo.

## 5. Contrato que a integração futura deve respeitar

1. A base nominal de serviço é uma amostra interna por rotação completa de 72 input clocks, não uma amostra CPAL de 48 kHz.
2. O PG de cada operador deve avançar exatamente na fronteira de atualização que for demonstrada para seu slot; não chamar o PG 18 vezes em paralelo nem fora da ordem serial.
3. O PM position usado por um PG deve ser o valor vigente na fronteira física de leitura. Até determinar se o PM avança antes ou depois do processamento da rotação, não escolher a ordem por conveniência.
4. O AM level deve ser lido na fronteira física de consumo do operador; o avanço do AM não pode ocorrer no meio da rotação de forma que alguns operadores usem a fase anterior e outros a seguinte, a menos que isso seja demonstrado.
5. Não reiniciar PM/AM por KEY-ON ou KEY-OFF sem evidência.
6. Manter fases iniciais explícitas e snapshots separados até que reset e eventual compartilhamento físico sejam medidos.
7. A atualização LFO e a atualização PG devem ser determinísticas e independentes da taxa do host/resampler.

## 6. Matriz de conclusão

| Item | Estado | Limite |
|---|---|---|
| PM: 8 posições × 1.024 amostras | **MEDIDO / IMPLEMENTADO como candidato** | Offset absoluto e reset pendentes |
| PM: 13-bit counter / bits 12..10 | **MEDIDO / IMPLEMENTADO como candidato** | A largura física total não é inferida para EG |
| AM: 210 × 64 amostras | **MEDIDO / IMPLEMENTADO como candidato** | Fase inicial e origem física pendentes |
| PG: acumulador 19-bit e incremento por amostra | **MEDIDO / IMPLEMENTADO como candidato** | Momento exato de atualização no slot pendente |
| PM aplicado ao PG | **MODELO consistente com a medição** | Fronteira de leitura/latched PM pendente |
| Relação EG↔PM | **INFERIDO / PENDENTE** | Identidade do contador não demonstrada |
| Relação AM↔EG/PM | **PENDENTE** | Não demonstrado contador comum nem independência física |
| Reset e KEY-ON/KEY-OFF | **PENDENTE** | Não reiniciar fases por hipótese |
| Integração a 18 slots / Scheduler | **PENDENTE** | Requer ordem serial e fronteiras de serviço verificadas |

## 7. Próximo experimento prioritário

Com hardware YM2413 original e aquisição sincronizada:
- capturar PM com FNUM conhecido por pelo menos 16.384 amostras e deslocar KEY-ON/KEY-OFF sem reset;
- capturar AM com uma senoide lenta e identificar as transições da tabela de 64 amostras;
- repetir após cold reset e warm reset;
- comparar fase PM e AM entre ensaios e contra as transições EG em operadores diferentes;
- correlacionar cada evento com contagem de input clocks, posição de slot e fronteira de rotação;
- não converter a observação para ciclos do host nem supor que o início da captura seja fase zero.

Sem essa captura, a cadência média é demonstrável, mas o alinhamento absoluto e a topologia física dos contadores permanecem abertos.

**Resultado:** nenhuma lógica de produção alterada. PG/PM/AM permanecem componentes candidatos isolados; FM-1/FM-3 seguem PARCIAIS. Não há integração cycle-accurate dos 18 slots.
