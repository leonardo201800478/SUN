# YM2413 — evento candidato de reset do PG na conclusão de DAMP

**Data:** 2026-11-19  
**Escopo:** explicitar a fronteira DAMP → ATTACK/DECAY no contrato entre EG e PG.  
**Status:** evento de software implementado; integração temporal/por slot continua pendente.

## Evidência

O modelo reverse-engineered de Andete em results/20170126/model5.cc executa a transição de DAMP no EG do carrier e, nessa fronteira:
- seleciona ATTACK ou DECAY quando AR=15;
- zera a fase PG do carrier;
- propaga a transição e o reset de fase ao modulator.

A nota results/20170126/attack.txt descreve a hipótese e os experimentos que sustentam esse comportamento, mas o próprio autor classifica o modelo como ainda necessitando de mais verificação. Portanto, este projeto trata o resultado como **modelo reverse-engineered/candidato**, não como prova de todos os offsets internos do chip.

Referências:
- https://github.com/andete/ym2413/blob/master/results/20170126/model5.cc
- https://github.com/andete/ym2413/blob/master/results/20170126/attack.txt

## Lacuna encontrada no código anterior

EnvelopeGenerator::complete_damp() alterava apenas o estágio do EG. O banco agregado mantinha os PGs separados e não recebia um evento explícito para representar o reset de fase descrito no modelo de referência. Assim, mesmo com a política CarrierDrivesBoth, a camada chamadora não tinha um sinal tipado e inequívoco para aplicar o reset dos PGs na fronteira correspondente.

## Alteração proposta nesta PR

- EnvelopeTickResult expõe level e phase_reset_required.
- EnvelopeGenerator::tick_with_events() informa o evento exatamente na atualização que conclui DAMP, inclusive quando AR=15 seleciona o atalho para DECAY.
- A API antiga tick() permanece compatível e continua retornando somente o nível.
- Ym2413SynthesisBank::tick_operator_envelope_with_events() propaga o evento sem escolher cadência, ordem de slots ou reset automático.
- reset_operator_phase() permite que o chamador consuma explicitamente o evento no limite temporal que controla.
- Sob CarrierDrivesBoth, o chamador ainda deve sincronizar o EG do modulator e resetar seu PG conforme a política selecionada.

## Limites preservados

Esta alteração não:
- cria um scheduler ou contador global;
- escolhe o clock/subfase em que o evento deve ser consumido;
- aplica automaticamente o reset de PG;
- altera o comportamento de reset do chip;
- afirma que CarrierDrivesBoth seja uma política física definitivamente comprovada;
- integra PCM, DAC, MO/RO ou runtime SMS.

## Validação adicionada

- evento emitido uma única vez ao concluir DAMP;
- evento emitido quando AR=15 pula ATTACK e entra em DECAY;
- ausência de evento enquanto o nível permanece abaixo do limiar;
- banco agregado propaga o evento sem reset implícito do PG;
- reset explícito do PG altera a fase para zero.

A aprovação final depende dos gates CI aplicáveis no HEAD desta PR. A existência do evento não fecha a integração temporal FM-6 nem certifica o pipeline YM2413.
