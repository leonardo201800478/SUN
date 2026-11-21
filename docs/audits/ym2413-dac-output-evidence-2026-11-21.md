# YM2413 DAC/output evidence — 2026-11-21

## Sources and correction to the earlier level count

The immutable andete revision
`4893d1c09faa0450b5d3355f19b7682d72822501` contains:

- `results/20150209.txt`
- `results/20150305.txt`
- `results/20150311.txt`

The 2015-02-09 note initially reported 511 distinct levels and inferred
`-255..0..+255`, because its original ADC histogram could not resolve the
two central peaks. The 2015-03-05 follow-up explicitly corrects that result:
16x oversampling resolves **512 distinct peaks**, with two unusually close
central peaks. The author tentatively interprets them as `-0` and `+0`,
and says that an idealized model may map both to one zero. The 2015-03-11
follow-up independently discusses both `-0` and `+0` and reports that at
volume settings 14 and 15 the signal takes the four distinct values
`-1, -0, +0, +1`.

Therefore the old statement “the YM2413 has only 511 measured DAC values and
-256 is absent” is superseded. The 2015 note reports 512 output peaks,
including two separate central measured levels interpreted as signed zeros.
However, the later checked-in calibration implementation uses
`insertMissing()` to insert a midpoint when adjacent detected peaks are more
than 160 ADC counts apart. The generated `.peaks`/`.bins` artifacts used for
the published `.out` sequences are absent from the pinned tree, so the exact
number of directly observed peaks versus heuristically inserted peaks cannot
be independently recovered from the committed artifacts. The report supports
the 512-peak interpretation, but its calibration provenance must remain
qualified. The author's proposed idealized collapse of the two central levels
is explicitly tentative, not a demonstrated raw digital encoding.

The 2015-03-05 note also describes a 32x16 switch matrix and thermometer
decoders inferred from a die image. The SMS Power YM2413 development index
warns that the die shots referenced by these notes are from an FHB013
derivative rather than a confirmed YM2413 die. Treat that internal DAC
topology as a hypothesis; it is not required for the externally measured
dual-zero observation.

## What is and is not established

### Supported by the sources

- Yamaha's application manual specifies a built-in 9-bit DAC.
- Andete's 2015 note reports 512 oversampled output peaks; the calibration code's missing-peak insertion heuristic and absent generated artifacts prevent independent confirmation that all 512 were directly observed.
- The two central peaks are separately observed and described as `-0` and
  `+0` in the 2015-03-05 and 2015-03-11 notes.
- The measured volume response includes four distinct values
  `-1, -0, +0, +1` at volume settings 14 and 15.
- MO and RO are separate analog output pins.

### Still unknown

- The exact binary 9-bit word presented internally to the DAC.
- The bit-level encoding of the two central zero states.
- A proven conversion from the serial operator's one's-complement `i16`
  intermediate to the DAC input word.
- The precise sub-clock timing of the DAC input latch relative to the four
  clocks of each serial slot.
- The neutral electrical level and the analog source-follower/load transfer.
- Whether the two measured zero peaks should remain distinct in an idealized
  digital model or be collapsed at a later analog/output abstraction.

The reported 512 peaks (with directly observed versus inserted peaks
not independently recoverable from the committed calibration artifacts) do
not, by themselves, reveal the raw DAC input bit pattern. In particular, no shift, mask, sign extension or reinterpretation of
the operator's signed `i16` value is justified by these notes.

## FM-5.3 implementation consequence

The previous `Ym2413DacCode` wrapper represented only signed amplitude
`-255..=+255` and therefore collapsed the two central measured levels. The
FM-5.3 retention layer needs to preserve observed values verbatim, even while
the raw DAC encoding remains unknown.

The current implementation now represents:
- `Ym2413DacCode::new(0)`: the positive-zero measured-level token;
- `Ym2413DacCode::negative_zero()`: the separately observed negative-zero
  measured-level token;
- `Ym2413DacCode::value()`: signed amplitude projection (both zeros return
  numeric zero);
- `Ym2413DacCode::is_negative_zero()`: preserves the distinction.

This is a measurement-domain representation only. It is **not** a raw
nine-bit DAC word and does not convert from the operator intermediate.

Because the serialized retention payload changed semantically, the rhythm
retention and composed MO/RO service snapshot versions were advanced to 2.
Directed tests verify that negative zero remains distinct from positive zero,
survives the original-to-repeat RO path, and survives snapshot/restore.

## Decision

- Correct the historical “511 levels / -256 absent” wording; it reflects the
  superseded 2015-02-09 measurement.
- Preserve both measured central levels in the serial retention representation.
- Do not infer or implement the raw 9-bit DAC word yet.
- Continue FM-5.4 investigation using the original physical captures and
  Yamaha application manual; keep the operator-to-DAC conversion blocked
  until its arithmetic and temporal boundary are independently established.

References:
- andete `results/20150209.txt`, `results/20150305.txt`,
  `results/20150311.txt` at commit
  `4893d1c09faa0450b5d3355f19b7682d72822501`.
- SMS Power, “YM2413 Reverse Engineering Notes 2015-03-05”:
  https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-05
- SMS Power, “YM2413 Reverse Engineering Notes 2015-03-11”:
  https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-11
- Yamaha, YM2413 Application Manual, II-1 and III-4:
  https://www.waveguide.se/?article=46&file=ym2413am.pdf
- SMS Power YM2413 development index (die-shot provenance note):
  https://www.smspower.org/Development/YM2413


## Implementation validation update

The dual-zero retention increment is present at code commit
`27650ff687c9495415644bca1db30585ffed21b9`.

- CI #9852: SUCCESS — Rustfmt, workspace Clippy, workspace tests,
  observability Clippy/tests and documentation hygiene.
- PG differential #388: SUCCESS — 147,456 calls, strict serial/control/
  LFSR/timing PASS; candidate differences remain limited to SD SL/EG.
- Rhythm reset differential #311: SUCCESS.

The new tests verify that positive and negative zero are distinct measured
tokens and that negative zero survives the original HH output, delayed RO
repeat, and version-2 snapshot/restore. This validates the retention
representation only; it does not validate raw nine-bit code encoding or
operator-to-DAC conversion.


## Atualização após integração do adaptador — limite das capturas

A PR #1159 (`eb01ca8a7ee6224a98e872143b41f0b28137b292`) preserva o bloqueio de conversão: o PG fornece `None` ao adaptador medido. Nenhum nível `Ym2413DacCode` é inferido do `i16` do operador no caminho integrado.

### Inspeção adicional das capturas originais

O repositório de reverse engineering contém a rotina [`analyze/volume/volumes.cc`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/volume/volumes.cc) e os arquivos [`volume0.raw`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/volume/volume0.raw), [`volume0.out`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/volume/volume0.out) e [resultados de 2015-03-11](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/results/20150311.txt).

A rotina configura um patch senoidal, define F-number `0x61`, percorre volumes `0..15`, ativa a nota e captura amostras ADC da saída analógica. O arquivo `.raw` contém amostras ADC, não a palavra digital interna do DAC. O arquivo `.out` é uma sequência de índices de bins analógicos. A cadeia está no mesmo repositório: `analyze/calibrate.cc` identifica picos ADC e grava limites em `.bins`; `analyze/decode.cc` usa `lower_bound` nesses limites para converter cada amostra ADC em um índice 0..511. O índice representa a posição na calibração analógica, não o código digital aplicado ao DAC.

Uma inspeção exploratória de `volume0.out` encontrou 20.030 tokens e 437 valores distintos no intervalo 0..511. Isso caracteriza apenas a cobertura dessa sequência processada; não prova a codificação interna, não estabelece a ordem dos códigos do DAC e não é uma tabela de conversão produção. A afirmação de 512 níveis distintos nos resultados de medição continua sendo uma afirmação sobre níveis analógicos observados, não uma captura direta dos bits internos.

### Consequência técnica

- Não usar `volume*.out` como palavra DAC: são índices de bins analógicos produzidos pela calibração ADC.
- Não assumir que a representação one's-complement do operador é idêntica à entrada digital do DAC.
- Não converter o latch `ch_out` em `Ym2413DacCode` por identidade ou fórmula candidata.
- Manter desconhecidos os níveis medidos no adaptador PG até estabelecer aritmética, polaridade, fase e correspondência com evidência independente.

O próximo trabalho FM-5.4 deve reconstruir e reproduzir a cadeia ADC→bins→índices, validar a fase de captura e confrontá-la com a documentação de aplicação Yamaha. Mesmo uma tabela analógica calibrada não substitui a demonstração da palavra digital interna.


Fontes de calibração e decodificação:
- [`analyze/calibrate.cc`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/calibrate.cc) — detecção dos picos ADC e geração dos limites de bins.
- [`analyze/decode.cc`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/decode.cc) — conversão de cada amostra ADC para índice de bin por `lower_bound`.
- [`analyze/ym-histogram.cc`](https://github.com/andete/ym2413/blob/4893d1c09faa0450b5d3355f19b7682d72822501/analyze/ym-histogram.cc) — histograma dos índices já decodificados.

### Detalhes do driver ADC e limite temporal

Na revisão do firmware de captura, `tool2/adc0.cc` configura o ADC0 com entrada diferencial `adcSingleInpCh0Ch1`, anotada no código como `RO-MO`, e `diff = true`. A configuração usa clock ADC de 12 MHz, oversampling 16x, referência VDD e trigger PRS no canal 5.

O driver oferece duas fontes de trigger:
- Timer0 overflow (`adc0::start0`): documentado no código como uma amostra por clock YM2413.
- Timer1 overflow (`adc0::start1`): documentado no código como uma amostra a cada 72 clocks YM2413.

A seleção efetiva da varredura de volume foi rastreada:
- `analyze/volume/volumes.cc` chama `startCapture()`.
- `analyze/common.cc` implementa `startCapture()` enviando o comando USB `'S'`.
- `tool2/event.cc` trata `'S'` chamando `adc0::start1(true)`, portanto a varredura de volume amostra a cada 72 clocks YM2413, uma vez por rotação serial completa de 18 slots.
- O caminho alternativo `startCapture72()` envia `'D'`; `tool2/event.cc` trata `'D'` chamando `adc0::start0(true)`, amostrando a cada clock YM2413. Esse caminho não é o usado por `volumes.cc`.

Assim, as capturas de volume existentes não resolvem individualmente os 18 slots nem os quatro clocks de cada slot. Elas medem o sinal diferencial analógico RO−MO uma vez por rotação. A fase, porém, não é arbitrária: `tool2/main.cc` chama `ym::syncTimer1()` no boot. Essa rotina usa Timer0 para capturar 72 amostras consecutivas, encontra o pulso de três clocks de MO do canal musical 1 e redefine Timer1 para zero no clock central desse pulso. Como Timer1 transborda a cada 72 clocks e `startCapture()` usa esse trigger, cada amostra da varredura/calibração fica alinhada ao mesmo clock central do pulso MO do canal 1. Isso ancora uma posição de MO; não mede as outras 17 posições, os pulsos RO nem as bordas/subclocks. Como a entrada ADC é RO−MO, o valor também não é uma leitura isolada do pino MO.

O utilitário `analyze/ym-histogram.cc` apenas consome valores já quantizados no intervalo 0..511 e calcula um histograma. A transformação ADC→índice está em `analyze/calibrate.cc` e `analyze/decode.cc`. Portanto, os arquivos `.out` são índices de níveis analógicos calibrados, não captura bruta dos 9 bits do DAC.

**Próximo passo de FM-5.4:** reproduzir a calibração ADC→bins→índices e usar o ponto de fase já ancorado no pulso MO do canal 1 para validar a cadeia de medição. Em paralelo, investigar `startCapture72()` (Timer0/clock YM2413) para observar a sequência dos 72 clocks, validando primeiro overflow zero, contagem de amostras e DMA. A captura Timer0 ainda não resolve eventos subclock nem separa os pinos, pois a entrada continua diferencial RO−MO. A correlação entre nível analógico e palavra digital interna exige evidência separada.

### Algoritmo de calibração ADC→bin encontrado

A revisão do código de análise encontrou a transformação que gera os índices `.out`:

1. `analyze/calibrate.cc` executa o patch senoidal, chama `startCapture()` e coleta por padrão 60.000 amostras ADC.
2. `createBins()` grava `.raw`, constrói histograma de 65.536 valores ADC, aplica filtro triangular de largura 30, detecta picos com ajuste parabólico e insere um pico quando a distância entre picos excede 160 unidades.
3. `intBoundaries()` cria os limites inteiros entre picos consecutivos e grava `.bins`; o programa espera 512 bins.
4. `analyze/decode.cc` usa `lower_bound` sobre esses limites para transformar cada amostra ADC em um índice de bin.
5. `analyze/ym-histogram.cc` apenas calcula o histograma dos índices já decodificados.

Portanto, os índices 0..511 são ranks/intervalos da calibração de tensão ADC diferencial. Não são os bits internos do DAC, nem uma prova de que a ordem de bins seja idêntica à codificação do operador.

Os arquivos de calibração gerados (`.bins`, `.histogram`, `.filtered`, `.peaks`) não estão presentes no tree fixado do repositório; o algoritmo está versionado, mas os limites numéricos usados para gerar os `.out` não são reproduzíveis apenas com os arquivos atualmente commitados. Uma reprodução independente exige executar a calibração no hardware ou recuperar os artefatos originais correspondentes.

### Risco de throughput no modo Timer0

O trigger Timer0 está documentado como solicitação de amostra por clock YM2413, mas isso não prova que o ADC complete uma conversão por trigger. O driver usa oversampling 16x e mantém contador/flag de overflow. Antes de tratar `startCapture72()` como uma série válida de 72 amostras por rotação, a execução deve demonstrar:

- contador de overflow igual a zero;
- número de amostras efetivamente entregues igual ao número de triggers esperados;
- ausência de perda/reordenação no DMA;
- fase de Timer0 conhecida em relação ao clock YM2413 e ao slot serial.

Sem essas verificações, Timer0 é apenas uma cadência solicitada, não uma medição temporal certificada.

### Verificação exploratória de `volume0.raw` versus `volume0.out`

Uma comparação dos dois arquivos commitados encontrou:

- `volume0.raw`: 20.032 amostras inteiras ADC.
- `volume0.out`: 20.030 índices de bins.
- Melhor alinhamento exploratório: diferença de dois índices entre as sequências; correlação de Pearson ADC bruto↔índice decodificado `r = 0,999917`.
- A sequência observada cobre 437 bins distintos, com índices no intervalo 0..511.
- As médias ADC dos bins observados são monotônicas em 436 de 436 pares adjacentes.

Isso é consistente com `decode.cc` classificando a amostra ADC em intervalos analógicos ordenados. Não é uma comparação com o código interno do DAC. A diferença de dois índices entre arquivos ainda não tem explicação temporal demonstrada e não deve ser usada para inferir latência ou fase de hardware.

### Clock do fixture de medição versus clock nominal

O fixture de reverse engineering não está operando na frequência nominal do Master System:

- `tool2/timer0.cc` configura Timer0 com `topValue = 48000 / 50`, usando HFPER de 48 MHz, resultando em período de 20 µs (50 kHz).
- O canal PWM de Timer0 é roteado ao pino PA0; `tool2/ym.cc` mapeia PA0 como XIN do YM2413.
- `ym::syncTimer1()` declara explicitamente que Timer0 avança uma vez por clock YM2413.

Portanto, as capturas de calibração/volume são feitas com o YM2413 operando a 50 kHz no fixture. Um pulso de três clocks dura aproximadamente 60 µs nesse setup; a 3,579545 MHz, duraria aproximadamente 0,84 µs.

A lógica digital e a sequência de slots podem continuar úteis para caracterização, mas não se pode assumir que os níveis analógicos capturados a 50 kHz representem a transferência analógica em clock nominal. Tempo de acomodação, carga e resposta do estágio de saída podem alterar a tensão medida. A validação de amplitude/DAC para o SMS exige repetição em clock nominal ou uma demonstração explícita de que a resposta analógica é independente da frequência nessa faixa.
