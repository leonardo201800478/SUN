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
- O log confirma pelo menos 60 comparações principais de instrução/PC e 390 registros B-loop em seis grupos (OTIR×2, OTDR×2, INIR, INDR), B observado conforme 0x0D + 3*i, flags exatas, calibração e retorno ao caller aprovados. Os seis grupos B-loop são operações de I/O block da família F-021; não os contabilizar como cobertura das oito formas de transferência/comparação de F-022. O mapeamento da stream principal para cada opcode F-022 ainda precisa ser reconciliado.
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