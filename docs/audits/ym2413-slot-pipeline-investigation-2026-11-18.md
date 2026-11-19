# YM2413 — Investigação P1 do pipeline de 18 slots

**Data:** 2026-11-18
**Estado:** investigação documental inicial concluída; offsets internos por subfase ainda não demonstrados.
**Escopo:** estabelecer um contrato temporal auditável antes de implementar o pipeline FM em Rust.
**Dependências:** PR #1010 (mapa serial de slots), PR #1011 (LFSR isolado), dossiê `ym2413-research-evidence-dossier-2026-11-18.md`.

> **Conclusão central:** a evidência pública confirma a reutilização serial de uma célula de operador por 18 slots e um período de 72 clocks de entrada por rotação. Ela não fornece uma tabela primária completa dos offsets de PG, EG, operador, feedback e DAC dentro dos quatro clocks associados a cada slot. Não vamos preencher essa lacuna por analogia com OPL ou copiando uma implementação GPL.

---

## 1. Unidades temporais e fatos estabelecidos

| Unidade | Valor | Evidência | Interpretação permitida |
|---|---:|---|---|
| Clock de entrada | 3.579.545 Hz nominal | Manual Yamaha | Oscilador nominal do YM2413 |
| Operadores/slots por rotação | 18 | Manual, seção II-5 / Tabela II-3 | Uma célula de operador é reutilizada sequencialmente |
| Clocks de entrada por rotação | 72 | Captura de saída multiplexada publicada | Uma rotação completa leva 72 clocks de entrada |
| Clocks de entrada por slot | 4 | Derivado de 72 / 18; compatível com o contrato atual do crate | Quatro clocks formam o orçamento temporal de cada slot; a função individual de cada subfase não está demonstrada |
| Taxa de rotação/amostra por canal | 3.579.545 / 72 ≈ 49.715,9027778 Hz | Derivação aritmética das medições | Taxa de repetição do quadro serial, não taxa CPAL |
| Espera após escrita de endereço | 12 clocks master | Manual, Tabela II-2 | Restrição externa entre endereço e dado |
| Espera após escrita de dados | 84 clocks master | Manual, Tabela II-2 | Restrição externa antes de outra escrita |

A medição publicada descreve os pinos MO/RO como sinais time-multiplexed, com 72 clocks para percorrer todos os canais. Cada saída de canal permanece ativa por três clocks antes de retornar ao nível neutro. Isso caracteriza o estágio de saída; não determina, por si só, o clock exato em que PG, EG e OP atualizam os seus estados.

## 2. Ordem serial de slots confirmada

A Tabela II-3 do manual e o mapa implementado em `native/ym2413/src/slot.rs` concordam na seguinte ordem. Os nomes de canal abaixo são apresentados em base 1, conforme o manual.

| Slot | Canal | Operador | Rhythm Mode |
|---:|---:|---|---|
| 1 | 1 | Modulator | — |
| 2 | 2 | Modulator | — |
| 3 | 3 | Modulator | — |
| 4 | 1 | Carrier | — |
| 5 | 2 | Carrier | — |
| 6 | 3 | Carrier | — |
| 7 | 4 | Modulator | — |
| 8 | 5 | Modulator | — |
| 9 | 6 | Modulator | — |
| 10 | 4 | Carrier | — |
| 11 | 5 | Carrier | — |
| 12 | 6 | Carrier | — |
| 13 | 7 | Modulator | Bass Drum |
| 14 | 8 | Modulator | Hi-Hat |
| 15 | 9 | Modulator | Tom-Tom |
| 16 | 7 | Carrier | Bass Drum |
| 17 | 8 | Carrier | Snare Drum |
| 18 | 9 | Carrier | Top Cymbal |

O manual confirma que o primeiro slot de cada par melódico atua como modulator e o segundo como carrier. Em Rhythm Mode, os slots 13–18 são reatribuídos conforme a tabela. Isso é **ordem de propriedade**, não uma descrição de todas as transferências de dados entre os estágios internos.

## 3. O que sabemos sobre cada bloco temporal

### 3.1 Phase Generator (PG)

**Evidência experimental publicada:**

- O phase counter tem 19 bits.
- Os 10 bits superiores endereçam a posição na tabela de seno; os 9 bits inferiores representam a fração.
- A fórmula publicada para o incremento candidato é `step-size = ((fnum * mlTab[ML]) << block) >> 1`, com `mlTab` documentada no artigo.
- A análise do die shot FHB013 identificou 18 cadeias de 19 flip-flops, interpretadas como um acumulador por operador.

**Limite:** a análise de die shot é de um derivado FHB013, não de uma fotografia do YM2413 original. A medição da fase sustenta a resolução e o incremento, mas não fornece o offset de clock dentro do slot em que o PG lê parâmetros, incrementa o acumulador ou entrega os bits à tabela SIN.

**Consequência para o código:** preservar estado PG independente por operador; não assumir que o acumulador avança no mesmo subclock em que o operador consome a fase.

### 3.2 Envelope Generator (EG)

**Evidência experimental publicada:**

- Há 18 estados/contadores de envelope, um por operador; a análise do die shot derivado aponta 18 × 7 bits.
- O reverse engineering descreve um contador global compartilhado pelos 18 operadores, incrementado por amostra de saída.
- O contador global participa da cadência de transição dos envelopes; isso explica por que a primeira transição após uma mudança de fase pode ser mais curta.
- Os estados ADSR dependem de AR/DR/RR/SL/EG-TYP, KEY-ON/OFF e SUS.

**Limite:** o modelo publicado descreve a progressão em unidades de amostra/rotação. Isso não identifica o subclock físico em que o EG é atualizado relativamente ao PG e ao operador, nem fecha o estado inicial absoluto do contador global após reset.

**Consequência para o código:** distinguir o tick lógico do contador global (unidade de rotação) do subclock de processamento de slot. Não incrementar EG 18 vezes por rotação nem uma vez por input clock sem evidência.

### 3.3 Operador SIN / FM

**Evidência documental e experimental:**

- A célula de operador é reutilizada serialmente para os 18 operadores.
- O modulator alimenta a fase do carrier; o diagrama funcional é descrito no manual.
- O cálculo é discreto; o reverse engineering confirma que feedback não é instantâneo.
- As medições de feedback foram reproduzidas pelo modelo com a soma escalada dos dois últimos outputs do modulator.
- A nota de 2015-12-03 reproduz as capturas modulator/carrier usando a fase do modulator de uma amostra de saída anterior: no modelo, `lookupSin((i - 1) / 16)` alimenta o modulator enquanto o carrier usa `lookupSin(i / 16 + ...)`.
- Para o vetor publicado FNUM=0x40, BLOCK=0, MUL=0, o incremento PG é 32 unidades no acumulador 10.9, isto é, 1/16 de uma entrada da tabela por amostra. Nesse caso, um atraso de uma amostra não equivale a subtrair uma entrada inteira da tabela.

**Limite:** o atraso de uma amostra é um modelo que reproduziu as capturas de saída, não uma medição direta dos latches internos. As condições iniciais dos dois outputs de feedback podem divergir e alterar bastante os primeiros outputs, especialmente em feedback alto. A posição exata de captura/consumo dentro dos slots e a conversão desse atraso em input clocks permanecem desconhecidas.

**Consequência para o código:** manter a cadeia de feedback e o índice de fase do modulator da amostra anterior no estado serializável do par. O primeiro valor histórico deve ser explícito ou permanecer desconhecido; não substituir um atraso de amostra por um deslocamento fixo de uma entrada da tabela. Isso ainda não autoriza afirmar em qual subfase o modulator alimenta o carrier.

### 3.4 LFO global

As notas de reverse engineering relacionam AM e PM a uma base de amostras/rotação (~49,7 kHz). O PM avança a posição de tabela a cada 1024 amostras e completa seu padrão de oito posições em 8192 amostras; AM tem sua própria sequência e duração.

**Limite:** a cadência média é conhecida pelo modelo medido, mas a fase inicial do contador global, o reset e o alinhamento com os slots/registradores continuam exigindo vetores dedicados.

### 3.5 DAC e pinos MO/RO

**Evidência:**

- DAC de 9 bits e saídas MO/RO constam no manual.
- Capturas publicadas mostram multiplexação de canais ao longo de 72 clocks.
- O valor de cada canal permanece ativo por três clocks e depois retorna ao nível neutro.
- O manual descreve MO/RO como source followers, exigindo circuito externo de integração/amplificação.

**Limite:** isso não determina todos os detalhes de seleção MO/RO por subfase, o valor elétrico do nível neutro, a resposta da placa Japanese SMS ou o instante exato de latch do DAC para cada slot.

## 4. Matriz temporal conservadora

A tabela abaixo é o máximo que a evidência permite afirmar hoje sem promover hipóteses a fatos.

| Intervalo | Evento conhecido | Estado que deve ser associado | Evento interno ainda não estabelecido |
|---|---|---|---|
| Entrada de clock 0 do slot | Slot corrente é selecionado no orçamento temporal de quatro clocks | Índice de slot/descritor | Commit de registrador, leitura de PG/EG ou latch |
| Entrada de clock 1 do slot | Parte do intervalo do slot | Estado do operador designado | Subfase de PG, EG, SIN ou OP |
| Entrada de clock 2 do slot | Parte do intervalo do slot | Estado do operador designado | Transferência M→C, feedback ou DAC |
| Entrada de clock 3 do slot | Fecha o intervalo de quatro clocks | Próximo slot/rotação | Momento exato de avanço do contador global |
| Fim do slot 18 / rotação de 72 clocks | Quadro serial completo | Estado global por rotação | Ordem exata de atualização de EG/LFO, latch de saída e efeitos de escrita concorrente |

**Importante:** os rótulos 0–3 são índices de contagem para instrumentação futura. Não significam que o silício tenha subfases funcionais correspondentes a essas quatro operações. A tabela não autoriza escrever um dispatcher com quatro ações especulativas.

## 5. Contrato de software que pode ser implementado sem especulação

1. Contador monotônico de input clocks, com fase preservada no snapshot.
2. Slot atual derivado deterministicamente do contador e da fase inicial explicitamente definida pelo contrato do chip.
3. Descritor de slot obtido exclusivamente de `YM2413_SLOT_MAP`.
4. Estado PG/EG por operador e feedback por canal, todos versionados.
5. Tick global por rotação separado do processamento do operador.
6. Interface de escrita de registrador com timestamp em input clocks e validação das esperas 12/84 do manual.
7. Saídas MO e RO mantidas separadas; não somar ou aplicar ganho de placa dentro do core.
8. Snapshots devem incluir contador de input clocks, índice/fase do slot, contador global, PG, EG, feedback, LFO, registradores e qualquer latch de saída já comprovado.

Este contrato é deliberadamente parcial. Não implementar ainda a atualização de PG/EG/OP dentro do slot até que a ordem das subfases seja sustentada por evidência.

## 5.1 Possível canal de diagnóstico interno (test mode)

O manual Yamaha descreve um modo de barramento normalmente não utilizado no qual `CS=0`, `WE=1` e `A0=0` colocam dados de teste do OPLL nos pinos D0/D1. Isso é um fato documentado sobre a interface, mas o formato serial interno não está especificado no manual.

Uma nota técnica de Nuke.YKT, reproduzida pelo arquivo de referência Silicon Prawn, propõe um formato para esse modo: D0 serializaria bits do PG de um operador e bits do EG de outro; D1 serializaria bits de um contador/fase. A própria nota marca a interpretação como **não verificada em hardware**.

**Uso permitido:** incluir esse modo como experimento de instrumentação de baixo nível, após confirmar o comportamento em um YM2413 físico. Se validado, pode fornecer observabilidade direta de estados internos e reduzir a ambiguidade dos offsets PG/EG.

**Uso proibido neste estágio:** implementar readback no emulador, assumir o formato de bits como especificação, ou usar a descrição de Nuke.YKT como prova. Primeiro é necessário capturar D0/D1 em múltiplos ciclos, comparar com saídas MO/RO e repetir após reset.

## 6. Plano experimental para fechar os offsets

### E1 — Captura de saída com resolução de clock

- Capturar MO e RO simultaneamente a pelo menos uma amostra por input clock (idealmente oversampling).
- Registrar também XIN e, se possível, CS/WE/A0.
- Configurar timbres de teste que isolem modulator, carrier e feedback.
- Identificar os três clocks ativos e o retorno ao nível neutro por canal.
- Repetir em cold reset e warm reset, registrando fase inicial.

### E2 — PG e slot alignment

- Usar FNUM/Block/ML que produzam incrementos conhecidos e longos segmentos constantes.
- Capturar a transição de fase em vários pontos do período de 72 clocks.
- Variar a escrita do FNUM ao redor do slot consumidor, respeitando os tempos mínimos do manual.
- Determinar se o novo incremento afeta o PG no mesmo slot, na próxima visita do slot ou em outro atraso.

### E3 — EG e contador global

- Usar carrier com envelope isolável, depois modulator com TL máximo.
- Executar sequências repetíveis KEY-OFF → KEY-ON, KEY-ON → KEY-OFF, SUS e alteração de AR/DR/RR.
- Deslocar o evento de escrita em cada uma das quatro posições de clock do slot.
- Medir a primeira transição e as subsequentes para estimar fase do contador global.
- Repetir após reset para distinguir estado inicial físico de fase arbitrária do instrumento de captura.

### E4 — Modulator/carrier e feedback

- Configurar modulator com TL/FB variados e carrier em configuração que facilite observar o sinal resultante.
- Comparar feedback 0, 1, 4 e 7.
- Capturar a primeira rotação após KEY-ON e o regime estável.
- Testar se os dois valores históricos são capturados em cada visita de slot, por rotação ou em outra cadência.
- Documentar latência M→C por input clocks, não apenas por número de slots.

### E5 — Escritas concorrentes e latência de barramento

- Fazer escritas de endereço/dados em offsets variados do slot, sempre respeitando 12/84 clocks para estabelecer baseline.
- Em ensaios separados, violar os tempos do manual apenas para registrar comportamento indeterminado, sem transformar resultados não garantidos em contrato.
- Distinguir instante de aceitação externa do instante em que o valor passa a afetar PG, EG, OP ou saída.

### E6 — Reprodução e validação independente

- Publicar os vetores brutos com clock count, registradores, estado de reset e capturas MO/RO.
- Criar gerador de fixtures que converta as capturas em vetores determinísticos.
- Exigir repetição de cada ensaio e registrar dispersão/ruído.
- Comparar modelo Rust contra hardware; usar outros emuladores somente como comparação secundária.
- Verificar equivalência de avanço contínuo e em blocos e snapshot/restore.

## 7. Questões bloqueadoras ainda abertas

| ID | Questão | Bloqueia |
|---|---|---|
| P1-01 | Qual subfase do slot atualiza PG? | Pipeline PG |
| P1-02 | Qual subfase EG atualiza estado por operador? | Pipeline EG |
| P1-03 | Qual a fase absoluta e o reset do contador global EG/LFO? | Determinismo após reset |
| P1-04 | Qual subfase executa SIN/EXP e registra o resultado do operador? | Operador digital |
| P1-05 | Em que momento o feedback é capturado e aplicado? | Feedback cycle-accurate |
| P1-06 | Quantos clocks/slots separam modulator de carrier? | FM por canal |
| P1-07 | Como e quando o resultado do carrier entra no latch de MO/RO? | Saída digital |
| P1-08 | Como o decode/commit de registradores se relaciona ao slot corrente? | Escritas temporais |
| P1-09 | Qual a relação elétrica/fase do clock YM2413 com PSG/Z80 na placa SMS? | Scheduler e integração SMS |
| P1-10 | Como o atraso de uma amostra medido no modelo M→C se mapeia em input clocks e latches físicos? | Pipeline M→C cycle-accurate |

Nenhuma dessas questões deve ser fechada com base exclusiva em uma implementação GPL. O modelo `emu198x-yamaha-ym2413` e Nuked-OPLL podem ser usados para formular hipóteses e casos de comparação; sua arquitetura não substitui captura física e o código GPL não deve ser copiado para este projeto.

## 8. Referências usadas nesta investigação

1. Yamaha, *YM2413 Application Manual*, seção II-5/Tabela II-3 (18 slots e ordem), seção II-4/Tabela II-2 (esperas de 12/84 clocks), seção de propriedades do DAC. [SMS Power manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual) · [PDF alternativo](https://www.waveguide.se/?article=46&file=ym2413am.pdf).
2. Andete, [Phase counter (2015-03-16)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-16): medição de 19 bits, 9 bits fracionários e fórmula de incremento.
3. Andete, [EG decay/release (2015-03-20)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20): cadência baseada em contador global compartilhado por 18 operadores, analisada em unidades de amostra.
4. Andete, [Feedback (publicado como 2015-03-27; página indexada como 2015-12-12)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-12): feedback baseado nos dois outputs anteriores e dependência das condições iniciais.
5. Andete, [ADSR transitions (2015-12-31)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31): transições KEY/SUS/EG-TYP/DAMP.
6. MSX Resource Center, [YM2413 reverse engineering progress](https://www.msx.org/photoshoots/ym2413-reverse-engineering-progress): medição de multiplexação MO/RO em 72 clocks e pulso ativo de três clocks por canal.
7. [emu198x-yamaha-ym2413 documentation](https://docs.rs/emu198x-yamaha-ym2413/latest/emu198x_yamaha_ym2413/): descrição de pipeline com offsets e relação explícita com Nuked-OPLL 1.0.2. Referência secundária, não prova primária; não copiar código.
8. [SMS Power YM2413 index](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes): índice cronológico das medições e das limitações do die shot FHB013.
9. [Silicon Prawn — YM2413 test data notes](https://siliconprawn.org/archive/doku.php?id=vendor:yamaha:opl2): hipótese de formato D0/D1 em test mode, explicitamente não verificada em hardware.

## 9. Resultado desta etapa

- **Confirmado/documentado:** 18 slots seriais, ordem de atribuição, 72 input clocks por rotação, 4 clocks por slot como derivação, 19-bit PG, EG por operador e contador global por rotação como modelo medido, feedback com dois valores históricos, multiplexação MO/RO.
- **Modelo de saída medido:** o modelo de Andete usa a fase do modulator da amostra anterior; o vetor FNUM=0x40/BLOCK=0/MUL=0 demonstra por que isso não pode ser representado por `phase_index - 1`.
- **Ainda não confirmado:** offsets PG/EG/OP por subfase, fase de reset, mapeamento do atraso M→C de uma amostra para input clocks/latches físicos, commit temporal de registradores, latch DAC exato e alinhamento de fase com a placa SMS.
- **Alteração candidata na branch de correção:** `OperatorPair` conserva a fase do modulator da avaliação anterior e a inclui no snapshot. A primeira fase histórica é opcional e não presume o estado elétrico após reset.
- **Decisão:** não iniciar ainda o pipeline executável nem integrar Scheduler. A próxima entrega precisa ser um conjunto de vetores de captura ou uma fonte primária adicional que resolva as questões P1-01 a P1-08.


---

## Atualização de investigação — comparação com modelo Rust de terceiros (2026-11-19)

Foi examinada a documentação pública do crate `emu198x_yamaha_ym2413` 0.1.0. Ela afirma que o clock de entrada é dividido por quatro e que, em cada clock interno, PG, EG e operador processam slots diferentes; uma amostra corresponde a 18 clocks internos (72 clocks de entrada). A documentação declara expressamente que offsets de pipeline e aritmética seguem Nuked-OPLL 1.0.2, licenciado GPL-2.0-or-later, com testes de equivalência de pinos executados clock a clock.

### Resultado da comparação

- A divisão 72 / 18 = 4 é compatível com o orçamento de quatro input clocks por slot já documentado neste projeto.
- A hipótese de estágios PG/EG/OP sobrepostos em slots distintos é uma arquitetura plausível e explícita em uma implementação de terceiros.
- Essa fonte não é uma medição independente: seu próprio texto atribui offsets e aritmética ao Nuked-OPLL. Não pode ser usada como prova primária nem como autorização para copiar código/offsets GPL.
- A medição de Andete de 2015-12-03 sustenta o atraso do modulador em uma amostra de saída do modelo, mas não informa o deslocamento exato em input clocks/subfases.
- O manual e as capturas de saída sustentam slots, rotação e multiplexação; não estabelecem o offset individual de PG, EG, operador, feedback e latch DAC em cada subfase.

### Decisão técnica

Manter o sequenciador de clock/slot como fronteira temporal comprovada (4 input clocks por slot, 18 slots por rotação) e manter os updates PG/EG e a avaliação FM fora de uma subfase física não demonstrada. Não importar, transcrever ou adaptar implementação derivada de Nuked-OPLL GPL.

### Evidência necessária para desbloquear a próxima implementação

1. Fonte primária ou captura física com resolução por input clock que localize os eventos PG/EG/OP/DAC, repetida com diferentes slots e após reset.
2. Vetores que correlacionem escrita de FNUM/BLOCK/MUL e KEY-ON com a primeira atualização observável de cada operador.
3. Comparação de MO e RO, incluindo a janela ativa de três clocks e os slots de saída, com estado e timestamp registrados.
4. Para cada hipótese de pipeline, teste diferencial que compare execução contínua e particionada e preserve o estado em snapshots nos limites de subfase.

Até que esses dados existam, o pipeline de estágios sobrepostos permanece hipótese de investigação, não implementação aprovada.


---

## Atualização — roteamento MO/RO e largura de pulso por slot (2026-11-19)

A PR #1042 integrou o módulo puro `native/ym2413/src/output.rs`.

### Contrato implementado

- MO recebe eventos de carrier dos nove canais em modo melódico.
- Em Rhythm Mode, MO recebe apenas canais 1–6.
- RO recebe eventos originais e repetidos das cinco vozes conforme o mapa medido de slots.
- A API registra três input clocks ativos dentro do slot de quatro clocks e um clock neutro.
- Eventos RO inválidos são rejeitados; estado de repetição ainda não inicializada permanece distinto de ausência de saída.

### O que permanece em aberto

A nova camada não resolve P1-07 por completo. Ela estabelece o roteamento lógico por slot e a largura de pulso, mas não determina:
- subfase exata em que o valor começa e termina;
- valor elétrico do nível neutro;
- latch e transformação do valor do operador para a palavra de nove bits do DAC;
- instante exato de seleção/commit do DAC;
- resposta analógica do source follower e carga da placa.

A Issue #1031 continua bloqueando a conversão do intermediário one's-complement para o domínio DAC, devido às formas de saída publicadas que divergem em polaridade e offset. Não contornar essa incerteza usando o wrapper `Ym2413DacCode`.

### Validação

CI #44450484181: Rustfmt, Clippy, testes unitários/integração do workspace, Clippy/testes de observability e gate final de qualidade SUCCESS. Jobs de integração SMS, PSG, Scheduler, CPAL, VDP, Z80 e desktop foram SKIPPED, por não serem afetados.


---

## 10. Auditoria específica — contador global do EG (2026-11-19)

### 10.1 Evidência positiva sobre a cadência

A nota de Andete de 2015-03-20 descreve explicitamente **um contador global compartilhado pelos 18 operadores**, incrementado uma vez por amostra do YM2413. A duração de amostra usada na própria análise é:

`72 / 3.579.545 s`

Logo, a cadência média descrita é uma atualização por rotação de 72 input clocks, aproximadamente 49.715,9027778 Hz. Isso é consistente com a rotação serial de 18 slots × 4 input clocks por slot documentada no manual e na investigação P1.

A mesma nota explica que o contador não é apenas um divisor: sua fase determina a posição das janelas de atualização. Por isso a primeira transição de um envelope pode ser mais curta que as seguintes, dependendo do valor corrente do contador quando a fase ADSR começa.

Fonte: Andete, [YM2413 Reverse Engineering Notes 2015-03-20](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20), seções “Envelope-Generator Decay-Rate timing” e “global counter”.

### 10.2 Relação com o seletor de taxa implementado

Para decay/release, a descrição publicada estabelece:

- RATE 0..3: não avança o nível.
- RATE 60..63: avança dois níveis por avaliação.
- RATE 4..59: usa `eg_shift = 13 - (RATE / 4)` e `eg_select = RATE & 3`.
- O avanço é elegível quando os bits inferiores descartados pelo shift são zero; os três bits seguintes selecionam a posição da tabela de oito passos.
- RATE 52..59 apresentou sequências que divergiram da extrapolação simples, e a análise publicada descreve tratamento especial/observações adicionais para essa faixa.

A implementação atual em `EnvelopeGenerator::advance_decay` corresponde estruturalmente a essa decomposição:
- 4..51: máscara de bits baixos, coluna derivada de `counter >> shift` e `EG_STEP_TABLE`;
- 52..59: `EG_FAST_DECAY_TABLE` indexada pelos quatro bits baixos;
- 60..63: incremento 2.

A função `advance_attack` também consome o mesmo contador externo, com regras próprias para ATTACK. As medições de 2015-03-27 relacionam os padrões de ataque ao mesmo contador global, mas registram menor confiança para a faixa de rates mais rápidos.

**Conclusão:** o parâmetro `global_counter` recebido pelo EG representa uma fase global de amostras/rotações, e não um contador local do operador nem o número de clocks dentro do slot.

### 10.3 Relação aritmética com a rotação de slots

O core de clock mantém:
- `elapsed_input_clocks`;
- `input_clock_phase` (0..3);
- `slot_index` (0..17);
- `completed_rotations`.

A implementação incrementa `completed_rotations` somente quando o avanço de input clocks cruza a fronteira de 18 slots. A identidade aritmética do snapshot é:

`elapsed_input_clocks = completed_rotations * 72 + slot_index * 4 + input_clock_phase`

Isso demonstra uma contagem determinística de rotações desde o reset lógico do objeto. Não demonstra que `completed_rotations` seja o registrador/contador global físico do EG, nem define se o EG observa o valor anterior ou posterior ao incremento na fronteira de rotação.

### 10.4 O que está demonstrado e o que permanece aberto

| Questão | Resultado | Limite |
|---|---|---|
| Cadência média do contador global | **MEDIDA/descrita:** uma vez por amostra de saída (~49,716 kHz) | Não identifica o subclock em que o incremento é visível aos slots |
| Relação com os 72 clocks | **Derivada:** uma amostra corresponde a 72 input clocks | Não determina a ordem de commit no clock 0..3 do slot 18 |
| Compartilhamento | **MEDIDO/interpretado:** um contador comum aos 18 operadores | Não prova largura física nem implementação em flip-flops |
| Gate de DECAY/RELEASE | **MEDIDO/modelado:** bits do contador global alimentam shift/seletor | As transições de fase podem observar valor antes/depois do incremento |
| Gate de ATTACK | **MEDIDO/modelado:** mesma base global, seleção específica de ataque | A faixa de rates rápidos possui ressalvas nas medições publicadas |
| Origem/fase após reset | **PENDENTE** | Não há evidência suficiente para escolher zero ou outro offset |
| Largura/wrap do contador | **PENDENTE** | Não inferir largura a partir de `u32`, da fase PM ou do contador de slots |
| Relação com PM/AM | **Parcial** | As notas associam outras funções ao contador, mas os offsets exatos e reset comum não estão fechados |
| Relação com os 18 slots | **Parcial** | A unidade de incremento é a rotação; a fase de observação por operador não está medida |

### 10.5 Decisão arquitetural

1. Não usar `completed_rotations` diretamente como substituto do contador físico EG.
2. Não incrementar o contador global 18 vezes por rotação nem uma vez por input clock.
3. Não escolher fase inicial zero como comportamento de hardware.
4. Não acoplar a contagem global ao EG isolado. O EG deve continuar recebendo o contador por parâmetro até existir um owner temporal do chip.
5. Na futura integração, manter um estado de contador global explícito e versionado, com origem/fase configuráveis como hipótese documentada; o avanço nominal deverá ocorrer uma vez por rotação completa, após estabelecer o ponto de observação dos operadores.
6. O estado global, a posição parcial de slot e os estados PG/EG/LFO deverão integrar o mesmo snapshot atômico do chip.

### 10.6 Próximo experimento que realmente reduz a incerteza

Para separar fase inicial, ordem de incremento e fase de serviço:
- manter um carrier isolado em DECAY com RATE conhecido (preferencialmente RATE=14, cuja sequência publicada é 2048, 1024, 1024 amostras);
- disparar KEY-ON/KEY-OFF em offsets controlados das 72 posições de input clock;
- repetir a mesma sequência após cold reset e warm reset;
- comparar a primeira transição e as subsequentes;
- repetir com um segundo operador em slot diferente para verificar se ambos consultam a mesma fase global;
- registrar o contador de input clocks e a amostra de saída, sem derivar o resultado de timestamps do host.

Sem captura física nova, a implementação não pode resolver a fase absoluta nem o subclock de observação. A evidência atual é suficiente para fixar a **cadência média por rotação** e rejeitar incrementos por slot/input clock, mas não para declarar o contador físico fechado.

**Resultado desta auditoria:** cadência global do EG caracterizada em unidades de amostra/rotação; origem, largura, wrap, ordem de incremento/consumo e alinhamento absoluto continuam pendentes. Nenhum código de produção foi alterado.
