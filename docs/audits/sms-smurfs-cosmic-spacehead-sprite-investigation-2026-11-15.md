# SMS — investigação de corrupção gráfica em The Smurfs e Cosmic Spacehead

**Estado:** INVESTIGATION_OPEN
**Escopo:** sprites/objetos e composição de imagem no VDP SMS
**Runtime:** SMS via Slint 1.18 + WGPU 30/Vulkan
**Data de abertura:** 2026-11-15

## 1. Relato de reprodução no Windows

O usuário executou os dois jogos no desktop atual. Ambos:

- inicializam e avançam para gameplay;
- possuem áudio;
- aceitam controles;
- apresentam comportamento de colisão/gameplay suficiente para serem jogáveis.

A falha reportada é gráfica: sprites/objetos aparecem incorretos. O relato atualizado do usuário delimita os sintomas:

- **The Smurfs: Travel the World (Brazil):** o defeito é a **ordem visual dos sprites durante o gameplay**. Não há relato de que o menu inicial apresente o mesmo defeito. Não tratar faixa preta, clipping ou corrupção de tiles como sintomas confirmados sem nova evidência.
- **Cosmic Spacehead (Europe, En/Fr/De/Es):** a corrupção gráfica já aparece na tela de introdução/menu, antes do gameplay.

Capturas antigas são evidência visual auxiliar, não substituem a reprodução por estado de jogo nem determinam sozinhas a causa. As duas linhas permanecem independentes.

## 2. Telemetria observada

| Jogo | FPS UI upload | Contador VDP frame | Dimensão/formato |
|---|---:|---:|---|
| The Smurfs: Travel the World | 59,8 | 8.043 | 256x192 RGB888 → RGBA8 |
| Cosmic Spacehead | 59,4 | 10.533 | 256x192 RGB888 → RGBA8 |

A interface reportou Vulkan, NVIDIA GeForce RTX 5070 e device/queue compartilhados entre Slint/WGPU. O contador de upload não é uma medição de scanout físico nem prova de correção da imagem.

## 2.1 Identificação criptográfica das ROMs fornecidas

As duas ROMs foram materializadas apenas no ambiente de análise para calcular hashes. Não foram adicionadas ao repositório.

| Jogo | Tamanho | SHA-1 | SHA-256 | CRC32 |
|---|---:|---|---|---|
| The Smurfs: Travel the World (Brazil) | 262.144 bytes | `15b882ad3263cdc1e78c732d96f9bb18e25f4743` | `f3d493c5a673f78213f7544b3d9bacb1601721490d65e6bd041f4e01e5c6f85e` | `E14C8A1C` |
| Cosmic Spacehead (Europe) (En,Fr,De,Es) | 262.144 bytes | `f46f716dd34a1a5013a2d8a59769f6ef7536a567` | `1377189c097116fd49f9298368fd3138a28fbe48a08ff7b097d9360ac891bb9b` | `29822980` |

## 2.2 Emuladores de referência e reprodutibilidade

A matriz de comparação será Sega Emulator × Ares × Gearsystem. Os builds de referência devem ser feitos no workflow, a partir de revisões fixadas (não baixar binários opacos nem usar `latest`):

- **Ares:** `ares-emulator/ares@0b928930af4cc59ed99e942996f0d3c815379388` (revisão consultada em 2026-11-15).
- **Gearsystem:** `drhelius/Gearsystem@96b70aac107a1a2acace9091cce37f9649eb6e0e` (revisão consultada em 2026-11-15).

O workflow deve compilar os dois projetos e publicar os executáveis/logs como artifacts. As ROMs comerciais não devem ser commitadas nem publicadas como artifacts. A execução comparativa com essas ROMs ocorre no ambiente local autorizado, usando os binários produzidos pelo workflow e os hashes desta seção. Para automatizar execução integral dentro de CI será necessário um mecanismo privado de fornecimento das ROMs; não usar URLs públicas, segredos de tamanho inadequado ou conteúdo comercial no repositório.

Ares é uma aplicação desktop; não presumir suporte a headless/screenshot CLI. Gearsystem documenta `--headless` com MCP/Link Cable. O método de captura precisa respeitar as capacidades reais de cada build, e as imagens só serão comparadas após normalizar região, overscan, escala e estado de gameplay.

## 2.3 Workflow de compilação dos comparadores

Foi adicionado `.github/workflows/sms-reference-emulators.yml`. O workflow compila, em jobs Linux x86_64 separados:

- Gearsystem na revisão `96b70aac107a1a2acace9091cce37f9649eb6e0e`, com SDL3 fixado em `877399b2b2cf21e67554ed9046410f268ce1d1b2`.
- Ares na revisão `0b928930af4cc59ed99e942996f0d3c815379388`, configurado para compilar somente o core SMS (`ARES_CORES=ms`).

Os executáveis e arquivos de proveniência são publicados como artifacts separados, com retenção de 14 dias. A compilação de cada comparador é uma verificação independente; ela não prova equivalência gráfica nem execução das ROMs.

### Limite deliberado do CI

As ROMs não são armazenadas em Git, cache ou artifacts. Portanto, este workflow compila e empacota os comparadores, mas não executa as ROMs comerciais no runner. A execução comparativa ocorre localmente, com as ROMs fornecidas pelo usuário. Não habilitar secret ou URL para redistribuir ROMs.

**Resultado de 2026-11-15:** o workflow de referência concluiu com sucesso (run #8): Gearsystem e Ares foram compilados em revisões fixadas. O job Ares passou após instalar SDL3 na mesma revisão fixada usada pelo job Gearsystem. A CI principal da PR #881 também concluiu com sucesso (run #7599), incluindo higiene documental.

### Protocolo de execução local após baixar os artifacts

1. Extraia os artifacts `ares-linux-x86_64` e `gearsystem-linux-x86_64` em ambiente Linux compatível com as bibliotecas listadas no artifact.
2. Execute Gearsystem com cada ROM usando a interface desktop ou MCP headless. Para MCP HTTP local, use uma instância por ROM e porta distinta; não compartilhe a mesma instância entre agentes.
3. Execute Ares com cada ROM pela interface desktop. Não assumir que Ares oferece screenshot CLI/headless; registrar screenshots manuais e a revisão exata.
4. No Sega Emulator, capture o framebuffer bruto RGB888 do core e a imagem final da janela no mesmo estado de jogo.
5. Alinhe as cenas por estado de gameplay, não apenas por número absoluto de frame. Normalize área visível/overscan, escala e proporção antes do diff.
6. Guarde screenshots, dumps e logs de VRAM/CRAM/registradores em armazenamento local privado; publique no PR apenas relatórios textuais sem conteúdo de ROM.

**Critério de comparação:** a imagem final de outro emulador é uma referência empírica. A regra de hardware só será adotada após corroborar com documentação técnica, testes de VDP ou comportamento conhecido do ASIC.

## 2.4 Refinamento do diagnóstico após esclarecimento do usuário

O usuário esclareceu que os sintomas são diferentes e devem ser tratados como duas linhas independentes:

### A. The Smurfs: Travel the World (Brazil)

- O jogo entra em gameplay e funciona; o problema é a **ordem visual dos sprites durante o gameplay**.
- Não há relato de falha no menu de introdução.
- Prioridade de investigação: ordem do SAT, buffer de oito sprites por linha, ordem de composição dos pixels transparentes/opacos, prioridade sprite/background e fase em que os números dos sprites são avaliados para a linha seguinte.
- Não classificar como corrupção de tiles, mapper ou problema de upload sem evidência.

A auditoria do código atual em main corrigiu a leitura histórica anterior: **o buffer persistente de sprites da linha seguinte já existe para o caminho SMS1**. Não devemos reimplementar essa etapa.

Evidências no código/testes atuais:
- `Mode4SpriteLineBuffer` armazena a avaliação destinada à linha seguinte.
- A avaliação percorre o SAT em ordem de índice e preserva os metadados selecionados (Y/linha, X e tile/pattern select).
- A renderização SMS1 consome a avaliação latched para a linha-alvo.
- Existem testes para ordem SAT, limite de oito sprites, overflow do nono, latch de Y/X/tile, prioridade de sprite/background, transparência, colisão e left-column mask.

Testes já presentes incluem `mode4_sprite_evaluation_preserves_sat_order_and_next_line_y_rule`, `sms1_hardware_sprite_buffer_selects_eight_and_overflows_on_ninth`, `sms1_sprite_x_tile_shift_and_pattern_select_are_latched_with_sat_metadata`, `sms1_opaque_sprite_overlap_sets_collision_and_preserves_earlier_sprite`, `sms1_transparent_sprite_overlap_does_not_set_collision` e `sms1_sprite_priority_respects_opaque_high_priority_background`.

Portanto, a hipótese não é mais “falta de buffer persistente”. A investigação deve localizar a primeira divergência durante o gameplay: confirmar o perfil efetivo, registrar SAT/VRAM/registradores por linha, comparar a lista selecionada com os pixels compostos e verificar se a ordem SAT observada pelo core corresponde ao comportamento de hardware. Referências de implementação: `docs/audits/sms1-sprite-line-buffer-phase2-2026-11-14.md`, `docs/audits/sms1-sprite-overflow-evaluation-phase3-2026-11-14.md` e `docs/audits/sms1-vdp-runtime-path-audit-2026-11-14.md`.

### B. Cosmic Spacehead (Europe, En/Fr/De/Es)

- O usuário esclareceu que o defeito aparece já na **tela de menu/introdução**, antes do gameplay.
- A ROM fornecida tem CRC32 `29822980`, correspondência exata no catálogo Genesis Plus GX para Cosmic Spacehead, mapper Codemasters, sistema SMS2 e região Europe. Essa correspondência é evidência de catalogação independente.
- O cabeçalho Codemasters no offset `$7FE0` também identifica essa família de cartucho. O runtime já contém detecção do cabeçalho Codemasters e implementação do mapper com bancos iniciais `[0, 1, 0]`; portanto, não presumir que falta implementar o mapper nem reescrever essa parte sem localizar uma divergência de bancos.

**Hipótese prioritária específica:** perfil do VDP/ASIC e modo de altura vertical. Cosmic Spacehead é um título Codemasters europeu catalogado como SMS2 e utiliza resolução estendida de 224 linhas.

**Correção já integrada:** PR #884 foi incorporada à main no commit `7a8f1dd72f9ea87e0625fb178a031bc0563d09ca`. O decodificador compartilhado agora exige R0.M4 + R0.M2 e seleciona 224/240 por R1.M1/R1.M3 mutuamente exclusivos; ambos os seletores ligados mantêm 192 linhas. SMS1 continua limitado a 192. Os testes de VDP, Clippy e build Release do SMS runtime passaram na CI aplicável.

**Limitação ainda aberta:** `SmsTemporalBus::new_with_psg_config` e `sms_temporal_region_profile` continuam selecionando SMS1 315-5124 no caminho desktop para todas as regiões. PAL não seleciona SMS2. A política do runtime precisa receber uma revisão de ASIC explícita e reproduzível, independente da região e sem hacks por ROM.

### Confirmação adicional no caminho real do runtime desktop

A inspeção de `rust/runtime/sms-runtime/src/worker.rs` e `docs/desktop/sms1-active-system-profile.md` confirmou:

- Antes da PR #892, `sms_temporal_region_profile("europe")` selecionava PAL, mas mantinha `SmsVdpRevision::Sega3155124` (SMS1).
- A região europeia/PAL não equivale à seleção de SMS2. A máquina precisa receber a revisão 315-5246 explicitamente.
- O VDP já possui decodificação condicional de 224/240 linhas; a PR #890 fechou a rejeição de dimensões no host.
- A PR #892 introduz a seleção explícita persistida e a propagação do ASIC até o worker, mas permanece sob CI/revisão e ainda não está integrada à main.

A PR #890 foi integrada na main em 2026-11-15 (merge commit `999a7391cd0a827f7035f8962e3a04ae4c985b76`). O host desktop deixou de rejeitar frames que não sejam 256×192: agora aceita 256×192/224/240, recria a textura importada pelo Slint quando a extensão muda e atualiza `bytes_per_row`, `rows_per_image` e `Extent3d` com as dimensões reais. A CI completa da PR passou, incluindo build Windows x64 Slint/WGPU 30/Vulkan.

Essa integração fecha a incompatibilidade estática de dimensão na cadeia de apresentação, mas não comprova ainda uma sessão real em Windows com Cosmic Spacehead em 224 linhas. A seleção explícita do ASIC no desktop continua sendo a lacuna funcional: PR #892 está aberta e propaga a revisão do VDP até o worker; aguarda validação completa.

A cadeia a validar agora é: identidade/configuração selecionada → ASIC efetivo → VDP → SMS runtime/temporal worker → mailbox/framebuffer dinâmico → textura WGPU → imagem Slint.

### Evidências técnicas e fontes

- SMS Power, Codemasters mapper: três slots de 16 KiB, registradores em `$0000/$4000/$8000`, primeiro 1 KiB do slot 0 paginável e estado inicial `[0,1,0]`.
- SMS Power, sprites: até oito sprites por linha; a ordem no buffer interno determina qual sprite opaco prevalece.
- SMS Power, Codemasters header: assinatura e metadados a partir de `$7FE0`; data de compilação de Cosmic Spacehead em 31/08/1993.
- Genesis Plus GX: CRC `29822980` catalogado como `MAPPER_CODIES`, `SYSTEM_SMS2`, `REGION_EUROPE`.
- Core atual: `native/sms_core/src/memory.rs` detecta o cabeçalho Codemasters e implementa o mapper; `native/video_core/src/sms/vdp.rs` implementa 224 linhas sob capacidade do perfil; `SmsTemporalBus::new_with_psg_config` usa perfil SMS1 T0 NTSC por padrão.

Referências públicas:
- https://www.smspower.org/Development/Mappers
- https://www.smspower.org/Development/Sprites
- https://www.smspower.org/Development/CodemastersHeader
- https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/cart_hw/sms_cart.c
- https://consolemods.org/wiki/Genesis:Master_System_Games_Info

## Decisão de escopo — 2026-11-15

A PR permanece **aberta e em Draft** como registro contínuo de validação dos dois títulos. Os problemas não serão tratados como uma única falha gráfica.

### Cosmic Spacehead: trilha SMS2

O projeto precisa construir e validar SMS2 como perfil de hardware real, não apenas habilitar uma resolução maior. Cosmic Spacehead será uma ROM de validação funcional desse perfil, mas não a única evidência de conformidade.

Escopo mínimo da trilha SMS2:

1. Especificar diferenças documentadas entre VDP SMS1 (315-5124) e SMS2 (315-5246), distinguindo ASIC/revisão, região e clock de vídeo.
2. Implementar seleção explícita e reproduzível do perfil de hardware. PAL não implica automaticamente SMS2; SMS2 não pode ser inferido apenas do país da ROM.
3. Auditar modos de 192/224/240 linhas, R1 e permissões por perfil, timing de HINT/VINT/VBlank, status/IRQ, composição e publicação do framebuffer.
4. Propagar dimensões e contratos do frame de ponta a ponta: VDP → temporal runtime/mailbox → video output → Slint/WGPU. Não deixar constantes fixas de 192 linhas truncarem um frame 224/240.
5. Auditar diferenças de sprites, scroll, clipping, palette/CRAM e comportamento de portas entre revisões.
6. Criar testes unitários/contratuais sintéticos para cada diferença documentada e testes de integração por perfil.
7. Validar com ROMs de teste VDP e jogos comerciais conhecidos, incluindo Cosmic Spacehead, sem hacks específicos por ROM.
8. Preservar SMS1/SG-1000 e demonstrar ausência de regressões.

### The Smurfs: trilha de ordenação de sprites

O relato atual é específico: os sprites ficam fora de ordem durante o gameplay. Não há relato de falha no menu inicial. Investigar primeiro SAT order, sprite line evaluation/buffer, prioridade entre sprites, transparência e composição por pixel. Não misturar essa investigação com a construção SMS2.

### Critério de saída

A PR só deve ser fechada quando ambas as trilhas tiverem diagnóstico, correção validada e testes de regressão, ou quando forem separadas em PRs de implementação com referências cruzadas e esta auditoria permanecer como índice de validação. Nenhum merge é presumido por esta decisão.

## 2.5 Lacunas da suíte SMS VDP — baseline independente

O arquivo comparativo anterior registra para SMS VDP Test v1.31: Data 17/19, Misc 11/16 e Sprites 17/19 no Sega Emulator; Ares registra 19/19, 11/16 e 16/19, respectivamente. O Sega Emulator já supera Ares em X-Scroll latchtime e no caso de overflow de sprites testado. Esses resultados são baseline, não uma especificação de comportamento.

A inspeção do código-fonte original de FluBBa confirma que o teste SMS é descrito como executado em hardware PAL SMS1 (50/60 Hz) e PAL SMS2 (50 Hz). Os casos Data que falham no comparativo são:

- `1byte wr sets rd adr` — RunTest11: após configurar leitura em 3E58, escreve apenas o byte baixo 0x10 na porta de controle e valida a sequência de leitura em 3E10.
- `1byte wr sets wr adr` — RunTest12: após configurar escrita em 3E72, escreve apenas bytes baixos 0x30, 0x00, 0x20 e 0x10 e valida os destinos de escrita.
- `1byte wr keeps wr mode` — RunTest13: verifica que a escrita de um único byte de controle não muda o destino CRAM/VRAM.

O código atual de `write_control()` apenas guarda o primeiro byte em `control_first`; ele ainda não atualiza o registrador de endereço no primeiro byte. Essa lacuna foi corrigida pela PR #889, integrada no commit `825c1cb4723a7fb931d720247ea797428495e041`. O primeiro byte agora atualiza o endereço baixo; em modo de leitura reinicia o read-ahead, enquanto VRAM/CRAM write preservam o destino sem incremento antecipado. RunTest11/12/13 e o contrato público SMS1 foram cobertos. CI VDP e SMS runtime passaram. Não misturar essa correção com o decodificador de altura já integrado.

Outras falhas SMS Misc a cobrir por teste contratual: `VCounter chg time`, `Frame IRQ VCount`, `Frame IRQ HCount`, `Line IRQ VCount`, `Line IRQ HCount` e `VINT flag HCount`. Falhas SMS Sprite ainda listadas: `Offscreen Y, col` e `Spr col correct HC`. O teste `X-Scroll latchtime` já passa no Sega Emulator e não deve ser regredido para igualar Ares.

Fontes:
- Comparativo versionado no projeto: `comparativo_de_testes_vdp_sega_emulator_vs_ares.md`.
- Test ROM e código-fonte: https://github.com/FluBBaOfWard/SmsVdpTest
- Histórico de validação em hardware: https://www.smspower.org/forums/10695-SMSVDPTester

## 2.6 Estado das correções em revisão

- **PR #884 — integrada:** decodificação dos gates de altura SMS2 (R0.M4/R0.M2 e seleção exclusiva R1.M1/R1.M3).
- **PR #889 — integrada:** latch do primeiro byte da porta de controle, read-ahead e preservação do modo; testes VDP/SMS runtime aprovados.
- **PR #890 — integrada:** dimensões dinâmicas 256×192/224/240 no host Slint/WGPU; CI completa e Windows x64 POC passaram.
- **PR #891 — integrada:** matriz documental de diferenças SMS1/SMS2 na main.
- **PR #892 — aberta:** seleção persistente SMS1/SMS2 e propagação explícita do ASIC até o worker; CI em execução após correções de integração.
- **PR #893 — integrada:** HINT/VINT/VCounter e publicação VBlank para o 315-5246; HBlank/raster writes SMS2 continuam abertos.
- **PR #894 — aberta:** compartilha o buffer SAT de Mode 4 entre 315-5124 e 315-5246, preservando zoom por revisão; validação de sprites em hardware e The Smurfs pendentes.
- **PR #881 — esta auditoria:** permanece aberta e em Draft como índice das duas investigações e da validação independente. O workflow de comparadores compila Ares e Gearsystem em revisões fixadas, mas não executa ROMs comerciais nem produz comparação visual automática.

Ainda não existe correção integrada na main para a seleção explícita do ASIC no desktop, para a ordenação visual de sprites de The Smurfs ou para as demais falhas de timing listadas em 2.5.

## 3. Hipóteses a investigar — sem assumir causalidade

1. Avaliação e seleção de sprites: Y termination, overflow, índice, limite por linha e modo de limite configurado.
2. Renderização Mode 4: atributos de sprite, pattern index, deslocamento X/Y, magnificação e tamanho.
3. Prioridade e composição: sprites atrás do background, cor transparente, colisão e ordem de desenho.
4. Clipping e bordas: pixels fora da área ativa, left-column mask e transições de linha.
5. Scroll horizontal/vertical e mudanças de registradores durante o frame.
6. Verificar preservação/limpeza de linhas do framebuffer somente se o dump RGB888 bruto demonstrar divergência; faixa preta não é um sintoma confirmado no relato atualizado de The Smurfs.
7. Diferenças de perfil SMS1/SMS2, padrão de vídeo/região e temporização de avaliação.
8. Conversão RGB888→RGBA8 e upload WGPU, somente se o framebuffer produzido pelo core estiver correto e a imagem apresentada divergir.

Não modificar o frontend ou inserir compensações de GUI antes de comparar os bytes do framebuffer produzido pelo VDP com a imagem efetivamente apresentada.

## 4. Protocolo de diagnóstico

Para cada jogo:

1. Registrar identificação da ROM, tamanho, SHA-1, SHA-256 e CRC32 localmente; não versionar ROM comercial.
2. Registrar região, perfil de VDP, padrão de vídeo, SpriteLimitMode e configuração efetiva do runtime.
3. Reproduzir a mesma cena em Ares e no Sega Emulator, com região, resolução e estado de jogo comparáveis.
4. Capturar frame bruto RGB888 diretamente do mailbox/runtime e screenshot final da janela no mesmo instante.
5. Comparar por linha e por pixel; identificar a primeira linha/área divergente, sem reduzir a análise a contagem de bytes não zero.
6. Registrar VRAM/CRAM e registros VDP relevantes, incluindo R0–R10 e dados da tabela de atributos de sprites, em pontos reproduzíveis.
7. Inspecionar trace de leitura/escrita de registradores e eventos raster quando disponíveis.
8. Criar testes sintéticos mínimos para cada regra confirmada e testes de regressão por ROM/cena.
9. Rodar os testes direcionados de vídeo e SMS core, workspace e CI no SHA exato.
10. Repetir a validação manual no Windows com gameplay, áudio e controles.

## 5. Critérios de aceite

- [ ] ROMs identificadas por hashes, sem serem adicionadas ao repositório.
- [ ] Cenas de referência reproduzíveis e capturas pareadas com Ares.
- [ ] Origem de cada artefato localizada no core VDP, no framebuffer ou na apresentação.
- [ ] Regras de sprite/prioridade/clipping/scroll relevantes demonstradas por teste.
- [ ] Ordem visual dos sprites de The Smurfs corrigida durante o gameplay e coberta por regressão; artefatos adicionais só entram no aceite se forem reproduzidos e medidos.
- [ ] Testes de regressão para os dois jogos e casos sintéticos.
- [ ] SMS1, SMS2 e SG-1000 sem regressões.
- [ ] CI verde no SHA final.
- [ ] Aceite manual no Windows.

## 6. Restrições arquiteturais

- Não alterar Slint/WGPU para mascarar defeitos do framebuffer.
- Não introduzir frame skipping, hacks temporais, sleeps ou clocks na GUI.
- Não reescrever o Scheduler nem modificar o Z80 sem evidência específica.
- Preservar o runtime determinístico e a separação entre hardware e apresentação.
- Não considerar áudio, controles ou colisão funcionais como prova de renderização correta.
- Não tratar o defeito como regressão introduzida pela migração sem comparação com o host anterior.

## 7. Evidências iniciais

As duas capturas de gameplay foram fornecidas pelo usuário em 2026-11-15. O relato funcional (jogabilidade, som, controles e colisões) também é do usuário. A análise causal e a reprodução independente ainda estão pendentes.
