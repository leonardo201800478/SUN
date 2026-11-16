# Checklist mestre de auditoria e fechamento do Z80

**Documento operacional:** atualizar este checklist a cada etapa efetivamente concluída, sempre vinculando evidências ao SHA integrado na `main`.
**Baseline remota reconciliada:** `main` = `22c489e03aaed440bbb150ee9a4b7b70c74b1b84` (2026-11-16, após PR #975). Histórico anterior preservado abaixo. Baseline de runtime/validação DD/FD histórica: PR #702, HEAD `34e85637eaac8ed2777ecc5cb9cd2e91ba137235`, com 34.000/34.000 vetores externos PASS; não atribuir esses resultados ao HEAD atual.
**Baseline documental reconciliada:** scorecard, matriz DD/FD e checklist foram atualizados pelas PRs #687 e #688; baselines históricas permanecem preservadas e não representam a main corrente.
**Núcleo canônico:** `native/z80_cpu` (`sega-z80-cpu`).
**Perfil inicial de referência:** Zilog Z80 NMOS usado pelas plataformas Sega documentadas. Não generalizar automaticamente para Z80 CMOS, Z180, R800 ou clones.
**Estado global:** `IN_PROGRESS`. E8/ZEXDOC/ZEXALL permanece bloqueada até o fechamento formal de E7.

> Este documento é um inventário de trabalho, não uma declaração de conformidade. Código presente, decoder sem erro, smoke test, igualdade entre dois caminhos internos ou execução de jogos não bastam isoladamente para marcar um item como concluído.

## 1. Estados e regras de evidência

### 1.1 Estado da auditoria de cada item

| Estado | Significado |
|---|---|
| `NOT_AUDITED` | O item ainda não recebeu inspeção completa e rastreável. |
| `AUDIT_PARTIAL` | Há inspeção/evidência parcial; faltam casos, variantes ou fontes. |
| `GAP_CONFIRMED` | Divergência ou ausência reproduzível identificada. |
| `IMPLEMENTED_PENDING_VALIDATION` | Existe implementação candidata, mas falta validação/gate. |
| `CLOSED` | Critérios de aceite satisfeitos e alteração integrada na main. |
| `BLOCKED` | Não é possível avançar por dependência ou falta de evidência identificada. |
| `NOT_APPLICABLE` | Item não se aplica ao perfil de hardware explicitamente delimitado, com justificativa. |

### 1.2 Classificação obrigatória de execução

Cada suíte/comando executado recebe exatamente um estado:

- `PASS`: executado no SHA indicado e aprovado.
- `FAIL`: executado no SHA indicado e falhou.
- `BLOCKED`: execução impedida por dependência/ambiente identificados.
- `NOT RUN`: não executado.

`SKIPPED` nunca equivale a `PASS`. Não transferir resultados de outro SHA.

### 1.3 Evidência mínima para fechar uma família

1. Inventário de bytes/opcodes e classificação de instruções documentadas, aliases, prefixos ignorados e encodings reservados.
2. Inspeção do decoder e do executor real, inclusive fallbacks e caminhos de erro.
3. Testes de resultado arquitetural: registradores, flags, PC, SP, R, I, WZ/MEMPTR, Q, memória e I/O conforme aplicável.
4. Vetores de fronteira e varreduras exaustivas quando o domínio permitir.
5. Diferencial contra referência independente, registrando proveniência e limitações. O executor atômico do próprio projeto pode ser comparador interno, não oráculo independente.
6. Validação temporal somente na granularidade exigida por efeitos observáveis da plataforma: transações, ordenação, T-states, WAIT, interrupções e periféricos.
7. Snapshot/restore e equivalência contínua versus retomada nos pontos suspensíveis aplicáveis.
8. Regressão SMS e SG-1000 e gates de qualidade no HEAD exato.
9. Atualização deste checklist, scorecard, matriz aplicável e relatório da auditoria.

### 1.4 Regra de granularidade temporal

A semântica funcional correta é obrigatória para toda instrução suportada. Não se deve criar microfases artificiais para operações internas sem efeito intermediário observável. Microsequenciamento e traces temporais são obrigatórios onde a ordem/duração da interação com barramento, I/O, VDP/PSG, mapper, IRQ/NMI, HALT, WAIT ou scheduler altera o resultado observável. Não declarar precisão elétrica ou de pinos sem evidência específica.

## 2. Ordem de auditoria e prioridade

A sequência abaixo é uma ordem inicial de trabalho por frequência esperada e impacto em software de console, não uma medição estatística de jogos. Ajustar a prioridade quando houver dados confiáveis de perfis de execução. Nenhum item é descartado por ser raro.

| Ordem | Prioridade | Grupo |
|---:|---|---|
| 1 | P0 — muito alta | Loads, transferências e registradores de 8 bits |
| 2 | P0 — muito alta | ALU de 8 bits, flags e operações lógicas |
| 3 | P0 — muito alta | INC/DEC, pares de 16 bits e aritmética |
| 4 | P1 — alta | Fluxo de controle, chamadas, retornos e pilha |
| 5 | P1 — alta | CB: rotações, shifts e manipulação de bits |
| 6 | P1 — alta | DD/FD: IX/IY, substituições e exceções H/L |
| 7 | P1 — alta | Memória indexada, DDCB/FDCB e prefixos mistos |
| 8 | P1 — alta | I/O e efeitos observáveis de barramento |
| 9 | P2 — média | Interrupções, HALT, EI/DI, IM e registradores especiais |
| 10 | P2 — média | ED aritmético, transferências e aliases |
| 11 | P2 — média | ED operações de bloco e repetição |
| 12 | P2 — transversal | Estado, snapshots, determinismo e integração temporal |
| 13 | P3 — fechamento | Instruções não documentadas, variantes e auditoria final de cobertura |

## 3. Checklist de cobertura funcional

**Nota:** `CÓDIGO PRESENTE` não significa `CLOSED`. Os estados abaixo são a fotografia inicial baseada na inspeção documental/código da baseline; devem ser promovidos somente após a revisão específica e validação da etapa.

| ID | Item a examinar | Estado inicial | Evidência / próxima ação |
|---|---|---|---|
| F-001 | LD r,r', LD r,n, LD r,(HL), LD (HL),r/n | CLOSED (NMOS delimitado) | Integrada pela PR #721 no squash commit `37047ca2739dcfcf1fae6d0fb7bff93dafcaed0b`; 28.161 vetores, oracle independente, flags/Q/WZ/R/PC, WAIT, snapshot/restore e equivalência scheduler contínua/retomada. CI #6091 PASS no HEAD final da PR. Sem validação física SMS; CMOS/clones NOT_VERIFIED. |
| F-002 | LD A,(BC)/(DE), LD (BC)/(DE),A | CLOSED (NMOS delimitado) | Integrada pela PR #722 no squash commit `29462cf0a788d5905de7a5e9e0499a1f1361fec1`; 5.120 vetores, WZ/MEMPTR, wrap, flags/Q/R, WAIT, snapshot/restore e integração temporal. CI #6097 PASS no HEAD final da PR. Sem validação física SMS; CMOS/clones NOT_VERIFIED. PR #720 foi encerrada como supersedida, sem merge. |
| F-003 | LD A,(nn), LD (nn),A | CLOSED (NMOS delimitado) | PR #724 integrada por squash em `7fde212aabc07e8c4b7df56938f8845f99d29499`; HEAD validado `455ae4ea5ce68fdba53de1958fe0123afd39dc6d`; CI #6111 / run `42675073118` PASS. 2.560 vetores, WZ, PC wrap, 13T/4 M-cycles, WAITs diferenciados, snapshot/restore e integração SMS. Rustfmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD 34.000/34.000, docs hygiene e agregador PASS. Workspace integral e hosts não relacionados NOT RUN/SKIPPED. Nenhuma alteração de runtime. CI pós-merge não confirmado; hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED. PR #723 encerrada sem merge. |
| F-004 | LD rr,nn e transferências entre pares | CLOSED (NMOS delimitado) | PR #726 integrada por squash em `6ce8399aa96937547e72c24e75f1521dc5a9d550`; HEAD validado `d4e0c6bcf7b339fee8674102fe26e5b2dd4d131a`; CI #6122 / run `42676982272` PASS. 1.052 vetores imediatos + 2.560 absolutos, seis casos PC-wrap, WAITs, snapshot/restore, diferencial atômico/micro e integração SMS. Rustfmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD 34.000/34.000, docs hygiene e agregador PASS. Workspace integral e hosts não relacionados NOT RUN/SKIPPED. Runtime não alterado. DD/FD E3A1/E3A2, ED F-034 e SP F-005 não duplicados. CI pós-merge NOT VERIFIED; hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED. |
| F-005 | LD SP,HL/IX/IY; LD (nn),SP; LD SP,(nn) quando aplicável | AUDIT_PARTIAL | Confirmar semântica e tempos nominais. |
| F-006 | EX DE,HL; EX AF,AF'; EXX; EX (SP),HL/IX/IY | AUDIT_PARTIAL | Conferir bancos alternativos, ordem de barramento e WZ. |
| F-007 | PUSH/POP de todos os pares, inclusive AF e IX/IY | AUDIT_PARTIAL | Conferir ordem da pilha, wrap e flags. |
| F-008 | ADD/ADC/SUB/SBC de 8 bits | AUDIT_PARTIAL | Vetores discriminantes de S/Z/H/PV/N/C/X/Y. |
| F-009 | AND/OR/XOR/CP e CP sem alteração de A | AUDIT_PARTIAL | Conferir flags, X/Y e preservações. |
| F-010 | INC/DEC de registradores e memória | AUDIT_PARTIAL | Exaustividade de valores e preservação de carry. |
| F-011 | ADD HL,rr e variantes IX/IY | AUDIT_PARTIAL | Carry/half-carry, WZ e flags preservadas. |
| F-012 | ADC/SBC HL,rr | AUDIT_PARTIAL | Todos os pares, carry de entrada e fronteiras. |
| F-013 | INC/DEC de pares de 16 bits | AUDIT_PARTIAL | Wraparound e flags preservadas. |
| F-014 | DAA, CPL, NEG, CCF e SCF | AUDIT_PARTIAL | Fórmulas e flags X/Y/Q; NEG inclui aliases ED. |
| F-015 | RLCA/RRCA/RLA/RRA | CLOSED (perfil NMOS delimitado) | PR #728 integrada por squash em `ce51578511a0090afbba3ded6312c9a25c5d78c7`; HEAD validado `c198cff1629270378a3bdd22a1126ca3d98a44b8`; CI #6131 / run `42678804879` PASS no HEAD exato. 262.144 vetores A×F, 1.024 snapshot/restore, quatro PC-wrap e 192 casos SMS/MachineScheduler. Rustfmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD 34.000/34.000, docs hygiene e agregador PASS. Workspace integral e jobs não relacionados NOT RUN/SKIPPED. Runtime não alterado. X/Y comparativo; sem prova física. CI pós-merge NOT VERIFIED; CMOS/clones NOT_VERIFIED. |
| F-016 | RLC/RRC/RL/RR/SLA/SRA/SLL/SRL | AUDIT_PARTIAL | Matriz CB para todos os destinos aplicáveis. |
| F-017 | BIT/RES/SET para registradores e (HL) | AUDIT_PARTIAL | Flags BIT, ausência de write-back e endereços. |
| F-018 | JP/JR/DJNZ, condicional e incondicional | AUDIT_PARTIAL | PC, offsets assinados, wrap e tempos tomados/não tomados. |
| F-019 | CALL/RET/RST e variantes condicionais | AUDIT_PARTIAL | Pilha, PC, tempos e condições. |
| F-020 | IN A,(n), OUT (n),A e ED IN/OUT (C) | AUDIT_PARTIAL | Portas, valor, flags, WZ e efeitos colaterais. |
| F-021 | INI/IND/INIR/INDR e OUTI/OUTD/OTIR/OTDR | AUDIT_PARTIAL | Flags, B, HL, WZ, L pós-atualização, repetição e evidência NMOS. |
| F-022 | LDI/LDD/LDIR/LDDR e CPI/CPD/CPIR/CPDR | AUDIT_PARTIAL | PR #712 cobre LDIR/LDDR/CPIR/CPDR interrompidos por IM2, flags via PUSH AF, RETI, retomada e snapshot/restore. PR #736 integrada em `55585bf2425f1c4eaf61c4a3a3ad42f9499c4797`, HEAD `1520cc77e2801d985401a5518eec9da712b89e3c`, CI #6166 / run `42698792012` PASS: 524.288 casos atômicos para LDI/LDD/CPI/CPD, flags e WZ. Evidência adicional: PR #936 (Draft), HEAD `a7f551336104e1c2c03d7e6790b0aafbb5e346e7`, CI #43846321428 PASS; ROM v5.0 executada (1 passed/0 failed), ≥60 comparações principais e 390 B-loop records em OTIR×2/OTDR×2/INIR/INDR, flags e B exatos. A suíte upstream foi testada em ZX Spectrum 128/Spectrum+ NMOS, mas declara cobertura limitada e não exaustiva. Ainda falta matriz completa de iteração simples e microsequenciada; hardware SMS e CMOS/clones não verificados. Não fechar F-022 por esta evidência isolada. |
| F-023 | RRD/RLD | AUDIT_PARTIAL | PR #714 integrada: 262.144 vetores atômicos e 16 microsequenciados com WAIT/snapshot. Evidência externa adicional: PR #942 (Draft), HEAD `7402ff138ced95da2d8b9e0008919c8cd22484f1`, CI #43850972785 PASS; suíte MEMPTR Mark Woodmass test 09 executada, 50.904.931 T-states, FNV-1 `F5AE5140`, 61 linhas/32 colunas, HALT 0x80E6. A suíte hardware-derived foi comparada upstream com NEC D780C-1 e outros clones; tabela MEMPTR inclui RLD/RRD → HL+1. TAP/ROM baixados somente no CI, com hashes verificados, sem binários commitados. A evidência reduz a lacuna WZ/MEMPTR, mas não fecha X/Y não documentados, matriz temporal completa, hardware SMS ou CMOS/clones. F-023 permanece AUDIT_PARTIAL. |
| F-024 | LD I,A; LD R,A; LD A,I; LD A,R | CLOSED (perfil NMOS delimitado) | PR #716 integrada em `56d75cc79803494e9fd8a54e8930871c9101fc84`; HEAD validado `9338f8dfee679fee6270c01f9670d6b678d219a0`; CI #6062/#6063 PASS. 4.096 vetores cobrem os quatro opcodes, todos os bytes, IFF2/C, refresh, flags e P; WAIT/snapshot no microsequenciador; erratum INT em ambos os executores. X/Y é referência MAME/ares, não especificação UM0080. Sem hardware SMS ou CMOS/clones. |
| F-025 | IM 0/1/2, RETN e RETI | CLOSED (perfil NMOS SMS/SG-1000) | PR #718 MERGED em `f0cfc7b9df1834ae8ad418c6241702e9003f072e`; HEAD validado `24fd53a5eb7c36462fa6ebc332e1a15b1504cbce`; CI #6071 / run `42658617730` PASS. Oito aliases RETN/RETI × IFF2 0/1 × SP normal/FFFF/0000 em ambos os executores; IFF1 inicial invertida; PC/WZ, SP wrap, R, ordem low/high, WAIT e snapshot/restore. 14T nominal / 18T com quatro WAITs. IM aliases já têm testes independentes. RETI daisy-chain NOT_APPLICABLE sem consumidor SMS/SG-1000. CI pós-merge #6072 / run `42658844499` PASS no HEAD exato da main; hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED. |
| F-026 | HALT, NOP e estado após HALT | AUDIT_PARTIAL | Fetch/refresh, R, saída por interrupção e snapshot. |
| F-027 | DD/FD substituição HL por IX/IY | AUDIT_PARTIAL | Matriz por opcode e exceções. |
| F-028 | IXH/IXL/IYH/IYL: LD, INC/DEC, ALU e transferências | AUDIT_PARTIAL | PR #702 MERGED, main `401bf4fb1ba8add6c84420cad5c9d2a62ceba72b`: correção Q/previous-Q nos dois executores. SingleStepTests DD 37/FD 3F: 2.000/2.000 PASS; F-037: 32.000/32.000 PASS. Total 34.000 PASS; runs #42626811588/#42626765522. Permanecem auditoria das demais exceções, prefixos, HALT e estado arquitetural. E4A/E4B preservados. |
| F-029 | (IX+d)/(IY+d): loads, stores e RMW | AUDIT_PARTIAL | Displacement assinado, wrap, exceções H/L e ordem de acesso. |
| F-030 | DDCB/FDCB: BIT/RES/SET/rotate/shift e destinos | AUDIT_PARTIAL | Matriz completa por prefixo, subopcode e destino. |
| F-031 | Prefixos repetidos/alternados DD/FD e last-prefix-wins | AUDIT_PARTIAL | Verificar consumo, PC/R, M1 e semântica terminal. |
| F-032 | Prefixos DD/FD ignorados antes de BASE/CB/ED | AUDIT_PARTIAL | Verificar não refetch, sem substituição indevida e custo prefixado. |
| F-033 | Encodings ED reservados e fallback ED-NOP | CLOSED | PR #706 integrada; HEAD validado `b2291a83848e236d2354d8f03e22b831f9030b72`, CI #42635309051 SUCCESS. A suíte `ed_reserved_f033.rs` cobre os 178 bytes, aposentadoria após dois M1, 8T nominal, PC/R, F/Q/previous-Q/WZ/P/IFF/IM, fase Ready, ausência de request pendente, WAIT e snapshot/restore. CLOSED somente como perfil de compatibilidade Zilog NMOS clássico; UM0080 não especifica universalmente os reservados; sem generalização a CMOS/R800/Z80N/clones.
| F-034 | Encodings ED não documentados e aliases ED | CLOSED (escopo ED, perfil NMOS delimitado) | PR #709 integrada; HEAD validado `b6e5db79047e7802620e76ad98f5cfd08deefd57`, CI #6034 SUCCESS; reconciliação documental PR #710 no HEAD `715a9fc739db731cea6572c8d56dc41b4f2df6fe`, CI #6036 SUCCESS. ED70 256/256, ED71 como OUT(C),0 observado no perfil NMOS, NEG oito aliases × 256 com fórmula independente, aliases RETN/RETI e modos IM aceitos pelo perfil. RETI daisy-chain NOT_APPLICABLE aos perfis SMS/SG-1000 atuais. Não generalizar a CMOS/R800/Z80N/clones nem alegar validação física. |
| F-035 | RESET e estado arquitetural inicial | AUDIT_PARTIAL | Testes já documentados; revisar campos preservados e contrato de host. |
| F-036 | DD/FD LD r,r' register-only com IXH/IXL/IYH/IYL | CLOSED | PR #684 integrada em `0b9b298f0f9791a6fa6db17580ad741db5c0515d`. CI #42614755935 PASS no HEAD exato `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3`: Z80 focused, docs hygiene e agregador PASS; D3D12/áudio/SDL3/workspace SKIPPED. 48 combinações, oracle de registradores, comparação atômica, 8T/2 M1, WAIT/snapshot, prefixos repetidos e HALT. As exceções 66/6E/74/75 permanecem no caminho E4A. |
| F-037 | DD/FD ALU A,IXH/IXL/IYH/IYL | CLOSED | PR #686 merge SHA e testes internos preservados. Validação externa adicional: PR #692, HEAD `1606556814a8746a445d3ad32fc948ab97f0fde5`, CI #42622828177 PASS; 32 fixtures SingleStepTests × 1.000 = 32.000/32.000 no microsequenciador retomável. Merge SHA/main `1304bc0438e3ea333719d0ba3a495e2db9089da3`. Corpus Ares-derived; não é prova de silício. |

## 4. Checklist de estado arquitetural e flags

| ID | Item | Estado inicial | Próxima ação |
|---|---|---|---|
| S-001 | AF/BC/DE/HL e bancos alternativos | AUDIT_PARTIAL | Cobrir leitura, escrita, EX/EXX e preservação. |
| S-002 | IX/IY e metades de 8 bits | AUDIT_PARTIAL | Matriz completa de uso e exceções. |
| S-003 | SP/PC/I/R | AUDIT_PARTIAL | Wrap, refresh e efeitos de fetch/interrupt. |
| S-004 | IFF1/IFF2, IM e EI delay | AUDIT_PARTIAL | Transições, snapshot e interrupções nas fronteiras. |
| S-005 | WZ/MEMPTR | AUDIT_PARTIAL | Tabela por família com vetores independentes. |
| S-006 | Q/previous-Q | AUDIT_PARTIAL | DD/FD→SCF/CCF RESOLVED pela PR #702; 2.000 vetores externos PASS, incluindo DD 37 e FD 3F, com snapshot/restore. O estado Q/previous-Q global nas demais famílias permanece sujeito à auditoria E7. |
| S-007 | S/Z/H/PV/N/C | AUDIT_PARTIAL | Fórmulas e fronteiras por família. |
| S-008 | X/Y e flags não documentadas | AUDIT_PARTIAL | Proveniência NMOS/CMOS e vetores discriminantes. |
| S-009 | Preservação de flags | NOT_AUDITED | Verificar instruções que preservam total ou parcialmente F. |
| S-010 | Estado interno e campos derivados | NOT_AUDITED | Verificar invariantes e ausência de duplicidade de fonte de verdade. |

## 5. Checklist temporal, barramento e interrupções

| ID | Item | Estado inicial | Próxima ação |
|---|---|---|---|
| T-001 | M1, opcode fetch e refresh | AUDIT_PARTIAL | Confirmar integração real entre M1 e sequencer. |
| T-002 | Memory Read/Write | AUDIT_PARTIAL | Ordenação, endereço/dado, duração e retomada. |
| T-003 | Port Read/Write | AUDIT_PARTIAL | Fronteira observável de I/O com VDP/PSG. |
| T-004 | INT acknowledge | AUDIT_PARTIAL | Revisar operação, vetor, WAIT, M1/IORQ e IM0/1/2. |
| T-005 | NMI | AUDIT_PARTIAL | Prioridade, acknowledge, pilha, PC e snapshot. |
| T-006 | HALT e amostragem de interrupção | AUDIT_PARTIAL | Fronteiras de aceitação e refresh. |
| T-007 | WAIT=0/1/múltiplo | AUDIT_PARTIAL | Matriz por classe de transação e ciclo suspenso. |
| T-008 | Ciclos internos e T-states nominais | AUDIT_PARTIAL | Conferir somente ciclos documentados/observáveis; não inventar fases. |
| T-009 | Integração com scheduler e clocks | AUDIT_PARTIAL | Confirmar causalidade CPU/VDP/PSG e execução em blocos. |
| T-010 | Mapper e efeitos de escrita em memória | NOT_AUDITED | Identificar dependências concretas de timing do console. |
| T-011 | Trace temporal comparável | NOT_AUDITED | Definir cobertura proporcional a cada consumidor real; sem alegar trace elétrico completo. |

## 6. Snapshot, restore e determinismo

| ID | Item | Estado inicial | Próxima ação |
|---|---|---|---|
| D-001 | Codec versionado e compatibilidade de schema | AUDIT_PARTIAL | Conferir evolução, rejeição e migração. |
| D-002 | Snapshot em cada fase do sequenciador | AUDIT_PARTIAL | Matriz por fase e payload. |
| D-003 | Request pendente e identidade | AUDIT_PARTIAL | Preservar request ID, operação, endereço e dados. |
| D-004 | WAIT pendente e retomada | CLOSED | PR #680 integrada: testes focados e regressão completa passaram no HEAD `e20ec4b6c858bcaca265114ba543c19301dbb2e0`, run #42609912469; merge SHA `04527a59e0695a800d4fd048d2da85c51126de68`. WAIT e snapshot/restore das operações DD/FD cobertas pela PR foram validados no SHA integrado. |
| D-005 | Estados inválidos/inconsistentes | AUDIT_PARTIAL | Validar todos os campos e combinações por fase. |
| D-006 | Execução contínua vs. step/resume | AUDIT_PARTIAL | Comparar estado completo e sequência de transações. |
| D-007 | Execução em blocos de tamanhos diferentes | NOT_AUDITED | Testar equivalência determinística global aplicável. |
| D-008 | Save state de máquina SMS/SG-1000 | NOT_AUDITED | Distinguir snapshot CPU de estado integral de máquina/scheduler. |

## 7. Testes, referências e regressão

| ID | Item | Estado inicial | Próxima ação |
|---|---|---|---|
| V-001 | Inventário individual por opcode/variante | AUDIT_PARTIAL | Gerar matriz executável e detectar bytes sem teste semântico. |
| V-002 | Vetores externos fixados e versionados | AUDIT_PARTIAL | Registrar fonte, revisão, licença, hash e limites. |
| V-003 | Diferencial independente | NOT_AUDITED | Separar executor interno, corpus derivado de emulador e evidência de hardware. |
| V-004 | Exaustividade ALU/CB | AUDIT_PARTIAL | Conferir domínios e condições efetivamente varridos. |
| V-005 | Cobertura semântica positiva ED | AUDIT_PARTIAL | Expandir casos e distinguir aliases/fallbacks. |
| V-006 | Matriz de prefixos e indexed-CB | AUDIT_PARTIAL | Cobertura por combinação e resultado, não apenas classificação. |
| V-007 | Regressão Master System | AUDIT_PARTIAL | Gates e testes integrados no SHA candidato. |
| V-008 | Regressão SG-1000 | AUDIT_PARTIAL | Confirmar preservação da máquina legada compartilhando Z80. |
| V-009 | ZEXDOC/ZEXALL | BLOCKED | Não iniciar antes do fechamento E7; depois executar em harness Rust identificado. |
| V-010 | Testes em jogos | COMPLEMENTAR | Evidência de produto, não gate primário de opcode individual. |

## 8. Auditoria estrutural do código

| ID | Item | Estado inicial | Próxima ação |
|---|---|---|---|
| C-001 | Decoder BASE/CB/ED/DD/FD | AUDIT_PARTIAL | Mapear cada ramo, fallback e erro. |
| C-002 | Executor atômico vs. microsequenciador | AUDIT_PARTIAL | Comparar semântica e divergências de cobertura. |
| C-003 | Helpers ALU e flags | AUDIT_PARTIAL | Verificar fórmula, fonte e reutilização. |
| C-004 | Prefix FSM e classificação | AUDIT_PARTIAL | Cobrir cadeias, ignorados e último prefixo. |
| C-005 | Fases e transições do sequenciador | AUDIT_PARTIAL | Procurar estados inalcançáveis, transições ausentes e requests órfãos. |
| C-006 | Validadores e codec de snapshot | AUDIT_PARTIAL | Confrontar validador com todos os estados produzidos. |
| C-007 | Contadores temporais e overflow | NOT_AUDITED | Conferir aritmética checked e invariantes. |
| C-008 | Duplicidade de CPU por plataforma | AUDIT_PARTIAL | Confirmar uso do crate canônico e evitar divergência SMS/SG-1000. |
| C-009 | Testes tautológicos/hipóteses compartilhadas | AUDIT_PARTIAL | Identificar comparações que não são independentes. |
| C-010 | Warnings, lint e testes de workspace | CLOSED | PR #680 e PR #683 passaram nos HEADs exatos. #683 executou regressão ampla após alteração do workflow: workspace, D3D12/DXGI, WASAPI/XAudio2, SDL3, Linux audio, PowerShell, documentação e agregador PASS; merge SHA `43772d5cd5395de273bd3898895cf1deed81aa53`. A validação focada Z80 será comprovada em PR #684. |
| C-011 | Documentação/scorecard/matrizes | GAP_CONFIRMED | Baselines documentais estavam atrás da main; reconciliar por PR. |

## 9. Estado inicial e bloqueios confirmados

### 9.1 Baseline

- Snapshot da `main` imediatamente após a PR documental #681, usada como baseline desta varredura: `db6b4ec6d2f01ddaabccc124fe9ca7d95a856320`.
- Baseline de código usada na abertura do checklist: `b41fa66e01e329d45f7704a229479c8ac3c2c9ea`; a PR #681 foi documental e não alterou runtime.
- Núcleo canônico: `native/z80_cpu`.
- E4C, E5 e E6 aparecem como integradas nos documentos consultados.
- E7.7 continua em andamento; E8 continua bloqueada.
- Scorecard e matriz DD/FD preservam fotografias históricas anteriores; esta auditoria passa a usar `db6b4ec...` como baseline documental atual sem reescrever os resultados antigos.

### 9.2 PR #680 — bloqueio atual

- PR: https://github.com/leonardo201800478/sega-emulator/pull/680
- Branch: `feat/z80-e7-index-half-incdec-load-sequencer-20261109`.
- HEAD candidato atual: `268ece8cc4189bf70f5b1be92967a6d2de5e551f`.
- A falha inicial #5921 (`42572152572`) foi reproduzida: 241 testes passaram e `dd_fd_index_half_inc_dec_and_immediate_load_match_atomic_with_waits` falhou com `InvalidState`.
- Causa isolada: `ImmediateReadRegister` permitia DD/FD 26/2E no validador de fase, mas `indices_valid` aceitava somente prefixos ignorados.
- Correção no commit `48aad1f67b6eb4f03a87f3581249c8a51f6602bb`: aceitar precisamente 26/2E `UsesIndex` na validação de índices, sem relaxar outras invariantes. CI #5924 (`42575151294`) passou no HEAD desse commit: fmt, Clippy Z80/SMS/SG-1000 e testes focados; `sega-z80-cpu` reportou 242/242 testes.
- Commit `ed113041...` passou a acionar a regressão completa para mudanças Z80. CI #5925 (`42575339923`) passou em fmt, Clippy workspace, testes workspace, observability, Linux áudio, Windows áudio, SDL3, D3D12 e PowerShell, mas o merge foi bloqueado porque o job Z80-focused estava SKIPPED e dois checks obrigatórios eram esperados.
- Commit `268ece8...` removeu a exclusão do job Z80-focused quando `full=true`, para executar os dois gates simultaneamente. CI #5929 (`42575886280`) no HEAD atual: Z80-focused PASS (242 testes do crate e integração SMS/SG-1000), Rust workspace PASS (fmt, Clippy workspace, `cargo test --workspace --locked`, observability). Linux áudio, SDL3 e PowerShell PASS; D3D12 e WASAPI/XAudio2 ainda em execução nesta fotografia.
- A PR #680 cobre DD/FD 24,25,26,2C,2D,2E (INC/DEC e LD imediato IXH/IXL/IYH/IYL). Não integrar até CI #5929 concluir com sucesso no HEAD exato e os checks obrigatórios estarem presentes.

### 9.3 Prioridade imediata

1. Diagnosticar e corrigir PR #680 sem ampliar permissivamente o validador de snapshot.
2. Integrar #680 somente com CI verde no HEAD exato.
3. Após a integração, auditar e implementar a próxima família IXH/IXL/IYH/IYL: LD r,r' e operações ALU, preservando exceções H/L quando há (IX+d)/(IY+d).
4. Prosseguir em ordem de prioridade até todas as famílias e efeitos aplicáveis estarem fechados.
5. Somente então desbloquear E8.

### 9.4 Varredura estática DD/FD — lacunas seguintes confirmadas

A inspeção cruzada de `native/z80_cpu/src/prefix.rs`, `cpu.rs` e `sequencer.rs` identificou duas lacunas no caminho retomável:

1. **LD r,r' register-only com metades IX/IY:** o classificador reconhece os opcodes DD/FD 40–7F que usam H/L como `UsesIndex`. O executor atômico possui o caminho correspondente. O microsequenciador, porém, só aceita famílias 16-bit, displacement e os seis opcodes cobertos por #680; os demais `UsesIndex` são rejeitados explicitamente.
2. **ALU com IXH/IXL/IYH/IYL:** os 16 opcodes 80–BF cuja fonte é código 4 ou 5 são classificados como `UsesIndex` e executados pelo caminho atômico, mas ainda não possuem dispatch específico no microsequenciador.

**Exceções obrigatórias:** DD/FD 66, 6E, 74 e 75 com (IX+d)/(IY+d) usam H/L reais, não metades IXH/IXL/IYH/IYL. HALT 76 também não deve ser tratado como LD indexado. O caminho de memória indexada permanece separado.

**Modelo temporal esperado:** são operações register-only. Após os dois M1 (prefixo + opcode), podem atualizar estado e aposentar a instrução sem ciclos de barramento adicionais; o total nominal é 8 T-states com um prefixo, antes de WAIT. Não inventar ciclos internos para elas.

**Profundidade de evidência atômica localizada:**
- `native/sms_core/tests/z80_indexed_half_register_matrix.rs` percorre os opcodes 40–7F para DD e FD, excluindo 76/HALT; verifica formas register-only e memória indexada, incluindo as exceções H/L.
- `native/sms_core/tests/z80_indexed_half_register_alu.rs` cobre os oito operadores ALU com IXH/IXL e carry 0/1, mas usa 13 operandos selecionados; não é uma varredura combinatória de A×operand nem evidência independente de silício.
- Esses testes comprovam a rota atômica nos cenários exercitados. Eles não cobrem o dispatch retomável, WAIT/snapshot no microsequenciador ou todos os estados de entrada das flags.

**Próxima implementação após #680:** adicionar dispatch direto de LD r,r' e ALU half-register, com cobertura por DD/FD, destino/fonte, flags, Q, IX/IY não selecionado, H/L e PC/R, WAIT nos dois M1 e snapshot/restore nos safe points. Comparar com o executor atômico como diferencial interno e complementar com vetores externos independentes.

## 10. Histórico de preenchimento

| Data | SHA da main | Etapa | Resultado | Evidência / PR |
|---|---|---|---|---|
| 2026-11-09 | `b41fa66e01e329d45f7704a229479c8ac3c2c9ea` | Abertura do checklist e auditoria documental inicial | IN_PROGRESS | Scorecard, matriz DD/FD, auditoria ED e PR #680 examinados; detalhes na seção 9. |
| 2026-11-09 | `db6b4ec6d2f01ddaabccc124fe9ca7d95a856320` | Publicação do checklist mestre pela PR #681 e varredura estática inicial DD/FD | AUDIT_PARTIAL | Fotografia histórica; supersedida pelas integrações #680 e #683 registradas abaixo. |
| 2026-11-09 | `04527a59e0695a800d4fd048d2da85c51126de68` | PR #680 integrada: INC/DEC IXH/IXL/IYH/IYL e LD imediato; run #42609912469 PASS no HEAD `e20ec4b6c858bcaca265114ba543c19301dbb2e0` | CLOSED | D-004 fechado; E7 segue em auditoria global. |
| 2026-11-09 | `43772d5cd5395de273bd3898895cf1deed81aa53` | PR #683 integrou seleção inteligente de CI, jobs focados PSG/VDP/input e agregador; run #42613123290 PASS no HEAD `eaa88a249989b2ca6aee931e2a963df75a396008` | CLOSED | PR #637 encerrada como supersedida; CI otimizado integrado. |
| 2026-11-09 | `0b9b298f0f9791a6fa6db17580ad741db5c0515d` | PR #684 integrada: F-036 LD r,r' register-only no microsequenciador; CI #42614755935 PASS no HEAD `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3` | CLOSED | Merge squash confirmado; D3D12/áudio/SDL3 SKIPPED e merge aceito pelo GitHub. |
| 2026-11-09 | `e7e37290e1baa9119326e9e7213f77cadcc37f5e` | PR #686 integrada: F-037 ALU A,IXH/IXL/IYH/IYL no microsequenciador | CLOSED | HEAD validado `a0937e8fa1bb4547d1ee53a9e3431392a2fa443a`; CI #42619522899 PASS; merge squash confirmado. |

### Modelo obrigatório para cada atualização

Adicionar uma linha no histórico e atualizar os IDs afetados:

- Data e SHA da main.
- IDs fechados/reabertos.
- Divergência encontrada e referência técnica.
- Testes executados com classificação individual e SHA.
- PR, HEAD candidato, resultado CI e SHA de merge.
- Pendências remanescentes e próxima tarefa.


## 18. Reconciliação após PR #680/#683 e candidato F-036 — 2026-11-09

**Main remota confirmada:** `43772d5cd5395de273bd3898895cf1deed81aa53`.

- PR #680 foi integrada por squash. HEAD validado `e20ec4b6c858bcaca265114ba543c19301dbb2e0`; execução #42609912469 PASS; merge SHA `04527a59e0695a800d4fd048d2da85c51126de68`. INC/DEC e LD imediato das metades IX/IY estão integrados. As exceções DD/FD 66, 6E, 74 e 75 continuam no caminho de memória indexada e usam H/L reais.
- PR #683 foi integrada por squash. HEAD validado `eaa88a249989b2ca6aee931e2a963df75a396008`; execução #42613123290 PASS; merge SHA `43772d5cd5395de273bd3898895cf1deed81aa53`. O CI agora seleciona Z80, PSG, VDP, input, Scheduler, hosts e tooling por dependências; o agregador diferencia jobs aplicáveis de jobs SKIPPED. A PR #637 foi encerrada como supersedida.
- F-036 foi integrada pela PR #684. HEAD validado `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3`; execução #42614755935 PASS; merge SHA `0b9b298f0f9791a6fa6db17580ad741db5c0515d`.
- A PR #684 comprovou a seleção Z80-only: D3D12, áudio, SDL3, VDP, PSG, Scheduler stress e workspace foram SKIPPED; Z80 focused, documentação e agregador passaram. O merge foi aceito pelo GitHub apesar do job D3D12 ignorado, confirmando o comportamento do check condicional.
- F-037 permanece GAP_CONFIRMED e será implementada em PR separada após F-036.
- E7.7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED. Não iniciar E8.


## 19. Fechamento F-036 — 2026-11-09

- **PR:** #684 — `feat/z80-e7-indexed-register-ld-20261109`.
- **HEAD validado:** `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3`.
- **Execução final:** #42614755935 — PASS. Z80 focused PASS; Documentation patch hygiene PASS; Rust quality and unit regression PASS. Jobs não aplicáveis foram SKIPPED.
- **Falhas intermediárias:** #42614347670 FAIL apenas em rustfmt; #42614617507 FAIL em Clippy por `field_reassign_with_default` no teste. Ambas corrigidas; não foram promovidas a PASS. A execução final no HEAD exato passou.
- **Merge SHA / main atual:** `0b9b298f0f9791a6fa6db17580ad741db5c0515d`.
- **Prova do CI seletivo:** D3D12, WASAPI/XAudio2, Linux audio, SDL3, VDP, PSG, Scheduler stress e workspace ficaram SKIPPED em uma PR exclusivamente Z80. O merge squash foi aceito pelo GitHub; o check condicional D3D12 não bloqueou.
- **F-036:** CLOSED. **F-037:** GAP_CONFIRMED. **F-028:** AUDIT_PARTIAL. **E7.7:** IN_PROGRESS. **E8:** BLOCKED.

A próxima tarefa é F-037 — ALU A,IXH/IXL/IYH/IYL — em PR independente. Não iniciar ZEXDOC/ZEXALL.


## 20. Candidato F-037 — ALU com metades IX/IY — 2026-11-09

- **PR:** #686 — `feat/z80-e7-indexed-half-alu-20261109`.
- **Main de entrada:** `5aaa74af3fbf6fa7f70ecb1e6b71f3a966bb91e4`.
- **Commit de implementação:** `5f9ca825feb45633c2baf200e502f1a05bf99c7d`; último commit de teste/otimização antes desta atualização documental: `2ef8d4c0c73cf5a339f8d258ba9e314bfeebcdd2`. O HEAD final da PR será consultado no GitHub antes do merge.
- O dispatch cobre as 16 combinações DD/FD de ADD/ADC/SUB/SBC/AND/XOR/OR/CP com IXH/IXL/IYH/IYL, reutilizando a ALU compartilhada. O operando código 6 permanece no caminho de memória indexada E4B.
- A suíte `indexed_half_register_alu_e7.rs` contém 10.816 casos diferenciais, vetores fixos para flags completas e X/Y de CP, preservação de A/IX/IY/H/L, Q/previous-Q, PC/R/WZ, WAIT nos dois M1, snapshot/restore e prefixos repetidos.
- **Estado:** IMPLEMENTED_PENDING_VALIDATION. Nenhuma suíte da PR #686 é PASS até o resultado no HEAD final. F-036 permanece CLOSED; F-028 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

A próxima ação é validar o HEAD final da PR #686. Não iniciar E8.


## 21. Fechamento F-037 — 2026-11-09

- **PR:** #686 — `feat/z80-e7-indexed-half-alu-20261109`.
- **HEAD validado:** `a0937e8fa1bb4547d1ee53a9e3431392a2fa443a`.
- **CI final:** run #42619522899 PASS no HEAD exato. Change impact classification, Z80 focused validation, documentation hygiene e agregador PASS. D3D12, áudio, SDL3, VDP, PSG, Scheduler stress, PowerShell, input e workspace SKIPPED por não aplicabilidade.
- **Falha intermediária:** run #42615731297 FAIL somente por rustfmt no novo teste; corrigida antes do HEAD final. Não foi promovida a PASS.
- **Merge SHA / main atual:** `e7e37290e1baa9119326e9e7213f77cadcc37f5e`.
- **Cobertura:** 10.816 casos diferenciais; vetores fixos de A/F e CP X/Y; carry-in; Q/previous-Q; PC/R/WZ/IFF/I; IX/IY/H/L; WAIT nos dois M1; snapshot/restore após prefixo; DD/FD repetidos/alternados; 8T nominal para prefixo único e sem ciclos internos artificiais.
- **Limite de evidência:** comparação com executor atômico é diferencial interno. Vetores fixos têm expected A/F explícitos e independentes do resultado do caminho microsequenciado, mas a suíte ainda não importa o corpus externo SingleStepTests. A validação externa permanece pendente na auditoria mestre E7.

**Estado:** F-036 CLOSED; F-037 CLOSED; F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

**Próxima tarefa:** auditoria residual Z80 orientada por evidência externa e cobertura semântica/temporal. Não iniciar ZEXDOC/ZEXALL.


## 22. Reconciliação da baseline documental — PR #688

- Main de entrada: `4d8a862f254097e9dd1d0d1640edcd74f40ab12c`.
- Scorecard e matriz DD/FD tinham cabeçalhos com baselines antigas apesar de conterem os fechamentos #684/#686 em seções posteriores.
- Esta PR atualiza os cabeçalhos para a baseline corrente, mantendo as baselines anteriores como histórico e sem alegar reexecução de testes antigos.
- Estado semântico: F-036 CLOSED; F-037 CLOSED; F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## Atualização de evidências — PR #692 (2026-11-09)

- Baseline main: `1304bc0438e3ea333719d0ba3a495e2db9089da3`.
- F-037: 32.000 vetores SingleStepTests PASS no HEAD `1606556814a8746a445d3ad32fc948ab97f0fde5`, run `4262282817`.
- F-028: nova hipótese de divergência Q/previous-Q documentada; sem correção implementada antes de reproduzir.
- F-033: divergência de cobertura entre executor atômico e microsequenciador identificada; teste retomável pendente.
- F-034: classificação inicial ED iniciada; inventário global permanece parcial.
- E7 permanece IN_PROGRESS. E8/ZEXDOC/ZEXALL permanece BLOCKED.


## 24. Reconciliação após PR #702 — Q/previous-Q

- Main: `401bf4fb1ba8add6c84420cad5c9d2a62ceba72b`.
- PR #702 MERGED; HEAD validado `34e85637eaac8ed2777ecc5cb9cd2e91ba137235`.
- Runs #42626811588 e #42626765522: CI SUCCESS.
- F-028: DD/FD→SCF/CCF corrigido no executor atômico e no microsequenciador. 2.000 vetores SingleStepTests PASS; snapshot/restore entre prefixo e opcode terminal PASS.
- F-037: 32.000 vetores SingleStepTests PASS no mesmo HEAD. Total externo integrado: 34.000/34.000.
- F-028 permanece AUDIT_PARTIAL; F-033 GAP_CONFIRMED; F-034 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## 25. Reconciliação após PRs #706–#710 — 2026-11-09

**Main:** `1142eb567746afe64356b60f2ce466581e1c0cef`.

- F-033: CLOSED para o perfil de compatibilidade Zilog NMOS clássico, conforme PR #706 e CI #42635309051 no HEAD exato. Os 178 encodings reservados aposentam após os dois M1 já consumidos; não são instruções universalmente especificadas por UM0080.
- F-034: CLOSED somente para a auditoria ED no perfil de compatibilidade Zilog NMOS clássico SMS/SG-1000, conforme PR #709 e CI #6034 no HEAD exato; encodings não documentados BASE/CB/DD/FD permanecem cobertos pelas respectivas lacunas F-001..F-032 e não são fechados por F-034; PR #710 reconciliou a documentação e CI #6036 passou.
- F-028 permanece AUDIT_PARTIAL. A divergência Q/previous-Q DD/FD→SCF/CCF está corrigida; outras exceções DD/FD, prefixos e efeitos arquiteturais ainda precisam de auditoria.
- F-001..F-032 permanecem nos estados individuais registrados na tabela; não promover famílias com base apenas em igualdade entre executores internos.
- E7 permanece IN_PROGRESS; E8 permanece BLOCKED. A execução do ROM Z80 Block Flags Test v5.0 completo no core e validação direta em hardware SMS continuam NOT RUN; CMOS/clones continuam NOT VERIFIED.
- CI #6037 no HEAD atual executou classificação de impacto e higiene documental (PASS); jobs Z80 focused, workspace, Scheduler stress e hosts foram SKIPPED/NOT RUN. Não há nova execução semântica do Z80 no HEAD atual.


## 26. F-022 — observação de flags no ciclo de repetição interrompido (candidata PR #712)

**Baseline main:** `fa069162306fb330864b6d77633ceba0f976e3ca`.

A branch `test/z80-f022-repeat-im2-observation-20261109` acrescenta o teste `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` em `native/z80_cpu/src/sequencer.rs`. O cenário cobre LDIR, LDDR, CPIR e CPDR: INT é afirmada no último T-state do M-cycle interno de repetição; IM2 desvia para handler `PUSH AF; POP AF; EI; RETI`; o teste inspeciona F empilhado e confirma retomada do ED com BC/PC/WZ/flags coerentes. Snapshot encode/decode é aplicado em cada fronteira do microsequenciador.

A expectativa de X/Y no ponto interrompido é derivada do teste público Z80 Block Flags Test v5.0 (PC.13→YF, PC.11→XF), confrontada com MAME e ares. A comparação entre executor atômico e microsequenciador permanece evidência diferencial interna, não oracle independente. A alteração não fecha F-022: flags de iteração simples, fronteiras combinatórias, WZ independente e demais efeitos permanecem AUDIT_PARTIAL. Não há validação em hardware SMS nem generalização a CMOS/clones.


## 27. Integração da evidência F-022 — PR #712 — 2026-11-09

**Main:** `5e4a703daefdebe62fb7f8d220752359df59f8e3`. **PR:** #712 MERGED. **HEAD validado:** `7390744d9d6dc61ef2db3786f75fc9073dabdc12`. **CI:** #6045 (`42649308679`) PASS no HEAD da PR; #6046 (`42649608469`) PASS no HEAD pós-merge.

A fixture `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` valida LDIR/LDDR/CPIR/CPDR com INT no último T-state do ciclo interno de repetição, serviço IM2, captura de F por PUSH AF, retorno POP AF/EI/RETI e retomada da instrução. O microsequenciador passa por encode/decode + restore em cada fronteira. O resultado confirma a subquestão de X/Y observáveis em LDxR/CPxR interrompido para o perfil NMOS delimitado, sem fechar F-022. A comparação atômico↔microsequenciador continua sendo diferencial interna.

**Gates no HEAD exato da PR:** fmt PASS; Clippy PASS; testes Z80/SMS/SG-1000 PASS; vetores SingleStepTests DD/FD PASS; documentation patch hygiene PASS; agregador Rust quality/unit PASS. Workspace regression, Scheduler stress, VDP/PSG, áudio, input e hosts não relacionados ficaram NOT RUN conforme política de impacto.

F-022 permanece AUDIT_PARTIAL por flags das iterações simples, fronteiras combinatórias, WZ/MEMPTR independente e demais efeitos. Hardware SMS e ROM v5.0 integral no core permanecem NOT RUN; CMOS/clones NOT_VERIFIED. E7 IN_PROGRESS; E8 BLOCKED.


## 28. F-023 — RRD/RLD, cobertura exaustiva integrada PR #714 — 2026-11-09

A PR #714 MERGED em `3a81437362dad79577058852b5f469381d3bde2c` integra `ed_rrd_rld_exhaustive_nibble_transform_and_flag_contract` no executor atômico, com 262.144 vetores e oracle algébrico independente para os nibbles e flags S/Z/PV/H/N/C documentadas no UM0080. X/Y é verificado em trilha separada por concordância MAME/ares, não como especificação ou oracle físico. O endereço HL=FFFF é incluído para wrap de WZ.

No microsequenciador, 16 vetores dirigidos abrangem RRD/RLD, zeros, nibbles alternados, extremos, ambos os estados de Carry e HL=FFFF; cada transação recebe WAIT e cada ciclo interno/barramento passa por snapshot encode/decode + restore. A ordem observada é opcode fetch ED, opcode fetch terminal, leitura (HL), escrita (HL), com 18T nominais e 22T com os quatro WAITs inseridos.

A evidência de timing/flags documentadas é do UM0080. MAME e ares concordam em WZ=HL+1 e X/Y derivados de A, mas são referências de emulação, não validação de silício. F-023 permanece AUDIT_PARTIAL; ROM externo e hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED; E7 IN_PROGRESS; E8 BLOCKED.


## 29. Fechamento documental da PR #714 — 2026-11-09

PR #714 MERGED; HEAD de validação `bb15e672f30d8aae3777761f1545347fc3d520ea`; CI #6056 PASS no HEAD da PR e CI #6057 PASS no HEAD da main `3a81437362dad79577058852b5f469381d3bde2c`. A cobertura integrada é 262.144 vetores atômicos e 16 vetores microsequenciados com WAIT/snapshot. F-023 continua AUDIT_PARTIAL pelos limites de X/Y/WZ, ROM dedicado e hardware SMS. E7 IN_PROGRESS; E8 BLOCKED.


## 30. F-024 — fechamento no perfil NMOS clássico — 2026-11-09

F-024 CLOSED no perfil de compatibilidade Zilog NMOS clássico pela PR #716 (squash merge `56d75cc79803494e9fd8a54e8930871c9101fc84`). HEAD validado `9338f8dfee679fee6270c01f9670d6b678d219a0`; CI #6062 / run `42653692191` PASS; CI pós-merge #6063 / run `42653949243` PASS.

Cobertura: 4.096 vetores para ED 47/4F/57/5F (todos os valores de byte, IFF2 e Carry); refresh R inclui os dois M1; LD R,A copia os oito bits; flags normativas S/Z/PV/H/N/C; X/Y verificado como referência MAME/ares. Microsequenciador com WAIT e snapshot/restore em cada fronteira, 9T/11T. Erratum P/V=0 quando INT é aceito durante LD A,I/R, ambas as combinações IFF2/C e ambos os executores; NMI e instrução intermediária cobertos pelos testes existentes.

Limite: não generalizar a CMOS/clones; X/Y não é declarado como especificação UM0080 nem prova física. Hardware SMS NOT RUN. E7 IN_PROGRESS; E8 BLOCKED.


## 31. F-025 — fechamento RETN/RETI — 2026-11-09

PR #718 MERGED por squash em `f0cfc7b9df1834ae8ad418c6241702e9003f072e`; HEAD validado `24fd53a5eb7c36462fa6ebc332e1a15b1504cbce`; CI #6071 / run `42658617730` PASS no HEAD exato da PR. CI pós-merge #6072 / run `42658844499` PASS no HEAD exato da main `f0cfc7b9df1834ae8ad418c6241702e9003f072e`.

A cobertura exercita os oito aliases ED45/4D/55/5D/65/6D/75/7D, ambos os estados de IFF2, três pontos de SP (normal, FFFF e 0000), IFF1 inicialmente inversa, PC, SP com wrap, WZ=PC retornado e R após os dois M1. O microsequenciador recebe WAIT em cada requisição nos casos de wrap e passa por snapshot encode/decode + restore após cada transação; valida ordem de fetches e leituras low/high da pilha. 14T nominal; 18T com quatro WAITs. A expectativa WZ=PC foi confrontada com MAME e ares.

Os oito encodings de IM0/1/2 já têm testes independentes com seleção explícita do modo. RETI daisy-chain é NOT_APPLICABLE aos perfis SMS/SG-1000 atuais sem dispositivo consumidor. F-025 CLOSED somente para o perfil Zilog NMOS clássico adotado nos alvos SMS/SG-1000; sem validação física SMS; CMOS/R800/Z80N/clones NOT_VERIFIED. E7 IN_PROGRESS; E8 BLOCKED.


## 32. F-003 — validação focada aprovada, registro histórico pré-merge — 2026-11-09 (supersedido pela seção 33)

- **Main pré-PR:** `29462cf0a788d5905de7a5e9e0499a1f1361fec1`.
- **PR:** #724; **HEAD validado:** `ed20e79c2fbaa0cd35685f1dd1994da3d394a4e9`.
- **CI:** #6108 / run `42674749663` PASS no HEAD exato.
- **Gates:** change classification PASS; rustfmt PASS; Clippy focado PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD 34.000/34.000 PASS; documentation patch hygiene PASS; agregador PASS.
- **Cobertura F-003 executada:** 2.560 vetores semânticos; teste de wrap de PC; integração SMS temporal com WAIT, snapshot/restore e comparação de retomada; nenhum teste falhou.
- **Escopo não executado:** `cargo test --workspace --locked`, Scheduler stress e jobs de hosts/subsistemas não relacionados ficaram SKIPPED/NOT RUN pela política de impacto. Não classificá-los como PASS.
- **Auditoria do diff:** somente testes e documentação; nenhuma alteração de runtime Z80.
- **Estado à época deste registro:** aguardava squash merge. **Atualização posterior:** PR #724 integrada e F-003 CLOSED na seção 33; CI pós-merge permanece NOT VERIFIED.
- **Próxima família após F-003:** F-004, cujo escopo foi delimitado e cuja expansão de testes está registrada na seção 34. Não iniciar E8.


## 33. Fechamento F-003 — PR #724 integrada — 2026-11-09

- **Main pré-PR:** `29462cf0a788d5905de7a5e9e0499a1f1361fec1`.
- **PR:** #724 — squash merge `7fde212aabc07e8c4b7df56938f8845f99d29499`.
- **HEAD validado da PR:** `455ae4ea5ce68fdba53de1958fe0123afd39dc6d`.
- **CI no HEAD exato:** #6111 / run `42675073118` PASS. Change impact classification, rustfmt, Clippy focado, testes Z80/SMS/SG-1000, SingleStepTests DD/FD (34.000/34.000), documentation patch hygiene e agregador PASS.
- **Escopo não executado:** regressão integral `cargo test --workspace --locked`, Scheduler stress e jobs de hosts/subsistemas não relacionados ficaram SKIPPED/NOT RUN pela política de impacto.
- **Auditoria do diff:** somente testes e documentação; nenhuma alteração de runtime Z80.
- **CI pós-merge:** NOT VERIFIED; não foi retornado status/check run associado ao SHA de merge nesta consulta. Não promover a PASS.
- **F-003:** CLOSED somente para semântica de software do perfil de compatibilidade Zilog NMOS delimitado. Sem validação física SMS e sem compatibilidade universal CMOS/clones.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED.
- **Próxima candidata:** F-004 — LD rr,nn e transferências entre pares; auditar código, testes, escopo e dependências antes de implementar. Não iniciar E8.


## 34. F-004 — candidata validada, registro histórico pré-merge — 2026-11-09 (supersedido pela seção 35)

- **Baseline main pré-PR:** `355a35662643ca357ca9360031e002964e5a5aef`.
- **PR #726 HEAD validado:** `ee085aeb00290a470c2018c3399e3943fe96e006`.
- **CI #6119 / run `42676687551`: PASS** no HEAD exato.
- **Escopo:** BASE `01/11/21/31` (`LD BC/DE/HL/SP,nn`), BASE `22` (`LD (nn),HL`) e BASE `2A` (`LD HL,(nn)`).
- **Não duplicar:** DD/FD E3A1/E3A2; ED `43/53/63/73` e `4B/5B/6B/7B` já tratados na família ED/F-034; `LD SP,HL/IX/IY` e variações específicas do SP pertencem a F-005; EX/EXX pertence a F-006.
- **Referência documental:** ZiLOG UM008011-0816 p. 99 (2 M/10T para `LD dd,nn`), p. 102 (5 M/16T para `LD HL,(nn)`) e p. 107 (5 M/16T para `LD (nn),HL`); sem flags afetadas.
- **Evidência comparativa WZ:** MAME `z80.lst` e ares `instructions.cpp` indicam WZ preservado em `LD dd,nn` e WZ=`nn+1` nas transferências absolutas HL. UM0080 não especifica WZ.
- **Matriz:** 1.052 vetores imediatos + 2.560 vetores absolutos, seis casos de wrap de PC; WAITs diferenciados, snapshot encode/decode + restore em cada fronteira, diferencial atômico/micro e retomada SMS com trace.
- **Gates:** rustfmt PASS; Clippy `-D warnings` PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD 34.000/34.000 PASS; documentation hygiene PASS; agregador PASS. `cargo test --workspace --locked`, Scheduler stress e jobs de hosts/VDP/PSG não relacionados ficaram SKIPPED/NOT RUN.
- **Histórico de correção:** #6115 rustfmt FAIL (formatação corrigida); #6117 Clippy FAIL (tipo do vetor temporal explicitado); #6118 Clippy FAIL (`too_many_arguments` tratado localmente); #6119 PASS integral dos gates aplicáveis.
- **Runtime:** nenhuma alteração. Nenhuma falha funcional exigiu modificação do Z80.
- **Estado à época do registro:** gates focados aprovados; integração pendente. **Atualização posterior:** PR #726 integrada por squash e F-004 CLOSED na seção 35. CI pós-merge permanece NOT VERIFIED.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED.


## 35. Fechamento F-004 — PR #726 integrada — 2026-11-09

- **Main pré-PR:** `355a35662643ca357ca9360031e002964e5a5aef`.
- **PR:** #726 — squash merge `6ce8399aa96937547e72c24e75f1521dc5a9d550`.
- **HEAD validado:** `d4e0c6bcf7b339fee8674102fe26e5b2dd4d131a`.
- **CI pré-merge:** #6122 / run `42676982272` PASS. Change classification, rustfmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD 34.000/34.000, docs hygiene e agregador PASS.
- **Escopo NOT RUN/SKIPPED:** regressão integral `cargo test --workspace --locked`, Scheduler stress e jobs de hosts/VDP/PSG não relacionados.
- **Auditoria do diff:** somente testes e documentação; nenhum runtime Z80 alterado.
- **CI pós-merge:** NOT VERIFIED; status/check run associado ao SHA de merge não retornado nesta consulta.
- **F-004:** CLOSED somente para semântica de software do perfil de compatibilidade Zilog NMOS delimitado. Sem validação física SMS ou compatibilidade universal CMOS/clones.
- **Próxima auditoria:** reavaliar dependências globais E7, especialmente F-023 (evidência independente X/Y/WZ), F-026 (HALT/NOP/refresh/interrupções) e F-027–F-032 (DD/FD/prefixos), antes de fixar a próxima implementação. F-005 continua na sequência de famílias.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED.


## 36. Triagem global de famílias E7 após F-004 — 2026-11-09

| Família | Evidência atual | Lacuna residual / prioridade |
|---|---|---|
| F-005 — LD SP,HL/IX/IY e SP absoluto | Cobertura parcial em `indexed_16bit_e3a1.rs`; transferências ED SP absolutas relacionadas estão na F-034. | Confirmar tempos, estado SP e fronteiras de prefixo; não duplicar F-034. Continua na sequência de pares de 16 bits. |
| F-015 — RLCA/RRCA/RLA/RRA | NOT_AUDITED; ALU tinha somente dois vetores direcionados e integração temporal curta. | **Selecionada como próxima implementação** por ser uma lacuna isolada e sem matriz semântica completa. PR de testes em andamento. |
| F-022 — LDI/LDD/LDIR/LDDR e CPI/CPD/CPIR/CPDR | PR #712 integrada; fixture IM2 cobre LDIR/LDDR/CPIR/CPDR, flags via PUSH AF, RETI, retomada e snapshot/restore. | Ampliar matriz independente, flags dos caminhos simples, WZ/MEMPTR e fronteiras restantes. |
| F-023 — RRD/RLD | PR #714 integrada; 262.144 vetores atômicos e 16 microsequenciados com WAIT/snapshot. | Evidência independente X/Y e WZ=`HL+1` permanece comparativa MAME/ares; sem prova física. É uma lacuna transversal antes do fechamento global E7. |
| F-026 — HALT/NOP/estado após HALT | Há auditorias separadas de HALT M1, refresh, interrupção e snapshots. | Reconciliar NOP, M1 repetido, R, saída por INT/NMI e restore sob Scheduler; não considerar os testes isolados como fechamento integrado. |
| F-027 — DD/FD substituição HL→IX/IY | Matriz parcial e prefix resolver E6. | Auditoria por opcode e exceções de substituição/ignorados. |
| F-028 — IXH/IXL/IYH/IYL | 34.000 SingleStepTests DD/FD PASS; PR #702 integrada. | Restam exceções, prefixos, HALT e estado arquitetural fora do corpus validado. |
| F-029 — (IX+d)/(IY+d) | Matriz parcial. | Displacement assinado, wrap, exceções H/L e ordem de acesso. |
| F-030 — DDCB/FDCB | Matriz parcial. | Fechar subopcode × prefixo × destino, incluindo BIT e destinos de registrador. |
| F-031 — prefixos repetidos/alternados | Matriz parcial. | Consumo, last-prefix-wins, PC/R, M1 e semântica terminal. |
| F-032 — prefixos ignorados | Matriz parcial. | Sem refetch, sem substituição indevida e custo temporal do prefixo. |

**Decisão de prioridade (pós-F-015):** F-015 foi fechada na seção 38. A próxima candidata é F-005, por continuidade da cadeia de transferências entre pares após F-004 e por já possuir evidência parcial reutilizável. Antes de implementar, auditar o diff/runtime e consolidar critérios de LD SP,HL/IX/IY e transferências absolutas de SP sem duplicar ED F-034. F-023 continua gate transversal de evidência independente X/Y/WZ; F-022 exige completar flags/WZ dos caminhos simples e F-026 exige reconciliação integrada HALT/NOP/refresh/interrupções. F-027–F-032 permanecem em auditoria contínua; não avançar E8.

### PRs abertas preservadas — sem integração automática

| PR | Estado | Escopo |
|---|---|---|
| #645 | OPEN | Scheduler/SRAM |
| #644 | OPEN | Diretórios configuráveis da interface |
| #643 | OPEN | Mapper SRAM |
| #639 | OPEN | Otimização Scheduler |
| #619 | OPEN | Documentação SMS |
| #533 | OPEN | Contrato de conclusão |

Todas permanecem abertas e fora do escopo da F-015; qualquer integração exige auditoria independente de dependências, conflitos, CI e supersessão.


## 37. F-015 — RLCA/RRCA/RLA/RRA — gates focados aprovados — 2026-11-09

- **Baseline main pré-PR:** `f726755c87fcfb61bdf9ac5e2af031afe7712e21`.
- **PR #728 HEAD validado:** `3a48894d585e6ff6599b6487b1fb7d0c1607fa0d`.
- **CI #6129 / run `42678520765`: PASS** no HEAD exato.
- **Matriz semântica:** 262.144 vetores (4 opcodes × 256 A × 256 F); verifica resultado A/F, S/Z/PV, H/N, C, X/Y comparativo, Q=F, previous-Q, WZ preservado, PC e R/R7.
- **Snapshot/restore:** 1.024 casos comparando execução contínua e retomada; quatro casos PC FFFF→0000.
- **Integração SMS:** 192 casos; WAIT no M1, snapshot com M1 pendente, restore, trace e 5T/15 master ticks.
- **Gates:** rustfmt PASS; Clippy `-D warnings` PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD 34.000/34.000 PASS; documentation patch hygiene PASS; agregador PASS.
- **Histórico de correção:** #6126 rustfmt FAIL (formatação corrigida); #6128 Clippy FAIL (`too_many_arguments` localizado e justificado); #6129 PASS nos gates aplicáveis.
- **Escopo NOT RUN/SKIPPED:** `cargo test --workspace --locked`, Scheduler stress e jobs de hosts/VDP/PSG não relacionados. Não são PASS.
- **Runtime:** nenhuma alteração.
- **Estado:** IMPLEMENTED_PENDING_VALIDATION até squash merge e confirmação da main. X/Y permanece comparação MAME/ares, sem prova física. E7 IN_PROGRESS; E8 BLOCKED.


## 38. Fechamento F-015 — PR #728 integrada — 2026-11-09

- **Main pré-PR:** `f726755c87fcfb61bdf9ac5e2af031afe7712e21`.
- **PR:** #728 — squash merge `ce51578511a0090afbba3ded6312c9a25c5d78c7`.
- **HEAD validado da PR:** `c198cff1629270378a3bdd22a1126ca3d98a44b8`.
- **CI final pré-merge:** #6131 / run `42678804879` PASS no HEAD exato. #6129 validou o HEAD de código intermediário; #6131 também validou a reconciliação documental final.
- **Gates:** change impact classification, rustfmt, Clippy `-D warnings`, testes Z80/SMS/SG-1000, SingleStepTests DD/FD 34.000/34.000, documentation patch hygiene e agregador PASS.
- **NOT RUN/SKIPPED:** `cargo test --workspace --locked`, Scheduler deterministic stress e jobs de VDP/PSG/hosts/input não relacionados. Não classificar como PASS.
- **Matriz:** 262.144 vetores semânticos A×F; 1.024 snapshot/restore; quatro PC-wrap; 192 casos SMS com WAIT no M1, snapshot/restore e trace.
- **Auditoria do diff:** somente testes e documentação; runtime Z80/ALU/sequenciador não alterado.
- **CI pós-merge:** NOT VERIFIED nesta consulta para o SHA `ce51578511a0090afbba3ded6312c9a25c5d78c7`.
- **F-015:** CLOSED somente para semântica de software no perfil Zilog NMOS delimitado. X/Y é comparação MAME/ares, não evidência física; sem compatibilidade universal CMOS/clones.
- **Próxima candidata:** F-005 — LD SP,HL/IX/IY e transferências absolutas de SP, após auditoria de dependências e separação dos opcodes já cobertos em ED F-034.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED.


## 39. F-005 — LD SP,HL/IX/IY — PR #731 integrada — 2026-11-09

- **Main pré-PR:** `43a721b2a293831601ac02dc68b853b11beb363b`.
- **PR:** #731, squash merge `0ea60a450c3d58a6231df73c3d1ff98fa118b4b6`.
- **HEAD validado:** `e9ab17ebbea042a5396a5cd017c70982c6d2f2cc`.
- **CI:** run #42696796388 PASS nos gates aplicáveis: Z80 focused, Rust workspace regression, Linux audio-output, Windows WASAPI/XAudio2, Windows egui e agregador.
- **NOT RUN:** D3D12/DXGI runtime e suítes não relacionadas selecionadas como SKIPPED pela classificação de impacto; gates locais Cargo não executados neste ambiente.
- **Cobertura:** BASE F9 = 6T/1 M-cycle; DD/FD F9 = 10T/2 M-cycles. Inclui estado arquitetural, flags, R/refresh, Q/previous-Q, WZ, prefixos repetidos/alternados, WAIT, snapshot/restore e comparação retomada/contínua.
- **Integração:** BASE F9 exercitada em SmsTemporalMachine/MachineScheduler com WAIT M1 e snapshot/restore. ED 73/7B reutiliza F-034.
- **Auditoria do diff:** somente testes/documentação; nenhuma alteração de runtime Z80/SMS.
- **F-005:** CLOSED para semântica de software do perfil Zilog NMOS delimitado. Não equivale a validação física SMS nem a compatibilidade universal CMOS/clones.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED.

### Próxima frente — auditoria residual E7.7

F-005 encerra esta família de transferências. Não iniciar E8/ZEXDOC/ZEXALL: o gate global E7 ainda não está fechado. A próxima tarefa deve ser uma triagem de evidência independente para as lacunas residuais já catalogadas, priorizando:
1. F-026: reconciliação integrada de NOP, M1 repetido, R/refresh, saída de HALT por INT/NMI e snapshot/restore sob Scheduler.
2. F-022: auditar o que resta após PR #736 — cobertura do caminho microsequenciado, proveniência independente dos efeitos não documentados e integração das fronteiras de interrupção/retomada; não repetir a matriz atômica já integrada.
3. F-023: evidência independente para X/Y e WZ de RRD/RLD, sem promover comparação MAME/ares a prova física.

A escolha da próxima implementação depende de auditoria do código/testes atuais e de referência independente; esta lista não autoriza duplicar cobertura já integrada nem introduzir microsequenciamento sem consumidor observável.


## 40. Reconciliação após PRs #736 e #843 — 2026-11-14

**Main auditada:** `2fe93dd4c1ed0af7725b92805e5d1b29c126217e`.

- **F-022 / PR #736:** merge SHA `55585bf2425f1c4eaf61c4a3a3ad42f9499c4797`; HEAD validado `1520cc77e2801d985401a5518eec9da712b89e3`; CI #6166 / run `42698792012` SUCCESS. A matriz atômica de LDI/LDD/CPI/CPD executa 524.288 casos, cobrindo flags documentadas, X/Y conforme fonte secundária e WZ/MEMPTR. Os cenários microsequenciados existentes verificam WZ, WAIT e snapshot/restore. PR test-only, sem alteração de runtime. F-022 continua AUDIT_PARTIAL.
- **Integração temporal / PR #843:** merge SHA `23c77a21ae427feecfb70fa6e78692e588f72416`; HEAD validado `0f2415f9382887c94d9214abe06cb6c295ed2154`; CI #7389 / run `43568659312` e SMS Debugger Runtime #857 / run `43568659270` SUCCESS. VDP core/machine, Z80 focused, workspace regression, input integration e Scheduler deterministic stress passaram. Testes cobrem VINT T13/T16 com snapshot/restore e chunk invariance, HINT R10 no limite T14 e chegada da linha INT ao Z80. É evidência de integração temporal, não fechamento global do Z80.
- **Estado global:** E7 IN_PROGRESS; E8 BLOCKED. Nenhum resultado foi transferido de um SHA para outro. Os 34.000 vetores DD/FD continuam vinculados ao HEAD validado da PR #692, sem nova execução atribuída à main atual.


## 41. Reconciliação de evidências E7 e estado dos gates — 2026-11-16

**Main documental:** `5f24e22f2c49512ea43fa24ea25f836464d70f61`.
**Baseline de código Z80:** `bf7e7584df421000d040a3ee057799a75bfeb3eb`. A PR #939 foi documental; a comparação das SHAs mostra somente alterações no scorecard e na matriz DD/FD.

Esta seção prevalece sobre afirmações históricas anteriores que, na data em que foram escritas, indicavam ausência de execução posterior à PR #692 ou ausência de tentativa integral do ROM v5.0.

### Evidências corrigidas

1. **F-022 / microsequenciador:** a PR #930 integrou a matriz microsequenciada de 786.432 execuções (4 opcodes × 256 A × 256 memória × BC inicial 0/1/2), com WAIT e snapshot/restore nas fronteiras cobertas. O CI #43815302707 validou o HEAD exato da PR. Essa matriz não equivale à execução integral do ROM v5.0.
2. **F-022 / ROM v5.0:** a tentativa inicial excedeu 50.000.000 T-states durante LDIR e terminou FAIL/timeout. A tentativa instrumentada posterior no HEAD `adaa349366ccda210faacecb0ce9a93031bc0c78`, CI #43839665389, chegou ao B-loop e falhou B=13 vs 15. O trace observou OTIR em PC=800E iniciado 54T antes da borda de INT, com intervalos de 21T e aceite após três decrementos de B. Isso é evidência de desalinhamento, não causa-raiz concluída.
3. **Instrumentação LDIR:** o HEAD `e0c4cadd443c5fd61717cfda07c386a4bcc4be0d` falhou Clippy (`collapsible_if`) antes da execução da ROM. A correção Clippy em `4ca567c13f1ca95d84a8ff3a8143775a53a3f6c7` permitiu executar o ROM, mas o trace inicial encerrou na primeira entrada IM2, com BC=`33CD`. A instrumentação `bdc7eb73f1a03e5d5a59e0cfce99e2fe6a404e1c` registrou `PC=8051`, `BC=3FFF→0000`, `elapsed=65871T`, mas o trace confundiu a conclusão do LDIR curto dentro do handler IM2 (PC=8595, BC=0) com a conclusão do LDIR principal interrompido. Essa medição é inválida para duração total e não prova undercount do core. O HEAD `a0f50a28c4615c779cca0407126115b56d3b3931` acrescentou amostras por instrução; o HEAD `5081a293f1cc664f4fef0bb330a904990995e4d7` agora exige observar RETI retornar ao PC do LDIR antes de aceitar a conclusão com BC=0. CI pendente.
4. **F-023 / MEMPTR:** os 16.384 cenários atômicos e 240 microsequenciados são regressões internas derivadas de documentação. O TAP de Mark Woodmass não foi recuperado/executado; MAME/ares não são oráculo independente de hardware. F-023 permanece AUDIT_PARTIAL.
5. **F-028 / SingleStepTests:** os 34.000 vetores foram executados explicitamente com o corpus pinado em três HEADs posteriores à PR #692: #930 (`c96d39f2...`, run #43815302707), #934 (`75f897c1...`, run #43818081949) e #935 (`6e665fe0...`, run #43819970660). Os três jobs Z80 focused passaram. O CI da main `5f24e22...` (#43831038791) marcou Z80 focused SKIPPED; portanto, para a main atual, NOT RUN.
6. **Checklist residual:** inventário funcional, efeitos observáveis, Q durante/depois de acknowledge, interrupções, WAIT/barramento, determinismo, HALT e interações DD/FD devem continuar com estado próprio e evidência rastreável. Não converter comparações atômico↔micro ou MAME↔ares em prova independente.
7. **Escopo:** o fechamento de software deve explicitar o perfil Zilog NMOS clássico SMS/SG-1000. Validação física em silício, CMOS e clones podem permanecer NOT_VERIFIED com justificativa, sem se tornarem bloqueios universais não previstos no escopo.

**Classificação atual:** F-022 AUDIT_PARTIAL; F-023 AUDIT_PARTIAL; F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED. Nenhum resultado de workflow foi transferido para SHA diferente daquela em que executou.

## 47. Fechamento da rodada de testes focados F-022/F-023/F-028 — 2026-11-16

Relatório consolidado: [docs/audits/z80-e7-current-status-2026-11-16.md](audits/z80-e7-current-status-2026-11-16.md).

- F-022 / PR #936: HEAD 306db2a91ff796b4c5f66eabc4ebd4c4baad903e; CI #43852582196 e SMS Runtime #43852582229 SUCCESS. ROM v5.0 PASS (1/1, 6,50 s), ≥60 comparações principais, 390 B-loop records em seis grupos, B=0x0D+3*i, flags exatas; 34.000 SingleStepTests DD/FD PASS. F-022 permanece AUDIT_PARTIAL por escopo não exaustivo.
- F-023 / PR #942: código validado 7402ff138ced95da2d8b9e0008919c8cd22484f1; CI #43851550529 e SMS Runtime #43851550520 SUCCESS. Woodmass MEMPTR test 09 PASS: HALT 0x80E6, 50.904.931 T-states, FNV-1 F5AE5140, 61 linhas, 32 colunas. WZ/X/Y em outras famílias e timing restante pendentes; F-023 AUDIT_PARTIAL.
- F-028 / PR #943: HEAD de código validado 98c1cf41587f6af66c3d272e4b07a638d5da7919; CI #43859000666 e SMS Runtime #43859000676 SUCCESS. Matriz com 108 combinações prefixadas por executor, nove cadeias DD/FD × 12 formas. Quatro testes PASS; WAIT e snapshot/restore por M1. Fixture SCF/CCF corrigido para controlar Q inicial; CPU/runtime não alterados. A matriz não cobre todos os 256 opcodes DD/FD; F-028 AUDIT_PARTIAL.
- As três PRs permanecem Draft e sem merge. Os testes foram executados nos SHAs de PR indicados, não na main atual. SKIPPED permanece NOT RUN.
- E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED. Nenhuma alteração de CPU, decoder, microsequenciador, MachineScheduler ou runtime nesta rodada.

## 48. Reconciliação integral dos critérios E7 — baseline main 2026-11-16

Main confirmada: caa9a76a6f09d588155ea4e0cbc95f487f079143.
Branch de continuidade: test/z80-f028-exception-matrix-20261116.
Estado global: E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.

Esta seção reconcilia as evidências de PR sem transferi-las para a main e identifica o estado dos critérios transversais que ainda impedem o fechamento formal.

### 48.1 Famílias funcionais críticas

| Critério | Estado | Evidência confirmada | Lacuna para fechamento |
|---|---|---|---|
| F-022 | AUDIT_PARTIAL | Matriz microsequenciada LDI/LDD/CPI/CPD (#930); interrupções IM2 (#932); ROM v5.0 no SHA 306db2a... (#936) PASS no executor atômico; não valida microsequenciador/WAIT/Scheduler. | A ROM cobre somente LDIR/LDDR/CPIR/CPDR interrompidos (16 comparações); LDI/LDD/CPI/CPD simples já têm matriz microsequenciada #930 (786.432 execuções). Restam ampliar os estados/condições de repetição, Q/flags em fronteiras de INT/NMI e retomada temporal, sem confundir o harness atômico ROM com microsequenciador/Scheduler. |
| F-023 | AUDIT_PARTIAL | Testes internos RRD/RLD→BIT; Woodmass MEMPTR 09 no SHA 7402ff1... PASS no executor atômico, com T-states nominais. | Matriz WZ/MEMPTR por família, condições tomadas/não tomadas, wrap, prefixos e X/Y sem evidência independente suficiente. |
| F-028 | AUDIT_PARTIAL | 34.000 vetores Ares-derived em HEADs de PR; matriz dirigida #943 com 108 casos por executor; ledger explícito dos 256 bytes e testes de INT/NMI durante DD/FD PASS no HEAD b3320ea; CI #43863265215 e SMS Runtime #43863265197 SUCCESS. | Semântica integral por opcode/estado, matriz de todas as combinações de interrupções/WAIT e oracle independente ainda incompletos. |
| F-026 | AUDIT_PARTIAL | Testes HALT/NMI/INT T4 e integração temporal SMS existentes. | Matriz combinada de HALT, prefixos, refresh e interrupções em todos os pontos observáveis ainda incompleta. |

### 48.2 Estado dos critérios arquiteturais e transversais

| Critério | Estado atual | Reconciliação |
|---|---|---|
| S-001/S-002 — bancos e IX/IY | AUDIT_PARTIAL | Exceções H/L e bancos alternativos têm testes direcionados; não há cobertura semântica completa por opcode/estado. |
| S-003 — SP/PC/I/R | AUDIT_PARTIAL | PC wrap, R por M1 e prefixos repetidos cobertos em cenários selecionados; interrupções e wrap global ainda precisam de reconciliação. |
| S-004 — IFF/IM/EI delay | AUDIT_PARTIAL | IM0/1/2, RETN/RETI, EI/DI e interrupções HALT têm evidência específica; interações combinadas com prefixos permanecem incompletas. |
| S-005 — WZ/MEMPTR | AUDIT_PARTIAL | Woodmass MEMPTR 09 adiciona evidência externa para um caso; falta matriz completa por família e condições. |
| S-006 — Q/previous-Q | AUDIT_PARTIAL | DD/FD→SCF/CCF resolvido; matriz dirigida #943 cobre 12 formas; Q durante/após acknowledge e demais famílias ainda requer reconciliação. |
| T-001/T-004/T-005 — M1, INT e NMI | AUDIT_PARTIAL | HALT T4 e resposta NMI/INT existentes; testes que solicitam NMI/INT entre DD e FD passam no microsequenciador no HEAD b3320ea. Não é integração adicional do Scheduler SMS. |
| T-006 — HALT | AUDIT_PARTIAL | HALT simples e saída por INT/NMI testados; combinações completas de prefixos e bordas ainda não fechadas. |
| T-007/T-008 — WAIT e T-states | AUDIT_PARTIAL | WAIT por M1 e transações selecionadas com snapshot; não declarar trace temporal global nem precisão elétrica. |
| T-009 — Scheduler | AUDIT_PARTIAL | Integração SMS/VDP e testes de interrupção existem; equivalência global contínua/em blocos de todos os perfis ainda não provada. |
| D-001/D-002/D-003 — snapshot/restore | AUDIT_PARTIAL | Codec e snapshots por safe point estão cobertos em famílias selecionadas; novos casos NMI/INT durante cadeia DD/FD e WAIT round-trip passam no HEAD b3320ea. Não fecha todos os estados/fases. |
| D-006/D-007 — retomada e chunk invariance | AUDIT_PARTIAL | Há equivalência local em testes dirigidos; não há prova global para todos os tamanhos de bloco e estados. |
| V-001 — inventário por opcode | AUDIT_PARTIAL | Ledger CSV de 256 bytes e teste explícito de classe (85 UsesIndex, 170 Ignored no helper, 1 IndexedCB) PASS no HEAD b3320ea. Classificação não substitui semântica por opcode. |
| V-002 — vetores externos | AUDIT_PARTIAL | ROM v5.0 MIT e Woodmass MEMPTR 09 com hashes/proveniência pinados; limites de domínio e licenças mantidos. |
| V-003 — diferencial independente | AUDIT_PARTIAL | Woodmass e ROM v5.0 são evidência hardware-derived em ambiente Spectrum; SingleStepTests é Ares-derived; atomic↔micro é consistência interna. |
| V-006 — prefixos/indexed-CB | AUDIT_PARTIAL | E5/E6 preservados; DDCB/FDCB tem testes de classificação dos 256 subopcodes e semântica exaustiva de rotate/shift; ledger DD/FD e classificação explícita PASS no HEAD b3320ea. Semântica de todas as famílias/estados ainda incompleta. |
| V-007/V-008 — regressões SMS/SG-1000 | AUDIT_PARTIAL | SMS Runtime PASS nas PRs citadas; regressões completas devem ser repetidas no SHA final integrado. |
| V-009 — ZEXDOC/ZEXALL | BLOCKED | Não iniciar antes do fechamento formal de E7. |
| C-001/C-004/C-005 — decoder/FSM/sequenciador | AUDIT_PARTIAL | Inspeção de prefix.rs, prefix_fsm.rs e prefix_fallback_e6.rs; nenhuma alteração de produção foi feita. |
| C-009 — testes tautológicos | AUDIT_PARTIAL | Comparações atomic↔micro explicitamente classificadas como internas; revisar cada novo oracle antes de promover status. |
| C-011 — documentação | AUDIT_PARTIAL | Baseline do checklist corrigida nesta branch; a main permanece na baseline caa9 até eventual integração. |

### 48.3 Novos artefatos candidatos e proveniência

- Ledger completo: docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv, 256 linhas de dados; contagens 85 UsesIndex, 167 Ignored, 1 IndexedCB, 1 ED-cancel e 2 repeat-prefix.
- Teste de classificação: native/z80_cpu/tests/index_prefix_classification_f028.rs; enumera todos os bytes e compara com uma lista explícita de 85 UsesIndex.
- Testes de interrupção: native/z80_cpu/tests/z80_dd_fd_exception_matrix_f028.rs; solicita NMI ou INT entre DD e FD, inclui WAIT e round-trip de snapshot, e exige atendimento somente após o opcode terminal.
- Código/harness validado: b3320ea79c6b951d2ad8b70e762589f14da597df. CI #43863265215 Z80 focused SUCCESS (classificação 3/3; F-028 6/6; 34.000 SingleStepTests PASS); SMS Runtime #43863265197 SUCCESS (Release format, Clippy, testes e build Windows x64).
- Os novos testes foram executados e passaram no HEAD b3320ea. O PASS anterior #43859000666/#43859000676 continua vinculado exclusivamente ao código 98c1cf4; a nova evidência é #43863265215/#43863265197.

### 48.4 Decisão de auditoria

- F-022, F-023 e F-028 permanecem AUDIT_PARTIAL.
- F-005 é CLOSED para o perfil NMOS delimitado, conforme PR #731 e seção 39; o estado inicial antigo da tabela foi corrigido.
- As novas tabelas e testes aumentam a cobertura de classificação e de fronteiras, mas ainda não fecham semântica, independência de oráculo ou timing global.
- Nenhuma alteração de CPU, decoder, microsequenciador de produção, MachineScheduler ou runtime foi feita.
- E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED.
### 48.5 Reconciliação do inventário funcional F-001–F-037

Fotografia de status na baseline main caa9a76a6f09d588155ea4e0cbc95f487f079143, com evidências adicionais ainda não integradas das PRs abertas:

- CLOSED (perfil NMOS delimitado): F-001, F-002, F-003, F-004, F-005, F-015, F-024, F-025, F-033, F-034, F-036 e F-037. Esses fechamentos são preservados; não reabrir sem regressão reproduzível.
- AUDIT_PARTIAL: F-006 a F-014; F-016 a F-023; F-026 a F-032; F-035. As evidências das PRs #936/#942/#943 atualizam os itens detalhados acima, mas não os promovem a CLOSED.
- F-021 e F-022 são famílias distintas: os 390 B-loop records da ROM v5.0 correspondem a OTIR/OTDR/INIR/INDR (F-021). A stream principal tem ≥60 comparações de instrução/PC, mas o mapeamento por opcode para as oito formas LDI/LDD/CPI/CPD de F-022 ainda precisa ser reconciliado. Não atribuir os 390 registros à cobertura F-022.
- O inventário de status não implica que toda a evidência de PR esteja na main: F-021/F-022, F-023 e F-028 permanecem associados às respectivas SHAs de PR; a main caa9 ainda não recebeu essas alterações.
- E7 continua IN_PROGRESS; E8/ZEXDOC/ZEXALL continua BLOCKED.

## 49. Reconciliação da main após integração das PRs #936/#942/#943/#944 — 2026-11-16

**Main:** `fbd70c4d668902d1869d7cfbdfde5c880c9b3857`. As quatro PRs estão MERGED: #944 `a3cc1edc0f94ca5da1850996d3bf164b0839a2e9`, #936 `3b85be47d1f07b7eaa25ea65299a0608e68761d4`, #942 `1a6f032861cebf3bf23867edec81ca08db97b064`, #943 `fbd70c4d668902d1869d7cfbdfde5c880c9b3857`.

A #943 foi validada por CI #43880197594 e SMS Runtime #43880197552 (SUCCESS). O tree SHA do merge commit e do HEAD testado é o mesmo: `9b13446b005b908d80607b8a258d87008db82433`. SKIPPED continua NOT RUN.

| Requisito | Estado pós-merge | Pendência que impede CLOSED |
|---|---|---|
| F-022 | AUDIT_PARTIAL | Mapear explicitamente as comparações ROM para LDI/LDD/CPI/CPD; completar a matriz simples/microsequenciada, WAIT, interrupções e flags sem atribuir B-loop de F-021 a F-022. |
| F-023 | AUDIT_PARTIAL | Expandir WZ/MEMPTR e X/Y por família/condição; acrescentar referência independente e validação temporal para os casos ainda não cobertos. |
| F-028 | AUDIT_PARTIAL | Demonstrar semântica por opcode/família para UsesIndex e fallback dos Ignored, incluindo custo M1, refetch, WAIT e fronteiras de interrupção/snapshot. |
| T-009 / D-006 / D-007 | AUDIT_PARTIAL | Ainda não há prova global de equivalência contínuo/em blocos para todos os estados e tamanhos de bloco. |
| V-007 / V-008 | AUDIT_PARTIAL | Executar regressões finais SMS/SG-1000 no SHA integrado e manter os resultados vinculados ao SHA efetivamente testado. |
| V-009 | BLOCKED | ZEXDOC/ZEXALL somente após fechamento formal de E7. |

Nenhum dos merges altera a classificação técnica parcial. A sequência correta é fechar as lacunas de evidência, revisar os oráculos para independência e só então avaliar promoção individual para CLOSED. Não alterar produção para facilitar os testes; qualquer defeito precisa de reprodução discriminante e justificativa temporal.

## 50. F-028 — evidência pós-merge dos 167 fallbacks Ignored — 2026-11-16

**PR #946 MERGED:** `3acf24c67d6e573120134b5b29ebbbf2a2893940`; HEAD testado `162772ad8fba7249f8f4c7b8fb67b9816202cc80`, tree SHA integrado/testado `77840465f65a3d6f4f522ea475e9f1d7a9f2c320`.

| Verificação | Resultado |
|---|---|
| Inventário do CSV | 167 efetivos Ignored |
| DD + FD | 334 execuções prefixadas |
| M1 | Prefixo em PC 0 e terminal em PC 1 |
| Refetch do terminal | Nenhum |
| R | Dois incrementos |
| Custo | BASE + 4 T-states |
| CI Z80 focused #43882287467 | PASS; matriz 1/1 |
| SMS Runtime #43882287458 | PASS; format, Clippy, testes e build Windows x64 |

**Estado F-028: AUDIT_PARTIAL.** A matriz é estrutural, não um oracle semântico independente. Ainda falta verificar efeitos arquiteturais e requests/ordem/endereço por família, além de WAIT/snapshot por classe e integração temporal. E7 IN_PROGRESS; E8 BLOCKED.

## 51. F-028 — request-trace diferencial pós-merge — 2026-11-16

**PR #948:** MERGED em `9c1fc19fa382a3272e3ffe04cc6beb01411319b7`; HEAD testado `74715cbc777acc6625745489a4dfa495cde18ee9`.

- [x] Percorrer os 167 bytes efetivos Ignored sob DD e FD (334 execuções prefixadas).
- [x] Comparar tipo/ordem de request, endereço, base T-states e ciclos internos contra BASE.
- [x] Confirmar M1 em PC 0/1, sem refetch, R +2 e custo BASE +4T.
- [x] CI Z80 focused #43884118254 PASS e Rust quality/unit PASS.
- [x] SMS Runtime #43884118298 PASS, incluindo build Windows x64.
- [ ] Distinguir stream fetch de acesso de dados no normalizador de endereços; não ajustar toda leitura `< 0x0100` por suposição.
- [ ] Registrar valores de leitura e payloads de escrita; comparar estado arquitetural final.
- [ ] Aplicar WAIT e snapshot/restore a cada classe de request.
- [ ] Obter review adversarial independente. GitHub não registra review submetido em #948; Codex review falhou por limite de uso.
- [ ] Localizar e validar workflow push pós-merge. O status combinado do merge SHA está vazio e nenhum run pós-merge foi exposto; classificar como NOT VERIFIED, não PASS.

A matriz continua estrutural e não fecha F-028. E7 IN_PROGRESS; E8 BLOCKED.

## 52. Gate de revisão adversarial dos merges E7 — 2026-11-16

- [x] Reconciliar o estado remoto das PRs #936, #942, #943, #944, #945, #946, #947 e #948: todas MERGED.
- [x] Confirmar que os diffs integrados deste conjunto são testes/documentação, sem alteração de CPU, decoder, Scheduler ou runtime de produção.
- [x] Consultar reviews e review threads no GitHub: nenhuma submissão encontrada nas oito PRs.
- [x] Confirmar que os reviews Codex em #946 e #948 falharam por limite de uso; não contar como review.
- [ ] Obter revisão adversarial independente e registrada no GitHub para a matriz E7 integrada e suas limitações.

A ausência de review no GitHub não prova que não houve inspeção fora do repositório; significa que não há evidência auditável. A gate permanece BLOCKED. E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.


## Atualização de evidências — 16/11/2026

- [x] PR #950 integrada: matriz BASE↔DD/FD para os 167 Ignored, WAIT, snapshot/restore e payloads; CI #43888693825 e SMS Runtime #43888693845 PASS no HEAD validado.
- [x] PR #951 integrada: previous-Q e WZ das formas simples LDI/LDD/CPI/CPD; CI #43890238681 e SMS Runtime #43890238056 PASS.
- [x] PR #952 integrada: matriz WZ/MEMPTR por família e classificação de proveniência; Documentation patch hygiene e Rust quality PASS no CI #43890997152.
- [x] PR #953: ambos os resultados condicionais e PC final exato para os 167 Ignored sob DD/FD; CI #43891989147 e SMS Runtime #43891989127 PASS no HEAD validado.
- [x] PR #954: IN/OUT imediato, WZ de RETN/RETI/RST e preservação de WZ em IM0/IM1; CI #43893014345, workspace regression e SMS Runtime #43893014300 PASS.
- [ ] PR #955: asserção WZ para BASE EX (SP),HL; aguardar CI.
- [x] Control flow WZ: condições tomadas/não tomadas e wrap em `z80_conditional_control_flow_coverage.rs`.
- [x] IM2 WZ: PC vetorial observado em `snapshot_restore_e7_2e.rs`.
- [x] Block repeat WZ: 64 cenários microsequenciados e quatro formas com snapshot/restore em cada fronteira.
- [ ] Revalidar gates e documentação no HEAD final da main antes de declarar E7 CLOSED.

## Reconciliação após PR #955 — 16/11/2026

PR #955 foi integrada na main pelo commit `67f4149e1933d18ffa5201da66ca6ff46c598e32`. O diff é test-only: quatro linhas de assertion em `native/z80_cpu/src/cpu.rs`, teste `exchange_stack_hl_and_di_ei_update_state`, confirmando `cpu.wz == 0xABCD` após BASE `EX (SP),HL`. Nenhuma alteração de produção, decoder, microsequenciador, Scheduler, SMS ou SG-1000.

Validação no HEAD exato da PR (`cbc75e9205db5f1c29726962ce3d3cdd312f4ba1`):
- CI #43897766618: PASS — Z80 focused, rustfmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD, Z80 Block Instruction Flags v5.0, Woodmass MEMPTR e Rust quality/unit.
- SMS Debugger Runtime #43897766471: PASS — format, Clippy Release, testes SMS Runtime e build Release Windows x64.
- SMS debugger Windows Release validation: SKIPPED; não contar como PASS.
- Workflow push pós-merge no SHA `67f4149e1933d18ffa5201da66ca6ff46c598e32`: não localizado; NOT_RUN/NOT_VERIFIED.

A pendência específica de WZ para BASE `EX (SP),HL` está coberta. F-023 permanece AUDIT_PARTIAL até revisão integral da matriz e reconciliação da proveniência. F-028 e F-022 permanecem IN_PROGRESS. A validação integrada do Scheduler/determinismo e a revisão adversarial transversal ainda não fecham E7.

E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanecem BLOCKED. Não converter SKIPPED/NOT_RUN em PASS nem promover testes internos comparativos a evidência física.

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


## Atualização de checklist pós-PR #964 — 16/11/2026

- [x] PR #964 integrada: INT no último T-state da repetição LDIR/LDDR/CPIR/CPDR pelo Scheduler, IM1/RETI e retomada com WAIT fixture e snapshot/restore em cada safe point.
- [x] CI #43903631872: Rust workspace regression e Rust quality/unit PASS.
- [x] SMS Runtime #43903632358: format, Clippy, testes e build Windows x64 PASS.
- [ ] Não converter Scheduler stress, Z80 focused ou SMS debugger Windows Release SKIPPED em PASS; executar gates correspondentes quando aplicável.
- [ ] PR #967: validar classificação stream/data DD/FD no HEAD atual; aguardar checks obrigatórios antes de integrar.
- [ ] Obter revisão adversarial independente registrada para a matriz E7 integrada.

E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.

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



## Atualização operacional — main pós-PR #975 (16/11/2026)

Main: `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`.

| Família | Estado vigente | Evidência/pendência |
|---|---|---|
| F-022 — Block Instructions | INTERNAL_REVIEW_COMPLETE (candidate; independent review pending) | Oito formas; 786.432 cenários simples, matrizes micro/repetição, ROM hardware-derived limitada e integração SMS/Scheduler; auditoria interna completa, review independente pendente |
| F-023 — WZ/MEMPTR | INTERNAL_REVIEW_COMPLETE (candidate; PR #977 and independent review pending) | Revisão interna dos handlers e matriz concluída; PR #977 cobre JR/DJNZ WZ sob DD/FD; HW somente RRD/RLD, INT/COMP explícitos, clones NOT_VERIFIED |
| F-028 — DD/FD | IN_PROGRESS | 85 UsesIndex, 167 Ignored; #975 cobre wrap 16-bit de retorno CALL/RST em DD/FD; falta revisão adversarial integral de exceções/limites |

PR #975 HEAD `7574245338549fd8f37a7483ef41b7fbc306ffe5`: CI #43937118700 Z80 focused e Rust quality/unit PASS; SMS Runtime #43937118785 PASS; Scheduler stress SKIPPED. Merge squash: `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`.

WAIT dos testes é fixture, não WAIT físico. LONG_PASS do Scheduler não significa TEMPORAL_PASS global. E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED. A integração da máquina SG-1000 ao Scheduler permanece P2/DEFERRED.


**Regra de aceitação:** os estados acima são auditorias internas/candidatos, não fechamentos formais. A revisão independente registrada no GitHub e a validação PR #977 no HEAD exato são gates obrigatórios antes de fechar E7.



## Revalidação PR #977

HEAD `a783a3c231b5de1678ffc71eb8b1e8dcda42c724`: Z80 focused/Rust quality CI #43939973117 SUCCESS; SMS Runtime #43939957730 SUCCESS; Scheduler stress SKIPPED. WZ/PC de JR/DJNZ/JR cc sob DD/FD Ignored e wrap PC=FFFD passaram. Revisão independente registrada ainda pendente para F-022/F-023/F-028. E7 IN_PROGRESS; E8 BLOCKED.



## Atualização F-028 — PR #977

HEAD `35a9028971a7a5f7140a8cbf097c63255214ab9d`: Z80 focused/Rust quality CI #43940980741 SUCCESS; SMS Runtime #43940971722 SUCCESS; Scheduler stress SKIPPED. A matriz de 167 Ignored agora valida WZ, PC, bus trace, WAIT/snapshot, CALL/RST stack wrap e branches relativos DD/FD. F-028 é INTERNAL_REVIEW_COMPLETE/candidate, mas depende de review independente registrada. E7 IN_PROGRESS; E8 BLOCKED.
