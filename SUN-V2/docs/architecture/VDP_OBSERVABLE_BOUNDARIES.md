# SUN V2 — boundaries observáveis do VDP SMS

**Baseline consultada:** V1 `207ccc4cebbc4f8c0aed45c68262eae963491689`

**Status R0:** primeira passagem documental concluída. A reexecução da ROM no SHA congelado e a evidência física por ASIC permanecem pendentes; não são pré-requisitos para fechar R0 se os limites e as incertezas estiverem explicitamente classificados. Não declarar os resultados históricos como validação do SHA congelado.

**Escopo:** SMS1/315-5124 como primeira fatia; SMS2/315-5246 permanece comparativo e incompleto.

## 1. Regra de precisão

O V2 só deve criar um boundary temporal quando existir um efeito observável por jogos ou periféricos suportados. O V1 usa um modelo de 228 T-states por linha, 342 posições horizontais e composição por spans para SMS1. Isso é uma representação coarse de eventos, não um pipeline de fetch por dot.

A fonte de timing central é a ROM [SmsVdpTest](https://github.com/FluBBaOfWard/SmsVdpTest), combinada com documentação SMS Power, código comparativo de ares/MAME e o relatório de testes fornecido no projeto. Valores de HPos/T-state são resultados do modelo de referência da ROM, não medições elétricas independentes.

## 2. Matriz de boundaries V1 observados no SHA congelado

| Evento | Boundary implementado | Efeito observável | Confiança e limite |
|---|---|---|---|
| Linha horizontal | 228 T-states; 342 HPos; razão comparativa 3/2 HPos por T-state | avanço do raster e conversão HCounter | Alta para código; média para hardware universal |
| Área ativa | HPos 63 inclusive até 319 exclusive; 256 pixels | composição de pixels visíveis e spans | Modelo comparativo; não é latch físico medido |
| HBlank | T213 / HPos≈319 | fecha o span ativo final; SMS1 também conclui o commit horizontal | Alta para implementação; hardware não medido |
| VINT IRQ | T13 / HPos≈20 na primeira linha não visível (scanline = height+1) | ativa `vint_pending`/INT | Coberto por teste unitário; precisa fixture full-machine e ROM |
| HINT / R10 | T14 / HPos≈21, nas linhas até height | decrementa contador ou ativa IRQ e recarrega R10 | Coberto por teste unitário; timing externo ainda precisa reexecução |
| VCounter | T15 / HPos≈23 | SMS1 atualiza latch da linha; SMS2 lê scanline diretamente | Coberto por teste; diferenças ASIC permanecem |
| Sprite overflow | T16 / HPos≈24, em linhas visíveis | publica índice/flag de overflow | Coberto por teste; SMS2/ASIC precisa auditoria própria |
| Frame publication | T16 / HPos≈24 na linha `height` | publica frame completo | Evento de apresentação, não substitui status VBlank |
| VBlank status | T17 / HPos≈25 na linha `height+1` | seta flag VBlank em status | Coberto por testes que distinguem F4/F5 |
| R8 horizontal scroll latch | HPos 21 inclusive / T14 | escrita até esse ponto pode alterar scroll da linha corrente; depois vale na próxima linha | Limite comparativo; sem medição física |
| R9 vertical scroll | escreve valor pendente durante display ativo e aplica na próxima linha | scroll vertical sem retroagir a linha corrente | Comportamento da implementação; confirmar em ROMs/jogos |
| VRAM/CRAM e R0–R7 | no completion boundary do write de porta, após commit do prefixo anterior | pixels anteriores usam estado antigo; suffix usa estado atualizado | Modelo de visibilidade, não timestamp de latch físico de cada registrador |
| Status read SMS1 | leitura de `$BF` comita pixels até o feixe antes de retornar flags | colisão/overflow que já cruzaram o feixe podem ser observados | Teste de software cobre pixel boundary; há comentário stale no código |
| HCounter | último valor capturado por TH rising edge/Light Phaser; fórmula `floor(3*T/2)`, amostra HPos anterior, subtrai 46 e divide por 2 | valor lido por `$7F` | Convenção 315-5124; normalização entre fontes ainda precisa fixture full-machine |
| Sprite SAT | metadados de sprite selecionados para a próxima scanline; bytes de pattern lidos durante spans | seleção, prioridade, overflow e colisão | Não modela fetches individuais de SAT/X/tile/bitplane |
| TH externo | eventos de entrada podem alterar TH no Scheduler | leitura de controle e, potencialmente, captura HCounter | Eventos TH externos ainda não provocam latch HCounter no caminho integrado |

## 3. Colisão SMS1: divergência documental dentro da V1

No SHA congelado, a implementação e os testes indicam que o status de colisão SMS1 se torna observável quando o feixe cruza o pixel opaco sobreposto:

- `commit_raster_span_to_beam()` renderiza o prefixo até HPos corrente e seta `status |= COLLISION` se o trecho contém colisão.
- `read_port($BF)` chama `commit_raster_span_to_beam()` antes de devolver o status.
- O teste `sprite_collision_becomes_visible_when_beam_crosses_overlap_pixel` verifica que a leitura antes do pixel não vê a flag e a leitura depois do pixel vê.
- O teste `sms1_hblank_boundary_commits_the_last_active_pixel_and_status` verifica que T213/HPos319 comita o último pixel ativo.

Entretanto, há um comentário em `read_port()` afirmando que “SMS1 collision is not visible until HBlank”. Esse comentário contradiz o fluxo de código e os testes citados. É drift documental/comentário, não evidência suficiente para alterar o comportamento.

**Regra V2:** usar o teste de pixel/beam como referência de software, registrar a divergência e exigir confirmação com ROM/trace/hardware antes de declarar o comportamento físico universal.

## 4. Limites de precisão e lacunas

1. **Sem dot pipeline:** não há arbitragem de fetch de tile, bitplane, sprite SAT, sprite X e pattern por dot.
2. **CRAM dots:** relatos SMS Power descrevem artefatos transitórios de escrita CRAM durante display ativo; o pipeline e o instante físico de latch não estão modelados.
3. **Latches R0–R7:** o commit no instante da escrita é um modelo de visibilidade. Não significa que cada registrador físico lache exatamente nesse timestamp.
4. **R5/SAT:** a avaliação SAT já preparada para a linha corrente não é refeita por uma escrita tardia de R5.
5. **HCounter:** a fórmula é a convenção implementada para 315-5124; os valores CPU-observáveis da ROM precisam ser testados pela máquina completa, incluindo ciclo de I/O, atraso de instrução e TH.
6. **TH externo:** a borda TH agendada não aciona ainda o latch HCounter. A regra MAME de latch no retorno a high usa offset HPos configurável e não deve ser importada como verdade física sem prova.
7. **SMS2:** o caminho de HBlank/raster-write ainda não está caracterizado; não generalizar limites SMS1 para 315-5246.
8. **PAL-M:** o perfil VDP V1 retorna 3.579545 MHz para PAL-M, em conflito com o modelo MAME de 3.575611 MHz no Z80/PSG. A placa TecToy usa MC1377P; há candidato comunitário de XTAL 10.726834 MHz e relato não confirmado de 10.746168 MHz. A subportadora 3.575611 MHz pode ser separada do clock CPU/VDP. O VDP PAL-M também usa 262 linhas no V1. Clock e perfil precisam de esquema/medição TecToy.
9. **HCounter report:** o comparativo fornecido em 15/11 registra início de sequência HCounter diferente entre Sega Emulator e ares, embora ambos apresentem salto `$93 → $E9`. O relatório não fixa SHA/ROM hash/ambiente, logo precisa ser reexecutado.

## 5. Priorização por impacto em jogos

| Prioridade | Efeito | Justificativa |
|---|---|---|
| P0 | VINT/HINT e leitura/limpeza de status | loops de jogo, sincronização de gameplay e raster |
| P0 | VCounter/HCounter em leituras por CPU | split-screen, sincronização e efeitos dependentes de raster |
| P1 | R8/R9 e writes de registradores durante display | scroll e efeitos visuais de gameplay |
| P1 | VRAM/CRAM writes e read-ahead | atualização de tiles/paleta e acesso correto a VDP |
| P1 | Sprite overflow/collision | flags podem ser consultadas por jogos e demos |
| P2 | CRAM dots e fetch-ahead | só avançar se ROM/jogo demonstrar efeito observável |
| P2 | diferenças de SMS2 | após SMS1 estável e com software-alvo comprovado |

## 6. Protocolo de reexecução obrigatório

Para cada ROM/caso:

1. Registrar SHA-256 da ROM, nome/versão, região/ASIC, BIOS quando aplicável e configuração exata.
2. Fixar SHA do emulador e toolchain; a baseline de auditoria é `207ccc4cebbc4f8c0aed45c68262eae963491689`.
3. Capturar saída esperada/observada, screenshots ou logs de teste e primeiro boundary divergente.
4. Comparar o mesmo caso em V1, ares/MAME e referência física quando disponível; diferenças de implementação não são automaticamente diferenças de hardware.
5. Registrar se o teste mede instante do VDP, instante de amostragem do Z80 ou valor observado pelo código do jogo.
6. Não corrigir um boundary baseado somente no HCount/HPos aparente sem normalizar a origem de contagem e o atraso da instrução/porta.
7. Reexecutar SMS VDP Test v1.31 completo e o subconjunto H/V IRQ, counters, status, TH, scroll, CRAM, sprite collision e overflow.

## 7. Fontes e proveniência

- Código V1 congelado: `native/video_core/src/sms/vdp.rs` (blob `768030a8bcd028a96989e9c29aa34c4cf8ffac60`).
- Arquitetura V1: `docs/architecture/sms-vdp.md` (blob `e577a40ef89955462b91bcb727d02a5d06cf60e1`).
- Integração temporal: `native/sms_core/src/temporal_bus.rs` (blob `394305a9685ec098df36f767d264ee1fbdf19c9a`).
- ROM: [FluBBa SmsVdpTest](https://github.com/FluBBaOfWard/SmsVdpTest).
- Referência: [SMS Power — VDP Registers](https://www.smspower.org/Development/VDPRegisters), [Modes](https://www.smspower.org/Development/Modes), [Sprites](https://www.smspower.org/Development/Sprites).
- Modelo comparativo: [MAME 315_5124](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp).
- Relatório histórico anexado ao projeto: `comparativo_de_testes_vdp_sega_emulator_vs_ares.md` (15/11/2026), sem SHA/ROM hash/ambiente.

## 8. Validações posteriores a R0

R0 foi aprovado documentalmente. Antes de atribuir os resultados históricos ao SHA congelado ou promover esses comportamentos a contrato físico, será necessário:
- classificar cada boundary por origem, perfil SMS1/SMS2, confiança, observabilidade e impacto em jogos;
- separar resultados de ROM/modelo comparativo de evidência elétrica independente;
- definir cenários discriminantes para Frame IRQ HCount, Line IRQ HCount, HCounter e TH, mesmo que sua execução seja posterior;
- registrar explicitamente quais timings e atrasos CPU permanecem UNKNOWN, sem promovê-los a constantes;
- limitar clocks PAL-M e demais perfis regionais sem evidência física suficiente.

A reexecução de SMS VDP Test v1.31 no SHA congelado com artefatos é necessária antes de atribuir seus resultados históricos àquele SHA ou usar esses resultados como aprovação. A fixture full-machine de IRQ/HCounter/TH deve ser construída no modelo de referência R2 ou antes do protótipo que dependa desses efeitos; ambas podem ser adiadas em relação ao fechamento documental de R0.

Nenhum requisito pede dot-accuracy universal. O critério é a menor granularidade que reproduz efeitos de gameplay comprovados.
