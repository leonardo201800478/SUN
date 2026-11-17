# Escopo do projeto — Sega Emulator

**Revisão:** 2026-11-16
**Baseline remota consultada:** `main` SHA `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`.
**Arquitetura desktop ativa:** Slint 1.18.x + WGPU 30.x/Vulkan + CPAL 0.18.2 + input SDL3 integrado em fatias.
**Runtime:** Rust; MachineScheduler como autoridade temporal.

## Objetivo

Construir um emulador Sega multissistema modular, determinístico e fiel ao hardware, preservando componentes validados e evoluindo os sistemas 8-bit antes de iniciar a família 16-bit.

## Arquitetura obrigatória

| Camada | Responsabilidade |
|---|---|
| Runtime | Rust: CPU, memória, barramentos, mappers, VDP, PSG/FM, periféricos, interrupções, Scheduler, timing e estados |
| GUI | Slint 1.18.x: janelas, menus, launcher, biblioteca, configurações, logs e telemetria |
| Vídeo | WGPU 30.x; Vulkan inicial no Windows x64 |
| Áudio físico | CPAL 0.18.2 |
| Input | Slint para teclado; `sega-input-runtime`/SDL3 para gamepads |
| Host legado | Não é caminho ativo. Código remanescente deve ser inventariado e removido somente sem quebrar contratos ainda consumidos |

A GUI não implementa hardware, instruções, clocks, temporização ou callbacks por instrução. A integração com o runtime é coarse-grained. Não duplicar CPU, VDP, PSG, Scheduler, worker, mailbox ou pipeline.

## Roadmap normativo

1. **SMS1/SMS2:** consolidar os perfis 315-5124/315-5246, boot, vídeo, áudio, I/O, mappers, BIOS e validação. Road Rash e alinhamento de sprites foram reportados pelo usuário como resolvidos; não reabrir sem regressão. BIOS boot permanece com validação física pendente.
2. **SG-1000:** concluir a máquina temporal e a integração dos componentes existentes, usando o Z80 compartilhado e preservando VDP/PSG/I/O. Não redesenhar a arquitetura nem criar CPU, GUI ou pipeline paralelo.
3. **Game Gear:** implementar como sistema próprio, com diferenças de VDP/LCD, CRAM, áudio estéreo, portas, memória, cartucho, snapshots e input. Não tratar como SMS com resolução reduzida.
4. **Expansões e acessórios 8-bit:** após Game Gear, implementar YM2413 integrado no perfil de Master System japonês (opcional e independente de região); Sega Card/My Card como mídia carregável; 3D Glasses/SegaScope com protocolo $FFF8–$FFFB, saída 2D por olho ou lado a lado e modo anaglífico opcional que exige óculos coloridos; Paddle, Sports Pad, Light Phaser, Rapid Fire e interfaces de expansão conforme compatibilidade real de cada console/modelo. Card Catcher C-1000 não será implementado.
5. **Mega Drive/Genesis.**
6. **Sega CD.**
7. **32X.**
8. **Consolidação, compatibilidade, auditoria e aceite final.**

A fase 4 é transversal apenas aos consoles Sega explicitamente incluídos neste roadmap. Sega Card/My Card permanece como formato de mídia para SMS/Mark III; Card Catcher C-1000 é excluído. **SC-3000 está integralmente fora do escopo do projeto como plataforma-alvo**: não haverá emulação do computador, de seus modos, software ou periféricos. Jogos SC/SG que sejam tecnicamente compatíveis com o SG-1000 podem ser carregados exclusivamente pelo perfil SG-1000; isso não constitui suporte ao SC-3000.

## Estado das trilhas transversais

- **Z80:** `CLOSED_WITH_LIMITATIONS` no perfil NMOS delimitado e nas possibilidades atuais de validação. E8/ZEXDOC/ZEXALL permanece complementar, não bloqueio artificial. Variantes CMOS/clones e validações não executadas continuam explicitamente não verificadas.
- **Debugger:** funcionalidade parcial; manter evolução independente, sem bloquear o fechamento de hardware.
- **BIOS boot:** implementação candidata em PR #928; validação física com BIOS real permanece pendente.
- **R2 vídeo:** frames SMS integrados ao desktop Slint/WGPU.
- **R3 áudio:** CPAL 0.18.2 integrado em software; validação física prolongada, hotplug e recuperação continuam gates separados.
- **R4 input:** primeira fatia SDL3/Slint integrada; controles físicos, hotplug real e configuração avançada permanecem pendentes.
- **R5 desktop:** launcher/biblioteca/configurações/HUD/telemetria e retirada de código legado ainda exigem consolidação.

## Invariantes temporais

MachineScheduler é a autoridade única de tempo. Preservar ordem causal, clocks, WAIT states comprovados, suspensão/retomada, snapshots e equivalência contínua/em blocos. Renderização, áudio físico e input nunca controlam ciclos.

## Evidência e aceite

Estados permitidos: `VALIDATED`, `INTERNAL_REVIEW`, `NOT_VERIFIED`, `CLOSED_WITH_LIMITATIONS`. Diferenciar evidência documental, comparativa, interna e física. `SKIPPED` e `NOT RUN` nunca são PASS. Não exigir console físico ou revisor independente como bloqueio permanente quando o escopo praticável está coberto; declarar explicitamente o que não foi validado.

Não iniciar 16-bit antes de SMS1/SMS2, SG-1000, Game Gear e a fase de expansões/acessórios 8-bit.
