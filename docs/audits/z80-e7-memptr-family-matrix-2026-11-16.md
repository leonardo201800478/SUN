# F-023 — Matriz de famílias WZ/MEMPTR

**Baseline examinada:** `2f72a6d94207cc78e5014363e72d16cf8806316a` (main após PRs #955–#957).
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
| `JR e`, `DJNZ e` | Quando tomado, WZ recebe o PC de destino; quando não tomado, preserva WZ | Destino calculado após displacement; wrap 16-bit | WZ, PC, flags e ciclos nos dois caminhos | `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_jr_covers_taken_and_not_taken_paths`, `djnz_covers_taken_and_not_taken_paths_without_changing_flags`, `conditional_jr_wraparound_and_signed_displacement_boundaries` | INT; ambos os caminhos e limites assinados/wrap cobertos |
| `JP nn`, `CALL nn` e condicionais | WZ recebe o alvo imediato, mesmo quando a condição de JP/CALL condicional não é tomada | WZ anterior → nn | WZ, PC, flags, stack writes e ciclos nos dois caminhos | `z80_conditional_control_flow_coverage.rs::conditional_jp_covers_all_conditions_without_changing_flags`, `conditional_call_covers_both_paths_and_stack_write_order`, `conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; oito condições, taken/not-taken e wrap do SP cobertos |
| `RET cc` | Quando tomado, WZ recebe o PC desempilhado; quando não tomado, preserva WZ | WZ anterior → PC retornado ou permanece | WZ, PC, stack reads e wrap do SP | `z80_conditional_control_flow_coverage.rs::conditional_ret_covers_both_paths_and_stack_read_order`, `conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; ambas as condições e wrap cobertos |
| `RET`, `RETN/RETI`, `RST` | WZ recebe o PC resultante do pop ou o vetor de restart | WZ anterior → PC/vetor; stack/PC com wrap | Estado WZ após retorno/restart | `cpu.rs::ed_retn_reti_aliases_restore_iff1_and_pop_return_address`; IM0 RST em `snapshot_restore_e7_2e.rs` | INT; PR #954 integrada: RETN/RETI WZ=PC retornado e IM0 RST WZ=vetor; CI #43893014345 e SMS Runtime #43893014300 PASS |
| `OUT (n),A` | WZ = `(port & FF00) | ((port + 1) & 00FF)`, port = A:n | Byte baixo incrementa com wrap dentro da página | Estado WZ após I/O | `cpu.rs::tests::immediate_io_updates_wz_with_port_specific_wrap_rules` (PR #954), inclui D3 com n=FF e wrap dentro da página | INT; PR #954 CI #43893014345 PASS |
| `IN A,(n)` | WZ = port + 1, port = A:n | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após I/O | `cpu.rs::tests::immediate_io_updates_wz_with_port_specific_wrap_rules` (PR #954), inclui DB com A:n=FFFF e wrap 16-bit | INT; PR #954 CI #43893014345 PASS |
| ED `IN r,(C)` / `OUT (C),r` | WZ = BC + 1 | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após acesso à porta | `native/sms_core/tests/z80_ed_io_exhaustive_semantics.rs` | INT; varredura de dados/encodings e wrap; não cobre semântica física de todas as variantes |
| `EX (SP),HL/IX/IY` | WZ recebe a palavra lida da pilha e trocada com HL/IX/IY | WZ anterior → palavra lida; endereço SP não é o valor final | Estado WZ e stack writes | BASE `cpu.rs::tests::exchange_stack_hl_and_di_ei_update_state` (asserção integrada pela #955); `indexed_exchange_stack_e3a2c.rs` para IX/IY | INT; CI #43897766618 e SMS Runtime #43897766471 PASS no HEAD da #955; debugger Windows Release SKIPPED |
| `LD (IX/IY+d),r/n`, `LD r,(IX/IY+d)`, ALU e INC/DEC indexed | WZ = endereço efetivo IX/IY + displacement assinado | Soma 16-bit; wrap em `0000/FFFF` | WZ diretamente e, para BIT, flags X/Y derivados do byte alto | `indexed_memory_e4a.rs`; `indexed_alu_e4b.rs`; `indexed_incdec_e4c.rs`; `indexed_hl_memory_special_cases.rs` | INT; WAIT/snapshot cobertos nos caminhos microsequenciados pertinentes |
| DDCB/FDCB | WZ = endereço efetivo IX/IY+d | Soma 16-bit com wrap | WZ direto; BIT usa bits 5/3 do byte alto do endereço | `indexed_cb_e5.rs`; `z80_indexed_cb_reference_vectors.rs`; `z80_indexed_cb_bit_res_set_wz.rs` | INT + referências de software; não é prova física |
| `LD (nn),IX/IY` / `LD IX/IY,(nn)` | WZ = nn + 1 | 16-bit; wrap | WZ direto e acessos little-endian | `indexed_absolute_transfer_e3a2a.rs` | INT |
| `LD SP,IX/IY` e famílias BASE que preservam WZ | WZ permanece inalterado quando o handler não define nova regra | `WZ_old → WZ_old` | Estado WZ antes/depois | `ld_sp_f005.rs`; `base_load_f001.rs`; `base_register_pair_f004.rs` | INT |
| BIT `(HL)` | WZ não é atualizado pelo CB register/memory handler; X/Y vêm do byte alto do WZ existente | WZ anterior permanece | Flags X/Y são a observação indireta | `z80_cb_exhaustive_semantics.rs` | INT; o valor auxiliar não é observável como registrador em hardware |
| RRD/RLD | WZ = HL + 1 | Incremento de 16 bits; `FFFF → 0000` | WZ direto e subsequente BIT 0,(HL) observa X/Y | 262.144 vetores em `cpu.rs::ed_rrd_rld_exhaustive_nibble_transform_and_flag_contract`; 16.384 cenários observáveis RRD/RLD→BIT 0,(HL) (PR #934); 240 cenários micro com WAIT/snapshot em `ed_nibble_rotate_f023_observable.rs`; Woodmass test 09, linhas RLD `117F` e RRD `34AE` | HW para regra WZ=HL+1 no escopo do teste 09; INT para transformação/WAIT/snapshot; X/Y ainda COMP |
| LDI/LDD/CPI/CPD simples | LDI/LDD preservam WZ; CPI/CPD atualizam WZ = WZ anterior ± 1 conforme direção | LDI/LDD: `WZ_old → WZ_old`; CPI/CPD: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `cpu.rs::ed_block_simple_forms_exhaustively_preserve_flags_and_track_wz` (524.288 casos); PR #951 acrescenta WZ e previous-Q à matriz micro de 786.432 casos | INT; sem oracle físico independente para esta regra |
| LDIR/LDDR/CPIR/CPDR quando repetem | WZ = PC após recuo + 1 | PC recua 2 bytes; WZ = PC+1 | WZ, flags/Q/previous-Q, IM2, PUSH AF/POP AF/EI/RETI e retomada | `z80_ed_block_repeat_interrupt_observation.rs` (32 casos atômicos); `sequencer.rs::repeat_im2_microsequencer_matrix_covers_all_pc_bc_and_q_boundaries` (64 cenários micro); `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` (4 formas com snapshot/restore em cada fronteira) | COMP + INT; matriz micro não é oracle independente, mas cobertura temporal/snapshot já existe |
| LDIR/LDDR/CPIR/CPDR quando terminam | LDIR/LDDR preservam WZ; CPIR/CPDR atualizam WZ = WZ anterior ± 1 conforme direção | Transferências: `WZ_old → WZ_old`; comparações: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `z80_ed_block_transfer_compare_boundaries.rs`; testes unitários em `cpu.rs`; integração Scheduler/replay das quatro formas em `snapshot_restore_e7_2e.rs` (PR #957) | INT; #957 comprova continuidade/restauração temporal, não oracle independente nem valor físico de WZ |
| INI/IND/INIR/INDR/OUTI/OUTD/OTIR/OTDR | Repetindo: WZ = PC+1. Não repetindo: base é BC antigo para input e BC novo para output; WZ = base ± 1 | Wrap 16-bit; direção conforme IN/OUT e incremento/decremento | Estado WZ, portas e memória | `cpu.rs::execute_block_io`; `z80_ed_block_io_reference_vectors.rs`; relatório F-021 (390 B-loop records) | COMP: SingleStepTests é traduzido do Ares; não é oráculo físico. Manter F-021 separado de F-022 |
| ED NEG, LD I/R, IM e ED reservados | WZ é preservado nos handlers que não o atualizam | `WZ_old → WZ_old` | Estado WZ após instrução | `z80_ed_neg_nibble_exhaustive.rs`; `z80_ed_special_registers_exhaustive.rs`; `ed_reserved_f033.rs` | INT; não generalizar codificações reservadas a CMOS/clones |

## Lacunas que permanecem explícitas

1. **BASE EX (SP),HL:** a assertion WZ foi integrada pela PR #955; a lacuna específica está resolvida. A revisão integral da matriz e da proveniência continua pendente.
2. **Proveniência:** somente RRD/RLD possui evidência hardware-derived específica nesta matriz. As demais regras não documentadas são classificadas como INT/COMP; não são garantias físicas universais. Os dígitos `117F` e `34AE` do Woodmass são assinaturas de resultado das linhas, não valores literais de WZ.
3. **Perfil e variantes:** CMOS, NEC/R800/Z180, Z80N e clones fora do perfil Zilog NMOS delimitado permanecem NOT_VERIFIED, deliberadamente fora do critério de fechamento. Não exigir hardware SMS físico nem universalidade de clones para fechar o perfil NMOS delimitado.

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
