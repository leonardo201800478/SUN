# Perfis SMS no desktop — seleção explícita de hardware

## Seleção do sistema

O menu **Sistema** permite selecionar explicitamente:

- **Master System / SMS1:** ASIC VDP Sega 315-5124.
- **Master System II / SMS2:** ASIC VDP Sega 315-5246.

A escolha é persistida em `slint_ui_settings.txt`. Preferências antigas sem a chave `sms_system` permanecem compatíveis e usam SMS1 como padrão legado.

A interface distingue:
- **Hardware em execução:** perfil já construído para a ROM atual.
- **Hardware selecionado:** perfil que será usado no próximo carregamento de ROM.

Alterar a seleção durante a execução não troca o hardware da máquina atual nem reinicializa o Scheduler. O novo perfil só é aplicado quando outra ROM é carregada.

## Região e ASIC são independentes

A região selecionada controla território, padrão de vídeo e clock:

- USA: NTSC — 3.579.545 Hz.
- Brasil: PAL-M — 3.579.545 Hz.
- Europa: PAL — 3.546.895 Hz.
- Japão: NTSC — 3.579.545 Hz.

O modelo/ASIC selecionado controla a revisão do VDP:

- SMS1 → `SmsVdpRevision::Sega3155124`.
- SMS2 → `SmsVdpRevision::Sega3155246`.

`sms_temporal_hardware_profile(region, revision)` resolve as duas dimensões sem derivar o ASIC da região, do CRC ou da extensão da ROM.

## Propagação até o runtime

`SmsWorkerSettings` carrega a revisão escolhida até `SmsTemporalWorker::new_with_hardware_profile_and_cartridge_sram_size`.

O worker preserva a revisão ao executar:
- Reset;
- Power Off / reconstrução da máquina;
- Load ROM no worker;
- reconstrução que preserva SRAM do cartucho.

Os construtores antigos permanecem compatíveis com o perfil SMS1. O desktop usa o construtor explícito.

## Sistemas ainda não habilitados no desktop

- **SG-1000 / TMS9918:** existe `native/sg1000_core`, mas não há worker/runtime temporal integrado ao launcher desktop.
- **Game Gear:** existe `native/gg_core`, mas não há worker/runtime temporal integrado ao launcher desktop.
- Mega Drive, Sega CD e 32X: não habilitados neste host.

Esses sistemas permanecem visíveis como indisponíveis, sem serem apresentados como funcionalidade concluída. A existência de um core isolado não equivale a uma máquina integrada ao desktop.

## Validação por ROM

O teste opt-in `rust/runtime/sms-runtime/tests/in_game_rom_smoke.rs` agora aceita:
- `SEGA_SMS_SYSTEM=sms1|sms2`;
- `SEGA_SMS_TEST_REGION=usa|brazil|europe|japan`;
- `SEGA_SMS_TEST_FRAMES` para ampliar a duração;
- `SEGA_SMS_EXPECT_HEIGHT=224` para exigir que o modo estendido seja publicado.

Para Cosmic Spacehead, o smoke test pode ser executado isoladamente com SMS2/Europa e altura esperada de 224 linhas. Ele valida execução, publicação de frames, dimensões e ausência de frames totalmente pretos; não simula navegação de menus, gameplay, áudio ou colisões. A validação funcional completa continua sendo uma etapa separada.

## Compatibilidade de cartuchos

A seleção explícita define a máquina antes do carregamento, mas a base atual de metadados do cartucho não declara um campo geral de requisitos mínimos de ASIC/revisão. A compatibilidade SMS1/SMS2 ainda não é validada por uma matriz de requisitos na abertura da ROM. A região não será usada como substituto dessa informação.

Os cartuchos SMS comuns compartilham mapper e mapa de memória; recursos exclusivos como 224/240 linhas dependem do VDP selecionado. A detecção de um jogo que tente ativar um modo estendido incompatível com SMS1 ainda precisa de um diagnóstico explícito em runtime. Não há troca automática de hardware.

## Limites de conclusão

A seleção SMS2 configura o VDP 315-5246 explicitamente, mas não declara concluída a fidelidade integral do SMS2. Permanecem necessárias as validações de sprite evaluation, HBlank/raster writes, portas, interrupções, áudio, Cosmic Spacehead e hardware real.

A implementação de VINT/HINT/VCounter e publicação de frame para 315-5246 foi integrada pela PR #893. A validação do timing em hardware real continua pendente; HBlank/raster writes e sprite evaluation SMS2 permanecem abertos.
