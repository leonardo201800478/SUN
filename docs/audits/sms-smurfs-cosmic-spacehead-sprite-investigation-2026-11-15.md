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
