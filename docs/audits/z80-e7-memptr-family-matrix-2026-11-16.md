# F-023 — Matriz de famílias WZ/MEMPTR

**Baseline examinada:** `45b40755e46aef752d58f571de856ae25ff9e81c` (main após PR #950).
**Perfil:** Zilog Z80 NMOS clássico usado pelo runtime SMS/SG-1000.
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
| INT modos 0/1/2 | IM0 segue a instrução injetada (RST atualiza WZ ao vetor); IM1 preserva WZ no modelo; IM2 recebe o PC carregado da tabela | IM0: vetor RST; IM1: WZ_old→WZ_old; IM2: WZ_old→PC vetorial | Estado após acknowledge/serviço | `snapshot_restore_e7_2e.rs::e7_2e_im0_restores_acknowledge_and_injected_rst`, `e7_2e_im1_restores_dispatch_and_both_stack_writes`, `e7_2e_im2_restores_stack_and_both_vector_reads`; IM2 já afirma WZ=PC | INT; IM0/IM1 WZ assertions propostas na PR #954, ainda pendentes de CI |
| `LD A,(BC/DE)` | WZ = endereço efetivo + 1 | `addr → addr.wrapping_add(1)`; wrap 16-bit | Estado WZ diretamente | `base_indirect_accumulator_f002.rs`; `z80_f002_temporal.rs`; comparação MAME/ares/superzazu | COMP + INT; UM0080 não especifica WZ |
| `LD (BC/DE),A` | WZ = `(A << 8) | ((addr + 1) & 00FF)` | Byte baixo de addr+1; byte alto vem de A | Estado WZ diretamente | Mesmo F-002; fórmula independente no teste | COMP + INT; sem prova física específica |
| `LD A,(nn)` | WZ = nn + 1 | 16-bit com wrap `FFFF → 0000` | Estado WZ diretamente | `base_absolute_accumulator_f003.rs`; `z80_f003_temporal.rs` | INT + COMP documentado no relatório F-003; sem evidência física |
| `LD (nn),A` | WZ = `(A << 8) | ((nn + 1) & 00FF)` | Incrementa somente o byte baixo do endereço no resultado WZ | Estado WZ diretamente | F-003, fórmula `expected_wz` | INT + COMP; não atribuir a UM0080 |
| BASE `LD (nn),HL` e ED `LD (nn),rr / LD rr,(nn)` | WZ = nn + 1 | Incremento de 16 bits; `FFFF → 0000` | Estado WZ e ordem little-endian de acessos | `base_register_pair_f004.rs`; `z80_base_absolute_word_transfer_boundaries.rs`; `z80_ed_absolute_word_wraparound.rs`; teste unitário `ed_absolute_word_loads_and_stores_cover_all_register_pairs` | INT; wrap coberto; WZ não é regra documentada por ZiLOG |
| `ADD HL,rr`, `ADD IX/IY,rr`, `ADC/SBC HL,rr` | WZ = valor antigo do par de destino + 1 | `old_HL/IX/IY → old + 1`, wrap 16-bit | Estado WZ após operação | `indexed_16bit_e3a1.rs`; `z80_ed_16bit_arithmetic_boundaries.rs`; helper `adc_sbc_hl` | INT + comparação com modelo BASE; não é especificação UM0080 |
| `JR e`, `DJNZ e` | Quando tomado, WZ recebe o PC de destino; quando não tomado, preserva WZ | Destino calculado após displacement; wrap 16-bit | WZ, PC, flags e ciclos nos dois caminhos | `native/sms_core/tests/z80_conditional_control_flow_coverage.rs::conditional_jr_covers_taken_and_not_taken_paths`, `djnz_covers_taken_and_not_taken_paths_without_changing_flags`, `conditional_jr_wraparound_and_signed_displacement_boundaries` | INT; ambos os caminhos e limites assinados/wrap cobertos |
| `JP nn`, `CALL nn` e condicionais | WZ recebe o alvo imediato, mesmo quando a condição de JP/CALL condicional não é tomada | WZ anterior → nn | WZ, PC, flags, stack writes e ciclos nos dois caminhos | `z80_conditional_control_flow_coverage.rs::conditional_jp_covers_all_conditions_without_changing_flags`, `conditional_call_covers_both_paths_and_stack_write_order`, `conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; oito condições, taken/not-taken e wrap do SP cobertos |
| `RET cc` | Quando tomado, WZ recebe o PC desempilhado; quando não tomado, preserva WZ | WZ anterior → PC retornado ou permanece | WZ, PC, stack reads e wrap do SP | `z80_conditional_control_flow_coverage.rs::conditional_ret_covers_both_paths_and_stack_read_order`, `conditional_call_and_ret_wrap_stack_pointer_at_zero` | INT; ambas as condições e wrap cobertos |
| `RET`, `RETN/RETI`, `RST` | WZ recebe o PC resultante do pop ou o vetor de restart | WZ anterior → PC/vetor; stack/PC com wrap | Estado WZ após retorno/restart | `cpu.rs::ed_retn_reti_aliases_restore_iff1_and_pop_return_address`; IM0 RST em `snapshot_restore_e7_2e.rs` | GAP: WZ de RETN/RETI e RST será afirmado pela PR #954, ainda pendente de CI |
| `OUT (n),A` | WZ = `(port & FF00) | ((port + 1) & 00FF)`, port = A:n | Byte baixo incrementa com wrap dentro da página | Estado WZ após I/O | Implementação `cpu.rs::step_inner` | GAP: teste específico de WZ para D3, inclusive n=FF |
| `IN A,(n)` | WZ = port + 1, port = A:n | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após I/O | Implementação `cpu.rs::step_inner` | GAP: teste específico de WZ para DB |
| ED `IN r,(C)` / `OUT (C),r` | WZ = BC + 1 | Incremento de 16 bits; wrap `FFFF → 0000` | Estado WZ após acesso à porta | `native/sms_core/tests/z80_ed_io_exhaustive_semantics.rs` | INT; varredura de dados/encodings e wrap; não cobre semântica física de todas as variantes |
| `EX (SP),HL/IX/IY` | WZ recebe a palavra lida da pilha e trocada com HL/IX/IY | WZ anterior → palavra lida; endereço SP não é o valor final | Estado WZ e stack writes | BASE `cpu.rs::step_inner`; `indexed_exchange_stack_e3a2c.rs` para IX/IY | INT; cobertura base específica deve ser mantida rastreável |
| `LD (IX/IY+d),r/n`, `LD r,(IX/IY+d)`, ALU e INC/DEC indexed | WZ = endereço efetivo IX/IY + displacement assinado | Soma 16-bit; wrap em `0000/FFFF` | WZ diretamente e, para BIT, flags X/Y derivados do byte alto | `indexed_memory_e4a.rs`; `indexed_alu_e4b.rs`; `indexed_incdec_e4c.rs`; `indexed_hl_memory_special_cases.rs` | INT; WAIT/snapshot cobertos nos caminhos microsequenciados pertinentes |
| DDCB/FDCB | WZ = endereço efetivo IX/IY+d | Soma 16-bit com wrap | WZ direto; BIT usa bits 5/3 do byte alto do endereço | `indexed_cb_e5.rs`; `z80_indexed_cb_reference_vectors.rs`; `z80_indexed_cb_bit_res_set_wz.rs` | INT + referências de software; não é prova física |
| `LD (nn),IX/IY` / `LD IX/IY,(nn)` | WZ = nn + 1 | 16-bit; wrap | WZ direto e acessos little-endian | `indexed_absolute_transfer_e3a2a.rs` | INT |
| `LD SP,IX/IY` e famílias BASE que preservam WZ | WZ permanece inalterado quando o handler não define nova regra | `WZ_old → WZ_old` | Estado WZ antes/depois | `ld_sp_f005.rs`; `base_load_f001.rs`; `base_register_pair_f004.rs` | INT |
| BIT `(HL)` | WZ não é atualizado pelo CB register/memory handler; X/Y vêm do byte alto do WZ existente | WZ anterior permanece | Flags X/Y são a observação indireta | `z80_cb_exhaustive_semantics.rs` | INT; o valor auxiliar não é observável como registrador em hardware |
| RRD/RLD | WZ = HL + 1 | Incremento de 16 bits; `FFFF → 0000` | WZ direto e subsequente BIT 0,(HL) observa X/Y | 262.144 vetores em `cpu.rs::ed_rrd_rld_exhaustive_nibble_transform_and_flag_contract`; 16.384 cenários observáveis RRD/RLD→BIT 0,(HL) (PR #934); 240 cenários micro com WAIT/snapshot em `ed_nibble_rotate_f023_observable.rs`; Woodmass test 09, linhas RLD `117F` e RRD `34AE` | HW para regra WZ=HL+1 no escopo do teste 09; INT para transformação/WAIT/snapshot; X/Y ainda COMP |
| LDI/LDD/CPI/CPD simples | LDI/LDD preservam WZ; CPI/CPD atualizam WZ = WZ anterior ± 1 conforme direção | LDI/LDD: `WZ_old → WZ_old`; CPI/CPD: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `cpu.rs::ed_block_simple_forms_exhaustively_preserve_flags_and_track_wz` (524.288 casos); PR #951 acrescenta WZ e previous-Q à matriz micro de 786.432 casos | INT; sem oracle físico independente para esta regra |
| LDIR/LDDR/CPIR/CPDR quando repetem | WZ = PC após recuo + 1 | PC recua 2 bytes; WZ = PC+1 | Estado WZ e interrupção/retomada | `z80_ed_block_repeat_interrupt_observation.rs`; `z80_ed_block_transfer_compare_boundaries.rs`; comparação MAME/ares no relatório de interrupção | COMP + INT; não é prova física |
| LDIR/LDDR/CPIR/CPDR quando terminam | LDIR/LDDR preservam WZ; CPIR/CPDR atualizam WZ = WZ anterior ± 1 conforme direção | Transferências: `WZ_old → WZ_old`; comparações: incrementa/decrementa WZ anterior com wrap 16-bit | Estado WZ diretamente | `z80_ed_block_transfer_compare_boundaries.rs`; testes unitários em `cpu.rs` | INT; temporal micro WZ ainda precisa ser ligado à matriz de retomada F-022 |
| INI/IND/INIR/INDR/OUTI/OUTD/OTIR/OTDR | Repetindo: WZ = PC+1. Não repetindo: base é BC antigo para input e BC novo para output; WZ = base ± 1 | Wrap 16-bit; direção conforme IN/OUT e incremento/decremento | Estado WZ, portas e memória | `cpu.rs::execute_block_io`; `z80_ed_block_io_reference_vectors.rs`; relatório F-021 (390 B-loop records) | COMP: SingleStepTests é traduzido do Ares; não é oráculo físico. Manter F-021 separado de F-022 |
| ED NEG, LD I/R, IM e ED reservados | WZ é preservado nos handlers que não o atualizam | `WZ_old → WZ_old` | Estado WZ após instrução | `z80_ed_neg_nibble_exhaustive.rs`; `z80_ed_special_registers_exhaustive.rs`; `ed_reserved_f033.rs` | INT; não generalizar codificações reservadas a CMOS/clones |

## Lacunas que permanecem explícitas

1. **Immediate I/O:** D3/DB têm regra no executor; a asserção WZ específica para os dois opcodes e os dois tipos de wrap está proposta na PR #954 e aguarda CI.
2. **IM0/IM1, RETN/RETI e RST:** os caminhos existem e há cobertura de acknowledge/stack; as asserções WZ adicionais estão propostas na PR #954 e aguardam CI.
3. **Block temporal:** LDI/LDD/CPI/CPD simples agora têm WZ e previous-Q na matriz micro F-022 pela PR #951. A repetição tem WZ=PC+1 na matriz de interrupção; falta demonstrar explicitamente snapshot/restore microsequenciado de WZ nas quatro formas de repetição de transferência/comparação.
4. **WZ de instruções não documentadas:** regras de MAME/Ares e valores do core não são promovidos a comportamento universal. Os quatro dígitos `117F` e `34AE` do Woodmass são assinaturas de resultado das linhas, não valores literais de WZ.
5. **Hardware SMS e variantes:** teste Woodmass é hardware-derived no perfil coberto; hardware Master System não foi executado nesta auditoria. CMOS, NEC/R800/Z180, Z80N e clones fora do perfil continuam NOT_VERIFIED.

## Decisão F-023

A regra RLD/RRD → HL+1 possui evidência hardware-derived rastreável. As demais famílias estão separadas entre regras internas testadas, compatibilidade comparativa e lacunas de teste. **F-023 permanece AUDIT_PARTIAL** até fechar os cinco gaps listados acima ou documentar, com fonte e justificativa, quais comportamentos são deliberadamente não verificáveis no escopo NMOS.

Nenhuma alteração de produção é autorizada por esta matriz. Não iniciar E8/ZEXDOC/ZEXALL.
