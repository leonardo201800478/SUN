# Prompt de continuidade — YM2413 / FM Sound Unit no Sega Emulator

## Fase e autoridade

- Repositório: https://github.com/leonardo201800478/sega-emulator
- Fase P4: iniciar somente após SMS1/SMS2, SG-1000 e Game Gear.
- Runtime 100% Rust; GUI Slint 1.18.x; vídeo WGPU 30.x/Vulkan; áudio físico CPAL 0.18.2; SDL3/input runtime.
- Não usar Python, Qt, PyO3 ou Maturin no runtime, tooling obrigatório, CI ou frontend.
- GitHub main/commits/testes/workflows são a fonte da verdade. Toda alteração em branch/PR. Sem commit direto, force-push ou merge sem autorização explícita.

## Objetivo

Implementar o Yamaha YM2413 (OPLL) como expansão FM do Mark III/Master System, preservando SN76489 e integrando ambos ao MachineScheduler, barramento SMS, mixer, resampler e Save States existentes.

## Leitura obrigatória

- `docs/architecture/ym2413-fm-model.md`
- `docs/hardware/sms-expansion-accessories-roadmap.md`
- `docs/architecture/master-system-code-model.md`
- `docs/audio/modern-hifi-audio-roadmap.md`
- `docs/z80-completion-scorecard.md`
- `docs/z80-dd-fd-semantic-matrix.md`

A especificação atual contém hipóteses, divergências e itens pendentes. Separar DOCUMENTADO, MEDIDO, INFERIDO e NOT_VERIFIED. Consultar manual Yamaha, SMS Power e referências independentes pinadas por commit/licença.

## Arquitetura

- Hardware, portas, clocks, síntese, mixer e resampler em Rust.
- MachineScheduler existente é autoridade temporal única. Não criar Scheduler/domínio FM paralelo quando a razão de clock for compartilhada.
- Não duplicar CPU, barramento, PSG, worker, mixer ou pipeline.
- Preservar estado interno de PSG e FM mesmo quando uma fonte estiver silenciada, salvo evidência física contrária.
- Misturar fontes PSG/FM alinhadas numa única operação por lote.
- GUI não implementa chip, portas, ciclos ou timing.
- Save State inclui registradores, fases, contadores, slots, operadores, envelopes, LFOs, pipeline, latch, roteamento e fase do mixer/resampler.

## Perfis de hardware

1. Japanese Master System: YM2413 integrado; $F2 controla FM/PSG e pode permitir ambos simultaneamente.
2. Mark III + FM Sound Unit externo: perfil próprio; roteamento/controle difere e FM/PSG podem ser mutuamente exclusivos.
3. SMS exportação/TecToy: não presumir FM de fábrica.
4. Mods de terceiros: perfis separados, nunca comportamento universal.

Portas: $F0 latch/endereço; $F1 dados; $F2 detecção/controle dependente do perfil.

## Síntese e integração

O modelo `ym2413-fm-model.md` define PG, EG, Log-Sine/EXP, feedback, AM/PM, instrumentos, Rhythm Mode, slots e pipeline. Não inventar tabelas, seeds, shifts, truncamentos ou aproximações para substituir medições.

- Auditar clock do PSG e avanços do Scheduler.
- Aplicar escritas FM no timestamp de conclusão da transação Z80.
- Reconciliar taxa YM2413 (~49.716 Hz nominal com clock 3.579.545/72) com PSG/resampler.
- Determinar taxa comum, fase e latência determinísticas.
- Reconciliar MO/RO e circuito de soma/ganho do perfil SMS japonês; não transplantar circuitos de outro sistema.
- $F2 não deve suspender síntese interna sem evidência.

## Aceite

Vetores diferenciais de síntese/envelope/instrumentos/ritmo/slots; testes $F0/$F1/$F2 por perfil; reset; snapshots/restore; mistura PSG/FM alinhada; equivalência contínua/em blocos; regressão SMS1/SMS2, SG-1000 e Game Gear; CI aplicável verde no HEAD exato. SKIPPED nunca é PASS.

Não declarar conclusão por gerar tom plausível, compilar ou tocar uma ROM. Auditar main, especificação, fontes e módulos reais antes de implementar.
