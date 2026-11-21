# Catálogo de shaders retro

Este diretório contém somente o catálogo de pesquisa SH-0. Nenhum shader, preset, LUT ou textura de terceiros é embarcado nesta etapa.

- [Catálogo TOML](catalog.toml)
- [ADR 0014 — pipeline Slang/WGPU](../adr/0014-retro-shader-pipeline-librashader-wgpu.md)
- [Arquitetura e roadmap](../architecture/video-shader-pipeline.md)

## Revisão upstream

O catálogo foi verificado contra `libretro/slang-shaders` no commit `02bf134555c4595bd6b69be89feaff923da83754` (2026-11-20). Atualizações do catálogo devem fixar um novo commit e verificar novamente todos os caminhos.

A grafia real do preset de mistura de frames é `motionblur/mix_frames.slangp` (underscore), não `mix-frames.slangp`. `handheld/zfast_lcd.slang` é um shader source, não um preset `.slangp` completo.

## Licenças

Não presumir uma licença uniforme para o repositório inteiro. Antes de distribuir um preset, auditar o próprio arquivo e todos os shaders/includes/LUTs referenciados. Manter copyright e notices aplicáveis. Não baixar assets automaticamente em runtime.
