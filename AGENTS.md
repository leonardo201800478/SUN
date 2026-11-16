# Instruções para agentes de código — Sega Emulator

A fonte da verdade é a `main` remota, seus commits, testes e workflows. Antes de cada tarefa, consulte o HEAD, PRs, branches, diffs e CI atuais. Não reutilize SHAs ou conclusões antigas sem revalidação.

## Arquitetura normativa

- **Runtime/hardware:** 100% Rust. CPU Z80 compartilhada, memória, barramentos, mappers, VDPs, PSG/FM, periféricos, interrupções, Scheduler, timing, estados e execução pertencem ao backend.
- **GUI/host ativo:** Slint 1.18.x.
- **Renderização:** WGPU 30.x; Vulkan como backend inicial no Windows x64.
- **Áudio físico:** CPAL 0.18.2.
- **Input:** eventos nativos Slint para teclado e `sega-input-runtime`/SDL3 para gamepads, respeitando o estado integrado atual.
- O host gráfico anterior não é caminho ativo do produto. Código remanescente só pode ser tratado como legado/transitório até inventário de consumidores e remoção segura; não desenvolver novas funcionalidades nele.

## Separação de responsabilidades

A GUI cuida de janelas, menus, launcher, biblioteca, configurações, logs, telemetria e integração com o SO. É proibido implementar hardware, instruções, clocks, temporização ou callbacks por instrução na GUI.

A integração com runtime permanece coarse-grained: `load_rom()`, `start()`, `pause()`, `stop()`, `reset()`, `step_frame()`, snapshots e frames com ownership explícito. Preservar `SmsTemporalWorker`, `FrameMailbox`, `MachineScheduler` e componentes validados.

## Tempo e determinismo

- `MachineScheduler` é a autoridade única de clocks, eventos e ordem causal.
- Preservar sincronização CPU/VDP/áudio/barramentos, WAIT states reais quando demonstrados, operações suspensíveis/retomáveis e estado necessário à retomada.
- Execução contínua e em blocos deve ser equivalente nas dimensões testadas.
- Save States/replays devem preservar determinismo.
- Renderização, áudio físico e input não podem avançar nem corrigir clocks do hardware.
- Não criar Scheduler, CPU, PSG, VDP, worker ou pipeline paralelo por conveniência.

## Roadmap de plataformas

1. SMS1/SMS2.
2. SG-1000, concluído na arquitetura temporal existente.
3. Game Gear como sistema próprio.
4. Expansões e acessórios 8-bit: YM2413/FM Sound Unit, Sega Card/My Card, Card Catcher (adaptador SG-1000), 3D Glasses, periféricos de controle e interfaces de expansão.
5. Mega Drive/Genesis.
6. Sega CD.
7. 32X.
8. Compatibilidade, consolidação e aceite final.

Não iniciar 16-bit antes do fechamento dos sistemas 8-bit e da fase de acessórios. Não reescrever arquitetura nem criar implementações paralelas para SG-1000.

## Z80

O Z80 compartilhado está encerrado no escopo praticável do perfil NMOS clássico utilizado pelos sistemas 8-bit suportados, com limitações explicitadas. E8/ZEXDOC/ZEXALL é validação complementar futura, não bloqueio artificial permanente. Não reabrir E7 sem bug, regressão ou evidência técnica concreta. CMOS/clones e precisão elétrica não cobertos continuam NOT_VERIFIED.

## Método e gates

1. Auditar main/PR/CI no SHA exato.
2. Consultar documentação primária, referências independentes e versões atuais das bibliotecas quando relevantes.
3. Definir contratos, ownership, estados, timing e critérios de aceite antes de implementar.
4. Implementar incrementos pequenos em branch/PR.
5. Executar `cargo fmt --all -- --check`, Clippy e testes direcionados; depois workflows aplicáveis.
6. Classificar PASS, FAIL, BLOCKED ou NOT RUN. SKIPPED nunca é PASS.
7. Auditar diff e preservar evidências históricas; nunca atribuir teste a SHA diferente.
8. Não fazer merge sem CI aplicável verde no HEAD final e autorização explícita.

Não usar Python, Qt, PyO3 ou Maturin no runtime, frontend, CI ou tooling obrigatório. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos.
