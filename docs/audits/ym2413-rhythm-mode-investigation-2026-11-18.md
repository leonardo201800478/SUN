
# YM2413 Rhythm Mode — evidence and implementation plan

**Data:** 2026-11-18
**Escopo:** FM-7 — cinco vozes de percussão
**Estado:** investigação documental; nenhuma síntese rítmica implementada por este documento.

## 1. Objetivo e classificação

Separar o que o manual Yamaha descreve, o que foi observado em hardware por reverse engineering, o que aparece em implementações comparativas e o que permanece sem comprovação.

- **D — documentação primária:** YM2413 Application Manual.
- **M — medição/reverse engineering:** captura física publicada por andete.
- **C — comparação de implementação:** código de terceiros; não normativo e sujeito a diferenças OPLL/OPL.
- **I — inferência técnica:** conclusão derivada explicitamente de D/M/C.
- **U — desconhecido:** evidência insuficiente.

## 2. Contrato estático de registradores [D]

O registrador $0E controla o modo e os cinco triggers:

| Bit | Campo | Voz |
|---|---|---|
| D5 | RHYTHM | Habilita modo de percussão |
| D4 | BD | Bass Drum |
| D3 | SD | Snare Drum |
| D2 | TOM | Tom-Tom |
| D1 | TOP-CY | Top Cymbal |
| D0 | HH | Hi-Hat |

Com RHYTHM habilitado, a seção melódica fica limitada a seis canais. O manual orienta manter KEY-ON dos canais 7–9 ($26–$28) em zero. Os seis slots são:

| Slot | Uso |
|---:|---|
| 13 e 16 | Bass Drum — par FM |
| 14 | Hi-Hat |
| 15 | Tom-Tom |
| 17 | Snare Drum |
| 18 | Top Cymbal |

Volumes independentes:
- $36.D3..D0: BD.
- $37.D7..D4: HH; $37.D3..D0: SD.
- $38.D7..D4: TOM; $38.D3..D0: TOP-CY.

A PR #1014 integrou o decoder desses campos em native/ym2413/src/rhythm.rs. Ele preserva os triggers brutos, o gate RHYTHM e FNUM/BLOCK/KEY/SUS dos canais 7–9. Não sintetiza áudio nem altera os registradores de entrada.

### 2.1 Interação com KEY-ON melódico [D/M/U]

O manual orienta manter em zero os bits KEY-ON dos canais físicos 7–9 enquanto Rhythm Mode está ativo. As capturas/notas de andete acrescentam uma observação ainda marcada como TODO: os bits de trigger em $0E e os bits KEY-ON de $26–$28 parecem ser considerados conjuntamente, como se fossem combinados por OR. Não há caracterização suficiente no material consultado para definir transições, prioridade ou comportamento em mudanças de modo.

Por isso, `Ym2413RhythmVoiceConfig::is_active` representa apenas o gate lógico do registrador $0E (RHY + trigger), não uma decisão completa de atividade do silício quando KEY-ON melódico está setado. O decoder preserva os bits KEY-ON/SUS crus dos canais 7–9. Não normalizar nem apagar esses bits.

## 3. Síntese das vozes segundo o manual [D]

### 3.1 Bass Drum

O Bass Drum usa dois slots para produzir FM. O manual diz que sua síntese segue essencialmente o procedimento FM normal dos operadores. Isso permite reutilizar os blocos de PG/EG/operadores já existentes, mas ainda não determina a ordem temporal de execução, a inicialização dos operadores, o estado de feedback ou o ganho final.

### 3.2 Hi-Hat, Snare Drum, Tom-Tom e Top Cymbal

O manual descreve um oscilador de ruído composto por um gerador de ruído branco e frequências derivadas das informações BLOCK, F-Number e MULTIPLE dos canais 8 e 9. A composição produz fases apropriadas para as quatro vozes e as entrega aos operadores correspondentes. A transcrição inglesa do manual registra uma relação ótima empírica como `f7CH = 3 × f8CH`. Há uma ambiguidade nos rótulos: a frase anterior nomeia os canais 8 e 9 como fontes de frequência, enquanto a relação cita 7CH e 8CH. O exemplo de registradores do próprio manual ($16=$20, $17=$50, $18=$C0, $26=$05, $27=$05, $28=$01) deve ser preservado como vetor de configuração separado, não como prova de que essa razão 3:1 foi atendida. A ambiguidade precisa ser resolvida a partir do scan original ou de medição, sem normalização por suposição.

O manual também informa que os envelopes de cada instrumento são armazenados em ROM, com uma atribuição de envelope por slot. O trecho consultado não fornece o conteúdo completo da ROM nem uma tabela de valores que permita reconstruí-la sem fonte adicional.

**Consequência:** não é correto implementar quatro vozes não-BD como simples ruído branco filtrado ou como osciladores independentes. O caminho exige phase-combining dependente das fases/frequências dos operadores dos canais 8 e 9, ruído e envelope dedicado.

## 4. LFSR e medições [M]

As notas de andete descrevem capturas do YM2413 real com amostragem 72 vezes mais rápida para separar os canais e observar simultaneamente MO e RO. A sequência de ruído observada foi analisada com Berlekamp–Massey e repetida em diferentes offsets e capturas.

A análise inicial de 2018 identificou a sequência decimada com polinômio:

x^23 + x^9 + x^8 + x + 1

A atualização experimental de 2020 corrigiu a interpretação da cadência física: o gerador interno usa a recorrência mais rápida x^23 + x^9 + 1, avançada a cada operador/slot, isto é, uma vez a cada quatro clocks externos. Como existem 18 slots por rotação, ocorrem 18 passos do LFSR por 72 clocks externos. Observar apenas um bit a cada 18 passos produz a sequência decimada descrita pelo polinômio anterior.

A relação entre os dois sons que consomem ruído também foi medida:
- Hi-Hat é calculado no slot 13 e usa o bit corrente.
- O LFSR avança três passos de slot até o slot 16.
- Snare Drum usa então o bit corrente no slot 16.
- Após SD, avançam-se os outros quinze passos até o próximo slot 13.

O período máximo matemático do LFSR de 23 bits não nulo é 2^23 - 1 passos internos. O estado físico de reset continua desconhecido.

O crate agora preserva a recorrência antiga em `Ym2413RhythmNoiseLfsr` como referência da sequência decimada e adiciona `Ym2413RhythmSlotNoiseLfsr` para a recorrência rápida. A nova primitiva exige seed explícito e avanço explícito por número de slots; ainda não está conectada ao MachineScheduler. Os testes verificam a recorrência rápida, a relação de decimação a cada 18 passos, o espaçamento de três slots HH→SD e snapshot/restore. Esses testes validam o modelo algébrico, não o seed físico.

## 5. Equações de fase e envelope por voz [M/C]

As notas de andete em `results/rhythm/rhythm.md` contêm capturas de hardware e modelos derivados para as cinco vozes. O mesmo material compara as fórmulas com Nuke.YKT. As equações abaixo são candidatas derivadas de medições; não definem o alinhamento inicial dos PGs.

### 5.1 Top Cymbal [M]

As medições mostram que a fase depende de cinco bits: bits 5 e 3 do carrier do canal físico 8 (`c85`, `c83`) e bits 7, 3 e 2 do modulador do canal físico 7 (`m77`, `m73`, `m72`). Portanto a fórmula anterior baseada no modulador do canal 8 e carrier do canal 9 foi descartada.

A análise de andete encontrou 16 tabelas booleanas compatíveis com as capturas, porque os offsets iniciais dos dois contadores de fase não foram determinados. A atualização de Nuke.YKT registrada por andete usa a forma OR e o offset 0x100, que reproduz a amplitude máxima observada:

```
rm_bit = (m72 XOR m77) OR (m73 XOR c85) OR (c83 XOR c85)
phase  = (rm_bit << 9) OR 0x100
```

O crate implementa essa forma como `top_cymbal_phase_candidate`, recebendo explicitamente os índices de fase de mod7 e car8. O resultado é 0x100 ou 0x300. A função é combinacional e não resolve o offset absoluto do PG; o scorecard mantém o estado CANDIDATE.

### 5.2 Hi-Hat [M]

O Hi-Hat reutiliza o mesmo `rm_bit` de Top Cymbal e combina esse resultado com o bit de ruído fornecido pelo LFSR:

```
phase = (rm_bit << 9) OR
        ((rm_bit XOR noise) ? 0x0D0 : 0x034)
```

Os quatro endereços possíveis são 0x034, 0x0D0, 0x234 e 0x2D0. O crate implementa a função pura `hi_hat_phase_candidate`; o bit de ruído é entrada explícita, sem avanço implícito do LFSR.

### 5.3 Snare Drum [M]

A fase usa o bit 8 do modulador do canal físico 7 e o bit corrente do ruído:

```
phase = (hh_bit8 << 9) OR ((hh_bit8 XOR noise) << 8)
```

A atualização de andete registra que novas capturas confirmaram os dois estados de sinal nulo (+0 e -0). O crate implementa a função pura `snare_drum_phase_candidate`, recebendo o bit de ruído explicitamente.

### 5.4 Tom-Tom [M]

O Tom-Tom usa um único operador, mod8, com a fase normal do PG do canal físico 9. As capturas mostram que sua frequência varia com os registradores FNUM/BLOCK desse canal. Não há combinador de ruído documentado para TOM nas medições consultadas.

### 5.5 Parâmetros de operadores/envelopes [M/C]

A investigação de andete apresenta estes bytes candidatos, derivados da comparação de capturas com parâmetros de instrumento customizados e confrontados com a tabela de Nuke.YKT:

| Voz | Bytes 0–7 | Operador ativo | Observação |
|---|---|---|---|
| BD | `01 01 18 0F DF F8 6A 6D` | mod6 + car6 | Aproximação próxima; resta diferença de alinhamento de uma amostra e pico |
| HH | `01 00 00 00 C8 00 A7 00` | mod7 | Envelope medido; parâmetros do carrier não usados |
| SD | `00 01 00 00 00 D8 00 68` | car7 | Envelope medido; parâmetros do modulator não usados |
| TOM | `05 00 00 00 F8 00 59 00` | mod8 | Um único operador tonal |
| TCY | `00 01 00 00 00 AA 00 55` | car8 | Envelope medido; parâmetros do modulator não usados |

Esses bytes foram adicionados como `YM2413_RHYTHM_PATCHES_CANDIDATE` em `instrument.rs`. Não são apresentados como dump físico completo da ROM. Para BD, o autor relata diferença residual de uma amostra e pico ligeiramente superior ao instrumento customizado. Para as outras vozes, os valores são candidatos de operador/envelope; a comparação de Nuke.YKT é corroborativa, não uma segunda medição de silício.

### 5.6 Duplicação e slots de saída [M]

A investigação de andete mediu que cada som de percussão é emitido duas vezes, com a segunda saída idêntica à primeira mesmo quando parâmetros são alterados entre as duas ocorrências. O mapa de 18 slots medido é:

| Saída original | Slot | Repetição | Slot |
|---|---:|---|---:|
| HH | 13 | HH2 | 0 |
| TOM | 14 | TOM2 | 1 |
| BD | 15 | BD2 | 2 |
| SD | 16 | SD2 | 6 |
| TCY | 17 | TCY2 | 7 |

O relatório identifica em um die shot um banco de nove shift registers de comprimento cinco, próximo ao DAC, compatível com o armazenamento dos cinco valores de saída de nove bits. Há uma ressalva importante: o SMS Power informa que os die shots disponíveis são de um FHB013, derivado do YM2413 com conjunto de instrumentos diferente, e não de um YM2413 real. Portanto, a posição física do banco de registradores é inferência estrutural comparativa, não confirmação do layout do YM2413. A duplicação e as posições de saída, por outro lado, foram medidas diretamente nas saídas MO/RO de um YM2413 real.

## 5.7 Limites das referências comparativas [C/U]

Genesis Plus GX contém rotinas de percussão que identificam HH/SD/TOM como verificadas em YM3812 e Top Cymbal como verificado em YM2413. Sua fórmula Top Cymbal usa fontes de fase diferentes das identificadas na análise de capturas de andete; por isso não foi adotada como contrato do crate. As equações atuais são derivadas das notas experimentais de andete e das correções documentadas de Nuke.YKT.

A tabela de patches de ritmo de `emu2413` é comentada como originária do VRC7; ela não foi usada como fonte dos bytes YM2413. Os valores incluídos no crate vêm das tabelas experimentais de andete, com as limitações descritas acima.

## 6. Matriz por voz

| Voz | Slots | Envelope/operadores | Fase | Evidência atual | Estado |
|---|---|---|---|---|---|
| Bass Drum | 13 + 16 | Dois operadores FM | FM normal com parâmetros de BD | Manual + comparação de captura com patch customizado | PATCH CANDIDATE IMPLEMENTED; forma de onda ainda não integrada |
| Hi-Hat | 14 | mod7 | rm_bit de mod7/car8 + LFSR | Capturas de hardware e modelo de fase/envelope de andete | PHASE/PATCH CANDIDATE IMPLEMENTED; offsets e áudio pendentes |
| Snare Drum | 17 | car7 | bit8 de mod7 + LFSR | Capturas de hardware e confirmação posterior de +0/-0 | PHASE/PATCH CANDIDATE IMPLEMENTED; offsets e áudio pendentes |
| Tom-Tom | 15 | mod8 | PG normal do canal físico 9 | Capturas de hardware e parâmetros de operador | PATCH CANDIDATE IMPLEMENTED; integração pendente |
| Top Cymbal | 18 | car8 | rm_bit de mod7/car8; 16 variantes equivalentes por offsets | Capturas de hardware e correção posterior de Nuke.YKT | PHASE/PATCH CANDIDATE IMPLEMENTED; offset absoluto pendente |

## 7. Plano de implementação incremental

### FM-7A — contratos e tabelas
- Manter Ym2413RhythmConfig como entrada estática.
- Adicionar testes one-hot para cada trigger, além do teste conjunto existente, garantindo que o mapeamento D4..D0 não seja validado apenas por todos os bits em 1.
- Separar parâmetros estáticos de cada slot rítmico de estados dinâmicos PG/EG/feedback/LFSR.
- Definir tipos para phase inputs, envelope ROM candidate e volume attenuation sem decidir clocks.

### FM-7B — Bass Drum
- Reutilizar o par FM e o slot map 13/16.
- Construir vetores determinísticos de parâmetros e saída digital em nível de operador.
- Não adicionar ganho/duplicação de saída sem evidência de hardware.
- Não assumir o estado inicial de fase nem o ponto de KEY-ON.

### FM-7C — quatro vozes não-BD
- Implementar a função de combinação de fase como componente puro e testável, com entradas explícitas de fase dos operadores, campos FNUM/BLOCK/MULTIPLE e bit LFSR.
- Implementar uma voz por vez, começando por vetores em que se varie uma única entrada.
- Preservar a cadência do LFSR como entrada externa.
- Separar fase calculada, lookup de waveform e envelope para localizar divergências.

### FM-7D — envelopes e volumes
- Identificar a ROM de envelopes rítmicos e sua procedência/licença.
- Não derivar envelopes a partir dos 15 presets melódicos.
- Confirmar como $36–$38 se combinam com envelope por slot e com KSL/TL.
- Testar todos os valores de volume, incluindo zero e máximo, em nível digital.

### FM-7E — saídas e validação
- Determinar a rota de cada slot para MO ou RO.
- Investigar a observação de emissão duplicada sem presumir que signifique duplicação de amplitude, repetição de amostra ou duas janelas de saída.
- Criar snapshots completos dos estados usados pelos componentes implementados.
- Validar equivalência contínuo/blocos para entradas explícitas; isso não substitui o contrato físico do relógio.

## 8. Critério de conclusão FM-7

FM-7 somente poderá ser marcado como VALIDATED quando:
1. as cinco vozes tiverem geração digital implementada;
2. cada voz tiver vetores determinísticos independentes;
3. os envelopes e volumes tiverem origem documentada;
4. a aplicação do ruído e a fase das quatro vozes não-BD tiverem evidência suficiente;
5. a atribuição e o roteamento MO/RO estiverem testados;
6. o estado necessário para snapshot/restore estiver completo;
7. as incertezas sem hardware estiverem explicitamente isoladas, sem alegação de validação física.

O fechamento funcional de software pode preceder a comparação com hardware, mas o scorecard deve distinguir IMPLEMENTED, INTEGRATED, VALIDATED e HARDWARE-CORROBORATED.

## 9. Referências

### Documentação primária
- Yamaha, YM2413 Application Manual, III-1-7 (Rhythm), III-1-8 (Instrument/Volume), V-4 (Procedure of Percussion Sound Synthesis): https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
- Espelho PDF do manual: https://map.grauw.nl/resources/sound/yamaha_ym2413.pdf

### Reverse engineering / medição
- andete, YM2413 Reverse Engineering Notes 2018-05-13: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13
- andete, Rhythm Mode waveform captures and reverse-engineering analysis: https://github.com/andete/ym2413/blob/master/results/rhythm/rhythm.md
- andete, LFSR measurements and 2020 slot-rate update: https://github.com/andete/ym2413/blob/master/results/lfsr/lfsr.txt
- Repositório de resultados de captura indicado pelo autor: https://github.com/andete/ym2413/tree/master/results

### Comparação não normativa
- Genesis Plus GX, core/sound/ym2413.c, pacote Debian 1.7.4+git20221128-2 (comparação; GPL): https://sources.debian.org/src/genesisplusgx/1.7.4+git20221128-2/core/sound/ym2413.c
- emu2413, tabela `default_inst` de três patches de ritmo comentados como VRC7 (MIT; não usar como dump YM2413): https://github.com/digital-sound-antiques/emu2413
- SMS Power, visão geral YM2413 e alocação de slots: https://www.smspower.org/Development/YM2413

Nenhum código GPL foi copiado. As referências de software acima servem somente para análise comparativa e desenho de experimentos.
