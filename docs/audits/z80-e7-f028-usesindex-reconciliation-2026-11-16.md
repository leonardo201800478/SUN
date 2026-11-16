# F-028 — reconciliação dos 256 bytes DD/FD com famílias de teste

**Data:** 2026-11-16
**Baseline main:** `caa9a76a6f09d588155ea4e0cbc95f487f079143`
**Ledger:** `docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv` (256 registros).

## 1. Partição efetiva da FSM

| Resultado efetivo após DD/FD | Quantidade | Significado |
|---|---:|---|
| UsesIndex | 85 | IX/IY substitui HL/H/L ou participa da forma indexada |
| Ignored | 167 | Executa a semântica BASE, sem refetch do opcode terminal |
| IndexedCB | 1 | CB transfere para DDCB/FDCB |
| ED cancel | 1 | ED cancela o latch IX/IY e transfere para o decoder ED |
| Prefix repeat | 2 | DD/FD substitui a seleção anterior (last-prefix-wins) |
| **Total** | **256** | |

A função auxiliar `classify_index_opcode()` retorna 85 UsesIndex, 170 Ignored e 1 IndexedCB. Os três bytes DD/FD/ED são pré-processados pela FSM; por isso, o resultado auxiliar não é a partição efetiva. Não confundir 170 Ignored do helper com 167 Ignored da FSM.

## 2. Reconciliação exata dos 85 UsesIndex

A extração do CSV foi reconciliada byte a byte. Os grupos abaixo são disjuntos e totalizam exatamente 85:

| Família já existente | Quantidade | Bytes após DD/FD | Testes / estado |
|---|---:|---|---|
| E3A1 — operações IX/IY 16-bit | 9 | 09, 19, 21, 23, 29, 2B, 39, E9, F9 | Testes E3A1 no sequenciador; preservar |
| E3A2 — transferência absoluta, stack e EX (SP) | 5 | 22, 2A, E1, E3, E5 | `indexed_absolute_transfer_e3a2a.rs`, `indexed_stack_e3a2b.rs`, `indexed_exchange_stack_e3a2c.rs`; preservar |
| E4A — LD com displacement | 15 | 36, 46, 4E, 56, 5E, 66, 6E, 70, 71, 72, 73, 74, 75, 77, 7E | `indexed_memory_e4a.rs`; manter exceções H/L reais em 66/6E/74/75 |
| E4B — ALU A,(IX/IY+d) | 8 | 86, 8E, 96, 9E, A6, AE, B6, BE | `indexed_alu_e4b.rs`; fonte 6 permanece memória |
| E4C — INC/DEC (IX/IY+d) | 2 | 34, 35 | `indexed_incdec_e4c.rs`; preservar |
| F-036 — LD r,r' register-only | 24 | 44, 45, 4C, 4D, 54, 55, 5C, 5D, 60, 61, 62, 63, 64, 65, 67, 68, 69, 6A, 6B, 6C, 6D, 6F, 7C, 7D | `indexed_half_register_ld_e7.rs`; F-036 CLOSED |
| F-028 — INC/DEC/LD imediato de half-register | 6 | 24, 25, 26, 2C, 2D, 2E | PR #680 integrada; teste `dd_fd_index_half_inc_dec_and_immediate_load_match_atomic_with_waits` no sequenciador |
| F-037 — ALU A,IXH/IXL/IYH/IYL | 16 | 84, 85, 8C, 8D, 94, 95, 9C, 9D, A4, A5, AC, AD, B4, B5, BC, BD | `indexed_half_register_alu_e7.rs`; F-037 CLOSED |
| **Total** | **85** | | |

**Conclusão desta reconciliação:** não há uma família UsesIndex sem destino documental. Os 85 bytes já se distribuem entre E3A1/E3A2, E4A/E4B/E4C, F-036, os seis opcodes integrados por PR #680 e F-037. Essa partição não prova novamente a semântica de cada família; os estados de fechamento anteriores permanecem preservados. Não reabrir E3/E4/F-036/F-037 sem regressão concreta.

## 3. Cobertura observada dos 167 Ignored

Os 167 bytes efetivos Ignored são instruções BASE cujo prefixo DD/FD não altera a semântica. A evidência atual é representativa, não exaustiva:

| Área | Evidência existente | Limite |
|---|---|---|
| Fallback geral sem refetch | `prefix_fallback_e6.rs`: 9 testes para opcode já buscado, WAIT/snapshot, imediato, ALU imediato, branch relativo, leitura/escrita de memória e entrada ED | Não enumera os 167 bytes em contexto prefixado |
| Exceções de troca e HALT | PR #943: EX AF,AF', EXX, EX DE,HL e HALT em nove cadeias; ambos executores, WAIT/snapshot | 12 formas dirigidas, não todos os opcodes BASE |
| Controle/flags e ED | PR #943: NEG, NOP, DAA, CPL, SCF, CCF, DI, EI em cadeias DD/FD | Não substitui cobertura de todas as famílias BASE |
| Prefixos repetidos/alternados | `prefix_fsm.rs`: last-prefix-wins, ED cancel e stream de 257 prefixos | Não prova a semântica de cada opcode terminal |
| INT/NMI entre prefixos | PR #943: INT e NMI solicitados entre DD e FD; atendimento após opcode terminal | Um opcode terminal NOP; não cruza as famílias dos 167 bytes |
| HALT e indexed-CB | HALT permanece BASE/Ignored; CB é IndexedCB e segue E5 | Não confundir HALT com UsesIndex nem CB com fallback BASE |

## 4. Próximo trabalho F-028

1. Gerar uma lista das 167 entradas Ignored agrupada por família BASE (register-only, immediate, relative branch, absolute/indirect memory, stack/call/return, I/O, control/flags e HALT), referenciando o ledger sem duplicar as exceções.
2. Relacionar cada grupo aos testes BASE existentes e marcar explicitamente o que já tem cobertura com DD/FD prefixado versus somente sem prefixo.
3. Para os opcodes implementados no microsequenciador, construir um harness de fallback que verifica: um M1 para cada prefixo, um único fetch M1 do opcode terminal, ausência de refetch, PC/R, T-states adicionais de prefixo e requests/ordem de operandos preservados.
4. Acrescentar WAIT e snapshot/restore nos safe points de cada classe de request; não presumir que um teste de memória read cobre immediate, stack, I/O ou branches.
5. Separar instruções BASE não implementadas e codificações reservadas; não contabilizá-las como PASS só por receberem a classificação Ignored.
6. Expandir INT/NMI durante cadeias somente para classes em que a aceitação no boundary ou a retomada difira; não multiplicar testes sem uma hipótese discriminante.

## 5. Limites e decisão

- Classificação dos 256 bytes: PASS no HEAD b3320ea da PR #943.
- 108 combinações de exceções dirigidas por executor: PASS no HEAD de código validado da PR #943.
- F-036/F-037 e E3/E4 permanecem preservadas; não reabrir sem defeito reproduzível.
- Os 167 Ignored ainda não têm validação prefixada por opcode/família completa.
- F-028 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.
- Nenhuma alteração de produção CPU/Scheduler/runtime é justificada por esta reconciliação.
