# Desktop settings screens — Figma transfer

**Status:** approved Video screen visual reference transferred as repository assets. These SVGs are design references, not runtime UI or implementation evidence.

- [Video — Light](video-settings-light.svg)
- [Video — Dark](video-settings-dark.svg)

## Tokens

| Token | Light | Dark |
|---|---|---|
| Canvas | #F4F6FA | #111722 |
| Surface | #FFFFFF | #192231 |
| Subtle | #EEF1F6 | #202C3D |
| Primary text | #182230 | #F1F5FA |
| Secondary text | #586579 | #B4C0D0 |
| Accent | #2457D6 | #8EABFF |

Spacing: 4, 8, 12, 16, 24, 32, 40, 48 px. Radius: 6, 10, 16, 22 px. Typeface direction: IBM Plex Sans with system sans-serif fallback.

## Video settings scope

- Window mode: Exclusive / Borderless.
- Hide menus: Yes / No.
- Scaling: Internal / Fill screen.
- Overscan: Disabled / Top + Bottom / Total; regional behavior needs a defined video-standard contract.
- Hide left bar: expose only on systems where this capability exists.
- Synchronization: Disabled / Mailbox / VSync / VRR, gated by host presentation capabilities.
- Telemetry: Show FPS / additional information.
- Sprite limit: global preference; must not change VDP timing or hardware semantics.
- 3D glasses: Both eyes / Left eye / Right eye, only for compatible systems.
- Shaders: Bilinear, Pixel Perfect, Bilinear + NTSC, CRT, CRT + NTSC, Scanlines, MMPX, NTSC Composite, Sharp, Off.
- Shader pack: RetroArch/Vulkan — future integration, not implemented by this transfer.
- HDR: On / Off. Nits and paper white remain pending technical validation of the presentation pipeline.

## Architecture boundary

The settings UI is host configuration only. It must not execute VDP work, modify emulated clocks, or move synchronization authority away from MachineScheduler. Host presentation choices must be capability-gated. UI build success does not prove a feature is implemented or validated.

## Next screen

The Sound screen has a separately approved specification and should use the same tokens. Figma cleanup is intentionally not part of this PR: remove the Figma Video frames only after this repository transfer is reviewed and confirmed.


## Revalidação de proveniência — 2026-11-21

**Main de integração:** `7bcbdb244a307f73c39ea1540afde7a8427f1e44`.

Este pacote é uma referência visual Light/Dark para a tela de Vídeo. Não constitui implementação Slint, inventário de capacidades nem aceite de recursos. Cada controle deve permanecer oculto/desabilitado até existir serviço e capability do host comprovados. Em especial, modos exclusivos, VSync/VRR, overscan, shaders, HDR e 3D glasses não são habilitados por constarem no desenho. A GUI não pode alterar clocks, VDP ou MachineScheduler.

Os SVGs são mantidos como assets estáticos de design. A limpeza das frames Figma é uma ação separada e não é executada por esta transferência.
