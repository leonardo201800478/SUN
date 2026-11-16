# Z80 E7 — revisão adversarial F-023 (WZ/MEMPTR)

**Data:** 2026-11-16
**Main após PR #975:** `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`
**Perfil:** Zilog Z80 NMOS clássico, limitado aos consumidores Sega SMS/SG-1000.
**Resultado:** auditoria interna concluída; candidata a fechamento, pendente da cobertura PR #977 e de revisão independente registrada.

## 1. Escopo e método

A revisão confrontou a matriz `z80-e7-memptr-family-matrix-2026-11-16.md` com os handlers do executor atômico `native/z80_cpu/src/cpu.rs`, do microsequenciador `native/z80_cpu/src/sequencer.rs` e os testes apontados por família.

A revisão procurou especificamente:
- regras WZ sem família/classificação;
- condições de atualização incorretas ou ambíguas;
- diferenças entre caminhos tomado/não tomado;
- erros de wrap de 8/16 bits;
- diferenças entre execução atômica e microsequenciada;
- estado WZ que afeta flags observáveis;
- confusão entre DOC, HW, COMP e INT;
- extrapolação indevida para variantes fora do perfil.

A matriz foi corrigida para referências explícitas e para a baseline atual. Não foi necessária alteração de produção.

## 2. Resultado da inspeção

| Domínio | Resultado da revisão | Evidência principal |
|---|---|---|
| Reset, NMI e INT IM0/IM1/IM2 | Regras explícitas; WZ após vetor/serviço ou preservação conforme modo | `nmi_e7.rs`, `snapshot_restore_e7_2e.rs` |
| LD indireto/absoluto e transferências de pares | Endereços efetivos, incremento e wrap 16-bit coerentes com a matriz | F-002/F-003/F-004 e testes de transferência absoluta |
| ADD/ADC/SBC HL/IX/IY | WZ usa o valor antigo do par de destino + 1; wrap explícito | E3A1 e testes de aritmética ED |
| JR/DJNZ, JP/CALL, RET e RST | Tomado/não tomado, alvo, retorno e stack tratados separadamente | `z80_conditional_control_flow_coverage.rs` |
| IN/OUT e block I/O | Regras distintas para incremento dentro da página, incremento 16-bit e direção IN/OUT | testes ED I/O e block I/O; F-021 permanece separado de F-022 |
| EX (SP),HL/IX/IY | WZ recebe a palavra lida da pilha, não o endereço SP | `exchange_stack_hl_and_di_ei_update_state`, E3A2C |
| IX/IY+d e DDCB/FDCB | WZ usa o endereço efetivo com displacement assinado e wrap | E4A/E4B/E4C/E5 |
| CB BIT (HL) | WZ é preservado; X/Y derivam do byte alto de WZ | `z80_cb_exhaustive_semantics.rs` |
| RRD/RLD | WZ = HL+1; observação indireta via BIT (HL) | matriz F-023 e Woodmass test 09 |
| LDI/LDD/CPI/CPD | Transferência preserva WZ; comparação ajusta WZ anterior conforme direção | matriz micro exaustiva F-022 |
| LDIR/LDDR/CPIR/CPDR | Repetição define WZ=PC+1; término tem regra distinta para transfer/compare | testes micro, Scheduler e snapshot/replay |
| ED reservadas/especiais | Preservação de WZ está explicitamente testada no perfil delimitado | F-033 e testes ED especiais |

**Achado:** não foi encontrado handler WZ relevante sem classificação na matriz nem divergência de produção entre as regras listadas e os handlers inspecionados para o perfil NMOS delimitado.

## 3. Evidência e proveniência

- RRD/RLD: 262.144 vetores de transformação/flags; 16.384 cenários observáveis RRD/RLD→BIT (HL); 240 cenários micro com WAIT de fixture e snapshot/restore. Woodmass test 09 fornece evidência hardware-derived específica para WZ=HL+1; as assinaturas RLD `117F` e RRD `34AE` são resultados do teste, não valores literais de WZ.
- Block transfer/compare simples: 786.432 combinações microsequenciadas, com fórmula independente de flags e WZ.
- Block repeat: matrizes de PC/BC/flags/Q e integração de interrupções; PR #972 acrescentou observação INT IM2 e NMI durante repetição no SMS Scheduler.
- A execução de WAIT nos testes é fixture determinística; não representa medição elétrica de WAIT físico.
- O UM0080 não documenta WZ como registrador arquitetural. As regras não documentadas permanecem INT e/ou COMP. Nenhuma foi promovida artificialmente a DOC ou HW.
- CMOS, NEC, R800, Z180, Z80N e clones não cobertos permanecem NOT_VERIFIED.

## 4. Validação vinculada aos SHAs

| Validação | SHA | Resultado |
|---|---|---|
| Z80 focused, SingleStepTests DD/FD, Z80 Block Flags ROM v5.0 e Woodmass | PR #975 HEAD `7574245338549fd8f37a7483ef41b7fbc306ffe5`, CI run [#43937118700](https://github.com/leonardo201800478/sega-emulator/actions/runs/43937118700) | SUCCESS; jobs Z80 focused e Rust quality/unit PASS |
| SMS Runtime Release format, Clippy, testes e build Windows x64 | PR #975 HEAD `7574245338549fd8f37a7483ef41b7fbc306ffe5`, run [#43937118785](https://github.com/leonardo201800478/sega-emulator/actions/runs/43937118785) | SUCCESS |
| Integração INT IM2/NMI durante repetição | PR #972 HEAD `ff922996e797b287e275fdbf1d1791314c61a749`, run #43918490165 | Rust workspace regression SUCCESS; `snapshot_restore_e7_2e`: 13 passed, 0 failed; Z80 focused e Scheduler stress SKIPPED |
| Merge da PR #975 | `22c489e03aaed440bbb150ee9a4b7b70c74b1b84` | Integrado; alteração exclusivamente de teste |

Os resultados acima são vinculados aos respectivos HEADs. Não se declara workflow push pós-merge no SHA 22c489e como executado. Scheduler deterministic stress não foi executado na PR #975.

## 5. Decisão

**F-023: INTERNAL_REVIEW_COMPLETE — CANDIDATE FOR CLOSURE.** A revisão interna da matriz está concluída; o fechamento formal depende da validação da PR #977 e de revisão independente registrada.

O fechamento significa que as famílias modeladas possuem regra, condição, wrap, observabilidade e teste rastreável, com proveniência classificada sem extrapolação. Não significa medição física individual de todas as regras internas nem conformidade universal de variantes/clones.

**E7 global permanece IN_PROGRESS.** F-022 e F-028 ainda precisam de revisão adversarial final e reconciliação documental; os gates finais integrados no HEAD candidato também permanecem necessários. E8/ZEXDOC/ZEXALL continua BLOCKED.



## Revalidação cruzada F-023/F-028 — PR #977 (16/11/2026)

A PR #977 fechou a lacuna técnica de WZ para JR, DJNZ e JR NZ/Z/NC/C sob DD/FD Ignored, nos caminhos tomado/não tomado e com PC inicial `FFFD` para wrap do destino relativo. Também verifica PC, WZ, preservação quando não tomado, fetch M1 e trace normalizado.

- HEAD validado: `a783a3c231b5de1678ffc71eb8b1e8dcda42c724`.
- CI #43939973117: SUCCESS; Z80 focused e Rust quality/unit PASS.
- SMS Runtime #43939957730: SUCCESS; Release format/Clippy/tests PASS.
- Scheduler deterministic stress: SKIPPED. Outros jobs não relacionados: SKIPPED.
- Nenhuma alteração de produção; a alteração é teste-only.

A cobertura interna F-023/F-028 está completa para esse limite. O estado continua **INTERNAL_REVIEW_COMPLETE — CANDIDATE FOR CLOSURE**, pois ainda não existe review independente registrada no GitHub. Não declarar CLOSED nem integrar a documentação até satisfazer esse gate.
