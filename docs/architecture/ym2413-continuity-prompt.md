# Prompt de continuidade — YM2413 integrado ao Master System japonês

## Fase e autoridade

- Repositório: https://github.com/leonardo201800478/sega-emulator
- A trilha SMS-FM começa após a PR #992 estar MERGED e o workflow pós-merge no SHA de `main` concluir SUCCESS. Baseline confirmado: `main` em `b82e0df4c9840b2cd59f090278b907cbb321869f`; workflow pós-merge `44186646679`.
- YM2413 integrado é uma característica do perfil de hardware Master System japonês, não uma nova plataforma e não a FM Sound Unit externa do Mark III. Esta trilha SMS pode ocorrer antes do Game Gear sem alterar a ordem das plataformas. A fase geral de acessórios P4 continua após Game Gear.
- Runtime 100% Rust; GUI Slint 1.18.x; vídeo WGPU 30.x/Vulkan; áudio físico CPAL 0.18.2; SDL3/input runtime.
- Toda alteração em branch/PR própria. Não fazer commit direto na main, force-push, reset destrutivo ou merge sem nova autorização explícita do usuário.
- Não usar Python, Qt, PyO3 ou Maturin no runtime, tooling obrigatório, CI ou frontend.

## Objetivo e escopo

Implementar o Yamaha YM2413 (OPLL) integrado ao perfil de hardware Japanese Master System como subsistema FM opcional, independente da região lógica do software e coexistente com o SN76489. Não implementar o Mark III FM Sound Unit externo.

- Configuração Auto/On/Off.
- Auto usa catálogo de compatibilidade por título/revisão; somente ativa FM quando houver driver/dados comprovados.
- On/Off são overrides explícitos de presença/saída, sem alterar região/BIOS do sistema.
- Não presumir que toda ROM com suporte a SMS possua driver ou dados FM.
- Preservar PSG e FM como estados independentes; mute de saída não interrompe clocks internos sem evidência física.
- SG-1000 não recebe YM2413; Game Gear permanece trilha de sistema próprio.

## Leitura obrigatória

- `docs/architecture/ym2413-fm-model.md`
- `docs/hardware/sms-expansion-accessories-roadmap.md`
- `docs/architecture/master-system-code-model.md`
- `docs/audio/modern-hifi-audio-roadmap.md`
- `docs/architecture/native-audio-runtime.md`
- `docs/architecture/adr/0002-machine-temporal-scheduler.md`

Consultar manual Yamaha, SMS Power e referências independentes pinadas por commit/licença. Separar DOCUMENTADO, MEDIDO, DERIVADO, INFERIDO e NOT_VERIFIED.

## Arquitetura auditada em main

- `SmsTemporalCoordinator` registra Z80 (ID 0), PSG (ID 1) e VDP (ID 2). O worker SMS usa `SMS_TEMPORAL_MASTER_TICKS_PER_TSTATE = 1`; esse tick lógico é um T-state Z80. O manual Yamaha especifica XIN de 3.579.545 Hz e o MAME modela o YM2413 japonês com XTAL(10.738.635)/3, mas isso não prova fonte elétrica/fase comum em todas as placas. A escolha entre reutilizar o `ClockAdvance` do PSG ou registrar um domínio FM separado permanece **PENDENTE** até evidência do perfil físico. Manter um único `MachineScheduler`.
- `SmsTemporalBus` aplica efeitos de `PortRead`/`PortWrite` no deadline de conclusão. $F0/$F1/$F2 ainda não têm decode FM. A leitura atual de $F2 cai no `SmsIo::read_port` e é tratada como leitura do controle DC (`read_dc`) pelo intervalo genérico $C0–$FE; não é readback YM2413/C-Sync. A escrita em $F2 é ignorada pelo caminho genérico.
- O SN76489 divide clock de entrada por 16 e produz PCM na taxa interna configurada de 48 kHz.
- YM2413 completa 18 slots × 4 clocks = 72 clocks por rotação; 3.579.545/72 ≈ 49.716,04 Hz é taxa interna derivada, não taxa de host.
- `AudioMixer` aceita múltiplas fontes alinhadas; `NativeAudioSession::process_samples` atualmente envia uma única fonte por chamada. Não chamar separadamente para PSG e FM: isso concatena blocos, não mistura.
- A sessão de áudio usa uma taxa de entrada única de 48 kHz. Será necessário alinhar/resamplear FM deterministicamente para a taxa comum, preservando fase e latência.
- `DesktopPreferences` ainda não possui modo FM. Preferências futuras devem ser aplicadas por comando coarse-grained ao worker; nunca implementar hardware/timing na GUI.
- Snapshot atual não contém estado FM. Versionar estado do bus/machine ao integrar. Só manter o esquema do coordenador se a decisão de clock permitir reutilizar um domínio existente; um domínio FM próprio exigirá versionar e validar o snapshot do coordenador.

## Contratos técnicos confirmados

- Yamaha YM2413 Application Manual: clock nominal 3.579.545 Hz; 12 clocks após escrita de endereço antes de dados; 84 clocks após escrita de dados antes da próxima escrita.
- $F0: latch/endereço, máscara de seis bits.
- $F1: dados.
- $F2: modo de saída: $00 PSG somente; $01 FM somente; $02 ambos silenciados; $03 ambos habilitados. Readback Japanese SMS: D7/D6/D5 recebem contador C-Sync bits 11/7/3; D4–D2 zero; D1–D0 latch de controle. Implementar contador somente após modelar C-Sync e reset/fase.
- As saídas MO/RO e a rede analógica do SMS japonês não devem ser presumidas como uma soma mono ideal. Preservar canais separados até a rede de soma/ganho ser confirmada.
- MAME é referência de implementação, não prova física. `emu2413` MIT é diferencial comportamental sample-oriented; não cycle-accurate. Nuked-OPLL e `emu198x_yamaha_ym2413` são GPL-2.0-or-later e não devem ser copiados/dependidos no runtime MIT.

## Etapas incrementais

1. Auditoria e matriz de evidências no SHA atual.
2. Fundação pura de latch, registradores, clock/slot e snapshot/restore (sem síntese/áudio).
3. Núcleo de síntese Rust: PG, EG, Log-Sine/EXP, feedback, AM/PM e pipeline sequencial.
4. Presets originais, patch de usuário e Rhythm Mode; vetores determinísticos.
5. Diferenciais sample-level com emu2413 MIT e vetores independentes; sem alegar precisão cycle-accurate sem evidência.
6. Adaptador temporal e portas SMS no deadline; reutilizar domínio PSG.
7. $F2 e contador C-Sync por perfil físico, com testes de reset/readback.
8. Alinhamento de fontes, mixer comum, resampling e latência.
9. Auto/On/Off, catálogo título/revisão, override e persistência.
10. Save State/replay, regressões SMS1/SMS2 e SG-1000 e CI no SHA exato.

## Aceite

Testes determinísticos para registradores, operadores/envelopes, temporização e writes; mixagem simultânea PSG+FM; snapshots/restores e equivalência contínua/em blocos; regressões SMS1/SMS2 e SG-1000; gates Rust/Windows aplicáveis no HEAD exato. SKIPPED, ausente, cancelado, pending e NOT RUN nunca são PASS.

Não declarar conclusão por gerar um tom plausível, compilar ou tocar uma ROM. Não afirmar compatibilidade sem catálogo e evidência. Não exigir E8/ZEXDOC/ZEXALL como gate desta trilha.
