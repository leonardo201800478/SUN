# Arquitetura de shaders de vídeo

**Estado:** preparação documental SH-0. Nenhum runtime de shader está integrado.

## 1. Caminho nativo atual

1. O VDP/runtime produz um `VideoFrame` completo e imutável.
2. `FrameMailbox` entrega o frame mais recente ao host; frames pendentes podem ser substituídos por um frame mais novo, sem alterar a execução da máquina.
3. O host Slint recebe o `wgpu::Device` e `wgpu::Queue` efetivos em `RenderingSetup`, verifica Vulkan e cria uma textura importada pelo Slint.
4. O host converte RGB888 em RGBA8 e atualiza a textura com `queue.write_texture`.
5. Slint compõe a imagem na janela.

O item “Shaders / CRT / NTSC” da UI continua marcado como não integrado. Não há carregador `.slangp`, cadeia multi-pass ou shader CRT/NTSC conectado ao host Rust atual.

## 2. Arquitetura-alvo

```text
SMS / SG-1000 runtime
        |
        v
VideoFrame imutável (RGB888)
        |
        v
FrameMailbox / contrato de frame
        |
        v
Host de apresentação Slint + WGPU 30 (Vulkan)
        |
        v
Upload da textura fonte RGBA8
        |
        v
sega-video-shaders (futuro)
  - librashader FilterChain WGPU
  - passes intermediários e LUTs
  - feedback/history quando habilitado
  - parâmetros refletidos e validados
        |
        v
Textura de saída importável pelo Slint
        |
        v
Composição da UI e apresentação
```

O componente de shader é consumidor da imagem concluída. Ele não é uma nova camada do runtime de hardware nem uma extensão do `MachineScheduler`.

## 3. Runtime e API gráfica

O candidato principal é `librashader 0.12.1` com o recurso Rust `runtime-wgpu`. A API documenta o fluxo `ShaderPreset → FilterChain → FilterChain::frame`, além de módulos para presets, pré-processamento, reflexão e runtime.

A integração deve compartilhar o device/queue WGPU que Slint já utiliza. O uso do runtime Vulkan direto de librashader exigiria uma ponte de ownership/handles entre Vulkan e o device gerido por WGPU; não introduzir essa ponte nem um segundo device sem um estudo específico.

A ressalva importante: librashader classifica WGPU como backend de suporte secundário e declara que a compatibilidade dos shaders não é garantida em APIs secundárias. O release 0.12.0 trouxe suporte a WGPU 30; 0.12.1 é o patch atual. Por isso, SH-1 deve testar cada preset escolhido no caminho real WGPU 30/Vulkan. Não assumir que “Vulkan funciona” implica que o runtime WGPU é equivalente ao runtime Vulkan de librashader.

### Naga

Naga 30.0.2 é a ferramenta do ecossistema WGPU para validar/traduzir WGSL, SPIR-V e GLSL 440+ com semântica Vulkan. Não é um parser de `.slangp`, não resolve o grafo de passes e dependências dos presets RetroArch e não compila diretamente a biblioteca Slang como substituto de librashader. É adequado para shaders WGSL próprios ou fontes GLSL/SPIR-V compatíveis com seus frontends.

## 4. Contrato de frame e GPU

- Entrada: somente `VideoFrame` completo, com dimensões, formato e stride validados.
- Formato GPU inicial: RGBA8, conforme a textura importada atual.
- A conversão RGB888→RGBA8 deve ocorrer uma vez; evitar cópias adicionais desnecessárias.
- A cadeia multi-pass possui texturas intermediárias próprias e uma textura final importável pelo Slint.
- A textura original não é modificada in-place; o caminho sem shader deve permanecer disponível.
- Resize, mudança de preset e recriação do renderer invalidam/recriam recursos GPU de apresentação, sem resetar o console.
- Erros de shader devem degradar para apresentação direta/Nearest e ser reportados ao host; não propagar para o runtime de hardware.

## 5. Histórico e determinismo visual

Presets com feedback consultam a saída de um frame anterior. A especificação Slang documenta feedback para efeitos como motion blur, persistence e ghosting. Isso não equivale automaticamente à simulação física de um LCD.

O `FrameMailbox` atual é single-slot/latest-only. Quando o host está atrasado, frames intermediários podem ser substituídos. Um shader temporal deve, portanto:

1. acompanhar `VideoFrame.sequence`;
2. detectar saltos de sequência e definir uma política explícita para o histórico;
3. limpar ou reconstruir o histórico ao trocar preset, ROM, sistema, dimensão ou após reset;
4. nunca fazer backpressure no worker/Scheduler;
5. declarar que o efeito é baseado em frames apresentados quando não consome todos os frames emulados.

Para uma futura simulação temporal mais fiel, avaliar uma fila de frames concluídos, limitada e desacoplada do Scheduler, ou uma política de atualização do histórico por frame de entrada. Essa decisão pertence à etapa SH-4, não a SH-0.

## 6. Catálogo inicial

O arquivo [`docs/shaders/catalog.toml`](../shaders/catalog.toml) fixa uma revisão upstream e lista os candidatos. Os caminhos foram verificados no repositório oficial em 2026-11-21.

| Perfil | Candidato | Observação |
|---|---|---|
| Pixels puros | `nearest.slangp` | Referência de pixels sem interpolação |
| Sharp bilinear | `pixel-art-scaling/sharp-bilinear-simple.slangp` | Para escalas não inteiras |
| Bilinear | `bilinear.slangp` | Interpolação linear |
| CRT RGB | `crt/crt-aperture.slangp`, `crt/crt-hyllian.slangp` | Perfil espacial RGB |
| CRT PVM/RGB | `presets/crt-royale-fast/4k/crt-royale-pvm-rgb.slangp` | Candidato mais complexo; medir custo |
| Composite NTSC | `crt/crt-guest-advanced-ntsc.slangp`, `ntsc/ntsc-adaptive.slangp` | Cadeias de passes e LUTs; validar compatibilidade |
| PAL | `crt/crt-guest-advanced-pal.slangp` | Perfil PAL padrão, não PAL-M |
| Game Gear LCD | `handheld/lcd-grid-v2.slangp` | Grade de subpixels |
| Ghosting/feedback | `motionblur/mix_frames.slangp` | O nome upstream usa underscore; requer histórico |
| Zfast LCD | `handheld/zfast_lcd.slang` | Shader source; não é preset `.slangp` independente |

### PAL-M brasileiro

Não há preset PAL-M pronto no catálogo verificado. O preset PAL padrão não deve ser rotulado como PAL-M. Uma variante PAL-M exigirá parâmetros explícitos para o sinal de cor brasileiro, preservando o timing NTSC definido pelo perfil de hardware. O resultado só poderá ser chamado de PAL-M fiel após validação independente de frequência de crominância, fase por linha, amostragem, cores e padrões de dithering. Antes disso, usar “PAL-M experimental/aproximado”.

## 7. Parâmetros e interface Slint

Os metadados `#pragma parameter` definem nome, valor padrão, mínimo, máximo e passo. O runtime deve refletir os parâmetros do preset, validar os valores e expor apenas controles suportados.

A GUI Slint não acessa `wgpu::Device`, `Queue` ou objetos de filter chain. Ela envia comandos coarse-grained ao host, por exemplo:

- `load_preset(id)`
- `set_shader_parameter(name, value)`
- `reset_shader_history()`
- `disable_shader()`

O host aplica mudanças na fronteira segura do frame e persiste apenas IDs/valores validados, nunca caminhos arbitrários não confiáveis.

## 8. Assets, segurança e licenças

- Fixar a revisão do repositório upstream; não baixar `master` em tempo de execução.
- Não embarcar o repositório completo.
- Preset `.slangp` pode referenciar vários `.slang`, includes, LUTs e texturas. Um preset sem todos os recursos dependentes não é um pacote válido.
- Auditar licença/copyright por arquivo, inclusive includes e LUTs, e gerar notices antes da redistribuição.
- Validar caminhos relativos e impedir traversal para fora da raiz de assets.
- Limitar tamanho, quantidade de passes, dimensões de framebuffer e memória de LUTs; reportar erro se limites forem excedidos.
- Não permitir download remoto automático de shaders sem consentimento/configuração explícita.

## 9. Plano de implementação

| Etapa | Entrega | Gate |
|---|---|---|
| SH-0 | ADR, catálogo e contrato | Documentação consistente; sem mudança no runtime |
| SH-1 | Spike isolado librashader/WGPU | Windows x64, Vulkan efetivo, device/queue compartilhados, preset simples, resize/shutdown |
| SH-2 | Pacote de presets licenciados | Grafo de dependências completo, revisão upstream fixada, notices |
| SH-3 | Filtros espaciais | Imagens sintéticas, pixel tests, escala, cor, desempenho |
| SH-4 | Feedback temporal/Game Gear | Sequência de frames, saltos, reset de history e política latest-only |
| SH-5 | UI e persistência | Parâmetros refletidos, validação, regressão de preferências |

Nenhuma etapa pode alterar CPU, VDP, PSG, barramentos, relógios ou Scheduler. Renderização, shader e saída de áudio permanecem consumidores de frames concluídos.
