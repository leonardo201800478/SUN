# Modos de apresentação em tela cheia — Desktop

O desktop oferece três estados de apresentação, todos controlados pela camada de janela Slint. Nenhum atalho de apresentação envia comandos ao worker SMS ou altera o MachineScheduler.

| Estado | Entrada | Interface |
|---|---|---|
| Janela redimensionável | Estado padrão / Alt+Enter ao sair do modo imersivo | Menus, barra de transporte, status e HUD conforme preferências |
| Tela cheia com interface | F11 | Menus, barra de transporte, overlays e HUD permanecem disponíveis |
| Tela cheia imersiva | Alt+Enter | Somente o vídeo e o contador de FPS sobreposto no canto superior esquerdo |

## Transições

- F11 alterna entre janela e tela cheia com interface.
- Alt+Enter entra no modo imersivo a partir da janela ou da tela cheia com interface.
- Alt+Enter sai do modo imersivo e retorna à janela.
- F11 pressionado no modo imersivo troca para tela cheia com interface.
- A abertura de qualquer modo de tela cheia fecha um menu que estivesse aberto.

## Limites arquiteturais

- O estado de apresentação pertence ao desktop e é independente do estado de execução da emulação.
- A implementação usa `slint::Window::set_fullscreen`; não introduz dependência de API gráfica nem callbacks por instrução.
- O modo imersivo não oculta nem pausa o runtime: apenas altera a composição da GUI.
- A janela continua redimensionável no modo janela.

## Validação

A função pura de transição dos três estados tem testes unitários. A validação final de compilação e execução Windows x64 é realizada pelos workflows da PR. A confirmação manual em hardware Windows deve verificar foco do teclado, troca repetida entre os três estados, proporção de vídeo, HUD e restauração da janela após sair do modo imersivo.
