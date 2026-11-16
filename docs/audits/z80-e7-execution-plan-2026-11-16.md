# Plano de execução — fechamento E7 do Z80

**Data da revisão:** 2026-11-16
**Baseline remota main:** `caa9a76a6f09d588155ea4e0cbc95f487f079143`
**Estado global:** E7 `IN_PROGRESS`; E8/ZEXDOC/ZEXALL `BLOCKED`.

Este documento organiza o trabalho subsequente a partir das evidências realmente observadas. Resultados de PR não são atribuídos à main até integração e validação no SHA pós-merge.

## 1. Estado remoto e evidências existentes

| Frente | Evidência | Limite que permanece |
|---|---|---|
| F-021 / PR #936 | ROM v5.0: 44 das 60 comparações principais (11 entradas I/O × quatro endereços); 390 B-loop records em seis grupos | Harness atômico; não cobre todas as formas I/O, microsequenciador, WAIT ou Scheduler |
| F-022 / main PRs #930/#932 | 786.432 casos microsequenciados para LDI/LDD/CPI/CPD; testes de repetição/interrupção IM2 para LDIR/LDDR/CPIR/CPDR | Reconciliar exatamente a contagem/variedade de cenários de repetição e Q; a ROM externa valida apenas 16 comparações repetitivas e usa executor atômico |
| F-022 / PR #936 | ROM v5.0: 16 comparações principais (LDIR/LDDR/CPIR/CPDR × quatro endereços) | Não cobre LDI/LDD/CPI/CPD simples; não valida microsequenciador, WAIT, snapshot de barramento ou Scheduler |
| F-023 / PR #942 | Woodmass test 09: 50.904.931 T-states, FNV-1 `F5AE5140`, 61 linhas × 32 colunas; referência upstream MEMPTR inclui RLD/RRD → HL+1 | O teste verifica saída agregada; mapear explicitamente a linha/colunas de RLD/RRD e distinguir o que é asserted diretamente do que está coberto pelo hash global |
| F-028 / PR #943 | Ledger dos 256 bytes DD/FD, 108 casos dirigidos por executor, INT/NMI entre DD/FD e 34.000 vetores DD/FD | Classificação não prova semântica integral por opcode/estado; falta reconciliar cada UsesIndex/Ignored com famílias E3/E4/E5/E6 e os gates de barramento |

### Mapeamento comprovado da ROM v5.0

O assembly upstream pinado em `MrKWatkins/ZXSpectrumNextTests`, commit `8adcbc8384b2311fb6d36d424478aef583d5c307`, contém 15 entradas `i_meta`, cada uma executada nos endereços `$8002`, `$8802`, `$A002` e `$A802`:

- **F-022:** LDIR, LDDR, CPIR e CPDR = 4 × 4 = 16 comparações.
- **F-021:** OTIR × 3, OTDR × 3, INIR × 3 e INDR × 2 = 11 × 4 = 44 comparações.
- **F-021 adicional:** 390 B-loop records = seis grupos de 65 (OTIR × 2, OTDR × 2, INIR e INDR). Não somar esses registros novamente às 60 comparações principais.

O teste não contém LDI/LDD/CPI/CPD simples. Essas quatro formas têm matriz microsequenciada própria na main, PR #930 integrada.

## 2. Próximas tarefas — ordem obrigatória

### P0 — fechar a proveniência e a cobertura de F-022

1. **Concluído nesta auditoria:** PR #932 adiciona `native/sms_core/tests/z80_ed_block_repeat_interrupt_observation.rs`, com exatamente 32 cenários = 4 opcodes (LDIR/LDDR/CPIR/CPDR) × 4 PCs ($8002/$8802/$A002/$A802) × 2 BC iniciais (0000/0002). Também há 4 cenários adicionais no teste `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume`, no microsequenciador compartilhado, um por opcode, com snapshot/restore em cada fronteira e observação via PUSH AF/RETI. O scorecard de 32 casos está correto; manter as duas camadas de evidência separadas.
2. **Cobertura atual reconciliada:** a matriz atômica PR #932 contém 32 casos (4 opcodes × quatro PCs × BC inicial 0000/0002); o teste no microsequenciador compartilhado cobre quatro casos (um por opcode), com INT no último T-state do repeat internal cycle e snapshot/restore em cada fronteira. O próximo incremento de código será espelhar os 32 cenários no microsequenciador, mantendo o teste de quatro casos existente, e comparar o estado arquitetural/traço com o executor atômico sem promovê-lo a oracle independente.
3. **Concluído para o caminho INT IM2 em PR #944:** a matriz varia Q inicial (00/A5), verifica Q=F e previous-Q=seed na aposentadoria do repeat; após aceitação IM2 verifica Q=0 e previous-Q=F; após PUSH AF, POP AF, EI e RETI verifica Q/previous-Q; após retomada verifica Q=F e previous-Q=0. CI #43873337726 e SMS Runtime #43873337682 SUCCESS no HEAD ef22ddc871e9790eb2fd65b1d4a70a6795c24872. Não extrapolar para NMI ou IM0/IM1, que não fazem parte desta matriz.
4. Separar resultados do executor atômico, microsequenciador, WAIT/snapshot e integração MachineScheduler. A comparação atomic↔micro é consistência interna, não oracle independente.
5. Reconciliar os 16 casos F-022 do ROM com a matriz interna e registrar explicitamente qualquer dimensão que o ROM não cobre.
6. Não alterar CPU/Scheduler sem uma divergência reproduzível, trace discriminante e causa-raiz.

**Critério de saída desta fatia:** matriz de repetição com cada caso rastreável a um teste e a uma fonte de expectativa; gaps remanescentes explicitamente listados. Isso não fecha automaticamente F-022.

### P1 — consolidar F-023 (RRD/RLD e WZ/MEMPTR)

1. **Concluído:** output do Woodmass test 09 extraído do CI #43870800175; linhas identificadas: RLD = `117F passed`, RRD = `34AE passed`. Esses quatro dígitos são assinaturas de resultado da suíte, não valores literais de WZ.
2. **Concluído:** o harness exige individualmente as duas linhas, incluindo os bytes de controle TAB/INK/PAPER, preservando FNV-1, dimensões, HALT e T-states. O teste explícito passou 1/1 no CI #43871236879, HEAD de código `df015e8c39e23d7d36da7076ef544e1892cff6a8`. O HEAD documental atual da PR #942, `53aaf5c415eba3acc4704d1d2214a358755fad08`, também recebeu CI #43871487677 e SMS Runtime #43871487582 SUCCESS; o diff adicional é apenas documentação.
3. **Matrizes existentes reconciliadas:** `native/sms_core/tests/z80_ed_neg_nibble_exhaustive.rs` contém 262.144 vetores atômicos (2 opcodes × 256 A × 256 memória × 2 estados C) para transformação e flags UM0080; o mesmo arquivo tem 16.384 vetores observáveis MEMPTR (2 opcodes × 4 HL × 4 A × 256 memória × 2 C), incluindo wrap de HL e BIT 0,(HL) subsequente. `native/z80_cpu/tests/ed_nibble_rotate_f023_observable.rs` contém 240 vetores microsequenciados (2 × 4 HL × 3 A × 5 memória × 2 C), com WAIT e snapshot/restore em cada fronteira. Não duplicar essas matrizes.
4. **Separação de oráculos:** UM0080 cobre transformação, flags documentadas e timing; Woodmass/NEC fornece evidência hardware-derived para MEMPTR observável nas linhas RLD/RRD (117F/34AE); MAME/ares continuam comparação de implementação para X/Y/WZ não documentados. Não transformar X/Y não documentados em requisito universal sem fonte adequada.
5. **Fonte complementar identificada:** Patrik Rak, raxoft/z80test, commit `c490c0ca94f1021509f8bd656b413861bd7cfb41`. O README afirma que os programas comparam resultados com um ZX Spectrum 48K real com Zilog Z80; a variante `z80memptr` executa BIT N,(HL) após cada instrução e o arquivo `src/tests.asm` inclui explicitamente RLD e RRD. O próprio autor alerta que os vetores atuais não foram desenhados especificamente para estressar MEMPTR e podem deixar muitos defeitos sem detecção. Tratar como evidência hardware-derived complementar, não como oracle específico suficiente. Antes de integrar, validar versão/artefato TAP, licença, hash e extrair o resultado por instrução.

**Critério de saída desta fatia:** a evidência atual já combina matrizes internas exaustivas/dirigidas e assinaturas hardware-derived por instrução. Resta reconciliar o significado/abrangência das 32 colunas do Woodmass test 09 com os vetores observáveis internos e delimitar quais propriedades X/Y/WZ são comprovadas, sem afirmar que 117F/34AE são valores literais de WZ. F-023 permanece AUDIT_PARTIAL até essa reconciliação documental e revisão independente; não requer universalidade CMOS/clones nem hardware SMS físico.

### P2 — completar a reconciliação semântica F-028

1. **Concluído documentalmente:** os 85 UsesIndex foram reconciliados byte a byte: E3A1=9, E3A2=5, E4A=15, E4B=8, E4C=2, F-036=24, PR #680 half-register INC/DEC/LD immediate=6 e F-037 half-register ALU=16. Soma=85; E3/E4/F-036/F-037 preservados. Detalhe em `docs/audits/z80-e7-f028-usesindex-reconciliation-2026-11-16.md`.
2. **Próximo gap real:** os 167 Ignored efetivos ainda não têm fallback sem refetch provado byte/família por byte/família. `prefix_fallback_e6.rs` tem nove testes representativos; PR #943 cobre exceções de troca/HALT/ED/control em 108 combinações por executor, e `prefix_fsm.rs` cobre 257 prefixos/last-prefix-wins. Gerar uma matriz das 167 entradas por família BASE e distinguir cobertura prefixada de cobertura BASE sem prefixo.
3. Manter separadas as transições especiais DD/FD repetidos, ED cancel-index e CB indexed-CB; o helper `classify_index_opcode` isolado não representa toda a FSM.
4. Ampliar semântica por opcode/estado apenas onde houver lacuna real, sem duplicar E4/E5/E6 nem reabrir etapas validadas.
5. Completar cenários de HALT, Q/previous-Q, interrupções, WAIT e snapshot nas fronteiras relevantes.

**Critério de saída desta fatia:** matriz de 256 bytes com rastreabilidade a semântica/testes, não apenas classificação; exceções e lacunas residuais documentadas.

### P3 — auditoria mestre e revisão independente

1. Reconciliar checklist mestre, scorecard e matriz DD/FD; remover afirmações históricas que contradigam o estado atual, preservando a proveniência histórica.
2. Revisar diffs completos das PRs #936, #942 e #943; confirmar exatamente quais arquivos de produção, testes, fixtures e workflows foram alterados.
3. Obter revisão técnica independente antes de converter Drafts em Ready ou integrar.
4. Executar os gates aplicáveis no SHA final de cada PR e novamente na main pós-merge.
5. E8 só pode ser desbloqueada depois que cada critério E7 estiver CLOSED ou NOT_APPLICABLE com justificativa e evidência verificável.

## 3. Restrições de execução

- Não iniciar ZEXDOC/ZEXALL.
- Não fazer merge automático das PRs Draft.
- Não atribuir CI de PR à main.
- `SKIPPED` significa `NOT RUN`.
- Não tratar atomic↔micro como oracle independente.
- Não aumentar limites de execução sem prova de que o limite é a causa.
- Não modificar CPU, Scheduler ou runtime para contornar limitações de harness.
- Preservar E4A/E4B/E5/E6 e os demais fechamentos validados sem regressão concreta.
- Hardware SMS direto e CMOS/clones permanecem `NOT RUN`/`NOT_VERIFIED`; não são gates universais automáticos do perfil NMOS delimitado.

## 4. Baseline de PRs em 2026-11-16

| PR | Base | HEAD observado | CI relevante |
|---|---|---|---|
| #936 | `caa9a76a6f09d588155ea4e0cbc95f487f079143` | `306db2a91ff796b4c5f66eabc4ebd4c4baad903e` | #43852582196 e SMS Runtime #43852582229 SUCCESS |
| #942 | `caa9a76a6f09d588155ea4e0cbc95f487f079143` | `15a8f1c1fe7b4b10f1526e6310b9308fdfcaf021` | #43851550529 e SMS Runtime #43851550520 SUCCESS |
| #943 | `caa9a76a6f09d588155ea4e0cbc95f487f079143` | `462ec750b9b4c20e4f89b4545e4161435bae6a13` | #43865677187 e SMS Runtime #43865677170 SUCCESS |

Essas SHAs são fotografias da auditoria de 2026-11-16; sempre consultar o estado remoto antes de executar a próxima tarefa.


### PR #944 — matriz IM2 do microsequenciador
- Branch: `test/z80-f022-repeat-microseq-matrix-20261116`
- HEAD: `c88739b6f027e62dfc28f4379bbafc633bbab7ed`
- Alteração: somente 164 linhas de testes dentro do módulo `ed_block_transfer_repeat_interrupt_tests` em `native/z80_cpu/src/sequencer.rs`.
- CI #43872577766 SUCCESS; SMS Runtime #43872577771 SUCCESS.
- 64 cenários: 4 opcodes × 4 padrões PC × 2 BC iniciais × 2 seeds Q; INT no último T-state do repeat, handler IM2, RETI/resume e snapshot/restore por fronteira.
- F-022 permanece AUDIT_PARTIAL: a matriz amplia o microsequenciador, mas não prova WZ independente nem toda a evolução Q durante acknowledge/handler.

 
### PR #944 — Q/previous-Q através de INT IM2
- HEAD validado: `ef22ddc871e9790eb2fd65b1d4a70a6795c24872`.
- CI #43873337726 SUCCESS; SMS Runtime #43873337682 SUCCESS.
- As assertions de Q/previous-Q passaram na aposentadoria do repeat, aceitação IM2, PUSH AF, POP AF, EI, RETI e retomada.
- Escopo: INT IM2 apenas; não afirmar cobertura NMI ou IM0/IM1.

 
### PR #944 — estado final observado após a extensão Q
- PR #944 continua OPEN/DRAFT, base `caa9a76a6f09d588155ea4e0cbc95f487f079143`, HEAD `ef22ddc871e9790eb2fd65b1d4a70a6795c24872`.
- CI #43873337726 SUCCESS; SMS Runtime #43873337682 SUCCESS.
- A extensão de Q/previous-Q está validada para a sequência INT IM2 → PUSH AF → POP AF → EI → RETI → retomada; NMI e IM0/IM1 continuam fora do escopo.
