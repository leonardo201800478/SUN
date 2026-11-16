# F-023 — Matriz de famílias WZ/MEMPTR

**Baseline examinada:** `22c489e03aaed440bbb150ee9a4b7b70c74b1b84` (main após PR #975). PRs #972 e #975 alteraram somente testes; os blobs de produção Z80 permanecem os mesmos da validação focada #968.
**Perfil:** Zilog Z80 NMOS compartilhado; runtime temporal SMS; SG-1000 mantido como core de referência com execução em fronteiras de instrução.
**Objetivo:** inventariar regras WZ/MEMPTR efetivamente implementadas, condições de atualização, wrap, observabilidade e proveniência dos testes. Esta matriz não atribui ao UM0080 regras que o fabricante não documenta.

## Classificação de evidência

- **DOC:** comportamento explicitamente documentado por ZiLOG. O UM0080 não define WZ/MEMPTR como registrador arquitetural e não é fonte para regras internas de WZ.
- **HW:** teste externo hardware-derived com proveniência e fixture pinadas. Não equivale a medição física executada neste projeto nem a prova universal de variantes.
- **COMP:** comparação com implementações ou corpus de emuladores. Evidência de compatibilidade, não oráculo absoluto.
- **INT:** teste interno com fórmula/estado inicial explícito. Pode demonstrar a implementação do modelo, mas não comprova silício por si só.
- **GAP:** regra presente no código, mas sem teste WZ específico rastreável nesta baseline, ou com cobertura temporal/independente incompleta.

## Matriz por família

| Família / formas | Regra no modelo atual e condição | Antes → depois / wrap | Observabilidade | Fonte e teste responsável | Classe / lacuna |
|---|---|---|---|---|---|
| Reset | Reset limpa WZ | Qualquer valor → `0000` | Estado após reset | `cpu.rs::reset`; `reset_preserves_general_registers_external_int_and_machine_time` | INT; reset não é regra de instrução |
| NMI | WZ recebe o vetor de serviço | Qualquer valor → `0066` | Estado após acknowledge/entrada NMI | `cpu.rs::service_nmi`; `native/z80_cpu/tests/nmi_e7.rs` | INT; não generalizar para outras variantes |
| INT modos 0/1/2 | IM0 segue a instrução injetada (RST atualiza WZ ao vetor); IM1 preserva WZ no modelo; IM2 recebe o PC carregado da tabela | IM0: vetor RST; IM1: WZ_old→WZ_old; IM2: WZ_old→PC vetorial | Estado após acknowledge/serviço | `snapshot_restore_e7_2e.rs::e7_2e_im0_restores_acknowledge_and_injected_rst`, `e7_2e_im1_restores_dispatch_and_both_stack_writes`, `e7_2e_im2_restores_stack_and_both_vector_reads`; IM2 já afirma WZ=PC | INT; PR #954 integrada, CI #43893014345 e SMS Runtime #43893014300 PASS no HEAD validado |
| `LD A,(BC/DE)` | WZ = endereço efetivo + 1 | `addr → addr.wrapping_add(1)`; wrap 16-bit | Estado WZ diretamente | `base_indirect_accumulator_f002.rs`; `z80_f002_temporal.rs`; comparação MAME/ares/superzazu | COMP + INT; UM0080 não especifica WZ |
| `LD (BC/DE),A` | WZ = `(A << 8) | ((addr + 1) & 00FF)` | Byte baixo de addr+1; byte alto vem de A | Estado WZ diretamente | Mesmo F-002; fórmula independente no teste | COMP + INT; sem prova física específica |
| `LD A,(nn)` | WZ = nn + 1 | 16-bit com wrap `FFFF → 0000` | Estado WZ diretamente | `base_absolute_accumulator_f003.rs`; `z80_f003_temporal.rs` | INT + COMP documentado no relatório F-003; sem evidência física |
| `LD (nn),A` | WZ = `(A << 8) | ((nn + 1) & 00FF)` | Incrementa somente o byte baixo do endereço no resultado WZ | Estado WZ diretamente | F-003, fórmula `expected_wz` | INT + COMP; não atribuir a UM0080 |
| BASE `LD (nn),HL` e ED `LD (nn),rr / LD rr,(nn)` | WZ = nn + 1 | Incremento de 16 bits; `FFFF → 0000` | Estado WZ e ordem little-endian de acessos | `base_register_pair_f004.rs`; `z80_base_absolute_word_transfer_boundaries.rs`; `z80_ed_absolute_word_wraparound.rs`; teste unitário `ed_absolute_word_loads_and_stores_cover_all_register_pairs` | INT; wrap coberto; WZ não é regra documentada por ZiLOG |
| `ADD HL,rr`, `ADD IX/IY,rr`, `ADC/SBC HL,rr` | WZ = valor antigo do par de destino + 1 | `old_HL/IX/IY → old + 1`, wrap 16-bit | Estado WZ após operação | `indexed_16bit_e3a1.rs`; `z80_ed_16bit_arithmetic_boundaries.rs`; helper `adc_sbc_hl` | INT + comparação com modelo BASE; não é especificação UM0080 |
| `JR e`, `DJNZ e` | Quando tomado, WZ recebe o PC de destino; quando não tomado, preserva WZ | Destino calculado após displacement; wrap 16-bit | WZ, PC, flags e ciclos nos dois caminhos | `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_jr_covers_taken_and_not_taken_paths`, `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::djnz_covers_taken_and_not_taken_paths_without_changing_flags`, `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_jr_wraparound_and_signed_displacement_boundaries` | INT; ambos os caminhos e limites assinados/wrap cobertos |
| `JP nn`, `CALL nn` e condicionais | WZ recebe o alvo imediato, mesmo quando a condição de JP/CALL condicional não é tomada | WZ anterior → nn | WZ, PC, flags, stack writes e ciclos nos dois caminhos | `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_jp_covers_all_conditions_without_changing_flags`, `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_call_covers_both_paths_and_stack_write_order`, `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; oito condições, taken/not-taken e wrap do SP cobertos |
| `RET cc` | Quando tomado, WZ recebe o PC desempilhado; quando não tomado, preserva WZ | WZ anterior → PC retornado ou permanece | WZ, PC, stack reads e wrap do SP | `z80_conditional_control_flow_coverage.rs::conditional_ret_covers_both_paths_and_stack_read_order`, `conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; ambas as condições e wrap cobertos |
| `RET`, `RETN/RETI`, `RST` | WZ recebe o PC resultante do pop ou o vetor de restart | WZ anterior → PC/vetor; stack/PC com wrap | Estado WZ após retorno/restart | `cpu.rs::ed_retn_reti_aliases_restore_iff1_and_pop_return_address`; IM0 RST em `snapshot_restore_e7_2e.rs` | INT; PR #954 integrada: RETN/RETI WZ=PC retornado e IM0 RST WZ=vetor; CI #43893014345 e SMS Runtime #43893014300 PASS |
| `OUT (n),A` | WZ = `(port & FF00) | ((port + 1) & 00FF)`, port = A:n | Byte baixo incrementa com wrap dentro da página | Estado WZ após I/O | `cpu.rs::tests::immediate_io_updates_wz_with_port_specific_wrap_rules` (PR #954), inclui D3 com n=FF e wrap dentro da página | INT; PR #954 CI #43893014345 PASS |
| `IN A,(n)` | WZ = port + 1, port = A:n | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após I/O | `cpu.rs::tests::immediate_io_updates_wz_with_port_specific_wrap_rules` (PR #954), inclui DB com A:n=FFFF e wrap 16-bit | INT; PR #954 CI #43893014345 PASS |
| ED `IN r,(C)` / `OUT (C),r` | WZ = BC + 1 | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após acesso à porta | `native/sms_core/tests/z80_ed_io_exhaustive_semantics.rs` | INT; varredura de dados/encodings e wrap; não cobre semântica física de todas as variantes |
| `EX (SP),HL/IX/IY` | WZ recebe a palavra lida da pilha e trocada com HL/IX/IY | WZ anterior → palavra lida; endereço SP não é o valor final | Estado WZ e stack writes | BASE `cpu.rs::tests::exchange_stack_hl_and_di_ei_update_state` (asserção integrada pela #955); `indexed_exchange_stack_e3a2c.rs` para IX/IY | INT; CI #43897766618 e SMS Runtime #43897766471 PASS no HEAD da #955; debugger Windows Release SKIPPED |
| `LD (IX/IY+d),r/n`, `LD r,(IX/IY+d)`, ALU e INC/DEC indexed | WZ = endereço efetivo IX/IY + displacement assinado | Soma 16-bit; wrap em `0000/FFFF` | WZ diretamente e, para BIT, flags X/Y derivados do byte alto | `indexed_memory_e4a.rs`; `indexed_alu_e4b.rs`; `indexed_incdec_e4c.rs`; `native/z80_cpu/tests/indexed_memory_e4a.rs::indexed_hl_register_loads_cover_signed_displacement_boundaries_and_snapshots` | INT; WAIT/snapshot cobertos nos caminhos microsequenciados pertinentes |
| DDCB/FDCB | WZ = endereço efetivo IX/IY+d | Soma 16-bit com wrap | WZ direto; BIT usa bits 5/3 do byte alto do endereço | `indexed_cb_e5.rs`; `z80_indexed_cb_reference_vectors.rs`; `z80_indexed_cb_bit_res_set_wz.rs` | INT + referências de software; não é prova física |
| `LD (nn),IX/IY` / `LD IX/IY,(nn)` | WZ = nn + 1 | 16-bit; wrap | WZ direto e acessos little-endian | `indexed_absolute_transfer_e3a2a.rs` | INT |
| `LD SP,IX/IY` e famílias BASE que preservam WZ | WZ permanece inalterado quando o handler não define nova regra | `WZ_old → WZ_old` | Estado WZ antes/depois | `ld_sp_f005.rs`; `base_load_f001.rs`; `base_register_pair_f004.rs` | INT |
| BIT `(HL)` | WZ não é atualizado pelo CB register/memory handler; X/Y vêm do byte alto do WZ existente | WZ anterior permanece | Flags X/Y são a observação indireta | `z80_cb_exhaustive_semantics.rs` | INT; o valor auxiliar não é observável como registrador em hardware |
| RRD/RLD | WZ = HL + 1 | Incremento de 16 bits; `FFFF → 0000` | WZ direto e subsequente BIT 0,(HL) observa X/Y | 262.144 vetores em `cpu.rs::ed_rrd_rld_exhaustive_nibble_transform_and_flag_contract`; 16.384 cenários observáveis RRD/RLD→BIT 0,(HL) (PR #934); 240 cenários micro com WAIT/snapshot em `ed_nibble_rotate_f023_observable.rs`; Woodmass test 09, linhas RLD `117F` e RRD `34AE` | HW para regra WZ=HL+1 no escopo do teste 09; INT para transformação/WAIT/snapshot; X/Y ainda COMP |
| LDI/LDD/CPI/CPD simples | LDI/LDD preservam WZ; CPI/CPD atualizam WZ = WZ anterior ± 1 conforme direção | LDI/LDD: `WZ_old → WZ_old`; CPI/CPD: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `cpu.rs::ed_block_simple_forms_exhaustively_preserve_flags_and_track_wz` (524.288 casos); PR #951 acrescenta WZ e previous-Q à matriz micro de 786.432 casos | INT; sem oracle físico independente para esta regra |
| LDIR/LDDR/CPIR/CPDR quando repetem | WZ = PC após recuo + 1 | PC recua 2 bytes; WZ = PC+1 | WZ, flags/Q/previous-Q, IM2, PUSH AF/POP AF/EI/RETI e retomada | `z80_ed_block_repeat_interrupt_observation.rs` (32 casos atômicos); `sequencer.rs::repeat_im2_microsequencer_matrix_covers_all_pc_bc_and_q_boundaries` (64 cenários micro); `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` (4 formas com snapshot/restore em cada fronteira); `snapshot_restore_e7_2e.rs` (PR #962: WZ, Q/previous-Q, WAIT e replay no Scheduler) | COMP + INT; matriz micro não é oracle independente, mas cobertura temporal/snapshot já existe |
| LDIR/LDDR/CPIR/CPDR quando terminam | LDIR/LDDR preservam WZ; CPIR/CPDR atualizam WZ = WZ anterior ± 1 conforme direção | Transferências: `WZ_old → WZ_old`; comparações: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `z80_ed_block_transfer_compare_boundaries.rs`; testes unitários em `cpu.rs`; integração Scheduler/replay com assertions WZ/Q/previous-Q das quatro formas em `snapshot_restore_e7_2e.rs` (PR #962) | INT; #962 verifica WZ/Q/previous-Q na integração temporal, sem constituir oracle independente ou evidência física |
| INI/IND/INIR/INDR/OUTI/OUTD/OTIR/OTDR | Repetindo: WZ = PC+1. Não repetindo: base é BC antigo para input e BC novo para output; WZ = base ± 1 | Wrap 16-bit; direção conforme IN/OUT e incremento/decremento | Estado WZ, portas e memória | `cpu.rs::execute_block_io`; `z80_ed_block_io_reference_vectors.rs`; relatório F-021 (390 B-loop records) | COMP: SingleStepTests é traduzido do Ares; não é oráculo físico. Manter F-021 separado de F-022 |
| ED NEG, LD I/R, IM e ED reservados | WZ é preservado nos handlers que não o atualizam | `WZ_old → WZ_old` | Estado WZ após instrução | `z80_ed_neg_nibble_exhaustive.rs`; `z80_ed_special_registers_exhaustive.rs`; `ed_reserved_f033.rs` | INT; não generalizar codificações reservadas a CMOS/clones |

## Lacunas que permanecem explícitas

1. **Proveniência:** somente RRD/RLD possui evidência hardware-derived específica nesta matriz. As demais regras não documentadas são classificadas como INT/COMP; não são garantias físicas universais. Os dígitos `117F` e `34AE` do Woodmass são assinaturas de resultado das linhas, não valores literais de WZ.
2. **Perfil e variantes:** CMOS, NEC/R800/Z180, Z80N e clones fora do perfil Zilog NMOS delimitado permanecem NOT_VERIFIED, deliberadamente fora do critério de fechamento. Não exigir hardware SMS físico nem universalidade de clones para fechar o perfil NMOS delimitado.

## Decisão F-023

As regras WZ/MEMPTR relevantes do perfil delimitado estão classificadas por família e possuem testes rastreáveis; as propriedades não comprovadas em hardware estão explicitamente classificadas como INT/COMP ou NOT_VERIFIED. **F-023 permanece AUDIT_PARTIAL até a revisão integral da matriz, a reconciliação da proveniência e a revisão adversarial; a assertion BASE EX (SP),HL da #955 já está integrada.**

Nenhuma alteração de produção é autorizada por esta matriz. Não iniciar E8/ZEXDOC/ZEXALL.

## Reconciliação após PR #955 — 16/11/2026

A PR #955 foi integrada na main pelo commit `67f4149e1933d18ffa5201da66ca6ff46c598e32`. O diff é exclusivamente test-only: quatro linhas de assertion em `native/z80_cpu/src/cpu.rs`, no teste `exchange_stack_hl_and_di_ei_update_state`, verificando `cpu.wz == 0xABCD` após BASE `EX (SP),HL`. Não houve alteração de produção, decoder, microsequenciador, Scheduler, SMS ou SG-1000.

Validação no HEAD exato da PR #955 (`cbc75e9205db5f1c29726962ce3d3cdd312f4ba1`):
- CI #43897766618: PASS — Z80 focused validation, incluindo rustfmt, Clippy, testes Z80/SMS/SG-1000, fixtures SingleStepTests DD/FD, Z80 Block Instruction Flags v5.0 e Woodmass MEMPTR; Rust quality/unit regression PASS.
- SMS Debugger Runtime #43897766471: PASS — formatação, Clippy Release, testes SMS Runtime e build Release Windows x64.
- SMS debugger Windows Release validation: SKIPPED. Não contar como PASS; o workflow concluiu SUCCESS porque esse job condicional não era aplicável ao diff.
- Workflow push pós-merge no SHA `67f4149e1933d18ffa5201da66ca6ff46c598e32`: não localizado na consulta de runs; classificar NOT_RUN/NOT_VERIFIED, não PASS.

A matriz WZ/MEMPTR agora possui assertion dirigida para BASE `EX (SP),HL`, além das coberturas IX/IY já existentes. Isso fecha a pendência específica de teste dessa instrução, mas não declara F-023 CLOSED: permanecem a revisão de consistência da matriz completa, a proveniência das regras e a revisão adversarial transversal.

Estado após a integração:
- F-023: AUDIT_PARTIAL — lacuna específica BASE `EX (SP),HL` coberta; revisão integral e reconciliação ainda necessárias.
- F-028: IN_PROGRESS — manter limites e lacunas documentados; não inferir fechamento a partir das matrizes.
- F-022: IN_PROGRESS — manter separação de F-021 e validar integralmente as oito formas.
- Temporalidade/Scheduler: NOT_VERIFIED como fechamento E7 até demonstração integrada de determinismo e equivalência contínua/em blocos.
- E7: IN_PROGRESS.
- E8/ZEXDOC/ZEXALL: BLOCKED.

A integração #955 não substitui regressão integrada no HEAD final da main nem revisão adversarial independente. Não promover SKIPPED, NOT_RUN ou evidência interna comparativa a PASS de hardware.

## Atualização pós-merge PR #957 — 16/11/2026

**Main:** `2f72a6d94207cc78e5014363e72d16cf8806316a`. **PR #957:** integrada; alteração somente de teste em `native/sms_core/tests/snapshot_restore_e7_2e.rs`.

O teste `e7_f022_repeat_forms_match_scheduler_wait_snapshot_replay` executa LDIR, LDDR, CPIR e CPDR pelo `SmsTemporalMachine`/MachineScheduler, cobrindo a iteração repetitiva e a final. Insere um WAIT de fixture em cada request de barramento; a execução retomada faz snapshot/restore em cada safe point de request/ciclo interno. A execução contínua e a retomada são comparadas por trace de dispatch, estado completo da máquina e timestamp final.

Validação no HEAD exato `8a6ef8bbf20c770479999f00675718a58349985e`:
- CI #43899876697: Rust workspace regression PASS.
- SMS Debugger Runtime #43899876806: rustfmt PASS, Clippy Release PASS, SMS Runtime tests PASS e build Release Windows x64 PASS.
- Scheduler deterministic stress e Z80 focused foram SKIPPED por classificação de impacto; não contar como PASS.
- SMS debugger Windows Release foi SKIPPED; não contar como PASS.
- A execução anterior #43899664243 falhou somente em rustfmt; a formatação foi corrigida antes do HEAD final e não se atribui PASS ao run anterior.
- Workflow push pós-merge no SHA `2f72a6d94207cc78e5014363e72d16cf8806316a`: não localizado; NOT_RUN/NOT_VERIFIED.

Esta integração fecha a lacuna específica de execução das quatro formas repetitivas através do Scheduler com WAIT de fixture e snapshot/restore. Não demonstra WAIT físico, interrupção injetada no meio da repetição dentro da mesma máquina temporal, nem oracle independente de silício. A cobertura IM2/flags/Q/previous-Q permanece demonstrada separadamente no microsequenciador; não somar essas duas camadas como se fossem uma única prova integrada.

Estado atualizado:
- F-022: IN_PROGRESS — agora possui integração Scheduler/replay para as quatro formas; permanecem revisão adversarial, reconciliação final das oito formas e proveniência independente dos efeitos não documentados.
- F-023: AUDIT_PARTIAL — BASE EX (SP),HL já possui assertion integrada pela #955; permanecem revisão integral da matriz/proveniência e limitações de hardware.
- F-028: IN_PROGRESS — 167 Ignored com estado/requests/WAIT/snapshot e caminhos condicionais; permanece a revisão adversarial e a validação independente de limites semânticos.
- Temporalidade: evidência integrada SMS ampliada; WAIT físico/DMA permanecem fora da fatia temporal atual.
- E7: IN_PROGRESS. E8/ZEXDOC/ZEXALL: BLOCKED.

## Atualização pós-merge PR #962 — 16/11/2026

**Main de referência:** `bb9e31bdf04099a4116d27836691f0b31ab8c8a8`. PR #962 integrada; alteração somente de teste em `native/sms_core/tests/snapshot_restore_e7_2e.rs`.

O teste `e7_f022_repeat_forms_match_scheduler_wait_snapshot_replay` agora verifica, nas quatro formas LDIR/LDDR/CPIR/CPDR executadas pelo `SmsTemporalMachine`/MachineScheduler:
- WZ=PC+1 na iteração repetitiva;
- WZ preservado em LDIR/LDDR e atualizado em CPIR/CPDR na iteração terminal;
- Q=F nas duas fronteiras de aposentadoria;
- previous-Q carrega o Q da iteração repetitiva para a iteração final;
- WAIT de fixture em cada request de barramento e snapshot/restore em cada safe point;
- equivalência entre execução contínua e retomada por trace, estado completo e timestamp.

Validação no HEAD exato da PR #962 (`316dba3ce1c435a96997f1d26ca98e88680fb0c5`):
- CI #43901912692: Rust workspace regression PASS — validação de workspaces/lockfiles, rustfmt, Clippy, testes de workspace e feature runtime de observabilidade PASS.
- SMS Debugger Runtime #43901912772: format, Clippy Release, testes SMS Runtime e build Release Windows x64 PASS.
- Scheduler deterministic stress, Z80 focused e SMS debugger Windows Release: SKIPPED; não contar como PASS.
- A tentativa anterior PR #961 falhou somente em rustfmt e foi encerrada como supersedida; a correção foi aplicada na PR #962 antes da validação final.
- Workflow push pós-merge no SHA `bb9e31bdf04099a4116d27836691f0b31ab8c8a8`: não localizado; NOT_RUN/NOT_VERIFIED.

### Evidência Scheduler LONG no código atual

A campanha opt-in LONG foi executada no workflow CI #43901362527, SHA `b60f08eeab7b848727661e64a6c7eed671c2fcfe`, branch da PR #960 baseada na main `f6ce78c8646964c9e0ff828587066c70a7165cbb`. A PR #960 era documental; o código de `machine_scheduler` era idêntico à base main e não foi alterado pela PR #962.
- 16 seeds × 20.000 operações = 320.000 operações verificadas.
- 16 seeds × 250.000 eventos = 4.000.000 dispatches verificados.
- `fixed_seed_operation_sequences_match_across_codec_restores`: PASS.
- `fixed_seed_stress_replay_survives_repeated_restores`: PASS.
- Scheduler deterministic stress: LONG_PASS; Documentation patch hygiene e Rust quality/unit regression: PASS.
- Os gates de Z80 focused, SMS, SDL3, CPAL, Slint/WGPU/Vulkan e demais caminhos não afetados foram SKIPPED nessa execução.

Esse resultado é prova de determinismo/replay do Scheduler como software no código atual; não comprova sub-tick dispatch, safe points físicos de dispositivos, WAIT físico, DMA ou TEMPORAL_PASS.

Estado vigente:
- F-022: IN_PROGRESS — quatro formas repetitivas integradas ao Scheduler com WZ, Q/previous-Q, WAIT de fixture e snapshot/replay; falta a revisão adversarial e a combinação de interrupção durante repetição dentro da mesma máquina temporal.
- F-023: AUDIT_PARTIAL — BASE EX (SP),HL e WZ terminal das quatro formas de bloco possuem assertions integradas; falta revisão integral da matriz/proveniência.
- F-028: IN_PROGRESS — 167 Ignored, 85 UsesIndex e exceções DD/FD reconciliados/testados; falta revisão adversarial independente dos limites semânticos.
- Temporalidade: Scheduler LONG_PASS e equivalência contínua/retomada demonstrados nas dimensões cobertas; WAIT físico/DMA e safe points de dispositivos permanecem NOT_VERIFIED.
- E7: IN_PROGRESS. E8/ZEXDOC/ZEXALL: BLOCKED.


## Evidência WZ pós-PR #964 — 16/11/2026

A PR #964 adiciona assertions temporais WZ para as quatro formas repetitivas durante interrupção IM1/RETI no Scheduler. WZ é observado após a iteração repetitiva, após acknowledge IM1, após RETI e após a iteração terminal, incluindo wrap em CPIR/CPDR. CI #43903631872 e SMS Runtime #43903632358 passaram nos gates aplicáveis no HEAD exato; debugger Windows Release, Z80 focused e Scheduler stress foram SKIPPED. Trata-se de evidência interna do modelo, não de oracle físico. F-023 permanece AUDIT_PARTIAL até revisão integral da matriz/proveniência.

## Estado final da rodada — pós-PR #969 — 16/11/2026

**Main atual:** `08f633affdb7478b64b1ec2007ca1986c2a48978`.

### PRs integradas nesta rodada
- #960 — documentação Scheduler stress; preservou evidências de determinismo/replay LONG.
- #962 — assertions WZ, Q e previous-Q nas quatro formas repetitivas através do Scheduler.
- #963 — reconciliação de documentação E7.
- #964 — INT no último T-state da repetição, IM1, RETI e retomada no Scheduler.
- #966 — reconciliação documental e correção da proveniência histórica de workflows.
- #968 — classificação explícita stream/data em fallback DD/FD.
- #969 — integração Scheduler/SMS das quatro formas simples LDI/LDD/CPI/CPD.

PRs #958, #959, #965 e #967 foram encerradas como supersedidas, preservando seus históricos e reaplicando as mudanças relevantes nas PRs atuais. Nenhuma alteração de produção foi descartada.

### F-022 — flags de instruções de bloco

A cobertura de integração temporal SMS agora abrange as oito formas:
- Simples: LDI, LDD, CPI, CPD — PR #969, teste `e7_f022_simple_forms_match_scheduler_wait_snapshot_replay`.
- Repetitivas: LDIR, LDDR, CPIR, CPDR — PRs #957/#962; assertions WZ/Q/previous-Q.
- Interrupção durante repetição: quatro formas com INT level-sensitive no último T-state, serviço IM1, RETI e retomada — PR #964, teste `e7_f022_repeat_interrupt_is_sampled_and_restored_by_sms_scheduler`.

As integrações aplicam WAIT de fixture em cada request de barramento, snapshot/restore em cada safe point e comparação de trace, estado completo e timestamp entre execução contínua e retomada. A cobertura simples verifica flags, Q/previous-Q, WZ, PC, BC, HL/DE e ausência de escrita para as formas compare. A cobertura repetitiva verifica também PC/BC/WZ e fronteiras de interrupção.

Validação exata da PR #969, HEAD `e831f6c94aac89ad515af045133520ed6de7e986`:
- CI #43906014092: Rust workspace regression PASS — workspace/lockfiles, rustfmt, Clippy, testes unitários e de integração, Clippy/testes do recurso observability PASS.
- SMS Runtime #43906014526: format, Clippy Release, testes SMS Runtime e build Windows x64 PASS.
- Z80 focused, Scheduler stress, demais hosts não relacionados e SMS debugger Windows Release: SKIPPED. Não classificar como PASS.
- A tentativa anterior de #969 teve rustfmt FAIL e expectativa errada de F para CPI/CPD; o resultado observado `$3F` foi reconciliado com a fórmula independente (compare preserva somente C, não S/Z). O HEAD final corrigiu as expectativas e passou.
- Workflow push pós-merge no SHA `08f633affdb7478b64b1ec2007ca1986c2a48978`: não localizado; NOT_RUN/NOT_VERIFIED.

F-022 avança para cobertura temporal integrada das oito formas, mas permanece IN_PROGRESS até revisão adversarial independente e revisão final das dimensões combinatórias/oráculos. A matriz ROM v5.0 permanece um conjunto não exaustivo, não prova universal.

### F-028 — semântica DD/FD

PR #968 integrada no SHA `96b21fc7568feb6f7232440eaf4620d0f584d6fe`. A correção substitui a heurística `MemoryRead.address == PC` por contagem explícita de operandos imediatos derivados do opcode BASE, impedindo que uma leitura de dados com endereço igual a PC seja classificada como leitura de stream. Regressão: DD + `LD A,(BC)`, BC=$0002.

Validação exata da PR #968, HEAD `c39f79161e45cfcbf88b9367a3abc2ba136614cc`:
- CI #43904788014: Z80 focused PASS, incluindo formatação, Clippy, testes Z80/SMS/SG-1000, Z80 Block Flags ROM v5.0, SingleStepTests DD/FD e Woodmass MEMPTR; Rust quality/unit PASS.
- SMS Runtime #43904788006: Release format, Clippy, testes e build Windows x64 PASS.
- Scheduler stress e SMS debugger Windows Release SKIPPED.
- Workflow push pós-merge: NOT_RUN/NOT_VERIFIED.

F-028 continua IN_PROGRESS até revisão adversarial integral da matriz e confirmação independente das exceções e limites DD/FD. As 85 UsesIndex reconciliadas não devem ser reimplementadas; os 167 Ignored permanecem separados dos casos UsesIndex.

### F-023 — WZ/MEMPTR

A matriz de famílias permanece rastreável. Evidências agora incluem BASE EX (SP),HL (#955), quatro formas simples de bloco com WZ/flags/Q/previous-Q em execução Scheduler (#969), e quatro formas repetitivas com WZ/flags/Q/previous-Q sob WAIT/snapshot e IM1/RETI (#962/#964).

**F-023 permanece AUDIT_PARTIAL:** a matriz classifica regras INT/COMP/HW, mas somente a regra WZ=HL+1 de RRD/RLD possui evidência hardware-derived específica na matriz. As regras não documentadas não são apresentadas como garantia física universal. CMOS, NEC/R800/Z180, Z80N e clones fora do perfil delimitado continuam NOT_VERIFIED. Revisão adversarial e reconciliação integral da proveniência continuam pendentes.

### Temporalidade e Scheduler

- LONG_PASS de software Scheduler na campanha #43901362527, SHA `b60f08eeab7b848727661e64a6c7eed671c2fcfe`: 16 seeds × 20.000 operações = 320.000 operações; 16 × 250.000 eventos = 4.000.000 dispatches; ambas as suítes passaram.
- Integração SMS temporal/replay dirigida: PRs #957/#962/#964/#969, cada uma validada em seu HEAD exato.
- WAIT usado nos testes de integração é fixture determinística; não representa medição elétrica ou validação de WAIT físico.
- DMA, safe points de todos os dispositivos, sincronização física e TEMPORAL_PASS global permanecem NOT_VERIFIED.
- A equivalência contínua/retomada está demonstrada nas dimensões cobertas pelos testes citados, não em todos os caminhos do hardware.

### Decisão consolidada
- F-022: IN_PROGRESS — oito formas integradas temporalmente; revisão adversarial independente pendente.
- F-023: AUDIT_PARTIAL — famílias mapeadas e testes rastreáveis; proveniência HW limitada e revisão independente pendente.
- F-028: IN_PROGRESS — matriz e classificação de stream/data corrigida; revisão adversarial independente pendente.
- Temporalidade: Scheduler software LONG_PASS; integração dirigida PASS; WAIT físico/DMA e TEMPORAL_PASS global NOT_VERIFIED.
- SMS/SG-1000: testes Z80/SMS/SG-1000 passaram no gate Z80 focused da #968; SMS Runtime Release passou nas #964/#968/#969. Gates SKIPPED permanecem SKIPPED.
- **E7: IN_PROGRESS. E8/ZEXDOC/ZEXALL: BLOCKED.**

Nenhuma etapa E8 deve iniciar até revisão independente registrada, fechamento formal das três frentes e aprovação de todos os critérios temporais e de regressão exigidos pelo contrato E7.


## Revisão adversarial F-023 — baseline pós-PR #975 (16/11/2026)

**Escopo da revisão:** semântica WZ/MEMPTR do perfil Zilog Z80 NMOS usado por SMS/SG-1000. Foram confrontados os handlers ativos de `native/z80_cpu/src/cpu.rs` e `native/z80_cpu/src/sequencer.rs` com as regras, condições, wrap, observabilidade e testes listados nesta matriz. A revisão não altera produção.

### Resultado por domínio

- **Fluxo de dados e aritmética:** LD indireto/absoluto, transferências de pares, ADD/ADC/SBC e operações IX/IY atribuem WZ conforme o endereço ou valor anterior definido pela matriz; os limites de 16 bits estão cobertos pelos testes F-002/F-003/F-004/E3A1/E3A2.
- **Fluxo de controle:** JR/DJNZ, JP/CALL condicionais, RET/RETN/RETI e RST têm regras separadas para caminho tomado/não tomado, endereço alvo, retorno e wrap; os testes de controle condicional verificam PC, WZ, stack e ciclos.
- **I/O:** IN/OUT imediato, IN/OUT por C e operações de bloco distinguem o incremento dentro da página de `OUT (n),A`, o incremento de 16 bits de IN/OUT por C e as bases específicas de input/output block. F-021 permanece separado de F-022.
- **Interrupções e trocas:** NMI, IM0/IM1/IM2 e EX (SP),HL/IX/IY têm transições WZ explícitas e assertions de estado/stack.
- **CB/ED e indexadas:** BIT (HL) preserva WZ e deriva X/Y do byte alto; RRD/RLD atualiza WZ para HL+1; DDCB/FDCB e operações com displacement usam o endereço efetivo com wrap; as quatro formas simples/repetitivas de bloco têm regras distintas para repetição e término.
- **Reset e instruções sem regra:** o reset limpa WZ; famílias ED sem regra específica preservam o valor. Não foi encontrado handler de produção sem classificação na matriz.

### Evidência e limites

- A regra WZ=HL+1 de RRD/RLD mantém evidência hardware-derived do Woodmass test 09, com as assinaturas RLD `117F` e RRD `34AE`; elas não são valores literais de WZ.
- As demais regras continuam honestamente classificadas como INT e/ou COMP. O UM0080 não documenta WZ como registrador arquitetural. Não promovemos fórmulas internas para DOC ou HW.
- A cobertura de RRD/RLD inclui 262.144 transformações, 16.384 sequências observáveis RRD/RLD→BIT (HL) e 240 cenários micro com WAIT/snapshot.
- As quatro formas simples de bloco têm 786.432 cenários microsequenciados com fórmula independente de flags e WZ; as formas repetitivas têm matrizes de flags/PC/BC/Q e observação de interrupção. A integração SMS/Scheduler pós-#972 acrescenta INT IM2 e NMI durante repetição, handlers e retomada com snapshot/replay.
- CMOS, NEC, R800, Z180, Z80N e clones não cobertos permanecem NOT_VERIFIED. Não são promovidos a parte do perfil NMOS delimitado.

### Decisão

**F-023: CLOSED — semântica de software WZ/MEMPTR para o perfil Zilog Z80 NMOS delimitado.** O fechamento significa que todas as famílias modeladas têm regra, condição, wrap, observabilidade e teste rastreável, com proveniência explicitamente classificada; não significa que cada regra interna tenha medição física própria, nem conformidade universal de clones.

Nenhuma alteração de produção foi necessária. A validação temporal de WAIT físico/DMA continua fora desta conclusão e não é inferida dos fixtures. E8 permanece bloqueada até F-028 e os gates globais E7 serem formalmente fechados.
