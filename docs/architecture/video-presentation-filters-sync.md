# Contrato de apresentação de vídeo e filtros

**Baseline verificada:** main `f65e13c0e85d103faddd31845324432d8641a308` — 2026-11-21.

## Caminho de vídeo implementado

1. O runtime publica um `VideoFrame` completo, imutável e com metadados de formato/dimensões/stride.
2. `FrameMailbox` mantém o frame mais recente para apresentação. Se o host não consumir o frame pendente, um frame concluído mais novo pode substituí-lo. Essa coalescência não altera a execução do Scheduler.
3. O host desktop Slint 1.18 obtém o device/queue WGPU 30 do renderer e confirma o backend Vulkan.
4. O host converte RGB888 em RGBA8 e atualiza uma textura importada pelo Slint com `queue.write_texture`.
5. Slint compõe a imagem com a UI.

A UI mantém o item “Shaders / CRT / NTSC” marcado como **Não integrado**. Nesta baseline, não existe carregador de presets `.slangp`, runtime librashader, cadeia multi-pass ou shader CRT/NTSC conectado ao host Rust.

## O que não deve ser considerado implementado

- Presets RetroArch/Slang ou carregamento de `.slangp`.
- Filtros selecionáveis Nearest, Sharp Bilinear, Bilinear, CRT RGB, NTSC ou PAL.
- Máscara de fósforo, scanlines, curvatura, simulação de subportadora composta ou PAL-M.
- LCD grid ou ghosting temporal do Game Gear.
- Parâmetros dinâmicos de shader expostos à UI.

Referências históricas a filtros CPU/NumPy ou hosts Qt pertencem a caminhos anteriores e não comprovam suporte no caminho nativo Slint/WGPU.

## Invariantes de apresentação

- O pipeline recebe somente frames concluídos e não muta o frame original.
- Nenhum filtro pode alterar registradores, VRAM, interrupções, clocks, barramentos, sequência de eventos ou estado do runtime.
- A GUI não controla ciclos de hardware e não faz callbacks por instrução.
- Resize/recriação de recursos afeta somente a apresentação.
- Falhas de shader devem voltar ao caminho de apresentação direta, sem interromper a máquina.

## Histórico temporal

O `FrameMailbox` é latest-only. Efeitos que usam feedback do frame anterior precisam acompanhar `VideoFrame.sequence` e definir comportamento para lacunas. Se o host perder frames, o histórico não representa todos os frames emulados. Não declarar ghosting físico fiel sem uma política que trate explicitamente frames descartados, troca de preset e reset de histórico.

## Próxima arquitetura

A proposta para integrar presets Slang através de librashader/WGPU, com catálogo fixado, histórico, parâmetros, licenças e gates de validação, está em:

- [ADR 0014 — pipeline de shaders retro](../adr/0014-retro-shader-pipeline-librashader-wgpu.md)
- [Arquitetura do pipeline de shaders](video-shader-pipeline.md)
- [Catálogo de candidatos](../shaders/catalog.toml)

A etapa SH-0 é documental. Nenhuma dependência, asset ou funcionalidade de shader foi adicionada ao binário.
