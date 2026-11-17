# SG-1000 P2 — reconciliação pós P2.4

**Data:** 2026-11-17  
**Baseline:** `main` `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`  
**Escopo:** reconciliação direcionada do runtime SG-1000, worker, integração desktop, documentação normativa e CI. Não representa auditoria integral dos 285 documentos Markdown.

## 1. Resultado executivo

A reconciliação documental #980 foi integrada em `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434`. A main atual contém seis commits posteriores e não está atrás desse merge. Em seguida, o SG-1000 avançou na arquitetura existente:

| Fatia | PR | Resultado integrado |
|---|---|---|
| P2.2 — máquina temporal | [#988](https://github.com/leonardo201800478/sega-emulator/pull/988) | `be883792025de3db7eaf4e7b8243293ea90b7e06` |
| P2.3 — worker Rust | [#989](https://github.com/leonardo201800478/sega-emulator/pull/989) | `a728c448bb656f19650e88e214a9a723bbcc2713` |
| P2.4 — integração desktop | [#990](https://github.com/leonardo201800478/sega-emulator/pull/990) | `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315` (HEAD atual) |

**Conclusão:** a arquitetura temporal, o worker e a seleção/roteamento desktop estão integrados em software. **SG-1000 P2 permanece aberto** por falta de evidência in-game dos perfis de cartucho e de validação física da sessão Windows. Não iniciar P3/Game Gear ainda.

## 2. Arquitetura ativa na main

| Documento/módulo | Símbolos e contrato | Testes/evidência | Workflow / SHA |
|---|---|---|---|
| `native/sg1000_core/src/lib.rs` | Exporta `Sg1000Machine` temporal; `machine` legado está sob `cfg(test)` | O caminho de produto não usa o executor atômico legado | PR #988 / merge `be883792`; push #44091248478 |
| `native/sg1000_core/src/temporal_machine.rs` | `Sg1000Machine`, `begin_instruction`, `schedule_pending_bus`, `dispatch_next`, `step_instruction`, `step_frame`, `export_state`, `restore_state` | NOP Scheduler; memória; PortRead no deadline; M1/WAIT; restore de M1 pendente; contínuo vs blocos; VDP/PSG; frame/áudio; INT; comparação básica com oracle; rejeição de estado inconsistente | PR #988 head `198ef8ad`; CI #44090848768; merge/push `be883792` / #44091248478 |
| `native/sg1000_core/src/temporal_bus.rs` | `Sg1000TemporalBus` aceita OpcodeFetch, MemoryRead/Write, PortRead/Write e InterruptAcknowledge; efeitos são aplicados no timestamp de conclusão | `port_read_samples_controller_at_deadline_and_advances_all_device_domains`; `port_read_samples_controller_after_request_start_at_completion`; `vdp_status_port_read_observes_vblank_at_completion_deadline`; `psg_port_write_is_committed_once_and_survives_completion_restore`; writes VDP no deadline | PR #988 head `198ef8ad`; CI #44090848768; merge/push `be883792` / #44091248478 |
| `native/sg1000_core/src/temporal.rs` | `Sg1000TemporalCoordinator` registra Z80, VDP e PSG no mesmo MachineScheduler; razão racional PSG | avanço conjunto, razão PSG exata e restore de deadline/fase | PR #988 / push #44091248478 |
| `native/sg1000_core/src/temporal_m1.rs` | M1 T1/T2/Tw/T3/T4 retomável; opcode amostrado em T3 | testes de M1/WAIT e restore no meio de WAIT | PR #988 / push #44091248478 |
| `rust/runtime/sg1000-runtime/src/worker.rs` | `Sg1000TemporalWorker`, comandos coarse-grained, save-state vinculado a ROM/perfil, AudioSession e FrameMailbox | frame/mailbox; controller snapshot; save-state roundtrip; ROM inválida preserva máquina; factory/lifecycle de áudio na thread proprietária | PR #989 head `cff6b217`; CI #44096800272; merge `a728c448` |
| `rust/runtime/sg1000-runtime/src/cartridge.rs` | `identify_cartridge_profile`, `memory_config_for_rom` | hashes conhecidos, mapas distintos e fallback de ROM desconhecida | PR #990 head `79357d23`; CI #44114863234; merge `0a7b24fe` |
| `native/sega_desktop/src/desktop_session.rs` | `DesktopSession` roteia SMS/SG-1000 e reutiliza mailbox/áudio/input | IDs explícitos, dispatch de lifecycle, tradução de controles e publicação de frames | PR #990 head `79357d23`; CI #44114863234; merge/push `0a7b24fe` / #44135959914 |

### Perfis de RAM no resolver

- Type-A: 8 KiB externos em `2000h–3FFFh`.
- Type-B: 8 KiB internos em `C000h–FFFFh`.
- Clones SG-1000 II catalogados: 2 KiB internos em `C000h–FFFFh`.
- Othello: 2 KiB externos espelhados em `8000h–BFFFh`.
- The Castle (Japan): 8 KiB externos espelhados em `8000h–BFFFh`.
- ROM sem hash conhecido mantém o mapa padrão de 1 KiB; tamanho/extensão não ativam expansão.

Os testes do resolver comprovam identificação e mapas, não gameplay desses títulos.

## 3. CI por SHA

| Escopo | SHA | Workflow | Resultado observado |
|---|---|---|---|
| #980 — HEAD da PR | `c83d459ba710785a86ae28632b2c0bcc98e3a05f` | [#43959831347](https://github.com/leonardo201800478/sega-emulator/actions/runs/43959831347) | Change impact, Documentation patch hygiene e Rust quality/unit PASS; demais jobs SKIPPED |
| #980 — merge | `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434` | [#43962737265](https://github.com/leonardo201800478/sega-emulator/actions/runs/43962737265) | Mesmos três gates PASS; jobs de runtime SKIPPED |
| P2.2 — HEAD da PR | `198ef8ad00e03d2db2549d51f42a49b109d340da` | [#44090848768](https://github.com/leonardo201800478/sega-emulator/actions/runs/44090848768) | 8 checks PASS, 8 SKIPPED; Scheduler, Z80, SMS1 VDP, input, SDL3 e workspace incluídos nos PASS |
| P2.2 — merge | `be883792025de3db7eaf4e7b8243293ea90b7e06` | [#44091248478](https://github.com/leonardo201800478/sega-emulator/actions/runs/44091248478) | 8 checks PASS, 8 SKIPPED |
| P2.3 — HEAD da PR | `cff6b2179614fab84008d729ab0f6e9d9f9f9ecf` | [#44096800272](https://github.com/leonardo201800478/sega-emulator/actions/runs/44096800272) | Workspace, CPAL, Slint/WGPU/Vulkan, SDL3, PowerShell, docs, Rust quality e SMS runtime Release PASS; demais jobs SKIPPED |
| P2.3 — merge | `a728c448bb656f19650e88e214a9a723bbcc2713` | [#44103751036](https://github.com/leonardo201800478/sega-emulator/actions/runs/44103751036) / [#44103751135](https://github.com/leonardo201800478/sega-emulator/actions/runs/44103751135) | Push workflows concluídos SUCCESS |
| P2.4 — HEAD da PR | `79357d23fccdebf553c01b104704241ab6db5a5e` | [#44114863234](https://github.com/leonardo201800478/sega-emulator/actions/runs/44114863234) | Workspace, Slint/WGPU/Vulkan, SDL3, PowerShell, docs, Rust quality e change impact PASS; CPAL/PSG e domínios não impactados SKIPPED |
| P2.4 — main HEAD | `0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315` | [#44135959914](https://github.com/leonardo201800478/sega-emulator/actions/runs/44135959914) | 7 PASS, 9 SKIPPED; CPAL output e PSG foram SKIPPED |

Nenhum log de falha nova foi aberto: as falhas conhecidas de #921 e #863 pertencem aos SHAs antigos, não houve novo check no HEAD atual dessas branches e as causas já estão registradas.

## 4. Estado das PRs acompanhadas

- #980: merged; CI documental PASS no HEAD e no merge. Não foi mergeada por esta execução.
- #988/#989/#990: merged e compõem P2.2/P2.3/P2.4.
- #981: fechada sem merge; sua fatia temporal de I/O foi supersedida pela implementação P2.2 de #988.
- #533: Draft permanente; 7 commits à frente / 18 atrás da main atual.
- #921: Draft; 52 à frente / 189 atrás. A ROM SmsVdpTest v1.31 ainda falhou RunTest32/34 no HEAD `2dae4329`; não houve nova execução nesse SHA.
- #878: Draft; 24 à frente / 273 atrás; CI antigo não valida a main atual.
- #863: Draft; 3 à frente / 314 atrás; falhas antigas de rustfmt/workspace permanecem sem nova execução.
- #983: documentação de referência visual, 3 à frente / 5 atrás; CI documental PASS, jobs de runtime SKIPPED.
- #978, #974, #769, #881 e #913 permanecem fechadas/supersedidas conforme a reconciliação; não reabrir nem cherry-pickar sem nova evidência.
- #928: merged; validação física de BIOS real continua pendente.

## 5. Limites e riscos remanescentes

1. **Gameplay dos cartuchos:** não há evidência in-game para Type-A, Type-B, clone 2 KiB, Othello e The Castle. O resolver/hash e os mapas têm testes unitários.
2. **Vídeo/áudio/input físicos:** o CI #990 na main passou WGPU/Vulkan POC e SDL3, mas SKIPPED CPAL output. Build/POC não prova apresentação sustentada; SDL3 CI não prova hotplug/controle físico; não há medição de underruns nesta execução.
3. **Clock:** 3.579545 MHz é perfil nominal/provisório; razão racional impede drift de conversão, mas não valida oscilador/divisor físico. VDP usa T-states do modelo, não clock de pixel/fetch medido.
4. **WAIT:** os estados Tw são exercitados por fixture de software; WAIT elétrico de placa permanece NOT_VERIFIED.
5. **Replay:** snapshots/restore e equivalência contínua/em blocos estão cobertos. Não foi localizado harness dedicado de replay determinístico de log de inputs/eventos.
6. **Oracle legado:** a comparação temporal com a máquina atômica é seletiva e básica; ela permanece somente em `cfg(test)`, não é segunda CPU/runtime.
7. **Documentação:** esta auditoria é direcionada ao P2 e não declara leitura integral dos 285 documentos.

## 6. Próximos gates para fechar P2

1. Executar ROMs legalmente obtidas/homebrew com hashes catalogados, cobrindo os perfis de RAM e registrar inicialização, gameplay, leituras/escritas, estabilidade e estado final no SHA testado.
2. Validar a build Windows atual em sessão real: seleção/troca de SMS e SG-1000, load inválido sem perda da sessão, start/pause/stop/reset/step, save/load, vídeo WGPU/Vulkan sustentado, áudio CPAL audível e sem underruns, controles SDL3 reais e hotplug.
3. Reexecutar CPAL/PSG e regressões de workspace no SHA candidato final quando o escopo de código exigir; SKIPPED nunca satisfaz gate.
4. Acrescentar replay de log determinístico somente se definido como requisito separado; não confundir com save-state/restore.
5. Manter SMS1/SMS2 como regressão compartilhada e não alterar Z80/VDP/Scheduler por hipótese. Só corrigir após primeira divergência reproduzível.
6. Não iniciar Game Gear/P3 até que os gates funcionais e físicos de P2 estejam classificados e aceitos.

