# ADR 0014 — Pipeline de shaders retro com Slang e WGPU

- **Status:** PROPOSTA — requer spike de integração antes de aceite.
- **Data:** 2026-11-21.
- **Baseline:** main `f65e13c0e85d103faddd31845324432d8641a308`.
- **Escopo:** pós-processamento da imagem já concluída, no host de vídeo Slint/WGPU.
- **Não escopo:** CPU, VDP, PSG, barramentos, MachineScheduler, clocks, áudio ou estado da máquina.

## Contexto

A aplicação desktop utiliza Slint 1.18 com renderer WGPU 30 e backend Vulkan. O host recebe `VideoFrame` imutável, usa `FrameMailbox`, converte RGB888 para RGBA8 e atualiza uma textura importada pelo Slint usando o mesmo `wgpu::Device` e `wgpu::Queue` obtidos em `RenderingSetup`.

A interface ainda identifica “Shaders / CRT / NTSC” como não integrado. Não há carregador de presets Slang nem runtime de shader conectado ao caminho nativo atual.

## Decisão proposta

1. **Candidato principal: `librashader 0.12.1`, runtime Rust `runtime-wgpu`.** Ele implementa o parser de presets `.slangp`, pré-processamento Slang, reflexão e filter chains multi-pass. A integração deve usar o mesmo device/queue WGPU do renderer Slint; não criar um segundo device Vulkan nem introduzir `ash`/ `vulkano` diretamente.
2. **Compatibilidade é um gate, não uma suposição.** A documentação de librashader classifica WGPU como suporte secundário e não garante compatibilidade de todos os shaders nesse backend. A versão 0.12.0 trouxe suporte a WGPU 30; 0.12.1 é a versão de correção mais recente na data desta proposta. O spike deve validar os presets reais no renderer WGPU 30/Vulkan do produto.
3. **Naga não é um carregador Slang.** Naga 30 aceita WGSL, SPIR-V e GLSL 440+ com semântica Vulkan e pode emitir WGSL, mas não interpreta diretamente o formato de preset `.slangp` nem substitui o pré-processador/gerenciador de dependências Slang. Para shaders próprios e pequenos, preferir WGSL nativo; usar Naga para validação/tradução de entradas que ele realmente suporta.
4. **Não copiar a biblioteca inteira de shaders.** Manter um catálogo versionado de candidatos e fixar o commit upstream. Presets podem referenciar muitos passes, includes e LUTs; qualquer distribuição futura precisa trazer o conjunto completo de dependências e uma auditoria de licença por arquivo.
5. **Separação de responsabilidades.** O futuro componente deve ficar na camada de apresentação (por exemplo, crate dedicada `sega-video-shaders`), fora de `sega-video`, dos runtimes SMS/SG-1000 e do Scheduler. A GUI Slint apenas escolhe preset/parâmetros e envia comandos coarse-grained ao host.
6. **PAL-M é um perfil customizado, não um preset PAL pronto.** O preset PAL existente não pode ser declarado PAL-M por simples troca do timing do VDP. O perfil brasileiro exige parâmetros de crominância e validação independentes; até lá, rotular qualquer resultado como aproximação experimental.

## Invariantes obrigatórios

- O shader recebe somente frames completos e imutáveis.
- O pós-processamento nunca escreve no framebuffer original nem altera VRAM, registradores, IRQs, clocks, sequência de eventos ou estado serializável da máquina.
- Nenhuma chamada por instrução/ciclo; a fronteira é frame/presentação.
- Resize e troca de preset recriam somente recursos de apresentação.
- Falhas de parse/compilação/recursos devem retornar ao modo Nearest/sem shader, sem parar ou atrasar o runtime.
- Parâmetros dinâmicos são validados pelo metadata do preset e enviados ao render host; Slint não manipula diretamente objetos GPU.
- Histórico temporal precisa considerar a semântica latest-only do `FrameMailbox`: se houver lacunas de sequência, a política de feedback deve ser explícita (reset, descarte ou consumo de todos os frames). Não fingir persistência física determinística quando frames intermediários foram descartados.

## Perfis candidatos

A lista e os caminhos upstream exatos estão em [catalog.toml](../shaders/catalog.toml). O catálogo é pesquisa, não implementação ou promessa de compatibilidade.

## Fases e gates

- **SH-0 — esta PR:** decisão, inventário de presets, revisão de licenças e contrato arquitetural. Nenhuma dependência ou shader é embarcado.
- **SH-1 — spike WGPU:** feature isolada com `librashader 0.12.1/runtime-wgpu`; construir e executar um preset simples no device/queue Slint. Confirmar Vulkan efetivo, lifecycle, resize, falhas e shutdown. Comparar o resultado com um caminho de referência.
- **SH-2 — assets e catálogo:** selecionar presets suportados, copiar dependências completas, fixar revisão upstream e gerar notices de licença.
- **SH-3 — filtros espaciais:** Nearest, sharp-bilinear, bilinear e CRT RGB/NTSC; testes de pixels, escala, cor, viewport e custo de GPU.
- **SH-4 — filtros temporais:** feedback/history, Game Gear LCD e `mix_frames`; validar sequência de frames, perda de frames, reset de histórico e estabilidade de estado.
- **SH-5 — UI:** ComboBox, parâmetros dinâmicos, persistência e telemetria de falhas/custo.

Nenhuma fase é considerada concluída por compilar apenas. SH-1 exige execução real no Windows x64 com Vulkan confirmado.

## Licenças e distribuição

A crate librashader e cada shader upstream têm seus próprios termos. O repositório `libretro/slang-shaders` não deve ser tratado como um único pacote sob uma licença uniforme. Antes de embarcar qualquer preset, shader, include ou LUT, identificar a licença e copyright de cada arquivo e preservar notices. Esta proposta não copia nem distribui código shader upstream.

## Referências

- https://docs.rs/librashader/0.12.1/librashader/
- https://github.com/SnowflakePowered/librashader/releases/tag/librashader-v0.12.1
- https://docs.rs/naga/30.0.2/naga/
- https://docs.libretro.com/development/shader/slang-shaders/
- https://github.com/libretro/slang-shaders/tree/02bf134555c4595bd6b69be89feaff923da83754
- ADR 0011 — [Slint + WGPU + CPAL](0011-slint-wgpu-cpal-desktop-platform.md)
