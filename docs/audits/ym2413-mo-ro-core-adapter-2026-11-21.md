# YM2413 — Auditoria da integração do adaptador MO/RO ao PG serial

**Data:** 2026-11-21  
**Repositório:** `leonardo201800478/SUN`  
**PR:** [#1159 — measured MO/RO core adapter](https://github.com/leonardo201800478/SUN/pull/1159)  
**Merge commit:** `eb01ca8a7ee6224a98e872143b41f0b28137b292`  
**Estado:** integração temporal/proveniência publicada; níveis medidos, palavra DAC e PCM continuam bloqueados.

## 1. Escopo efetivamente integrado

A PR #1159 integra o adaptador de saída medido ao serviço PG serial sem substituir o pipeline de referência Nuked e sem criar uma segunda autoridade de ciclo.

- MO e RO mantêm coordenadas independentes.
- O adaptador mantém buffer de saída com atraso de um ciclo e registra separadamente ciclo de origem e ciclo de emissão.
- O ciclo esperado do adaptador é derivado do cursor RO; não há um segundo cursor serial independente.
- O PG serial registra a procedência do latch anterior em `previous_channel_output_is_rhythm`.
- Essa procedência é derivada do dispatch serial anterior efetivamente concluído: `Rhythm` marca a próxima amostra como originada de percussão; `Melodic`, transição e origem desconhecida não a marcam.
- A procedência é persistida no snapshot PG, versão 12.
- O estado do adaptador é versão 2 e restaura o ciclo esperado a partir do cursor RO.
- A restauração exige alinhamento dos cursores no limite de slot concluído.

A ordem no limite do slot é importante: o adaptador observa a procedência do latch anterior antes de o pipeline de referência instalar o próximo `ch_out`. O buffer de um ciclo do adaptador é uma fronteira adicional; não substitui nem duplica o latch anterior.

## 2. Limite de evidência: níveis MO/RO

O adaptador recebe `Option<Ym2413DacCode>`, isto é, um nível explícito do domínio de medição. Ele não converte o `i16` intermediário do operador em `Ym2413DacCode`.

Na integração PG atual, o nível fornecido é deliberadamente `None`. A conversão candidata `Ym2413DacCode::from_operator_channel_output` não é usada no caminho integrado porque ainda não existe evidência independente suficiente para estabelecer a correspondência entre o latch do operador e os níveis medidos.

Consequências deliberadas:

- Slots MO ativos sem nível medido permanecem `mo_known = false`; não são convertidos em zero.
- Slots RO originais sem nível/proveniência válidos resultam em `UnknownSource`.
- Repetições sem retenção inicializada resultam em `RepeatNotInitialized`.
- Fora de Rhythm Mode, RO resulta em `ModeInactive`, não em uma afirmação de silêncio elétrico.
- `NoOutput` permanece reservado às posições em que o modelo de roteamento não agenda um pulso.

**Esta PR não implementa nem comprova:** palavra digital bruta de 9 bits, codificação/sinal do DAC, tensão analógica, fase subclock, PCM, mixer PSG+FM ou saída física CPAL.

## 3. Preservação do pipeline Nuked

O pipeline de referência Nuked permanece separado e inalterado semanticamente:

- Continua consumindo o `rm_enable` assinado pertencente ao core.
- Não desloca `rm_enable` novamente.
- Mantém sua própria validação diferencial de software.
- Não é usado como substituto do serviço medido MO/RO.

A saída medida não herda os resultados do Nuked como prova física.

## 4. Testes direcionados adicionados

Os testes cobrem:

- `measured_adapter_buffers_previous_latch_and_rejects_melodic_source_as_ro`: atraso de um ciclo, ciclo de origem/emissão e rejeição de origem melódica para RO.
- `rhythm_transition_does_not_reinterpret_a_melodic_latch_as_ro_source`: transição de modo sem reaproveitar latch melódico como percussão.
- `unknown_rhythm_source_invalidates_the_retained_value_before_repeat`: invalidação da origem e da retenção repetida.
- `measured_adapter_snapshot_restore_is_block_equivalent_across_rotation`: equivalência entre execução contínua e em blocos, com snapshot/restore.
- `measured_adapter_restore_derives_core_cycle_from_ro_cursor`: cursor derivado do RO, sem cursor duplicado.
- `rhythm_owned_slots_use_seeded_rhythm_operator_path_and_reach_do_rhythm`: procedência Rhythm persistida e restauração em limite de slot concluído.
- `rhythm_mode_and_selector_transitions_do_not_reassign_melodic_latches`: transições e seletores não reclassificam indevidamente slots melódicos.

Os testes do serviço de baixo nível também exercitam níveis medidos explícitos, retenção, `-0`/`+0`, coordenadas MO/RO e estados desconhecidos. Esses fixtures explícitos não constituem uma conversão física do operador.

## 5. Evidências CI da PR #1159

Todos os workflows aplicáveis da PR concluíram com sucesso no HEAD validado `497372f16c042fb555b3fb3820837e9d2914c426`.

| Workflow | Run | Resultado | Interpretação |
|---|---:|---|---|
| Rust workspace / quality / unit regression | `44939611035` | PASS | Rustfmt, Clippy e testes aplicáveis aprovados |
| YM2413 PG differential | `44939611011` | PASS | Execução da caracterização diferencial concluída; divergências abaixo permanecem |
| EG characterization | `44939611021` | PASS | Execução da caracterização concluída; não significa fechamento semântico |
| Output pipeline differential | `44939611027` | PASS | 1.200 chamadas OPLL_Clock; pipeline de referência sem divergência |
| Rhythm reset differential | `44939611036` | PASS | 144 chamadas; 16 decisões de reset de fase verificadas |

### 5.1 PG/PM — caracterização, não certificação física

- PG: 65.536 tuplas-base; zero divergências após normalização por floor; 256 diferenças exatas de unidade comum/meio LSB.
- PM: 524.288 tuplas; 404.965 divergências de floor na comparação sem rotação.
- Melhor alinhamento cíclico observado: deslocamento 6; ainda restam 135.136 divergências de floor.
- O workflow PASS significa que a comparação e seus relatórios foram executados com sucesso. Não significa igualdade física nem fechamento de PG/PM.

### 5.2 EG — caracterização ainda aberta

- Matriz de 38 configurações: 243.884 amostras, 44.064 divergências de nível e 71 divergências de estágio no candidato caracterizado.
- Replay de operador: 506.768 amostras sem divergências de nível/estágio.
- Replay de sequenciador: 1.140.000 chamadas sem divergências de campos.
- Replay de timer: 1.140.000 chamadas sem divergências de campos.
- A fase externa caracterizada depende de latches/estados extraídos do Nuked e não é um oracle físico independente.

### 5.3 Pipeline de referência e reset

- Output pipeline: 1.200 chamadas OPLL_Clock; 181 saídas `ch_out` não nulas; escritas qualificadas `0x3f` e `0x00`; sem divergências na referência Nuked.
- Rhythm reset: 144 chamadas; 16 decisões de reset de fase; resultado PASS do diferencial de software.

## 6. Limitações e próximo gate

Permanecem abertos:

1. Correspondência independente entre o intermediário do operador e os 512 níveis medidos.
2. Palavra digital bruta do DAC, largura, codificação e sinal.
3. Fase/janela subclock da saída.
4. Conversão digital para PCM e relação exata com o master clock.
5. Acumulação dos nove canais.
6. Integração temporal FM no runtime SMS.
7. Mistura PSG+FM e integração host CPAL.
8. Seed físico e ROM de patches de percussão.
9. Fechamento semântico de PG/PM e EG.

O próximo gate deve investigar as capturas físicas originais e a documentação de aplicação Yamaha. As capturas disponíveis são amostras ADC da saída analógica; não devem ser tratadas como captura direta da palavra digital interna do DAC. Não habilitar conversão de produção até que a aritmética e a fronteira temporal sejam demonstradas independentemente.

## 7. Referências

- [PR #1154 — FM-5.2A](https://github.com/leonardo201800478/SUN/pull/1154)
- [PR #1156 — FM-5.2B](https://github.com/leonardo201800478/SUN/pull/1156)
- [PR #1157 — FM-5.3 measured MO/RO](https://github.com/leonardo201800478/SUN/pull/1157)
- [PR #1158 — Nuked MO/RO reference pipeline](https://github.com/leonardo201800478/SUN/pull/1158)
- [PR #1159 — measured MO/RO core adapter](https://github.com/leonardo201800478/SUN/pull/1159)
- `docs/audits/ym2413-dac-output-evidence-2026-11-21.md`
- `docs/audits/ym2413-mo-ro-cycle-alignment-2026-11-21.md`
- `docs/audits/ym2413-mo-ro-output-pipeline-2026-11-21.md`
