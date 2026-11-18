
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

## 3. Síntese das vozes segundo o manual [D]

### 3.1 Bass Drum

O Bass Drum usa dois slots para produzir FM. O manual diz que sua síntese segue essencialmente o procedimento FM normal dos operadores. Isso permite reutilizar os blocos de PG/EG/operadores já existentes, mas ainda não determina a ordem temporal de execução, a inicialização dos operadores, o estado de feedback ou o ganho final.

### 3.2 Hi-Hat, Snare Drum, Tom-Tom e Top Cymbal

O manual descreve um oscilador de ruído composto por um gerador de ruído branco e frequências derivadas das informações BLOCK, F-Number e MULTIPLE dos canais 8 e 9. A composição produz fases apropriadas para as quatro vozes e as entrega aos operadores correspondentes. O manual registra empiricamente uma relação ótima de frequência 3:1 entre os canais usados no exemplo.

O manual também informa que os envelopes de cada instrumento são armazenados em ROM, com uma atribuição de envelope por slot. O trecho consultado não fornece o conteúdo completo da ROM nem uma tabela de valores que permita reconstruí-la sem fonte adicional.

**Consequência:** não é correto implementar quatro vozes não-BD como simples ruído branco filtrado ou como osciladores independentes. O caminho exige phase-combining dependente das fases/frequências dos operadores dos canais 8 e 9, ruído e envelope dedicado.

## 4. LFSR e medições [M]

As notas de andete de 2018-05-13 descrevem captura do YM2413 real com amostragem 72 vezes mais rápida para separar os canais e observar simultaneamente as saídas MO e RO. A sequência de bits observada foi analisada com Berlekamp–Massey e repetida em capturas diferentes.

A análise identifica um LFSR de 23 bits com polinômio:

x^23 + x^9 + x^8 + x + 1

O período máximo matemático de um LFSR não nulo desse polinômio é 2^23 - 1. Isso confirma a recorrência candidata e sua sequência, mas não determina:
- o estado físico depois de reset;
- o ponto de amostragem da saída do LFSR;
- a cadência de clock-enable dentro do chip;
- a fase do LFSR em relação aos 18 slots;
- como cada bit de ruído entra no phase-combining de HH/SD/TOM/TOP-CY.

A implementação existente em rhythm.rs permanece corretamente como primitiva explicitamente clockada e com seed explícito. Não adicionar clock automático por input clock, slot ou sample até obter evidência.

## 5. Comparação com software [C — não normativa]

Uma implementação pública consultada no pacote Genesis Plus GX contém uma rotina de cálculo de ritmo com expressões booleanas sobre bits das fases dos operadores e o bit de ruído. A própria rotina distingue alegações de verificação em YM3812 para Hi-Hat/Snare/Tom-Tom e uma alegação de verificação em YM2413 para Top Cymbal.

Essa implementação é útil para formular experimentos e vetores candidatos, mas não é especificação primária do YM2413. Não copiar código. Antes de qualquer reimplementação independente das expressões:
1. mapear exatamente qual operador/fase é fonte de cada bit;
2. conferir o deslocamento de fase e largura dos acumuladores contra o PG já existente;
3. conferir as equações com o manual;
4. procurar as capturas originais de andete e identificar se permitem reproduzir as transições;
5. manter as equações como CANDIDATE até validação independente.

Não transportar automaticamente para YM2413 as rotinas que o código identifica como “verified on real YM3812”.

## 6. Matriz por voz

| Voz | Slots | Envelope/operadores | Fase | Evidência atual | Estado |
|---|---|---|---|---|---|
| Bass Drum | 13 + 16 | Dois operadores FM | PG normal, parâmetros dos canais de BD | Manual descreve FM em dois slots | CANDIDATE; falta mapear parâmetros/EG ROM e testar |
| Hi-Hat | 14 | Envelope dedicado do slot 14 | Ruído + phase-combining dos canais 8/9 | Manual + captura global de ritmo; software comparativo | BLOCKED para implementação física; algoritmo candidato precisa validação |
| Snare Drum | 17 | Envelope dedicado do slot 17 | Ruído combinado com fase derivada de canal | Manual + captura global; detalhe fino insuficiente | BLOCKED para implementação física |
| Tom-Tom | 15 | Envelope dedicado do slot 15 | Componente tonal baseada em fase de operador | Manual + software comparativo | CANDIDATE; confirmar se e como ruído participa |
| Top Cymbal | 18 | Envelope dedicado do slot 18 | Phase-combining de canais 8/9 e ruído | Manual + software comparativo com nota de validação YM2413 | CANDIDATE; exige vetores independentes |

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
- Repositório de resultados de captura indicado pelo autor: https://github.com/andete/ym2413/tree/master/results

### Comparação não normativa
- Genesis Plus GX, core/sound/ym2413.c, pacote Debian 1.7.4+git20221128-2: https://sources.debian.org/src/genesisplusgx/1.7.4+git20221128-2/core/sound/ym2413.c
- SMS Power, visão geral YM2413 e alocação de slots: https://www.smspower.org/Development/YM2413

Nenhum código GPL foi copiado. As referências de software acima servem somente para análise comparativa e desenho de experimentos.
