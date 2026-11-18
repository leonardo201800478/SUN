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
| Taxa de rotação/amostra por canal | 3.579.545 / 72 ≈ 49.716,04 Hz | Derivação aritmética das medições | Taxa de repetição do quadro serial, não taxa CPAL |
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
- As medições de feedback foram reproduzidas pelo modelo com a média/soma escalada dos dois últimos outputs do modulator.

**Limite:** o modelo de dois samples explica as capturas publicadas depois de sincronizado, mas as condições iniciais dos dois valores anteriores podem divergir e alterar bastante os primeiros outputs, especialmente em feedback alto. A posição exata de captura e consumo da cadeia dentro dos slots não é exposta pela tabela de slots.

**Consequência para o código:** manter a cadeia de feedback por canal e seu estado no snapshot; não presumir que o output do modulator de um slot seja consumido pelo carrier do mesmo par na mesma rotação sem identificar o latch e o atraso.

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

## 9. Resultado desta etapa

- **Confirmado:** 18 slots seriais, ordem de atribuição, 72 input clocks por rotação, 4 clocks por slot como derivação, 19-bit PG, EG por operador e contador global por rotação como modelo medido, feedback com dois valores históricos, multiplexação MO/RO.
- **Ainda não confirmado:** offsets PG/EG/OP por subfase, fase de reset, transferência M→C em clocks exatos, commit temporal de registradores, latch DAC exato e alinhamento de fase com a placa SMS.
- **Decisão:** não iniciar ainda o pipeline executável nem integrar Scheduler. A próxima entrega precisa ser um conjunto de vetores de captura ou uma fonte primária adicional que resolva as questões P1-01 a P1-08.

