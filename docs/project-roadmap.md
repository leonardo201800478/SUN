# Roadmap mestre — Sega Emulator

**Revisão:** 2026-11-16
**Baseline consultada:** `main` `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`.
**Autoridade temporal:** MachineScheduler.
**Arquitetura desktop ativa:** Slint 1.18.x + WGPU 30.x/Vulkan + CPAL 0.18.2 + SDL3/input runtime.

O roadmap de plataformas é a sequência normativa. As trilhas de desktop, debugger, áudio host e validação são transversais e não autorizam reescrever hardware.

## 1. Diretrizes

- Runtime e hardware integralmente em Rust.
- MachineScheduler é a autoridade única de clocks, eventos e ordem causal.
- Preservar Z80 compartilhado, SmsTemporalWorker, FrameMailbox, VDPs, PSG, barramentos e componentes validados.
- GUI Slint; WGPU para renderização; Vulkan backend inicial; CPAL para áudio físico; SDL3 para gamepads.
- Host gráfico anterior fora de uso ativo; código remanescente é legado, não arquitetura-alvo.
- Sem CPU/Scheduler/PSG/VDP/worker paralelo, JIT, frame skipping, hacks temporais ou callbacks por instrução.
- Save States/replays devem preservar estado suficiente e determinismo; renderização/áudio/input não comandam ciclos.

## 2. Roadmap de plataformas

| Fase | Sistema/escopo | Critério de saída |
|---|---|---|
| P0 | Fundação Rust, toolchain, CI, contratos e hosts | Workspaces e boundaries reproduzíveis; host Slint/WGPU ativo |
| P1 | SMS1/SMS2 | Perfis 315-5124/315-5246, runtime, BIOS/cartucho, VDP, PSG, I/O, mappers e regressões consolidados |
| P2 | SG-1000 | Máquina temporal integrada ao Scheduler existente; Z80 compartilhado; VDP TMS9918A, PSG, memória, I/O e snapshots integrados |
| P3 | Game Gear | Máquina própria com LCD 160×144, VDP/CRAM, áudio estéreo, memória/cartucho, portas e snapshots específicos |
| P4 | Expansões e acessórios 8-bit | YM2413 integrado do Master System japonês (opcional, independente de região); Sega Card/My Card como mídia; 3D Glasses/SegaScope com saídas 2D e estéreo; Paddle, Sports Pad, Light Phaser, Rapid Fire e expansões conforme compatibilidade |
| P5 | Mega Drive / Genesis | M68k, VDP, áudio, barramentos, cartuchos e Scheduler próprios, reutilizando somente contratos realmente comuns |
| P6 | Sega CD | Subsystem CD, CPUs/co-processadores e mídia conforme hardware documentado |
| P7 | 32X | SH-2 e expansão 32X após Mega Drive/Sega CD |
| P8 | Consolidação do produto | Biblioteca, launcher, configurações, HUD, telemetria, input e remoção segura de código legado |
| P9 | Compatibilidade e aceite final | Campanhas por sistema, regressões, documentação, release e limites declarados |

**Ordem obrigatória:** SMS1/SMS2 → SG-1000 → Game Gear → expansões/acessórios 8-bit → Mega Drive/Genesis → Sega CD → 32X. Não antecipar M68k ou SH-2.

### P4 — escopo inicial de acessórios

1. **YM2413:** emular o perfil de Master System japonês com YM2413 integrado e mistura simultânea com SN76489. Chip opcional por configuração Auto/On/Off, independente da região; sem emular Mark III FM Sound Unit como módulo externo.
2. **Sega Card / My Card:** mídia de ROM, slot, seleção e arbitragem com cartucho via port $3E, respeitando diferenças Mark III/SMS.
3. **Sega Card/My Card:** manter carregamento da mídia e o slot nativo SMS/Mark III quando necessário. Excluir Card Catcher C-1000; o adaptador físico não é necessário para carregar software no core SG-1000.
4. **SegaScope/3D Glasses:** emular protocolo $FFF8–$FFFB e obturadores sincronizados ao VDP. A interface Video adotará o modelo simples do Gearsystem: `Both Eyes / OFF`, `Only Left Eye` e `Only Right Eye`. A seleção é de apresentação e não altera a temporização do hardware; modos adicionais não fazem parte do escopo inicial.
5. **Periféricos DE-9:** Paddle, Sports Pad e Light Phaser, além de Rapid Fire quando suportado pelo perfil de hardware.
6. **Expansões e interfaces:** auditar pinouts, sinais, arbitragem e compatibilidade por console/revisão antes de implementar.

Cada item terá especificação própria, proveniência DOCUMENTADO/MEDIDO/INFERIDO, modelo temporal, snapshots, testes sintéticos e regressões. Não presumir que um acessório funciona em todos os consoles ou regiões.

## 3. Trilhas transversais — desktop R1–R5

| Trilha | Estado consolidado | Limites |
|---|---|---|
| R1 Slint/WGPU/Vulkan | POC e integração de software presentes | Revalidar execução física Vulkan, resize/lifecycle e apresentação sustentada no HEAD atual |
| R2 Frames SMS | Integrada via SmsTemporalWorker/FrameMailbox | Não atribuir evidência de snapshot antigo ao HEAD atual |
| R3 CPAL 0.18.2 | Integrada em software | Reprodução física prolongada, hotplug/reconexão, underruns e formatos reais ainda exigem evidência |
| R4 SDL3 input | Primeira integração SDL3/Slint presente | Controles físicos, hotplug, identidade persistente e remapeamento avançado pendentes |
| R5 Produto desktop | Parcial | Biblioteca, configurações consolidadas, HUD/telemetria e remoção segura dos componentes legados |

R5 é transversal; não deve bloquear implementação de hardware quando a integração coarse-grained necessária já existe.

## 4. Trilhas transversais adicionais

### Z80 compartilhado

Estado: `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e o escopo praticável de SMS/SG-1000. As famílias E4C/E5/E6/E7 e as extensões de F-021/F-022/F-023/F-028 possuem testes internos, diferenciais e integrações registradas. Permanecem limites: CMOS/clones fora do perfil, WAIT físico, DMA, validação elétrica e cobertura universal não demonstrados. ZEXDOC/ZEXALL e outras suítes são complementares futuras. Não reabrir E7 sem bug/regressão ou evidência concreta.

### Debugger

Implementado parcialmente e em evolução. Melhorar captura, retenção/exportação, snapshots CPU/VDP, janela de diagnóstico e usabilidade em trilha separada. Não é bloqueio para fechar SMS/SG-1000.

### BIOS

O boot por BIOS convencional de 8 KiB está em implementação candidata na PR #928. Validação física com firmware real, handoff e primeiro código do cartucho permanece pendente. Não declarar boot físico validado.

## 5. Estado operacional da baseline

- Main SHA: `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`.
- CI associado à baseline: run #43945796678 e SMS Runtime #43945796477. Quatro checks SUCCESS; 14 SKIPPED. PASS inclui Rust quality/unit, Z80 focused, change impact e SMS Runtime Release format/Clippy/tests. Os jobs SKIPPED não são validação.
- Árvore atual: 285 documentos Markdown sob `docs/`; inventário estrutural anterior não equivale à leitura e reconciliação semântica integral dos 285 arquivos.
- Road Rash: usuário reportou teste físico OK; retirar da lista de defeitos abertos, sem inventar SHA/relatório.
- Alinhamento de sprites SMS: usuário reportou corrigido; não manter como defeito aberto sem regressão.
- BIOS boot: validação física pendente.
- Host gráfico anterior: não está em uso ativo.
- SG-1000: ainda precisa concluir integração temporal na arquitetura existente.

## 6. Método, evidência e gates

Antes de alterar código: auditar main/PRs/branches/workflows; consultar documentação técnica primária; identificar consumidor e efeito observável; modelar ownership, clocks, eventos, estados e snapshots; implementar mudanças pequenas.

Gates Rust conforme workspace/target afetado:
- `cargo fmt --all -- --check`
- `cargo clippy --release --workspace --all-targets --locked -- -D warnings`
- `cargo test --release --workspace --locked`

Executar também os workspaces standalone e os testes focados pertinentes. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED nunca é PASS. Vincular cada resultado ao SHA exato. Não transferir resultados de PR para a main sem validação pós-merge.

## 7. GitHub e integração

Toda alteração em branch/PR. Não fazer commit direto na main, force-push, reset destrutivo ou merge cego. Portar seletivamente branches divergentes. Merge somente após diff revisado, CI aplicável verde no HEAD final e autorização explícita.
