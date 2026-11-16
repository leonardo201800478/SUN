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
2. **Concluído documentalmente:** os 167 Ignored efetivos foram agrupados por família BASE x/y/z: x=0:46, x=1:26, x=2:40, x=3:55. A lista exata permanece no ledger CSV e a decomposição está em `docs/audits/z80-e7-f028-usesindex-reconciliation-2026-11-16.md`. O gap de código permanece: `prefix_fallback_e6.rs` tem nove testes representativos; PR #943 cobre exceções de troca/HALT/ED/control em 108 combinações por executor, e `prefix_fsm.rs` cobre 257 prefixos/last-prefix-wins. Próximo passo: gerar fixtures por formato de request (register-only, immediate, relative, memory, stack, I/O, control flow) e validar fallback prefixado para cada opcode implementado, sem contar teste BASE sem prefixo como cobertura DD/FD.
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

## 4. Baseline remota reconciliada — 2026-11-16

**Main:** `caa9a76a6f09d588155ea4e0cbc95f487f079143`. As quatro PRs permanecem baseadas nesta SHA; não houve integração das candidatas.

| PR | HEAD atual | Evidência de CI | Situação |
|---|---|---|---|
| #936 | `c77759796349e02ee1f31449ce2a616b8a16d5d0` | CI #43873956181 SUCCESS; SMS Runtime #43873956227 SUCCESS | OPEN/DRAFT. O commit c777 reverteu uma matriz temporária para manter o microsequenciador isolado na #944; ROM v5.0 permanece nesta PR. |
| #942 | `53aaf5c415eba3acc4704d1d2214a358755fad08` | CI #43871487677 SUCCESS; SMS Runtime #43871487582 SUCCESS | OPEN/DRAFT. As assertions por linha RLD/RRD foram validadas no código df015e8; o HEAD atual acrescenta documentação. |
| #943 | `b95cb0ecd730cf8dbd3d34abb71716013d0b5ca0` | CI/SMS Runtime do HEAD atual aguardando consulta | OPEN/DRAFT. Documentação e ledger de reconciliação F-028, sem alteração de CPU/runtime. |
| #944 | `ef22ddc871e9790eb2fd65b1d4a70a6795c24872` | CI #43873337726 SUCCESS; SMS Runtime #43873337682 SUCCESS | OPEN/DRAFT. Matriz IM2 do microsequenciador, somente testes em sequencer.rs. |

### PR #944 — matriz IM2 e Q/previous-Q

- 64 cenários: quatro opcodes × quatro padrões PC × dois valores BC iniciais × dois seeds Q.
- INT é afirmado no último T-state do ciclo interno de repetição; handler IM2 observa F com PUSH AF/POP AF, EI e RETI.
- Q/previous-Q verificados na aposentadoria, aceitação IM2, handler e retomada.
- Snapshot encode/decode/restore exercitado nas fronteiras de barramento e T-state interno.
- CI #43873337726 e SMS Runtime #43873337682 SUCCESS no HEAD ef22ddc.
- Escopo: INT IM2. NMI e IM0/IM1 não foram validados por esta matriz.

### PR #943 — reconciliação F-028

- Ledger: 256 bytes; 85 UsesIndex, 167 Ignored, 1 IndexedCB, 1 ED cancel e 2 prefix repeats.
- Novo relatório: `docs/audits/z80-e7-f028-usesindex-reconciliation-2026-11-16.md`.
- Os 85 UsesIndex foram reconciliados com E3A1/E3A2/E4A/E4B/E4C, F-036, PR #680 e F-037. O gap residual está nos 167 Ignored: fallback prefixado não foi provado byte/família por byte/família.
- A matriz de classificação e os 108 casos dirigidos não fecham a semântica de todos os opcodes/estados.

### Governança

- Todas as PRs #936/#942/#943/#944 continuam OPEN/DRAFT; nenhuma review independente registrada.
- Não fazer merge automático nem atribuir CI de PR à main.
- Nenhuma alteração de produção CPU/Scheduler/runtime nesta continuidade. A PR #944 altera somente testes em `native/z80_cpu/src/sequencer.rs`.
- E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED.


### F-028 — decomposição dos 167 Ignored
- 167 bytes foram agrupados por decoder BASE x/y/z: x=0 = 46; x=1 = 26; x=2 = 40; x=3 = 55.
- O próximo teste deve percorrer os 167 bytes implementados com fixtures específicos por formato: register-only, immediate, relative, indirect/absolute memory, stack, I/O e control flow.
- Assertions estruturais mínimas: prefix M1 e R incrementados, exatamente um fetch M1 do opcode terminal, ausência de refetch, PC/R corretos, custo 4T do prefixo e requests/ordem de operandos preservados.
- WAIT/snapshot devem ser cobertos por classe de request. Instruções BASE sem implementação e codificações reservadas devem ser marcadas NOT_IMPLEMENTED/NOT_APPLICABLE, nunca PASS por mera classificação.
- Nenhuma alteração de CPU de produção é autorizada sem falha reproduzível no fallback.

## Atualização pós-merge — PR #946 e próximo recorte F-028

**Baseline main:** `3acf24c67d6e573120134b5b29ebbbf2a2893940`.

PR #946 está MERGED. A matriz ledger-driven executou 167 bytes efetivos Ignored sob DD e FD (334 casos), confirmou M1 em PC 0/1, ausência de refetch, R +2 e custo BASE +4T. CI Z80 focused #43882287467 e SMS Runtime #43882287458 passaram no HEAD `162772ad8fba7249f8f4c7b8fb67b9816202cc80`, cujo tree é idêntico ao merge.

### Próximo passo F-028 — requests e efeitos BASE por família

A matriz estrutural não fecha a semântica. Próximo trabalho deve decompor os 167 bytes em famílias de request e acrescentar fixtures independentes por formato:
1. Sem operandos e registrador/flags.
2. Imediato de 8/16 bits e fluxo relativo.
3. Memória indireta e absoluta, leitura e escrita.
4. Stack: PUSH/POP, CALL/RET/RST e EX (SP),HL.
5. I/O: IN/OUT imediato e (C).
6. Controle de fluxo: condições tomadas/não tomadas, JP (HL), HALT, DI/EI.

Para cada família, comparar explicitamente o caminho BASE com DD e FD: opcode terminal consumido uma vez, endereços e ordem dos requests, valores transferidos, efeitos arquiteturais e custo adicional de 4T. WAIT e snapshot/restore devem ser aplicados em cada classe de request, não apenas no M1. Se um opcode não for coberto pelo executor ou se o comportamento não tiver referência independente, registrar NOT_IMPLEMENTED/NOT_VERIFIED; não inferir PASS pela classificação do ledger.

Não alterar CPU/decoder/FSM/microsequenciador de produção sem uma falha reproduzível. F-028 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

## Atualização pós-merge — PR #948 request-trace diferencial — 2026-11-16

**Main:** `9c1fc19fa382a3272e3ffe04cc6beb01411319b7`. **HEAD testado:** `74715cbc777acc6625745489a4dfa495cde18ee9`. CI Z80 focused #43884118254 e SMS Runtime #43884118298 SUCCESS nos jobs aplicáveis.

A nova matriz compara a sequência de requests BASE × DD/FD para os 167 bytes efetivos Ignored: tipo, endereço, duração base e ciclos internos, além de M1/refetch, R e custo +4T. Ela não compara payloads, valores lidos ou estado final; leituras são sintéticas, e WAIT/snapshot por classe continuam ausentes.

O normalizador de memória abaixo de `0x0100` ainda não prova que o endereço pertence ao stream. O fixture atual mantém os endereços de dados esperados acima dessa página, mas a hipótese deve virar asserção ou ser substituída por classificação explícita via PC no request.

**Próximo recorte:** normalização stream/data robusta; payloads e efeitos arquiteturais por família; WAIT/snapshot por classe; oracle independente e revisão adversarial. Nenhuma alteração de CPU/decoder/FSM/microsequenciador de produção é autorizada por esta evidência.

F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## Atualização operacional — 16/11/2026 (pós #950–#952)

- P0 recebeu a matriz #950 para payloads, estado arquitetural, WAIT e snapshot/restore nos 167 fallbacks. A PR #953 integrou as duas fixtures condicionais e o PC final exato; CI #43891989147 e SMS Runtime #43891989127 PASS no HEAD validado.
- P1 recebeu as asserções previous-Q e WZ nas quatro formas simples de bloco pela #951. LDI/LDD preservam WZ; CPI/CPD alteram WZ em ±1.
- P2 recebeu a matriz por família WZ/MEMPTR na #952. Control flow, IM2, IN/OUT imediato, IM0/IM1, RETN/RETI e RST possuem testes WZ rastreáveis após a PR #954 (CI #43893014345, workspace regression e SMS Runtime #43893014300 PASS). A única lacuna de teste identificada na matriz é a asserção BASE EX (SP),HL da PR #955.
- Não reimplementar UsesIndex nem iniciar E8. Manter F-028/F-022/F-023 abertas até passar a PR #955 e concluir a revisão adversarial documentada.

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
