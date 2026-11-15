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

## Limites de conclusão

A seleção SMS2 configura o VDP 315-5246 explicitamente, mas não declara concluída a fidelidade integral do SMS2. Permanecem necessárias as validações de sprite evaluation, HBlank/raster writes, portas, interrupções, áudio, Cosmic Spacehead e hardware real.

A implementação de VINT/HINT/VCounter para 315-5246 está sendo tratada separadamente na PR #893; a validação em hardware real continua pendente.
