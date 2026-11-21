# Arquitetura — SUN — Strife Uranus Neptune

**Baseline:** `main` `be883792025de3db7eaf4e7b8243293ea90b7e06` (2026-11-17).
**Decisões normativas:** ADR 0010 (runtime Rust), ADR 0011 (desktop Slint/WGPU/CPAL) e ADR-0016 (fidelidade observável).

## Produto e hosts ativos

O runtime de hardware é 100% Rust. O host de produto ativo é Slint 1.18.x, com renderização WGPU 30.x (Vulkan inicial no Windows x64), saída física CPAL 0.18.2 e SDL3/input runtime para gamepads. O host anterior egui/D3D12/Qt não é caminho ativo. Referências e módulos remanescentes devem ser classificados como histórico/legado e removidos somente após inventário de consumidores e regressão.

## Camadas

| Camada | Responsabilidade |
|---|---|
| Hardware Rust | CPU Z80 compartilhada, memória, barramentos, mappers, VDP, PSG/FM, periféricos e interrupções |
| Tempo Rust | MachineScheduler, clocks, eventos, causalidade, WAIT, suspensão/retomada e snapshots |
| Runtime | Máquinas SMS, SG-1000 e futuras plataformas; worker e comandos coarse-grained |
| GUI | Slint: janelas, menus, biblioteca, launcher, configurações, logs e telemetria |
| Vídeo host | WGPU/Vulkan, texturas, composição e apresentação |
| Áudio host | CPAL e pipeline PCM já existente |
| Input host | Eventos de teclado Slint e `sega-input-runtime`/SDL3 |

## Estado integrado

- R1: POC Slint/WGPU/Vulkan integrada em software; revalidar execução física Vulkan, resize/lifecycle e apresentação sustentada no HEAD atual.
- R2: frames SMS integrados por `SmsTemporalWorker`/`FrameMailbox`, preservando Scheduler.
- SG-1000 P2.2: máquina temporal integrada ao Z80 compartilhado e MachineScheduler, com TMS9918A, PSG, I/O e snapshots compostos (PR #988).
- SG-1000 P2.3: worker Rust dedicado, publicação FrameMailbox, input snapshot e AudioSession compartilhada em validação; host Slint/WGPU/CPAL ainda não seleciona SG-1000.
- R3: CPAL 0.18.2 integrado em software; validação física prolongada, hotplug e underruns permanecem gates separados.
- R4: primeira fatia SDL3/Slint integrada; identidade persistente, hotplug físico e remapeamento avançado permanecem pendentes.
- R5: launcher, biblioteca, configurações, HUD, telemetria e limpeza segura do legado ainda parciais.

## Regras arquiteturais

- Uma única implementação compartilhada do Z80.
- Uma autoridade temporal: MachineScheduler.
- Nenhuma instrução, ciclo ou timestamp de hardware atravessa a GUI.
- GUI envia comandos de sessão; runtime possui estado mutável da máquina.
- Vídeo, áudio físico e input não avançam nem corrigem clocks emulados.
- Não duplicar worker, mailbox, PSG, VDP, Scheduler, mixer ou pipeline.
- Sem JIT, frame skipping ou hacks temporais.
- Aplicativos e benchmarks usam Release.

## Roadmap de plataformas

SMS1/SMS2 → SG-1000 → Game Gear → expansões/acessórios 8-bit (YM2413/FM, Sega Card/My Card, Card Catcher, 3D Glasses e periféricos) → Mega Drive/Genesis → Sega CD → 32X.

O SG-1000 deve ser concluído na arquitetura existente. Game Gear é sistema próprio, não apenas SMS com resolução reduzida. 16-bit só começa depois da fase 8-bit e dos acessórios.

## Z80 e evidência

E7: `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e escopo praticável SMS/SG-1000. CMOS/clones, WAIT físico, DMA e sincronismo físico global permanecem não verificados. E8/ZEXDOC/ZEXALL é complementar, não gate permanente. Não reabrir sem defeito/regressão reproduzível.

Classificar evidência como documental, comparativa, interna ou física; vincular testes ao SHA exato. SKIPPED e NOT RUN nunca são PASS.
