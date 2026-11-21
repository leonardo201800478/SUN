# Expansões e acessórios Sega 8-bit — escopo P4

**Status:** acessórios gerais P4 planejados; a trilha do YM2413 integrado foi antecipada como característica do perfil SMS, em branch/PR própria, após a PR #992. **Ordem:** acessórios gerais P4 continuam após SMS1/SMS2, SG-1000 e Game Gear. Esta exceção não altera a ordem das plataformas. Hardware em Rust, MachineScheduler existente; sem host ou pipeline paralelo.

## Terminologia e compatibilidade

- **Sega Card / My Card:** mídia de jogo em cartão.
- **Card slot:** slot nativo no Master System/Mark III. Port $3E controla seleção/arbitragem; prioridade entre card e cartucho difere entre SMS e Mark III.
- **Card Catcher:** adaptador histórico para uso de My Cards no SG-1000/SC-3000; documentado para contexto, mas excluído da implementação do emulador.
- **FM Sound Unit:** módulo externo histórico do Mark III, documentado apenas para distinguir sua topologia do YM2413 integrado do Master System japonês; fora do escopo de emulação.
- **3D Glasses/SegaScope:** acessório de obturadores LCD controlado por $FFF8–$FFFB; o protocolo será emulado, mas não é necessário reproduzir o adaptador físico.

A nomenclatura técnica encontrada é **Sega Card / My Card** (mídia) e **Card Catcher** (adaptador). O suporte ao arquivo/mídia não implica implementar o adaptador físico.

## YM2413 — FM integrado do Master System japonês

**Decisão de escopo:** implementar somente o comportamento do Master System japonês com YM2413 integrado. A trilha SMS-FM foi autorizada após a PR #992 e não aguarda Game Gear, pois não é uma nova plataforma nem a fase geral de acessórios. O primeiro estágio é somente a fundação de registradores/clock; ainda não há síntese nem integração de portas. Não emular o Mark III FM Sound Unit como módulo físico/adaptável nem seu roteamento externo.

- O YM2413 é um dispositivo opcional do perfil de hardware emulado, selecionável independentemente da região do console.
- Quando presente, o YM2413 opera em conjunto com o SN76489/PSG. O port $F2 deve reproduzir o controle do SMS japonês: $00 FM off/PSG on; $01 FM on/PSG off; $02 ambos off; $03 ambos on. A capacidade de silenciar o PSG por esse controle é específica do SMS japonês.
- Não inferir FM apenas da região. A presença do chip é uma opção de hardware; região/BIOS continuam propriedades separadas da máquina.
- Oferecer `FmMode::Auto`, `On` e `Off`, independente de `Territory`: Auto exige correspondência exata de título/revisão/hash e driver/dados FM comprovados; On ignora o catálogo somente em perfil físico que já possua YM2413 integrado; Off silencia a saída sem parar o estado interno do chip. On não cria hardware em modelos sem FM e nenhum modo injeta dados em ROM sem suporte.
- Algumas versões internacionais mantêm o driver FM; outras removeram o código durante localização. Há títulos que só utilizam FM com determinadas versões/condições regionais. A seleção do chip não injeta áudio FM em ROM sem suporte.
- $F0: latch/endereço; $F1: dados; $F2: detecção/controle do perfil Japanese SMS.
- Preservar SN76489 e YM2413 como dispositivos/estados independentes, com mixer final que permite saída simultânea real.
- Reutilizar o `ClockAdvance` do PSG somente se FM-A01 confirmar fonte e relação de fase na placa alvo; caso contrário registrar domínio FM explícito dentro do mesmo `MachineScheduler`. Nunca criar Scheduler paralelo.
- Alinhar fontes PSG/FM e misturá-las em uma operação determinística, com taxas/fases reconciliadas antes do resampler CPAL.
- Save State inclui latch, registradores, operadores, EG/PG, LFOs, pipeline, roteamento e fase do mixer/resampler.
- Validar portas, $F2, reset, timing, mute/enable, síntese, Rhythm Mode, snapshot/restore e mistura contínua/em blocos.

O Mark III FM Sound Unit fica documentado como hardware histórico fora do escopo de emulação. [A especificação arquitetural YM2413](../architecture/ym2413-fm-model.md) é a fonte normativa dos contratos por modelo físico, `FmMode`, scorecard e pendências; a auditoria M2 permanece separada em `docs/audits/`.

## Sega Card / My Card

- **Manter o formato de mídia e o carregamento de ROMs Sega Card/My Card.** Não é apenas um adaptador para SG-1000: Mark III e Master System possuem slot nativo e receberam jogos nesse formato.
- Modelar o arquivo/mídia e sua conexão ao sistema sem criar um periférico físico Card Catcher obrigatório.
- **Excluir Card Catcher C-1000 do escopo de implementação.** Ele é um adaptador físico para conectar My Cards ao slot de cartucho de SG-1000/SG-1000 II/SC-3000; como o emulador oferece carregamento direto e suporte ao SG-1000, reproduzir esse adaptador não agrega função necessária.
- Preservar distinções entre Sega Card/My Card, My Card Mark III e cartuchos; compatibilidade depende de geração, BIOS, VDP e software.
- Para SMS/Mark III, considerar port $3E, enable lines e arbitragem BIOS/card/cartucho apenas se forem necessários ao slot nativo e à inicialização do software.
- Não transportar a lógica do Card Catcher para o core SG-1000. Jogos de cartão compatíveis com SG-1000 devem ser carregáveis diretamente como software, respeitando o mapeamento de memória e os limites do sistema.

## SegaScope / 3D Glasses

- Emular o protocolo real: escrita em $FFF8 (mirrors $FFF9–$FFFB), bit 0 seleciona o obturador/olho ativo. A presença física dos óculos não é detectável pelo software.
- O adaptador físico dependia do card slot em determinados modelos; isso não obriga o emulador a implementar um acessório físico.
- O estado dos obturadores deve ser derivado do tempo emulado e sincronizado ao VDP/frame scheduler, nunca por timer da GUI.
- **Objetivo de jogabilidade sem óculos:** oferecer visualização 2D selecionável (olho esquerdo/direito) e composição lado a lado. Isso permite jogar/inspecionar sem óculos, mas não cria percepção estereoscópica em monitor 2D convencional.
- Modo anaglífico (vermelho/ciano) pode ser oferecido como opção adicional, mas exige óculos coloridos. Saída estereoscópica real pode ser considerada para displays/headsets compatíveis.
- Referências de implementação existentes: MAME separa saídas LCD esquerda/direita e oferece opções SegaScope/binocular; Dega implementa composição red/blue; Snepulator declara anaglyph 3D.
- Não presumir compatibilidade física com SMS II ou Game Gear. A emulação do protocolo e a apresentação 3D são camadas distintas.

## SC-3000 — limite de escopo

**SC-3000 está integralmente fora do escopo do SUN — Strife Uranus Neptune como plataforma-alvo.** Não implementar o computador SC-3000, seus modos de execução, software de computador, BASIC, teclado SK-1100, cassette/data recorder, printer ou periféricos específicos. A existência de jogos SC/SG não altera essa decisão: somente títulos comprovadamente compatíveis com o hardware SG-1000 poderão ser carregados pelo perfil SG-1000. Isso não representa emulação ou compatibilidade geral com SC-3000.

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
- Sega official SC-3000 software catalog (SC/SG game cartridges, My Cards, keyboard/software): https://www.sega.jp/history/hard/sc3000/software.html
- Sega official SG-1000 II accessories (My Card, Card Catcher C-1000, SK-1100, controllers): https://www.sega.jp/history/hard/sg1000-2/devices.html
- MAME SMS SegaScope left/right LCD output and binocular options: https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp
- Dega red/blue 3D composition and FM options: https://github.com/Plombo/dega
- Snepulator anaglyph 3D support: https://snepulator.net/
- SMS Power Sports Pad Investigation (2025): https://www.smspower.org/forums/20657-SportsPadInvestigation
- SMS Power Development index (Rapid Fire Unit, Pause/Reset, Handle Controller, GG accessories): https://www.smspower.org/Development/Index
- YM2413 model: `docs/architecture/ym2413-fm-model.md`

As fontes documentam interfaces; não provam implementação ou validação do emulador.
