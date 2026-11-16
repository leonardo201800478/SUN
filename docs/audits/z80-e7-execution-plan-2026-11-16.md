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
3. Auditar Q/previous-Q antes da repetição, no limite final do ciclo de repetição, durante acknowledge e após RETI. Não extrapolar o valor de Q observado em um único ponto para toda a sequência.
4. Separar resultados do executor atômico, microsequenciador, WAIT/snapshot e integração MachineScheduler. A comparação atomic↔micro é consistência interna, não oracle independente.
5. Reconciliar os 16 casos F-022 do ROM com a matriz interna e registrar explicitamente qualquer dimensão que o ROM não cobre.
6. Não alterar CPU/Scheduler sem uma divergência reproduzível, trace discriminante e causa-raiz.

**Critério de saída desta fatia:** matriz de repetição com cada caso rastreável a um teste e a uma fonte de expectativa; gaps remanescentes explicitamente listados. Isso não fecha automaticamente F-022.

### P1 — consolidar F-023 (RRD/RLD e WZ/MEMPTR)

1. **Concluído:** output do Woodmass test 09 extraído do CI #43870800175; linhas identificadas: RLD = `117F passed`, RRD = `34AE passed`. Esses quatro dígitos são assinaturas de resultado da suíte, não valores literais de WZ.
2. **Concluído no código candidato:** o harness agora exige individualmente as duas linhas, incluindo os bytes de controle TAB/INK/PAPER, preservando FNV-1, dimensões, HALT e T-states. O teste explícito passou 1/1 no CI #43871236879, HEAD `df015e8c39e23d7d36da7076ef544e1892cff6a8`. O job SMS Runtime deste HEAD ainda estava em execução no momento da atualização.
3. Construir uma matriz de RRD/RLD por A, (HL), carry, WZ inicial, wrap de HL, e fronteiras WAIT/snapshot; manter a matriz atômica de 262.144 casos e os 240 casos microsequenciados existentes.
4. Separar UM0080 (flags documentadas e timing), Woodmass/NEC hardware-derived (MEMPTR), e referências MAME/ares (comparação de implementação). Não transformar X/Y não documentados em requisitos primários sem fonte adequada.
5. **Fonte complementar identificada:** Patrik Rak, raxoft/z80test, commit `c490c0ca94f1021509f8bd656b413861bd7cfb41`. O README afirma que os programas comparam resultados com um ZX Spectrum 48K real com Zilog Z80; a variante `z80memptr` executa BIT N,(HL) após cada instrução e o arquivo `src/tests.asm` inclui explicitamente RLD e RRD. O próprio autor alerta que os vetores atuais não foram desenhados especificamente para estressar MEMPTR e podem deixar muitos defeitos sem detecção. Tratar como evidência hardware-derived complementar, não como oracle específico suficiente. Antes de integrar, validar versão/artefato TAP, licença, hash e extrair o resultado por instrução.

**Critério de saída desta fatia:** resultado hardware-derived RLD/RRD rastreável por caso e por output esperado, com escopo e lacunas X/Y/WZ explicitados. Não requer universalidade CMOS/clones nem hardware SMS físico.

### P2 — completar a reconciliação semântica F-028

1. Relacionar cada um dos 85 bytes UsesIndex às famílias E3A1/E3A2/E4A/E4B/E4C/E5 e às exceções documentadas.
2. Para os 167 bytes Ignored, provar fallback sem refetch, preservação de M1/R e custo prefixado nos grupos implementados.
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
