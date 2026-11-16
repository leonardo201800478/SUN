# Expansões e acessórios Sega 8-bit — escopo P4

**Status:** planejado; não declara implementação. **Ordem:** após SMS1/SMS2, SG-1000 e Game Gear. Hardware em Rust, MachineScheduler existente; sem host ou pipeline paralelo.

## Terminologia e compatibilidade

- **Sega Card / My Card:** mídia de jogo em cartão.
- **Card slot:** slot nativo no Master System/Mark III. Port $3E controla seleção/arbitragem; prioridade entre card e cartucho difere entre SMS e Mark III.
- **Card Catcher:** adaptador de cartucho que permite usar My Cards no SG-1000/SC-3000 sem slot próprio; não é o slot do SMS.
- **FM Sound Unit:** expansão Mark III com Yamaha YM2413. O Master System japonês possui FM integrado.
- **3D Glasses:** adaptador e óculos obturadores; usa $FFF8–$FFFB e não é uma mídia card.

“Card Graphics” não é a nomenclatura técnica encontrada nas referências consultadas; este escopo registra Sega Card/My Card e Card Catcher separadamente.

## YM2413 / FM Sound Unit

Perfis: Japanese SMS com chip integrado e controle $F2; Mark III com módulo externo e restrições de roteamento; export SMS/TecToy sem presumir FM de fábrica; mods de terceiros como perfis separados.

- $F0: latch/endereço; $F1: dados; $F2: detecção/controle conforme perfil.
- Preservar SN76489 e YM2413 como estados independentes.
- Reutilizar o clock do Scheduler quando compartilhado; não criar domínio FM duplicado.
- Alinhar fontes PSG/FM e misturá-las em uma operação, com taxas/fases reconciliadas antes do resampler CPAL.
- Save State inclui latch, registradores, slots, operadores, EG/PG, LFOs, pipeline, roteamento e fase do mixer/resampler.
- Validar portas, $F2 por perfil, reset, timing, mute/enable, síntese, Rhythm Mode, snapshot/restore e mistura contínua/em blocos.

`docs/architecture/ym2413-fm-model.md` é a especificação algorítmica existente; registra hipóteses e lacunas, não implementação concluída.

## Sega Card / My Card / Card Catcher

- Modelar mídia, slot e adaptador como entidades distintas.
- SMS/Mark III: ROM card, port $3E, enable lines e arbitragem BIOS/card/cartucho/expansão.
- Respeitar prioridade: SMS dá prioridade ao card quando card e cartucho estão inseridos; Mark III dá prioridade ao cartucho.
- SG-1000/SC-3000: Card Catcher no slot de cartucho para permitir My Cards.
- Testar mapa de memória, seleção, prioridade, reset e coexistência com cartuchos por console.

## 3D Glasses

- $FFF8 com mirrors $FFF9–$FFFB; bit 0 seleciona obturador.
- Adaptador no card slot de SMS exportação/Mark III; Japanese SMS tem conexão apropriada.
- Não há detecção de presença confiável pelo software.
- Shutter sincronizado por tempo emulado; não usar temporizador da GUI.
- Não presumir compatibilidade com SMS II ou Game Gear.

## Periféricos de controle

SMS Power documenta portas DE-9 para Joypad, Paddle, Sports Pad e Light Phaser. Rapid Fire requer auditoria específica.

- Paddle: posição de 8 bits serializada em nibbles e botão.
- Sports Pad: protocolo de trackball/strobe via I/O.
- Light Phaser: trigger e sensor óptico em TH; sincronização com raster e latch HCounter.
- Rapid Fire: modelar eletricamente o periférico e efeito nas linhas de botão.
- Keyboard e acessórios SG-1000/Mark III pertencem à interface de teclado/expansão, não aos DE-9 do SMS.

## Método e aceite

Para cada dispositivo: identificar console/revisão/conector/pinout; classificar DOCUMENTADO/MEDIDO/INFERIDO/NOT_VERIFIED; mapear endereço/port/timestamp; modelar reset e snapshot; definir testes de protocolo/timing; executar regressões SMS/SG-1000/GG.

## Referências

- SMS Power Card Slot: https://www.smspower.org/Development/CardSlot
- SMS Power Audio Control Port: https://www.smspower.org/Development/AudioControlPort
- SMS Power Peripheral Ports: https://www.smspower.org/Development/PeripheralPorts
- SMS Power I/O Port Map: https://www.smspower.org/Development/IOPortMap
- SMS Official Docs (port $3F, Sports Pad e I/O): https://www.smspower.org/Development/SMSOfficialDocs
- SMS Power 3D Glasses: https://www.smspower.org/Development/3DGlasses
- SMS Power Light Phaser: https://www.smspower.org/Development/LightPhaser
- SMS Power Paddle: https://www.smspower.org/Development/Paddle
- SMS Power Sports Pad Investigation (2025): https://www.smspower.org/forums/20657-SportsPadInvestigation
- Sega official SG-1000 II accessories/Card Catcher: https://www.sega.jp/history/hard/sg1000-2/devices.html
- YM2413 model: `docs/architecture/ym2413-fm-model.md`

As fontes documentam interfaces; não provam implementação ou validação do emulador.
