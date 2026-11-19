# YM2413 — auditoria dinâmica dos registradores do EG

**Data:** 2026-11-19  
**Escopo:** efeito dinâmico de AR, DR, RR, SL, EG-TYPE e SUS no componente `EnvelopeGenerator`. Não altera cadência do contador global, slots, Scheduler, PG, LFO, DAC ou PCM.

## Evidência de referência

Andete, nas notas de 2015-12-31, relata:

- AR, DR e RR alterados durante a reprodução passam a valer imediatamente quando o EG usa aquele parâmetro. “Imediatamente” não significa que o nível muda naquele instante: a nova taxa passa a aguardar o próximo gate correspondente do contador global.
- Alterar DR fora da fase DECAY não produz efeito até a entrada nessa fase.
- Alterar SL durante SUSTAIN não desloca o nível sustentado.
- Alterar SL durante DECAY muda a condição de parada. Se o alvo novo já foi ultrapassado, a envoltória continua até o limite inferior, comportamento compatível com igualdade em vez de comparação menor-ou-igual.
- Alterar SUS durante RELEASE altera imediatamente a taxa efetiva, como alterar RR.
- Alterar EG-TYPE durante SUSTAIN/primeiro RELEASE alterna entre nível parado (sustained) e decaimento por RR (percussive).
- Alterar EG-TYPE durante RELEASE também alterna a seleção de taxa do estado unificado de release.

Fonte primária desta etapa de engenharia reversa: [Andete — YM2413 Reverse Engineering Notes 2015-12-31](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31), seções “Some atypical transitions”, itens a–d.

As notas de 2015-12-24 estabelecem os níveis nominais SL=0..15 como 0, 8, 16, …, 120 unidades internas e descrevem a incerteza original entre igualdade e comparação por faixa: [Andete — Sustain Level and KSR](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-24).

## Inspeção da implementação

O componente recebe `EnvelopeParameters` por chamada de `tick`/`tick_with_events`; não mantém cópia latched de AR, DR, RR, SL, EG-TYPE ou SUS.

- ATTACK calcula a taxa a partir do AR recebido naquela atualização.
- DECAY calcula a taxa a partir do DR recebido naquela atualização.
- O estado SUSTAIN seleciona RR ou taxa zero com base no EG-TYPE recebido naquela atualização.
- O KEY-OFF do carrier seleciona o rate-base com os valores atuais de EG-TYPE, SUS e RR; a matriz SUS × EG-TYPE foi integrada pela PR #1055.
- O caminho de KEY-OFF do modulator permanece distinto e não recebe o segundo release do carrier.
- O comparador de SL é consultado somente no estado DECAY. Depois da entrada em SUSTAIN, mudanças posteriores de SL não reabrem DECAY nem movem o nível.

Isso é compatível com a semântica dinâmica descrita nas medições. Os testes novos caracterizam o comportamento do componente quando os parâmetros mudam entre chamadas e o contador fornecido alcança um gate que produz incremento.

## SL=1 e nível ímpar 9 — limite de evidência

A implementação atual usa:

`(level >> 3) == sustain_level`

Isso equivale a comparar os quatro bits superiores e ignorar os três bits inferiores. O teste `sustain_level_one_candidate_comparator_accepts_observed_odd_level_nine` caracteriza que SL=1 aceita o nível 9.

**Não se declara que o silício compara exatamente os quatro bits superiores.** Andete observou o nível 9 e propôs uma igualdade de largura reduzida, mas escreveu que as medições não distinguiam ignorar um bit inferior ou três. O teste é uma regressão da hipótese atualmente implementada, não uma nova prova física.

A observação de que mudar SL durante DECAY para um alvo já ultrapassado não interrompe a descida também favorece uma comparação de igualdade, mas não determina sozinha a largura do comparador.

## Cobertura adicionada

Onze testes focados:

1. AR alterado durante ATTACK.
2. DR alterado durante DECAY sem reiniciar a fase.
3. RR alterado durante o primeiro release do modulator em EG-TYPE percussive.
4. RR alterado durante KEY-OFF do carrier em EG-TYPE sustained/SUS=OFF.
5. EG-TYPE alternado entre SUSTAIN e primeiro RELEASE.
6. EG-TYPE alternado durante o segundo RELEASE do carrier.
7. SUS alternado durante o segundo RELEASE do carrier.
8. SL alterado durante DECAY para corresponder ao nível corrente.
9. SL alterado depois da entrada em SUSTAIN sem mover o nível.
10. SL alterado durante DECAY para um alvo já ultrapassado, preservando DECAY.\n11. SL=1 com nível ímpar 9, caracterizando a comparação candidata.

Os testes usam contadores explícitos escolhidos para distinguir taxa zero da nova taxa e para alcançar um gate do seletor. Eles não certificam a frequência absoluta nem a fase física do contador.

## Limites e próximos passos

- A API recebe um contador global externo; esta auditoria não demonstra que o chamador o incrementa exatamente uma vez por sample físico.
- A cadência aproximada de 49,7 kHz, a origem/fase do contador e sua relação com 72 input clocks/18 slots continuam pendentes de auditoria temporal independente.
- O instante de amostragem dos registradores dentro de uma rotação de slots não foi determinado.
- A largura exata do comparador SL permanece incerta (um ou três bits inferiores ignorados são compatíveis com a observação relatada).
- Não houve alteração de lógica de produção nesta etapa; somente testes de caracterização e documentação.

**Estado:** semântica dinâmica do componente caracterizada em testes de software; não é validação elétrica nem fechamento cycle-accurate do YM2413. FM-1 permanece PARCIAL.
