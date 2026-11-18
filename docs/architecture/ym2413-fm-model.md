# YM2413 / OPLL — Especificação arquitetural e modelo FM

**Projeto:** Sega Emulator — Emulação de Alta Precisão  
**Escopo:** core digital YM2413 independente de console/host; integração futura no perfil físico Japanese Master System.
**Estado:** modelo M0/M1 reconciliado por M2; a fundação FM-1 de registradores/clock está isolada em `native/ym2413`; não há síntese PCM nem integração de barramento SMS.
**Auditoria M2 independente:** [auditoria pré-implementação](../audits/ym2413-pre-implementation-audit-2026-11-17.md).

> Convenção de evidência usada neste documento:
> - **DOCUMENTADO:** descrito no manual Yamaha ou documentação técnica primária.
> - **MEDIDO:** resultado de engenharia reversa com observações em hardware real.
> - **INFERIDO:** hipótese consistente com evidências, ainda não comprovada diretamente no YM2413.
> - **PENDENTE:** exige medição, comparação diferencial ou decisão arquitetural.
>
> Não converter INFERIDO/PENDENTE em comportamento normativo do core.

---

## 1. Objetivo e limites

Modelar o Yamaha YM2413 (OPLL) como um dispositivo digital determinístico, independente de console e host, com estado integral, avanço explícito de clock e integração temporal pelo `MachineScheduler`.

O núcleo deve reproduzir:
- síntese FM de 9 canais melódicos;
- dois operadores por canal, com modulator e carrier;
- 15 instrumentos predefinidos e um instrumento programável;
- geradores de fase, envelope, AM e PM;
- feedback do modulator;
- modo Rhythm com cinco vozes de percussão;
- registradores, latch, temporização e pipeline interno;
- saídas digitais que representem os pinos de áudio do chip.

Fora do escopo do core:
- decodificação de portas específicas do Master System;
- presença física do FM em cada console;
- política de seleção PSG/FM;
- mixer, resampler e dispositivo de áudio do host.

Essas responsabilidades pertencem aos adaptadores e à configuração da máquina.

## 2. Parâmetros fundamentais

| Parâmetro | Valor / regra | Evidência |
|---|---:|---|
| Clock de entrada | 3.579.545 Hz | DOCUMENTADO |
| Slots por período de saída | 18 | DOCUMENTADO |
| Clocks por slot | 4 | DOCUMENTADO |
| Clocks por período de saída | 72 | DOCUMENTADO |
| Taxa interna de saída | 3.579.545 / 72 ≈ 49.716,04 Hz | Derivada |
| Canais melódicos | 9 | DOCUMENTADO |
| Operadores lógicos | 18 (2 por canal) | DOCUMENTADO |
| Células físicas do operador | 1, reutilizada sequencialmente | DOCUMENTADO |
| Resolução do DAC | 9 bits | DOCUMENTADO |
| Instrumentos fixos | 15 + patch de usuário | DOCUMENTADO |
| Saídas | MO e RO | DOCUMENTADO |

A taxa de aproximadamente 49,716 kHz é a taxa interna do gerador, **não** a taxa final do dispositivo de áudio. A conversão para 44,1/48/96 kHz deve preservar fase e histórico do resampler entre blocos.

### 2.1 Ordem dos slots

| Slot físico | Canal | Operador |
|---:|---:|---|
| 1 | 1 | Modulator |
| 2 | 2 | Modulator |
| 3 | 3 | Modulator |
| 4 | 1 | Carrier |
| 5 | 2 | Carrier |
| 6 | 3 | Carrier |
| 7 | 4 | Modulator |
| 8 | 5 | Modulator |
| 9 | 6 | Modulator |
| 10 | 4 | Carrier |
| 11 | 5 | Carrier |
| 12 | 6 | Carrier |
| 13 | 7 | Modulator |
| 14 | 8 | Modulator |
| 15 | 9 | Modulator |
| 16 | 7 | Carrier |
| 17 | 8 | Carrier |
| 18 | 9 | Carrier |

Cada slot ocupa quatro clocks de entrada. A visita de todos os slots ocupa 72 clocks. O modelo deve manter os 18 estados lógicos, mas executar as operações na ordem sequencial observada. Não implementar 18 engines independentes em paralelo.

O processamento em lote é permitido somente quando preserva a sequência de slots, os pontos de escrita de registradores, os eventos de envelope e os limites de saída. Uma escrita no instante T exige avançar o chip até T, aplicar a escrita na fase correta e continuar a partir dali.

## 3. Modelo de síntese

### 3.1 Caminho de sinal por operador

Cada operador possui, conceitualmente:
1. Phase Generator (PG);
2. forma de onda log-sine e conversão exponencial;
3. Envelope Generator (EG);
4. controle de amplitude e sinal.

O canal usa modulator seguido de carrier. O modulator pode receber feedback de sua própria saída histórica. A saída do modulator altera a fase do carrier.

Representação matemática conceitual:

\[
y(t)=A_c(t)\sin(\phi_c(t)+I(t)\sin(\phi_m(t)))
\]

Essa equação descreve a topologia funcional, não a aritmética interna exata. Larguras, deslocamentos, truncamentos e atrasos devem ser determinados pelas medições e pelo manual.

### 3.2 Phase Generator

**DOCUMENTADO/MEDIDO**
- O contador de fase identificado nas notas de engenharia reversa tem 19 bits, com representação fixa 10.9.
- Cada operador lógico mantém seu próprio estado de fase.
- FNUM, BLOCK e MUL controlam a frequência.
- PM/VIB pode alterar o incremento de fase.

Tabela de multiplicadores do campo MUL:

| MUL | Multiplicador |
|---:|---:|
| 0 | 0,5 |
| 1–9 | 1–9 |
| A–B | 10 |
| C–D | 12 |
| E–F | 15 |

Fórmula conceitual para o incremento base:

\[
\Delta P \propto FNUM\times MUL\times 2^{BLOCK}
\]

**PENDENTE:** escala exata, posição dos bits fracionários, truncamento, tratamento de overflow, fase inicial, cadência de atualização e aplicação precisa de PM. Não codificar a expressão simplificada como fórmula final sem confronto de vetores.

### 3.3 Log-Sine / EXP

O caminho de amplitude deve utilizar representação logarítmica e conversão exponencial por tabelas inteiras. O loop de síntese não deve chamar funções trigonométricas ou logarítmicas de ponto flutuante.

As notas de engenharia reversa documentam uma tabela de seno em domínio logarítmico e uma tabela EXP de 256 entradas. Uma implementação de referência aberta, `emu2413`, contém tabelas concretas e está licenciada sob MIT; deve ser usada como referência diferencial, não como prova isolada do comportamento físico.

**PENDENTE:** estabelecer os vetores oficiais do projeto para cada entrada, convenção de arredondamento, saturação, simetria de quadrantes, meia onda e conversão final para o DAC. Tabelas copiadas de implementações não devem ser aceitas sem comparação.

### 3.4 Envelope Generator

Não usar um ADSR genérico contínuo. O EG é uma máquina de estados discreta e temporizada por hardware.

Fases a modelar:
- DAMP;
- ATTACK;
- DECAY;
- SUSTAIN;
- RELEASE;
- variantes de release e transições dependentes de KEY-ON, KEY-OFF e SUS.

Parâmetros relevantes por operador:
- AR, DR, RR, SL;
- EG-TYP;
- KSR;
- KSL;
- AM enable.

Parâmetros por canal:
- KEY-ON;
- SUS;
- BLOCK/FNUM.

A relação de taxa documentada é:

\[
RATE=4R+R_{ks}
\]

com taxa nula quando R=0. A implementação ainda precisa reconstruir o gerador de taxas, seus contadores e padrões de incremento. Não aceitar uma tabela inventada ou uma curva exponencial aproximada como substituta.

**DIVERGÊNCIA A RESOLVER:** o manual e as medições/implementações de referência não apresentam a mesma granularidade de nível/atenuação em todas as descrições. As notas de andete descrevem 128 níveis com passo de 0,375 dB; o manual contém uma especificação de atenuação que precisa ser reconciliada com o comportamento medido. Não escolher um valor sem documentar a evidência e o teste que o confirma.

### 3.5 KSR e KSL

KSR afeta a taxa do envelope por meio da escala de nota derivada de BLOCK e FNUM. KSL aplica atenuação dependente da altura da nota.

A expressão simplificada `(BLOCK << 1) | (FNUM >> 8)` pode servir como índice inicial de análise, mas não é por si só a especificação completa. A tabela, o arredondamento e a aplicação de KSR/KSL devem ser confrontados com o manual e as notas de medição.

### 3.6 Feedback

O feedback é aplicado ao modulator. As medições indicam dependência das duas saídas anteriores do modulator, não de uma saída instantânea ideal.

O manual associa os valores FB a índices de modulação:

| FB | Índice documentado |
|---:|---:|
| 0 | 0 |
| 1 | π/16 |
| 2 | π/8 |
| 3 | π/4 |
| 4 | π/2 |
| 5 | π |
| 6 | 2π |
| 7 | 4π |

O estado precisa preservar pelo menos o histórico temporal exigido pelo modelo confirmado. A média das duas últimas amostras aparece nas implementações de referência e nas análises de engenharia reversa.

**PENDENTE:** largura do sinal histórico, ponto exato de truncamento, deslocamento de feedback por FB, sinal, saturação e pipeline por slot. Não fixar `9 - FB` ou qualquer shift simplificado sem vetores diferenciais.

### 3.7 LFO AM e PM

O chip possui modulação de amplitude e de fase. A documentação de aplicação indica modulações lentas; as notas de reverse engineering investigam suas tabelas e sequências.

O estado deverá incluir:
- contador/posição de AM;
- contador/posição de PM;
- fase e divisor de cada LFO;
- valores efetivos aplicados ao EG e PG.

**PENDENTE:** período exato em clocks, forma da tabela, fase após reset, sequência inicial e ponto de aplicação em cada slot. A frequência aproximada divulgada em documentos não substitui a sequência discreta.

## 4. Instrumentos e registradores

### 4.1 Instrumentos

- Instrument ID 0: patch de usuário programável nos registradores $00–$07.
- Instrument IDs 1–15: presets internos do YM2413.
- O instrumento selecionado é aplicado aos operadores do canal.
- O patch de usuário é global/compartilhado: todos os canais que selecionam ID 0 observam o mesmo conjunto de registradores.

**PENDENTE:** validar byte a byte a ROM dos 15 instrumentos no YM2413 original. Não importar ROM de VRC7, YMF281B ou derivativos FHB013 como se fosse automaticamente a ROM do YM2413. As notas de reverse engineering alertam que algumas imagens de die analisadas são de um derivativo FHB013, não de um YM2413 original.

### 4.2 Mapa funcional de registradores

| Endereço | Função |
|---|---|
| $00–$07 | Patch de usuário (parâmetros dos dois operadores) |
| $0E | Ativação e controles do Rhythm Mode |
| $0F | Registrador de teste; comportamento não destinado ao uso normal |
| $10–$18 | FNUM LSB dos canais 1–9 |
| $20–$28 | FNUM MSB, BLOCK, KEY-ON e SUS dos canais 1–9 |
| $30–$38 | Instrument ID e volume dos canais 1–9 |

Os campos reservados e os efeitos de escrita nos registradores de teste devem ser tratados como PENDENTES até haver evidência. Não normalizar escritas inválidas para um comportamento conveniente.

## 5. Rhythm Mode

O Rhythm Mode altera a utilização dos canais 7–9 e de seus seis operadores para formar cinco vozes de percussão:

| Voz | Slots envolvidos |
|---|---|
| Bass Drum | 13 e 16 |
| Hi-Hat | 14 |
| Tom-Tom | 15 |
| Snare Drum | 17 |
| Top Cymbal | 18 |

Ao ativar Rhythm Mode, os canais 7–9 deixam de ser três canais melódicos normais. Os registradores de frequência desses canais continuam relevantes para algumas vozes; o controle de volume e disparo das percussões passa a utilizar os campos específicos do registrador de ritmo.

Não presumir que desligar individualmente uma voz de percussão libere seu operador para recuperar um canal melódico.

**PENDENTE:** equações de fase metálica, combinações de ruído e fase, taps e estado inicial do gerador pseudoaleatório, sincronização do LFSR, envelopes e saída duplicada de percussão. O estado de ruído deverá ser serializado integralmente. Não assumir um polinômio ou seed como confirmado sem comparação com hardware.

## 6. Interface de barramento do chip

O core deve expor operações explícitas equivalentes a:
- reset;
- escrita de endereço/latch;
- escrita de dados;
- avanço de clocks;
- consulta/retirada de amostras já produzidas;
- exportação e restauração de estado.

O manual especifica:
- aguardar 12 clocks mestres entre escrita de endereço e escrita de dados;
- aguardar 84 clocks mestres depois da escrita de dados antes da próxima escrita de endereço ou dados.

Se os intervalos não forem respeitados, o manual classifica os dados como incertos. Não converter automaticamente esses intervalos em WAIT states de Z80. A camada de barramento deve registrar o timestamp da transação e o core deve modelar o comportamento de escrita no instante correto.

## 7. Integração Master System

### 7.1 Separação de responsabilidades

| Camada | Responsabilidade |
|---|---|
| `sega-audio` / core YM2413 | Síntese e estado digital do chip |
| Adaptador temporal | Relógio e sincronização com o Scheduler |
| Perfil físico SMS | Modelo, revisão, região e presença do chip |
| Barramento SMS | Portas e timestamps de I/O |
| Controle de áudio SMS | Seleção de saída PSG/FM conforme hardware |
| AudioMixer | Soma das fontes |
| Resampler | Conversão para a taxa de saída |
| Host | Dispositivo físico; sem controle de ciclos do chip |

O núcleo YM2413 não deve depender de tipos do Master System.

### 7.2 Portas SMS

- $F0: escrita do latch/endereço; somente os seis bits inferiores selecionam o registrador.
- $F1: escrita de dados no registrador previamente selecionado.
- $F2: controle de roteamento/mute e readback dependente do perfil físico.

No perfil Japanese Master System, os endereços $F0/$F1/$F2 são decodificados exatamente; não aplicar espelhamento genérico por máscara/paridade. As escritas devem produzir efeito no deadline de conclusão da transação de I/O, preservando timestamp e ordem causal. O intervalo de 12 clocks entre endereço e dados e de 84 clocks após dados é DOCUMENTADO pelo manual Yamaha; o efeito de violações permanece PENDENTE e não deve ser convertido em WAIT do Z80 sem evidência.

### 7.3 Perfis de hardware e controle $F2

Separar três conceitos:
1. capacidade física: o YM2413 existe nesta placa/revisão;
2. seleção/roteamento de saída: PSG e FM estão audíveis ou silenciados;
3. atividade de síntese: o chip continua avançando seu estado interno.

Tabela de saída documentada para o Japanese Master System:

| Valor escrito em $F2 | PSG | FM |
|---:|---|---|
| $00 | habilitado | silenciado |
| $01 | silenciado | habilitado |
| $02 | silenciado | silenciado |
| $03 | habilitado | habilitado |

No Japanese Master System, PSG e FM podem operar simultaneamente. No Mark III com FM Unit, o bit de controle do PSG não tem o mesmo efeito: a seleção é mutuamente exclusiva e o bit correspondente é forçado a zero. Não aplicar esse comportamento do Mark III ao SMS japonês.

A presença de FM não deve ser inferida apenas de `Territory::Japan`. O perfil deve representar explicitamente modelo/revisão e capacidade física. SMS exportação e variantes Tec Toy não devem receber YM2413 por suposição.

Na leitura de $F2 do Japanese Master System, D7/D6/D5 refletem os bits 11/7/3 de um contador de 12 bits incrementado por pulsos C-Sync na entrada do 315-5297; D4 é sempre zero e D3–D0 refletem os últimos quatro bits escritos. Na escrita, somente D1 (mute PSG) e D0 (mute YM2413) controlam o roteamento; D3–D2 não são controles de mute. A descrição publicada pelo SMS Power registra esse readback, mas a integração ainda exige teste por revisão física. O contador não pode ser sintetizado a partir de VCounter, scanline ou temporizador do host: o runtime ainda não expõe o sinal C-Sync necessário.

### 7.3.1 Matriz de contratos por modelo físico

| Modelo físico | Presença de FM | Roteamento/portas | Contrato desta trilha |
|---|---|---|---|
| Mark III sem FM Unit | Sem YM2413 instalado | Não há FM; SMS Power documenta leitura de $F2 a partir do port 0 neste perfil | Sem FM; não criar chip por configuração |
| Mark III + FM Sound Unit | Unidade FM externa; não é YM2413 integrado à placa | PSG e FM são mutuamente exclusivos; o bit de controle PSG de $F2 é forçado a zero | Contrato de compatibilidade apenas; o módulo externo e seu adaptador físico ficam fora da implementação |
| Japanese Master System MK-2000 | YM2413 integrado | $F0/$F1 para endereço/dados; $F2 permite PSG e FM simultâneos e expõe C-Sync no readback | Único perfil físico alvo da implementação integrada nesta trilha |
| Export SMS sem FM de fábrica | Sem YM2413 de fábrica | Não usar o readback C-Sync japonês; SMS Power relata D1–D0 = $02 em SMS não japonês sem FM | FM ausente por padrão; revisões modificadas exigem perfil físico explícito e evidência própria |

A matriz descreve o modelo físico, não a região do software. `Territory` continua sendo uma propriedade separada: `Territory::Japan` não prova que a placa contém YM2413, e uma ROM de território export pode executar em um perfil físico japonês.

### 7.3.2 Política FmMode — independente do modelo e do território

| FmMode | Política |
|---|---|
| `Auto` | Só habilita FM quando o modelo físico selecionado suporta o chip integrado e o catálogo identifica correspondência exata de título/revisão/hash com driver/dados FM comprovados. Sem correspondência exata, mantém FM desativado. |
| `On` | Override do catálogo para um modelo físico que já possua YM2413 integrado. Não adiciona hardware a Mark III sem unidade ou Export SMS sem FM; não implementa o Mark III FM Sound Unit. Também não fabrica writes, driver ou dados FM. |
| `Off` | Silencia a saída FM. Se o chip estiver fisicamente presente, o estado interno continua avançando, salvo evidência de que o hardware realmente faça clock-gating. |

O perfil Japanese Master System MK-2000 pode ser selecionado independentemente de `Territory`. A seleção física e `FmMode` não devem alterar BIOS, região do software ou modelo de VDP. Perfis desconhecidos permanecem sem FM em `Auto`; qualquer override de placa modificada deve ser explícito e separado do catálogo de compatibilidade.



### 7.4 PSG, MO/RO, mixer e resampler

PSG e YM2413 mantêm estados internos independentes. A relação de fonte elétrica e fase entre os clocks permanece **PENDENTE** (FM-A01); a igualdade da frequência nominal não prova clock comum ou alinhamento de fase.

O manual Yamaha identifica MO e RO como saídas analógicas source-follower que exigem estágio posterior. A medição publicada no MSX Resource Center descreve multiplexação de canais em 72 clocks: o valor de cada canal permanece ativo por aproximadamente três clocks e retorna ao nível neutro no restante da janela. Isso não fornece a rede de soma/ganho do SMS japonês. Preservar MO e RO separadamente; não somar como PCM mono normalizado nem transplantar o circuito SFG-05/MSX. A rede analógica específica da placa permanece **PENDENTE** (FM-A02).

No código auditado em M2:
- `AudioMixer::mix_mono_into` aceita várias fontes mono apenas quando os blocos têm o mesmo número de frames.
- `NativeAudioSession::process_samples` e o trait `AudioSession` recebem uma única fonte por chamada. Duas chamadas PSG/FM separadas concatenariam blocos no tempo, não os misturariam.
- A taxa PCM interna canônica é 48.000 Hz; `PcmResampler::new` recebe taxas explícitas de entrada/saída e a taxa host é negociada.
- A taxa nominal YM2413 de 3.579.545/72 ≈ 49.716,04 amostras/s é derivada, não taxa final de host.

A integração futura deve alinhar determinística e continuamente as fontes antes de uma única operação de mixagem por intervalo (ou generalizar explicitamente o mixer para fontes com timestamps/taxas diferentes). Não entregar PSG e FM em chamadas sequenciais à sessão e não duplicar mixer/resampler por chip.

## 8. Scheduler e invariância temporal

O `MachineScheduler` existente continua sendo o único dono da linha do tempo. O core YM2413 recebe avanços explícitos de clock; não possui relógio de host nem Scheduler próprio.

A escolha entre consumir o mesmo `ClockAdvance` do domínio PSG ou registrar um domínio FM explícito **permanece PENDENTE** até evidência elétrica da placa alvo (FM-A01). O worker atual usa um tick lógico por T-state Z80 e o domínio PSG tem razão 1/1 nessa linha do tempo; isso não comprova fase elétrica comum. MAME XTAL(10,738,635)/3 corrobora a frequência nominal, não a conexão física. Se a evidência futura demonstrar clock compartilhado, reutilizar o avanço do PSG; se demonstrar fonte/fase distinta, registrar outro domínio no mesmo `MachineScheduler`. Nunca criar Scheduler paralelo.

Em ambos os casos, a integração deve avançar o chip até o timestamp de conclusão de cada escrita, preservar resto de clock/slot/subfase, respeitar eventos simultâneos e manter snapshot/replay coerentes. Só definir ID/domínio e versionamento após fechar FM-A01.

## 9. Save State

Serializar, no mínimo:
- banco completo de registradores e latch;
- contador de clock e fase residual;
- slot corrente e subfase;
- estado dos 9 canais;
- estado dos 18 operadores;
- PG, EG, AM, PM;
- dois históricos de feedback por operador/canal conforme o modelo final;
- estado do Rhythm Mode e do gerador pseudoaleatório;
- pipeline de saída e amostras pendentes necessárias à equivalência;
- estado do adaptador e domínio de clock;
- controle $F2, perfil físico e roteamento SMS;
- estado do resampler e mixer que afete a continuidade.

Restaurar deve validar coerência entre timestamp do Scheduler, clock acumulado do chip, eventos pendentes, estado do barramento e fase do resampler. Não basta serializar registradores visíveis.

## 10. Estratégia de implementação incremental

1. Fechar especificação de PG, EG, AM/PM, feedback e Rhythm com evidências.
2. Fixar vetores dourados e resultados diferenciais para cada bloco.
3. Implementar núcleo puro em Rust, sem integração SMS.
4. Validar reset, registradores, clock, invariância de blocos e snapshots.
5. Implementar presets e patch de usuário.
6. Validar síntese melódica e envelopes.
7. Validar Rhythm Mode.
8. Implementar adaptador temporal no único MachineScheduler; compartilhar o domínio PSG somente se FM-A01 for fechado positivamente, caso contrário registrar domínio explícito na mesma timeline.
9. Integrar portas $F0–$F2 por perfil físico.
10. Integrar fontes PSG + FM no mixer e no resampler.
11. Executar regressão SMS e SG-1000 e todos os gates do workspace.

## 11. Matriz de aceitação

| ID | Teste | Estado inicial |
|---|---|---|
| YM-001 | Clock de entrada e 18 slots / 72 clocks | NOT RUN |
| YM-002 | Ordem dos slots e correspondência canal/operador | NOT RUN |
| YM-003 | Escrita de endereço com latência de 12 clocks | NOT RUN |
| YM-004 | Escrita de dados com latência de 84 clocks | NOT RUN |
| YM-005 | Tabela MUL e sweep FNUM/BLOCK | NOT RUN |
| YM-006 | Vetores Log-Sine/EXP | NOT RUN |
| YM-007 | Attack/Decay/Sustain/Release e DAMP | NOT RUN |
| YM-008 | KSR/KSL e SUS | NOT RUN |
| YM-009 | AM/PM e fase inicial | NOT RUN |
| YM-010 | Feedback com histórico e atrasos | NOT RUN |
| YM-011 | Presets originais e patch de usuário | NOT RUN |
| YM-012 | Cinco vozes de Rhythm Mode | NOT RUN |
| YM-013 | Continuidade de execução contínua vs blocos | NOT RUN |
| YM-014 | Snapshot/restore bit-exato e continuidade de áudio | NOT RUN |
| YM-015 | $F2 Japanese SMS: PSG/FM simultâneos | NOT RUN |
| YM-016 | $F2 Mark III FM Unit: seleção exclusiva | NOT RUN |
| YM-017 | SMS sem FM e perfis desconhecidos | NOT RUN |
| YM-018 | Mixagem PSG + FM e resampling | NOT RUN |
| YM-019 | Regressão SG-1000 e SMS | NOT RUN |
| YM-020 | Gates Rust do workspace | NOT RUN |

Nenhum teste é PASS por ter sido ignorado ou não executado. Registrar PASS, FAIL, BLOCKED ou NOT RUN com evidência e SHA.

## 12. Referências técnicas

### Documentação primária / técnica
- Yamaha YM2413 Application Manual, espelhado em SMS Power: https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
- SMS Power — YM2413: https://www.smspower.org/Development/YM2413
- SMS Power — Audio Control Port: https://www.smspower.org/Development/AudioControlPort
- SMS Power — índice das notas de reverse engineering: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes
- Andete — repositório de medições e notas: https://github.com/andete/ym2413

### Implementações de referência (uso diferencial, não autoridade única)
- emu2413 (MIT): https://github.com/digital-sound-antiques/emu2413
- Nuked-OPLL: https://github.com/nukeykt/Nuked-OPLL
- ymfm: https://github.com/aaronsgiles/ymfm
- ares: https://github.com/ares-emulator/ares

Auditoria de licença/proveniência consultada nesta revisão:
- `emu2413` declara MIT, mas seus acknowledgements listam fontes de presets VRC7 e YMF281B. Pode servir como diferencial sample-oriented após pinagem de commit; não tratar automaticamente suas tabelas/presets como a ROM de instrumentos original do YM2413.
- `ymfm` declara BSD-3-Clause e pode ser outro diferencial independente, sempre com commit/licença registrados.
- `Nuked-OPLL` declara GPL-2.0-or-later; `emu198x_yamaha_ym2413` também está sob GPL-2.0-or-later. Não copiar nem criar dependência desses projetos no runtime MIT.
- O die analisado por andete é de um derivativo FHB013; não tratá-lo como fotografia do silício YM2413 original. Não misturar presets/tabelas de VRC7, YMF281B ou FHB013 com o banco original YM2413.

Links de licença/proveniência: [emu2413 MIT](https://github.com/digital-sound-antiques/emu2413/blob/develop/LICENSE), [emu2413 acknowledgements](https://github.com/digital-sound-antiques/emu2413#acknowledgements), [ymfm BSD-3-Clause](https://github.com/aaronsgiles/ymfm/blob/main/LICENSE), [Nuked-OPLL GPL](https://github.com/nukeykt/Nuked-OPLL/blob/master/LICENSE).

---

## 13. Decisões congeladas nesta revisão

- Núcleo YM2413 compartilhado e independente do SMS.
- Backend de hardware 100% Rust.
- Scheduler único, sem callbacks por instrução.
- Operador processado sequencialmente em 18 slots.
- PSG + FM simultâneos no perfil Japanese SMS.
- Mark III FM Unit tem contrato de roteamento exclusivo documentado, mas o módulo externo não será implementado nesta trilha.
- Não presumir FM em Export SMS ou Tec Toy sem perfil físico e evidência.
- A síntese digital pura pode avançar em etapas isoladas com modelos candidatos e pendências explicitamente rastreadas; FM-A01/A02/A03/A04/A05/A07 bloqueiam integração de hardware/áudio, não a construção isolada dos operadores.
- Incertezas digitais de PG/EG/feedback/pipeline, estado inicial, presets originais e comportamento de writes fora de 12/84 continuam gates de fidelidade: exigir vetores/diferenciais antes de declarar PASS.
- Nenhum resultado autoriza alegação cycle-accurate sem comparação independente no escopo declarado.
- Nenhuma mudança de hardware é considerada concluída sem testes diferenciais, temporais, de snapshot e regressão.


---

## 14. Reconciliação de evidências M1 — PG, PM, EG e feedback (2026-11-06)

Esta revisão transforma parte dos itens pendentes de M0 em modelos candidatos com evidência publicada. Ela **não** declara o core implementado e não substitui vetores do projeto nem validação independente no YM2413 original.

### 14.1 Phase Generator — incremento base

As medições de andete sustentam um acumulador de fase de 19 bits em formato 10.9. Os 10 bits superiores selecionam a posição da tabela seno; os 9 bits inferiores preservam a fração. A ausência de interpolação é uma inferência consistente com as medições, não uma especificação elétrica independente.

Tabela de multiplicadores usada no cálculo:

| MUL | ML |
|---:|---:|
| 0 | 1 |
| 1 | 2 |
| 2 | 4 |
| 3 | 6 |
| 4 | 8 |
| 5 | 10 |
| 6 | 12 |
| 7 | 14 |
| 8 | 16 |
| 9 | 18 |
| A | 20 |
| B | 20 |
| C | 24 |
| D | 24 |
| E | 30 |
| F | 30 |

Incremento base medido, sem PM:

`phase_step = ((FNUM * ML[MUL]) << BLOCK) >> 1`

O acumulador deve manter os 19 bits efetivos; o cálculo intermediário do incremento pode exceder 19 bits, mas a soma observável é reduzida ao domínio de fase de 19 bits. FNUM=0 não avança a fase. MUL=0, BLOCK=0, FNUM=1 também resulta em incremento zero por causa do deslocamento à direita.

### 14.2 Phase Modulation / Vibrato

As notas de medição apresentam uma tabela PM de 8 × 8 indexada por `FNUM[8:6]` e pela posição do LFO. O índice de posição avança a cada 1024 amostras internas; o ciclo de oito posições dura 8192 amostras. Com 3.579.545 / 72 ≈ 49.716,04 amostras/s, o período corresponde a aproximadamente 6,069 Hz.

`pm = pmTable[FNUM >> 6][(global_sample_counter >> 10) & 7]`

`phase_step = (((2 * FNUM + pm) * ML[MUL]) << BLOCK) >> 2`

A expressão reduz-se ao incremento base quando PM=0. A tabela medida é:

| FNUM[8:6] | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 1 | 0 | 0 | 1 | 0 | 0 | 0 | -1 | 0 |
| 2 | 0 | 1 | 2 | 1 | 0 | -1 | -2 | -1 |
| 3 | 0 | 1 | 3 | 1 | 0 | -1 | -3 | -1 |
| 4 | 0 | 2 | 4 | 2 | 0 | -2 | -4 | -2 |
| 5 | 0 | 2 | 5 | 2 | 0 | -2 | -5 | -2 |
| 6 | 0 | 3 | 6 | 3 | 0 | -3 | -6 | -3 |
| 7 | 0 | 3 | 7 | 3 | 0 | -3 | -7 | -3 |

**PENDENTE:** fase inicial e alinhamento do contador global com reset, slots e atualização dos registradores; ordem exata entre incremento de fase e leitura da fase no mesmo slot; confirmação do comportamento de PM em todas as combinações de frequência/MUL.

### 14.3 Envelope Generator — níveis e escala

As medições publicadas sustentam um nível de envelope de 7 bits (0–127), com passo de 0,375 dB. O nível 127 corresponde a -47,625 dB; 128 níveis cobrem aproximadamente 48 dB. O manual convertido para HTML informa 0,325 dB, valor incompatível com 128 níveis em 48 dB e com as medições. A hipótese de erro de transcrição no manual é forte, mas a divergência deve permanecer registrada até que os vetores de referência do projeto a encerrem.

O sustain level foi medido como:

`EG_sustain_level = 8 * SL`

Cada incremento de SL representa 3 dB. O KSR usa:

`BF = (BLOCK << 1) | FNUM[8]`

`RKS = KSR ? BF : (BF >> 2)`

`RATE = min(63, 4 * R + RKS)`

O texto do manual confirma a relação RATE=4R+RKS e que RATE=0 não altera o envelope. A tabela de medições de andete detalha o escalonamento do RKS e as curvas de ataque/decay. Há listagens C++ nas notas de 2017 que exibem `max(63, 4*R + RKS)` e `max(127, env + step)`; essas expressões contradizem os limites superiores documentados/medidos e o próprio comportamento descrito. Tratá-las como provável erro tipográfico da listagem: a hipótese coerente é saturar superiormente com `min(63, ...)` e `min(127, ...)`, sujeita a vetores antes de codificar.

### 14.4 EG — temporização candidata e transições observadas

O modelo de andete descreve um contador global compartilhado, incrementado uma vez por amostra de saída (uma travessia completa dos 18 slots, equivalente a 72 clocks de entrada), que distribui as atualizações de envelope. Para as taxas de decay usuais:

- `eg_shift = 13 - (RATE / 4)`
- `eg_select = RATE & 3`
- a atualização só ocorre quando os bits inferiores exigidos do contador global são zero;
- uma tabela 4 × 8 seleciona se o nível avança 0 ou 1 passo.

Para ATTACK, o mesmo escalonamento é usado com exceção de que os dois bits inferiores são ignorados; quando a entrada da tabela permite atualização, o nível é transformado por uma das funções discretas:
- `f4(x) = x - (x >> 4) - 1`
- `f3(x) = x - (x >> 3) - 1`
- `f2(x) = x - (x >> 2) - 1`
- `f1(x) = x - (x >> 1) - 1`

As taxas rápidas 12–14 usam combinações específicas dessas funções. RATE 15 é instantâneo quando selecionado na transição DAMP→ATTACK; se alterado para 15 durante uma fase ATTACK já iniciada, as medições indicam que o avanço pode ficar parado. Não simplificar as taxas rápidas como uma curva exponencial contínua.

As medições também registram:
- DAMP com taxa-base 12, aproximadamente 10,22 ms para a queda de 0 a -48 dB; a transição observada para ATTACK pode ocorrer perto de -45 dB.
- O início do ATTACK pode ser nível 124 ou 127, dependendo se a sequência anterior concluiu RELEASE ou permaneceu em DAMP.
- EG_TYPE seleciona comportamento percussivo/sustentado: ao atingir SL, o tom sustentado mantém o nível até KEY-OFF; o percussivo passa para RELEASE.
- O modelo de engenharia reversa atribui estados DAMP, ATTACK, DECAY e SUSTAIN, com RELEASE condicionado a KEY-OFF e aos parâmetros EG_TYPE/SUS.

**PENDENTE:** alinhar exatamente o contador global e as atualizações EG aos 18 slots e aos clocks de entrada; resolver o ponto de amostragem de RATE e dos parâmetros alterados durante uma fase; fixar a semântica do operador modulator em KEY-OFF e os estados de release; criar vetores para todas as taxas 0–63, incluindo transições durante a execução.

### 14.5 Feedback do modulator

O modelo que reproduz as medições usa duas saídas históricas do modulator (`p0`, `p1`), cada uma representada em 12 bits com faixa assinada aproximada -2047…+2047:

`feedback_phase = FB == 0 ? 0 : (p0 + p1) >> (8 - FB)`

O resultado do modulator é reduzido antes de atualizar o histórico; o carrier recebe modulação derivada da saída recente do modulator. As medições mostram FB=0 sem feedback e aumento de instabilidade/caos para feedback alto em determinadas configurações.

**PENDENTE:** inicialização física de p0/p1, ordem de atualização por slot, larguras e truncamentos intermediários, tratamento de overflow e qualquer offset de fase no primeiro ciclo. O modelo de andete reproduz os dados medidos após sincronização, mas reporta divergências iniciais dependentes das condições iniciais.

### 14.6 Log-Sine / EXP e limites de reutilização

As notas de engenharia reversa descrevem duas tabelas de 256 entradas:
- `logsin[i] = round(-log2(sin((i + 0.5) * PI / 512)) * 256)`
- `exp[i] = round((2^(i / 256) - 1) * 1024)`

O caminho de lookup usa espelhamento de quadrante e sinal; a conversão EXP usa mantissa, expoente e complemento de um para o sinal negativo. Essas fórmulas constituem um modelo de referência que deve ser transformado em tabelas inteiras fixas e testado por vetor. Não executar trigonometria/logaritmos no caminho de síntese.

### 14.7 Pipeline de slots — refinamento arquitetural

A entrada é dividida por quatro; cada clock interno processa um dos 18 slots. O pipeline não deve ser interpretado como 18 operadores físicos paralelos: PG, EG e operador/DAC são estágios sequenciais com offsets de slot, trabalhando em contextos de slots distintos no mesmo clock interno. Um período completo contém 18 clocks internos, ou 72 clocks de entrada.

A implementação Rust de referência `emu198x_yamaha_ym2413` documenta seguir o pipeline e os offsets de Nuked-OPLL 1.0.2 (commit `1269cf5a`) e comparar os pinos com Nuked clock a clock. É uma referência técnica útil para ordenar os estágios, não prova independente de silício. O crate e Nuked-OPLL são GPL-2.0-or-later: não copiar código ou tabelas para o projeto sem decisão explícita de compatibilidade de licença. A arquitetura do Sega Emulator deve manter implementação própria e vetores de validação independentes.

### 14.8 Critérios de aceitação M1 adicionais

| ID | Critério | Estado |
|---|---|---|
| YM-021 | Vetores do incremento base para todos os MUL, FNUM e BLOCK, incluindo casos de incremento zero e wrap de 19 bits | NOT RUN |
| YM-022 | Sequência PM 8 × 8 e período do LFO, com alinhamento de reset documentado | NOT RUN |
| YM-023 | Níveis EG 0–127 e escala 0,375 dB reconciliados com vetores | NOT RUN |
| YM-024 | KSR/RKS e RATE 0–63, inclusive padrões de atualização global | NOT RUN |
| YM-025 | ATTACK f1–f4, taxas 12–15 e condições de início 124/127 | NOT RUN |
| YM-026 | DAMP, KEY-ON/OFF, EG_TYPE, SUS, DECAY e RELEASE por operador | NOT RUN |
| YM-027 | Feedback p0/p1 com reset, FB 0–7, TL e alinhamento por slot | NOT RUN |
| YM-028 | Pipeline PG/EG/OP por slot e saídas MO/RO em cada clock interno | NOT RUN |

### 14.9 Fontes desta revisão

- Yamaha YM2413 Application Manual, seções PG, EG e OP/DAC: https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
- Andete, phase counter / fórmula base: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-16
- Andete, PM/vibrato: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01
- Andete, decay/release: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20
- Andete, SL/KSR: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-24
- Andete, feedback: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-12
- Andete, envelope, ATTACK e transições: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31 e https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2017-01-26
- Pipeline Rust comparativo (Nuked-OPLL 1.0.2): https://docs.rs/emu198x-yamaha-ym2413/latest/emu198x_yamaha_ym2413/
- Implementação de referência emu2413 (MIT; usar somente para comparação e conferir branch/commit): https://github.com/digital-sound-antiques/emu2413

**Status M1:** PG base e PM têm fórmulas candidatas com suporte de medições; a escala EG de 0,375 dB e a temporização por tabela estão caracterizadas; feedback possui modelo que reproduz medições após sincronização. Pipeline exato, estados iniciais, todos os offsets, integração de slots e vetores completos continuam pendentes. Nenhum core YM2413 foi implementado por esta atualização documental.


### 14.10 Auditoria de integração SMS/áudio contra o `main` (SHA `c65594cf840090bad6036810a0df98f22b9d1484`)

A leitura dos fontes ativos confirmou os seguintes limites concretos:

1. **Domínio de clock compartilhável.** O coordenador SMS registra exatamente três domínios: Z80 (ID 0), PSG (ID 1) e VDP (ID 2). O restore exige exatamente três domínios. O PSG recebe 3.579.545 Hz; o YM2413 também recebe 3.579.545 Hz. Como ambos compartilham a mesma fonte/razão de clock no perfil SMS NTSC considerado, o adapter FM deve consumir o mesmo `ClockAdvance` do domínio 1, sem registrar um quarto domínio nem avançar o Scheduler duas vezes. O estado interno de fase/divisor de cada chip continua independente.

2. **Barramento FM ausente.** `SmsIo::write_port` trata somente o registrador de controle $3F e declara explicitamente que FM ainda não está conectado. `SmsTemporalBus::write_port` intercepta VDP, PSG e memória; $F0/$F1/$F2 atualmente caem no caminho genérico de I/O. `read_port` também não fornece o contador/control readback do FM. O decode de portas, a ordem do latch/endereço, a escrita de dados e o readback de $F2 precisam ser implementados no barramento SMS, com timestamp de conclusão do ciclo Z80.

3. **Save State incompleto para FM.** `SmsTemporalBusState` serializa atualmente `psg`, `vdp`, `io` e o estado do coordenador, mas não contém latch FM, registradores, slots, operadores, LFOs, pipeline, saídas MO/RO ou estado de mix/resampling FM. A adição do chip exige extensão/versionamento do estado do barramento e validação cruzada com o perfil físico do SMS. Como não se adiciona domínio ao Scheduler, a versão do estado do coordenador não deve ser incrementada apenas por essa razão; a versão do estado do barramento deve ser revista.

4. **Mixer multi-source ainda não está exposto pela sessão.** `AudioMixer::mix_mono_into` aceita várias `MixSource` com o mesmo número de frames, mas `NativeAudioSession::process_samples` constrói sempre um array com uma única fonte. Chamá-lo uma vez para PSG e outra para FM não mistura os chips: concatena dois blocos temporalmente no resampler e envia blocos separados. É necessário expor uma operação de sessão que receba todas as fontes alinhadas e invoque o mixer uma única vez por lote; manter `process_samples` como wrapper de fonte única é compatível.

5. **Taxas nativas diferentes.** O SN76489 produz amostras na taxa solicitada ao core e aplica seu próprio filtro/downsampling a partir do clock interno dividido por 16. O `PcmResampler` de `sega-audio-runtime` assume entrada fixa de 44.100 Hz e converte para a taxa de saída do host. O YM2413 produz uma amostra por 72 clocks de entrada (aprox. 49.716,04 amostras/s). Portanto, a saída YM2413 não pode ser entregue diretamente ao atual `NativeAudioSession` como se fosse 44.100 Hz. Antes da mistura, deve existir uma conversão determinística para uma taxa comum (ou uma generalização explícita do mixer/resampler para fontes com taxas e timestamps próprios). A fase dos conversores precisa ser derivada do tempo do Scheduler, não do tamanho dos blocos da GUI/host.

6. **MO/RO e circuito analógico.** O manual Yamaha identifica MO e RO como saídas source-follower e exige circuito integrado/amplificador posterior; portanto, os pinos não devem ser tratados como amostras digitais já normalizadas nem somados sem modelo de carga. O manual mostra um circuito de interface de áudio SFG-05/MSX, que não deve ser transplantado automaticamente para o SMS. Existe esquema do SMS japonês traçado a partir das trilhas da placa, mas o próprio autor alerta que ele pode não ser 100% preciso. O core deve preservar MO e RO separadamente; a conversão para a fonte mono deve seguir a rede de soma/ganho da revisão SMS escolhida, após confirmação por esquema confiável ou medição. O controle de mute/roteamento de $F2 não deve parar os geradores internos sem evidência física.

**Decisão arquitetural M1:** o YM2413 recebe os mesmos avanços de clock do domínio PSG (ID 1), mas mantém estado e pipeline próprios. O adapter não cria um relógio paralelo nem um novo domínio no Scheduler. O barramento SMS passa a possuir o adapter e o estado de portas FM. A sessão de áudio passa a misturar fontes alinhadas antes do resampling PCM final.

**Bloqueios para implementação:**
- confirmar o circuito MO/RO e as revisões/modelos SMS japoneses alvo;
- definir a taxa comum de mistura e o ponto único de conversão, preservando latência/fase entre PSG e FM;
- especificar o alinhamento temporal entre o ciclo de escrita $F0/$F1/$F2, a amostra YM2413 e o avanço compartilhado do domínio 1;
- projetar os vetores de $F2 (mute/enable e contador C-Sync) e a compatibilidade de perfil antes de habilitar FM por região/modelo.

Fontes de código auditadas:
- `native/sms_core/src/temporal.rs`: IDs 0/1/2 e restore com três domínios.
- `native/sms_core/src/io.rs`: I/O atual e ausência explícita de FM.
- `native/sms_core/src/temporal_bus.rs`: decode de portas, estado do barramento e avanço do PSG.
- `native/audio_runtime/src/dsp.rs`: mixer multi-source.
- `native/audio_runtime/src/session.rs`: sessão que atualmente passa uma única fonte ao mixer.
- `native/audio_runtime/src/lib.rs`: entrada fixa de 44.100 Hz do resampler PCM.
- `native/sn76489/src/psg.rs`: geração/downsampling interno do SN76489 para a taxa configurada.

**Status:** auditoria de integração PASS (leitura estática dos fontes no SHA acima); implementação FM, validação elétrica da mistura MO/RO e testes de integração permanecem NOT RUN.


### 15. Errata M2 — reconciliação com o main de 2026-11-17

A seção 14.10 permanece preservada como snapshot histórico do SHA `c65594cf840090bad6036810a0df98f22b9d1484`. Ela **não deve ser usada isoladamente como decisão de implementação**. A auditoria atual, baseada no main `b82e0df4c9840b2cd59f090278b907cbb321869f`, está no documento independente [auditoria M2](../audits/ym2413-pre-implementation-audit-2026-11-17.md) e corrige os pontos abaixo:

1. **Clock compartilhado: decisão suspensa.** O worker SMS ativo usa `SMS_TEMPORAL_MASTER_TICKS_PER_TSTATE = 1`; o tick lógico é um T-state Z80, não um tick de 10,738635 MHz. O domínio PSG tem razão 1/1 nessa linha do tempo. O manual Yamaha especifica um oscilador de 3,579545 MHz em XIN/XOUT; MAME modela o YM2413 japonês com XTAL(10,738,635)/3. Isso corrobora frequência nominal, mas não prova fonte elétrica/fase comum em todas as placas. A decisão de consumir o mesmo `ClockAdvance` do PSG só será fechada após FM-A01.

2. **Taxa do resampler: correção.** O caminho ativo não está fixado em 44.100 Hz. `DEFAULT_PCM_SOURCE_RATE_HZ` é 48.000 Hz e `PcmResampler::new` recebe taxas explícitas de entrada e saída. O worker SMS usa 48 kHz como taxa PCM interna; a saída host é negociada separadamente.

3. **Mixer e sessão: estado atual.** `AudioMixer` suporta múltiplas `MixSource` alinhadas, mas `AudioSession`/`NativeAudioSession` continuam expondo uma única fonte mono por chamada. O FM exige uma API multi-source na sessão ou uma abstração equivalente; duas chamadas separadas não misturam PSG e FM.

4. **Portas japonesas: ausentes no runtime.** $F0/$F1/$F2 não são implementados em `SmsTemporalBus`/`SmsIo`. No caminho atual, $F0/$F1 são ignorados pelo I/O genérico; a leitura de $F2 cai em `SmsIo::read_port` e é aliasada para `read_dc()` (entrada dos controles) pelo intervalo de portas pares $C0–$FE. O valor pode ser $FF sem entradas ativas, mas não é open bus nem readback FM. O contador C-Sync necessário ao readback YM2413 também não foi localizado como evento no VDP/runtime.

5. **Decode de $3E/$3F: perfil específico necessário.** O caminho SMS atual aceita espelhamento de controle por paridade no intervalo baixo. As medições de Japanese SMS consultadas indicam mapeamento somente nos endereços exatos $3E/$3F. O perfil japonês precisa de decode próprio, sem alterar regressivamente SMS1/SMS2 export.

6. **Perfil de placa e compatibilidade: ausentes.** `Territory` e revisão de VDP não identificam por si sós a presença do YM2413. `DesktopSystem` não possui variante Japanese SMS integrada com FM; as preferências não possuem `FmMode` e não há catálogo por título/revisão/hash no runtime.

7. **Saída e taxa de amostragem.** A taxa aproximada de 49,716 kHz deriva de 72 clocks por rotação. A saída MO/RO é multiplexada e o valor de cada canal permanece ativo por uma janela curta antes de retornar ao neutro; o modelo deve preservar as duas saídas e não tratá-las como uma fonte mono já normalizada. A rede analógica da placa SMS continua pendente.

**Status atualizado:** a arquitetura geral proposta na seção 14 continua válida (core Rust, máquina SMS existente, Scheduler único, mixer existente). As decisões específicas de domínio de clock, conversão de taxa, decode japonês, C-Sync e mixagem MO/RO ficam condicionadas às evidências registradas na auditoria M2. Nenhum teste YM2413 foi executado.


### 15.1 O que bloqueia síntese e o que bloqueia integração

| Item | Síntese digital pura | Integração SMS/aceite físico | Estado |
|---|---|---|---|
| PG/EG/AM/PM/feedback, pipeline e estados iniciais | Pode avançar como modelo candidato isolado, com vetores e divergências explícitas | Exige resolução/diferencial suficiente para o nível de fidelidade declarado | PENDENTE |
| Presets do YM2413 original (não VRC7/YMF281B/FHB013) | Bloqueia aceite de presets originais; não impede scaffolding de operadores | Bloqueia catálogo/aceite de timbres originais | PENDENTE (FM-A08) |
| Latência de escrita 12/84 e comportamento fora da janela | Não bloqueia o algoritmo dos operadores; precisa ser modelada no contrato de registradores | Bloqueia aceite do barramento e transações FM | PENDENTE (FM-A06) |
| Fonte/fase do clock PSG/FM | Não bloqueia core que recebe clocks explícitos | Bloqueia escolha de domínio/razão no Scheduler e sincronismo físico | PENDENTE (FM-A01) |
| Rede analógica MO/RO | Não bloqueia geração digital separada das saídas | Bloqueia soma/ganho/filtragem específica do SMS | PENDENTE (FM-A02) |
| C-Sync e readback de $F2 | Não bloqueia síntese | Bloqueia readback fiel de $F2; não derivar de VCounter | BLOCKED (FM-A03) |
| Perfil físico/revisão da placa | Não bloqueia core genérico | Bloqueia seleção confiável da máquina e decode de portas | PENDENTE (FM-A04) |
| Catálogo título/revisão/hash | Não bloqueia core | Bloqueia comportamento Auto conservador | PENDENTE (FM-A05) |
| Alinhamento de mixer/resampler e continuidade | Não bloqueia core | Bloqueia áudio simultâneo, save state/replay de áudio e aceite do pipeline | PENDENTE (FM-A07) |

O contrato permite iniciar a síntese digital pura em incrementos pequenos, mas não declarar o modelo completo nem integrar FM ao SMS. Os bloqueios físicos ficam isolados no adaptador/mixer; as pendências digitais continuam visíveis no scorecard e exigem evidência antes de qualquer alegação de fidelidade.

### 15.2 Errata M2.1 — readback da metade baixa de $F2

Uma releitura da página SMS Power — Audio Control Port identificou uma correção necessária à formulação resumida anterior: no readback japonês, D7–D5 são o contador C-Sync, D4 é sempre zero e **D3–D0** devolvem o último nibble escrito. Na escrita, somente D1 (controle de mute do PSG) e D0 (controle de mute do YM2413) têm efeito de roteamento; D3–D2 não são controles de saída. O Mark III com FM Unit ainda força D1 a zero e mantém PSG/FM mutuamente exclusivos. Para sistemas sem FM, o readback é dependente do modelo: SMS não japonês retorna D1–D0 = %10 segundo a documentação, enquanto Mark III sem FM Unit retorna a entrada do port 0.

**Classificação:** DOCUMENTADO pela página SMS Power — Audio Control Port, que referencia as observações de Charles MacDonald; ainda PENDENTE de teste físico do Japanese Master System MK-2000 e das revisões cobertas. A auditoria M2 independente permanece preservada; esta errata atualiza o modelo normativo e não transforma o readback em implementação.

## 16. Scorecard por subsistema

**Snapshot de validação:** PR #996 head `b3a78474289994ec4328db47091d5845a1c6fd69`; CI [44213606017](https://github.com/leonardo201800478/sega-emulator/actions/runs/44213606017) ainda estava em andamento no último inventário. SMS runtime [44213606027](https://github.com/leonardo201800478/sega-emulator/actions/runs/44213606027) concluiu o job Release de formato/Clippy/testes com PASS. O próximo commit documental criará outro SHA; nenhum check deste snapshot valida esse novo HEAD.

| Subsistema | Estado | Evidência e limite |
|---|---|---|
| Auditoria M2 independente | PASS | PR #994 integrada por `faecfc7fc6a869884bf5aaad063473ca5002be0d`; auditoria estática, sem teste físico |
| Contrato dos quatro modelos físicos | PASS | Matriz documental da seção 7.3.1; não representa validação de todas as revisões de placa |
| Isolamento do crate `sega-ym2413` | PASS | `native/ym2413` sem dependências; dependência unidirecional `sega-audio-runtime → sega-ym2413`, no SHA `b3a7847...` |
| Latch/endereço e imagem de registradores | PASS | Testes de máscara de seis bits e escrita no registrador latched, no SHA `b3a7847...` |
| Contagem 18 × 4 = 72 clocks e invariância em blocos | PASS | Testes aritméticos do fundamento; não validam pipeline físico nem ordem de operadores |
| Snapshot/restore do fundamento | PASS | Round-trip de registradores e fase parcial; não é save state completo do chip/máquina |
| Alocação/locks no fundamento | PASS | Métodos atuais usam array fixo e não contêm heap/locks; o hot path de síntese ainda não existe |
| CI do PR #996 — jobs concluídos até o snapshot | PASS | Change impact, workspace regression, docs hygiene, PowerShell e SMS runtime Release fmt/Clippy/tests concluíram PASS no SHA indicado |
| CI do PR #996 — jobs ainda em execução | NOT RUN | Slint/WGPU/Vulkan, CPAL e SDL3 ainda estavam em andamento; reconsultar no SHA exato |
| CI do novo HEAD após atualização documental | NOT RUN | Exige nova execução no SHA criado por esta revisão |
| Síntese PG/EG/AM/PM/feedback/Log-Sine/EXP/Rhythm | NOT RUN | Nenhum operador ou PCM implementado |
| Latência 12/84 e writes no deadline SMS | NOT RUN | Fundação atual aplica writes imediatamente; adaptador ainda não existe |
| Clock/fase compartilhada PSG/FM | BLOCKED | FM-A01; frequência nominal não prova fase/fonte elétrica |
| C-Sync e readback $F2 | BLOCKED | FM-A03; runtime não expõe C-Sync |
| Circuito analógico MO/RO do SMS | BLOCKED | FM-A02; preservar saídas separadas |
| Mixer multi-source na sessão e conversão FM→48 kHz | BLOCKED | Mixer suporta fontes alinhadas, mas sessão aceita uma fonte por chamada; 49,716 kHz exige alinhamento determinístico |
| Auto/On/Off e catálogo por título/revisão/hash | NOT RUN | Não existe preferência/catálogo no runtime |
| Save state/replay integrado | NOT RUN | Só há snapshot do fundamento puro |
| Testes físicos de placa e saída | NOT RUN | Nenhuma medição física executada nesta trilha |

**SKIPPED no CI até o snapshot (não são PASS):** SMS1 VDP, input core, geração manual de lockfiles, workflow validation, Scheduler stress, SMS frame-pacing ROM build, Z80 focused validation, PSG core e SMS debugger Windows Release validation.

## 17. Implementação candidata PG-1 — 2026-11-18

A branch `feat/ym2413-phase-generator-candidate` adiciona `native/ym2413/src/phase.rs`, um gerador de fase isolado por operador. Esta fatia implementa o modelo matemático M1 publicado por andete, sem importar código ou tabelas de terceiros.

### Escopo implementado

- Acumulador de fase de 19 bits em ponto fixo 10.9; os 10 bits superiores são expostos como índice da tabela de seno.
- Tabela MUL → ML com 16 entradas.
- Incremento base: `((FNUM * ML[MUL]) << BLOCK) >> 1`.
- PM opcional com a tabela medida 8×8 e a expressão `(((2 * FNUM + PM) * ML[MUL]) << BLOCK) >> 2`.
- Wrap modular de 19 bits.
- Snapshot/restore versionado por acumulador.
- Validação dos limites FNUM (9 bits), BLOCK (3 bits), MUL (4 bits) e posição PM (0..7).

### Fronteiras deliberadas

O PG não possui contador global de amostras nem avança o LFO. O chamador deve fornecer a posição PM correspondente ao instante exato da atualização do operador. Essa decisão evita fixar prematuramente o alinhamento de reset, a cadência de 1024 amostras, a ordem PG/EG/OP e os offsets dos 18 slots. O PG também não decodifica registradores, não produz seno/PCM e não escolhe quando cada operador é visitado.

### Vetor diferencial incluído

O exemplo publicado por andete para FNUM=`0x1c0`, BLOCK=6 e ML=2 produz os oito incrementos:
`28672, 28768, 28896, 28768, 28672, 28576, 28448, 28576`.
O teste `pm_sequence_matches_published_ym2413_measurement_vector` reproduz essa sequência. É um vetor publicado de engenharia reversa, não uma medição independente executada pelo projeto.

### Estado de validação

- Código e testes adicionados: implementação candidata.
- CI no SHA final `224f2c3b215f9e6accf613d385e90ca7db931cc5`: SUCCESS (run #44223435767); formatação, Clippy, testes do workspace e higiene documental concluídos.
- Comparação independente contra hardware YM2413 original: NOT RUN.
- Alinhamento PM/reset/slot e pipeline cycle-accurate: PENDENTE.
- Integração de PG ao core de 18 slots, EG, operador, DAC e saídas MO/RO: NOT RUN.
- Não declarar síntese PCM, fidelidade cycle-accurate ou integração SMS concluída.

Fonte do modelo e vetor: [andete — YM2413 Reverse Engineering Notes 2015-12-01](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01). A tabela e as fórmulas foram transcritas a partir da especificação documental do projeto; nenhum código da implementação C emu2413 ou da implementação GPL Nuked-OPLL foi incorporado.


## 18. Snapshot de validação após merge da fundação e PG-1 — 2026-11-18

**Base estável:** `main` em `55206fd9ae87bb3fc76608330d6d7db77f3c8985`, merge da PR #996.

| Subsistema / gate | Estado | Evidência e limite |
|---|---|---|
| Fundação YM2413 — latch, registradores, 18×4 clocks, snapshot | PASS | PR #996 integrada em main; CI pós-merge #8569 e SMS Runtime #1533 concluíram SUCCESS no SHA da main |
| Isolamento do crate e dependência unidirecional | PASS | `sega-audio-runtime → sega-ym2413`; crate sem dependências externas |
| PG candidato — acumulador 19-bit, MUL/ML e PM 8×8 | IMPLEMENTED / CI PASS | PR #997; CI #44223435767 SUCCESS no HEAD `224f2c3b215f9e6accf613d385e90ca7db931cc5` |
| Vetor PM publicado de oito incrementos | PASS (reprodução do vetor publicado) | Teste reproduz o exemplo de andete; não equivale a validação física independente |
| PG contra hardware YM2413 original | NOT RUN | Sem captura física ou oráculo independente do projeto |
| Alinhamento do LFO PM, reset e offsets de slot | PENDENTE | Chamador fornece posição PM; contador global e pipeline ainda não foram implementados |
| EG, AM, feedback, Log-Sine/EXP, operadores, instrumentos e Rhythm | NOT RUN | Não fazem parte da PR #997 |
| Scheduler/clock/fase PSG-FM | BLOCKED | FM-A01 permanece pendente de evidência física |
| Portas SMS $F0/$F1/$F2 e C-Sync | NOT RUN / BLOCKED | Adaptador SMS ausente; FM-A03 depende de C-Sync real |
| MO/RO e circuito analógico SMS | BLOCKED | FM-A02; preservar saídas separadas |
| Mixer multi-source e alinhamento FM→48 kHz | BLOCKED | FM-A07; não integrar PCM FM à sessão atual |
| Perfil físico, catálogo Auto e save state integrado | NOT RUN | Fora do escopo desta fatia |

**CI da PR #997:** execução #44223435767 concluiu SUCCESS no HEAD `224f2c3b215f9e6accf613d385e90ca7db931cc5`. Jobs executados: classificação de impacto, regressão do workspace Rust (format, Clippy, testes e observability runtime), higiene documental e gate agregado de qualidade. Jobs específicos de VDP, PSG, Z80, Scheduler, Slint/WGPU, CPAL, SDL3, ROM build e tooling Windows foram SKIPPED por não serem aplicáveis aos caminhos alterados; SKIPPED não é PASS. A comparação da PR indica 11 commits ahead, 0 behind, sem conflito de merge no snapshot consultado. Reviews e review threads: nenhuma.

**Política:** estados IMPLEMENTED descrevem código presente, não fidelidade comprovada. Vetor publicado, CI, teste unitário e teste físico são evidências distintas. SKIPPED, pending, cancelled, ausente e NOT RUN nunca são PASS.


## 19. Contador PM LFO candidato — próximo incremento

A implementação PG-1 foi integrada à `main` pela PR #997, commit `a6adda38cc7db9317e93ca3c072f980fb5d239ec`. Esta fatia candidata isola o contador global de PM, sem acoplá-lo ao relógio do core ou ao Scheduler.

### Modelo documentado

As notas de andete de 2015-12-01 descrevem:
- contador global com 13 bits relevantes;
- avanço de uma posição PM a cada 1.024 amostras YM2413;
- seleção da posição por `counter >> 10`;
- período completo de 8.192 amostras;
- uma amostra por rotação dos 18 slots, isto é, 72 clocks de entrada.

A medição registra que o alinhamento inicial do contador global com o início da onda/saída não foi determinado. Portanto, a implementação exige fase inicial explícita e não atribui significado físico ao valor zero após reset.

### Contrato do candidato

- `PhaseModulationLfo::new(initial_counter)` valida e recebe explicitamente o contador inicial de 13 bits.
- `position()` expõe a posição PM selecionada pelos bits 12..10.
- `advance_samples(n)` avança por amostras completas já contabilizadas pelo chamador e aplica wrap módulo 8192.
- Snapshot/restore versionado preserva o contador exato.
- O módulo não avança clocks, não decide limites de rotação, não altera o PG e não presume reset/alinhamento de fase.
- O proprietário futuro deverá chamar `advance_samples` uma vez por rotação completa (72 clocks), e não uma vez por slot ou por T-state.

### Gates e limitações

- Testes de fronteira 1023/1024, sequência de oito posições, wrap 8192, fase inicial explícita, invariância por particionamento e snapshot/restore.
- Vetor/cadência fundamentados na medição publicada; não equivalem a comparação física independente.
- Integração ao `Ym2413Core`, relação entre reset e primeiro sample, alinhamento PM com PG/modulador/carrier e temporização do pipeline permanecem pendentes.
- AM LFO não faz parte desta fatia: AM tem cadência/tabela distinta (210 posições, atualização a cada 64 amostras) e deve ser tratada separadamente.
- Não conectar ao SMS, Scheduler, mixer ou áudio host.

Fonte: [andete — YM2413 Reverse Engineering Notes 2015-12-01](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01). A implementação é uma tradução independente da descrição matemática, sem copiar código de emuladores.


## 20. Auditoria de alinhamento temporal do PM LFO — 2026-11-18

### Evidências públicas reavaliadas

**MEDIDO — andete, 2015-12-01**
- A posição PM muda a cada 1.024 amostras YM2413 e a sequência repete a cada 8.192 amostras.
- A amostragem do experimento corresponde a uma amostra por período de 72 clocks.
- Ao alinhar uma senoide de referência à captura real, somente trechos de aproximadamente 1.024 amostras coincidiram; o modelo reproduziu as regiões ao aplicar as correções PM.
- O autor declara que o contador global de 13 bits provavelmente não estava sincronizado ao início da senoide; na simulação, ele estava sincronizado. A comparação completa permaneceu aproximada, também por a captura conter modulator + carrier e a simulação usar um único operador.

**MEDIDO / HIPÓTESE — andete, 2015-12-03**
- Repetições da mesma nota produziram formas de onda visualmente idênticas após sincronização do experimento pelo evento de key-on.
- O autor sugere, sem confirmar, que os phase counters dos operadores poderiam ser zerados no início da fase ATTACK.
- A inspeção ampliada sugere que modulator e carrier podem estar separados por um sample; a própria nota apresenta isso como explicação possível, não como contrato fechado.
- O início do sinal ainda apresenta discrepância associada à transição DAMP/ATTACK; o autor não a considera resolvida.

### Decisões arquiteturais

1. **Não inicializar automaticamente o contador PM global em zero no reset IC.** As fontes consultadas não demonstram que o reset de hardware determine essa fase.
2. **Não reiniciar o PM LFO em KEY-ON.** A hipótese de reset dos phase counters individuais no começo de ATTACK não prova reset do contador global PM.
3. **Manter separados três estados:** contador global PM, acumulador PG de cada operador e sincronização modulator/carrier. Não inferir que compartilham reset ou borda de atualização.
4. **Preservar a API explícita de fase inicial** de `PhaseModulationLfo::new(initial_counter)`. Não substituir por zero default nem acoplar ao `PhaseGenerator::reset()`.
5. A cadência de 1.024/8.192 amostras é evidência MEDIDA publicada; o alinhamento absoluto e o offset em relação a reset, KEY-ON, slot e saída continuam **PENDENTES**.

### Experimento necessário para fechar o contrato

Uma validação física independente deve capturar a saída de um YM2413 original com clock e escritas controlados, preservando timestamps em clocks mestres:

- executar reset IC e aguardar período conhecido antes de configurar a nota;
- capturar pelo menos 16.384 amostras (duas voltas PM) a uma amostra por 72 clocks;
- repetir a captura após reset IC, após KEY-OFF/KEY-ON e após reprogramação sem reset;
- comparar capturas entre si para localizar se a fase global PM reinicia, continua ou depende de outro evento;
- executar variantes com PM desabilitado/habilitado e com modulator/carrier configurados para frequências iguais e diferentes, para separar fase PG de posição PM;
- registrar o deslocamento em amostras e, quando possível, em clocks dentro do período de 72 clocks; não arredondar a observação para fronteiras de sample;
- repetir as medições para excluir ruído e efeitos de captura.

Sem acesso à placa YM2413 e sem uma captura sincronizada, esse experimento não pode ser substituído por comparação com emuladores. A documentação pública consultada não fornece o offset absoluto nem a relação completa entre reset, PM, PG e slots.

### Estado após PR #998

- Contador PM global candidato de 13 bits: **IMPLEMENTED**; PR #998 integrada na `main`, commit `fc383defbe035e699f33b07e7a525cfdb5b7fa5a`.
- Cadência e decode 8 × 1.024 / 8.192: **MEDIDO (fonte externa)** e coberto por testes de unidade do candidato.
- Fase inicial após reset IC: **PENDENTE**.
- Comportamento do contador global em KEY-ON/KEY-OFF: **PENDENTE**.
- Offset temporal modulator/carrier: **PENDENTE**.
- Offset dentro do ciclo de 72 clocks e instante exato de atualização PM/PG: **PENDENTE**.
- Integração ao `Ym2413Core`/Scheduler: **BLOQUEADA** até definir o contrato de atualização sem inventar alinhamento.

**Fontes:** [andete — notas 2015-12-01](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-01), especialmente a discussão do contador global e das medições; [andete — notas 2015-12-03](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-03), especialmente as observações de sincronização de operadores, key-on e offset modulator/carrier. As hipóteses do autor foram mantidas como hipóteses, não como especificação.


## 21. Envelope Generator candidato — 2026-11-18

### Escopo implementado em PR #1001

O crate `sega-ym2413` passa a conter `native/ym2413/src/envelope.rs`, exportado pela raiz do crate. O componente é um modelo de EG por operador, dirigido pelo contador global fornecido pelo chamador.

- Nível de atenuação de 7 bits: 0..127, com 128 níveis e resolução medida de 0,375 dB.
- Estados DAMP, ATTACK, DECAY e SUSTAIN.
- Cálculo de RATE com KSR: `RATE = 4 * R + Rks`, saturado em 63.
- Cadência discreta de DECAY/RELEASE para rates 0..63, incluindo os grupos rápidos 52..63.
- Curva ATTACK por fórmulas inteiras `f_m(x) = x - (x >> m) - 1`, incluindo o modelo medido para rates 12:0..14:3.
- DAMP com base-rate 12. O limiar de saída, a coordenação entre operadores e o comportamento de AR=15 são parâmetros explícitos de candidatos, não constantes declaradas como hardware: a nota de 2015 descreve o fim de DAMP como próximo de -45 dB e ainda sujeito a investigação, enquanto o modelo de software de andete usa o comparador de quatro bits superiores (nível 124..127) e o atalho AR=15.
- SL comparado pela metade superior do nível (`level >> 3`), incluindo o comportamento SL=0.
- Release do carrier: RR para EG-TYPE sustentado; taxas fixas 5/7 selecionadas por SUS-ON/OFF para EG-TYPE percussivo.
- Snapshot local versionado (formato 2, incluindo a configuração candidata), validação de largura dos campos e testes determinísticos.
- Nível inicial explícito por `EnvelopeGenerator::with_level`; não há valor default de reset declarado porque as fontes consultadas não comprovam o nível EG após reset IC.

### Contrato temporal deliberado

`EnvelopeGenerator::tick(global_counter, key_on, is_carrier, parameters)` recebe o valor do contador global no limite de atualização do operador. O EG não incrementa contador, não escolhe fase inicial e não avança relógio.

A transição DAMP do modulator permanece uma questão em aberto. `DampTransitionConfig` exige que o chamador selecione explicitamente o limiar, a coordenação (`IndependentPerOperator` ou `CarrierDrivesBoth`) e o comportamento de AR=15 (`SkipAttackToDecay` ou `EnterAttack`). `synchronize_damp_transition(&carrier)` só atua no candidato `CarrierDrivesBoth`; não é chamado automaticamente. O reset dos acumuladores PG na fronteira correspondente continua sob responsabilidade do pipeline de operadores e não foi conectado nesta PR.

### Evidência e limites

- **MEDIDO — andete, 2015-03-20:** níveis de envelope, cadência de decay e tabelas de avanço.
- **DOCUMENTADO / MEDIDO — andete, 2015-12-24:** KSR, fórmula de RATE e níveis SL.
- **MEDIDO — andete, 2015-12-31:** transições DAMP/ATTACK/DECAY/SUSTAIN, EG-TYPE, SUS-ON/OFF e taxas fixas DP/RR'/RS.
- **MEDIDO / MODELO CANDIDATO — andete, 2017-01-26:** fórmulas e sequência de ataque para taxas lentas e rápidas. O próprio autor registra que o modelo ainda precisava de verificação adicional.

Este componente é uma tradução independente das relações matemáticas descritas nas fontes; não incorpora código-fonte ou tabelas de outro emulador.

**Não implementado nesta fatia:** integração dos 18 EGs ao scheduler de slots; ordenação exata modulator/carrier; alinhamento do contador global com reset/KEY-ON; pipeline PG/EG por slot; integração no snapshot agregado `Ym2413State`; síntese PCM; validação diferencial de PCM; integração SMS, mixer e PSG.

**Validação:** PR #1001. Os testes unitários do EG foram adicionados; o resultado do CI deve ser associado ao HEAD efetivo da PR, e não a um SHA anterior. Nenhum teste local de workspace é declarado.

**Fontes:** [andete — 2015-03-20](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20), [2015-12-24](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-24), [2015-12-31](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31), [2017-01-26](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2017-01-26).
