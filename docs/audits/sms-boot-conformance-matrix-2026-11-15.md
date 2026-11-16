# SMS Boot Compatibility — matriz de conformidade existente

**Data:** 2026-11-15  
**Baseline de testes inventariada:** `main` — `90036d8d9f988640faf1540d2b4dc1548780c13f`  
**Branch:** `fix/sms-boot-compatibility`  
**Tipo de alteração:** somente documentação; nenhum teste ou código de runtime foi alterado.

## 1. Objetivo

Inventariar os testes já existentes antes de acrescentar cobertura. Evitar duplicar testes temporais, de mapper, VDP, PSG, worker ou Save State que já estão presentes. “Teste existente” significa que o teste está no código; não significa que foi executado nesta branch.

## 2. Matriz por subsistema

| Subsistema | Cobertura existente relevante | Estado da cobertura | Lacuna para SMS Boot Compatibility |
|---|---|---|---|
| CPU / reset | `cpu_reset_preserves_sms_scheduler_and_peripheral_state`; `cpu_reset_rejects_an_in_flight_instruction_without_mutating_state`; `nop_runs_on_the_scheduler_timeline_for_four_tstates`; `nop_opcode_fetch_uses_four_scheduler_driven_m1_tstates` | Boa cobertura temporal e de reset isolado | Falta teste explícito de todo o snapshot `Z80MicroSequencer::default()` e comparação com contrato de boot (sem declarar os defaults como hardware) |
| CPU / CALL / stack | `push_and_pop_use_scheduler_bus_cycles_and_nominal_timing`; `call_and_rst_use_scheduler_stack_transactions_and_timing`; `ret_and_conditional_ret_use_scheduler_stack_reads_and_timing`; `direct_boot_default_sp_call_overwrites_sega_mapper_registers` | Mecanismo SP=0/mapper já tem teste sintético | Falta trace real dos dois jogos para saber se usam CALL/PUSH antes de inicializar SP |
| CPU / WAIT / retomada | `opcode_fetch_wait_states_are_scheduler_tw_events`; `snapshot_restore_resumes_pending_fetch_with_wait_without_repeating_it`; `m1_snapshot_restores_pending_tstate_and_retires_exactly_once`; `restore_rejects_bus_deadline_that_disagrees_with_cpu_wait_counter` | Cobertura forte de retomada e causalidade | Nenhuma lacuna específica identificada para boot além da execução com ROM real |
| Memória / BIOS | `bios_maps_all_eight_kib_and_can_be_disabled`; `bios_power_on_profile_matches_sms_slot_and_ram_latches`; `cartridge_has_priority_when_bios_and_cartridge_slots_are_both_enabled`; `disabled_cartridge_slot_does_not_receive_mapper_register_writes` | Cobertura sintética do mapa BIOS/cartucho | Não prova execução de BIOS comercial nem estado no handoff |
| Memória / direct boot | `cartridge_boot_profile_matches_post_bios_slot_and_ram_latches`; `cartridge_slot_disable_returns_open_bus_without_disabling_work_ram`; `memory_control_port_can_disable_and_reenable_work_ram` | Cobertura do latch e do mapa escolhido pelo core | Falta caracterizar o contrato explícito de boot direto e seus campos não determinados |
| Memória / RAM | `mirrors_work_ram_across_the_upper_address_window`; `mapper_register_writes_also_update_the_ram_mirror`; `shared_bus_reads_cartridge_and_writes_mirrored_work_ram` | Cobertura de espelhamento e side effects | Falta distinguir RAM zerada pelo construtor de RAM efetivamente inicializada por BIOS; observar $C000 |
| Mapper Sega | `maps_fixed_first_kib_and_three_banked_slots`; `slot_zero_preserves_bank_offset_after_fixed_vector_area`; `mapper_register_writes_also_update_the_ram_mirror`; `mapper_write_snapshot_restore_replays_identical_temporal_trace` | Cobertura de bancos e snapshot | Falta validar o mapper das ROMs-alvo por hash/cabeçalho/trace |
| Mapper Codemasters | `codemasters_header_selects_its_three_slot_mapper`; `codemasters_mapper_registers_stay_disabled_during_bios_power_on`; `codemasters_mapper_does_not_expose_sega_cartridge_sram` | Cobertura básica presente | Detecção por assinatura não é um classificador fail-closed; ROMs não reconhecidas caem em Sega |
| Mapper desconhecido | `invalid_codemasters_metadata_keeps_the_sega_mapper`; `unknown_sega_mapper_cartridge_does_not_infer_sram_from_rom_size` | Política atual explícita de fallback | Falta estado `Unknown/Ambiguous` previsto no plano de integridade de cartuchos; não implementar sem identidade das ROMs |
| VDP / registradores | `control_register_vram_write_and_wrap`; `first_control_byte_updates_only_low_read_address_without_read_ahead`; `register_write_command_updates_vdp_address_register`; `status_port_preserves_sprite_index_and_clears_status_flags` | Cobertura funcional ampla | Valores zerados no construtor não estão validados como power-on físico ou estado após BIOS |
| VDP / VC/HC | `vdp_counters_are_read_through_the_sms_mirrored_ports`; `h_counter_port_reads_return_the_th_latched_value_without_retiming_it`; `h_counter_th_capture_is_invariant_to_scheduler_chunking`; `h_counter_latch_survives_bus_snapshot_restore_and_rejects_old_format` | Cobertura temporal de contador/latch presente | Falta reproduzir o caminho exato de leitura VC/HC do SmsVdpTest e capturar em T-state equivalente |
| VDP / raster e IRQ | `sms1_vint_boundary_is_chunk_invariant_and_survives_snapshot_restore`; `sms1_hint_r10_boundary_is_preserved_by_scheduler_snapshot_restore`; `sms1_vcounter_changes_at_hpos23_of_each_scanline`; `sms1_vint_irq_precedes_the_vblank_status_flag_on_the_following_scanline` | Cobertura temporal robusta | PR #912 ainda investiga discrepâncias do ROM completo; não alterar VDP fora da evidência de boot |
| I/O | `released_controllers_read_high_and_ports_mirror`; `io_control_mirrors_and_driven_th_levels_are_observable_on_export_sms`; `external_th_lines_are_active_low_and_independent`; `driven_th_output_overrides_external_input_level`; `SmsIo::read_dd()` expõe reset_pressed no bit 4 | Cobertura de direção/TH e mirrors | Falta teste do bit RESET ativo-baixo e gating por modelos sem botão físico |
| PSG | `reset_and_repeated_advancement_are_deterministic`; `sms_audio_generation_is_deterministic_and_chunk_invariant`; `sms_white_noise_trace_matches_independent_gear_system_reference`; `psg_clock_advances_with_cpu_clock_domain_for_each_dispatched_boundary` | Cobertura temporal/determinística forte | Fase inicial de contadores/LFSR ainda carece de referência de hardware; especificação de áudio anexada diverge do valor tone counter no core |
| Worker / reset | `stop_powers_off_machine_but_keeps_rom_available_for_restart`; `reset_and_load_rom_restore_power_on_machine_counters`; `worker_exports_sram_and_preserves_it_across_reset_and_power_off` | Cobertura de lifecycle existente | Adicionar caminho BIOS real quando a integração for implementada; diferenciar Reset do worker de pino RESET |
| Worker / Save State | `debugger_snapshot_and_mapper_reads_are_owned_and_bounded`; APIs `save_state()`/`load_state()`; testes de restore no core e worker | Snapshot em memória e restore validados estruturalmente | Não há prova de arquivo persistente neste caminho; manter versões e validar BIOS no snapshot quando integrada |
| Worker / trace | `trace_capture_is_opt_in_and_exposes_bounded_owned_events`; `trace_opt_in_does_not_change_machine_progression`; APIs `set_trace_enabled` e `debug_snapshot` | Captura bounded existente | Capturar antes do primeiro `start()`; monitorar `trace_dropped`; janela atual 2.048 eventos pode não bastar para boots longos |
| ROM real | `in_game_rom_smoke` (opt-in) | 120 frames, instruções, frame RGB888 e não-preto | Não calcula identidade SHA-256 local, não captura trace de boot e não cobre gameplay completo |

## 3. Casos de jogo

| ROM | Evidência existente | Estado atual |
|---|---|---|
| Andre Agassi Tennis (Europe, Brazil) | Auditoria estática anterior e teste sintético SP/mapper | **NOT RUN** nesta baseline; falta ROM local e trace |
| Ace of Aces (Europe, Brazil) | Auditoria anterior centrada em região; ainda sem reprodução | **NOT RUN** nesta baseline; falta ROM local e trace |
| Road Rash | Smoke positivo reportado; PR #913 registra stall separado na animação do título | Boot positivo; temporização da tela de título ainda requer regressão independente |
| Schtroumpfs 2 | Smoke positivo reportado; PR #881 registra The Smurfs: Travel the World (Brazil) (CRC32 `E14C8A1C`, SHA-1 `15b882...`, SHA-256 `f3d493...`) inicializando até gameplay com defeito de ordem de sprites | **NOT RUN** nesta baseline; confirmar se é o mesmo dump antes de reutilizar a evidência |
| SMS VDP Test v1.31 | Fonte identifica `ColdReset -> JP DumpAllRegisters`; a rotina salva CPU, lê I/R e IFF2, alterna $3F `$05->$FF`, lê HC e depois VC; PR #912 adiciona harness de ROM completa | Captura exata da fonte conhecida; validar hash/checksum $20F3 do binário e comparar em T-state equivalente |

## 4. Testes novos realmente necessários

Não duplicar os testes listados na matriz. A cobertura adicional deve concentrar-se em:

1. **Boot contract:** teste explícito dos campos iniciais por perfil (direct cartridge, BIOS, reset, restore), classificando valores documentados, policy do emulador e valores não especificados.
2. **Mapper fail-closed:** introduzir `Unknown/Ambiguous` somente após desenhar a API de identidade; testes com cabeçalhos/assinaturas ausentes, conflitantes e ROMs reais identificadas.
3. **BIOS real:** executar pelo menos uma BIOS SMS1 identificada por hash; capturar primeiro fetch, writes de $3E, efeitos em $C000, inicialização VDP/PSG/I/O e handoff. Não substituir por fixture sintética para declarar compatibilidade.
4. **Trace de ROM:** harness opt-in com caminho de ROM local, tamanho/CRC32/SHA-256, região/modelo, captura bounded antes do primeiro start, checkpoints por blocos de instruções e logs sem versionar ROM.
5. **Games:** reproduzir Andre Agassi e Ace of Aces separadamente; verificar o primeiro CALL/PUSH, inicialização de SP, mapper efetivo, primeira leitura de banco divergente e primeiro write VDP/PSG. Executar Road Rash e Schtroumpfs 2 como regressões; antes de aproveitar PR #881, confirmar se o hash do arquivo local corresponde a The Smurfs: Travel the World (Brazil).
6. **VDP startup screen:** confirmar que o binário local corresponde ao fonte v1.31, validar checksum de cabeçalho $20F3 e comparar o read de HC após a transição TH com o read de VC aproximadamente 24 T-states depois, no mesmo perfil PAL/NTSC. Fixar também o SHA-256 da BIOS se a execução de Ares/Gearsystem for repetida.

## 5. Gates

- Não alterar Z80 compartilhado, VDP ou PSG para acomodar um jogo sem trace que prove a causa.
- Não tratar testes `ignored`, `skipped` ou smoke parcial como PASS geral.
- Não alterar formato de Save State sem análise de versão/compatibilidade.
- Toda correção deve acrescentar um teste que falhe antes e passe depois.
- Execução local do workspace e boot manual no Windows continuam obrigatórios antes de PR/merge.

## 6. Resultado da auditoria de cobertura

A base já tem cobertura temporal profunda de CPU, memória, mapper, VDP, PSG, I/O, worker, WAIT e restore. O gargalo atual não é ausência generalizada de testes unitários. É a falta de evidência de ROM real e de um contrato de boot explícito que distinga hardware documentado de defaults determinísticos do emulador.

A próxima implementação deve ser guiada pelo trace dos dois jogos e por um perfil de boot reproduzível, não por uma reescrita de periféricos.
