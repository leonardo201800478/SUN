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


## 6. Partição dos 167 Ignored por campos x/z do opcode BASE

A partir do opcode terminal (sem DD/FD), os 167 bytes foram agrupados pela decomposição Z80 x/y/z. A tabela registra quantidades por grupo; a lista exata de bytes continua no CSV ledger.

| Grupo | Família BASE | Quantidade Ignored |
|---|---|---:|
| x=0,z=0 | NOP, EX AF,AF', DJNZ, JR e JR cc | 8 |
| x=0,z=1 | LD BC/DE/SP,nn (HL→IX/IY já está em UsesIndex) | 3 |
| x=0,z=2 | transferências indiretas BC/DE e absolutas de A | 6 |
| x=0,z=3 | INC/DEC BC/DE/SP | 6 |
| x=0,z=4 | INC B/C/D/E/A | 5 |
| x=0,z=5 | DEC B/C/D/E/A | 5 |
| x=0,z=6 | LD B/C/D/E/A,n | 5 |
| x=0,z=7 | RLCA/RRCA/RLA/RRA/DAA/CPL/SCF/CCF | 8 |
| **x=0 subtotal** | | **46** |
| x=1 | LD r,r' BASE sem H/L e HALT | 26 |
| **x=1 subtotal** | | **26** |
| x=2 | ALU A,r BASE sem H/L e (HL) | 40 |
| **x=2 subtotal** | | **40** |
| x=3,z=0 | RET cc | 8 |
| x=3,z=1 | POP BC/DE/AF, RET e EXX | 5 |
| x=3,z=2 | JP cc,nn | 8 |
| x=3,z=3 | JP nn, IN/OUT (n), EX DE,HL, DI/EI | 6 |
| x=3,z=4 | CALL cc | 8 |
| x=3,z=5 | PUSH BC/DE/AF e CALL nn | 4 |
| x=3,z=6 | ALU A,n | 8 |
| x=3,z=7 | RST | 8 |
| **x=3 subtotal** | | **55** |
| **Total efetivo Ignored** | | **167** |

A contagem confirma que a lacuna não é “mais 167 variantes de IX/IY”: são opcodes BASE agrupáveis por famílias de execução. O plano de teste deve cobrir os diferentes formatos de request (sem operandos, imediato, relativo, memória indireta/absoluta, stack, I/O e controle de fluxo), e não apenas um representante genérico sem WAIT.

**Observação:** os subtotais x/z descrevem o decoder BASE e não substituem o ledger nem provam a semântica. As exceções especiais DD/FD/ED, IndexedCB e os bytes UsesIndex continuam fora deste conjunto efetivo de 167.

## 7. Pós-merge — cobertura estrutural integral do conjunto Ignored — 2026-11-16

**Main:** `3acf24c67d6e573120134b5b29ebbbf2a2893940`. PR #946 MERGED. O HEAD validado `162772ad8fba7249f8f4c7b8fb67b9816202cc80` tem o mesmo tree SHA integrado `77840465f65a3d6f4f522ea475e9f1d7a9f2c320`.

O novo teste `native/z80_cpu/tests/z80_dd_fd_ignored_fallback_matrix_f028.rs` lê o ledger e percorre os 167 registros efetivos Ignored sob DD e FD (334 execuções). Exige dois M1 em sequência, opcode terminal em PC 1 sem refetch, R +2 e custo total BASE +4T. Resultado PASS 1/1 no CI Z80 focused #43882287467; Rust quality/unit SUCCESS; SMS Runtime #43882287458 SUCCESS em format, Clippy, testes e build Windows x64.

A primeira execução CI/SMS (#43882092547) parou no rustfmt; a correção de formatação foi aplicada antes do HEAD validado. Não atribuir PASS à execução inicial.

**Limite:** o teste é uma prova estrutural de dispatch e custo M1, não prova independente de semântica, efeitos arquiteturais ou requests de barramento por família. WAIT/snapshot por tipo de request e comparação de endereços/ordem continuam pendentes. F-028 permanece AUDIT_PARTIAL.

## Atualização após PR #950 — 16/11/2026

A PR #950 não altera a classificação dos 85 opcodes UsesIndex. Ela adiciona validação de semântica BASE para os 167 opcodes Ignored, sem promover qualquer UseIndex a fallback. A reconciliação de 85 UsesIndex e 167 Ignored permanece preservada. PR #953 integrou a segunda fixture condicional e PC final exato (CI #43891989147 e SMS Runtime #43891989127 PASS no HEAD validado). PR #954 não altera a classificação.
