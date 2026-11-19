# YM2413 — Caracterização da relação temporal Modulator → Carrier

**Data:** 2026-11-19  
**Branch:** `test/ym2413-modulator-carrier-sample-latency`  
**Escopo:** relação entre o índice de fase usado pelo modulator e o índice corrente usado pelo carrier no modelo de síntese por amostra.  
**Estado:** evidência documental reconstituída; teste diferencial do modelo Rust em preparação. Não é uma medição física nova.

## 1. Resultado executivo

As notas públicas de Andete sustentam que um modelo de saída que reproduz as capturas usa o índice de fase anterior do modulator e o índice corrente do carrier. Para o vetor de baixa frequência FNUM=0x40, BLOCK=0, MUL=0, o acumulador de fase candidato avança 32 unidades por atualização de amostra em formato 10.9. Como apenas os dez bits superiores endereçam a tabela, o índice da tabela avança uma posição a cada 16 amostras.

A expressão publicada `(i - 1) / 16` representa, portanto, um atraso de **uma atualização de amostra** no endereço de fase do modulator. Ela não representa um atraso de uma entrada inteira da tabela, um slot, quatro input clocks ou qualquer outra quantidade física de clocks.

A evidência é de ajuste de um modelo aos sinais de saída capturados. Não é observação direta de um latch interno M→C.

## 2. Reconstituição do experimento de 2015-12-03

Fonte: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-03

Condições descritas pelo autor:

- FNUM low = `0x40`, BLOCK = 0.
- ML = 0 nos dois operadores.
- Modulator TL variado de 0 a 63.
- EG habilitado; AR=15, DR=0, SL=0, RR=15.
- KEY-OFF seguido por KEY-ON para iniciar o teste.
- Período escolhido: 16.384 amostras por ciclo (~3 Hz).
- A tabela de seno tem 1.024 entradas; cada entrada se repete por 16 amostras nesse vetor.

### Observações reportadas

1. As formas de onda repetiam-se entre execuções do mesmo teste.
2. O autor observou 15 amostras consecutivas com o mesmo valor, seguidas por uma amostra diferente, em vez de 16 valores constantes.
3. O autor propôs como explicação que modulator e carrier estariam defasados por uma etapa.
4. Um spike de uma amostra foi observado próximo ao início do ataque. O autor não conhecia sua causa e apenas sugeriu que poderia estar relacionado ao desalinhamento.
5. O próprio texto considera possível inversão de sinal no setup de medição e registra incerteza sobre a fase inicial e o comportamento de ATTACK.

### O que foi medido versus o que foi inferido

- **Medido pelo autor:** waveform de saída analógica sob as condições listadas, incluindo o padrão 15+1 e o spike de uma amostra.
- **Inferido pelo autor:** sincronismo provável dos phase counters, possível reset de fase no início do ataque e possível relação entre o spike e o desalinhamento.
- **Não medido diretamente:** valor de fase em cada latch interno, clock exato de cálculo do modulator, clock de consumo pelo carrier ou atraso M→C em XIN.

## 3. Reconstituição do ajuste de modelo

Na mesma página, o autor parte de um modelo por operador e ajusta a contribuição do modulator ao carrier:

- Para TL=0, o fator de modulação inicialmente ajustado é 16.
- Para TL diferente de zero, o modelo passa a usar o fator 32 na entrada do caminho EXP do modulator.
- O índice de fase é avaliado em resolução de 16 amostras por entrada da tabela.
- O modelo é então ajustado para usar `(i - 1) / 16` no modulator e `i / 16` no carrier.
- O atraso produz os spikes observados, mas o autor relata que ainda faltavam oscilações.
- A etapa seguinte mascara o bit menos significativo do resultado EXP do modulator (`& ~1`), após a qual o gráfico gerado passa a coincidir com as capturas dentro das ressalvas de ruído e discrepância de início.

A conclusão metodológica é importante: o atraso de uma amostra é uma parte de um **modelo de saída ajustado**, combinado com outras escolhas aritméticas. Não deve ser isolado como prova de uma transferência física específica.

## 4. Fonte vinculada como 2015-12-12: discrepância editorial

URL indicada no histórico de investigação:  
https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-12

O conteúdo atualmente servido nessa URL apresenta o título **“YM2413 Reverse engineering notes (2015-03-27)”**, com navegação anterior para a nota de 2015-12-03 e seguinte para 2015-12-16. Portanto, o slug da URL e o título exibido não concordam.

O conteúdo é a investigação de feedback. Ele apresenta o modelo:

```cpp
auto f = FB ? (p0 + p1) >> (8 - FB) : 0;
auto s = lookupSin((i - 1) / 16 + f);
auto m = lookupExp(s + 32 * TL) >> 1;
p1 = p0;
p0 = m;
auto s2 = lookupSin(i / 16 + 2 * m);
auto c = lookupExp(s2) >> 4;
```

O autor relata que o modelo reproduz as capturas, com ressalvas explícitas:

- ruído de medição;
- condições iniciais de ATTACK;
- valores iniciais desconhecidos dos dois outputs históricos `p0` e `p1`;
- divergência significativa possível no início, especialmente em FB alto, antes de os estados entrarem em sincronismo.

A URL é útil como evidência do modelo combinado de feedback e modulação, mas não deve ser citada como se o título/data da página estivesse confirmado como 2015-12-12.

## 5. Comparação com a implementação Rust

Arquivo: `native/ym2413/src/operator.rs`.

O estado `OperatorPair` contém:

- `feedback_history: [i16; 2]`, os dois outputs anteriores do modulator;
- `previous_modulator_phase_index: Option<u16]`, índice de fase do modulator da atualização de amostra anterior.

Na avaliação:

1. O feedback é calculado a partir dos dois outputs históricos, com caminho especial para FB=0.
2. O modulator usa o índice de fase salvo da chamada anterior, somado ao offset de feedback e com wrap de 10 bits.
3. O output do modulator desloca o histórico de feedback.
4. O índice de fase atual do modulator é salvo para a próxima atualização.
5. O carrier usa seu índice de fase corrente somado a duas vezes o output do modulator.

Isso corresponde estruturalmente à sequência de cálculo do modelo publicado. A implementação não presume que os PGs do modulator e carrier estejam alinhados; recebe os dois índices separadamente.

### Limite da correspondência

O estado salvo é o índice de 10 bits usado pela tabela, não uma captura de latches internos. A chamada de `evaluate` é definida por atualização de saída/amostra do modelo. Nenhum vínculo com `Ym2413Core::advance_input_clocks`, `slot_index` ou fase parcial 0..3 foi demonstrado.

Assim, o teste diferencial em Rust pode certificar a equivalência do **modelo por amostra**, não a conversão em input clocks nem o pipeline físico.

## 6. Vetor diferencial adotado

Para FNUM=0x40, BLOCK=0, MUL=0, PM desabilitado:

- `PhaseGenerator::phase_step(0x40, 0, 0, false, 0) = 32`.
- Acumulador de fase: 19 bits, 10.9.
- Índice de tabela: `phase >> 9`.
- Incremento de índice: uma posição a cada 16 atualizações de amostra.
- Período: 16.384 atualizações de amostra para completar 1.024 posições.

Para o índice inteiro de amostra `i`, o modelo usa:

- Modulator: `floor((i - 1) / 16)` para `i > 0`.
- Carrier: `floor(i / 16)`.
- Na primeira avaliação, o fallback Rust sem histórico usa o índice corrente; essa condição inicial é uma convenção explícita do modelo Rust, não uma conclusão sobre reset físico.

O teste em Rust verifica ao longo do período completo:

- a fase usada pelo modulator é a fase da atualização anterior;
- a fase do carrier é a fase corrente mais duas vezes o output do modulator;
- o histórico de fase avança de forma contínua;
- o acumulador de 19 bits completa o wrap após 16.384 atualizações.

O teste não valida waveform analógica, escala DAC, fase física após reset ou clock de transferência entre slots.

## 7. Conclusões classificadas

| Afirmação | Classe | Estado |
|---|---|---|
| 18 operadores são combinados em nove canais melódicos | Documentado | Confirmado pelo manual e notas |
| Captura de saída apresenta padrão 15 amostras iguais + 1 diferente no vetor descrito | Medido/publicado | Evidência experimental de Andete |
| Desalinhamento M/C de uma etapa explica o padrão | Inferência do autor | Hipótese plausível, não observação interna |
| `(i - 1) / 16` para modulator e `i / 16` para carrier reproduz os gráficos | Modelo ajustado a dados | Confirmado no escopo do vetor/modelo publicado |
| Dois outputs anteriores participam do feedback | Modelo corroborado por waveform e análise de die shot | Estrutura modelada; condições iniciais físicas permanecem incertas |
| O atraso equivale a um slot ou quatro input clocks | Não demonstrado | Proibido inferir |
| O atraso equivale a uma amostra de saída = 72 input clocks | Não demonstrado como latência interna | Não converter sem captura correlacionada |
| Clock/subfase em que M é calculado e consumido por C | Desconhecido | Bloqueado por evidência física |

## 8. Próxima evidência necessária

Para identificar a latência física, a captura deve correlacionar:

- XIN como referência temporal;
- MO e RO com resolução por input clock;
- reset reproduzível e condição de início registrada;
- FNUM/BLOCK/MUL/TL/FB fixos e documentados;
- escritas de registradores em offsets conhecidos;
- múltiplas repetições com mesma sequência;
- vetores de frequência que mudem a fase de tabela em cadências diferentes;
- captura de startup separada da região estacionária.

Uma captura apenas do áudio agregado não identifica o instante interno de cálculo/consumo. O objetivo é distinguir latência de modelo em amostras, atualização da tabela SIN e pipeline físico, sem pressupor equivalência.

## 9. Decisão

- Preservar o modelo Rust de atraso de uma atualização de amostra como comportamento do modelo de saída já caracterizado.
- Adicionar teste diferencial do vetor de 16.384 amostras.
- Não alterar a contagem de clocks, a rotação de slots ou o Scheduler.
- Não declarar M→C cycle-accurate.
- Manter o pipeline de 18 slots BLOCKED até evidência física XIN/MO/RO por input clock.

## Referências

- Andete, 2015-12-03: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-03
- URL vinculada como 2015-12-12, conteúdo atualmente intitulado 2015-03-27: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-12
- Yamaha YM2413 Application Manual: https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
