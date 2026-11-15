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

A falha reportada é gráfica: sprites/objetos aparecem incorretos. As capturas anexadas à conversa mostram:

- **The Smurfs: Travel the World (Brazil):** cena de árvore com objetos/personagens e uma faixa horizontal preta larga na região inferior da imagem, além de fragmentos visuais junto à borda direita.
- **Cosmic Spacehead (Europe, En/Fr/De/Es):** cena de plataforma/cenário com elementos visuais aparentemente deslocados ou incorretamente compostos.

As capturas são evidência de reprodução visual, mas não determinam por si só se a origem é avaliação de sprites, composição de prioridade, clipping, scroll, framebuffer, conversão ou apresentação. Não classificar antecipadamente os sintomas como um único defeito.

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

O próprio scorecard do VDP documenta que a fase atual separou a avaliação dos sprites, mas ainda não implementa o buffer persistente de números de sprites da linha seguinte: a lista de candidatos ainda é produzida para a linha renderizada. Isso torna a avaliação/ordem por linha uma hipótese prioritária, mas não prova que seja a causa específica do jogo. Ver `docs/audits/sms-vdp-sprite-evaluation-phase1-2026-11-14.md`.

### B. Cosmic Spacehead (Europe, En/Fr/De/Es)

- O usuário esclareceu que o defeito aparece já na **tela de menu/introdução**, antes do gameplay.
- A ROM fornecida tem CRC32 `29822980`, correspondência exata no catálogo Genesis Plus GX para Cosmic Spacehead, mapper Codemasters, sistema SMS2 e região Europe. Essa correspondência é evidência de catalogação independente.
- O cabeçalho Codemasters no offset `$7FE0` também identifica essa família de cartucho. O runtime já contém detecção do cabeçalho Codemasters e implementação do mapper com bancos iniciais `[0, 1, 0]`; portanto, não presumir que falta implementar o mapper nem reescrever essa parte sem localizar uma divergência de bancos.

**Hipótese prioritária específica:** perfil do VDP/ASIC e modo de altura vertical. Cosmic Spacehead é um título Codemasters PAL europeu e pertence à família SMS2. A documentação de compatibilidade registra que Cosmic Spacehead usa o modo de resolução vertical estendida de 224 linhas, disponível no VDP 315-5246 (SMS2 e revisões PAL tardias), não no 315-5124 SMS1. O core já implementa a seleção de 224 linhas quando R1 bit 4 está ativo e o perfil permite `active_height_modes`; porém `SmsTemporalBus::new_with_psg_config` escolhe por padrão `SmsVdpProfile::sms1_t0_ntsc()`. Precisamos rastrear a seleção de perfil feita pelo launcher/loader e confirmar qual perfil chega à máquina quando essa ROM é carregada. Não forçar SMS2/PAL apenas pelo nome do jogo: a identificação de hardware deve ser explícita e reproduzível.

### Confirmação adicional no caminho real do runtime desktop

A inspeção de `rust/runtime/sms-runtime/src/worker.rs` e `docs/desktop/sms1-active-system-profile.md` confirmou:

- `sms_temporal_region_profile("europe")` seleciona PAL, mas mantém `SmsVdpRevision::Sega3155124` (SMS1).
- O perfil ativo no desktop é SMS1 em todas as regiões; SMS2/315-5246 existe no VDP, mas não está selecionado nem habilitado no runtime desktop.
- Portanto, **a região europeia/PAL não equivale à seleção de SMS2**. A configuração atual não satisfaz o requisito de hardware conhecido para Cosmic Spacehead.
- O VDP já tem suporte interno condicional para 224 linhas, mas a política de runtime/host não seleciona o ASIC necessário. A janela desktop também declara `SMS_HEIGHT = 192`; precisamos rastrear o contrato de publicação e apresentação antes de habilitar 224 linhas para evitar truncamento ou buffer inconsistente.

Isso transforma a hipótese de Cosmic Spacehead em uma divergência concreta de configuração arquitetural, não em uma suspeita genérica de sprites. Próxima investigação deve rastrear a cadeia completa: identidade da ROM → perfil de sistema/cartucho → seleção do ASIC → padrão PAL → dimensões do mailbox/framebuffer → composição da janela. Não corrigir apenas aumentando a altura constante da GUI.

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

## 3. Hipóteses a investigar — sem assumir causalidade

1. Avaliação e seleção de sprites: Y termination, overflow, índice, limite por linha e modo de limite configurado.
2. Renderização Mode 4: atributos de sprite, pattern index, deslocamento X/Y, magnificação e tamanho.
3. Prioridade e composição: sprites atrás do background, cor transparente, colisão e ordem de desenho.
4. Clipping e bordas: pixels fora da área ativa, left-column mask e transições de linha.
5. Scroll horizontal/vertical e mudanças de registradores durante o frame.
6. Preservação/limpeza de linhas do framebuffer, especialmente a faixa preta observada em The Smurfs.
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
- [ ] Faixa preta e fragmentos visuais explicados e corrigidos, se confirmados como defeito do core.
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
