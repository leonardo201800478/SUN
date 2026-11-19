# YM2413 — proveniência das tabelas Log-Sine e EXP

**Data:** 2026-11-18  
**Escopo:** proveniência matemática e reprodutibilidade das duas tabelas de transformação usadas pelo candidato de operador.  
**Fora de escopo:** provar leitura direta da ROM física, validar toda a aritmética SIN/EXP, definir o código de entrada/saída do DAC ou estabelecer temporização por slot.

## Conclusão

As tabelas de 256 entradas em `native/ym2413/src/synthesis_tables.rs` correspondem, entrada por entrada, às curvas matemáticas publicadas por Andete nas notas de reverse engineering de 2015-04-09 e repetidas nos modelos de 2016-07-16 e 2017-01-26.

A proveniência é **uma reconstrução matemática apoiada por análise de die shots e comparação com medições de saída**. Não é um dump integral, diretamente lido, da ROM interna de um YM2413 físico. O texto de 2015 descreve a comparação entre regiões de die shots OPLL e OPL3 como evidência sugestiva, mas registra incerteza na interpretação de alguns bits/decoders.

## Fórmulas e domínios

Para cada índice i = 0, ..., 255:

- Log-Sine, escala fixa de 8 bits fracionários:

  `round(-log2(sin((i + 0.5) * pi / 512)) * 256)`

- EXP, mantissa em escala de 1024:

  `round((2^(i / 256) - 1) * 1024)`

A variante de 2015 escreve a fórmula EXP como `round(2^(i / 256) * 1024) - 1024`. A avaliação das duas expressões para os 256 índices produz os mesmos inteiros armazenados nesta revisão.

O deslocamento de meia amostra no Log-Sine evita avaliar `log2(sin(0))`, que é indefinido, e permite reconstruir os demais quadrantes por espelhamento e sinal. O hardware/software transforma o domínio logarítmico em amplitude linear através da tabela EXP.

## Cadeia de evidência

1. **Andete, 2015-04-09 — “sine and exp tables”.** O estudo descreve:
   - regiões de die shots do OPLL e OPL3 com estruturas e padrões ROM correspondentes;
   - tabelas de 256 entradas, Log-Sine entre 0 e 2137 e EXP entre 0 e 1018;
   - as fórmulas acima e a lógica de lookup;
   - comparação da forma de onda prevista com medições do chip: o autor relata correspondência exata para volumes 1–15 e atribui discrepâncias em volume 0 a ruído nas medições de maior amplitude.
   
   O mesmo estudo ressalva que a interpretação dos die shots não é inteiramente certa. A afirmação de ROM idêntica entre OPLL/OPL2/OPL3 é apresentada como inferência, não como leitura digital completa.

2. **Andete, 2016-07-16 — modelo parcial inicial.** Reutiliza as fórmulas Log-Sine e EXP e amplia a comparação experimental para o modelo de operador, incluindo modulator/carrier, feedback, volume e KSL. A página deixa explícito que envelope e AM ainda são níveis fixos, não geradores temporais completos.

3. **Andete, 2017-01-26 — modelo melhorado.** O fonte de referência `model5.cc` mantém as mesmas fórmulas de geração das tabelas. A publicação trata principalmente de refinamentos do envelope/ataque; não deve ser citada como a origem primária das tabelas.

## Auditoria reproduzível nesta revisão

Os testes em `synthesis_tables.rs` agora percorrem todas as 256 posições de cada tabela e comparam o valor armazenado com a avaliação da fórmula correspondente, arredondada ao inteiro mais próximo.

Uma verificação independente das 512 entradas, executada durante esta auditoria, encontrou **zero divergências** entre as tabelas de `main` e as fórmulas. Os testes usam ponto flutuante apenas em `cfg(test)`; o caminho de síntese em execução continua usando constantes inteiras e não ganha cálculo transcendental em runtime.

## O que esta auditoria não prova

- Não prova que cada valor foi lido diretamente da ROM física do YM2413.
- Não resolve os detalhes de compressão/decodificação interna da ROM sugeridos pela análise dos die shots.
- Não valida, por si só, os shifts, a representação one's-complement, os limites de atenuação ou a transformação entre o intermediário `i16` e o DAC.
- Não estabelece que o intermediário `-256` seja aceito ou emitido pelo DAC físico.
- Não caracteriza filtros, pinos MO/RO, pulso de três clocks, latch, slots ou alinhamento com o Scheduler.

Essas questões permanecem separadas. Em particular, a fronteira operador/DAC continua sob investigação na [Issue #1031](https://github.com/leonardo201800478/sega-emulator/issues/1031).

## Referências

- Andete, [YM2413 Reverse Engineering Notes 2015-04-09](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-04-09); fonte original: [results/20150409/tab.txt](https://github.com/andete/ym2413/blob/master/results/20150409/tab.txt).
- Andete, [YM2413 Reverse Engineering Notes 2016-07-16](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2016-07-16).
- Andete, [YM2413 Reverse Engineering Notes 2017-01-26](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2017-01-26); modelo: [results/20170126/model5.cc](https://github.com/andete/ym2413/blob/master/results/20170126/model5.cc).
- [Índice cronológico das notas de reverse engineering](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes).
