# Z80 compartilhado — scorecard de conclusão

**Reconciliação atual:** `main` = `bf7e7584df421000d040a3ee057799a75bfeb3eb` (2026-11-16, após PRs #929/#930/#932/#934/#935; #937/#938 não alteraram o Z80). **Baseline histórica:** `c2dae949d24b6a41899b0335ae2dd7f35b89ef26` (2026-11-09). As seções anteriores preservam evidências vinculadas aos respectivos SHAs; esta fotografia atualiza o estado sem reatribuir resultados de testes a commits não executados.
**Reconciliação:** PR #672 integrou GUI/D3D12; PR #673 adicionou teste ED; PR #674 reconciliou evidências; PR #675 adicionou o B-loop de flags; PRs #677/#678 adicionaram observação IM2, handler PUSH/POP AF, EI/RETI e retomada do bloco. Nenhuma alterou a semântica de runtime do Z80.
**Nota de reconciliação:** as PRs #615 (filtro/reset SN76489) e #617 (clarificação documental Rust/egui) não alteraram o núcleo Z80, decoder DD/FD ou suas evidências. O conteúdo técnico e as conclusões semânticas abaixo permanecem conforme as auditorias de Z80 registradas.
**Política vigente:** [ADR-0016 — fidelidade observável](architecture/adr/0016-observable-fidelity-and-proportional-timing.md).
**Escopo:** um único núcleo Z80 compartilhado pelos consoles que realmente utilizam essa CPU.

Este scorecard mede o que é necessário para executar corretamente SG-1000 e Master System. Não define uma meta de simulação elétrica, cobertura integral de pinos ou microsequenciamento de cada instrução sem dependência de plataforma.

## 1. Critérios funcionais

| Área | Critério |
|---|---|
| Instruções documentadas | Implementar corretamente as instruções que o software dos consoles suportados pode executar |
| Registradores e flags | Preservar os resultados arquiteturais e os efeitos observáveis |
| Prefixos | Tratar corretamente CB, ED, DD, FD, DDCB e FDCB nos caminhos suportados |
| Interrupções | Reproduzir os modos e as transições usados pela plataforma |
| HALT / reset | Preservar comportamento funcional e retomada correta |
| Estado compartilhado | Uma implementação Rust, sem cópias de CPU por console |
| Regressão | Alterações no Z80 não podem quebrar SMS nem SG-1000 |

## 2. Estado conhecido na baseline consultada

- O núcleo Z80 compartilhado está em `native/z80_cpu`.
- E3A1/E3A2 e E4A/E4B/E4C constam como integradas nas auditorias recentes.
- E5 DDCB/FDCB e E6 resolução geral de prefixos constam como integradas na main.
- E7.3/HALT T4 consta como integrada.
- A entrega VBlank/Line IRQ do SMS convencional permanece coberta pelos testes integrados da PR #531. A ligação da IRQ de nível do VDP à INT do Z80 temporal foi integrada pela PR #595: combina a fonte do VDP com INT externa, preserva a fonte externa no snapshot e valida o vetor de acknowledge no restore. O CI #5178 passou nos testes VBlank/IM1/HALT e de preservação da fonte externa; isso não fecha a auditoria semântica global E7.7.
- A semântica do pino RESET do Z80 está coberta por testes unitários e de integração SMS na PR #536: PC/I/R zerados, IFF1/IFF2 desabilitados, IM 0, HALT/NMI/EI delay limpos; registradores gerais/alternativos, IX/IY, SP, linha INT externa e T-states da máquina preservados.
- A duração elétrica mínima do pulso RESET não é simulada: o reset de host é uma transição atômica de estado, conforme ADR-0016; não há consumidor de gameplay demonstrado para a largura do pulso.
- O fechamento semântico global E7.7 ainda não foi comprovado como concluído. A auditoria de I/O em bloco agora possui evidência física parcial para flags no ciclo de repetição NMOS e fixture independente no Rust; a execução do ROM v5.0 completo, flags de iteração simples em todo o espaço, CMOS/clones e validação direta em hardware SMS permanecem pendentes; a observação IM2/RETI adaptada já está coberta por testes do core. `OUT (C),0` está resolvido para o perfil NMOS das placas SMS documentadas; não generalizar para todos os clones/variantes.
- O caminho de barramento/temporização está implementado apenas em fatias; não há prova de integração temporal sistêmica completa.

Esta lista é uma fotografia histórica. Antes de alterar código ou declarar qualquer critério concluído, confirmar os arquivos, testes, PRs e workflows no SHA atual da `main`. PR aberta ou Draft não equivale a implementação integrada.

## 3. Política temporal do Z80 — prioridade de gameplay e baseline atômica

A semântica de cada instrução suportada deve ser correta, independentemente do caminho de execução. A implementação atômica é suficiente como padrão para instruções cujo estado intermediário não seja observável pelo console. A precisão temporal detalhada é prioritária para sequências executadas por jogos quando a ordem/duração altera uma interação com o hardware.

| Caminho | Nível obrigatório | Critério |
|---|---|---|
| ALU/registradores internos sem acesso externo | Atômico | Resultado, flags, PC/R e duração nominal correta; sem microfases obrigatórias |
| Fetch/decode e prefixos | Atômico por instrução como base | Bytes consumidos, seleção de prefixo, PC/R e semântica exatos; detalhar M1 somente se consumidor exigir |
| Acesso a RAM/ROM sem contenção ou efeito temporal | Atômico | Endereço, dado, side effects e duração nominal |
| Escritas/leitura de I/O e registradores VDP/PSG | Temporal prioritário quando o instante afeta gameplay | Ordenação relativa à CPU/VDP/áudio e efeito no instante correto |
| Mapper/banco de cartucho | Atômico por transação, temporal se a troca interagir com execução em andamento | Estado de banco e ordem observável corretos |
| INT/NMI, EI delay, HALT, acknowledge e leitura de status | Temporal nos pontos de amostragem/aceitação observáveis | Prioridade, fronteira de instrução e efeitos colaterais corretos |
| Raster/H-counter, scroll e writes intralinha | Temporal quando jogos usam a interação | Reproduzir a menor fronteira temporal demonstrada |
| WAIT/arbitragem/DMA | Condicional | Exigir ligação real no console e efeito reproduzível |
| Pinos e fases elétricas | Fora do escopo | Não são requisito de conclusão |

**Regra de leitura do scorecard:** `IMPLEMENTED_ATOMIC` pode satisfazer integralmente o requisito funcional de uma família. `NOT_IMPLEMENTED` no microsequenciador indica apenas ausência daquele caminho temporal, não ausência da instrução no executor atômico. Só registrar uma lacuna temporal como bloqueio quando houver consumidor de gameplay e evidência que exija essa granularidade.

Uma representação diagnóstica de sinais pode ser derivada do estado do core, mas nunca deve criar uma segunda CPU ou caminho de execução.

## 4. Testes e validação

- Testes unitários por instrução/família e por estado arquitetural alterado.
- Testes de integração com os barramentos e periféricos dos consoles suportados.
- Testes de regressão SG-1000 e SMS.
- Testes diferenciais e ROMs de diagnóstico quando agregarem evidência independente.
- Testes temporais detalhados apenas para interações cuja ordem/duração altere o resultado observável.
- Desempenho Release medido quando houver mudança no caminho crítico.

ZEXDOC, ZEXALL, traces completos de pinos e validação de silício podem ser ferramentas auxiliares, não gates universais de produto. Nenhum resultado de teste deve ser declarado sem execução no SHA correspondente.

## 5. Trabalho temporal em pausa

As PRs #512, #514 e #526 permanecem abertas/Draft na consulta de 2026-11-05 e não estão integradas na main. Seus contratos de M-cycle/T-state e timeline racional não são tarefas ativas até que uma dependência concreta seja demonstrada sob a ADR-0016.

A PR #527 é uma proposta documental não integrada. Não usar os relatórios dessa PR como autoridade normativa.

## 6. Regra de atualização

Atualizar este scorecard somente quando uma mudança integrada, um teste executado ou uma evidência de hardware alterar o estado de um critério funcional relevante. Não criar fases, percentuais de completude ou bloqueios de precisão elétrica sem necessidade comprovada.

## 7. Reconciliação de evidências — 2026-11-06

Baseline histórica: `main` = `0fb09e2cad46a92aefe6a98ca07549a5ba91d78c`. As evidências específicas de cada gate permanecem vinculadas aos SHAs e workflows indicados nas auditorias.

- PR #562 integrou a correção do erratum NMOS de P/V em `LD A,I` / `LD A,R` no microsequenciador; INT imediato, instrução intermediária, NMI e snapshot/restore têm testes dedicados.
- PR #563 integrou as matrizes de regressão de I/O em bloco e dos aliases DD/FD half-register (LD, ALU e INC/DEC), além das auditorias correspondentes.
- A auditoria `docs/audits/z80-e7-block-io-flags-2026-11-05.md` caracteriza as fórmulas de flags de INI/IND/INIR/INDR/OUTI/OUTD/OTIR/OTDR para o perfil Zilog NMOS.
- CMOS/clones em geral e validação direta em hardware SMS permanecem NOT_VERIFIED; `OUT (C),0` está RESOLVED_FOR_DOCUMENTED_SMS_NMOS_PROFILE, com variantes fora desse perfil ainda não verificadas.
- E7.7 continua IN_PROGRESS e E8 continua BLOCKED.

## 8. Registro histórico após PR #595 — fotografia de 2026-11-06

A main estava em `733152f61b706b2243914ae017643cef949d4ae1`. A PR #595 alterou integração temporal SMS e testes/documentação; não alterou o decoder Z80, as famílias DD/FD ou os resultados E4C/E5/E6.

- A conexão VDP IRQ → Z80 INT temporal foi integrada, com preservação da fonte externa e validação de snapshot/acknowledge.
- E7.7 permaneceu IN_PROGRESS; E8 permaneceu BLOCKED.

## 9. Reconciliação após PR #597 — fotografia histórica de 2026-11-06

A main estava em `0fb09e2cad46a92aefe6a98ca07549a5ba91d78c`. A PR #597 alterou a fronteira do worker de áudio SMS, não o núcleo Z80. E7.7 permaneceu IN_PROGRESS; E8 permaneceu BLOCKED.

## 10. Baseline atual — 2026-11-07

A main atual consultada é `3ad1781f84b7824b5aa889a583ae6d4545987ec1`. As PRs #615 e #617 não alteraram a implementação ou a evidência do Z80. As PRs #613/#614 ainda não foram integradas na fotografia desta baseline.

- E4C, E5 e E6 permanecem integradas conforme evidências anteriores.
- E7.7 permanece IN_PROGRESS.
- E8 permanece BLOCKED; não iniciar ZEXDOC/ZEXALL antes do fechamento E7.
- A auditoria semântica deve continuar com lacunas específicas, incluindo `OUT (C),0` variante-dependente, sem exigir microsequenciamento universal.


## 11. Reconciliação documental — baseline `main` de 2026-11-07

**Baseline auditada:** `e77eff26d80fd77a64546373c3e92326a0cc4e7a` (merge da PR #628).

Esta seção supersede as referências anteriores de baseline como fotografia corrente; as seções históricas permanecem preservadas para rastreabilidade.

- PR #613 (`ec08703d8e07ed7f91283e7db0787d2457273853`) integrou o runtime SMS temporal/vídeo à aplicação e alterou `native/sega_app`, `native/video_runtime` e `rust/runtime/sms-runtime/src/worker.rs`. Não alterou `native/z80_cpu`, o decoder DD/FD ou as fórmulas de flags do Z80.
- PR #614 (`63eb154acedb993cdaf944746cb1f46cc8ccb3c6`) integrou áudio e controles na aplicação; não alterou o núcleo Z80.
- PR #628 (`e77eff26d80fd77a64546373c3e92326a0cc4e7a`) alterou o workflow CI e documentação de validação; não alterou runtime Z80, decoder DD/FD, microsequenciador ou evidências semânticas.
- As conclusões de E4C, E5 e E6 permanecem as registradas nas auditorias anteriores. Esta reconciliação não reexecuta nem promove seus testes.
- **E7.7 permanece IN_PROGRESS.** A auditoria semântica global não está encerrada; continuam pendentes comprovação para variantes CMOS/clones e validação direta em hardware SMS; `OUT (C),0` está resolvido apenas para o perfil NMOS das placas SMS documentadas.
- **E8 permanece BLOCKED.** Não iniciar ZEXDOC/ZEXALL antes do fechamento documentado de E7.
- A integração de aplicação/runtime SMS não comprova por si só a precisão temporal sistêmica do Z80. Não declarar TEMPORAL_PASS global nem equivalência contínua/em blocos sem testes específicos.

Nenhuma alteração de código Z80 decorre desta reconciliação documental.


## 12. Baseline remota e fechamento da subquestão OUT (C),0 — 2026-11-07

**Baseline atual auditada:** `main` = `43323df653497a98c7afae52a6f3ea198f46cdb0`.

- PR #632 integrou esta reconciliação documental de scorecard/matriz no merge SHA `415861b7c550512e0380ac85b273748de796b5f3`.
- PR #634 integrou o registro das validações de CI no merge SHA `43323df653497a98c7afae52a6f3ea198f46cdb0`. Nenhuma das duas PRs alterou o runtime Z80.
- CI pós-merge da PR #634: run [42379523450](https://github.com/leonardo201800478/sega-emulator/actions/runs/42379523450), SUCCESS no SHA exato desta baseline. O job de Z80 focused e Scheduler stress foi SKIPPED por classificação de impacto; não é validação semântica do Z80.
- A investigação E7.7 específica de `ED 71` (`OUT (C),0`) está registrada em [z80-e7-77-out-c-zero-variant-2026-11-07.md](audits/z80-e7-77-out-c-zero-variant-2026-11-07.md).
- Para as placas SMS documentadas e examinadas, as peças listadas são Zilog Z0840004PSC e, em uma revisão inicial PAL, NEC D780C-1 ou Zilog Z0840004PSC. A evidência do fabricante classifica Z08400 como NMOS; a família Z84C00 é CMOS. A referência experimental/comparativa atribui saída zero ao ED 71 no Z80 NMOS e FF no Zilog CMOS, com exceções entre clones.
- O runtime integrado escreve `0x00` para ED 71 e tem teste de I/O que confirma a transação `(BC, 0x00)`. Esse comportamento corresponde ao perfil NMOS das placas SMS documentadas. Não foi necessária alteração de runtime.
- **Limite:** o levantamento de placas não é um censo de todas as unidades/revisões produzidas. Não inferir que todos os clones CMOS ou todas as variantes Z80 tenham comportamento idêntico. A exceção Sharp LH5080A demonstra que uma regra binária “CMOS = FF” não é universal.
- **Estado E7.7:** a subquestão ED 71 fica RESOLVED_FOR_DOCUMENTED_SMS_NMOS_PROFILE, com evidência documental de componentes e regressão de runtime. A auditoria semântica integral E7.7 continua IN_PROGRESS; CMOS/clones em geral e outras famílias/efeitos ainda exigem reconciliação. **E8 permanece BLOCKED.**


## 13. Reconciliação da main — 2026-11-09

**Baseline auditada:** `main` = `ca1b26089bd649a9248fab4ecbe47b78bc2eb49d`.

- PR #672 integrou a composição dos menus egui sobre o viewport D3D12; sem alterações no Z80.
- PR #673 integrou `ed_absolute_word_transfers_wrap_address_and_wz_at_ffff`, cobrindo ED 43/53/63/73 e ED 4B/5B/6B/7B no limite `nn=0xFFFF`: wrap FFFF→0000, ordem little-endian, WZ=0000, PC, F preservada e 20 T-states. CI #42568163213 passou no HEAD exato `b6f94ff010782a5dd18e6c684e625c2eec9d7a6e`; merge SHA `ca1b26089bd649a9248fab4ecbe47b78bc2eb49d`.
- A auditoria de I/O em bloco agora referencia o teste independente `Z80 Block Flags Test v5.0`, de Peter Helcmanovsky. O projeto de teste documenta validação em hardware Z80 NMOS e o wiki registra resultados em diversos chips. Seu escopo é específico: flags de instruções de bloco quando a repetição é interrompida por IM2, incluindo PF/HF para INxR/OTxR. O próprio autor ressalta que o teste é limitado e não exaustivo.
- Essa evidência reduz a lacuna de proveniência para a subquestão de flags da repetição em hardware NMOS; não constitui validação direta no Master System, nem prova exaustiva das flags de uma iteração simples, de todos os valores/estados ou de clones CMOS.
- Os testes atuais `block_io_repeat_flags_cover_half_carry_and_parity_edges` e `block_io_normal_flags_ports_and_wz_match_documented_edge_vectors` são regressões dirigidas. A comparação atomic↔microsequencer não é um oráculo independente, pois os caminhos podem compartilhar a mesma hipótese semântica.
- **E7.7 permanece IN_PROGRESS.** A auditoria semântica global ainda exige reconciliação de todas as famílias e efeitos observáveis, além de execução independente dos testes ROM/hardware pertinentes.
- **E8 permanece BLOCKED.** Não iniciar ZEXDOC/ZEXALL antes do fechamento formal de E7.
- E4C, E5 e E6 não foram alteradas nem reabertas por estas PRs.

Auditoria complementar: [E7.7 — evidência de hardware para flags de I/O em bloco](audits/z80-e7-block-io-hardware-evidence-2026-11-09.md).


## 14. Reconciliação após PR #675 — 2026-11-09

**Baseline atual:** `main` = `1e14d32d5871326cc7f904cef7c5ad4feb1cd86c`.

- PR #506 (merge SHA `54f881872ec9f5a9923ebdb17643b37a19954ec5`) integrou E7.2E; o estado histórico QUEUED da matriz foi supersedido. Os checks aplicáveis no merge SHA passaram: Rust quality/unit regression, Windows x64 D3D12 compile e Python lint/regression.
- PR #675 integrou `block_io_repeat_flags_match_hardware_banks_formula_across_b_loop`. O teste cobre B inicial de `0x10` a `0xD0` em passo 3 (65 valores), quatro posições PC que cobrem XY derivados de PC.13/PC.11 e seis casos INIR/INDR/OTIR/OTDR. O esperado é calculado pela álgebra publicada com o ROM Z80 Block Flags Test v5.0, incluindo L pós-atualização para OUTI/OUTD.
- O teste compara executor atômico e microsequenciador com a fórmula independente e verifica flags, B, PC, WZ, 21 T-states e transações de I/O. CI #42568885363 passou no HEAD exato `33d3dcf8d41f2f18d0826a1401013e9e1582503e`; merge SHA `1e14d32d5871326cc7f904cef7c5ad4feb1cd86c`.
- O teste reproduz a fórmula/domínio do ROM; não executa o ROM v5.0 nem constitui uma nova execução em hardware. A evidência física publicada continua limitada ao escopo NMOS e aos casos descritos na auditoria complementar.
- `OUT (C),0` permanece RESOLVED_FOR_DOCUMENTED_SMS_NMOS_PROFILE. CMOS/clones fora do levantamento e SMS físico permanecem NOT_VERIFIED/NOT_RUN.
- E7.7 permanece IN_PROGRESS por outras famílias e efeitos semânticos ainda não reconciliados. E8 permanece BLOCKED; não iniciar ZEXDOC/ZEXALL.


## 15. Reconciliação IM2/RETI — 2026-11-09

**Baseline atual:** `main` = `043ecb61d8d1cc77245fa72639d7f210d88cab26`.

- PR #677 integrou o cenário IM2 que assertiva INT no último T-state de `EdBlockIoRepeatInternal`, atende o vetor `I:FF`, executa `PUSH AF` e confirma F observável no handler.
- PR #678 estendeu o handler para `PUSH AF; POP AF; EI; RETI` e executou a iteração de bloco seguinte, confirmando retorno ao PC ED, restauração de SP, IFF1/IFF2, EI delay, B, PC e WZ.
- A cobertura contempla INIR, INDR, OTIR e OTDR e compara executor atômico e microsequenciador. Tempos verificados: 21 T da iteração repetitiva, 19 T da entrada IM2, 11 T PUSH AF, 10 T POP AF, 4 T EI, 14 T RETI e 21 T da iteração retomada.
- CI #42569668886 (PR #677) e #42569931640 (PR #678) passaram nos HEADs exatos; merge SHA atual `043ecb61d8d1cc77245fa72639d7f210d88cab26`.
- O teste respeita a semântica level-sensitive de `set_int_line`: o dispositivo desasserta INT após o serviço. A API one-shot `request_int` do executor atômico é limpa após o acknowledge para comparação.
- Isso conclui o fixture CPU-only de observação e retomada. O ROM v5.0 completo ainda não foi executado no core, e não houve validação física no Master System.
- E7.7 permanece IN_PROGRESS por outras famílias/efeitos e validações pendentes. E8 permanece BLOCKED.


## 16. Checklist mestre de auditoria integral — 2026-11-09

**Baseline de código reconciliada:** `main` = `b41fa66e01e329d45f7704a229479c8ac3c2c9ea`.

Foi criado o [Checklist mestre de auditoria e fechamento do Z80](z80-master-audit-checklist.md). Ele passa a ser o ledger operacional da auditoria: inventário de instruções e prefixos, estado arquitetural/flags, barramento/temporização, interrupções, snapshots/determinismo, referências, testes e estrutura do código. Cada fechamento deve ser registrado com IDs, evidências, testes classificados e SHA integrado na main.

As baselines antigas mantidas nas seções anteriores são fotografias históricas e não devem ser reescritas como se tivessem sido auditadas no SHA atual. A partir desta seção, novas atualizações devem referenciar o SHA efetivamente consultado.

**Estado:** auditoria global IN_PROGRESS; PR #680 permanece aberta e bloqueada por CI FAIL; E8/ZEXDOC/ZEXALL permanece BLOCKED.


## 17. Reabertura do inventário operacional — snapshot documental após PR #681

**Baseline da main consultada para esta varredura:** `db6b4ec6d2f01ddaabccc124fe9ca7d95a856320` (PR #681, checklist mestre, merge concluído).

- O checklist `docs/z80-master-audit-checklist.md` passa a ser o registro de cobertura e evidências por ID.
- A primeira varredura estática DD/FD confirmou que o executor atômico cobre famílias IXH/IXL/IYH/IYL que ainda não têm dispatch equivalente no microsequenciador: LD r,r' register-only e ALU A,IXH/IXL/IYH/IYL.
- A PR #680 permanece em andamento para INC/DEC e LD imediato das metades indexadas. A falha de snapshot foi corrigida no HEAD `48aad1f...`; CI #5924 passou nos gates focados. O HEAD atual da PR é `ed113041...`, com CI #5925 executando regressão integral; fmt, Clippy workspace e testes workspace já passaram, mas validações Windows ainda estão em execução.
- Não declarar F-028/F-036/F-037 fechados nem liberar E8. O fechamento exige merge e atualização do ledger no SHA resultante.

**Estado:** E7.7 IN_PROGRESS; E8 BLOCKED.


## 18. Reconciliação após PR #680/#683 e candidato F-036 — 2026-11-09

**Main remota confirmada:** `43772d5cd5395de273bd3898895cf1deed81aa53`.

### Integrações concluídas

- **PR #680:** squash merge confirmado em `04527a59e0695a800d4fd048d2da85c51126de68`. HEAD validado `e20ec4b6c858bcaca265114ba543c19301dbb2e0`; CI #42609912469 PASS. INC/DEC IXH/IXL/IYH/IYL e LD imediato das metades indexadas estão no microsequenciador. WAIT/snapshot dos safe points cobertos pela PR foram validados.
- **PR #683:** squash merge confirmado em `43772d5cd5395de273bd3898895cf1deed81aa53`. HEAD validado `eaa88a249989b2ca6aee931e2a963df75a396008`; CI #42613123290 PASS. A seleção por dependências, jobs focados Z80/PSG/VDP/input, validações de hosts e agregador foram integrados. A PR #637 foi encerrada como supersedida.

### Estado atual do fechamento E7

| Família | Estado | Evidência / pendência |
|---|---|---|
| F-028 — IXH/IXL/IYH/IYL | AUDIT_PARTIAL | INC/DEC e LD imediato #680; F-036 LD r,r' #684 CLOSED; F-037 ALU #686 CLOSED. Auditoria residual de precedência, exceções e efeitos temporais ainda pendente. |
| F-036 — LD r,r' register-only | CLOSED | PR #684 integrada em `0b9b298f0f9791a6fa6db17580ad741db5c0515d`; HEAD `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3`, run #42614755935 PASS. 48 combinações DD/FD, oracle, comparação atômica, 8T/2 M1, WAIT, snapshot/restore, prefixos repetidos e HALT. D3D12/áudio/SDL3/workspace SKIPPED e agregador PASS. |
| F-037 — ALU A,IXH/IXL/IYH/IYL | CLOSED | PR #686 merge SHA `e7e37290e1baa9119326e9e7213f77cadcc37f5e`; HEAD validado `a0937e8fa1bb4547d1ee53a9e3431392a2fa443a`; run #42619522899 PASS. 10.816 casos diferenciais, vetores fixos, carry-in, flags/X/Y/CP, Q, WAIT, snapshot/restore, prefixos repetidos. Corpus externo SingleStepTests ainda pendente como evidência complementar E7. |
| E7.7 — auditoria semântica integral | IN_PROGRESS | Outras famílias, efeitos observáveis, proveniência independente e auditoria temporal ainda pendentes. |
| E8 — ZEXDOC/ZEXALL | BLOCKED | Não iniciar antes do fechamento formal de E7. |

**Regra de promoção:** F-036 foi promovida a CLOSED somente após CI verde no HEAD final da PR #684 e merge confirmado. Os jobs D3D12/áudio/SDL3/workspace foram SKIPPED, classificados internamente como NOT RUN, e o GitHub aceitou o merge; comportamento comprovado em PR real.

**Próxima tarefa objetiva:** auditoria residual do Z80 com vetores externos, revisão de flags/Q/WZ, prefix FSM, interrupções e microtiming. Não avançar para E8.


## 19. Fechamento F-036 — evidência final

- PR #684: merged.
- HEAD validado: `03dbd1552c8eb58ff8f9ef28d9ab0e97b615b4f3`.
- CI final: #42614755935 — PASS no HEAD exato. Z80 focused, documentation hygiene e agregador PASS; jobs não aplicáveis SKIPPED.
- Falhas intermediárias registradas: #42614347670 (rustfmt) e #42614617507 (Clippy field_reassign_with_default). Corrigidas antes do HEAD final.
- Merge SHA e main atual: `0b9b298f0f9791a6fa6db17580ad741db5c0515d`.
- F-036 CLOSED; F-037 GAP_CONFIRMED; F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

**Próxima tarefa:** F-037 — ALU A,IXH/IXL/IYH/IYL — com vetores independentes para ADD/ADC/SUB/SBC/AND/XOR/OR/CP, carry de entrada, flags completas, Q, caminho retomável, WAIT e snapshot/restore.


## 20. Candidato F-037 — ALU com metades IX/IY — 2026-11-09

**Main remota de entrada:** `5aaa74af3fbf6fa7f70ecb1e6b71f3a966bb91e4`.

- **PR:** #686 — `feat/z80-e7-indexed-half-alu-20261109`.
- **Commit de implementação:** `5f9ca825feb45633c2baf200e502f1a05bf99c7d`.
- **Último commit de teste/otimização antes desta atualização documental:** `2ef8d4c0c73cf5a339f8d258ba9e314bfeebcdd2`. O HEAD final da PR será consultado no GitHub antes do merge.
- **Estado:** IMPLEMENTED_PENDING_VALIDATION. O microsequenciador agora tem allowlist e dispatch para as 16 combinações DD/FD de ADD, ADC, SUB, SBC, AND, XOR, OR e CP com IXH/IXL/IYH/IYL.
- A implementação reutiliza `read_indexed_register()` e `execute_alu_operation()`; não duplica a ALU nem cria ciclos internos artificiais.
- A suíte `indexed_half_register_alu_e7.rs` contém 10.816 casos diferenciais (8 operações, 2 fontes, DD/FD, 13 acumuladores, 13 operandos e 2 carry-in), 16 vetores fixos por prefixo/fonte, WAIT nos dois M1, snapshot/restore, prefixos repetidos e verificações de A/F/Q/previous-Q/WZ/PC/R/IFF/I/IX/IY/H/L.
- CP mantém A e valida X/Y derivados do operando. As formas com fonte código 6 continuam no caminho E4B de memória indexada.
- F-036 permanece CLOSED. F-037 aguarda CI/merge. F-028 permanece AUDIT_PARTIAL. E7 IN_PROGRESS; E8 BLOCKED.

**Próximo gate:** validar o HEAD final da PR #686 com a suíte focada Z80 e os consumidores SMS/SG-1000; comprovar novamente que hosts não relacionados são SKIPPED. Só depois integrar F-037 e prosseguir à auditoria mestre residual.


## 21. Fechamento F-037 — evidência final

- PR #686 squash-merged em `e7e37290e1baa9119326e9e7213f77cadcc37f5e`.
- HEAD validado: `a0937e8fa1bb4547d1ee53a9e3431392a2fa443a`; run #42619522899 PASS.
- Rust fmt focado, Clippy focado, testes Z80 + SMS + SG-1000, documentation hygiene e agregador: PASS.
- D3D12, WASAPI/XAudio2, Linux audio, SDL3, VDP, PSG, Scheduler stress, PowerShell, input e workspace: SKIPPED / NOT RUN por não aplicabilidade.
- Run intermediário #42615731297 FAIL em rustfmt foi corrigido antes do HEAD final.
- F-036 CLOSED; F-037 CLOSED; F-028 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.
- Pendência de evidência: integrar vetores externos (SingleStepTests Z80, variante NMOS e licença MIT) à suíte diferencial, sem tratar o executor atômico como oracle independente.


## 22. Validação externa SingleStepTests F-037 — 2026-11-09

**Main após integração:** `1304bc0438e3ea333719d0ba3a495e2db9089da3`
**PR:** #692 — merge squash
**HEAD da PR validado:** `1606556814a8746a445d3ad32fc948ab97f0fde5`
**CI:** run `4262282817` — PASS nos checks aplicáveis
**Upstream:** SingleStepTests/z80, revisão `ebe1875d48f374bcfd4b505d8eb8ee751568b5f7`, licença MIT.

- 32 fixtures: DD/FD × 16 segundos bytes.
- 1.000 vetores por fixture; **32.000/32.000 PASS**, sem amostragem.
- Executados no microsequenciador retomável em `native/z80_cpu/tests/singlestep_indexed_alu.rs`.
- Comparados registradores principais/alternativos, F, Q/P, PC, R, WZ, IFF, IM, EI, IX/IY e RAM.
- Duas requisições lógicas OpcodeFetch e 8 T-states agregados conferidos.
- Os traces de barramento não foram tratados como equivalência elétrica de pinos.

**Limite de proveniência:** o README upstream informa que os vetores foram traduzidos do núcleo Z80 do Ares e corrigidos. O corpus é externo ao repositório, mas não é independente do Ares nem constitui prova direta de silício.

**Estado após integração:** F-036 CLOSED; F-037 CLOSED com validação externa complementar; F-028 AUDIT_PARTIAL; F-033 CLOSED para o perfil NMOS clássico delimitado após PR #706; F-034 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

## 23. F-028 — reprodução da divergência Q/previous-Q — 2026-11-09

- Reprodução: PR #695, workflow run #42624833386.
- Fixture DD 37 0001: esperado F=0x65; obtido F=0x45. **FAIL reproduzido** no microsequenciador retomável.
- A execução interrompeu no primeiro mismatch; FD 3F está presente no corpus, mas não foi alcançado.
- O problema decorre da preservação de q_previous através de DD/FD antes de SCF/CCF no perfil Zilog NMOS.
- PR de correção Q/previous-Q limpa q_previous nos caminhos atômico e retomável, atualiza o teste interno e valida snapshot/restore após o prefixo. CI ainda pendente nesta baseline documental.
- F-028 permanece AUDIT_PARTIAL até CI PASS e auditoria das demais exceções/variantes. E7 IN_PROGRESS; E8 BLOCKED.


## 24. F-028 — fechamento da divergência Q/previous-Q em DD/FD — 2026-11-09

**Main após merge:** `401bf4fb1ba8add6c84420cad5c9d2a62ceba72b`
**PR:** #702 — MERGED (squash)
**HEAD exato validado:** `34e85637eaac8ed2777ecc5cb9cd2e91ba137235`
**Workflow runs:** #42626811588 e #42626765522 — CI SUCCESS.

- Executor atômico: limpa `q_previous` ao entrar em `execute_indexed`.
- Microsequenciador: limpa `q_previous` ao transitar/continuar no estado Index; cobre prefixos DD/FD únicos, repetidos e alternados.
- Testes F-036/F-037 foram reconciliados para a semântica de prefixo; E4A/E4B não foram alterados.
- Teste diferencial interno DD 37 / FD 3F verifica F, Q/previous-Q, 8T e snapshot/restore entre prefixo e opcode terminal.
- SingleStepTests/Z80: 32.000 vetores F-037 + 2.000 vetores DD 37/FD 3F = **34.000/34.000 PASS**, executados no microsequenciador retomável.
- O log do job Z80 focused validation confirma os dois testes externos executados e aprovados; o harness compara estado arquitetural e T-states agregados, não equivalência elétrica de pinos.
- rustfmt, Clippy, testes Z80, integração SMS/SG-1000, workspace regression e documentação: PASS nos runs registrados.

**Classificação:** a divergência específica Q/previous-Q de DD/FD→SCF/CCF está RESOLVED. F-028 permanece AUDIT_PARTIAL porque a auditoria integral de todas as exceções DD/FD, prefixos, HALT, flags e efeitos arquiteturais ainda não foi concluída. F-036 CLOSED; F-037 CLOSED; F-033 GAP_CONFIRMED; F-034 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.

**Pendências E7 que não são resolvidas por esta PR:** microsequenciador para os 178 bytes ED reservados (F-033), classificação/validação dos aliases e encodings não documentados (F-034), auditoria global das famílias F-001..F-034, interrupções/WAIT/barramento e determinismo sistêmico.


## 25. Candidata F-033 — aposentadoria ED reservada no microsequenciador — 2026-11-09

**Main de entrada:** `9d5d83d0df16a1c6fa9fb82bdd953af54369c613`
**PR:** #706 — `fix/z80-f033-ed-reserved-resume-20261109`
**HEAD candidato:** `cab1603e9699e44b5f2edd418186f63410cad65a`
**Estado histórico:** IMPLEMENTED_PENDING_VALIDATION na abertura da PR; a implementação foi posteriormente integrada.

- O fallback residual de `execute_ed_register_control()` agora aposenta a instrução após os dois fetches M1 já consumidos, sem ciclos internos artificiais nem transações extras de memória/I/O.
- A suíte `native/z80_cpu/tests/ed_reserved_f033.rs` percorre os 178 bytes fora das 78 codificações explícitas e verifica 8 T-states, PC com wrap, dois refresh increments, F/Q/previous-Q/WZ/P/IFF/IM, fase Ready e ausência de request pendente.
- A suíte também cobre WAIT aditivo e snapshot/restore antes e depois do byte terminal ED.
- Regressões antigas no sequenciador e no SMS temporal foram atualizadas de `UnsupportedOpcode` para aposentadoria determinística.
- A hipótese de 8T/no-effect é delimitada ao perfil Zilog NMOS clássico. UM0080 não especifica universalmente todos os bytes reservados; MAME e fontes secundárias são evidência de implementação/referência, não prova de silício. CMOS, R800, Z80N e clones permanecem fora da afirmação.
- CI #42631919433 no HEAD candidato: Z80 focused PASS; Rust quality and unit regression PASS; Windows x64 D3D12 PASS. Rust workspace regression, documentação e outros jobs aparecem SKIPPED/NOT RUN por não aplicabilidade; a política de CI determina pacote focado para mudanças exclusivamente Z80. Nenhum resultado de outro SHA é transferido.
- A revisão automática Codex da PR não concluiu por limite de uso; não há review threads inline abertas nem reviews submetidas na consulta. Isso não equivale a aprovação de revisão.

**Classificação após merge:** F-033 CLOSED exclusivamente para o perfil de compatibilidade Zilog NMOS clássico descrito nesta seção. F-034 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## 26. Reconciliação pós-merge da PR #706 — 2026-11-09

**Main:** `2d15a2311cda9d087bbdd6c05f6c3851d52f4150`
**PR integrada:** #706, HEAD validado `b2291a83848e236d2354d8f03e22b831f9030b72`.
**CI no HEAD exato:** run #42635309051 SUCCESS. Z80 focused validation, Rust quality/unit regression e Windows x64 D3D12 PASS. Jobs não aplicáveis foram SKIPPED/NOT RUN pela política de impacto; não são classificados como PASS.

- O microsequenciador aposenta os 178 bytes ED reservados após os dois M1 já consumidos, sem ciclos internos ou transações extras.
- `native/z80_cpu/tests/ed_reserved_f033.rs` cobre todos os 178 bytes, estado arquitetural discriminante, WAIT e snapshot/restore.
- A implementação é uma política de compatibilidade para Zilog NMOS clássico. UM0080 não especifica universalmente esses encodings; MAME e referências secundárias não são prova de silício. CMOS, R800, Z80N e clones permanecem fora da afirmação.
- O fallback do executor atômico permanece distinto e não é usado como substituto da evidência temporal do microsequenciador.

**Classificação atualizada:** F-033 CLOSED com limite de evidência; F-034 AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED. E8 não pode começar antes do fechamento formal de E7.


## 27. F-034 — fechamento da auditoria ED para o perfil SMS/SG-1000 NMOS — 2026-11-09

**PR #708:** merged; squash SHA `d364b901e2547f44d53faf7ca107151e551d2c2e`. CI #6025 SUCCESS no HEAD exato `42284d251d21645ba73a28ef4923cf934c068f59`. NEG: todos os oito aliases × 256 valores de A com fórmula independente de resultado/flags no executor atômico; microsequenciador diferencial para todos os valores e ambos os estados de IFF2.

**Cobertura complementar integrada:** ED70 tem vetores para todos os 256 bytes de entrada no executor atômico e no microsequenciador; verificam leitura em BC, flags S/Z/H/PV/N, preservação de Carry, descarte do dado, WZ, snapshot/restore da requisição e WAIT aditivo. ED71 tem asserção explícita de escrita zero no endereço BC e comparação dos traces nos dois executores. RETN/RETI: semântica de retorno/restore IFF é testada; notificação RETI daisy-chain é NOT_APPLICABLE aos perfis SMS/SG-1000 sem dispositivo daisy-chain. Modos IM: os oito encodings/aliases (IM0: ED46/4E/66/6E; IM1: ED56/76; IM2: ED5E/7E) agora têm asserções independentes do modo esperado nos dois executores.

**Limites:** nenhum teste em hardware físico foi executado; não se reivindica equivalência de silício, CMOS, R800, Z80N ou clones. MAME é comparador de implementação, não autoridade de silício. A classificação só pode ser CLOSED para o perfil de compatibilidade NMOS clássico adotado pelo projeto e para os sistemas-alvo declarados.


## 28. Fechamento F-034 após CI e merge da PR #709 — 2026-11-09

**Main:** `5dd98ddcadbb9a1d0316c41044e0dd3767a6fab8` (PR #709, squash). **HEAD validado da PR:** `b6e5db79047e7802620e76ad98f5cfd08deefd57`. **CI #6034 / run `42645765052`: SUCCESS no HEAD exato.**

- Documentation patch hygiene: PASS.
- Z80 focused validation: PASS — fmt, Clippy, testes Z80/SMS/SG-1000 e vetores externos DD/FD.
- Rust quality and unit regression aggregate: PASS.
- Jobs de workspace regression e subsistemas não relacionados: SKIPPED pela política de impacto; classificados NOT RUN, não PASS.
- PR #709 merged; sem review threads inline ou reviews submetidas na consulta final.
- ED70: 256/256 valores de entrada no executor atômico e microsequenciador; flags documentadas, Carry, destino descartado, WZ, WAIT e snapshot/restore da operação pendente.
- ED71: OUT(C),0 preservado apenas como comportamento de compatibilidade NMOS observado; não documentado por UM0080 nem generalizado a variantes.
- NEG: oito aliases × 256 valores com oracle algébrico independente no executor atômico e comparação microsequenciada.
- RETN/RETI e IM: aliases e modos aceitos pelo perfil atual cobertos; sinalização RETI daisy-chain NOT_APPLICABLE aos perfis SMS/SG-1000 sem consumidor.

**Classificação vigente:** F-034 CLOSED exclusivamente para o perfil de compatibilidade Zilog NMOS clássico adotado pelo projeto nos alvos SMS/SG-1000. Não há alegação de equivalência de silício, compatibilidade CMOS/R800/Z80N/clones ou validação física. F-033 permanece CLOSED com limite de evidência; E7 permanece IN_PROGRESS por auditoria global e outras famílias/efeitos ainda pendentes; E8 permanece BLOCKED até o fechamento formal de E7.


## 29. Reconciliação da main após PR #710 — 2026-11-09

**Baseline:** `1142eb567746afe64356b60f2ce466581e1c0cef`.

- PR #709 integrada no merge SHA `5dd98ddcadbb9a1d0316c41044e0dd3767a6fab8`; HEAD validado `b6e5db79047e7802620e76ad98f5cfd08deefd57`; CI #6034 SUCCESS.
- PR #710 integrada no SHA atual acima; CI #6036 de higiene documental SUCCESS no HEAD da PR #710. CI pós-merge #6037 SUCCESS no HEAD atual, mas executou somente classificação de impacto e higiene documental.
- F-033 CLOSED para o perfil de compatibilidade Zilog NMOS clássico, com a hipótese dos 178 encodings reservados delimitada; não é especificação universal UM0080 nem afirmação para CMOS/R800/Z80N/clones.
- F-034 CLOSED para o perfil de compatibilidade Zilog NMOS clássico SMS/SG-1000: ED70, ED71, aliases NEG, RETN/RETI e modos IM nos escopos e limites registrados na seção 28 e na matriz ED.
- E7 permanece IN_PROGRESS. E8/ZEXDOC/ZEXALL permanece BLOCKED.
- E7.7 continua exigindo reconciliação do inventário global F-001..F-034, flags e efeitos observáveis de outras famílias, interrupções, prefixos e determinismo. A execução do ROM v5.0 completo e validação física SMS não são declaradas PASS.
- Não transferir os resultados de CI #6034/#6036/#6037 para suites que não foram executadas; jobs SKIPPED permanecem NOT RUN.


## 30. F-022 — flags de bloco observáveis durante interrupção IM2 (candidata PR #712)

**Baseline main:** `fa069162306fb330864b6d77633ceba0f976e3ca`.

A candidata adiciona `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` em `native/z80_cpu/src/sequencer.rs`. Para LDIR/LDDR/CPIR/CPDR, o teste injeta INT no último T-state do ciclo interno de repetição, atende IM2, observa F por `PUSH AF`, retorna com `POP AF; EI; RETI` e verifica a retomada da instrução ED. O snapshot encode/decode ocorre em cada fronteira de barramento/ciclo interno do microsequenciador.

**Frente documental:** UM0080 especifica o decremento de BC e as flags documentadas das operações de bloco; não especifica X/Y no caso interrompido. O teste público Z80 Block Flags Test v5.0 documenta que, quando uma instrução de bloco repetitiva já executou uma iteração e a interrupção é aceita em vez da repetição, YF corresponde a PC.13 e XF a PC.11. O próprio autor classifica o teste como limitado e alerta para diferenças em clones/CMOS.

**Frente de engenharia reversa:** MAME (`src/devices/cpu/z80/z80.lst`, macros `ldir`, `lddr`, `cpir`, `cpdr`) e ares (`ares/component/processor/z80/instructions.cpp`, `instructionLDIR/LDDR/CPIR/CPDR`) também substituem X/Y pelos bits correspondentes do PC após recuo de dois bytes; ambos ajustam WZ/MEMPTR para PC+1. redcode/Z80 fornece macros equivalentes para LDXR/CPXR. Foram consultadas apenas como referências de comportamento; nenhum código foi copiado.

**Oráculo e limites:** os valores esperados de F no handler derivam do comportamento descrito pelo teste v5.0 e das flags documentadas no UM0080. A comparação atômico↔microsequenciador é somente diferencial interna. O WZ=PC+1 é confrontado com MAME/ares, não alegado como observação física independente. O teste cobre um vetor dirigido por opcode, não uma varredura combinatória. F-022 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED. Hardware SMS, CMOS e clones não foram validados.

**Validação:** aguardando CI da PR #712 no HEAD exato. Nenhum resultado PASS é atribuído antes da execução.


## 31. Integração da evidência F-022 — PR #712

**Main:** `5e4a703daefdebe62fb7f8d220752359df59f8e3` (squash merge da PR #712; merge SHA idêntico ao HEAD).
**HEAD da PR validado:** `7390744d9d6dc61ef2db3786f75fc9073dabdc12`.
**CI:** #6045 / run `42649308679` PASS no HEAD exato; CI pós-merge #6046 / run `42649608469` PASS no HEAD da main.

- `ed_block_transfer_repeat_interrupt_tests::block_transfer_and_compare_repeat_flags_are_observable_through_im2_and_resume` executa LDIR, LDDR, CPIR e CPDR.
- INT é afirmada no último T-state do ciclo interno de repetição; IM2 atende o vetor, `PUSH AF` expõe F e `POP AF; EI; RETI` retorna à instrução ED.
- O teste confirma BC, PC, WZ, SP, IFF1/IFF2, flags e retomada da iteração seguinte.
- Snapshot encode/decode + restore é aplicado em cada fronteira do microsequenciador.
- Z80 focused validation: fmt PASS, Clippy PASS, testes Z80/SMS/SG-1000 PASS e SingleStepTests DD/FD PASS. Documentation patch hygiene PASS; agregador Rust quality/unit PASS.
- Workspace regression e jobs não relacionados ficaram SKIPPED/NOT RUN pela política de impacto; não são promovidos a PASS.

**Limite:** o novo teste é um vetor dirigido por opcode e não fecha F-022. Flags das iterações simples, matriz combinatória completa, WZ/MEMPTR independente, execução integral do ROM v5.0 no core e validação direta em hardware SMS continuam pendentes. CMOS/clones permanecem NOT_VERIFIED. E7 IN_PROGRESS; E8 BLOCKED.


## 32. F-023 — RRD/RLD: cobertura exaustiva integrada pela PR #714

**Baseline main:** `3a81437362dad79577058852b5f469381d3bde2c`.

A candidata acrescenta 262.144 vetores ao executor atômico: RRD e RLD × 256 valores de A × 256 valores de (HL) × Carry=0/1. O oracle independente calcula a permutação dos nibbles e as flags documentadas pelo UM0080: S/Z/PV a partir do novo A, H/N zerados e C preservado. O teste também verifica HL, PC, R e o wrap de WZ em HL=FFFF.

A observação X/Y = bits 5/3 do novo A é testada como comportamento de referência concordante entre MAME e ares; não é apresentada como especificação UM0080 nem como evidência física independente. A candidata acrescenta ainda 16 vetores dirigidos no microsequenciador, com WAIT em cada transação de barramento, snapshot encode/decode + restore por ciclo e ordem de leitura/escrita.

**Limites:** WZ=HL+1 e X/Y permanecem comportamento comparativo de emuladores, não prova de silício; execução do ROM específico e hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED. F-023 permanece AUDIT_PARTIAL. E7 IN_PROGRESS; E8 BLOCKED. PR #714 MERGED em `3a81437362dad79577058852b5f469381d3bde2c`; HEAD validado `bb15e672f30d8aae3777761f1545347fc3d520ea`; CI #6056 / run `42651927378` PASS no HEAD exato da PR e CI pós-merge #6057 / run `42652163312` PASS no HEAD exato da main.


## 33. Integração F-023 — PR #714 — 2026-11-09

**Main:** `3a81437362dad79577058852b5f469381d3bde2c` (squash merge PR #714). **HEAD validado da PR:** `bb15e672f30d8aae3777761f1545347fc3d520ea`. **CI:** #6056 / run `42651927378` PASS no HEAD da PR; #6057 / run `42652163312` PASS no HEAD pós-merge.

- 262.144 vetores atômicos passaram: RRD/RLD × 256 A × 256 (HL) × C=0/1.
- Oracle independente para transformação dos nibbles e flags documentadas S/Z/PV/H/N/C; X/Y esperado de A permanece uma comparação de referência MAME/ares, não especificação do fabricante.
- HL=FFFF e wrap WZ verificados; PC/R/HL e memória conferidos.
- 16 vetores microsequenciados dirigidos passaram com WAIT por pedido de barramento, snapshot/restore em cada ciclo e ordem de barramento confirmada.
- fmt PASS; Clippy PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD PASS; agregador Rust quality/unit PASS. Workspace regression e jobs não relacionados ficaram NOT RUN.
- F-023 permanece AUDIT_PARTIAL: X/Y e WZ não têm prova física independente, ROM dedicado e hardware SMS NOT RUN, CMOS/clones NOT_VERIFIED. E7 IN_PROGRESS; E8 BLOCKED.


## 34. F-024 — LD I,A / LD R,A / LD A,I / LD A,R — CLOSED no perfil NMOS clássico

**Main:** `56d75cc79803494e9fd8a54e8930871c9101fc84` (PR #716 integrada). **HEAD validado:** `9338f8dfee679fee6270c01f9670d6b678d219a0`. **CI:** #6062 / run `42653692191` PASS no HEAD da PR; #6063 / run `42653949243` PASS no HEAD pós-merge.

- 4.096 vetores: quatro opcodes × 256 valores × IFF2 0/1 × Carry 0/1.
- LD A,R considera os dois M1 de fetch antes da leitura de R; LD R,A confirma a cópia integral de A; LD I,A confirma que R ainda recebe os dois incrementos M1.
- Flags S/Z/PV/H/N/C verificadas por fórmula independente; X/Y derivado do valor carregado segue concordância MAME/ares e permanece evidência de compatibilidade, não especificação UM0080.
- Microsequenciador: WAIT em cada M1, snapshot encode/decode + restore em cada fronteira; 9T nominal / 11T com dois WAITs.
- Erratum: INT após LD A,I/R limpa PV; ambos os estados de IFF2 e Carry cobertos nos executores atômico SMS e microsequenciado. NMI e consumo do latch por instrução intermediária continuam cobertos pelos testes dedicados anteriores.
- fmt, Clippy, testes Z80/SMS/SG-1000, SingleStepTests DD/FD e agregador Rust quality/unit PASS. Workspace regression e outros subsistemas NOT RUN por política de impacto.

**Limites:** CLOSED somente para o perfil de compatibilidade Zilog NMOS clássico. UM0080 documenta o erratum e as flags normativas; X/Y e sua fonte são tratados como comportamento de referência MAME/ares, sem prova física. Hardware SMS NOT RUN; CMOS/clones NOT_VERIFIED. E7 permanece IN_PROGRESS por outras famílias; E8 permanece BLOCKED.


## 35. F-025 — RETN/RETI e aliases; modos IM — CLOSED no perfil NMOS SMS/SG-1000

**Main:** `f0cfc7b9df1834ae8ad418c6241702e9003f072e` (PR #718 integrada). **HEAD validado da PR:** `24fd53a5eb7c36462fa6ebc332e1a15b1504cbce`. **CI da PR:** #6071 / run `42658617730` PASS no HEAD exato. **CI pós-merge:** #6072 / run `42658844499` — PASS no HEAD pós-merge `f0cfc7b9df1834ae8ad418c6241702e9003f072e`.

- Oito encodings RETN/RETI: ED45, ED4D, ED55, ED5D, ED65, ED6D, ED75, ED7D.
- 48 combinações por executor: oito aliases × IFF2=0/1 × SP normal/FFFF/0000; IFF1 inicial invertida para comprovar `IFF1 <- IFF2`.
- Executor atômico: PC, SP com wrap, IFF1/IFF2 e refresh R verificados.
- Microsequenciador: comparação diferencial com o executor atômico, leitura de stack low/high em ordem, PC/WZ/SP/IFF/R, WAIT em cada pedido nas fronteiras SP=FFFF/0000 e snapshot encode/decode + restore após cada transação.
- 14T nominais; 18T com quatro WAITs. WZ recebe o PC retornado; MAME e ares confirmam `WZ = PC` para RETN/RETI.
- Os oito aliases de IM0/1/2 permanecem cobertos pelos testes ED anteriores; RETI daisy-chain é NOT_APPLICABLE aos perfis SMS/SG-1000 atuais sem consumidor.

**Limites:** F-025 CLOSED somente para o perfil de compatibilidade Zilog NMOS clássico usado nos alvos SMS/SG-1000. Sem validação física no SMS; CMOS/R800/Z80N/clones NOT_VERIFIED. O fechamento de F-025 não fecha E7 global; E7 IN_PROGRESS e E8 BLOCKED.


## 36. F-001 — BASE LD register/(HL)/immediate — fechamento proposto pela PR #721

**Baseline main pré-PR:** `6945a5845be1233eda2681d8092e5d5d5feadc0c`.
**PR:** #721, HEAD final validado `f22daad35219c9df3e12be7f9df4f0810076947f`, integrada no squash commit `37047ca2739dcfcf1fae6d0fb7bff93dafcaed0b`.
**CI focado:** #6091 / run `42669044781` PASS no HEAD final da PR.

- 25.344 vetores cobrem as 63 codificações válidas `LD r,r'`; HALT `0x76` permanece em F-026.
- 2.816 vetores cobrem os sete destinos de `LD r,n` e `LD (HL),n`, todos os bytes e quatro endereços de HL para o destino memória.
- Um vetor adicional executa `LD A,n` com PC=`FFFF`, buscando o imediato em `0000`.
- O conjunto percorre todos os 256 valores de dado e todos os 256 padrões iniciais de F por permutação bijetiva.
- Os casos H/L verificam que o endereço HL é amostrado antes de `LD H,(HL)`/`LD L,(HL)` alterar o par e que H/L fonte é lido antes de `LD (HL),H`/`LD (HL),L`.
- Oracle independente verifica registradores, PC, memória, flags preservadas, WZ preservado, Q zerado, previous-Q, refresh e R7; comparação adicional entre executor atômico e microsequenciador.
- Ordem e duração: `LD r,r'` = 4T/1 M-cycle; transferências via (HL) e `LD r,n` = 7T/2 M-cycles; `LD (HL),n` = 10T/3 M-cycles.
- WAIT de 1 T-state em cada request; snapshot encode/decode + restore antes/depois de cada resposta.
- Integração SMS: sete formas representativas; snapshot durante M1 pendente; retomada e comparação do estado completo e trace com execução contínua; WAIT e transações de memória verificados.
- Nenhuma alteração de runtime foi necessária: os executores existentes passaram pela matriz.

**Referências:** ZiLOG UM008011-0816, 8-bit Load Group, pp. 71, 72, 74, 79 e 85; MAME `src/devices/cpu/z80/z80.lst`; ares `ares/component/processor/z80/instructions.cpp`. MAME/ares sustentam comportamento comparativo de WZ/Q, não substituem a especificação ZiLOG.

**Estado:** CLOSED para semântica de software do perfil Zilog NMOS delimitado; integrada pela PR #721 no squash commit `37047ca2739dcfcf1fae6d0fb7bff93dafcaed0b`. Não há validação física SMS; CMOS/clones permanecem NOT_VERIFIED. O workflow seletivo validou os pacotes Z80/SMS/SG-1000; regressão integral do workspace ficou SKIPPED/NOT RUN. E7 permanece IN_PROGRESS; E8 permanece BLOCKED.


## 37. F-002 — LD A,(BC)/(DE) e LD (BC)/(DE),A — fechamento proposto pela PR #722

**Baseline main pré-PR:** `37047ca2739dcfcf1fae6d0fb7bff93dafcaed0b`.
**PR:** #722, HEAD final validado `cb54250c7679a55e052cce51ee71323b4db6eb16`, integrada no squash commit `29462cf0a788d5905de7a5e9e0499a1f1361fec1`.
**CI focado:** #6097 PASS no HEAD final da PR.

- 5.120 vetores: quatro opcodes × cinco endereços `00FE`, `00FF`, `1234`, `FFFE`, `FFFF` × 256 valores.
- Todos os 256 padrões de F; flags preservadas; Q zerado e previous-Q retido.
- Loads: `WZ=(BC/DE+1) mod 65536`.
- Stores: `WZ=(A<<8) | ((BC/DE+1) & 0xFF)`.
- Endereço `FFFF` valida wrap de WZ para a parte baixa; A permanece a fonte do byte alto no store.
- M1 + ciclo de dados: 7T/2 M-cycles (4,3), com WAIT 1T no M1 e 2T no ciclo de dados.
- Snapshot encode/decode + restore antes/depois de cada resposta; comparação atômica/microsequenciada e oracle WZ independente.
- Integração SMS com as quatro formas, WAIT, snapshot durante M1 pendente, retomada e igualdade de estado/trace com execução contínua.
- Nenhuma alteração de runtime necessária.

**Referências:** ZiLOG UM008011-0816 pp. 88, 89, 91 e 92; MAME `z80.lst`; ares `instructions.cpp`; superzazu/z80. O manual não especifica WZ; as fórmulas são comportamento de compatibilidade NMOS corroborado pelas três implementações.

**Estado:** CLOSED para semântica de software do perfil NMOS delimitado; integrada por squash na PR #722, commit `29462cf0a788d5905de7a5e9e0499a1f1361fec1`. Sem validação física SMS ou compatibilidade universal CMOS/clones. E7 permanece IN_PROGRESS; E8 permanece BLOCKED.


## 38. F-003 — LD A,(nn) / LD (nn),A — CLOSED no perfil NMOS delimitado

**Baseline main pré-PR:** `29462cf0a788d5905de7a5e9e0499a1f1361fec1`.
**PR:** #724, integrada por squash em `7fde212aabc07e8c4b7df56938f8845f99d29499`.
**HEAD validado da PR:** `455ae4ea5ce68fdba53de1958fe0123afd39dc6d`.
**CI no HEAD exato:** #6111 / run `42675073118` PASS.

- 2.560 vetores: `LD A,(nn)` e `LD (nn),A` × cinco endereços (0000, 00FF, 1234, FFFE, FFFF) × 256 bytes.
- Todos os 256 padrões de F; F preservado, Q zerado, previous-Q retido e refresh/R7.
- WZ load: `(nn+1) mod 65536`; WZ store: `(A<<8) | ((nn+1)&0xFF)`.
- Fetch de operandos testa wrap de PC em FFFE→FFFF→0000.
- Quatro M-cycles / 13T nominais; M1, leitura low, leitura high e ciclo de dados.
- WAITs de 1T/2T/3T/1T por pedido, snapshot encode/decode + restore antes/depois de cada resposta.
- Diferencial atômico × microsequenciador, oracle WZ separado e integração SMS com snapshot durante M1, WAIT, retomada e comparação de trace.
- Nenhuma alteração de runtime.

**Gates no HEAD validado:** rustfmt PASS; Clippy Z80/SMS/SG-1000 PASS; testes Z80/SMS/SG-1000 PASS (incluindo os dois testes F-003 atômicos e a integração temporal SMS); SingleStepTests DD/FD 34.000/34.000 PASS; documentation patch hygiene PASS; agregador Rust quality/unit PASS. Regressão integral do workspace, Scheduler stress e hosts/subsistemas não relacionados ficaram NOT RUN/SKIPPED pela política de impacto.

**CI pós-merge:** NOT VERIFIED nesta consulta. O endpoint de status do commit de merge não retornou checks e a listagem de workflows disponível não expôs execução push associada ao SHA de merge; não atribuir PASS sem run identificado.

**Referências:** ZiLOG UM008011-0816, LD A,(nn), p. 90, e LD (nn),A, p. 93; MAME `z80.lst`; ares `instructions.cpp`. O manual documenta operação/temporização, mas não WZ; as fórmulas WZ são comportamento comparativo NMOS de implementações, não prova física.

**Limites:** CLOSED somente para semântica de software do perfil de compatibilidade Zilog NMOS delimitado. Sem validação física SMS ou compatibilidade universal CMOS/clones. E7 permanece IN_PROGRESS; E8 permanece BLOCKED.


## 39. F-004 — LD dd,nn / LD (nn),HL / LD HL,(nn) — CLOSED no perfil NMOS delimitado

**Baseline main pré-PR:** `355a35662643ca357ca9360031e002964e5a5aef`.
**PR:** #726, integrada por squash em `6ce8399aa96937547e72c24e75f1521dc5a9d550`.
**HEAD validado da PR:** `d4e0c6bcf7b339fee8674102fe26e5b2dd4d131a`.
**CI pré-merge:** #6122 / run `42676982272` PASS no HEAD exato.
**Escopo:** BASE `01/11/21/31` para BC/DE/HL/SP e BASE `22/2A` para transferências absolutas HL. DD/FD E3A1/E3A2, ED F-034 e SP transfer F-005 permanecem em suas famílias, sem duplicação.
**Implementação:** testes semânticos em `native/z80_cpu/tests/base_register_pair_f004.rs`; integração temporal SMS em `native/sms_core/tests/z80_f004_temporal.rs`; relatório em `docs/audits/z80-f004-register-pair-loads-2026-11-09.md`.
**Matriz:** 1.052 vetores `LD dd,nn` + 2.560 vetores `LD (nn),HL`/`LD HL,(nn)` + seis casos de wrap de PC. WAITs diferenciados, snapshot/restore, diferencial atômico/microsequenciado, flags/Q/previous-Q/WZ/R, ordem low/high e scheduler SMS.
**Gates pré-merge:** rustfmt PASS; Clippy com `-D warnings` PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD 34.000/34.000 PASS; documentation hygiene PASS; agregador PASS. Workspace integral, Scheduler stress e hosts/VDP/PSG não relacionados NOT RUN/SKIPPED.
**CI pós-merge:** NOT VERIFIED nesta consulta; não foram retornados checks ou workflow push associado ao SHA de merge. Não atribuir PASS.
**Referências:** ZiLOG UM008011-0816 pp. 99, 102 e 107; MAME `z80.lst`; ares `instructions.cpp`. O manual define tempos/ordem/flags, mas não WZ/Q. WZ=`nn+1` nas transferências HL é comparação MAME/ares, não prova física.
**Runtime:** nenhuma alteração.
**Limites:** CLOSED somente para semântica de software do perfil de compatibilidade Zilog NMOS delimitado. Sem validação física SMS ou compatibilidade universal CMOS/clones. E7 IN_PROGRESS; E8 BLOCKED.


## 40. F-015 — RLCA/RRCA/RLA/RRA — CLOSED no perfil NMOS delimitado

**Baseline main pré-PR:** `f726755c87fcfb61bdf9ac5e2af031afe7712e21`.
**PR #728:** squash merge `ce51578511a0090afbba3ded6312c9a25c5d78c7`.
**HEAD validado da PR:** `c198cff1629270378a3bdd22a1126ca3d98a44b8`.
**CI final pré-merge:** #6131 / run `42678804879` PASS no HEAD exato. #6129 validou o HEAD intermediário `3a48894d585e6ff6599b6487b1fb7d0c1607fa0d`; #6131 validou a revisão documental final.
**Escopo:** BASE `07/0F/17/1F`; 1 M1 / 4T por opcode.
**Implementação:** `native/z80_cpu/tests/base_accumulator_rotates_f015.rs`; integração SMS em `native/sms_core/tests/z80_f015_temporal.rs`; relatório em `docs/audits/z80-f015-accumulator-rotates-2026-11-09.md`.
**Matriz:** 262.144 vetores (4 × 256 A × 256 F), mais 1.024 casos snapshot/restore e quatro PC-wrap. SMS: 192 casos com WAIT M1, snapshot/restore e comparação de trace.
**Semântica documentada:** S/Z/PV preservadas; H/N resetadas; C do bit deslocado; RLA/RRA consomem carry anterior; 4T. UM0080 pp. 205/207/209/211 não documenta X/Y.
**Comparação:** MAME/ares calculam X/Y a partir do novo A. Q não deve ser comparado em representação bruta entre projetos: este runtime guarda o último F escrito em `q`; após rotate, `q=F` e previous-Q preserva o latch anterior.
**Gates:** rustfmt PASS; Clippy `-D warnings` PASS; testes Z80/SMS/SG-1000 PASS; SingleStepTests DD/FD 34.000/34.000 PASS; documentation patch hygiene PASS; agregador PASS. `cargo test --workspace --locked`, Scheduler stress e jobs não relacionados NOT RUN/SKIPPED.
**Runtime:** nenhuma alteração.
**CI pós-merge:** NOT VERIFIED; a consulta ao SHA `ce51578511a0090afbba3ded6312c9a25c5d78c7` não retornou status checks associados. Não atribuir PASS.
**Limites:** CLOSED somente para semântica de software do perfil Zilog NMOS delimitado. Sem validação física SMS, evidência independente para X/Y ou compatibilidade universal CMOS/clones. F-023 permanece gate transversal. E7 IN_PROGRESS; E8 BLOCKED.


## 41. F-005 — LD SP,HL/IX/IY — CLOSED no perfil NMOS delimitado — 2026-11-09

**Baseline main pré-PR:** `43a721b2a293831601ac02dc68b853b11beb363b`.
**PR:** [#731](https://github.com/leonardo201800478/sega-emulator/pull/731), squash merge `0ea60a450c3d58a6231df73c3d1ff98fa118b4b6`.
**HEAD validado:** `e9ab17ebbea042a5396a5cd017c70982c6d2f2cc`.
**CI:** run [42696796388](https://github.com/leonardo201800478/sega-emulator/actions/runs/42696796388), PASS nos gates aplicáveis ao HEAD exato.

- BASE F9 (`LD SP,HL`): 1 M-cycle / 6 T-states.
- DD F9 (`LD SP,IX`) e FD F9 (`LD SP,IY`): 2 M-cycles / 10 T-states.
- Testes cobrem flags preservadas, registradores não-alvo, PC/R, WZ, Q/previous-Q, prefixos repetidos/alternados, WAIT e snapshot/restore nos limites M1/ciclo interno.
- A integração SMS executa BASE F9 através de `SmsTemporalMachine`/`MachineScheduler`, com WAIT M1, snapshot/restore e comparação entre execução retomada e contínua.
- ED 73/7B (`LD (nn),SP` / `LD SP,(nn)`) reutilizam a evidência já registrada em F-034; não houve duplicação.
- Evidência primária: ZiLOG UM008011-0816. Comparação independente de implementação: MAME `src/devices/cpu/z80/z80.lst`.
- Nenhuma divergência de runtime foi reproduzida; a PR alterou somente testes e documentação.

**Gates no HEAD validado:** Z80 focused (fmt, Clippy, Z80/SMS/SG-1000, SingleStepTests DD/FD) PASS; Rust workspace (fmt, Clippy, testes e observability) PASS; Linux audio-output PASS; Windows WASAPI/XAudio2 PASS; Windows egui PASS; agregador PASS. D3D12/DXGI runtime e suítes não relacionadas ficaram NOT RUN/SKIPPED por classificação de impacto. Gates locais Cargo NOT RUN neste ambiente.

**Limites:** CLOSED para semântica de software do perfil Zilog NMOS delimitado. Sem validação física SMS e sem afirmação universal para CMOS/clones. E7 permanece IN_PROGRESS; E8 permanece BLOCKED.


## 42. Reconciliação da main após PRs #736 e #843 — 2026-11-14

**Main auditada:** `2fe93dd4c1ed0af7725b92805e5d1b29c126217e`.

### F-022 — caminhos simples LDI/LDD/CPI/CPD

- **PR #736:** integrada por squash no commit `55585bf2425f1c4eaf61c4a3a3ad42f9499c4797`; HEAD validado `1520cc77e2801d985401a5518eec9da712b89e3c`.
- **CI no HEAD exato:** run #6166 / `42698792012` SUCCESS. Z80 focused validation e agregador de qualidade passaram. Rust workspace regression e Scheduler deterministic stress ficaram SKIPPED; não são PASS.
- A PR não alterou handlers/runtime: acrescentou matriz exaustiva no executor atômico para LDI (ED A0), LDD (ED A8), CPI (ED A1) e CPD (ED A9): 4 × 256 valores de A × 256 bytes de memória × BC inicial 1 ou 2 = **524.288 casos**.
- Transferências: preservação de S/Z/C, H/N limpas, P/V a partir de BC após decremento, X/Y derivados de A + byte transferido, cópia, direção de HL/DE e PC.
- Comparações: preservação de C, N, S/Z/H, P/V após decremento de BC, X/Y a partir do resultado com half-borrow ajustado, A preservado, direção de HL e PC.
- WZ: LDI/LDD preservam o valor semeado; CPI/CPD incrementam/decrementam WZ, incluindo wraparound.
- Os cenários microsequenciados existentes passaram a verificar WZ esperado nos caminhos atômico e sequenciado, mantendo WAIT e snapshot encode/decode + restore nas fronteiras de barramento/ciclo interno. Isso não equivale a executar a matriz de 524.288 casos no microsequenciador.
- **Proveniência e limites:** UM008011-0816 pp. 130–143 fundamenta operação, direção, tempo nominal e flags documentadas. X/Y utiliza Sean Young, *The Undocumented Z80 Documented* v0.4; WZ/MEMPTR usa referência comparativa redcode/Z80. Não é prova física de hardware SMS nem compatibilidade universal CMOS/clones.
- **Classificação:** a lacuna anteriormente identificada para flags/WZ dos caminhos simples foi substancialmente coberta. F-022 permanece AUDIT_PARTIAL: não declarar fechamento global da família nem E7; execução integral do ROM Z80 Block Flags Test v5.0 no core e validação física SMS continuam NOT RUN, e CMOS/clones permanecem NOT_VERIFIED.

### Integração temporal HINT/VINT — PR #843

- **PR #843:** merge SHA `23c77a21ae427feecfb70fa6e78692e588f72416`; HEAD validado `0f2415f9382887c94d9214abe06cb6c295ed2154`.
- **CI no HEAD exato:** run #7389 / `43568659312` SUCCESS; workflow SMS Debugger Runtime #857 / `43568659270` SUCCESS.
- Passaram os jobs SMS1 VDP core/machine integration, Z80 focused validation, Rust workspace regression, input integration e Scheduler deterministic stress.
- A cobertura VINT verifica a fronteira T13/T16, snapshot/restore imediatamente antes do evento e equivalência entre avanço contínuo e em blocos. A cobertura HINT verifica R10=0 no limite T14, restauração de snapshot e a chegada da linha INT ao Z80 após cruzar T14.
- Esta PR acrescenta evidência de integração VDP/Scheduler/linha INT; não altera a semântica de instruções do Z80 e não constitui TEMPORAL_PASS global ou fechamento de E7.6/E7.7.

### Estado global após reconciliação

- E4C/E5/E6 e as evidências DD/FD anteriores não foram reabertas.
- Os 34.000 vetores SingleStepTests DD/FD permanecem vinculados ao HEAD validado da PR #692 (`1606556814a8746a445d3ad32fc948ab97f0fde5`); não foram reexecutados nem reatribuídos ao HEAD atual da main.
- E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED até fechamento demonstrável de E7.
- Nenhuma alteração de implementação Z80 foi integrada pelas PRs #736 ou #843; #736 é test-only e #843 adiciona testes de integração temporal SMS.


## 43. Reconciliação E7 após PRs #929 e #930 — 2026-11-16

**Main auditada:** `6c963d040a4f293c259c19fd09e5e18f2d67be84`.

### E7 — HALT M1, NMI e integração temporal SMS

- **PR #929:** integrada por squash no commit `de5395c1c6e8115324fe23a2086cdcbd9121d1bb`.
- **Teste:** `native/sms_core/tests/z80_halt_nmi_e7.rs`, `sms_temporal_nmi_asserted_during_halt_m1_is_sampled_at_t4_and_restores_exactly`.
- O caso inicia o M1 repetido de HALT antes de solicitar NMI; agenda o barramento e solicita NMI enquanto o M1 está em andamento. Verifica a amostragem na fronteira T4, snapshot/restore, equivalência da retomada, serviço NMI de 11T, PC=$0066, SP=$C0FE, refresh R=$03, bytes empilhados, tempo da máquina e igualdade do trace.
- **CI da PR:** HEAD `66409f3489ad1932cc57f8884a2548afca8fa293`; CI #43813686879 PASS e SMS Runtime #43813686809 PASS.
- **CI pós-merge:** main `de5395c1c6e8115324fe23a2086cdcbd9121d1bb`; CI #43814278038 PASS e SMS Runtime #43814278065 PASS.
- **Limite:** esta evidência fecha o cenário específico de NMI solicitado durante HALT M1 no worker temporal SMS. Não declara cobertura integral de todas as combinações HALT/INT/NMI, clones ou TEMPORAL_PASS global.

### F-022 — matriz microsequenciada LDI/LDD/CPI/CPD

- **PR #930:** integrada por squash no commit `6c963d040a4f293c259c19fd09e5e18f2d67be84`.
- **HEAD da PR:** `c96d39f2a7c3f53a44a44c425f3d34b459eb0a81`.
- **Cobertura adicionada:** 4 opcodes × 256 valores de A × 256 bytes de memória × BC inicial 0/1/2 = **786.432 execuções** no microsequenciador retomável. O oráculo de flags é calculado diretamente das regras de instrução, sem chamar helpers de produção ou o executor atômico.
- Verifica flags preservadas/substituídas, A, BC, DE/HL, PC, refresh, Q e write-back/não write-back. Inclui WAIT e snapshot encode/decode + restore em cada fronteira de barramento e ciclo interno.
- **CI da PR:** HEAD exato #43815302707 PASS (fmt, Clippy, testes Z80/SMS/SG-1000, 34.000 vetores externos DD/FD); SMS Runtime #43815302680 PASS (fmt, Clippy, testes e build Windows x64).
- **CI pós-merge:** main exata `6c963d040a4f293c259c19fd09e5e18f2d67be84`; CI #43815538834 PASS e SMS Runtime #43815538794 PASS.
- **Limites:** WZ/MEMPTR não é promovido por esta matriz. A execução integral do Z80 Block Instruction Flags Test v5.0 no core e a validação física SMS continuam NOT RUN; CMOS/clones permanecem NOT_VERIFIED. **F-022 permanece AUDIT_PARTIAL**, apesar da nova cobertura microsequenciada.

### F-023 — nova fonte de teste MEMPTR derivada de hardware

- Foi identificada a suíte pública [raxoft/z80test](https://github.com/raxoft/z80test), cujo README declara que os resultados esperados foram obtidos em um ZX Spectrum 48K com Zilog Z80 real.
- A variante `z80memptr` executa `BIT 0,(HL)` após cada instrução para tornar observáveis flags relacionadas ao MEMPTR/WZ. Os grupos `RLD` e `RRD` registram CRC MEMPTR esperado `0x670a95ea` cada em `src/tests.asm`.
- **Limite explícito da própria suíte:** o autor informa que os vetores atuais não foram projetados especificamente para estressar MEMPTR e podem deixar defeitos sem detecção. Portanto, é uma fonte complementar de evidência hardware-derived, não uma prova isolada nem fechamento de WZ.
- Integração/execução da variante MEMPTR no core Sega Emulator: NOT RUN. X/Y e WZ ainda não têm evidência física específica e independente no projeto. CMOS/clones permanecem NOT_VERIFIED.

### Estado E7 após a reconciliação

- F-022: AUDIT_PARTIAL — matriz atômica e microsequenciada ampliadas; ROM v5.0 completo e hardware SMS pendentes.
- F-023: AUDIT_PARTIAL — cobertura exaustiva de transformação/flags documentadas; X/Y e WZ sem prova física específica; suíte MEMPTR externa identificada, ainda não executada no core.
- F-028: AUDIT_PARTIAL — auditoria residual de exceções DD/FD, Q/previous-Q e efeitos observáveis ainda pendente.
- E7.7: IN_PROGRESS. E8/ZEXDOC/ZEXALL: BLOCKED até o fechamento formal e demonstrável de E7.
- Nenhuma alteração de implementação do Z80 ou do MachineScheduler foi necessária nas PRs #929 e #930.


## 44. Observação de flags de repetição no acknowledge IM2 — 2026-11-16

**Main auditada:** `f6ee8558b22f8fb9586287545d553db1642705cf`.

### F-022 — flags visíveis ao handler após repetição de instruções de bloco

- **PR #932:** integrada por squash no commit `f6ee8558b22f8fb9586287545d553db1642705cf`.
- **HEAD validado:** `149b928e96cfebd7ca34089e0aec815d4a24339e`.
- **CI da PR:** #43816894151 PASS; SMS Runtime #43816894091 PASS. Ambos executaram no HEAD exato da PR.
- **Cobertura adicionada:**
  - LDIR/LDDR/CPIR/CPDR: 32 cenários com quatro combinações de PC.13/PC.11, BC inicial 0/2 e interrupção IM2 aceita na fronteira após a repetição.
  - INIR/INDR/OTIR/OTDR: 1.024 cenários variando B, C, HL, dados e PC, usando o modelo independente já existente para flags de bloco I/O.
  - Verifica F observado pelo handler, WZ, retorno do PC ao endereço da instrução repetida, contador/ponteiros, efeitos de memória/I/O, pilha IM2, refresh e serviço de 19T.
- **Falha encontrada e corrigida durante a validação:** a primeira versão do teste assumia que Q permanecia igual após o acknowledge. O log mostrou Q=0 na entrada da resposta de interrupção, pois o ciclo iniciado por `Z80::step` limpa Q antes de despachar a resposta. A asserção indevida de Q após INT foi removida; Q continua verificado no limite de repetição anterior ao acknowledge. Nenhuma alteração no runtime foi necessária.
- **Limites:** o teste observa F após o aceite de INT no core; não executa o ROM v5.0 integral nem constitui medição física nova. O comportamento de Q especificamente durante/apos o acknowledge permanece separado e sem prova independente nesta etapa.

### F-023 — investigação adicional de MEMPTR/WZ

- Além da suíte raxoft/z80test registrada na seção 43, foi identificada a suíte de Mark Woodmass listada no README do projeto [redcode/Z80](https://github.com/redcode/Z80). O README descreve que ela verifica os documentos MEMPTR e compara resultados com um NEC D780C-1; também relata testes adicionais em outros clones.
- O runner upstream registra entradas `TEST_FORMAT_WOODMASS` e CRCs/ciclos esperados para `Z80 Test Suite (2008)(Woodmass, Mark)[!].tap`.
- O artefato TAP é referenciado pelo upstream via Wayback Machine; a recuperação do binário expirou por timeout nesta auditoria. Portanto, a suíte é apenas uma fonte candidata identificada, **não executada** no Sega Emulator e não fecha F-023.
- F-023 permanece AUDIT_PARTIAL: WZ após RRD/RLD ainda precisa de execução independente no core e confronto verificável com um artefato hardware-derived. X/Y e variantes NMOS/CMOS também não são promovidos por inferência.

### Estado após PR #932

- F-022: AUDIT_PARTIAL — agora há matriz microsequenciada simples (PR #930) e observação de flags em repetição + acknowledge IM2 (PR #932); execução integral do ROM v5.0 e validação física continuam NOT RUN.
- F-023: AUDIT_PARTIAL — suíte MEMPTR de Mark Woodmass identificada, mas artefato não recuperado/executado; WZ/X/Y ainda sem fechamento independente.
- F-028: AUDIT_PARTIAL — Q/previous-Q em exceções e após acknowledge, precedência residual DD/FD e reconciliação semântica ainda pendentes.
- E7.7: IN_PROGRESS. E8/ZEXDOC/ZEXALL: BLOCKED até o fechamento formal de E7.


## 45. Reconciliação da main após PRs #934/#935 e tentativa ROM v5.0 — 2026-11-16

**Main auditada:** `bf7e7584df421000d040a3ee057799a75bfeb3eb`.

As seções 43 e 44 preservam a evidência já registrada para PR #929 (NMI durante HALT M1), #930 (matriz microsequenciada LDI/LDD/CPI/CPD) e #932 (flags de repetição observadas na aceitação IM2). Esta seção acrescenta somente as mudanças posteriores e a tentativa atual do ROM v5.0.

### F-023 — MEMPTR/WZ observável após RRD/RLD

- **PR #934:** merge SHA `2bec56aaa017909ef3d4583257a2c1c8fb8bb6f0`; HEAD validado `75f897c1fb8270bcc22ea1ff0274bc7dc11c0d58`.
- Executor atômico: 16.384 cenários de RRD/RLD, com BIT 0,(HL) para observar F3/F5 derivados de WZ[11]/WZ[13].
- Microsequenciador: 240 cenários com WAIT e snapshot/restore em cada fronteira relevante.
- CI no HEAD exato: [43818081949](https://github.com/leonardo201800478/sega-emulator/actions/runs/43818081949) PASS; SMS Runtime [43818081966](https://github.com/leonardo201800478/sega-emulator/actions/runs/43818081966) PASS.
- **Limite:** os vetores são regressões observáveis derivadas da documentação. O TAP de Mark Woodmass continua não recuperado/não executado; a suíte externa não foi usada como oráculo independente. F-023 permanece AUDIT_PARTIAL.

### F-028 — exceções de troca com prefixos DD/FD

- **PR #935:** merge SHA `f86108f368f0e7b27af941b91ceec66e53c048b9`; HEAD validado `6e665fe08fa1a32f4b45b42ab78224881b53734a`.
- A nova matriz cobre DD/FD antes de EX AF,AF' (08), EXX (D9) e EX DE,HL (EB), no executor atômico e no microsequenciador.
- Verifica bancos alternativos, IX/IY, WZ, PC/R com wrap, 8T sem WAIT, 10T com WAIT nos dois M1, snapshot/restore e o reset de previous-Q provocado pelo prefixo.
- CI no HEAD exato: [43819970660](https://github.com/leonardo201800478/sega-emulator/actions/runs/43819970660) PASS; SMS Runtime [43819971679](https://github.com/leonardo201800478/sega-emulator/actions/runs/43819971679) PASS.
- **Limite:** são três exceções específicas, não auditoria integral dos 256 opcodes DD/FD. F-028 permanece AUDIT_PARTIAL.

### F-022 — tentativa de execução integral do Z80 Block Instruction Flags Test v5.0

- **PR #936**, HEAD atual observado `6cf3c45c2812d5e96f086630b4218209284a929f`, adiciona o harness `z80_block_flags_rom_v5_passes_on_spectrum48_timing`.
- CI [43823701736](https://github.com/leonardo201800478/sega-emulator/actions/runs/43823701736): **FAIL** no job Z80 focused `133152525181`, na etapa de execução do ROM. O teste excedeu o limite de 50.000.000 T-states sem retornar ao caller.
- Último estado capturado nesse run: PC=`819F`, SP=`87FA`, AF=`0184`, BC=`0763`, DE=HL=`D89D`, I=`86`, R=`66`, IM2, IFF1/IFF2 ativos, sem HALT/INT pendente; opcode no PC inicia com `ED B0` (LDIR). O ROM ainda estava em uma transferência repetitiva quando o orçamento expirou.
- SMS Runtime [43823701761](https://github.com/leonardo201800478/sega-emulator/actions/runs/43823701761) PASS no mesmo HEAD.
- **Classificação:** a execução integral do ROM v5.0 foi tentada e terminou em FAIL/timeout; não é PASS nem NOT_RUN. O log não discrimina ainda entre orçamento do harness, cadência de INT/IM2, stubs das rotinas de apresentação e semântica de repetição do core. Não aumentar o limite nem alterar a CPU sem trace discriminante.

### Estado global no baseline `bf7e7584`

- E7.7 permanece IN_PROGRESS; F-022, F-023 e F-028 permanecem AUDIT_PARTIAL nos limites descritos.
- CMOS/clones fora do perfil Zilog NMOS documentado e validação direta em hardware SMS continuam NOT_VERIFIED/NOT_RUN.
- Os 34.000 vetores SingleStepTests DD/FD continuam vinculados somente ao HEAD validado da PR #692; não foram reexecutados no baseline atual.
- PRs #937 (áudio) e #938 (catálogo de ROMs) não alteraram o núcleo Z80. O CI pós-merge da main `bf7e7584` passou em [43822939548](https://github.com/leonardo201800478/sega-emulator/actions/runs/43822939548), mas os jobs Z80 não foram executados por classificação de impacto.
- **E8/ZEXDOC/ZEXALL permanece BLOCKED.** Não iniciar E8 antes do fechamento demonstrável de E7.


## 46. Reconciliação de baseline, SingleStepTests e diagnóstico ROM v5.0 — 2026-11-16

**Main documental consultada:** `5f24e22f2c49512ea43fa24ea25f836464d70f61`.

**Baseline de código Z80:** `bf7e7584df421000d040a3ee057799a75bfeb3eb`. A comparação entre essas SHAs mostra somente alterações em `docs/z80-completion-scorecard.md` e `docs/z80-dd-fd-semantic-matrix.md`; a PR #939 foi documental e não alterou o código Z80. Portanto, a baseline documental atual e a baseline de código validado devem ser registradas separadamente.

### SingleStepTests Z80 DD/FD — proveniência corrigida

Os 34.000 vetores externos não foram comprovados exclusivamente na PR #692. A execução explícita do harness com o checkout fixado `vendor/SingleStepTests-z80/v1` foi repetida em HEADs posteriores:

| PR | HEAD validado | Run CI / job Z80 focused | Resultado |
|---|---|---|---|
| #930 | `c96d39f2a7c3f53a44a44c425f3d34b459eb0a81` | [43815302707](https://github.com/leonardo201800478/sega-emulator/actions/runs/43815302707) / 133125928882 | PASS |
| #934 | `75f897c1fb8270bcc22ea1ff0274bc7dc11c0d58` | [43818081949](https://github.com/leonardo201800478/sega-emulator/actions/runs/43818081949) / 133134775584 | PASS |
| #935 | `6e665fe08fa1a32f4b45b42ab78224881b53734a` | [43819970660](https://github.com/leonardo201800478/sega-emulator/actions/runs/43819970660) / 133140618874 | PASS |

Os logs de cada job mostram a invocação explícita de `cargo test --release --locked -p sega-z80-cpu --test singlestep_indexed_alu -- --ignored`, com os dois testes de corpus executados e aprovados. O último HEAD de código Z80 com a evidência de 34.000/34.000 é o HEAD validado da PR #935, `6e665fe...`; isso não transfere PASS para a main documental `5f24e22...`.

No CI da main [43831038791](https://github.com/leonardo201800478/sega-emulator/actions/runs/43831038791), o job Z80 focused (job 133176280039) ficou **SKIPPED**. Classificação para a main atual: **NOT RUN**, não PASS. As regressões de workspace também ficaram SKIPPED nesse run.

### F-022 — execução ROM v5.0 e diagnóstico temporal

A tentativa inicial registrada na seção 45 excedeu 50.000.000 T-states durante LDIR, sem retornar ao chamador. Essa execução foi FAIL/timeout e não pode ser promovida a aprovação.

Uma revisão instrumentada posterior, HEAD `adaa349366ccda210faacecb0ce9a93031bc0c78`, executou a ROM até o B-loop e falhou na asserção: B obtido 13, esperado 15. CI [43839665389](https://github.com/leonardo201800478/sega-emulator/actions/runs/43839665389), job Z80 focused 133205064133. O trace válido dessa execução registrou:
- início do OTIR em T-state 11.112.138, PC=800E, BC=10FE;
- borda de INT em T-state 11.112.192, delta de 54T em relação ao início;
- iterações OTIR subsequentes em intervalos de 21T;
- aceite de INT em T-state 11.112.201, PC=800E, BC=0DFE, após três decrementos de B.

O resultado confirma uma divergência de fase observável no harness/execução da ROM, mas não isola sua causa. Não conclui se o orçamento de preparação LDIR, o overhead da rotina, a fase do pulso INT/IM2 ou o accounting do executor é responsável. Não aplicar compensação fixa de 42T nem alterar a CPU sem medir o caminho de preparação completo.

Uma instrumentação adicional do LDIR de calibração foi adicionada em `e0c4cadd443c5fd61717cfda07c386a4bcc4be0d`; esse HEAD falhou antes da execução por `clippy::collapsible_if`. A correção mínima foi aplicada em `4ca567c13f1ca95d84a8ff3a8143775a53a3f6c7`, cujo teste executou, mas o primeiro trace encerrou incorretamente na entrada do handler IM2: PC mudou enquanto BC ainda era `33CD`, logo o LDIR estava suspenso e não concluído. A amostra parcial foi `start_t=10627312`, `end_t=10692893`, `elapsed=65581`, `start_bc=3FFF`, `end_bc=33CD`, com borda de frame em `10692864`. A lógica foi corrigida em `bdc7eb73f1a03e5d5a59e0cfce99e2fe6a404e1c` para continuar através do handler/RETI. O novo trace registrou `elapsed=65871`, `BC=3FFF→0000`, `PC=8051→8595`, nominal calculado 344.038T. A amostra ainda era inválida: `PC=8051 / BC=3FFF` identifica o LDIR principal de preenchimento, mas `PC=8595 / BC=0000` corresponde ao LDIR curto executado dentro do próprio handler IM2 (`push AF,HL,DE,BC; ... LDIR; pop BC,DE,HL,AF; EI; RET`). O trace confundiu a saída do handler com a conclusão do LDIR interrompido. Portanto, não há medição válida da duração total do LDIR nem evidência de undercount do core. O HEAD `a0f50a28c4615c779cca0407126115b56d3b3931` adicionou amostras por instrução; a correção seguinte `5081a293f1cc664f4fef0bb330a904990995e4d7` exige observar o retorno ao PC do LDIR após o desvio e só então aceitar sua saída com BC=0. CI pendente. O limite atual de 80M T-states já estava presente no harness antes desta reconciliação; não foi aumentado nesta investigação e não é considerado correção do problema.

### F-022 — resultado final ROM v5.0 no core — 2026-11-16

A PR #936, HEAD `a7f551336104e1c2c03d7e6790b0aafbb5e346e7`, executou o teste `z80_block_instruction_flags_test_v5_passes_on_spectrum48_timing` com **1 passed/0 failed** em 6,48 s. CI [#43846321428](https://github.com/leonardo201800478/sega-emulator/actions/runs/43846321428), job Z80 focused, SUCCESS.

O harness passou ≥60 comparações principais de instrução/PC e 390 registros B-loop em seis grupos (OTIR×2, OTDR×2, INIR, INDR), com B observado `13 + 3 × iteration` e flags exatas em todos os casos. A calibração de delay, a ausência de IN inesperado e o retorno ao chamador também passaram. A ROM usa quadro Spectrum 48K de 69.888 T-states, pulso INT de 32T e orçamento máximo de 80M T-states.

A tentativa inicial de 50M T-states expirava durante a rotina LDIR de calibração; o trace posterior demonstrou que o handler IM2 contém seu próprio LDIR e exigiu acompanhar o RETI até o PC original para não confundir o LDIR do handler com o LDIR interrompido. O orçamento de 80M já existia no harness antes desta reconciliação e não foi aumentado nesta investigação. A execução final completou a sequência.

A referência upstream [Peter Helcmanovsky — Z80 Block Flags Test](https://github.com/MrKWatkins/ZXSpectrumNextTests/tree/8adcbc8384b2311fb6d36d424478aef583d5c307/Tests/ZX48_ZX128/Z80BlockInstructionFlags) informa testes em hardware real ZX Spectrum 128/Spectrum+ e limita expressamente a suíte a um conjunto não exaustivo de condições. O SNA MIT é baixado no CI (49.179 bytes; Git blob SHA `41b7b0da0f5abfd5d2841fbae005f05291986f8b`), sem binário commitado. A auditoria detalhada está em [z80-e7-block-flags-rom-v5-2026-11-16.md](audits/z80-e7-block-flags-rom-v5-2026-11-16.md).

SMS Debugger Runtime [#43846321313](https://github.com/leonardo201800478/sega-emulator/actions/runs/43846321313) também PASS em Release format, Clippy e testes. SingleStepTests DD/FD 34.000/34.000 e regressões Z80/SMS/SG-1000 passaram no mesmo HEAD. Jobs não relacionados ao diff ficaram SKIPPED.

### Estado E7 e escopo de hardware

- F-022: **AUDIT_PARTIAL** — matriz microsequenciada de 786.432 casos registrada na seção 43; ROM v5.0 agora PASS no HEAD da PR #936 (≥60 comparações principais + 390 B-loop records), mas a suíte upstream é limitada e não fecha a matriz completa de iteração simples/microsequenciada.
- F-023: **AUDIT_PARTIAL** — os 16.384 cenários atômicos e 240 microsequenciados não substituem a suíte independente MEMPTR/WZ; o TAP de Mark Woodmass não foi recuperado/executado.
- F-028: **AUDIT_PARTIAL** — 34.000 vetores DD/FD PASS em três HEADs de PR posteriores, mas sem reexecução na main atual e sem fechamento das exceções/interações residuais.
- O checklist mestre deve classificar individualmente cada requisito como CLOSED, NOT_APPLICABLE com justificativa ou pendente; SKIPPED permanece NOT RUN.
- O escopo de fechamento de software permanece o perfil Zilog NMOS clássico SMS/SG-1000 declarado no projeto. Validação física de silício, variantes CMOS e clones não são gates universais automáticos; devem permanecer explicitamente NOT_VERIFIED quando não executados.

**Estado global:** E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED. Esta reconciliação documental não altera o runtime nem fecha F-022/F-023/F-028.


## 47. F-023 — execução da suíte MEMPTR de Mark Woodmass — 2026-11-16

**PR candidata:** #942 (Draft). **HEAD exato testado:** `7402ff138ced95da2d8b9e0008919c8cd22484f1`.

**Base main da PR:** `caa9a76a6f09d588155ea4e0cbc95f487f079143`. O HEAD atual da PR após a inclusão desta reconciliação (`2d7d796b0da94b3b4108e1510fdf2ae7d9b1b643`) difere do HEAD testado somente nos três arquivos documentais desta seção; o código/harness/CI testados não foram alterados.

- A suíte MEMPTR de Mark Woodmass (2008), descrita pelo upstream como comparada com NEC D780C-1 e outros clones, foi executada no core compartilhado. A referência MEMPTR documenta RLD/RRD com WZ=HL+1.
- TAP GPL-2.0 e ROM Spectrum 48K são obtidos somente no CI e verificados por SHA3-512/Git blob SHA; nenhum binário foi adicionado ao repositório.
- CI [#43850972785](https://github.com/leonardo201800478/sega-emulator/actions/runs/43850972785) PASS no job Z80 focused. Teste 09: 50.904.931 T-states, FNV-1 `F5AE5140`, 61 linhas, 32 colunas e HALT em 0x80E6; 1 passed/0 failed.
- SMS Runtime [#43850972708](https://github.com/leonardo201800478/sega-emulator/actions/runs/43850972708) PASS; SingleStepTests DD/FD 34.000/34.000 e regressões Z80/SMS/SG-1000 PASS no mesmo HEAD.
- Esta evidência valida a saída agregada da suíte MEMPTR hardware-derived, não uma captura física local nem todos os bits não documentados. X/Y, matriz temporal completa e validação física SMS permanecem pendentes; CMOS/clones NOT_VERIFIED.

**Estado:** WZ/MEMPTR agora possui uma referência externa executada e aprovada; F-023 continua AUDIT_PARTIAL. A PR #942 ainda não foi integrada. E7 IN_PROGRESS; E8 BLOCKED.

## 49. Encerramento da rodada de testes E7 — 2026-11-16

Relatório consolidado: [audits/z80-e7-current-status-2026-11-16.md](audits/z80-e7-current-status-2026-11-16.md).

- F-022 / PR #936: HEAD 306db2a91ff796b4c5f66eabc4ebd4c4baad903e; CI #43852582196 + SMS Runtime #43852582229 SUCCESS. ROM v5.0 1/1 PASS; 390 B-loop records, ≥60 comparações principais; 34.000 SingleStepTests DD/FD PASS. Não exaustivo; AUDIT_PARTIAL.
- F-023 / PR #942: código 7402ff138ced95da2d8b9e0008919c8cd22484f1; CI #43851550529 + SMS Runtime #43851550520 SUCCESS. Woodmass MEMPTR 09 PASS, 50.904.931 T-states, FNV-1 F5AE5140, HALT 0x80E6, saída 61×32. Resto da matriz WZ/X/Y pendente; AUDIT_PARTIAL.
- F-028 / PR #943: HEAD testado 98c1cf41587f6af66c3d272e4b07a638d5da7919; CI #43859000666 + SMS Runtime #43859000676 SUCCESS. 108 casos prefixados por executor em 12 formas, quatro testes PASS, WAIT/snapshot. Fixture SCF/CCF corrigido pelo Q inicial, sem alteração de CPU. Matriz integral pendente; AUDIT_PARTIAL.
- Nenhuma das PRs foi mesclada. Resultados pertencem aos HEADs indicados, não à main.
- E7 IN_PROGRESS; E8 BLOCKED.

## 50. Matriz integral de classificação DD/FD e fronteiras de interrupção — 2026-11-16

**Baseline main:** caa9a76a6f09d588155ea4e0cbc95f487f079143.
**Branch PR #943:** test/z80-f028-exception-matrix-20261116.
**Código de teste após extensão:** dcfb74fae2d207126da59562eb8745fa0d733bd4.

### Inventário completo dos bytes após DD/FD

- Foi adicionado o CSV docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv, com 256 registros, um para cada byte 00–FF.
- A ação efetiva da FSM particiona o espaço em 85 UsesIndex, 167 Ignored, 1 IndexedCB, 1 cancelamento de indexação por ED e 2 repetições DD/FD.
- A função auxiliar classify_index_opcode() retorna Ignored para DD/FD/ED porque a FSM trata esses bytes antes de chamar o classificador. O ledger distingue a classificação auxiliar da transição efetiva para não mascarar essa precedência.
- HALT 76 permanece BASE/Ignored; EX AF,AF' (08), EXX (D9) e EX DE,HL (EB) permanecem exceções ignoradas. As formas 66/6E/74/75 usam o endereço IX/IY+d, mas preservam H/L normais.
- O ledger é uma fotografia da classificação implementada, não uma prova independente de semântica para os 256 opcodes.

### Testes candidatos F-028

- index_prefix_classification_f028.rs enumera os 256 bytes e compara o resultado com uma lista explícita de 85 UsesIndex; a contagem do classificador auxiliar esperada é 85 UsesIndex, 170 Ignored e 1 IndexedCB.
- z80_dd_fd_exception_matrix_f028.rs recebeu dois testes adicionais: NMI e INT são solicitados após o M1 de DD, durante a cadeia DD FD 00; WAIT e snapshot/restore são aplicados nos M1; o teste exige que o opcode terminal seja aposentado antes de NMI/INT ser aceito na próxima fronteira.
- Esses testes usam o microsequenciador Z80 diretamente; não são um teste adicional de integração do MachineScheduler SMS nem um oracle físico independente.
- O primeiro CI das extensões, #43862912238, e SMS Runtime #43862912321 falharam exclusivamente em cargo fmt --all -- --check; Clippy, testes e build foram SKIPPED. As diferenças apontadas foram corrigidas nos commits 0e29f164e2047bf206902de7d87b1a0c6efabe19 e dcfb74fae2d207126da59562eb8745fa0d733bd4.
- O diff de rustfmt foi aplicado aos dois arquivos de teste. A revisão desta atualização ainda aguarda o CI do HEAD atual; até conclusão, os novos testes são NOT RUN/NOT VALIDATED.

### Estado

- O repositório já possuía smoke tests de 256 bytes para BASE/DD/FD/CB/ED e para os 256 subopcodes DDCB/FDCB; esses testes comprovam dispatch sem erro, não semântica. A suíte SMS indexed-CB cobre rotate/shift para todos os valores/carry, enquanto outras famílias têm matrizes dirigidas. O novo ledger e teste adicionam classificação explícita UsesIndex/Ignored/IndexedCB. F-028 permanece AUDIT_PARTIAL: semântica integral por opcode/estado e interações restantes ainda não demonstradas.
- A extensão não alterou código de produção, CPU, decoder, microsequenciador, MachineScheduler ou runtime.
- Não transferir o PASS anterior de #43859000666/#43859000676 ao código posterior.
- E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED.

## 51. Estado pós-merge das evidências E7 — 2026-11-16

**Main integrada:** `fbd70c4d668902d1869d7cfbdfde5c880c9b3857`.

PRs #936 (F-022), #942 (F-023), #943 (F-028) e #944 (cenários microsequenciados de repetição/INT IM2) estão MERGED. A #943 foi testada nos workflows CI #43880197594 e SMS Runtime #43880197552, ambos SUCCESS no HEAD `81d6f2fe955867dc16c5e80061524ea713c99c28`; o merge commit `fbd70c4d668902d1869d7cfbdfde5c880c9b3857` possui a mesma árvore Git do HEAD testado.

- **F-022 — AUDIT_PARTIAL:** ROM v5.0 e os cenários microsequenciados da #944 estão integrados. A distinção F-021/F-022 permanece obrigatória: os 390 B-loop records pertencem a OTIR/OTDR/INIR/INDR (F-021). Ainda é necessário reconciliar as comparações principais da ROM com LDI/LDD/CPI/CPD e demonstrar a cobertura de flags/iteração nos casos simples e microsequenciados, WAIT e interrupções.
- **F-023 — AUDIT_PARTIAL:** Woodmass MEMPTR test 09 está integrado e foi aprovado no HEAD da #942; assinatura FNV-1 `F5AE5140`, 50.904.931 T-states, HALT `0x80E6`, saída 61×32. A suíte não fecha a matriz de WZ/MEMPTR, X/Y e temporização para todas as famílias e condições.
- **F-028 — AUDIT_PARTIAL:** ledger de 256 bytes, classificação UsesIndex/Ignored/IndexedCB e testes de INT/NMI entre DD/FD estão integrados. CI #43880197594 e SMS Runtime #43880197552 passaram no HEAD #943; a árvore é idêntica à main integrada. O ledger classifica o dispatch, não prova semântica integral dos 85 UsesIndex nem fallback temporal dos 167 Ignored.
- **#944 — evidência parcial complementar:** 64 cenários microsequenciados de repetição/INT IM2 permanecem integrados; não cobrem NMI nem IM0/IM1.

Os estados F-022/F-023/F-028 não são promovidos a CLOSED. E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED. Não reabrir famílias já fechadas sem regressão reproduzível.
