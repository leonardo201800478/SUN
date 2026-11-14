# Roadmap mestre — Sega Emulator

**Revisão documental:** 2026-11-14. A fotografia operacional deve ser vinculada ao HEAD consultado no GitHub.
**Decisões:** ADR 0010 (runtime/hardware), ADR 0011 (desktop/hosts) e ADR-0016 (fidelidade temporal).

Duas trilhas coordenadas: evolução dos sistemas Sega (P0–P9) e replataforma desktop/host (R1–R5). A segunda não autoriza reescrever hardware nem antecipar sistemas.

## 1. Diretrizes

- Runtime 100% Rust; MachineScheduler é autoridade de clocks, eventos e ordem causal.
- CPU, VDP, PSG, memória, barramentos, mappers, interrupções e estados pertencem ao runtime.
- Slint é GUI; WGPU é API gráfica; Vulkan é backend inicial preferencial; CPAL é saída física.
- SDL3 será avaliado para input/gamepads após auditoria.
- Não ampliar nem reintroduzir o host desktop anterior. A retirada física dos módulos transitórios ocorre somente após R3/R5 e inventário de consumidores.
- GUI não executa hardware, avança clocks ou usa callbacks por instrução.
- Preservar SmsTemporalWorker, Scheduler, SMS e SG-1000. Não duplicar componentes.
- Sem JIT, frame skipping, hacks temporais ou otimizações especulativas.

## 2. Plataformas — P0 a P9

1. **P0:** infraestrutura/hosts, toolchain, workspaces, CI e contratos.
2. **P1:** Master System temporal, Z80 compartilhado, ROM, lifecycle, vídeo, áudio, input e validação.
3. **P2:** SG-1000, preservando componentes compartilhados e regressões.
4. **P3:** Game Gear.
5. **P4:** Mega Drive/Genesis.
6. **P5:** Sega CD.
7. **P6:** 32X / Sega CD 32X.
8. **P7:** frontend desktop completo em Slint: launcher, biblioteca, configurações, HUD e telemetria.
9. **P8:** compatibilidade e auditoria.
10. **P9:** comprovação temporal sistêmica dos caminhos observáveis.

Não avançar para plataforma posterior com bloqueadores estruturais anteriores. O shell mínimo e as integrações necessárias ao SMS não equivalem ao frontend completo P7.

## 3. Replataforma — R1 a R5

### R1 — POC gráfica mínima

Comprovar no Windows x64: janela Slint; WGPU 30; Vulkan explicitamente selecionado/identificado; mesmo wgpu::Device e wgpu::Queue compartilhados com Slint; textura externa WGPU atualizada continuamente; composição na mesma janela; resize, lifecycle da surface e shutdown limpo; build Release reproduzível.

R1 é isolada. Não integrar SMS real, SmsTemporalWorker, menu de jogo, pausa do Scheduler, HUD, CPAL, SDL3, HDR ou VRR. Não alterar hardware/runtime.

**Gate:** execução real Windows x64 com Vulkan confirmado, textura atualizada e composição na mesma janela. Análise documental não é PASS de integração.

### R2 — vídeo real

Frames SMS integrados por contrato portátil, preservando SmsTemporalWorker, FrameMailbox e Scheduler. O host desktop Release passou em 26 testes automatizados e compilou para Windows x64 no SHA `118b96bf411248173b6215b7bf16aebdf3a66bff`. O aceite físico continua pendente: janela, backend Vulkan efetivo, ROM real, apresentação sustentada, resize, lifecycle e gameplay. Frame pacing P0 continua sendo um gate independente.

### R3 — áudio CPAL 0.18.2

CPAL 0.18.2, pipeline PCM moderno e integração de software estão na main. Em 2026-11-14, o crate independente `native/audio_output` passou por fmt, Clippy Release sem warnings e 21 testes no Windows x64/MSVC. Isso não comprova stream físico aberto, reprodução audível, estabilidade prolongada, hotplug, recuperação de endpoint ou ausência de underruns. Esses itens continuam no aceite físico R3.

### R4 — input SDL3

Auditar APIs; integrar teclado, gamepads, hotplug e mapeamentos; encaminhar eventos sem lógica de hardware no host. O teclado SMS básico já é tratado pelo host Slint. Gamepads SDL3 permanecem pendentes.

### R5 — frontend

Consolidar native/sega_desktop, native/video_output_wgpu e native/audio_output; implementar launcher, biblioteca, configurações, menus, HUD, logs e telemetria; provar paridade; remover egui/eframe e hosts antigos somente após regressão e aceite Release no HEAD exato.

## 4. Estado operacional

A fotografia operacional de 2026-11-14, incluindo SHA, comandos locais, resultados, limites e protocolo de aceite físico, está em [Validação Windows x64 Release — 2026-11-14](project-status-2026-11-14.md).

- Main validada localmente: `118b96bf411248173b6215b7bf16aebdf3a66bff`.
- Desktop Slint: fmt PASS; Clippy Release `-D warnings` PASS; testes 26/26 PASS; build Release Windows x64 PASS.
- CPAL `native/audio_output`: fmt PASS; Clippy Release `-D warnings` PASS; testes 21/21 PASS.
- Executável `sega-desktop.exe` encontrado, 22.192.640 bytes.
- Aceite físico R2/R3: NOT RUN nesta validação. Não declarar Vulkan ativo, gameplay ou áudio audível com base somente na compilação/testes.
- R4 SDL3 e R5 consolidação permanecem pendentes.
- P0 frame pacing permanece independente e não pode ser considerado resolvido por esta build.

## 5. Z80

E4C → E5 → E6 → E7 → E8 → E9. Consultar docs/z80-completion-scorecard.md e docs/z80-dd-fd-semantic-matrix.md antes de qualquer tarefa. Não iniciar E8/ZEXDOC/ZEXALL antes do fechamento demonstrável de E7. Não reabrir etapas validadas sem evidência de defeito/regressão.

## 6. Método e gates

Auditar SHA/PRs/workflows; pesquisar fontes primárias; definir critérios; modelar estados/ownership/timing; implementar incrementos pequenos; conferir formatação, Clippy e testes antes do workflow; regredir; auditar diff; atualizar documentação/evidências; publicar por PR.

Gates gerais:

- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Validar workspaces/targets standalone. Aplicativos e testes de desempenho devem usar Release. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Vincular evidências ao SHA exato.

## 7. GitHub e integração contínua

GitHub é a fonte da verdade. Integrar continuamente na main o código tecnicamente validado, em incrementos pequenos. Manter branches curtas; para branches antigas/divergentes, preservar o histórico e portar seletivamente as mudanças úteis para uma branch nova baseada na main atual. Não fazer merge cego, force-push ou reset destrutivo. Toda alteração passa por branch/PR, diff revisado, testes direcionados, regressões pertinentes e CI aplicável verde no HEAD final. A autorização permanente do usuário permite integrar após os gates; não permite contorná-los. SKIPPED, queued e NOT RUN não são PASS.
