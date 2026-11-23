# P0 — Alocação por dispatch: baseline, alteração e medição

**Plano de referência:** [`temporal-core-optimization-plan.md`](temporal-core-optimization-plan.md) §4.1, §4.7 e §7 (P0).
**Data:** 2026-11-23
**Base:** `main` em `207ccc4`.
**Escopo:** um único item (P0). Nenhuma outra otimização foi aplicada.

---

## 1. Hipótese

`calculate_advances()` cria um `Vec::with_capacity(self.clocks.len())` por dispatch e `MachineDispatch` mantém esse `Vec`. Com 49.780 dispatches por frame, isso significa 49.780 alocações e liberações de heap por frame apenas para transportar 3 tokens de 192 B. Hipótese: eliminar essa alocação reduz o custo do dispatch e o jitter, sem alterar sequência, estado ou resultado temporal.

## 2. Método (§4.7 — metodologia corrigida)

A decomposição da auditoria anterior misturava fronteiras de medição: uma janela instrumentada em volta de `dispatch_next` era comparada com um microbenchmark que usava um tipo menor e pulava parte do trabalho de produção. O harness novo (`tools/temporal_bench`) corrige isso:

**Fronteira efetivamente medida:** cada iteração executa `schedule_at_with_phase()` e, em seguida, `dispatch_next()` ou `dispatch_next_into()`. Portanto, os tempos reportados são de uma operação combinada de agendamento + dispatch, não de dispatch isolado. A região também inclui `next_timestamp()`, contabilidade do orçamento por timestamp, conversão racional por domínio, validação e commit dos avanços, `pop_next`, `trace!` de dispatch e construção/consumo do resultado.

**Excluído:** trabalho dos consumidores (`sms_core`, VDP e PSG), que pertence às auditorias específicas desses subsistemas.

A comparação A/B continua válida para essa operação combinada porque as variantes usam a mesma fronteira. Não atribuir todo o tempo medido exclusivamente ao dispatch.

**Tipo real:** `ClockAdvance` de 192 B (confirmado pelo harness: 576 B para 3 domínios, 768 B para 4).

**Contagem de alocações:** alocador global instrumentado, contando `alloc`, `realloc` e bytes solicitados.

**Plataforma:** VM Neoverse-N1 (ARM64, 6 vCPU), Rust 1.99.1, `--release`, thin LTO, `master_ticks_per_tstate = 3`, ROM sintética.

## 3. Baseline

Uma operação de agendamento + dispatch (caminho histórico com `Vec`), mediana de 3 execuções de 200.000 operações:

| Cenário | ns/op (schedule + dispatch) | alocações/op | bytes/op | projeção por frame |
|---|---:|---:|---:|---:|
| 3 domínios (SMS) | **202,2** | **1,000** | 576 | **10,06 ms** |
| 9 domínios | 570,5 | 1,000 | 1.728 | — |

## 4. Variantes avaliadas

| Variante | ns/op (schedule + dispatch) (3 dom.) | alocações | Veredito |
|---|---:|---:|---|
| A. `Vec` por dispatch (baseline) | 202,2 | 1,000 | referência |
| B. Inline por valor (`ClockAdvances` de 768 B retornado em `MachineDispatch`) | **306,0** | 0,000 | **rejeitada**: +51% de latência |
| C. Buffer do chamador (`dispatch_next_into`) | **184,0** | **0,000** | **adotada**: −9,0% e zero alocação |

A variante B é o resultado que o plano §4.1 pedia para avaliar, e a medição a rejeita: com o token real de 192 B, mover 768 B por valor em cada dispatch custa mais do que o `malloc` que ela elimina. Também foi medida uma variante intermediária (corpo compartilhado por um *sink* genérico, sem buffer do chamador): 235 ns no caminho antigo (+16%), também rejeitada. Por isso os dois pontos de entrada mantêm corpos separados e compartilham apenas `begin_dispatch`, `commit_dispatch`, `apply_advances` e a conversão racional — a duplicação é deliberada e está documentada no código com o número que a justifica.

## 5. Alteração aplicada

- `native/machine_scheduler/src/advances.rs` (novo): `ClockAdvances`, buffer com 4 slots inline (`MaybeUninit`, sem drop glue — garantido por `assert!(!needs_drop::<ClockAdvance>())`) e fallback explícito para o heap quando a máquina registra mais domínios. Nunca trunca, nunca reordena. `Deref<Target = [ClockAdvance]>` mantém indexação, `len()` e iteração idênticas ao `Vec`.
- `MachineScheduler::dispatch_next_into(&mut self, &mut ClockAdvances)`: caminho sem alocação. Em `Ok(None)` ou erro, o buffer é esvaziado (nunca expõe avanços obsoletos).
- `MachineScheduler::dispatch_next()`: mantido com o corpo e o payload `Vec` originais. **Sem regressão** no caminho legado (202,4 ns/op de agendamento + dispatch, medidos depois da mudança).
- `SmsTemporalBus` passou a usar o buffer reutilizável no laço quente (`temporal_bus.rs`), com um campo transiente, fora dos save states.

## 6. Resultado

### 6.1 Operação combinada: agendamento + dispatch

| Caminho | ns/op (schedule + dispatch) | alocações/frame (49.780 operações) |
|---|---:|---:|
| `dispatch_next` (antes e depois) | 202,4 | 49.780 |
| `dispatch_next_into` (novo) | **184,0** | **0** |

### 6.2 Frame completo da máquina SMS (timing desligado, mediana de 3)

| Cenário | Antes | Depois | Δ |
|---|---:|---:|---:|
| `relative-loop` (`jr $`) | 15.782,9 µs | 15.460,6 µs | **−322 µs (−2,0%)** |
| bus-heavy | 13.490,3 µs | 13.164,2 µs | **−326 µs (−2,4%)** |
| read-heavy | 16.194,5 µs | 15.663,7 µs | **−531 µs (−3,3%)** |
| port-heavy | 13.773,5 µs | 13.637,0 µs | **−137 µs (−1,0%)** |

Nenhum cenário regrediu. Os tempos desta seção são da operação combinada de agendamento + dispatch. O ganho end-to-end da máquina é menor que o ganho observado nessa operação combinada medida pelo harness; não inferimos, com os dados atuais, a participação percentual exclusiva do Scheduler no tempo total do frame.

### 6.3 Validação

- `sega-machine-scheduler`: 44 unitários + 2 + 2 + 1 de integração + **3 novos** de equivalência (`dispatch_into_equivalence.rs`).
- `sega-sms-core`: 188. `sega-sg1000-core`: 79. `sega-temporal-core`: 41. **Total 715 testes, 0 falhas.**
- `cargo fmt --all -- --check` limpo; `cargo clippy` limpo em `machine_scheduler` e `sms_core` (todos os alvos).
- O teste de equivalência compara 400 dispatches com razões mistas e 4 domínios: mesma sequência de eventos, mesmos avanços, mesmo `export_state()` — ou seja, save states e replays continuam idênticos entre os dois caminhos.

## 7. Limitações

- Medições em ARM64 (Neoverse-N1) apenas. Windows x64 (alvo do Vulkan inicial) não foi medido; a ordem de grandeza do `malloc` pode diferir.
- ROMs sintéticas. O perfil de eventos (5 dispatches por fetch) é estrutural, mas os tempos absolutos variam com o jogo.
- O ganho de jitter/p99 não foi medido; só a média. A remoção de 49.780 alocações por frame deve reduzir pausas do alocador, mas isso é expectativa, não resultado.
- `advance_idle_to` continua devolvendo `Vec` (caminho frio, não migrado neste item).

## 8. Próximo item

Pelo §7 do plano, o próximo é **P1 — compartilhar conversões equivalentes**, cuja condição de início ("equivalência dos estados de origem demonstrada") já foi investigada na auditoria: os 3–4 domínios do SMS têm a mesma razão e o mesmo estado de origem, e a conversão é refeita 3–4 vezes por evento. O harness já está no repositório para medir antes/depois.
