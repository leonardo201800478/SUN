# YM2413 — Dossiê de pesquisa, evidências e lacunas

**Projeto:** Sega Emulator — Emulação de Alta Precisão
**Escopo:** YM2413/OPLL integrado ao perfil físico Japanese Master System.
**Data de referência:** 2026-11-18
**Status:** documento de pesquisa vivo; não certifica o chip como cycle-accurate nem declara a síntese FM concluída.
**Implementação:** Rust puro no crate `native/ym2413`; integração SMS, Scheduler e PCM FM permanecem fora do escopo já validado.

> **Regra de leitura:** “documentado” não significa que todo detalhe interno esteja especificado; “medido” descreve o experimento publicado, não uma validação feita pelo Sega Emulator; “inferido” não deve virar comportamento normativo sem teste independente; “desconhecido” deve permanecer explícito no código e nos snapshots.

---

## 1. Objetivo e decisão arquitetural

Este dossiê consolida a evidência técnica necessária para concluir o YM2413 sem introduzir temporização especulativa no core ou contaminar o runtime SMS com hipóteses ainda não comprovadas.

Decisões:

1. O core YM2413 permanece independente de console, GUI, CPAL, WGPU e SDL3.
2. O `MachineScheduler` continua sendo a única autoridade temporal da máquina. O chip não cria scheduler paralelo.
3. A ordem de slots publicada pelo manual pode ser representada como mapa de propriedade. Isso **não** define, por si só, os offsets internos de PG, EG, operador, feedback e DAC.
4. Componentes isolados (PG, EG, LFO, par FM, KSL, decoder de instrumentos/canais e LFSR) não equivalem a um sintetizador integrado.
5. Nenhum seed físico, alinhamento de fase, cadência de LFSR ou latência interna deve ser inventado para tornar a saída audível.
6. O caminho SMS deve preservar a distinção entre chip digital, saídas MO/RO, circuito analógico da placa e conversão para o dispositivo de áudio.
7. Não copiar código ou tabelas de implementações GPL. Referências de código servem para comparação técnica, respeitando suas licenças.

## 2. Estado atual do repositório

### 2.1 Componentes existentes

A partir das entregas integradas até PR #1011:

| Componente | Estado no projeto | O que isso não prova |
|---|---|---|
| Core, latch e registradores | Implementados em `native/ym2413` | Decode elétrico completo de $F0–$F2 no SMS |
| Clock de entrada e rotação de 18 slots | Implementados; 4 input clocks por slot, rotação de 72 clocks | Pipeline interno PG/EG/OP com offsets físicos |
| Mapa serial de slots | Implementado em `slot.rs`, com atribuição melódica e de ritmo | Geração de vozes rítmicas |
| PG | Acumulador de fase de 19 bits implementado | Alinhamento temporal físico com EG e operadores |
| EG | DAMP/ATTACK/DECAY/SUSTAIN candidato, com estado/snapshot | Transições completas certificadas e interação exata KEY/SUS |
| Log-Sine / EXP e par modulator/carrier | Implementados como componentes candidatos | PCM diferencialmente validado contra hardware |
| Feedback | Histórico de dois samples no caminho candidato | Posição exata de captura/consumo dentro dos slots |
| KSL | Lookup BLOCK/FNUM | Integração temporal e escala física definitiva |
| AM / PM LFO | Componentes globais implementados | Fase absoluta, reset e alinhamento com o pipeline |
| Decoder de instrumentos/canais | User patch, 15 presets candidatos e resolução de parâmetros por canal | ROM física YM2413 certificada |
| Banco agregado | PG/EG por operador, feedback por canal, LFOs e snapshot | Execução automática da sequência física de slots |
| LFSR de ritmo | Recorrência Galois de 23 bits implementada como primitiva explicitamente clockada | Seed físico, clock-enable e síntese das cinco vozes |
| PCM FM / mixer SMS+FM | Não integrado | Qualquer compatibilidade de áudio final |
| Barramento SMS $F0/$F1/$F2 | Não integrado no runtime ativo | Readback C-Sync e roteamento/mute |
| Save state completo SMS+FM | Não integrado | Continuidade bit-exata entre execução e restore |

### 2.2 Entregas que devem permanecer rastreáveis

- PR #1008: ponte de parâmetros de patch/canal para entradas de síntese.
- PR #1009: banco agregado de estado digital dos operadores.
- PR #1010: tabela de atribuição dos 18 slots e descritores de ritmo.
- PR #1011: LFSR de ruído de ritmo de 23 bits, com seed explícito e sem cadência automática.

As PRs #1010 e #1011 adicionaram entregas reais. O scorecard deve refletir esses componentes, mas não pode marcar FM-5/Rhythm ou o pipeline FM como concluídos.

## 3. Hierarquia e classificação das evidências

### Níveis

| Classe | Significado | Uso permitido |
|---|---|---|
| **D — Documentado** | Manual Yamaha, documentação de hardware ou tabela publicada | Base de contratos funcionais, respeitando o escopo do documento |
| **M — Medido** | Captura/experimento em YM2413 físico, com método descrito | Vetores de referência e hipóteses diretamente limitadas ao experimento |
| **R — Reverse-engineered** | Inferência de lógica interna a partir de medições, die shot ou análise de sinais | Modelo candidato; registrar limitações e alternativas |
| **C — Corroborado** | Comportamento coincidente em implementações independentes ou emulação de referência | Evidência secundária, não substitui hardware |
| **I — Inferido** | Explicação plausível sem confirmação suficiente | Não normativo |
| **U — Desconhecido / pendente** | Evidência insuficiente ou contraditória | Não preencher por conveniência |

Uma observação obtida de um die shot derivado (por exemplo, FHB013) não é prova direta de todos os detalhes do YM2413 original. Uma implementação de referência pode estar correta em um aspecto e aproximada em outro. Resultados de emuladores não são medições elétricas.

---

## 4. Matriz de evidências do chip

### 4.1 Clock, slots e taxa de atualização

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| Oscilador nominal de 3,579545 MHz | D | Manual de aplicação Yamaha |
| Uma célula de operador FM reutilizada em 18 slots para nove canais | D | Manual, seção Channels and Slots |
| Ordem de atribuição dos 18 slots | D | Tabela II-3 do manual; transcrita em `native/ym2413/src/slot.rs` |
| 4 input clocks por slot e 72 clocks por rotação | D/R | 4 clocks/slot consta na transcrição técnica do manual; 18 × 4 = 72. A rotação também aparece nas notas de reverse engineering |
| 72 clocks equivalem a aproximadamente 49,716 k atualizações por segundo com entrada 3,579545 MHz | Derivado | Aritmética: 3.579.545 / 72. É taxa interna de atualização, não taxa do host |
| Fases internas de PG, EG, feedback, operador e DAC | U | A ordem de slot não revela offsets de subfase nem o instante de captura/consumo |
| Relação de fase elétrica com Z80/PSG na placa Japanese SMS | U/I | Clock nominal coincidente não prova fonte elétrica compartilhada ou fase inicial idêntica |

**Não inferir:** “um slot = quatro clocks” não basta para definir em qual dos quatro clocks são lidos registradores, atualizada a fase, atualizada a envoltória, calculado o operador ou atualizado o pino MO/RO.

### 4.2 Escritas e latência de registradores

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| Espera mínima de 12 ciclos após escrita de endereço | D | Yamaha Application Manual, tabela II-2 |
| Espera mínima de 84 ciclos após escrita de dados | D | Yamaha Application Manual, tabela II-2 |
| Dados podem ficar incertos se os intervalos não forem respeitados | D | Manual Yamaha |
| Latência interna de commit de cada registrador/slot | R/U | Notas de reverse engineering propõem deslocamento/aliasing e espera por rotação; não é contrato completo por endereço |
| Writes simultâneos a rotação de slots e sua ordem causal | U | Requer experimento dirigido e definição de timestamp de conclusão |

A implementação deve separar **restrição externa de acesso ao barramento** (12/84 ciclos do manual) de **latência interna até o valor alcançar o estágio consumidor**. Não substituir a segunda pela primeira.

### 4.3 PG, modulação e operadores

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| Síntese FM usa modulator e carrier em cada canal melódico | D | Manual Yamaha |
| A saída do modulator modula a fase do carrier | D/R | Arquitetura OPLL e notas de reverse engineering |
| Feedback existe no caminho do modulator | D/R | Manual e experimentos publicados |
| Largura exata e ponto de truncamento do sinal de modulação | U/R | Notas identificam perguntas abertas sobre fator, largura e truncamento |
| Fase PG inicial após reset e KEY-ON | U | Não fixar sem evidência específica |
| Latência modulator → carrier e o slot exato em que o valor é consumido | U | Não resolvido apenas pela ordem M/C do mapa |
| Histórico de feedback: momento de captura, atualização e deslocamento | U | Implementação de dois samples é candidata até comparação de sinais |

O mapa de slots mostra que os slots do modulator precedem os slots do carrier na sequência serial. Isso não determina automaticamente quantos ciclos separam o cálculo e a utilização do valor modulado.

### 4.4 Envelope Generator (EG)

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| O envelope tem fases relacionadas a attack, decay, sustain e release | D/M | Manual e capturas publicadas |
| AR, DR, RR, SL e EG-TYP influenciam o comportamento | M/R | Experimentos de Andete |
| KEY-ON/OFF e SUS alteram transições e seleção de taxas | M/R | Notas de ADSR publicadas |
| Re-key em timbre sustentado passa por DAMP antes de ATTACK | M | Capturas publicadas; comportamento YM2413 específico |
| A modelagem completa das transições ADSR é trivialmente equivalente a outros OPL | Não | Não assumir; as notas descrevem particularidades OPLL |
| Contador global compartilhado e sua fase absoluta no reset | R/U | Evidências de contagem compartilhada; alinhamento absoluto não certificado |
| Limiar DAMP exato, incluindo o candidato 124 | R/U | Manter parametrizado; não cristalizar como constante física sem vetor |
| AR=15 e fronteiras DAMP/ATTACK | U | Necessita vetores direcionados |
| Diferença de atualização entre modulator e carrier | U | Necessita capturas por operador e sequência de slots |

O EG existente é um componente candidato, não uma validação completa do comportamento ADSR do chip. O contador global deve permanecer explícito no contrato de atualização até que a origem, a cadência e o alinhamento sejam demonstrados.

### 4.5 LFO AM e PM

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| AM e vibrato/PM existem no OPLL | D |
| Há medições e modelos publicados para AM e vibrato | M/R |
| AM e PM usam relógios/contadores globais | R |
| Período e alinhamento exatos com reset, slots e saída | U |
| Momento de amostragem do valor LFO por operador | U |
| Reset de LFO em reset IC, KEY-ON ou troca de instrumento | U |

Os componentes AM/PM implementados devem continuar desacoplados do relógio do chip até que a fase absoluta e os pontos de amostragem estejam validados. “Avançar uma vez por amostra” é contrato de software atual, não prova da cadência elétrica.

### 4.6 Instrument ROM e decodificação

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| 15 instrumentos melódicos predefinidos e user patch | D |
| Registradores de user patch e campos de instrumento/canal | D |
| Valores candidatos dos 15 presets em fontes MIT | C/R |
| Dump exato da ROM física do YM2413 original, confirmado byte a byte | U |
| Equivalência de YM2413 com VRC7, YMF281B ou variantes FHB013 | Não demonstrada | Não misturar ROMs/decoders sem comparação específica |
| Comportamento completo dos campos KSR/KSL/EG-TYP em todos os presets | Parcial | Requer validação cruzada com medições e hardware |

Os presets atualmente presentes no projeto devem ser descritos como **candidatos com procedência/licença**, nunca como dump certificado da ROM interna.

### 4.7 Rhythm Mode e LFSR

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| Modo Rhythm oferece cinco vozes e reaproveita slots dos canais 7–9 | D | Manual Yamaha e SMS Power |
| Bass Drum usa os dois slots do canal 7 | D |
| Hi-Hat, Tom-Tom, Snare Drum e Top Cymbal usam slots individuais dos canais 8/9 | D |
| Noise generator usa LFSR de 23 bits, polinômio (23,9,8,1) | M/R | Sequências capturadas e análise de Berlekamp–Massey publicada por Andete |
| Período máximo não nulo de 2^23−1 | Derivado matematicamente | Propriedade do polinômio/recorrência; teste local cobre o período candidato |
| Máscara Galois usada no software atual | Implementada | `0x400181`; representação equivalente depende da convenção de shift/XOR |
| Seed de reset físico | U | A publicação demonstra que o estado não pode ser zero, mas não determina seed específico |
| Clock-enable/cadência do LFSR | U | O artigo não estabelece quantos clocks YM2413 entre avanços |
| Mapeamento de noise para os quatro instrumentos não-BD | Parcial/U | A relação exata de ruído e informação de frequência precisa de mais captura |
| Envelope, volume e fase de cada voz rítmica | Parcial/U | Exige vetor individual de MO/RO |
| Bass Drum como par FM e comportamento completo | Parcial | A alocação é conhecida; pipeline de voz ainda não implementado |

O `Ym2413RhythmNoiseLfsr` é uma primitiva determinística explicitamente clockada. Não deve receber seed default arbitrário nem avançar uma vez por input clock, slot ou sample sem uma evidência nova.

### 4.8 DAC, MO/RO e saída analógica

| Afirmação | Classe | Evidência / limite |
|---|---|---|
| DAC interno de 9 bits | D | Manual Yamaha |
| Pinos de saída MO e RO | D | Manual Yamaha |
| MO/RO são source followers e precisam de estágio externo | D | Manual Yamaha |
| Canais são serializados/multiplexados ao longo da rotação | M/R | Medições publicadas de captura em 72x |
| Janela ativa e duração exata de cada canal em MO/RO | M/R | Capturas e análises; confirmar por revisão e condições de teste |
| Nível neutro, polaridade, escala e quantização exata | Parcial/U | Não derivar apenas da resolução nominal do DAC |
| Relação de ganho e rede de soma MO/RO na placa Japanese SMS | U | Não transportar circuito de MSX/SFG-05 |
| Filtro, carga, clipping e resposta analógica do SMS | U | Exige esquema confiável ou medição da revisão física alvo |

O modelo digital deve preservar MO e RO como sinais distintos até a etapa de integração de placa. Somá-los com ganho arbitrário dentro do core apagaria uma fronteira de hardware ainda não documentada.

---

## 5. Integração específica do Japanese Master System

A implementação do chip não fecha o caminho do console. A auditoria do runtime aponta os seguintes pontos externos:

| ID | Lacuna | Critério de fechamento |
|---|---|---|
| SMS-FM-01 | Perfil explícito de placa com YM2413 integrado | Perfil de hardware separado de território e revisão de VDP |
| SMS-FM-02 | Decode de $F0 endereço, $F1 dados e $F2 controle/readback | Testes de portas apenas no perfil Japanese SMS aplicável |
| SMS-FM-03 | C-Sync usado no readback de $F2 | Evento/sinal originado pelo hardware/VDP e validado; não derivar da GUI |
| SMS-FM-04 | Roteamento e mute PSG/FM por $F2 | Testar os quatro estados; chips continuam clockando quando silenciados |
| SMS-FM-05 | Clock/fase da placa | Esquema confiável ou medição antes de compartilhar domínio temporal |
| SMS-FM-06 | Saídas MO/RO e circuito de soma | Perfil elétrico de placa; manter coeficientes desconhecidos enquanto não medidos |
| SMS-FM-07 | Mixer e taxa de amostragem | Alinhar fontes em base comum determinística, sem concatenar blocos |
| SMS-FM-08 | Save state/replay | Restaurar latch, clock, subfase, PG/EG/LFO, feedback, LFSR, saídas pendentes e estado dos conversores que afetam a sequência |

A frequência 3.579545 MHz do YM2413 e do PSG nominal NTSC não prova que compartilhem a mesma fase ou a mesma origem elétrica em todas as placas. A decisão de usar o domínio PSG no Scheduler deve ficar bloqueada até fechar SMS-FM-05.

### 5.1 Portas $F0–$F2

A auditoria pré-implementação do projeto documenta, com base nas medições publicadas do Japanese SMS:

- $F0: escrita do endereço YM2413.
- $F1: escrita de dados YM2413.
- $F2: controle de roteamento/mute e leitura do contador C-Sync.
- Os dois bits inferiores escritos em $F2 selecionam PSG/FM audíveis de forma independente.
- O readback de $F2 reúne bits do contador C-Sync e os bits de controle inferiores.

A tabela de bits e a semântica de cada revisão devem ser mantidas junto aos testes de barramento e ligadas à referência de medição de Charles MacDonald/SMS Power. O caminho de readback não pode ser implementado por aproximação baseada em scanline.

### 5.2 Mixer e taxas

A especificação de áudio anexada pelo projeto descreve principalmente o PSG do SMS e seu estágio analógico. Ela fornece taxas de PSG de aproximadamente 223,72 kHz NTSC e 221,68 kHz PAL, mas essas taxas **não são clocks do YM2413**.

Para o YM2413, 3.579.545 / 72 ≈ 49.716,04 Hz é uma taxa interna derivada do período de slots. Não é taxa de saída CPAL. O caminho FM precisa de conversão determinística para a base de mixagem do runtime; fase e histórico do conversor devem ser preservados por bloco e no restore, caso influenciem a sequência PCM.

A especificação de áudio também propõe ganho FM de 1,0×–1,25× e circuitos analógicos genéricos. Esses números não são medições do Japanese SMS e **não devem ser promovidos a constantes** do mixer/placa. Os valores de filtro, clipping e carga também exigem validação física.

---

## 6. Plano de pesquisa prioritário

A sequência abaixo minimiza implementação sobre hipóteses. Cada item produz evidência reutilizável antes de ampliar o runtime.

### P0 — Fechar o modelo de evidências e contratos (documentação)

- Atualizar scorecard e inventário de componentes.
- Anotar para cada comportamento: fonte, classe, método, condições de captura, limitações e testes existentes.
- Registrar divergências entre manual, reverse engineering e referências de software.
- Não iniciar novas vozes de ritmo antes de consolidar o pipeline melódico.

**Saída:** este dossiê + scorecard coerente.

### P1 — Pipeline de 18 slots (pesquisa antes de codificar)

1. Identificar o que muda em cada slot e em cada subfase dos quatro input clocks.
2. Determinar quando PG é lido/atualizado e como o valor chega ao operador.
3. Determinar a atualização EG e a posição do contador global.
4. Determinar quando o modulator escreve o resultado e em qual slot o carrier o consome.
5. Determinar quando o feedback é capturado e quantos estados anteriores são usados.
6. Determinar o ponto de atualização do DAC e os eventos de MO/RO.
7. Definir o efeito de uma escrita de registrador que chega antes, durante ou depois do slot consumidor.

**Saída:** tabela temporal por subfase, com estados anteriores/novos, referência de cada afirmação e casos ainda ambíguos. Não basta uma descrição narrativa.

### P2 — Vetores de EG e LFO

- Construir sequências de escrita/KEY-ON/OFF para timbres sustentados e percussivos.
- Cobrir DAMP, AR=15, DR, RR, SL, EG-TYP, SUS e KSR.
- Medir modulator e carrier separadamente.
- Capturar AM/PM com fase de partida conhecida, key-on e reset.
- Comparar níveis e fronteiras, não apenas a forma de onda audível.

**Saída:** vetores dourados com timestamp, registradores escritos e amostras MO/RO capturadas.

### P3 — Instrument ROM

- Identificar a fonte exata dos 15 patches candidatos e sua licença.
- Comparar patches contra captura física de cada preset, incluindo envelope e modulação.
- Documentar divergências YM2413/VRC7/YMF281B e variantes.
- Só promover a tabela a “ROM confirmada” após comparação byte a byte ou validação equivalente do decoder.

### P4 — Rhythm Mode

- Primeiro medir cada voz isoladamente em MO e RO.
- Fixar configurações de frequência e registradores recomendadas.
- Determinar cadência do LFSR por amostragem de ruído em múltiplos offsets temporais.
- Determinar combinação de ruído/frequência para HH, SD, TOM e CYM.
- Medir envelopes, volume, KEY-ON/OFF e latência de cada slot.
- Determinar estado inicial físico apenas se houver observação reproduzível; caso contrário, manter parâmetro explícito.

### P5 — DAC e placa SMS

- Capturar MO e RO simultaneamente com amostragem suficiente para observar a rotação de 72 clocks.
- Medir nível neutro, largura de pulso, polaridade, amplitude e repetição.
- Obter esquema da revisão de Japanese SMS alvo ou medir resistores/capacitores de soma e saída.
- Definir modelo digital e estágio de placa em módulos separados.
- Não adotar os valores de circuito MSX como valores de SMS.

### P6 — Integração temporal e áudio

Só após P1 e SMS-FM-05:
- Definir domínio de clock no Scheduler.
- Integrar timestamps de escrita no barramento e latência do chip.
- Criar fluxo PCM FM determinístico e conversor de taxa.
- Generalizar a sessão/mixer para múltiplas fontes alinhadas no mesmo intervalo.
- Preservar estado de conversão e saída pendente no snapshot.
- Testar equivalência contínuo vs. blocos, save/restore e replay.

---

## 6.1 Resultado da investigação P1 — pipeline de 18 slots

A investigação documental inicial foi detalhada em [YM2413 — Investigação P1 do pipeline de 18 slots](ym2413-slot-pipeline-investigation-2026-11-18.md).

Conclusões adicionais:

- O manual confirma a reutilização de uma célula de operador em 18 slots e sua ordem; medições independentes confirmam 72 clocks de entrada por rotação.
- Quatro clocks por slot é a divisão aritmética do período de 72 clocks pelos 18 slots e é compatível com o contrato atual do crate. A função interna de cada um dos quatro clocks **não está documentada**.
- O PG de 19 bits, com 9 bits fracionários e 10 bits de endereço da tabela SIN, possui suporte em medições de fase publicadas. A fórmula de incremento está documentada no relatório P1.
- O modelo de EG usa um contador global compartilhado e avalia taxas em unidades de rotação/amostra; a fase absoluta após reset e o offset interno no slot permanecem desconhecidos.
- O feedback com os dois outputs anteriores do modulator reproduz medições publicadas após sincronização; condições iniciais diferentes explicam discrepâncias no início.
- A captura publicada de MO/RO identifica 3 clocks ativos por canal em um quadro de 72 clocks, mas não fecha todos os detalhes de latch do DAC.

**Decisão P1:** nenhuma ordem de subfase PG/EG/OP, latência M→C ou ponto de commit de registrador foi promovido a contrato. A implementação do pipeline permanece bloqueada até obter vetores de captura com resolução por input clock ou evidência primária equivalente.

## 6.2 Decodificação estática de Rhythm Mode — FM-7

O crate agora expõe `decode_rhythm_registers(&[u8; 64])`, que produz uma configuração estática e sem efeitos colaterais:

- RHYTHM em `$0E.D5`;
- triggers BD/SD/TOM/TOP-CY/HH em `$0E.D4..D0`;
- volumes de quatro bits distribuídos em `$36`, `$37` e `$38`;
- FNUM/BLOCK/KEY-ON/SUS dos canais 7, 8 e 9, sem interpretar os campos `$36–$38` como instrumentos melódicos.

A configuração preserva separadamente o bit de trigger e a habilitação global. Uma voz é considerada ativa somente quando ambos estão ligados. Os KEY-ON melódicos dos canais 7–9 são expostos como dados brutos; o decoder não os limpa automaticamente, embora o manual oriente que `$26–$28` permaneçam com KEY-ON=0 em Rhythm Mode.

**Evidência primária:** Yamaha YM2413 Application Manual, III-1-7, III-1-8 e V-4. O manual descreve cinco vozes, os seis slots usados, os registradores de volume e a geração das fases de quatro instrumentos a partir de ruído branco combinado com as frequências dos canais 8 e 9. O Bass Drum utiliza dois slots para FM. O manual também informa que as saídas de percussão são emitidas duas vezes; o significado temporal exato dessa duplicação ainda não foi integrado ao modelo.

**Corroboração de reverse engineering:** as notas de andete de 2015-12-16 identificam os bits de RHY/BD/SD/TOM/CYM/HH em `$0E`, mas registram que o comportamento de ritmo ainda não havia sido investigado naquela etapa. A captura de 2018-05-13 demonstra a necessidade de amostrar os canais e saídas MO/RO em resolução 72 vezes maior para isolar as vozes rítmicas. Essas notas não substituem o manual nem fecham o pipeline.

**Limites preservados:** a entrega não implementa ROM de timbres rítmicos, EG por voz, phase-combining, ruído aplicado às quatro vozes não-BD, duplicação temporal da saída, cadência do LFSR, mixagem ou PCM. O decoder é apenas uma ponte de registradores e não altera o Scheduler ou o banco agregado.

Referências:
1. Yamaha YM2413 Application Manual: https://map.grauw.nl/resources/sound/yamaha_ym2413.pdf (III-1-7, III-1-8, V-4).
2. Andete, YM2413 Reverse Engineering Notes 2015-12-16: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-16.
3. Andete, YM2413 Reverse Engineering Notes 2018-05-13: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13.


## 7. Matriz mínima de validação

Todos os testes ainda não executados devem permanecer explicitamente como **NOT RUN**, **BLOCKED** ou **CANDIDATE**. A existência de testes unitários de componentes não significa validação do chip completo.

| ID | Teste | Estado atual | Critério de aprovação |
|---|---|---|---|
| T01 | Latch e mapa de registradores | Parcial | Vetores independentes de escrita/leitura e máscaras |
| T02 | Restrições de escrita 12/84 clocks | Não demonstrado no scorecard | Limites exatos, timestamp e comportamento abaixo do mínimo |
| T03 | Slot map 1–18 | Implementado/testado | Tabela do manual e wrap determinístico |
| T04 | Subfases dos quatro clocks por slot | NOT RUN | Vetor por subfase contra referência física |
| T05 | PG por operador | CANDIDATE | Frequência, wrap, reset e modulação diferencial |
| T06 | EG ADSR/DAMP | CANDIDATE | Vetores de envelope completos e fronteiras |
| T07 | Feedback e latência M→C | NOT RUN | Captura por slot/subfase |
| T08 | AM/PM | CANDIDATE | Fase e profundidade contra hardware |
| T09 | 15 presets | CANDIDATE | Comparação física por preset |
| T10 | Rhythm LFSR | Primitiva implementada | Recorrência conhecida; seed/cadência física continuam pendentes |
| T11 | Cinco vozes de ritmo | NOT RUN | Captura individual de MO/RO |
| T12 | DAC/MO/RO | NOT RUN | Sequência, níveis e timing medidos |
| T13 | Portas $F0–$F2 | BLOCKED | Decode, readback C-Sync e roteamento no perfil alvo |
| T14 | Clock/fase da placa | BLOCKED | Evidência elétrica suficiente |
| T15 | Mixer FM+PSG | NOT RUN | Mix simultâneo alinhado, headroom medido |
| T16 | Save state/replay FM | NOT RUN | Restore e replay bit-exatos no domínio digital |
| T17 | Regressão SMS e SG-1000 | NOT RUN | Gates relevantes PASS sem alterar semântica SG-1000 |
| T18 | Comparação física final | NOT RUN | Vetores de hardware reproduzidos dentro de tolerância definida |

### Regras de promoção de estado

- **CANDIDATE → IMPLEMENTED:** implementação e testes unitários determinísticos.
- **IMPLEMENTED → INTEGRATED:** conectado ao pipeline real sem clocks artificiais.
- **INTEGRATED → VALIDATED:** vetores independentes e regressões aplicáveis aprovados.
- **VALIDATED → HARDWARE-CORROBORATED:** comparação documentada com hardware real.
- Nunca usar “concluído” como sinônimo de “compila” ou “produz som”.

---

## 8. Fontes e referências

### Primárias / documentação de fabricante

1. Yamaha, *YM2413 Application Manual* — [espelho SMS Power](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual) e [PDF digitalizado](https://www.waveguide.se/?article=46&file=ym2413am.pdf). Fonte para recursos funcionais, registradores, canais/slots, tempos de escrita, DAC e pinos.
2. Transcrição técnica do manual — [YM2413AP](https://d4.princess.ne.jp/msx/datas/OPLL/YM2413AP.html). Consultar junto ao PDF; transcrição secundária.

### Reverse engineering com medições

3. [SMS Power — YM2413 reverse engineering index](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes). Índice cronológico das notas sobre PG, EG, tabelas, AM/PM, operadores, feedback, ADSR e ritmo.
4. [Decay/release measurements](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20). Medições de envelope; não generalizar além dos parâmetros e condições do experimento.
5. [Modulator/carrier and FM](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-03). Arquitetura de operadores e questões sobre sinal de modulação.
6. [ADSR transitions](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31). Interação KEY, SUS, EG-TYP, DAMP e taxas.
7. [Rhythm/noise/LFSR](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13). Evidência experimental do LFSR e limites declarados pelo autor sobre como o ruído é aplicado às vozes.
8. [SMS Power — YM2413 overview and rhythm allocation](https://www.smspower.org/Development/YM2413). Configuração das vozes rítmicas e referências de reverse engineering.
9. [MSX Resource Center — YM2413 reverse engineering progress](https://www.msx.org/photoshoots/ym2413-reverse-engineering-progress). Observações de saída e circuito MSX; não usar seus valores analógicos como valores da placa SMS.

### Integração Japanese SMS

10. [SMS Power — Audio Control Port](https://www.smspower.org/Development/AudioControlPort). Referência para $F2, roteamento e controle de áudio.
11. [SMS Power — Japanese SMS / Mark III schematics and measurements](https://www.smspower.org/forums/15878-SG1000IISEGAMARKIIIAndJAPSMSSchematic). Evidência secundária de placa; considerar as ressalvas do autor sobre o esquema.
12. [MAME SMS driver](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp) e [SMS machine](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms_m.cpp). Corroboradores de configuração e mapeamento, não autoridade elétrica.

### Referências de implementação — não normativas

13. [emu2413 (MIT)](https://github.com/digital-sound-antiques/emu2413). Comparação diferencial; não substituir a validação física.
14. [ymfm (BSD-3-Clause)](https://github.com/aaronsgiles/ymfm). Referência alternativa; não assumir precisão digital integral.
15. [Nuked-OPLL (GPL-2.0)](https://github.com/nukeykt/Nuked-OPLL). Referência para investigação de temporização; não copiar código/tabelas para este projeto.
16. [emu198x-yamaha-ym2413 documentation](https://docs.rs/emu198x-yamaha-ym2413/latest/emu198x_yamaha_ym2413/). Referência de arquitetura de pipeline/clock; não é medição primária nem dependência aprovada.

---

## 9. Materiais do projeto que não devem ser confundidos com especificação YM2413

O arquivo anexado `Especificação Técnica de Áudio do Master System (PSG e DSP em Rust).md` trata majoritariamente do PSG integrado ao VDP e do estágio analógico genérico do SMS. Ele é útil para a arquitetura de áudio compartilhada, mas:

- as taxas de tick de 223,72 kHz NTSC e 221,68 kHz PAL são do PSG, não do YM2413;
- o ganho FM de 1,0×–1,25× é uma proposta sem medição específica da placa japonesa;
- filtros, clipping, carga e componentes sugeridos não substituem o circuito real da revisão de SMS alvo;
- o documento não fecha o pipeline interno FM, a cadência dos slots, o LFSR ou o DAC MO/RO.

Os guias gerais de hardware SMS/SG-1000 e o comparativo de VDP do projeto também não fornecem vetores de áudio FM. Eles permanecem referências para outras partes do console, sem serem usados como prova do YM2413.

---

## 10. Critério de encerramento do dossiê

Este dossiê será considerado suficientemente fechado para iniciar a integração funcional do pipeline somente quando:

1. A tabela temporal por slot/subfase estiver documentada com proveniência e ambiguidades explícitas.
2. Os contratos de PG, EG, feedback e M→C tiverem vetores independentes.
3. Os limites de escrita do manual estiverem cobertos por testes de timestamp.
4. Os presets e o Rhythm Mode tiverem plano de captura física reproduzível.
5. O contrato MO/RO estiver separado da rede analógica SMS.
6. A decisão de clock domain da placa estiver sustentada por evidência, ou permanecer explicitamente isolada como domínio independente no mesmo Scheduler.

**Conclusão atual:** o projeto tem uma fundação real de componentes YM2413, incluindo slot map e primitiva LFSR, mas ainda não tem pipeline FM cycle-accurate, PCM FM, Rhythm Mode sintetizado nem integração SMS. A próxima tarefa técnica prioritária é P1 — pesquisa e especificação temporal do pipeline de 18 slots — e não ampliar a GUI, o mixer ou o Scheduler por suposição.
