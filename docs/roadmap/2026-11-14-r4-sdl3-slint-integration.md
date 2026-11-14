# R4 — Integração SDL3 com o desktop Slint

**Data:** 2026-11-14
**Baseline main:** `19e3155a0efcaadc9d5fff9b3e8bf1b12b30784c`
**Branch de trabalho:** `feat/r4-slint-sdl3-gamepad-input-20261114`

## 1. Auditoria do estado anterior

- `native/input_runtime` já pertence ao workspace e contém contratos host-independent, perfis genéricos, buffers reutilizáveis e `Sdl3Backend`.
- O backend SDL3 carrega a ABI C dinamicamente no Windows, sem link estático; suporta Gamepad API e Joystick API.
- `scripts/install_sdl3_runtime.ps1` instala SDL3 3.4.20 com SHA-256 verificado.
- O `sega-desktop` não dependia de `sega-input-runtime`; `main.rs` só recebia teclado por eventos nativos Slint e enviava snapshots ao `SmsTemporalWorker`.
- O workflow `windows-input-retired` estava intencionalmente sempre skipped e declarava que SDL3 não estava integrado.
- A PR #313 foi integrada na arquitetura histórica Qt/PyO3. Ela não prova a integração do backend no desktop Slint atual.
- `DeviceDescriptor.physical_key` é `None` no backend SDL3 atual. Não existe ainda identidade física persistente suficiente para perfis por dispositivo.
- O backend SDL3 é Windows-only nesta implementação. O enum `BackendKind::XInput` não representa um backend XInput nativo implementado.

## 2. Primeira fatia implementada nesta branch

- Adiciona `sega-input-runtime` como dependência do desktop Slint.
- Introduz `native/sega_desktop/src/input_host.rs`, adaptador seguro entre SDL3 e `ControllerButtons` do SMS.
- Mantém teclado no Slint e combina os níveis de teclado e gamepad por OR, evitando que o release de uma fonte libere a outra.
- Polling ocorre em timer do host Slint a cada 8 ms (125 Hz nominal); o backend existente verifica hotplug a cada 250 ms.
- Primeiro e segundo dispositivos elegíveis na ordem de sessão são atribuídos a SMS P1 e P2.
- Gamepad padronizado: D-pad/analógico esquerdo, SDL_GAMEPAD_BUTTON_SOUTH/EAST para Botão 1/2 e START para NMI (Pause) por borda de subida.
- Joystick genérico: hat 0, eixos raw 0/1, botões raw 0/1 e botão raw 2 para NMI.
- Joystick genérico: hat 0, eixos raw 0/1 e botões raw 0/1.
- Em menu aberto ou sem ROM, o estado de gameplay do gamepad é neutralizado.
- Erro de inicialização SDL3 não impede o uso do teclado; o status é exposto no painel Controles.
- Atualizações de input são snapshots coarse-grained via `SmsTemporalWorker::set_controller_at`. Nenhuma alteração no Scheduler, CPU, VDP, PSG, clocks ou timing.
- Adiciona testes unitários de mapeamento e smoke test Windows para carregar a DLL, inicializar SDL3, enumerar e poll sem controle físico.
- Reativa validação Windows SDL3 no CI com runtime 3.4.20 instalado por SHA-256.

## 3. Limites explícitos desta fatia

- A atribuição P1/P2 é provisória e baseada na ordem de sessão, não em configuração persistente.
- Perfis persistentes por identidade física, remapeamento visual e captura analógica calibrada ainda não foram implementados.
- Não há backend XInput direto. O SDL3 pode reconhecer dispositivos XInput conforme drivers/mapeamentos do SDL.
- Não há suporte declarado a rumble, force feedback, sensores, touchpad ou GameInput.
- Hotplug é detectado pelo scan periódico existente; ainda precisa de validação física com desconexão/reconexão.
- O threshold de 10.000 para o analógico é um mapeamento inicial de gameplay, não um algoritmo de captura/calibração.
- Não há medição de input-to-frame ou button-to-photon neste incremento.
- O teste Windows de CI valida ABI/runtime sem hardware; não substitui teste físico com controles.

## 4. Critérios para fechar R4

1. Windows x64 CI verde no HEAD final, incluindo smoke test SDL3 e build/test do desktop.
2. Teste físico com pelo menos um gamepad SDL mapeado e um joystick genérico, se disponível.
3. Press/release, diagonais, analógico centrado, dois botões, troca de foco, abertura de menus e troca de ROM sem estado preso.
4. Hotplug e reconexão, incluindo remoção do dispositivo que ocupa P1.
5. Dois controles conectados, ordem/atribuição P1/P2 documentada.
6. Perfis por dispositivo com identidade estável, migração de configurações e testes de dois controles idênticos.
7. Captura analógica separada da deadzone de gameplay.
8. Validar Start físico como NMI por borda de subida, sem confundir com Pause do frontend; reproduzir o caso Road Rash de forma determinística antes de qualquer mudança temporal.
9. Nenhuma alteração na autoridade temporal do MachineScheduler.

## 5. Referências externas

- [SDL3 3.4.20 release](https://github.com/libsdl-org/SDL/releases/tag/release-3.4.20)
- [SDL_GamepadButton](https://wiki.libsdl.org/SDL3/SDL_GamepadButton)
- [SDL_GamepadAxis](https://wiki.libsdl.org/SDL3/SDL_GamepadAxis)
- [SDL_UpdateGamepads](https://wiki.libsdl.org/SDL3/SDL_UpdateGamepads)
