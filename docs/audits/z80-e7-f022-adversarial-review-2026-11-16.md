# Z80 E7 — revisão adversarial F-022 (Block Transfer/Compare)

**Data:** 2026-11-16
**Main de referência:** `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`
**Perfil:** Zilog Z80 NMOS clássico, escopo SMS/SG-1000.
**Resultado:** auditoria interna concluída; candidata a fechamento, pendente de revisão independente registrada.

## 1. Escopo

F-022 cobre exclusivamente as oito instruções de transferência/comparação em bloco:

- LDI, LDD, LDIR, LDDR;
- CPI, CPD, CPIR, CPDR.

INI/IND/OUTI/OUTD e variantes repetitivas pertencem a F-021 e não são usadas como evidência de F-022.

A revisão confrontou as regras de flags, WZ, PC, BC, HL/DE, Q/previous-Q, condições de repetição, interrupção e retomada nos executores atômico e microsequenciado, e na integração SMS/MachineScheduler. Não houve alteração de produção.

## 2. Revisão por família

### Formas simples — LDI/LDD/CPI/CPD

O teste `native/z80_cpu/tests/ed_block_transfer_f022_resumable.rs` executa **786.432 cenários**: quatro opcodes × todos os 256 valores de A × todos os 256 valores de byte de memória × três estados BC (0, 1, 2).

A fórmula esperada é independente dos helpers de produção e verifica:
- LDI/LDD: preservação de S/Z/C, PV conforme BC após decremento, H/N limpos e X/Y a partir de A+byte transferido;
- CPI/CPD: C preservado, N setado, S/Z/H do resultado de A−byte, PV conforme BC após decremento e X/Y a partir do resultado ajustado por half-borrow;
- Q acompanha F e previous-Q preservado;
- direção e wrap de HL/DE/BC;
- WZ: LDI/LDD preservam; CPI/CPD ajustam WZ conforme direção.

O mesmo arquivo executa WAIT de fixture e round-trip de snapshot/restore em cada request e ciclo interno, verificando T-states e estado final.

A integração SMS Scheduler das formas simples é exercitada por `e7_f022_simple_forms_match_scheduler_wait_snapshot_replay` em `native/sms_core/tests/snapshot_restore_e7_2e.rs`, comparando execução contínua e retomada.

### Formas repetitivas — LDIR/LDDR/CPIR/CPDR

O microsequenciador cobre flags, PC/BC/WZ/Q e interrupção IM2 em `repeat_im2_microsequencer_matrix_covers_all_pc_bc_and_q_boundaries`: 4 opcodes × 4 padrões de byte alto do PC × 2 sementes BC × 2 sementes Q (64 cenários). A fórmula de flags é separada dos helpers de produção; o teste declara corretamente que é validação interna, não oracle de silício.

O teste atômico `ldir_lddr_cpir_cpdr_flags_survive_im2_acceptance_after_repeat` em `native/sms_core/tests/z80_ed_block_repeat_interrupt_observation.rs` cobre 32 combinações opcode/PC/BC, captura flags visíveis no handler IM2 e verifica WZ, stack, PC, BC, HL/DE e refresh.

A integração temporal SMS está coberta em `native/sms_core/tests/snapshot_restore_e7_2e.rs`:
- `e7_f022_repeat_interrupt_is_sampled_and_restored_by_sms_scheduler`;
- `e7_f022_repeat_im2_handler_observes_flags_and_restores_through_scheduler`;
- `e7_f022_repeat_nmi_is_sampled_and_restored_by_sms_scheduler`.

Os testes abrangem as quatro formas, INT IM1/IM2 e NMI, amostragem no último T-state da repetição, handlers PUSH AF/POP AF/EI/RETI e PUSH AF/POP AF/RETN, flags observadas/restauradas, WZ, IFF, EI delay, stack, PC/BC e retomada. A comparação contínua versus snapshot/restore inclui trace e timestamp.

## 3. Proveniência — distinção necessária

A ROM Z80 Block Instruction Flags Test v5.0, harness `native/z80_cpu/tests/z80_block_flags_rom_v5.rs`, é hardware-derived segundo a proveniência upstream documentada em `z80-e7-block-flags-rom-v5-2026-11-16.md`. O README upstream declara execução em ZX Spectrum 128/Spectrum+ e cobre um conjunto limitado de cenários.

A separação correta é:
- **F-022:** a ROM contém 16 comparações principais para LDIR/LDDR/CPIR/CPDR em cenários de repetição interrompida;
- **F-021:** os 390 registros B-loop são OTIR/OTDR/INIR/INDR. Eles não são evidência para F-022;
- a ROM não cobre a matriz completa das oito formas nem substitui os 786.432 cenários simples e a validação microsequenciada;
- os flags documentados seguem ZiLOG UM0080; X/Y das formas simples seguem a referência independente Sean Young, *The Undocumented Z80 Documented* v0.4. Não classificamos essas fórmulas como nova medição física;
- WAIT continua sendo fixture, não medição de WAIT físico.

## 4. Achados adversariais

1. **Nenhuma divergência** foi encontrada entre as fórmulas independentes das formas simples e os resultados do microsequenciador nos 786.432 cenários.
2. **Nenhuma divergência** foi encontrada entre repetição e término nas quatro formas, incluindo BC=0/underflow, match-stop das comparações, PC rewind, WZ, flags/Q e direção.
3. A matriz de interrupção não confunde INT com NMI: IM2 preserva flags observáveis no handler e RETI restaura o retorno; NMI usa RETN para restaurar IFF1 a partir de IFF2.
4. A evidência ROM foi delimitada corretamente: 16 comparações principais de repetição para F-022; 390 registros B-loop de I/O para F-021.
5. Não foi encontrado caminho de produção em F-022 que dependa de polling/callback da GUI; execução e retomada permanecem no Z80/Scheduler Rust.

## 5. Validação vinculada aos SHAs

- PR #972 HEAD `ff922996e797b287e275fdbf1d1791314c61a749`: CI #43918490165 Rust workspace regression SUCCESS; `snapshot_restore_e7_2e`: 13 passed, 0 failed; SMS Runtime #43918490096 SUCCESS. Z80 focused e Scheduler deterministic stress foram SKIPPED nessa PR.
- PR #975 HEAD `7574245338549fd8f37a7483ef41b7fbc306ffe5`: CI #43937118700 SUCCESS, Z80 focused e Rust quality/unit PASS; SMS Runtime #43937118785 SUCCESS. Esse Z80 focused executa os pacotes Z80/SMS/SG-1000; Scheduler deterministic stress separado ficou SKIPPED.
- Main após merge #975: `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`. Não se declara workflow push pós-merge executado nesse SHA.

## 6. Decisão

**F-022: INTERNAL_REVIEW_COMPLETE — CANDIDATE FOR CLOSURE.** A revisão interna e a evidência técnica estão concluídas; o fechamento formal depende de revisão independente registrada.

O fechamento é sustentado por fórmula independente nas formas simples, matrizes microsequenciadas, observação hardware-derived limitada às formas repetitivas, integração SMS/Scheduler e retomada determinística. Não significa medição física individual de todos os estados ou WAIT elétrico, nem conformidade universal de clones.

F-023 também tem auditoria interna concluída, mas permanece candidata a fechamento até a cobertura PR #977 e revisão independente. **F-028 e o gate global E7 permanecem IN_PROGRESS. E8/ZEXDOC/ZEXALL permanece BLOCKED.**


## Decisão de escopo supersedente — 2026-11-16

E7 está `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e para as necessidades praticáveis dos sistemas SMS/SG-1000. Esta decisão do projeto não declara revisão externa, conformidade elétrica universal ou validação de hardware para os casos não executados. As evidências desta auditoria permanecem internas e vinculadas aos SHAs/workflows descritos em cada seção. CMOS/clones não caracterizados, WAIT físico, DMA e sincronização física global permanecem NOT_VERIFIED/NOT_RUN. E8/ZEXDOC/ZEXALL é validação complementar futura e não bloqueia o início do SG-1000. Reabrir E7 somente diante de defeito/regressão reproduzível ou evidência técnica concreta.
