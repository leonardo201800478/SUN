# YM2413 — Matriz de taxas PG e feedback no domínio de amostras

**Data:** 2026-11-19  
**Branch:** `test/ym2413-sample-domain-frequency-feedback-matrix`  
**Base:** `63fb10e630b5d789e8c8887cd5cdd2cec588c988`  
**Escopo:** regressão interna do contrato sample-domain entre OperatorPair e PhaseGenerator.  
**Estado:** teste adicionado; aguardando CI. Não constitui nova medição física.

## 1. Objetivo

Ampliar a cobertura do atraso M→C caracterizado pela PR #1067 para além do vetor de baixa frequência publicado. A matriz verifica se o modelo mantém o uso da fase anterior do modulator quando os incrementos PG variam, o carrier possui incremento independente e o feedback percorre todos os valores válidos.

A matriz é uma extrapolação de teste de software. Não atribui os vetores adicionais a Andete, não os apresenta como medições de hardware e não converte atualizações de amostra em input clocks.

## 2. Vetores

O teste combina quatro configurações legais de FNUM/BLOCK/MUL do modulator:

| FNUM | BLOCK | MUL | Incremento PG calculado | MUL do carrier |
|---:|---:|---:|---:|---:|
| 0x040 | 0 | 0 | 32 | 1 |
| 0x080 | 2 | 4 | 2.048 | 5 |
| 0x1C0 | 6 | 1 | 28.672 | 2 |
| 0x1FF | 7 | 15 | 981.120 antes do wrap de 19 bits | 0 |

Para cada configuração são usados:
- FB de 0 a 7;
- histórico inicial [128, 64], com soma positiva;
- histórico inicial [-128, -64], com soma negativa;
- 512 atualizações de amostra.

Total: 4 × 8 × 2 = 64 cenários e 32.768 atualizações do par.

O PG do carrier recebe o mesmo FNUM/BLOCK, mas MUL distinto do modulator. Isso exercita a independência das fases fornecidas à API; não pretende afirmar uma atribuição de instrumento específica.

## 3. Oráculo do teste

Em cada atualização, o teste mantém um estado de referência separado para os dois outputs históricos e a fase anterior do modulator.

O offset esperado é:
- zero quando FB=0;
- soma dos dois outputs históricos deslocada aritmeticamente à direita por (8-FB), para FB diferente de zero.

A fase esperada do modulator é a fase salva da atualização anterior, ou a fase corrente na primeira chamada, somada ao offset e reduzida módulo 1.024.

A fase esperada do carrier é sua fase corrente independente somada a duas vezes o output atual do modulator, com wrap módulo 1.024.

Após cada avaliação, o oráculo desloca o histórico e salva a fase corrente do modulator para a iteração seguinte. Os PGs avançam independentemente pelos seus próprios multiplicadores, com o acumulador de 19 bits aplicando o wrap definido pelo componente.

## 4. O que o teste demonstra

Se aprovado, o teste demonstra no contrato das APIs atuais:
- consistência do atraso de uma atualização de amostra para vários incrementos PG;
- aritmética de feedback para FB=0..7 e históricos iniciais de sinais opostos;
- independência entre os índices de fase recebidos para modulator e carrier;
- continuidade do histórico ao longo de múltiplas atualizações;
- comportamento do wrap de 19 bits do PG e do wrap de 10 bits do endereço de fase.

## 5. O que não demonstra

O teste não demonstra:
- latência física M→C em XIN;
- latch interno ou subfase em que o modulator é calculado/consumido;
- relação com quatro clocks por slot ou rotação de 72 clocks;
- ordem temporal dos 18 slots;
- alinhamento físico dos PGs, EGs ou LFOs;
- waveform analógica, DAC, escala de saída ou PCM final;
- fase de reset físico do YM2413.

A correspondência do teste com o código é evidência de regressão do modelo, não validação independente de hardware. A identificação do pipeline físico continua dependente de captura sincronizada de XIN, MO e RO por input clock, com reset e escritas temporizadas e reproduzíveis.

## 6. Decisão

- Manter o atraso M→C de uma atualização no domínio de amostras, sem traduzi-lo para clocks.
- Usar esta matriz apenas como cobertura de regressão interna.
- Não alterar lógica de produção, Scheduler, rotação de slots, PG/EG ou PCM.
- Aguardar CI aplicável antes de integrar a caracterização.
