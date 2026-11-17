# SG-1000 P2.4 — Auditoria da integração desktop

**Data:** 2026-11-17  
**Baseline:** `main` após merge da PR #989  
**Baseline SHA:** `a728c448bb656f19650e88e214a9a723bbcc2713`  
**Branch de trabalho:** `feat/sg1000-desktop-integration-p24-20261117`

## 1. Estado remoto confirmado

A PR #989 foi integrada por squash. O GitHub retornou `merged=true` e o SHA `a728c448bb656f19650e88e214a9a723bbcc2713`; a consulta subsequente confirmou esse mesmo SHA como HEAD da main e a PR como `closed/merged`.

O CI #44096800272 terminou com sucesso nos jobs aplicáveis: workspace regression, Slint/WGPU/Vulkan POC, CPAL output, SDL3 input, PowerShell tooling, documentation hygiene e Rust quality/unit regression. Scheduler, Z80, VDP, PSG e outros domínios não impactados foram skipped e não são contabilizados como executados. O workflow pós-merge não foi localizado para o SHA do squash nesta consulta.

A leitura do endpoint de proteção da branch retornou HTTP 403 (`Resource not accessible by integration`). O merge foi solicitado sem force e com `expected_head_sha`; o próprio endpoint de merge do GitHub confirmou a operação, portanto a proteção não foi contornada.

## 2. Infraestrutura desktop existente

O host de produto está em `native/sega_desktop` e atualmente concentra sua implementação Windows em `src/main.rs` (5.434 linhas na baseline auditada).

Contratos e componentes reutilizáveis já existentes:

- `sega_video_runtime::FrameMailbox` e `VideoFrame`.
- Textura externa importada para a cena Slint por `GraphicsAPI::WGPU30`, usando o device/queue do renderer.
- Conversão RGB888 → RGBA8 e upload com `queue.write_texture`.
- `sega_audio_runtime::AudioSession`, agora compartilhado e reexportado pelo runtime SMS.
- `sega_audio_output::NativeCpalOutput` e `into_audio_session`.
- Polling SDL3 em thread própria e snapshots consumidos pela thread Slint.
- Persistência de preferências e histórico recente de ROMs.

O caminho de apresentação já aceita 256×192, resolução nativa publicada pelo SG-1000. A validação atual também aceita 256×224 e 256×240 por causa dos modos SMS. A conversão RGB888 e o upload WGPU são independentes do console, desde que largura, altura, stride e formato do frame sejam validados.

## 3. Acoplamentos SMS que precisam ser removidos da fronteira de sessão

O host ainda tipa seu estado de sessão como `Option<SmsTemporalWorker>`. Esse tipo é passado diretamente para:

- polling SDL3 e atualização de controles;
- callbacks de Start/Pause/Stop/Reset/StepFrame;
- apresentação/telemetria de frames;
- volume, mute e configuração DSP;
- depuração e trace;
- persistência SRAM;
- troca e encerramento de ROM.

A UI identifica `sms1` e `sms2`; a seleção de sistema está desabilitada durante uma sessão ativa. O texto do menu ainda declara SG-1000 indisponível. O roteamento de ROM cria exclusivamente `SmsTemporalWorker`.

O carregador atual lê arquivo simples ou ZIP, com limites de tamanho e quantidade de entradas. O ZIP aceita apenas uma entrada `.sms` ou `.bin`. O carregamento de bytes não identifica por si só o sistema. A seleção explícita deve determinar o runtime; não usar heurísticas ambíguas de extensão ou enviar bytes ao worker SMS por padrão quando SG-1000 estiver selecionado.

A preparação de cartucho, catálogo de hardware, BIOS convencional, firmware regional e persistência SRM são fluxos SMS. Não devem ser executados para SG-1000 nem reaproveitados como se fossem perfis SG.

## 4. Contratos do worker SG-1000 disponíveis

`Sg1000TemporalWorker` já fornece:

- `new(Vec<u8>)`, `is_running`, `start`, `pause`, `stop`, `reset`;
- `load_rom`, `step_frame`;
- `set_controller`, `set_sprite_limit_enabled`, `request_nmi`;
- `debug_snapshot`, `save_state`, `load_state`;
- `configure_audio_output`, `set_audio_volume`, `set_audio_dsp_config`, `close_audio_output`;
- `attach_video_mailbox`, `detach_video_mailbox`.

A factory de áudio aceita `Box<dyn AudioSession>` e é executada na thread proprietária do worker. A sessão de áudio pode permanecer não-`Send`; somente a factory `Send` cruza a fila de comandos. O host CPAL pode reutilizar o mesmo `NativeCpalOutput` e o mesmo pipeline PCM.

O tipo `ControllerButtons` do SG-1000 é distinto do tipo do SMS, embora possua os mesmos seis campos lógicos. A integração SDL3 deve converter os campos explicitamente no limite do host. O worker SG recebe snapshots por comando coarse-grained `set_controller`; não possui o timestamp de telemetria de `set_controller_at` do SMS. Não adicionar callbacks por instrução/T-state para compensar essa diferença.

A telemetria de pacing e o trace atuais são SMS-específicos. Para SG-1000, usar `Sg1000DebugSnapshot` e métricas disponíveis, ou apresentar explicitamente indisponibilidade de métricas SMS; não fabricar valores.

## 5. Direção de implementação recomendada

1. Criar uma fronteira host-side `DesktopSession` (enum/adapter) com variantes SMS e SG-1000. Não duplicar janela, renderer, mailbox, CPAL ou polling SDL3.
2. Centralizar somente operações realmente comuns: lifecycle, frame-step, mailbox, controller snapshots, áudio e consulta de estado básico.
3. Manter operações SMS específicas (BIOS, região SMS, SRAM, mapper, trace SMS, H2 renderer e revisão do VDP SMS) sob a variante SMS, sem no-op silencioso.
4. Incluir `sega-sg1000-runtime` como dependência Windows do desktop.
5. Fazer o sistema selecionado definir explicitamente o factory de runtime. Validar bytes e preparar o worker SG antes de pausar/destruir a sessão atual.
6. Para troca de sessão, preservar a ordem: construir e validar o próximo worker pausado; anexar mailbox ao próximo sem dois produtores simultâneos; pausar/salvar/destacar a sessão anterior; confirmar a troca; iniciar a nova sessão. Em qualquer falha anterior ao commit da troca, restaurar a sessão anterior.
7. Reutilizar `FrameMailbox → WGPU` e `AudioSession → CPAL`; manter o caminho de vídeo 256×192 comum.
8. Manter o ciclo de entrada SDL3 em snapshots. Para SG-1000, converter `ControllerButtons` SMS para o tipo SG por campos e enviar `set_controller`.
9. Desabilitar/indicar como não suportadas as operações exclusivamente SMS enquanto SG-1000 estiver ativo, sem permitir que alcancem o worker errado.
10. Adicionar testes de roteamento de sistema, falha de ROM inválida preservando a sessão, lifecycle multi-sistema, mailbox, áudio factory/thread ownership, entrada e regressão SMS.

## 6. Limites desta auditoria

Esta etapa é uma auditoria de arquitetura e contratos, não uma declaração de integração funcional. Nenhuma alteração de hardware, Z80, MachineScheduler, VDP, PSG ou temporização é necessária para a primeira fatia P2.4. O Z80 compartilhado e o Scheduler permanecem intocados.

A seleção de sistema deve ser explícita e persistida. Uma ROM `.bin` não contém necessariamente informação suficiente para identificar o console sem catálogo/hash confiável; portanto, a extensão isolada não deve sobrepor a escolha do usuário nem servir como prova de compatibilidade.
