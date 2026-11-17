# Native Input Runtime

## Objetivo e fronteira

O crate `sega-input-runtime` é a camada host-independent de entrada compartilhada pelos cores Sega. Ele normaliza dispositivos físicos, identidade de sessão, perfis e ações lógicas; não implementa comportamento elétrico do console.

## Caminho ativo

- GUI/host: Slint 1.18.x.
- Teclado: eventos nativos Slint enviados em snapshots/coarse-grained ao runtime.
- Gamepads: `Sdl3Backend` em Rust por ABI oficial SDL3; Gamepad API para controles mapeados e Joystick API para eixos/botões/hats brutos.
- O desktop Slint faz polling na fronteira do host (intervalo nominal inicial de 8 ms); hotplug é observado pelo backend (intervalo nominal 250 ms).
- Atualizações são enviadas via `SmsTemporalWorker::set_controller_at`; não há polling no Scheduler nem callbacks por instrução.
- A primeira integração atribui os dois primeiros dispositivos elegíveis a P1/P2 por ordem da sessão.
- Não há XInput direto no caminho atual; SDL3 reconhece dispositivos conforme driver/mapeamento disponível.

## Fluxo de dados

1. Enumerar dispositivos e obter snapshots.
2. Manter DeviceId válido durante a sessão; `physical_key` persistente ainda não está disponível em todos os descritores.
3. Preservar botões, eixos, hats, classe, capacidades e sequência.
4. Aplicar perfil/mapeamento de ações.
5. Roteá-las para jogadores lógicos.
6. Adaptar o snapshot às portas ativas-baixas e aos registradores específicos de cada console.

## Determinismo e scheduler

- Consultar estado do host somente em fronteiras seguras do host/frame.
- Nunca consultar APIs físicas dentro de instruções Z80, operações de memória, callbacks de barramento, pixels ou amostras de áudio.
- Timestamp de host não é tempo emulado ciclo a ciclo.
- Não criar thread de polling por padrão.
- Input é latched no limite de integração; não altera clocks nem a ordem causal já agendada.

## Compatibilidade e pendências

Integrado em primeira fatia: gamepads SDL3, joysticks genéricos, polling de host e encaminhamento ao worker SMS.

Pendências:
- teste físico de controles, hotplug/reconexão e remoção de dispositivo P1;
- identidade persistente e seleção manual;
- múltiplos dispositivos por jogador;
- perfis individuais, remapeamento e calibração analógica;
- validação física do Start/NMI e caminhos Pause/NMI;
- rumble, force feedback, sensores e HID específicos;
- backend Linux/macOS;
- adaptadores específicos para Game Gear e demais consoles.

Não introduzir Qt/PyO3, polling no Scheduler ou implementação de input na GUI. A GUI apenas coleta e envia snapshots; o runtime interpreta a semântica de hardware.
