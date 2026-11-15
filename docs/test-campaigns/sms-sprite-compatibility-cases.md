# SMS VDP — campanha de compatibilidade de sprites

**Status:** campanha exploratória interna retomada em 15/11/2026; campanha manual/física permanece pausada
**Objetivo:** registrar ROMs, cenas, resultados observados e evidências comparativas antes de alterar a implementação do VDP.
**Hardware de referência do desktop:** perfil SMS1 / ASIC 315-5124, salvo indicação explícita em cada caso.
**Regra:** resultados relatados pelo usuário são registrados como observações; não constituem, isoladamente, prova da causa técnica.

## Convenções

- **REPRODUZIDO PELO USUÁRIO:** comportamento observado no build local informado pelo usuário.
- **REFERÊNCIA ARES:** imagem ou comportamento usado como comparação externa.
- **IDENTIDADE PENDENTE:** ROM ainda sem CRC32/SHA-1 confirmado.
- **CAUSA NÃO DETERMINADA:** não atribuir o defeito a SAT, bitplanes, prioridade ou temporização sem trace/teste que discrimine a hipótese.
- Screenshots enviados na conversa são evidência visual de referência, mas não estão versionados neste documento. Quando os arquivos forem anexados ao repositório, registrar caminho e hash.

## Casos registrados

### SMS-SPR-001 — SMS 3-D Demo (World) (Aftermarket) (Unl)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `SMS 3-D Demo (World) (Aftermarket) (Unl).sms` |
| CRC32 / SHA-1 | PENDENTE — arquivo ROM ainda não fornecido para identificação |
| Perfil do emulador | SMS1 / 315-5124 (perfil ativo do desktop no teste relatado) |
| Referência | Ares, mesma cena visual informada pelo usuário |
| Resultado no Ares | Objetos circulares verdes, sombras e elementos da cena renderizados de forma coerente |
| Resultado no Sega Emulator | Fundo e faixas de cores aparecem; objetos 3D/sprites aparecem fragmentados, deslocados ou substituídos por blocos |
| Classificação | REPRODUZIDO PELO USUÁRIO; IDENTIDADE PENDENTE |
| Causa | NÃO DETERMINADA |

**Evidência visual:** comparação lado a lado enviada pelo usuário em 14/11/2026. A janela à esquerda é o Sega Emulator; à direita, Ares. A imagem demonstra divergência visual na mesma demonstração, mas não isola por si só o mecanismo responsável.

**Perguntas a responder:**
1. A ROM usa exclusivamente sprites Mode 4 ou também altera registradores/modos durante a cena?
2. Os atributos SAT e os índices de padrão selecionados coincidem com a referência?
3. Os bytes dos padrões e bitplanes são lidos do endereço correto?
4. Há divergência em limite de oito sprites por linha, zoom, deslocamento X/Y, clipping ou prioridade?
5. O defeito persiste com execução determinística e após reset/reload?

**Próxima ação:** obter o arquivo exato, calcular CRC32/SHA-1 e registrar região/configuração, cena reproduzível e screenshot sem redimensionamento. Não alterar o VDP com base apenas neste caso.

### SMS-GFX-004 — SMS Chip-8 Interpreter (World) (v0.21) (Proto) (Aftermarket) (Unl)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `SMS Chip-8 Interpreter (World) (v0.21) (Proto) (Aftermarket) (Unl).sms` |
| CRC32 / SHA-1 | PENDENTE — arquivo ROM não fornecido para identificação |
| Perfil do emulador | Não confirmado nesta captura; registrar antes de repetir |
| Referência | Ares, tela de menu do interpretador |
| Resultado no Ares | Menu, logotipo CHIP-8 e texto descritivo legíveis e alinhados |
| Resultado no Sega Emulator | Fundo verde aparece, mas logotipo/textos estão fragmentados, com caracteres deformados/deslocados e linhas ilegíveis |
| Classificação | REPRODUZIDO PELO USUÁRIO; IDENTIDADE PENDENTE |
| Domínio aparente | Renderização de caracteres/tile patterns e/ou composição de plano; não classificar ainda como defeito exclusivo de sprites |
| Causa | NÃO DETERMINADA |

**Evidência visual:** comparação lado a lado enviada pelo usuário em 14/11/2026. Sega Emulator à esquerda e Ares à direita, ambos exibindo a tela de menu do interpretador. A diferença é visível já em texto estático, sem necessidade de iniciar um jogo CHIP-8.

### Evidência adicional — tela do labirinto (14/11/2026)

O usuário avançou para a tela do labirinto e enviou uma segunda comparação lado a lado da mesma ROM.

| Critério | Ares | Sega Emulator |
|---|---|---|
| Fundo verde | Correto | Presente |
| Paredes/corredores do labirinto | Geometria contínua, paredes brancas e áreas pretas bem definidas | Geometria não é reconstruída; aparecem fragmentos pequenos e caracteres dispersos |
| Elementos do cenário | Distribuição coerente | Ausentes ou visualmente corrompidos |
| Classificação desta cena | Referência visual | Falha reproduzida pelo usuário |

**Interpretação limitada:** a falha persiste em uma segunda tela e afeta elementos de cenário além do logotipo/texto do menu. Isso reforça a necessidade de auditar a renderização de background tiles/name table/pattern data. Ainda não determina se a causa está na escrita de VRAM, leitura de padrões, seleção de tabela, paleta ou temporização. Não há base para atribuir este caso somente ao subsistema de sprites.

**Identidade e configuração:** CRC32/SHA-1 da ROM e perfil efetivo do VDP continuam pendentes. A captura é evidência visual da conversa e não foi adicionada como binário ao repositório.

**Perguntas a responder:**
1. A ROM escreve os caracteres como tiles de background, sprites ou combinação dos dois?
2. Os dados de pattern e os índices de tile são idênticos entre os emuladores?
3. Há divergência na interpretação de name table, pattern generator, palette/CRAM ou atributos?
4. A deformação depende do perfil SMS1/315-5124 ou também ocorre em SMS2?
5. O defeito é estático desde o menu ou muda após navegação/entrada?

**Próxima ação:** obter a ROM exata e seus hashes; repetir a mesma tela com perfil e região registrados. Usar este caso como controle para distinguir defeitos de tiles/fontes de defeitos de SAT/sprites. Não alterar o renderer antes de identificar como a ROM desenha o menu.

### SMS-SPR-002 — Cosmic Spacehead (Europe) (En,Fr,De,Es)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Cosmic Spacehead (Europe) (En,Fr,De,Es).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `29822980` |
| SHA-1 | `f46f716dd34a1a5013a2d8a59769f6ef7536a567` |
| Mapper | Codemasters |
| Resultado relatado | Jogo inicia e áudio funciona; sprites aparecem totalmente fora de posição |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Causa | NÃO DETERMINADA; compatibilidade de modo/perfil de vídeo é hipótese a investigar |

**Nota de compatibilidade:** evidência de catálogo previamente consultada indica requisito de modo extra-height e compatibilidade associada a SMS2 para esta ROM. Isso deve ser verificado contra a configuração efetiva e a referência antes de classificar o sintoma como falha genérica de sprites SMS1.

### SMS-SPR-003 — The Smurfs Travel the World (Brazil)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Smurfs Travel the World, The (Brazil).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `E14C8A1C` |
| SHA-1 | `15b882ad3263cdc1e78c732d96f9bb18e25f4743` |
| Resultado relatado | Áudio funciona; sprites aparecem fora de posição |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Identidade comercial/dump | Não confirmada por catálogo conhecido na investigação inicial |
| Causa | NÃO DETERMINADA |

### SMS-BOOT-001 — Andre Agassi Tennis (Europe, Brazil) (En)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `Andre Agassi Tennis (Europe, Brazil) (En).sms` |
| Tamanho | 262.144 bytes |
| CRC32 | `F499034D` |
| SHA-1 | `81dbacad4739b98281c750d9af21606275398fc8` |
| Resultado relatado | Tela totalmente negra e zunido contínuo; nenhuma imagem do jogo |
| Snapshot observado | PC variou entre `00BD`, `008E` e `0089` em capturas distintas; SP `DFF7` |
| Classificação | REPRODUZIDO PELO USUÁRIO |
| Causa | NÃO DETERMINADA |

Este caso é registrado como falha de boot/vídeo, não como caso isolado de sprites. O movimento do PC entre snapshots não demonstra, por si só, progresso correto do boot.

### SMS-GFX-005 — SMS Power 7th Anniversary (World) (Auto Demo) (Aftermarket) (Unl)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `SMS Power 7th Anniversary (World) (Auto Demo) (Aftermarket) (Unl).sms` |
| CRC32 / SHA-1 | PENDENTE — arquivo ROM não fornecido para identificação |
| Referência | Ares, auto demo |
| Resultado no Ares | Logotipo "SMS POWER", subtítulo e faixas coloridas apresentados corretamente; letras da frase comemorativa distribuídas em arco |
| Resultado no Sega Emulator | Logotipo, subtítulo e faixas coloridas visualmente correspondentes; letras da frase em arco aparecem em posições diferentes da referência, com distribuição/posicionamento divergente |
| Classificação | PARCIALMENTE CORRETO; DIVERGÊNCIA DE POSICIONAMENTO |
| Domínio aparente | Movimento/posicionamento de elementos gráficos durante a demo; pode envolver sprites, atributos SAT, coordenadas ou temporização de atualização |
| Causa | NÃO DETERMINADA |

**Evidência visual:** comparação lado a lado enviada pelo usuário em 14/11/2026. Os elementos estáticos principais (logotipo, subtítulo e faixas) parecem corretos nos dois emuladores. A diferença mais evidente está nas letras individuais que formam a frase em arco: sua distribuição espacial difere do Ares.

**Interpretação limitada:** este caso não é uma falha total de renderização. É um controle parcialmente positivo para elementos estáticos, acompanhado de uma divergência em elementos móveis/posicionados individualmente. A captura não permite concluir se as letras são sprites ou tiles de background atualizados dinamicamente.

**Próxima ação:** registrar a cena e comparar a posição das letras ao longo do tempo; obter CRC32/SHA-1 e confirmar perfil/região. Não classificar como defeito SAT até confirmar como a ROM implementa o efeito.

### SMS-PAL-001 — SMS Palette (World) (Aftermarket) (Unl)

| Campo | Registro |
|---|---|
| Sistema | Sega Master System |
| ROM | `SMS Palette (World) (Aftermarket) (Unl).sms` |
| CRC32 / SHA-1 | PENDENTE — arquivo ROM não fornecido para identificação |
| Referência | Ares, tela de demonstração da paleta |
| Resultado no Ares | Título, índices 0–63 e quatro linhas de amostras de cores visíveis |
| Resultado no Sega Emulator | Título, índices e quatro linhas de amostras de cores visualmente correspondentes à referência |
| Classificação | RESULTADO VISUAL APARENTEMENTE CORRETO; IDENTIDADE PENDENTE |
| Causa | Não aplicável enquanto não houver divergência observada |

**Evidência visual:** comparação lado a lado enviada pelo usuário em 14/11/2026. As barras de cores e a numeração aparentam alinhamento e correspondência entre Sega Emulator e Ares.

**Limite da conclusão:** a captura sugere que a paleta/CRAM e sua apresentação básica estão funcionando para esta ROM e cena. Não comprova exatidão RGB de cada entrada, comportamento de escrita CRAM durante raster, nem valida globalmente o caminho de paleta em todos os modos/perfis. A escala e captura das janelas também impedem uma comparação colorimétrica pixel a pixel.

**Próxima ação:** manter como caso de controle visual positivo. Se necessário, obter a ROM e capturas nativas para comparação de pixels e identificação por CRC32/SHA-1. Não priorizar correção de paleta com base nesta evidência.

## Protocolo para novos casos

Para cada ROM nova, registrar:

1. Nome exato do arquivo e tamanho em bytes.
2. CRC32 e SHA-1.
3. Região e perfil de hardware selecionados (SMS1/315-5124, SMS2 ou outro).
4. Versão/commit do Sega Emulator e versão do Ares usada na comparação.
5. Cena, sequência de entrada e tempo desde o reset.
6. Resultado de vídeo e áudio em cada emulador.
7. Screenshot da mesma cena, sem filtros ou escalas diferentes quando possível.
8. Snapshot/trace, se disponível, e número de eventos descartados.
9. Estado da reprodução: reproduzido, não reproduzido ou inconclusivo.

## Matriz de investigação — não são conclusões

| Área | Verificação |
|---|---|
| SAT | Endereço base, leitura de Y, X, tile/pattern, atributos e ordem |
| Padrões | Endereçamento, seleção de pattern, bitplanes e leitura de VRAM |
| Geometria | 8×8/8×16, zoom, clipping e coordenadas negativas |
| Composição | Cor transparente, prioridade, sobreposição e ordem |
| Limites | Oito sprites por scanline, overflow e índice reportado |
| Colisão | Interseção de pixels opacos e instante de publicação do status |
| Temporização | Fetch de SAT/padrões, escritas intralinha e boundaries por dot/T-state |
| Perfil | Diferenças SMS1 315-5124 versus SMS2 e modos de vídeo especiais |

## Política de implementação

- Não modificar SAT, sprite renderer ou temporização para corrigir um único título sem teste que identifique o comportamento.
- Separar defeitos de semântica de sprite de defeitos de pipeline temporal.
- Criar testes sintéticos mínimos para cada comportamento isolado e manter os casos reais como regressão.
- Preservar Scheduler, determinismo e equivalência entre execução contínua e em blocos.
- Atualizar este documento incrementalmente à medida que novos testes forem recebidos.


### SMS-VDPTEST-001 — SMS VDP Test (World) v1.31 (14/11/2026)

**Comparação:** Sega Emulator à esquerda; Ares à direita. Capturas sequenciais da mesma ROM e das telas de resultado. Perfil do desktop: SMS1 / ASIC 315-5124. A ROM não foi anexada ao repositório; identidade por hash ainda pendente.

#### Falhas exclusivas do Sega Emulator em relação ao Ares

| Grupo | Caso do teste | Sega Emulator | Ares | Classificação |
|---|---|---:|---:|---|
| VDP data test | `1byte wr sets rd adr` | Error | OK | REPROVADO — divergência exclusiva |
| VDP data test | `1byte wr sets wr adr` | Error | OK | REPROVADO — divergência exclusiva |
| VDP misc test | `Line IRQ VCount` | Error | OK | REPROVADO — divergência exclusiva |

Esses três resultados são as reprovações funcionais diretamente demonstradas nas telas comparativas.

#### Erros compartilhados — não atribuir como regressão exclusiva

| Grupo | Caso do teste | Sega Emulator | Ares | Interpretação |
|---|---|---:|---:|---|
| VDP misc test | `VCounter chg time` | Error | Error | Falha compartilhada |
| VDP misc test | `Frame IRQ VCount` | Error | Error | Falha compartilhada |
| VDP misc test | `Frame IRQ HCount` | Error | Error | Falha compartilhada |
| VDP misc test | `Line IRQ HCount` | Error | Error | Falha compartilhada |
| VDP misc test | `VINT flag HCount` | Error | Error | Falha compartilhada |
| Sprite test | `Offscreen Y, col` | Error | Error | Falha compartilhada |
| Sprite test | `Spr col correct HC` | Error | Error | Falha compartilhada |
| Sprite test | `Spr ovf correct HC` | OK | Error | Sega Emulator passa onde Ares reprova; não é falha do Sega Emulator |

#### Resultados positivos ou equivalentes observados

- **VDP data test:** os demais casos visíveis — leitura/escrita normal, leituras após code 1/2/3, escritas após code 0/2/3, mixed read/write, VRAM/CRAM write-set VDPbuffer, byte write keeps write mode, address wrap, VRAM/CRAM shared address, reset latch de controle/dados e unused registers — aparecem como OK nos dois emuladores.
- **VDP misc test:** VCounter NTSC values, HCounter keeps value, HC change on TH 0→1, VDP register/data/control mirrors, VCounter/HCounter mirrors e VINT flag kept aparecem OK nos dois. O caso `HCounter correct` aparece OK no Sega Emulator e Error no Ares.
- **Sprite test:** No sprite collision, no display/no sprite collision, 9th sprite no collision, transparent pixel no collision, offscreen X no collision, sprite collision, collision flag kept in VBlank, collision behind tile, collision correct line, collision many lines, no sprite overflow, no display/sprite overflow, offscreen Y/no overflow, sprite overflow, overflow kept in VBlank, overflow correct line e overflow many lines aparecem OK nos dois.
- O Sega Emulator também apresenta OK em `Spr ovf correct HC`, enquanto Ares apresenta Error.

#### Observações de tela sem veredito funcional conclusivo

- **X-scroll latch:** a coluna vertical exibida como “Column should be straight” parece reta nos dois. A captura do Ares contém um pequeno traço horizontal isolado no campo; a captura do Sega Emulator não mostra esse traço. Sem sincronização temporal e sem o veredito textual do teste, registrar como diferença visual inconclusiva, não como reprovação.
- **HCounter values:** as sequências numéricas visíveis não começam no mesmo valor (a captura do Sega Emulator inicia em torno de `2A`, enquanto a do Ares começa em torno de `32`). Como as capturas não foram sincronizadas no mesmo ponto de fase e a tela não mostra um veredito PASS/FAIL, registrar como discrepância de amostragem/temporização a investigar, não como reprovação confirmada.
- A comparação de escala, enquadramento e área útil das janelas é diferente; isso não deve ser confundido com falha funcional do VDP.

#### Prioridade de investigação derivada do teste

1. Auditar a semântica de escrita de um byte no data port e os efeitos colaterais esperados sobre read address e write address, preservando o comportamento do buffer/latch.
2. Auditar a geração de Line IRQ em função de VCount: comparação do contador, reload/enable, fronteira de linha e instante de publicação do status/IRQ pelo Scheduler.
3. Separar os erros compartilhados do Ares dos defeitos exclusivos; não “corrigir” os casos compartilhados apenas para imitar Ares sem uma referência independente.
4. Manter os resultados de sprites como cobertura parcialmente positiva: não há, nesta captura, uma reprovação exclusiva do Sega Emulator nos casos de colisão/overflow listados; os erros de Offscreen Y collision e collision correct HC também aparecem no Ares.
5. Repetir os casos com mesma região, perfil, build e fase temporal, registrando snapshot/trace do VDP e do Scheduler antes de alterar código.

**Limite da evidência:** são resultados observados em comparação visual com Ares, não medições de hardware físico. Ares é a referência operacional desta campanha, mas um erro compartilhado não prova que ambos estejam errados nem que o comportamento físico corresponda a um dos dois.

### SMS-VDP-SCROLL-001 — Scroll Test Program by Charles MacDonald

**Origem:** capturas comparativas enviadas pelo usuário em 14/11/2026. **Campanha física/manual pausada após este registro**, por decisão do usuário; não solicitar novos testes até haver correção e progresso automatizado demonstrável.

| Região | Referência Ares | Sega Emulator | Observação |
|---|---|---|---|
| Faixa 1 | Scroll vertical descendente; faixa fina de dois blocos | Visualmente correspondente | Controle positivo parcial |
| Faixa 2 | Fundo diagonal do canto inferior direito para o superior esquerdo; bloco de 8×6 sobreposto move-se da direita para a esquerda | Fundo desloca-se, mas o bloco não aparece como sobreposição nem avança horizontalmente | Divergência de composição/movimento |
| Faixa 3 | Procedimento semelhante à faixa 2, com movimento horizontal inverso e bloco de 8 unidades de altura | Fundo desloca-se; bloco sobreposto não aparece e permanece fixo à direita | Divergência de composição/movimento |
| Faixa 4 | Fundo desloca-se de cima para baixo em direção à esquerda; bloco sólido 8×8 move-se da direita para a esquerda | Fundo desloca-se de baixo para cima em direção à esquerda; bloco não aparece/move | Divergência de sentido vertical e composição |

**Classificação:** REPRODUZIDO PELO USUÁRIO; divergência visual clara contra Ares. ROM não foi anexada para hash/inspeção. O relato de “bloco sobreposto” não determina se o elemento é sprite, tile prioritário ou atualização dinâmica do tilemap.

**Hipóteses discriminantes (não conclusões):**
1. Escritas intralinha em R8/R9 e instante de latch/aplicação por scanline.
2. Fine scroll, wrap e cálculo das coordenadas de origem do tilemap.
3. Prioridade de tiles Mode 4 versus sprites e composição em spans.
4. Escritas de VRAM/SAT durante a linha e ausência de fetch-ahead temporizado.
5. Ordem de atualização do estado de raster em relação aos commits de span.

**Próxima ação interna:** construir fixtures sintéticas para cada faixa (scroll horizontal/vertical, wrap, prioridade e alteração de R8/R9 em boundaries controlados) e validar equivalência entre avanço contínuo e em blocos. Não ajustar o renderer WGPU nem inverter sinais globalmente para imitar uma única cena.

### SMS-VDP-ZOOM-001 — Zoom Effect #1_3 by Charles MacDonald

**Origem:** captura comparativa enviada pelo usuário em 14/11/2026.

| Critério | Ares | Sega Emulator |
|---|---|---|
| Padrão de xadrez e barras coloridas | Faixas e transições uniformes | Padrão geral correspondente, com leve distorção de linha próximo à extremidade direita e parte final da imagem |

**Classificação:** DIVERGÊNCIA LOCAL DE GEOMETRIA/RASTER; causa não determinada. A captura não discrimina se a origem é coordenada de composição, limite de span, escala de saída ou amostragem do framebuffer. A investigação deve primeiro comparar o framebuffer lógico RGB888 antes do WGPU, para separar VDP de apresentação.

### SMS-VDP-ZEXALL-001 — ZEXALL V0.12VDP

**Origem:** captura comparativa enviada pelo usuário em 14/11/2026.

| Emulador | Observação |
|---|---|
| Ares | Exibe tela de resultados do ZEXALL com vários testes de instruções marcados OK |
| Sega Emulator | Tela preta |

**Classificação:** INCONCLUSIVO PARA Z80. A variante VDP depende da inicialização e apresentação pelo VDP; a tela preta não prova falha de instrução nem localiza a falha entre boot, CPU, VDP e apresentação. Não promover esta execução a evidência E8 nem a resultado de ZEXALL.

### SMS-VDP-ZEXALL-002 — ZEXALL V0.12SDSC

**Origem:** captura comparativa enviada pelo usuário em 14/11/2026.

| Emulador | Observação |
|---|---|
| Ares | Permanece na tela de inicialização do Master System; não chega à tela de resultados |
| Sega Emulator | Tela preta |

**Classificação:** INCONCLUSIVO. A variante SDSC usa um canal de saída de depuração, não a tela normal do VDP. A captura não demonstra execução correta ou incorreta do Z80. Não iniciar E8 com base nesta ROM/versão.

## Política de campanha após 14/11/2026

A campanha manual/física permanece **PAUSADA** por decisão do usuário. A campanha interna automatizada foi retomada em 15/11/2026 por solicitação explícita: priorizar testes sintéticos direcionados, regressões automatizadas, traces reproduzíveis e correções integradas. Não solicitar novas capturas nem binários ROM ao usuário; usar as identidades/hashes registrados, os casos sintéticos e os resultados de CI. Os binários comerciais não estão versionados no repositório.


### Retomada interna — 15/11/2026

- Seleção explícita SMS1/315-5124 e SMS2/315-5246 integrada pela PR #892.
- O smoke harness opt-in executa 1.800 frames por ROM por padrão e registra hashes de framebuffer e fingerprints de estado Z80/VDP/mapper.
- A caracterização do zoom horizontal SMS1 (regra fixa dos primeiros quatro vs. N−4) foi separada em #898; não alterar o modelo sem evidência de hardware.
- A compatibilidade genérica cartucho ↔ ASIC está em #897.
- Cosmic Spacehead e The Smurfs continuam sem execução automatizada nesta sessão porque os binários comerciais não estão no repositório; não inferir resultado de gameplay a partir de testes sintéticos.

A ordem de trabalho passa a ser:
1. Corrigir e ampliar testes do caminho de data/control port e Line IRQ/VCount já reprovados no SMS VDP Test v1.31.
2. Criar fixtures sintéticas para scroll raster, prioridade e composição de tiles/sprites.
3. Isolar a distorção de zoom comparando framebuffer lógico com saída WGPU.
4. Completar a evidência de fetch/pipeline de sprites e VRAM antes de alterações de timing.
5. Manter os casos de ZEXALL fora do gate E8 até fechamento formal de E7 do Z80.
