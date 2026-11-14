# Roadmap mestre — Sega Emulator

**Revisão documental:** 2026-11-14. Estado operacional consultado na main em 19e3155a0efcaadc9d5fff9b3e8bf1b12b30784c; as alterações R4 descritas como em andamento pertencem à branch feat/r4-slint-sdl3-gamepad-input-20261114.
**Decisões:** ADR 0010 (runtime/hardware), ADR 0011 (desktop/hosts) e ADR-0016 (fidelidade temporal).

Duas trilhas coordenadas: evolução dos sistemas Sega (P0–P9) e replataforma desktop/host (R1–R5). A segunda não autoriza reescrever hardware nem antecipar sistemas.

## 1. Diretrizes

- Runtime 100% Rust; MachineScheduler é autoridade de clocks, eventos e ordem causal.
- CPU, VDP, PSG, memória, barramentos, mappers, interrupções e estados pertencem ao runtime.
- Slint é GUI; WGPU é API gráfica; Vulkan é backend inicial preferencial; CPAL é saída física.
- sega-input-runtime já contém contratos host-independent e backend SDL3 Windows x64; R4 integra esse backend ao host Slint. Teclado continua nos eventos nativos do Slint.
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

| Trilha | Situação | Avaliação |
|---|---|---|
| R1 — Slint + WGPU 30 + Vulkan | POC integrada; workflow Windows x64 passou | Integração de software aprovada. O relatório local 118b96bf registra inicialização física, Vulkan efetivo, resize/lifecycle e apresentação sustentada como NOT RUN; repetir no HEAD atual. |
| R2 — Frames reais do SMS | FrameMailbox e SmsTemporalWorker integrados; frame pacing resolvido (#755); smoke test físico relatado | Caminho de frames integrado. O relato manual ainda não está vinculado a log/HEAD reproduzível; completar campanha de ROM/gameplay no HEAD atual. |
| R3 — Áudio CPAL 0.18.2 | CPAL e pipeline PCM integrados; CI Windows e testes automatizados aprovados; reprodução física relatada | Integração aprovada. Reprodução prolongada, hotplug, recuperação de endpoint e underruns ainda não estão documentados no HEAD atual. |
| R4 — SDL3 input | Backend existente em native/input_runtime; integração com Slint em PR #867 | Em andamento. CI Windows SDL3 e testes físicos com controles ainda pendentes; perfis/remapeamento também. |
| R5 — Frontend completo | Slint desktop parcial, menus e controles em evolução | Pendente: biblioteca, configurações consolidadas, HUD, telemetria e retirada dos hosts antigos. |


### R1 — POC gráfica mínima

Comprovar no Windows x64: janela Slint; WGPU 30; Vulkan explicitamente selecionado/identificado; mesmo wgpu::Device e wgpu::Queue compartilhados com Slint; textura externa WGPU atualizada continuamente; composição na mesma janela; resize, lifecycle da surface e shutdown limpo; build Release reproduzível.

R1 é isolada. Não integrar SMS real, SmsTemporalWorker, menu de jogo, pausa do Scheduler, HUD, CPAL, SDL3, HDR ou VRR. Não alterar hardware/runtime.

**Estado:** POC Slint/WGPU 30/Vulkan integrada e validada pelo workflow Windows x64. O relatório local vinculado ao snapshot 118b96bf411248173b6215b7bf16aebdf3a66bff classifica inicialização física da janela, seleção física do Vulkan, apresentação sustentada e lifecycle da surface como NOT RUN. Não promover o PASS do CI a aceite físico. Repetir os critérios no HEAD atual.

### R2 — vídeo real

Frames SMS integrados pelo contrato portátil, preservando `SmsTemporalWorker`, `FrameMailbox` e `MachineScheduler`. A issue #755 de frame pacing foi encerrada como resolvida. Há relatos manuais de execução, mas o relatório Windows do snapshot 118b96bf... ainda classifica ROM/gameplay e apresentação física sustentada como NOT RUN; repetir e registrar a sessão no HEAD atual. Compatibilidade ampla de ROMs permanece fora do gate de integração.

### R3 — áudio CPAL 0.18.2

CPAL 0.18.2 e o pipeline PCM estão integrados em software; testes automatizados e CI Windows foram aprovados em SHAs anteriores. O relatório Windows do snapshot 118b96bf... registra abertura/reprodução física, hotplug, estabilidade prolongada e underruns como NOT RUN. Relatos manuais de áudio audível não estão vinculados a um SHA reproduzível. Repetir no HEAD atual. A thread de áudio não controla o Scheduler.

### R4 — input SDL3

O crate native/input_runtime e o backend SDL3 dinâmico Windows x64 já existem na linha principal, mas não eram consumidos pelo desktop Slint: o host usava somente eventos de teclado nativos. A PR #313 histórica pertence ao host Qt/PyO3 e não comprova integração no desktop atual.

A branch feat/r4-slint-sdl3-gamepad-input-20261114 inicia a integração: dependência do sega-input-runtime no desktop, polling SDL3 no limite do host Slint com intervalo nominal de 8 ms, atualizações hotplug observadas pelo backend a cada 250 ms, mapeamento inicial Gamepad API (D-pad/analógico esquerdo + South/East + Start como NMI por borda de subida) e fallback Joystick API. Os dois primeiros dispositivos elegíveis são atribuídos deterministicamente a P1/P2 na sessão. Atualizações usam SmsTemporalWorker::set_controller_at; não há polling no Scheduler nem callbacks por instrução.

O CI Windows passa a instalar SDL3 3.4.20 com SHA-256 fixado e executar smoke test da ABI dinâmica sem exigir controle físico. A integração de software desta primeira fatia ainda precisa passar CI.

**Pendências R4:** aceite físico com controles reais, hotplug/reconexão, identidade persistente por dispositivo (o descritor atual ainda retorna physical_key=None), perfis individuais, remapeamento, atribuição configurável P1/P2, threshold separado para captura analógica e investigação dos caminhos Pause/NMI. XInput direto não é declarado implementado; dispositivos XInput podem ser reconhecidos por SDL3 conforme driver/mapeamento. Não medir nem alegar input latency sem instrumentação específica.

### R5 — frontend

Consolidar native/sega_desktop, native/video_output_wgpu e native/audio_output; implementar launcher, biblioteca, configurações, menus, HUD, logs e telemetria; provar paridade; remover egui/eframe e hosts antigos somente após regressão e aceite Release no HEAD exato.

## 4. Estado operacional

**Evidência Windows x64:** [relatório local de validação Release](testing/windows-x64-release-validation-2026-11-14.md) registra uma execução do usuário no snapshot `118b96bf411248173b6215b7bf16aebdf3a66bff`: desktop 26/26 testes, crate CPAL 21/21 testes, fmt/Clippy e build Release PASS. Essa evidência é histórica e limitada ao SHA declarado; não certifica automaticamente commits posteriores nem aceite físico.

**Auditoria VDP SMS1:** [auditoria do caminho ativo](audits/sms1-vdp-runtime-path-audit-2026-11-14.md) inspeciona estaticamente o mesmo snapshot `118b96bf...`. Confirma a cadeia de construção do `SmsVdp` SMS1 315-5124 no host desktop e documenta limitações de fetch/temporização de sprites; não identifica a causa de bugs específicos sem reprodução por jogo.

- Frame pacing: issue #755 encerrada como resolvida em 2026-11-14; não reabrir sem evidência reproduzível de regressão.
- R1 está integrada e o workflow Windows da POC passa; os critérios físicos listados no relatório histórico ainda não foram executados no SHA atual.
- R2 (frames SMS) e R3 (CPAL) estão integradas em software; frame pacing foi resolvido, mas a campanha física reproduzível e o aceite sustentado ainda precisam ser vinculados ao HEAD atual.
- R4 possui backend SDL3 existente; a integração com Slint está em andamento na branch dedicada. Não confundir a antiga PR #313 Qt/PyO3 com integração no desktop Slint.
- R5 consolidação final permanece pendente.
- Para evidência local ou auditoria estática, sempre preservar o SHA exato e não promover um snapshot histórico a validação da `main` atual.

Branches antigas devem ser reconciliadas por portabilidade seletiva para branches novas sobre a `main` atual. Não fazer merge cego de histórico divergente.

## 5. Z80

E4C → E5 → E6 → E7 → E8 → E9. Consultar docs/z80-completion-scorecard.md e docs/z80-dd-fd-semantic-matrix.md antes de qualquer tarefa. Não iniciar E8/ZEXDOC/ZEXALL antes do fechamento demonstrável de E7. Não reabrir etapas validadas sem evidência de defeito/regressão.

## 6. Método e gates

Auditar SHA/PRs/workflows; pesquisar fontes primárias; definir critérios; modelar estados/ownership/timing; implementar incrementos pequenos; conferir formatação, Clippy e testes antes do workflow; regredir; auditar diff; atualizar documentação/evidências; publicar por PR.

Gates:
- cargo fmt --all -- --check
- cargo clippy --release --workspace --all-targets --locked -- -D warnings
- cargo test --release --workspace --locked

Validar workspaces/targets standalone. Aplicativos e testes de desempenho devem usar Release. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Vincular evidências ao SHA exato.

## 7. GitHub e integração contínua

GitHub é a fonte da verdade. Integrar continuamente na main o código tecnicamente validado, em incrementos pequenos. Manter branches curtas; para branches antigas/divergentes, preservar o histórico e portar seletivamente as mudanças úteis para uma branch nova baseada na main atual. Não fazer merge cego, force-push ou reset destrutivo. Toda alteração passa por branch/PR, diff revisado, testes direcionados, regressões pertinentes e CI aplicável verde no HEAD final. A autorização permanente do usuário permite integrar após os gates; não permite contorná-los. SKIPPED, queued e NOT RUN não são PASS.
