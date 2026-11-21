# Instruções para agentes de código — SUN — Strife Uranus Neptune

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

## Escopo orientado ao console — regra prioritária

O produto a emular é o **console Sega alvo**, não a especificação universal de cada chip isoladamente. O comportamento implementado deve ser determinado pelo hardware efetivamente presente no console e pelos usos necessários para executar corretamente seus jogos e periféricos suportados.

- Implementar primeiro e prioritariamente as funções que o console e seus jogos realmente utilizam.
- Não perseguir compatibilidade universal do chip, recursos genéricos, modos não utilizados pelo console, possibilidades teóricas ou comportamento de outros produtos que compartilham o mesmo componente.
- Antes de implementar uma função, exigir vínculo concreto com o console alvo: uso por jogos/periféricos suportados, documentação do hardware da plataforma ou evidência reproduzível de incompatibilidade. Sem esse vínculo, a função fica fora do escopo.
- Não iniciar pesquisa, caracterização, modelagem elétrica ou abstrações que não removam um bloqueio funcional ou melhorem de forma demonstrável a compatibilidade, o áudio, o vídeo, a entrada ou o desempenho do console.
- Preservar código e estudos excedentes já existentes quando não prejudicarem desempenho, estabilidade, manutenção ou clareza. Não ampliar, completar ou aprofundar esse trabalho apenas porque existe.
- Não remover trabalho existente por ser excedente sem demonstrar custo ou prejuízo concreto. A correção é interromper a expansão desnecessária e concentrar novos esforços na meta do console.
- Preferir a menor implementação correta que satisfaça o hardware e os jogos alvo. Generalização, reutilização entre produtos e abstrações só entram quando reduzem complexidade real sem ampliar o escopo.
- Medir conclusão pelo funcionamento verificável do console em jogos reais e testes pertinentes, nunca pela cobertura de todas as capacidades teóricas de seus chips.

### Aplicação imediata — YM2413

O YM2413 é implementado para reproduzir o áudio FM dos jogos compatíveis do Japanese Master System com YM2413 integrado. Não transformar o projeto em emulador universal de OPLL nem implementar funções de outros equipamentos ou aplicações que não sejam necessárias a esse perfil. Recursos do chip sem uso demonstrado nos jogos SMS ficam fora da prioridade e não podem bloquear a entrega de áudio FM in-game. A investigação elétrica/analógica e a paridade física absoluta são opcionais e posteriores; só serão retomadas se resolverem uma diferença audível concreta.

## Roadmap de plataformas

1. SMS1/SMS2.
2. SG-1000 — concluir a máquina temporal na arquitetura existente.
3. Game Gear como sistema próprio.
4. Expansões e acessórios 8-bit após Game Gear: YM2413 integrado do Master System japonês (não o FM Sound Unit externo do Mark III); Sega Card/My Card como mídia; 3D Glasses/SegaScope; periféricos de controle e interfaces de expansão conforme compatibilidade documentada. Card Catcher C-1000 está fora do escopo de implementação.
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

Não incorporar Python ao código-fonte de produção compilado do runtime ou frontend, nem criar dependência de Python para executar o produto. O runtime/hardware e o frontend de produção permanecem em Rust. Python é permitido exclusivamente em testes, instrumentação externa, scripts auxiliares e validações de CI, desde que essas ferramentas não sejam incorporadas ao binário nem exigidas para executar o emulador. Qt, PyO3 e Maturin não devem ser introduzidos no runtime ou frontend de produção sem decisão arquitetural explícita. Não versionar ROMs comerciais, BIOS proprietárias ou dumps protegidos.
