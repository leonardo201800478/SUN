# E7 — matriz consolidada dos 12 critérios temporais e de integração

**Data:** 2026-11-16
**Main de referência:** `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`
**HEAD de validação complementar:** PR #972, `ff922996e797b287e275fdbf1d1791314c61a749`
**Perfil:** Zilog Z80 NMOS clássico delimitado aos consumidores Sega SMS/SG-1000.

## Escopo e regra de leitura

Esta é a consolidação dos 12 critérios temporais e de integração solicitados na continuidade E7. O checklist mestre do repositório não contém uma lista autônoma numerada de exatamente 12 itens; seus requisitos estão distribuídos pelos grupos F-, S-, T-, D-, V- e C-. Esta matriz agrupa os itens temporais expressamente enumerados na continuidade, sem substituir o checklist mestre.

**Workflow da validação complementar:** [CI #43918490165](https://github.com/leonardo201800478/sega-emulator/actions/runs/43918490165) — Rust workspace regression SUCCESS. O executável `snapshot_restore_e7_2e` registrou **13 passed, 0 failed**. [SMS Debugger Runtime #43918490096](https://github.com/leonardo201800478/sega-emulator/actions/runs/43918490096) — SUCCESS. Os jobs Z80 focused e Scheduler deterministic stress foram SKIPPED nessa execução e permanecem NOT RUN nela.

| # | Critério | Arquivos/símbolos de evidência | Resultado de software | Limite / classificação |
|---:|---|---|---|---|
| 1 | HALT e ciclos M1 de retenção | `native/sms_core/tests/snapshot_restore_e7_2e.rs::e7_2e_halt_m1_hold_restores_each_tstate_and_ignores_data`; `native/z80_cpu/tests/halt_e7.rs` | PASS direcionado; incluído na suíte workspace do HEAD #972 | Não extrapolar para sinalização elétrica externa não modelada |
| 2 | INT/NMI, amostragem e prioridade | `native/sms_core/tests/snapshot_restore_e7_2e.rs::e7_2e_nmi_restore_matches_trace_with_zero_and_multiple_wait_states`; `e7_f022_repeat_interrupt_is_sampled_and_restored_by_sms_scheduler`; `e7_f022_repeat_im2_handler_observes_flags_and_restores_through_scheduler`; `e7_f022_repeat_nmi_is_sampled_and_restored_by_sms_scheduler` | PASS direcionado para os cenários testados; INT IM1/IM2 e NMI durante repetição cobertos no SMS Scheduler | Não significa cobertura de toda combinação possível de bordas/níveis ou todos os dispositivos |
| 3 | Modos IM0/IM1/IM2 | `snapshot_restore_e7_2e.rs::e7_2e_im0_restores_acknowledge_and_injected_rst`; `e7_2e_im1_restores_dispatch_and_both_stack_writes`; `e7_2e_im2_restores_stack_and_both_vector_reads`; novo teste IM2 e NMI durante repetição | PASS direcionado para os vetores e handlers cobertos | Não é validação de variantes CMOS/R800/Z180 |
| 4 | EI delay e retorno RETI | `snapshot_restore_e7_2e.rs::e7_2e_ei_delay_survives_machine_snapshot_until_following_instruction`; handler IM2 da PR #972 | PASS direcionado: EI delay persiste por snapshot e RETI permite a retomada | Não generalizar para daisy-chain; SMS/SG-1000 não modelam periférico RETI daisy-chain |
| 5 | Interrupt acknowledge | `snapshot_restore_e7_2e.rs::e7_2e_wait_matrix_covers_m1_memory_io_internal_and_interrupt_cycles`; testes IM0/IM1/IM2 | PASS no contrato de software testado | Vetor/acknowledge e tempos nominais não constituem medição de pinos |
| 6 | WAIT | Matriz WAIT em `snapshot_restore_e7_2e.rs`; F-022 simple/repeat e IM1/IM2 com WAIT por request | PASS para WAIT determinístico de fixture | WAIT físico/externo: NOT_VERIFIED |
| 7 | M-cycles/T-states | `Z80MicroSequencer`; `snapshot_restore_e7_2e.rs`; matrizes temporais Z80 em `native/z80_cpu/tests` | PASS para a sequência nominal e durações cobertas | Não alegar precisão elétrica/sub-T-state ou timing de dispositivo ainda não integrado |
| 8 | Operações de barramento e ordem dos requests | Trace de `SmsTemporalMachine`; WAIT matrix; requests M1, memória, I/O, acknowledge e ciclos internos | PASS nas classes de request cobertas | DMA/arbitragem externa e operações de dispositivos não presentes no perfil: NOT_VERIFIED/NOT_APPLICABLE conforme escopo |
| 9 | Suspensão e retomada | `run_boundary`, `compare_runs`, `run_boundary_with_repeat_interrupt`, `compare_repeat_interrupt_runs` em `snapshot_restore_e7_2e.rs` | PASS nos safe points de request e ciclo interno exercitados | Não equivale a provar todos os estados possíveis de cada dispositivo |
| 10 | Snapshot/restore | helper `snapshot` em `snapshot_restore_e7_2e.rs`; encode/decode de `Z80MicroSequencerState`; comparações de trace/estado/timestamp | PASS; a suíte de integração registrou 13/13 no workspace Release do HEAD #972 | A PR #972 foi integrada em `1b16256358e3f17837a95cc071048c5ee5307f82`; o 13/13 foi executado no HEAD validado da PR. No push pós-merge, Rust workspace PASS, mas Z80 focused e Scheduler stress foram SKIPPED/NOT RUN |
| 11 | Ordem causal e determinismo do Scheduler | LONG workflow #43901362527, SHA `b60f08eeab7b848727661e64a6c7eed671c2fcfe`; 16 seeds, 320.000 operações e 4.000.000 dispatches | LONG_PASS de software | A campanha não prova WAIT físico, DMA, safe points de todos os dispositivos ou TEMPORAL_PASS global; stress foi SKIPPED no #972 |
| 12 | Integração SMS/SG-1000 | SMS: `SmsTemporalMachine`/MachineScheduler e testes desta matriz. SG-1000: `native/sg1000_core/src/cpu.rs` reexporta `sega_z80_cpu`; `machine.rs` usa stepping por instrução; `temporal.rs`/`temporal_bus.rs` são primitives ainda não conectadas à máquina | SMS: PASS direcionado. SG-1000: integração do Z80 compartilhado PASS; integração da máquina SG-1000 ao Scheduler PARTIAL/deferida para P2 | O fechamento E7 do core compartilhado não deve ser confundido com conclusão do milestone standalone SG-1000 |

## Síntese de aceitação

- Critérios 1–5: PASS direcionado no contrato de software e nos cenários citados.
- Critérios 6–10: PASS nas dimensões de fixture, requests e safe points efetivamente exercitadas; WAIT físico e cobertura universal de dispositivos continuam NOT_VERIFIED.
- Critério 11: LONG_PASS do Scheduler como software; não é validação física global.
- Critério 12: SMS temporal integrado; SG-1000 já consome o Z80 compartilhado, mas sua máquina ainda não foi migrada para o caminho Scheduler.

## Bloqueios E7 que permanecem fora desta matriz resumida

1. F-022: revisar integralmente as oito formas e a proveniência/oráculos, embora a integração IM2 com PUSH AF/POP AF/EI/RETI no Scheduler agora tenha teste e CI PASS na PR #972.
2. F-023: concluir revisão adversarial da matriz por família e da classificação DOC/HW/COMP/INT; somente RRD/RLD tem evidência HW específica para WZ=HL+1.
3. F-028: revisão adversarial final da matriz/exceções e limites de equivalência interna; a ausência da matriz dos 167 Ignored foi resolvida por #950/#953.
4. Obter revisão técnica independente registrada e executar os gates aplicáveis no candidato final e na main pós-merge.

**Decisão:** E7 permanece IN_PROGRESS. E8/ZEXDOC/ZEXALL permanece BLOCKED. A matriz não libera E8 por si só.


## Equivalência de código de produção entre os SHAs de validação

A comparação de blobs confirma que o código de produção Z80 na main `1b16256358e3f17837a95cc071048c5ee5307f82` é idêntico ao HEAD validado da PR #968 (`c39f79161e45cfcbf88b9367a3abc2ba136614cc`) nos arquivos centrais:

| Arquivo | Blob SHA na main e no HEAD #968 |
|---|---|
| `native/z80_cpu/src/cpu.rs` | `53b14327aaa84fa4f32289aa1cc373755b98a1e0` |
| `native/z80_cpu/src/sequencer.rs` | `59734821b9bf32cbf22cc2ba55a77f756e7bde04` |
| `native/z80_cpu/src/prefix.rs` | `177a52e55ed3d22ef34d6325222372240fffe960` |

Assim, o Z80 focused PASS da #968 (CI #43904788014, incluindo SingleStepTests DD/FD, Z80 Block Flags ROM v5.0 e Woodmass MEMPTR) permanece aplicável ao mesmo código de produção da main. As PRs #969/#970 posteriores alteraram testes/documentação, não esses três arquivos de produção.

A comparação também confirma que o código central do Scheduler na main é idêntico ao SHA LONG validado `b60f08eeab7b848727661e64a6c7eed671c2fcfe`:

| Arquivo | Blob SHA na main e no SHA LONG |
|---|---|
| `native/machine_scheduler/src/lib.rs` | `e25a62668102d18b452b9a358241a93450d5d189` |
| `native/machine_scheduler/src/codec.rs` | `dfbaa492bec9a506653ba210ba420e9044610d9a` |

Portanto, LONG #43901362527 é evidência válida para o mesmo código central do Scheduler atualmente em main. A PR #972 altera somente o teste de integração SMS e não altera o Scheduler nem o Z80 de produção. O CI #43918490165 executou o workspace no HEAD #972 e registrou 13/13 testes em `snapshot_restore_e7_2e`; o Z80 focused e o Scheduler stress foram SKIPPED nessa PR, mas suas campanhas anteriores continuam vinculadas aos mesmos blobs de produção.


## Pós-merge #972 — evidência na main

- Main/merge SHA: `1b16256358e3f17837a95cc071048c5ee5307f82`.
- CI push [#43933821228](https://github.com/leonardo201800478/sega-emulator/actions/runs/43933821228): Rust workspace regression PASS; Z80 focused e Scheduler deterministic stress SKIPPED (NOT RUN).
- SMS Runtime push [#43933821198](https://github.com/leonardo201800478/sega-emulator/actions/runs/43933821198): Release format/Clippy/tests PASS; Windows debugger SKIPPED.
- O resultado 13/13 do `snapshot_restore_e7_2e` permanece vinculado ao HEAD da PR #972 e à sua árvore integrada; não converte os gates SKIPPED em PASS nem fecha E7.


## Pós-merge #975 — F-028 wrap do endereço de retorno

- Main/merge SHA: `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`.
- PR #975 acrescenta casos dirigidos CALL em PC `FFFC` e RST em `FFFE`, comparando BASE com DD/FD e verificando os dois bytes da pilha no wrap `FFFF→0000`.
- Push CI [#43937688464](https://github.com/leonardo201800478/sega-emulator/actions/runs/43937688464): Z80 focused PASS e Rust quality/unit PASS; jobs não relacionados SKIPPED.
- Push SMS Runtime [#43937688349](https://github.com/leonardo201800478/sega-emulator/actions/runs/43937688349): Release format/Clippy/tests PASS; Windows debugger SKIPPED.
- A matriz continua sendo cobertura dirigida estrutural, não oracle independente de hardware. GitHub não registra review; Codex review falhou por limite de uso. F-028 permanece IN_PROGRESS.
