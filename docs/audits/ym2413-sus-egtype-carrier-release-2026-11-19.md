# YM2413 — auditoria SUS × EG-TYPE no KEY-OFF do carrier

**Data:** 2026-11-19
**Escopo:** seleção do rate-base no KEY-OFF do carrier; não altera a semântica KEY-OFF do modulator, o contador global ou a temporização do Scheduler.

## Decisão

| EG-TYPE | SUS | Rate-base no KEY-OFF do carrier |
|---|---:|---:|
| 0 — percussive | 0 | 7 |
| 0 — percussive | 1 | 5 |
| 1 — sustained | 0 | RR programado do operador |
| 1 — sustained | 1 | 5 |

Depois da seleção do rate-base, aplicar KSR: RATE = min(63, 4 × R + Rks). O caso R=0 continua desabilitando o incremento e não recebe offset KSR.

## Evidências e reconciliação

1. **Yamaha YM2413 Application Manual.** O mapa do registrador de canal identifica D5 como SUS-ON/OFF e declara que, quando ativo, o RR com KEY-OFF será 5. O manual também especifica RATE = 4 × R + Rks e que o rate-zero não avança. Fonte: [manual Yamaha](https://www.waveguide.se/?article=46&file=ym2413am.pdf).
2. **Andete, 2015-12-31 — medições ADSR.** A descrição do tom percussivo informa SUS=ON → 5 e SUS=OFF → 7. A descrição do tom sustentado informa SUS=ON → 5 e SUS=OFF → RR programado. A nota também afirma que KSR é aplicado às taxas fixas. Fonte: [YM2413 Reverse Engineering Notes 2015-12-31](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31).
3. **Andete, 2015-12-31 — transições dinâmicas.** Alterar SUS durante o estado de release altera imediatamente a taxa efetiva, como se RR tivesse sido alterado. Isso sustenta tratar SUS como seletor ativo, não apenas como valor amostrado na entrada do KEY-OFF.
4. **Andete, 2017-01-26 — model5.cc.** O modelo de software usa a expressão eg ? rr : ch.sustain ? 5 : 7 no caminho de release do carrier. Como EG=1 representa sustained tone e ch.sustain é o bit SUS, essa expressão ignora SUS para EG=1 e conflita com a descrição de medição de 2015 e com o manual. O arquivo é um modelo de software/reconstrução, não uma medição física independente que revogue as duas fontes anteriores: [model5.cc](https://github.com/andete/ym2413/blob/master/results/20170126/model5.cc).

### Limitação da fonte de 2015

O texto descritivo de 2015 distingue explicitamente os dois valores de SUS para sustained tone, mas o exemplo gráfico publicado não inclui o valor do registrador de canal que permitiria reconstruir de forma independente a configuração SUS daquele traço. A decisão, portanto, é sustentada pela regra textual de medição e pela especificação Yamaha, não por uma captura isolada SUS=ON anexada ao repositório.

A máquina de estados em pseudocódigo no final do artigo de 2015 contém uma condição internamente inconsistente (EG_TYPE == 0 envolvendo um ternário que também testa EG_TYPE == 1). Ela é tratada como pseudocódigo preliminar, não como regra que prevalece sobre a descrição experimental anterior.

## Modulator — caminho separado

- EG-TYPE=1: o modulator não entra no release secundário do carrier; mantém seu comportamento de sustain observado.
- EG-TYPE=0: o modulator continua no primeiro release RR.
- Não aplicar o seletor SUS→5/7 do carrier ao modulator.
- Esta correção não modifica o caminho do modulator.

## Implementação e cobertura

A função interna carrier_key_off_base_rate() seleciona a matriz acima. O caminho KEY-OFF do carrier aplica em seguida effective_rate(), preservando o KSR e o tratamento especial de R=0.

Testes adicionados:
- matriz completa EG-TYPE × SUS;
- matriz de RATE efetivo com KSR desligado/ligado;
- regressão funcional para EG-TYPE=1, SUS=ON e RR=0, demonstrando que o rate-base 5 continua ativo;
- testes anteriores para EG-TYPE=0/SUS=ON e EG-TYPE=0/SUS=OFF permanecem.

## Limites

Esta decisão fecha a seleção lógica do rate-base segundo as fontes acima. Não prova a fase absoluta do contador EG, o instante físico de amostragem de SUS, o deslocamento de slot, nem o pipeline de 18 operadores. Não altera Scheduler, clocks, PG, DAC, PCM ou integração SMS.

**Status:** decisão de software baseada em documentação Yamaha e notas de medições físicas; aguardando CI da alteração. Não declarar YM2413 concluído ou cycle-accurate.
