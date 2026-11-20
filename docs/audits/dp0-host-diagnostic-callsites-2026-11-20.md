# DP0 — inventário dos 43 callsites de diagnóstico do desktop

- **Data:** 2026-11-20
- **Escopo:** `native/sega_desktop/src/main.rs`, módulo Windows host.
- **Branch de auditoria:** `fix/dp0-unified-debugger-consent-and-passive-trace-removal`.
- **Autoridade de consentimento:** `LoggingHandle::is_enabled`, consultada por `host_diagnostics_enabled()`.
- **Total:** 43 callsites classificados.

## Política de emissão

Todos os callsites são emitidos por `diagnostic_event!`, que consulta o consentimento central e usa o nível e o target definidos no próprio callsite. O macro não usa `format!`, não serializa campos em um único `message` e não altera o estado do runtime.

Os dois eventos de telemetria periódica foram rebaixados para `TRACE`:
- `sega::video::frame_pacing / host_metrics`: métricas de wakeups, mailbox, upload, worker, Scheduler, orçamento de frame e latência.
- `sega::audio::health / output_metrics`: fila, consumo, underruns, descartes, flushes e taxas de áudio.

Erros recuperáveis usam `WARN`; falhas que impedem inicialização/importação/finalização usam `ERROR`; transições operacionais usam `INFO`; estados opcionais ou teardown usam `DEBUG`.

## Inventário

| # | Target | Nível | Evento | Finalidade |
|---:|---|---|---|---|
| 1 | `sega::input::sdl3` | ERROR | `poll_thread_panicked` | Thread de polling encerrou por panic |
| 2 | `sega::input::sdl3` | ERROR | `initialization_failed` | Inicialização SDL3 falhou |
| 3 | `sega::input::sdl3` | WARN | `poll_failed` | Polling falhou; host pode manter fallback |
| 4 | `sega::input::sdl3` | WARN | `thread_start_failed` | Thread de polling não iniciou |
| 5 | `sega::input::sdl3` | WARN | `pause_nmi_failed` | NMI/pausa por entrada falhou |
| 6 | `sega::input::sdl3` | WARN | `controller_update_failed` | Atualização do controle P1 falhou |
| 7 | `sega::input::sdl3` | WARN | `controller_update_failed` | Atualização do controle P2 falhou |
| 8 | `sega::preferences` | WARN | `recent_roms_save_failed` | Persistência da lista recente falhou |
| 9 | `sega::video::lifecycle` | INFO | `rendering_setup` | Backend/adaptador de renderização selecionado |
| 10 | `sega::video::frame_notify` | WARN | `renderer_recovery_failed` | Recuperação do renderer falhou |
| 11 | `sega::video::lifecycle` | ERROR | `texture_import_failed` | Importação de textura externa falhou |
| 12 | `sega::video::lifecycle` | DEBUG | `rendering_teardown` | Encerramento da configuração de renderização |
| 13 | `sega::video::lifecycle` | ERROR | `rendering_setup_failed` | API gráfica solicitada não é WGPU 30 |
| 14 | `sega::video::frame_pacing` | TRACE | `host_metrics` | Telemetria de frame pacing e apresentação |
| 15 | `sega::audio::health` | TRACE | `output_metrics` | Telemetria de saúde da fila de áudio |
| 16 | `sega::video::frame_notify` | WARN | `enqueue_failed` | Publicação/notificação de frame falhou |
| 17 | `sega::audio::output` | WARN | `renderer_selection_failed` | Seleção do renderer falhou; PSG permanece fallback |
| 18 | `sega::audio::output` | DEBUG | `skipped` | Saída física ignorada no modo video-only |
| 19 | `sega::audio::output` | INFO | `cpal_output_opened` | CPAL abriu saída física |
| 20 | `sega::audio::output` | WARN | `cpal_unavailable` | CPAL indisponível; execução sem saída física |
| 21 | `sega::sram::profile` | WARN | `diagnostic_override` | Perfil SRAM foi substituído por override diagnóstico |
| 22 | `sega::sram::profile` | WARN | `unknown` | Perfil SRAM não reconhecido |
| 23 | `sega::sram::profile` | INFO | `verified` | Perfil SRAM validado |
| 24 | `sega::audio::output` | DEBUG | `skipped` | Áudio físico não suportado para SG-1000 nesse caminho |
| 25 | `sega::audio::output` | INFO | `sg1000_cpal_output_opened` | CPAL abriu saída física para SG-1000 |
| 26 | `sega::audio::output` | WARN | `sg1000_cpal_unavailable` | CPAL indisponível para SG-1000 |
| 27 | `sega::sram::save` | INFO | `profile_persisted` | Persistência SRAM concluída; registra capacidade do perfil |
| 28 | `sega::machine::region` | WARN | `catalog_error` | Catálogo de região falhou; fallback USA aplicado |
| 29 | `sega::machine::bios` | INFO | `validated` | BIOS/firmware validado |
| 30 | `sega::machine::control` | WARN | `action_failed` | Ação coarse-grained Start falhou |
| 31 | `sega::machine::control` | WARN | `action_failed` | Ação coarse-grained Pause falhou |
| 32 | `sega::machine::control` | WARN | `action_failed` | Ação coarse-grained Reset falhou |
| 33 | `sega::machine::control` | WARN | `action_failed` | Ação coarse-grained Step Frame falhou |
| 34 | `sega::input::keyboard` | WARN | `controller_press_failed` | Pressionamento de controle via Slint falhou |
| 35 | `sega::input::keyboard` | WARN | `nmi_failed` | NMI via teclado falhou |
| 36 | `sega::input::keyboard` | WARN | `controller_release_failed` | Liberação de controle via Slint falhou |
| 37 | `sega::input::keyboard` | WARN | `focus_release_failed` | Liberação de entradas ao perder foco falhou |
| 38 | `sega::machine::firmware` | INFO | `region_selected` | Região/firmware selecionado |
| 39 | `sega::preferences` | WARN | `save_failed` | Persistência de preferências falhou |
| 40 | `sega::video::sprites` | WARN | `sprite_limit_toggle_failed` | Alteração do limite de sprites falhou |
| 41 | `sega::sram::save` | WARN | `pause_before_exit_failed` | Pausa antes de sair falhou |
| 42 | `sega::sram::save` | ERROR | `final_save_failed` | Persistência final de SRAM falhou |
| 43 | `sega::app` | ERROR | `event_loop_failed` | Event loop do host encerrou com erro |

## Contagem por nível

| Nível | Quantidade |
|---|---:|
| ERROR | 6 |
| WARN | 25 |
| INFO | 7 |
| DEBUG | 3 |
| TRACE | 2 |
| **Total** | **43** |

## Conclusão e limites

- Os 43 callsites são host-side e permanecem fora do caminho de execução por instrução do Z80, VDP, PSG e Scheduler.
- Todos são condicionados pelo consentimento central; sem consentimento, nenhum evento é emitido pelo macro.
- Os dois eventos periódicos são TRACE e mantêm campos numéricos/booleanos estruturados.
- Nenhum evento inclui ROM, BIOS, SRAM, caminho pessoal completo ou payload de memória.
- Este inventário não classifica logs de testes, exemplos, CLI e build scripts; esses sinks são tratados separadamente por target.
