# YM2413 — Caracterização da fronteira lógica da rotação de 18 slots

**Data:** 2026-11-19  
**Escopo:** contrato lógico já implementado em `Ym2413Core`; não é uma especificação das subfases internas do silício.  
**Base:** PR #1064, merge commit `4bc964361da310e5550c0b4d4b747669266f1d65`.

## 1. Fatos que esta caracterização verifica

O core mantém:

- contador monotônico de input clocks desde o reset lógico;
- fase parcial de 0 a 3 clocks dentro do intervalo de slot;
- índice do slot corrente, de 0 a 17;
- contador de rotações completas;
- descritor do slot obtido de `YM2413_SLOT_MAP`.

A implementação atual avança o índice de slot ao completar quatro input clocks e conta uma rotação ao completar dezoito slots (72 input clocks). O descritor identifica somente a atribuição serial de canal/operador; não executa síntese.

## 2. Testes de caracterização

### 2.1 Estabilidade do descritor durante o intervalo do slot

O teste `slot_descriptor_changes_only_at_four_clock_boundaries_and_wraps_after_one_rotation` percorre a rotação completa clock a clock e verifica:

1. o descritor corresponde à entrada atual de `YM2413_SLOT_MAP`;
2. após o primeiro, segundo e terceiro clocks, o descritor não muda e a fase parcial reflete a quantidade acumulada;
3. no quarto clock, a fase parcial retorna a zero e o descritor passa ao próximo slot;
4. após o slot 18, o contador de rotação incrementa e o descritor retorna ao slot 1.

### 2.2 Snapshot/restore em todos os offsets da rotação

O teste `snapshot_restore_preserves_slot_boundary_state_at_every_clock_offset` cria snapshots após cada offset de 0 a 71 clocks e verifica a restauração exata de:

- descritor do próximo slot;
- fase parcial dentro do intervalo;
- número de rotações completas;
- total de input clocks decorridos.

## 3. O que não é inferido

A fase parcial 0–3 é apenas um contador de clocks decorridos dentro do intervalo lógico de quatro clocks. Não se atribui a esses valores qualquer evento físico, incluindo:

- leitura ou incremento de PG;
- tick ou commit de EG;
- SIN/EXP ou avaliação de operador;
- captura/consumo de feedback ou latência M→C;
- latch do DAC, seleção MO/RO ou largura do pulso;
- aceitação/commit interno de escrita de registrador.

A estabilidade do descritor não significa que o silício mantenha todos os dados internos estáticos durante esses clocks. Significa apenas que a API de consulta do core identifica o mesmo slot até a fronteira aritmética estabelecida.

## 4. Resultado e decisão

A caracterização fecha uma propriedade interna importante: avanço contínuo clock a clock, sequência serial de 18 slots, wrap de 72 clocks e restauração de snapshot são coerentes no modelo de clock atual.

Ela **não** fecha P1-01 a P1-10, não prova alinhamento absoluto com reset físico, não valida o contador global EG/LFO, não define o momento do output sample e não autoriza ligar `Ym2413SynthesisBank` ao core.

A próxima implementação do pipeline permanece bloqueada por evidência física com resolução por input clock. Capturas necessárias: XIN, MO e RO simultâneos, reset reproduzível e escritas de registradores em offsets controlados, com vetores repetidos e timestampados.

## Referências internas

- `native/ym2413/src/lib.rs`: `Ym2413Core::advance_input_clocks`, `next_slot_descriptor`, `export_state` e `restore_state`.
- `native/ym2413/src/slot.rs`: `YM2413_SLOT_MAP` e atribuição de Rhythm Mode.
- `docs/audits/ym2413-slot-pipeline-investigation-2026-11-18.md`: matriz temporal conservadora e questões bloqueadoras P1.
- Yamaha, *YM2413 Application Manual*, seção II-5/Tabela II-3: https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
