# Controller Input Backends

## Estado atual — Slint + SDL3

O caminho ativo de input físico é Rust e está integrado em primeira fatia ao desktop Slint.

1. `Sdl3Backend` carrega a ABI oficial SDL3 no Windows.
2. Gamepad API fornece controles padronizados; Joystick API preserva eixos, botões e hats brutos.
3. `NativeInputManager` mantém handles, lê estados, processa hotplug e produz ações semânticas.
4. O host Slint coleta teclado por eventos nativos e encaminha snapshots em fronteiras coarse-grained.
5. O runtime SMS recebe atualizações via `SmsTemporalWorker::set_controller_at`; o Scheduler não faz polling de host.

## Integração e compatibilidade

Implementado:
- Gamepads mapeados via SDL3 Gamepad API.
- Joysticks genéricos via SDL3 Joystick API.
- Polling no limite do host, sem callbacks por instrução.
- Hotplug observado pelo backend.
- Primeira atribuição P1/P2 por ordem da sessão.
- Teclado nativo Slint.

Pendente:
- Testes físicos de controles, hotplug/reconexão e remoção de P1.
- Identidade persistente (`physical_key` ainda não disponível em todos os descritores).
- Seleção manual, múltiplos dispositivos por jogador e remapeamento persistente.
- Calibração analógica e perfis por dispositivo.
- Validação física de Start/NMI e Pause/NMI.
- Rumble, force feedback, sensores e relatórios HID específicos.
- Backends Linux/macOS e GameInput complementar.

## Fronteira arquitetural

A GUI não implementa protocolos elétricos, clocks ou registradores. O host coleta estado físico e o runtime adapta snapshots às portas do console. Não chamar APIs de host dentro de instruções, barramentos ou Scheduler. Não criar polling por instrução, runtime paralelo ou ponte Qt/PyO3.

O comportamento de cada console (SMS, SG-1000, Game Gear e sistemas futuros) permanece em adaptadores Rust específicos, sem duplicar backend físico.
