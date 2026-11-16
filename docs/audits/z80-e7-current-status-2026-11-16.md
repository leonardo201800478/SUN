# Z80 E7 — fechamento da rodada de testes e estado consolidado

**Data:** 2026-11-16
**Main de referência:** caa9a76a6f09d588155ea4e0cbc95f487f079143
**Perfil:** Zilog Z80 NMOS clássico delimitado aos sistemas SMS/SG-1000 suportados. Não generalizar automaticamente para CMOS, Z180, R800 ou clones.
**Estado global:** E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.

## 1. Resumo executivo

A rodada encerrou os testes atuais de F-022, F-023 e F-028. As três PRs permanecem Draft, sem merge nesta rodada.

| Frente | PR | HEAD de código validado | CI Z80 focused | SMS Runtime | Estado |
|---|---:|---|---|---|---|
| F-021 — I/O block B-loop | [#936](https://github.com/leonardo201800478/sega-emulator/pull/936) | 306db2a91ff796b4c5f66eabc4ebd4c4baad903e | [#43852582196](https://github.com/leonardo201800478/sega-emulator/actions/runs/43852582196) SUCCESS | [#43852582229](https://github.com/leonardo201800478/sega-emulator/actions/runs/43852582229) SUCCESS | AUDIT_PARTIAL |\n| F-022 — Block Instruction Flags ROM v5.0 | [#936](https://github.com/leonardo201800478/sega-emulator/pull/936) | 306db2a91ff796b4c5f66eabc4ebd4c4baad903e | [#43852582196](https://github.com/leonardo201800478/sega-emulator/actions/runs/43852582196) SUCCESS | [#43852582229](https://github.com/leonardo201800478/sega-emulator/actions/runs/43852582229) SUCCESS | AUDIT_PARTIAL |
| F-023 — Woodmass MEMPTR | [#942](https://github.com/leonardo201800478/sega-emulator/pull/942) | 7402ff138ced95da2d8b9e0008919c8cd22484f1; HEAD documental da PR 15a8f1c1fe7b4b10f1526e6310b9308fdfcaf021 | [#43851550529](https://github.com/leonardo201800478/sega-emulator/actions/runs/43851550529) SUCCESS | [#43851550520](https://github.com/leonardo201800478/sega-emulator/actions/runs/43851550520) SUCCESS | AUDIT_PARTIAL |
| F-028 — exceções DD/FD | [#943](https://github.com/leonardo201800478/sega-emulator/pull/943) | 98c1cf41587f6af66c3d272e4b07a638d5da7919 | [#43859000666](https://github.com/leonardo201800478/sega-emulator/actions/runs/43859000666) SUCCESS | [#43859000676](https://github.com/leonardo201800478/sega-emulator/actions/runs/43859000676) SUCCESS | AUDIT_PARTIAL |

Os SHAs foram conferidos com o campo head_sha dos respectivos Actions runs. Jobs SKIPPED não são PASS. Os jobs não relacionados aos diffs foram ignorados pela classificação de impacto; isso não valida o workspace integral, os hosts ou o MachineScheduler.

Nenhuma PR altera CPU, decoder, microsequenciador, MachineScheduler ou runtime. As mudanças são harnesses/testes, CI e documentação.

## 2. F-022 — Z80 Block Instruction Flags Test v5.0

- PR #936, Draft; HEAD validado 306db2a91ff796b4c5f66eabc4ebd4c4baad903e.
- Upstream: MrKWatkins/ZXSpectrumNextTests, commit 8adcbc8384b2311fb6d36d424478aef583d5c307; fixture Tests/ZX48_ZX128/Z80BlockInstructionFlags/z80bltst.sna; Git blob SHA 41b7b0da0f5abfd5d2841fbae005f05291986f8b.
- CI #43852582196: Z80 focused SUCCESS; SMS Runtime #43852582229 SUCCESS.
- ROM v5.0: 1 passed/0 failed em 6,50 s.
- O assembly upstream pinado contém 15 entradas i_meta, executadas em quatro endereços ($8002, $8802, $A002, $A802): 60 comparações principais. Quatro entradas são F-022 — LDIR, LDDR, CPIR e CPDR (16 comparações); onze são F-021 — OTIR×3, OTDR×3, INIR×3 e INDR×2 (44 comparações). Os 390 B-loop records adicionais são seis grupos de 65 (OTIR×2, OTDR×2, INIR, INDR), também F-021, sem dupla contagem. A fonte é z80_block_flags_test.asm no commit upstream 8adcbc8384b2311fb6d36d424478aef583d5c307.
- O mesmo CI também executou 34.000 SingleStepTests DD/FD no SHA 306db2a, mas esse resultado é evidência separada de F-028; não é cobertura do ROM F-022.

### Diagnóstico preservado

A primeira execução limitada a 50M T-states expirou durante LDIR. A instrumentação confirmou:
- LDIR de calibração em PC 819F, BC 0CE5→0000, consumindo 69.316 T-states nominais;
- término 100T antes da borda INT; preparação até OTIR de 46T, deixando 54T;
- OTIR a 21T por iteração; três iterações completam antes do boundary seguinte;
- assembly upstream captura B real no ISR e o usa no cálculo de flags; não exige B=15;
- harness diferencia o LDIR principal suspenso do LDIR curto executado dentro do handler IM2.

O limite de 80M T-states já existia no harness e não foi aumentado nesta investigação. O PASS é específico para a ROM no modelo Spectrum 48K do teste; não prova todas as operações de bloco nem hardware SMS.

**Escopo do harness:** o ROM é executado pelo executor atômico Z80::step com Spectrum48Bus; não exercita o microsequenciador, WAIT, snapshot de transação ou MachineScheduler. **Limite:** a suíte upstream é declarada não exaustiva. Casos adicionais de iteração simples, flags/Q/IRQ e integração temporal continuam pendentes. F-022 permanece AUDIT_PARTIAL.

## 3. F-023 — Mark Woodmass MEMPTR/WZ

- PR #942, Draft. Código/harness testado em 7402ff138ced95da2d8b9e0008919c8cd22484f1; HEAD documental atual 15a8f1c1fe7b4b10f1526e6310b9308fdfcaf021.
- CI #43851550529 e SMS Runtime #43851550520: SUCCESS.
- Suíte: Z80 Test Suite (2008), Mark Woodmass, caso MEMPTR 09.
- Resultado exato: HALT em 0x80E6; 50.904.931 T-states; FNV-1 F5AE5140; 61 linhas; largura máxima 32 colunas.
- TAP original obtido no runner, tamanho e SHA3-512 verificados; ROM Spectrum 48K reconstruída no runner e Git blob SHA verificado. Artefatos não são commitados.

O caso fornece evidência externa hardware-derived para MEMPTR, incluindo a regra RLD/RRD → HL+1 documentada pela suíte. O harness usa executor atômico Z80::step e T-states nominais; não exercita microsequenciador, WAIT, snapshot de transação ou MachineScheduler. Não fecha X/Y não documentados em todas as famílias, toda a tabela de WZ, todos os limites temporais ou validação física SMS. F-023 permanece AUDIT_PARTIAL.

## 4. F-028 — matriz de exceções DD/FD

- PR #943, Draft; HEAD de código/harness validado 98c1cf41587f6af66c3d272e4b07a638d5da7919.
- CI #43859000666: Z80 focused SUCCESS, Rust quality/unit SUCCESS, documentation hygiene SUCCESS.
- SMS Runtime #43859000676: SUCCESS; Release format, Clippy, testes e build Windows x64 concluídos.
- Teste: native/z80_cpu/tests/z80_dd_fd_exception_matrix_f028.rs.

Nove cadeias: DD, FD, DD DD, DD FD, FD DD, FD FD, DD FD DD, FD DD FD e DD FD DD FD.

Quatro testes cobrem:
1. EX AF,AF' (08), EXX (D9), EX DE,HL (EB);
2. HALT (76);
3. ED NEG (ED 44), cancelando substituição indexada antes do decoder ED;
4. NOP, DAA, CPL, SCF, CCF, DI e EI como instruções que ignoram substituição IX/IY.

Total: 108 combinações prefixadas por executor (45 nas cinco formas principais e 63 nas sete formas adicionais). Atomic e microsequenciador são comparados. O microsequenciador recebe um WAIT por M1 e executa snapshot encode/decode/restore antes e depois de cada conclusão de barramento.

Verificações: PC com wrap em 0xFFFF, R por M1, Q/previous-Q, flags, WZ, IX/IY, bancos alternativos, IFF/IM, interrupções pendentes, HALT, limpeza de prefix_state, ciclos e equivalência arquitetural entre executores.

### Falha de fixture corrigida

A primeira versão falhou em SCF/CCF porque o fixture tentava igualar q_previous diretamente. Isso não modelava o ciclo: no início de cada instrução, Z80::step executa q_previous = q; q = 0; no caminho indexado, execute_indexed também limpa q_previous, conforme a regra NMOS de DD/FD como instrução anterior não modificadora de flags.

A correção foi somente no fixture: parametrizar q inicial, não q_previous. O caso BASE inicia com Q=0; o caso prefixado comprova a limpeza feita pelo caminho DD/FD. Nenhum código de produção da CPU foi alterado.

No HEAD 98c1cf4, os quatro testes passaram: 4 passed/0 failed. Os 34.000 vetores SingleStepTests DD/FD também passaram explicitamente nesse mesmo HEAD. SMS Runtime e build Windows x64 passaram.

**Limite:** não é uma varredura de todos os 256 opcodes DD/FD nem de todas as combinações de estados, interrupções e WAIT. Atomic↔micro é consistência interna, não oracle independente de silício. F-028 permanece AUDIT_PARTIAL até fechar o ledger integral de substituições/exceções, HALT e prefixos.

## 5. Estado consolidado

| Item | Estado | Evidência | Lacuna |
|---|---|---|---|
| F-021 | AUDIT_PARTIAL | 390 B-loop records para OTIR/OTDR/INIR/INDR; B=0x0D+3*i e flags exatas | Outras formas I/O block, semântica completa e microsequenciador/WAIT |
| F-022 | AUDIT_PARTIAL | ROM v5.0 PASS; ≥60 comparações principais de instrução/PC | Mapeamento por opcode das oito formas LDI/LDD/CPI/CPD; casos adicionais de bloco/IRQ/Q/timing |
| F-023 | AUDIT_PARTIAL | Woodmass MEMPTR 09 PASS; checksum/ciclos/HALT/output exatos | WZ/X/Y em todas as famílias e limites temporais |
| F-028 | AUDIT_PARTIAL | 34k vetores DD/FD; PR #935; matriz #943 com 108 casos por executor; ledger 256 bytes e INT/NMI durante prefixo PASS no HEAD b3320ea | Semântica integral por opcode/estado e todas as interações de interrupção/WAIT |
| E7 | IN_PROGRESS | Evidência ampliada; nenhuma alteração de produção nesta rodada | Reconciliação de todos os critérios e integração dos candidatos |
| E8 | BLOCKED | Não iniciado | Só iniciar após fechamento formal de E7 |

## 6. Próximas ações obrigatórias

1. Atualizar cada branch candidata contra a main vigente sem descartar diffs nem reescrever runtime.
2. Revisar os diffs de #936, #942 e #943 e confirmar que só adicionam harness/testes, workflow e documentação.
3. Obter revisão independente por PR; manter Draft até a revisão.
4. Reconciliar F-022/F-023/F-028 com cada linha do checklist mestre, marcando coberto, não coberto ou NOT RUN.
5. Completar a auditoria DD/FD: tabela de todos os bytes 00–FF, classificação UsesIndex/Ignored/IndexedCB, exceções, prefixos repetidos/alternados, HALT, CB/ED e interações com IRQ/WAIT.
6. Completar F-022 nos casos de bloco não cobertos pela ROM e na observação de flags/Q em fronteiras de interrupção. Não alterar CPU sem reprodução e trace discriminante.
7. Completar mapa F-023 de WZ/MEMPTR por família, separando fonte documental, vetor externo, teste interno e evidência física ausente.
8. Após revisão e gates, decidir merges; repetir os testes necessários no SHA resultante da main.
9. Atualizar scorecard, matriz e checklist após cada integração; SKIPPED continua NOT RUN.
10. Manter E8 bloqueado.

## 7. Restrições de continuidade

- Não modificar Z80, Scheduler ou runtime para “fazer passar” teste sem trace e causa-raiz.
- Não elevar limites temporais sem demonstrar que o limite, e não a semântica/timing, é o problema.
- Não promover atomic↔micro a oracle externo.
- Não atribuir PASS ao SHA da main a partir de resultados de PR.
- Não exigir hardware físico ou universalidade CMOS/clones como gate automático do perfil NMOS definido.
- Preservar E4A/E4B/E5/E6 já validados; não reabrir sem regressão concreta.
- Não iniciar E8.

**Conclusão:** os testes focados atuais foram encerrados com PASS nos SHAs indicados. Nenhuma PR foi mesclada. E7 permanece IN_PROGRESS; E8 permanece BLOCKED.

## 8. Extensão da auditoria DD/FD após a revisão remota — 2026-11-16

### 8.1 Baseline e estado real das PRs

- Main: caa9a76a6f09d588155ea4e0cbc95f487f079143. A comparação Git com a baseline de código Z80 bf7e7584df421000d040a3ee057799a75bfeb3eb mostra dois commits exclusivamente documentais até caa9; não houve mudança de runtime Z80 nesse intervalo.
- PR #936: OPEN/DRAFT, head 306db2a91ff796b4c5f66eabc4ebd4c4baad903e, base caa9a76a6f09d588155ea4e0cbc95f487f079143, mergeable=true. CI #43852582196 e SMS Runtime #43852582229 SUCCESS no SHA exato. O harness da ROM v5.0 executa Z80::step (executor atômico) e Spectrum48Bus; não testa microsequenciador, WAIT ou Scheduler.
- PR #942: OPEN/DRAFT, head documental 15a8f1c1fe7b4b10f1526e6310b9308fdfcaf021; código/harness 7402ff138ced95da2d8b9e0008919c8cd22484f1. CI do código #43850972785/#43850972708 SUCCESS; CI documental #43851550529/#43851550520 SUCCESS. O harness Woodmass também usa Z80::step e T-states nominais, sem microsequenciador/WAIT/Scheduler.
- PR #943: OPEN/DRAFT, base caa9a76a6f09d588155ea4e0cbc95f487f079143, head atualizado pela extensão desta rodada. Nenhuma das três PRs tem revisão independente registrada ou reviewers solicitados; nenhuma foi integrada.

### 8.2 Ledger dos 256 bytes após DD/FD

Foi adicionado docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv com 256 linhas de dados, um por byte após DD/FD. A classificação efetiva da FSM é: 85 UsesIndex, 167 Ignored, 1 IndexedCB, 1 ED que cancela o index latch e 2 bytes de prefixo repetido (DD/FD).

A função auxiliar classify_index_opcode() é consultada depois de a FSM tratar DD, FD e ED; por isso, em chamada direta, esses três bytes retornam Ignored. O ledger separa o resultado auxiliar da ação real da FSM. O teste index_prefix_classification_f028.rs percorre os 256 bytes contra uma lista explícita de 85 UsesIndex e espera 85 UsesIndex, 170 Ignored e 1 IndexedCB no classificador auxiliar.

Exceções mantidas no ledger: HALT 76; EX AF,AF' 08; EXX D9; EX DE,HL EB; H/L convencionais em 66/6E/74/75; CB para DDCB/FDCB; ED para cancelamento; DD/FD repetidos com last-prefix-wins. A classificação não prova a semântica integral de cada opcode.

### 8.3 Interrupções solicitadas durante a cadeia de prefixos

O teste z80_dd_fd_exception_matrix_f028.rs recebeu dois casos: NMI e INT são solicitados após o M1 de DD e antes de FD, numa cadeia DD FD 00. O teste insere WAIT nos M1, faz snapshot encode/decode/restore e exige que NOP seja aposentado antes de a próxima fronteira aceitar NMI/INT. Esses testes exercitam o microsequenciador Z80 diretamente; não são um novo teste de integração Scheduler/VDP SMS.

### 8.4 Primeiro CI das extensões e correção

- Código candidato inicial da extensão: 958585010fb47f30333cac9494bd45d90a03801b. CI #43862912238 e SMS Runtime #43862912321 falharam somente em cargo fmt --all -- --check. Os passos de Clippy, testes, SingleStepTests e build ficaram SKIPPED em consequência do gate de formatação; não houve execução semântica nesse primeiro run.
- O log de rustfmt identificou apenas reordenação de imports, quebra de linhas e espaçamento nos dois arquivos de teste. As correções foram aplicadas em commits 0e29f164e2047bf206902de7d87b1a0c6efabe19 e dcfb74fae2d207126da59562eb8745fa0d733bd4.
- As correções de rustfmt foram aplicadas em 0e29f164e2047bf206902de7d87b1a0c6efabe19 e dcfb74fae2d207126da59562eb8745fa0d733bd4. Validação final no HEAD b3320ea79c6b951d2ad8b70e762589f14da597df: CI #43863265215 SUCCESS (Z80 focused, Rust quality/unit, documentation hygiene); SMS Runtime #43863265197 SUCCESS (Release format, Clippy, testes e build Windows x64). O teste de classificação passou 3/3; a matriz F-028 passou 6/6; os 34.000 SingleStepTests DD/FD passaram no mesmo SHA. Jobs não relacionados ficaram SKIPPED. As atualizações documentais posteriores não modificam os testes.
- A documentação patch hygiene do primeiro run passou. Não transferir esse resultado para testes Rust nem declarar PASS para o código corrigido.

### 8.5 Estado atualizado

- F-021: AUDIT_PARTIAL; 390 B-loop records da ROM v5.0 para OTIR/OTDR/INIR/INDR (seis grupos), B=0x0D+3*i e flags exatas, no SHA 306db2a; executor atômico, sem microsequenciador/WAIT/Scheduler.
- F-023: AUDIT_PARTIAL; Woodmass MEMPTR 09 PASS no SHA 7402ff1 com executor atômico e T-states nominais, sem tabela completa WZ/X/Y nem integração temporal.
- F-028: AUDIT_PARTIAL; matriz original 108 combinações PASS no SHA 98c1cf4; ledger de classificação de todos os 256 bytes e testes de INT/NMI entre DD/FD PASS no SHA b3320ea. A semântica completa por opcode/estado continua pendente.
- Nenhuma alteração de produção da CPU, prefix FSM, microsequenciador, Scheduler ou runtime foi feita.
- E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED.

## 8. Atualização de continuidade — após a auditoria de cobertura

**Main permanece em** `caa9a76a6f09d588155ea4e0cbc95f487f079143`. As alterações abaixo estão em PRs Draft e não são atribuídas à main.

| PR | HEAD observado | CI | SMS Runtime | Escopo |
|---|---|---|---|---|
| #936 | `c77759796349e02ee1f31449ce2a616b8a16d5d0` | #43873956181 SUCCESS | #43873956227 SUCCESS | ROM v5.0 e atribuição F-021/F-022; matriz temporária microsequenciada foi removida desta branch e isolada na #944 |
| #942 | `53aaf5c415eba3acc4704d1d2214a358755fad08` | #43871487677 SUCCESS | #43871487582 SUCCESS | Woodmass test 09 com assertions específicas RLD 117F e RRD 34AE, além do hash global |
| #943 | `f6254bb56d861cc3ce3da08d08ebab032841b7e7` | A consultar no HEAD atual | A consultar no HEAD atual | Documentação, plano de execução e reconciliação 85 UsesIndex/167 Ignored |
| #944 | `ef22ddc871e9790eb2fd65b1d4a70a6795c24872` | #43873337726 SUCCESS | #43873337682 SUCCESS | 64 cenários microsequenciados de repetição/IM2, incluindo Q/previous-Q até retomada |

### Evidências novas consolidadas

- **F-022:** PR #944 adiciona 64 cenários no microsequenciador: quatro opcodes × quatro padrões PC × dois BC × dois seeds Q. Verifica flags, Q/previous-Q na aposentadoria, aceitação IM2, PUSH AF/POP AF, EI/RETI e retomada. Snapshot/restore é exercitado nas fronteiras de barramento e T-state interno. O escopo é INT IM2; NMI e IM0/IM1 não foram cobertos.
- **F-023:** o output Woodmass extraído no CI #43870800175 contém RLD `117F passed` e RRD `34AE passed`. O harness exige ambas as linhas individualmente e preserva o FNV-1 `F5AE5140`, 61×32, HALT 0x80E6 e 50.904.931 T-states. Os quatro dígitos são assinaturas da suíte, não valores literais de WZ.
- **F-028:** os 85 bytes UsesIndex foram reconciliados: E3A1=9, E3A2=5, E4A=15, E4B=8, E4C=2, F-036=24, PR #680=6 e F-037=16. O gap residual está nos 167 Ignored: os testes E6 atuais são representativos e não provam fallback prefixado de todas as famílias BASE.

Nenhuma dessas alterações altera CPU de produção, MachineScheduler ou runtime. Todas as PRs permanecem Draft e sem revisão independente. E7 IN_PROGRESS; E8 BLOCKED.

## 9. Reconciliação pós-merge das PRs #936, #942, #943 e #944 — 2026-11-16

**Baseline integrada:** `main` = `fbd70c4d668902d1869d7cfbdfde5c880c9b3857`.

As quatro PRs desta rodada estão fechadas e integradas:
- #944 — merge commit `a3cc1edc0f94ca5da1850996d3bf164b0839a2e9`.
- #936 — merge commit `3b85be47d1f07b7eaa25ea65299a0608e68761d4`.
- #942 — merge commit `1a6f032861cebf3bf23867edec81ca08db97b064`.
- #943 — merge commit `fbd70c4d668902d1869d7cfbdfde5c880c9b3857`.

A #943 foi validada nos workflows CI #43880197594 e SMS Debugger Runtime #43880197552, ambos SUCCESS no HEAD `81d6f2fe955867dc16c5e80061524ea713c99c28`. O merge commit final tem exatamente a mesma árvore Git do HEAD testado (`9b13446b005b908d80607b8a258d87008db82433`); portanto, o conteúdo integrado corresponde ao conteúdo validado. Jobs não relacionados foram SKIPPED e continuam NOT RUN.

As sincronizações preservaram as alterações concorrentes nos documentos compartilhados. Não houve alteração de CPU, decoder, prefix FSM, microsequenciador de produção, MachineScheduler ou runtime nesta reconciliação.

**Status técnico não muda com o merge:** F-022, F-023 e F-028 permanecem AUDIT_PARTIAL. O merge integra evidências e harnesses, não converte cobertura parcial em fechamento semântico. F-021 continua separado de F-022: os 390 B-loop records da ROM v5.0 são evidência de OTIR/OTDR/INIR/INDR e não devem ser atribuídos às oito formas LDI/LDD/CPI/CPD.

**Estado global:** E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED. A próxima etapa é reconciliar, no checklist mestre, cada lacuna residual e sua prova independente exigida antes de implementar novos testes ou tocar no runtime.

## 10. F-028 — matriz estrutural dos 167 fallbacks Ignored integrada — 2026-11-16

**Main:** `3acf24c67d6e573120134b5b29ebbbf2a2893940`. **PR #946:** MERGED pelo commit `3acf24c67d6e573120134b5b29ebbbf2a2893940`.
**HEAD testado:** `162772ad8fba7249f8f4c7b8fb67b9816202cc80`; tree integrado/testado `77840465f65a3d6f4f522ea475e9f1d7a9f2c320`.

O teste `native/z80_cpu/tests/z80_dd_fd_ignored_fallback_matrix_f028.rs` lê o inventário efetivo do CSV de 256 bytes e executa todos os 167 bytes classificados como `Ignored / ExecuteBaseWithoutRefetch` sob DD e FD. Resultado: 334 execuções prefixadas, além das execuções BASE de referência; um teste de matriz PASS. Para cada caso, exige M1 em PC 0 e PC 1, ausência de fetch adicional, incremento de R para dois M1 e custo total igual à execução BASE + 4 T-states.

- CI #43882287467: Z80 focused validation e Rust quality/unit SUCCESS; o teste `all_167_effective_ignored_bytes_fall_back_without_refetch_for_dd_and_fd` passou 1/1.
- SMS Runtime #43882287458: Release format, Clippy, testes e build Windows x64 SUCCESS.
- O primeiro run #43882092547 falhou somente no rustfmt; a formatação foi corrigida antes dos runs aprovados.
- Jobs não relacionados ao diff permaneceram SKIPPED/NOT RUN.

**Limite da evidência:** esta matriz comprova dispatch para BASE, ausência de refetch e custo M1. Não compara integralmente efeitos arquiteturais, valores/endereço de todos os requests, WAIT/snapshot por classe, nem constitui oracle independente. A decomposição de requests por família (imediato, relativo, memória, stack, I/O e controle de fluxo) continua pendente. F-028 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

## 11. F-028 — diferencial estrutural de requests BASE/DD/FD — 2026-11-16

**Main atual:** `9c1fc19fa382a3272e3ffe04cc6beb01411319b7`. **PR #948:** MERGED por esse commit.
**HEAD testado:** `74715cbc777acc6625745489a4dfa495cde18ee9`.
**CI:** #43884118254 (Z80 focused + Rust quality/unit SUCCESS). **SMS Runtime:** #43884118298 (Release format, Clippy, testes e build Windows x64 SUCCESS). Jobs não relacionados foram SKIPPED por classificação de impacto; Windows debugger também foi SKIPPED.

A matriz amplia o teste dos 167 bytes efetivos `Ignored / ExecuteBaseWithoutRefetch` sob DD e FD (334 execuções prefixadas). Além das asserções M1 em PC 0/1, ausência de refetch, R +2 e custo BASE +4T, compara a sequência de eventos `OpcodeFetch`, `MemoryRead/Write`, `PortRead/Write` e ciclos internos, verificando tipo, endereço, duração base e duração interna após remover o M1 do prefixo e normalizar o deslocamento do stream.

**Limites que permanecem explícitos:**
- A comparação é estrutural contra o próprio caminho BASE, não um oracle semântico independente.
- O trace não registra valores lidos, payloads escritos nem o estado arquitetural final.
- O harness retorna `0x5A` para leituras e não cobre WAIT/snapshot por classe de request.
- O normalizador subtrai 1 de todo `MemoryRead` abaixo de `0x0100`, sem provar que cada leitura é fetch de operando no stream. Os registradores do fixture (BC/DE/HL/SP) e os operandos `0x5A` mantêm os acessos de dados esperados fora dessa página, mas o teste não afirma isso. Endurecer a regra registrando se o endereço coincide com PC no request e normalizando apenas fetches de stream; adicionar payloads e estado final na próxima matriz.
- O resultado não fecha F-028 nem E7.

**Auditoria de integração:** o merge commit é o HEAD atual da main e o diff desde a main anterior contém somente o teste. A consulta de status combinado do merge SHA não retornou status e a listagem disponível de Actions expõe apenas runs de PR; nenhum workflow push pós-merge foi localizado. Portanto, a validação pós-merge é **NOT VERIFIED/NOT RUN**, não PASS.

**Gate de revisão:** GitHub não registra review independente submetido para #948; o Codex review falhou por limite de uso. A PR foi integrada, mas a gate de revisão adversarial continua pendente para fechamento de E7. A mudança é exclusivamente teste, sem alteração de CPU, decoder, Scheduler ou runtime.

**Estado:** F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.

### Auditoria formal de revisão dos merges E7 — 2026-11-16

A consulta a `pull_request_reviews` e a `review_threads` retornou coleções vazias para as PRs integradas #936, #942, #943, #944, #945, #946, #947 e #948. Os comentários automáticos do Codex em #946 e #948 registram falha por limite de uso; não constituem review. Isso não prova ausência de revisão humana fora do GitHub, mas não há trilha independente auditável no repositório. Os merges permanecem integrados (todos os diffs deste conjunto são testes/documentação, sem alteração de CPU/Scheduler/runtime de produção), porém a revisão adversarial formal continua BLOCKED como gate de fechamento E7. Não converter F-022/F-023/F-028 para CLOSED nem iniciar E8 com base apenas nos checks verdes.


## 2026-11-16 — atualização após PRs #950–#952

Baseline remota atual após as integrações de teste e documentação: `bdbe950ba86a78eee2875e29f184d0b07c6eeeaa`.

- **PR #950 — F-028:** integrada em `45b40755e46aef752d58f571de856ae25ff9e81c`. A matriz acrescenta 167 opcodes Ignored × DD/FD, comparação arquitetural BASE, payloads de leitura/escrita, transações, WAIT e snapshot/restore em fronteiras de barramento/ciclo interno. CI #43888693825 e SMS Runtime #43888693845 PASS no HEAD validado. A árvore do merge foi confirmada idêntica à árvore validada.
- **PR #951 — F-022/F-023:** integrada em `b9d18f48466e3647aec73207f38c416bfef2b3dc`. Acrescenta assertions de previous-Q e WZ para LDI/LDD/CPI/CPD na matriz microsequenciada de 786.432 casos e nos ensaios dirigidos de WAIT/snapshot. CI #43890238681 e SMS Runtime #43890238056 PASS no HEAD validado. A regra é diferenciada: LDI/LDD preservam WZ; CPI/CPD atualizam WZ por ±1.
- **PR #952 — F-023:** integrada em `564074366fdc9bad59895163c8501134e88cce99`. Adiciona matriz documental WZ/MEMPTR por família, separando DOC/HW/COMP/INT/GAP. CI #43890997152: Documentation patch hygiene PASS e Rust quality and unit regression PASS; os jobs de runtime foram SKIPPED por serem alterações documentais.
- **PR #953 — F-028:** integrada em `a6cec0d744b8120c8c3e2d1d56abd1aa75fc1031`. Adiciona segundo fixture F=FF/B=1, complementa os resultados condicionais e verifica o delta exato de PC por opcode. CI #43891989147 e SMS Runtime #43891989127 PASS no HEAD validado; árvore do merge idêntica à árvore testada.
- **PR #954 — F-023:** integrada em `bdbe950ba86a78eee2875e29f184d0b07c6eeeaa`. Acrescenta WZ para D3/DB, RETN/RETI, IM0 RST e preservação em IM1. CI #43893014345 e SMS Runtime #43893014300 PASS no HEAD validado; workspace regression PASS; árvore do merge idêntica à árvore testada.
- **PR #955 — F-023:** aberta, aguardando CI. Acrescenta asserção WZ para BASE EX (SP),HL, completando a família de troca com pilha.

A matriz PR #950 é diferencial interno BASE↔DD/FD, não oracle independente de silício. O PC final exato por resultado condicional permanece pendente da PR #953. WZ é tratado na matriz F-023 e não é mascarado como equivalência universal.

**Estado:** F-028 IN_PROGRESS (matrizes 167 Ignored, WAIT/snapshot e PC condicional integradas); F-022 IN_PROGRESS; F-023 AUDIT_PARTIAL (asserção BASE EX (SP),HL pendente na PR #955); E7 IN_PROGRESS; E8 BLOCKED. Não iniciar ZEXDOC/ZEXALL.

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


## Correção de validação pós-merge — 16/11/2026

A nota anterior que classificava o workflow push da PR #955 como NOT_RUN/NOT_VERIFIED foi superada por uma consulta posterior às Actions. Os runs existem e terminaram com sucesso:

- PR #955 foi integrada em `67f4149e1933d18ffa5201da66ca6ff46c598e32`; a árvore do merge é idêntica à árvore do HEAD testado `cbc75e9205db5f1c29726962ce3d3cdd312f4ba1` (`588cf96cfc59c438932899391fb58cb556912ee2`).
- CI push pós-merge [#43898370234](https://github.com/leonardo201800478/sega-emulator/actions/runs/43898370234): COMPLETED SUCCESS.
- SMS Debugger Runtime pós-merge [#43898370419](https://github.com/leonardo201800478/sega-emulator/actions/runs/43898370419): COMPLETED SUCCESS.
- PR #956 integrou a reconciliação documental em `fd7692cb8156eea7ff6ad907fe959bc507fec3a7`; a árvore integrada é idêntica à árvore testada da PR (`c9d0183cc31b24ff19e556dc764081850fe28a53`). CI push [#43899020946](https://github.com/leonardo201800478/sega-emulator/actions/runs/43899020946): Change impact classification, Documentation patch hygiene e Rust quality/unit regression PASS. Z80 focused, workspace regression e jobs de runtime ficaram SKIPPED por classificação documental; portanto, esse run não é validação de runtime na main `fd7692`.
- PR #956 não possui review submetida nem review thread; o comentário Codex registra falha por limite de uso. PR #955 também não possui review auditável. A revisão adversarial independente permanece BLOCKED como gate transversal E7. PR #949 foi encerrada como supersedida pela #956.

**Estado técnico não muda:** F-022 e F-028 permanecem IN_PROGRESS/AUDIT_PARTIAL nos limites já descritos; F-023 permanece AUDIT_PARTIAL apesar da asserção BASE EX (SP),HL integrada; E7 permanece IN_PROGRESS e E8/ZEXDOC/ZEXALL permanece BLOCKED. Não iniciar E8 nem converter SKIPPED em PASS.