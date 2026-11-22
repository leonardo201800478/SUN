# ADR 0011 — Replataforma desktop com Slint, WGPU e CPAL

- **Status:** ACCEPTED; arquitetura-alvo normativa. R1/R2/R3 integradas em software; primeira fatia R4 SDL3/Slint integrada; R5 permanece em consolidação.
- **Revisão:** 2026-11-22.
- **Baseline desta revisão:** `main` SHA `e4ebba40f67ba090720ee8b282d9f81c21a9ffd2`.
- **Escopo:** substituir gradualmente GUI e adaptadores host; preservar runtime Rust.
- **Precedência:** complementa ADR 0010 para frontend/hosts. Não altera CPU, VDP, PSG, barramentos, Scheduler, ownership ou determinismo.

## 1. Decisão

A replataforma ocorre no repositório existente. Não criar outro repositório nem reescrever hardware.

| Responsabilidade | Tecnologia |
|---|---|
| GUI | Slint 1.18.x |
| API gráfica | WGPU 30.x |
| Backend inicial | Vulkan via WGPU, Windows x64 |
| Saída física de áudio | CPAL 0.18.2 |
| Teclado SMS | Eventos nativos Slint no host desktop |
| Gamepads/hotplug | SDL3 via sega-input-runtime (primeira fatia integrada) |
| Runtime/hardware | Rust existente |

WGPU é a única API gráfica do produto; Vulkan é backend WGPU, não API paralela. R1 deve selecionar e registrar Vulkan explicitamente, sem fallback silencioso. CPAL é a API de saída de áudio; WASAPI pode ser backend interno no Windows.

O caminho Slint/WGPU é o frontend desktop-alvo. CPAL é a única API de saída física definida para a nova arquitetura. O teclado de gameplay usa eventos nativos do Slint; SDL3 permanece reservado à integração de gamepads/hotplug e remapeamento após auditoria. Não criar novos consumidores dos hosts anteriores. A remoção física dos módulos ainda usados pelo aplicativo antigo fica condicionada a R3/R5 e aos gates de regressão. O inventário transitório é mantido em docs/architecture/migrations/legacy-host-retirement.md.

## 2. Fronteiras

Slint cuida de janelas, menus, diálogos, biblioteca, configurações e eventos de teclado do host. WGPU cuida de device/queue/surface, texturas, composição e apresentação. CPAL cuida do stream físico. SDL3 é o backend de gamepads na primeira fatia integrada; hotplug físico, identidade persistente e remapeamento avançado continuam pendentes.

Essas camadas não executam instruções, avançam clocks, implementam VDP/PSG, possuem estado mutável da máquina ou usam callbacks por instrução. O runtime permanece proprietário da máquina SMS, CPU, Scheduler, VDP, PSG e estado temporal. A UI usa comandos coarse-grained como load_rom, start, pause, stop, reset e step_frame.

## 3. Versões e compatibilidade

Versões de referência:
- Slint 1.18.x; integração WGPU 30 exposta como API/feature instável. A POC habilita explicitamente `renderer-femtovg-wgpu` e `unstable-wgpu-30`, além de `backend-winit`, `std` e `compat-1-18`.
- WGPU 30.x.
- CPAL 0.18.2.

Fixar versões reproduzíveis no Cargo.lock. Fixar a série Slint 1.18 durante a POC e revalidar antes de atualizar. Verificar documentação oficial, changelog, MSRV, licença e compatibilidade.

Referências:
- https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/
- https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/enum.WGPUConfiguration
- https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/struct.WGPUSettings
- https://wgpu.rs/doc/wgpu/
- https://docs.rs/cpal/0.18.2/cpal/

## 4. R1 — POC gráfica mínima

R1 verifica as possibilidades reais das APIs. Não integra o runtime SMS nem constitui frontend de produto.

Critérios:
1. Aplicação mínima Slint 1.18.
2. Inicialização WGPU 30.
3. Slint usa o renderer FemtoVG baseado em WGPU 30 (`renderer-femtovg-wgpu`).
4. Vulkan explicitamente selecionado por `WGPUSettings.backends = Backends::VULKAN`; confirmar `Device::adapter_info().backend`.
5. Mesmo wgpu::Device e wgpu::Queue compartilhados, obtidos pelo rendering notifier Slint.
6. Textura externa RGBA8 com TEXTURE_BINDING, RENDER_ATTACHMENT e COPY_DST, atualizada continuamente pela queue compartilhada.
7. Composição da textura e UI Slint na mesma janela.
8. Resize, recriação de surface e shutdown limpo.
9. Build e execução Release no Windows x64.

A POC seleciona explicitamente o backend `winit` e o renderer `femtovg-wgpu` via `BackendSelector::backend_name()` e `renderer_name()`, além de usar `WGPUConfiguration::Automatic(WGPUSettings)` para restringir o backend WGPU subjacente a Vulkan. `Window::set_rendering_notifier()` fornece o device/queue efetivos do renderer. A textura é importada por `Image::try_from<wgpu::Texture>()` e recuperada para atualização por `Image::to_wgpu_30_texture()`. O caminho manual permanece uma alternativa a avaliar se o renderer automático não satisfizer o ciclo de vida/presentação. Não presumir compatibilidade apenas pela documentação.

**Fora de R1:** SmsTemporalWorker, ROM real, FrameMailbox, menu de jogo, pausa do Scheduler, HUD, CPAL, SDL3, HDR, VRR e paridade do frontend. Não alterar runtime, CPU, VDP, PSG ou Scheduler. Não exigir zero-copy físico; evitar cópias redundantes e demonstrar ownership correto.

**Gate:** execução real com Vulkan confirmado, textura atualizada e composição Slint/WGPU na mesma janela. Análise documental ou CI de jobs não relacionados não é PASS de integração.

## 5. R2–R5

- **R2 — vídeo real:** INTEGRADO pelas PRs #752 e #754. Frames SMS chegam ao desktop Slint/WGPU por contrato portátil/FrameMailbox; SmsTemporalWorker e Scheduler foram preservados. Validação física e frame pacing permanecem gates independentes.
- **R3 — CPAL:** integrado em software com CPAL 0.18.2 e pipeline PCM/SPSC. Permanecem pendentes validação física prolongada, hotplug/reabertura automática, latência e underruns em endpoints reais. O callback não controla o Scheduler.
- **R4 — SDL3:** primeira fatia integrada ao host Slint pelo `sega-input-runtime::Sdl3Backend`; teclado continua nos eventos nativos do Slint. Permanecem pendentes aceite físico com controles atuais, hotplug/reconexão e remapeamento avançado.
- **R5 — frontend:** consolidar launcher, biblioteca, configurações, menus, HUD, logs e telemetria. Demonstrar paridade antes de remover egui/eframe e hosts antigos.

Cada fase tem gates independentes. Não ampliar uma POC para antecipar fases posteriores. O corte completo dos hosts anteriores não é autorizado por mera integração R2; depende de CPAL R3 e consolidação R5.

## 6. Matriz de validação CI

A CI é orientada pelo impacto real dos arquivos alterados, não por uma matriz fixa de todas as APIs históricas.

- Mudanças em `native/sega_desktop/**` executam o gate Windows x64 Release da POC Slint/WGPU 30/Vulkan.
- Mudanças no runtime e nos componentes compartilhados continuam sujeitas aos gates Rust de workspace e aos testes focados correspondentes.
- Os checks de compatibilidade remanescentes não validam a arquitetura-alvo. Não os interpretar como gates funcionais de Slint/WGPU, CPAL ou SDL3; a retirada dos shims depende de atualização da proteção de branch.
- Os testes do `sega-input-runtime` e o gate SDL3 validam o comportamento de software; não substituem a aceitação física de controles, hotplug e reconexão no Windows.
- Os gates CPAL validam integração e transporte PCM em software; não substituem a validação de endpoint físico, latência, hotplug e underruns prolongados.
- Alterações exclusivamente documentais devem receber revisão de links, referências e consistência, sem executar testes Rust, Clippy ou builds de hardware sem relação com o diff.
- Alterações exclusivas em workflows não acionam por si só a bateria completa de testes de hardware. Alterações globais de manifests/lockfiles continuam executando a regressão Rust de workspace.

A matriz não declara paridade integral. O corte dos adaptadores anteriores é controlado pelo plano de retirada e só ocorre após R3/R5.

## 7. Preservação

MachineScheduler permanece autoridade de clocks/eventos. UI, renderização, áudio físico e input não comandam tempo virtual. Preservar ordem causal, WAIT states, operações retomáveis e estado necessário à restauração. Execução contínua e em blocos deve ser equivalente. Não duplicar worker, CPU, PSG, VDP ou Scheduler.

## 8. Licenças

Antes de distribuir binários, revisar licenças/notices de Slint, WGPU, CPAL e dependências transitivas. Registrar obrigações em THIRD_PARTY_NOTICES.md; confirmar termos oficiais vigentes para o uso pretendido.

## 9. Estado reconciliado — 2026-11-22

- R1: POC Slint/WGPU 30/Vulkan integrada em software; a confirmação física do backend Vulkan, lifecycle, resize e apresentação sustentada continua sendo gate no Windows x64.
- R2: frames SMS reais integrados via `SmsTemporalWorker`/`FrameMailbox`; a UI consome o frame publicado mais recente e não avança a emulação. O aceite de pacing e 60 FPS sustentados continua separado.
- R3: CPAL 0.18.2 e pipeline PCM/SPSC integrados em software; validação física prolongada, latência, hotplug/reabertura e underruns continuam pendentes.
- R4: primeira fatia SDL3/gamepad integrada ao host Slint; validação física de Xbox, controles PlayStation, USB/Bluetooth genéricos e hotplug/reconexão continua pendente.
- R5: consolidação do frontend ainda parcial.
- Slint/WGPU/CPAL/SDL3 não devem ser declarados fisicamente validados apenas por build ou CI.

O host anterior egui/D3D12 não é o caminho ativo do produto. Código remanescente deve ser inventariado como legado e removido somente após verificar todos os consumidores e workspaces.


## 10. Reconciliação de estado — 2026-11-16

R2 (frames SMS) e R3 (CPAL 0.18.2) estão integradas em software. A primeira fatia R4 (SDL3/input runtime no desktop Slint) foi integrada pela PR #867. Esses estados não significam aceite físico completo: Vulkan/lifecycle, reprodução prolongada/hotplug/underruns e controles reais/remapeamento ainda exigem evidência no HEAD atual. R5 permanece parcial.

O host anterior egui/D3D12 não é mais caminho ativo do produto. Código remanescente deve ser inventariado como legado e removido somente após verificar todos os consumidores e workspaces. Esta ADR não autoriza reativar nem desenvolver o host anterior.


## 11. Contrato mínimo de entrada, áudio e vídeo — 2026-11-22

Esta seção consolida as decisões aprovadas para reduzir o custo de implementação, compilação, revisão e validação. Ela complementa esta ADR sem criar uma nova camada de abstração e sem reabrir componentes já integrados.

### 11.1 Minimalismo de APIs e dependências

- Preferir Rust `std` e as APIs diretas das tecnologias já aprovadas.
- Não adicionar crates, frameworks, adaptadores, wrappers, traits ou interfaces genéricas sem necessidade técnica demonstrada.
- Reutilizar os contratos e componentes já existentes; não migrar uma implementação funcional apenas por preferência de estilo.
- Não criar APIs públicas ou contratos futuros sem consumidor atual.
- Preservar as operações coarse-grained de sessão e o ownership do runtime.
- Uma nova dependência só será aceita quando a biblioteca nativa ou a stack aprovada não resolver a necessidade com custo menor.

### 11.2 Entrada

- Slint permanece responsável pelos eventos nativos de teclado e pela interação da interface.
- SDL3 fica restrito a gamepads e joysticks físicos; não será usado para teclado, janela, vídeo ou áudio.
- Preservar o backend existente `sega-input-runtime::Sdl3Backend`, que carrega dinamicamente a ABI oficial SDL3 no Windows e usa Gamepad API para controles mapeados e Joystick API para estados brutos quando necessário.
- Não adicionar outra biblioteca de input nem migrar o backend existente para outro binding Rust sem limitação concreta e ganho demonstrado.
- O host coleta estados/snapshots e os encaminha ao runtime em fronteiras seguras. Não há polling físico, callbacks de botão ou leitura de dispositivo dentro do MachineScheduler.
- Aceitação física pendente: controles Xbox/XInput, DualShock 4, DualSense, controles USB e Bluetooth genéricos, conexão, desconexão e reconexão. Recursos avançados como rumble, sensores, touchpad e remapeamento visual ficam fora do aceite inicial.

### 11.3 Áudio

- CPAL **0.18.2**, fixado no lockfile, permanece como única API de saída física de áudio.
- PSG, YM2413, mixer e resampler continuam no runtime Rust existente; CPAL recebe PCM já preparado.
- Preservar o transporte SPSC/FIFO e o callback CPAL existentes. O callback não executa emulação, não acessa GUI, não faz I/O e não aloca buffers.
- O callback não controla nem corrige o MachineScheduler. Underruns são contabilizados e preenchidos conforme o contrato existente.
- No Windows, WASAPI é apenas o backend interno utilizado pelo CPAL. Não adicionar adaptador WASAPI próprio, XAudio2, SDL Audio, Rodio ou miniaudio.
- Aceitação física pendente no Windows 11: formatos e taxas realmente negociados, reprodução prolongada, latência, pause/stop/reset/troca de ROM, remoção/reabertura de endpoint e underruns/overruns.

### 11.4 Vídeo e apresentação

- Slint **1.18.x** permanece responsável pela GUI e composição; WGPU **30.x** é a única API gráfica do produto; Vulkan é selecionado como backend do WGPU no Windows x64.
- Não chamar Vulkan diretamente nem adicionar OpenGL, Direct3D, SDL Renderer, Skia ou outra API/framework gráfico paralelo.
- Slint e o caminho WGPU devem compartilhar o mesmo `wgpu::Device` e `wgpu::Queue` obtidos do renderer Slint. Não criar device, queue ou surface concorrentes para a mesma janela.
- Preservar `SmsTemporalWorker`, `FrameMailbox` e `VideoFrame` imutável. O host consome o frame publicado mais recente; a GUI nunca avança o runtime nem o MachineScheduler.
- Preservar RGB888 como formato canônico do frame do runtime SMS. A conversão para RGBA8 ocorre somente na fronteira de apresentação, evitando conversões e cópias repetidas.
- Texturas, upload e composição permanecem no WGPU. WGSL pode ser usado para shaders de apresentação quando houver requisito concreto, sem introduzir outra biblioteca ou alterar a semântica do VDP.
- Não exigir zero-copy físico por presunção. Primeiro preservar ownership e correção; depois medir upload, staging, cópias e composição para otimizar somente o gargalo demonstrado.
- A arquitetura de filtros/CRT e seus requisitos específicos pertencem às decisões próprias de vídeo; esta seção não declara filtros, HDR ou VRR implementados.

### 11.5 Workflow de compilação e validação

A validação deve ser proporcional ao diff e separada entre CI e aceite físico:

1. **Mudança documental:** revisar consistência com ADRs e código, links e formatação documental. Não executar cargo test, Clippy ou build de hardware para alterações exclusivamente `.md`.
2. **Mudança no host de vídeo:** executar o gate Windows x64 Release do desktop com `--locked`; validar Vulkan selecionado, device/queue compartilhados, textura, composição, resize, lifecycle e shutdown.
3. **Integração de frames:** validar ROM SMS real, continuidade visual, proporções, pause/resume e consumo de frames sem avanço de tempo pela GUI.
4. **Aceite de desempenho:** medir throughput de emulação separadamente de pacing/apresentação. A meta de produto permanece 60 FPS estáveis, com orçamento máximo de 16,67 ms por frame; registrar p95/p99 e investigar cópias, upload e alocações.
5. **Aceite físico:** executar no Windows 11 com Vulkan e dispositivos reais. CI/build bem-sucedidos não são evidência de apresentação física, reprodução de áudio ou compatibilidade física de controles.

Nenhuma otimização pode alterar clocks, ordem causal, resolução lógica, comportamento do VDP ou tempo emulado. Não reescrever componentes validados sem defeito ou regressão reproduzível.
