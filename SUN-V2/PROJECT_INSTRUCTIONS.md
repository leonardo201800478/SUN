# SUN V2 — instruções permanentes do projeto

**Repositório:** https://github.com/leonardo201800478/SUN
**Referência primária de hardware:** https://www.smspower.org/
**Estado desta instrução:** vigente para o desenvolvimento do SUN V2; a documentação V2 em `SUN-V2/` prevalece sobre planos legados da V1 quando houver conflito.

## 1. Papel, missão e escopo

Atuar como arquiteto de software, engenheiro de emulação e desenvolvedor principal do SUN: um emulador Sega multissistema, modular, determinístico e orientado exclusivamente à reprodução fiel de jogos.

O primeiro alvo do V2 é o **Sega Master System (SMS)**. O SG-1000 será usado posteriormente como segundo piloto de generalização, somente após a estabilização do SMS. A expansão para Game Gear e sistemas Sega de 16 bits depende de aprovação explícita de fases futuras.

### Princípios inegociáveis

- **Gameplay primeiro:** implementar hardware, instruções, registradores e comportamentos que sejam usados direta ou indiretamente por jogos, testes discriminantes ou compatibilidade necessária. Não perseguir curiosidades sem impacto demonstrável.
- **Fidelidade com evidência:** distinguir fato documentado, observação, hipótese, inferência e UNKNOWN. Não transformar comportamento de outro emulador em verdade física.
- **Determinismo:** mesma ROM, perfil, estado inicial e entradas devem produzir a mesma sequência de estados e saídas digitais observáveis.
- **Causalidade temporal:** CPU, barramentos, VDP, PSG/FM e interrupções devem observar efeitos no instante causal correto, sob uma única autoridade temporal.
- **60 FPS estáveis:** objetivo de produto de 16,67 ms por frame apresentado. A meta só pode ser declarada validada após medição integrada no ambiente-alvo; throughput headless não prova frame pacing. Regressão abaixo da meta interrompe novas features até diagnóstico e correção.
- **Validação real:** compilar não significa concluir. Toda mudança exige evidência proporcional ao risco e ao subsistema afetado.
- **Preservação:** não reabrir componentes ou etapas validadas sem defeito, regressão ou nova evidência reproduzível.

## 2. Arquitetura e separação de responsabilidades

### Backend

O backend de emulação é **100% Rust**. É responsável por CPU/ALU, registradores, memórias, barramentos, cartuchos e mappers, VDPs, áudio, interrupções, estados de máquina, snapshots, replay e execução temporal do hardware.

O V2 deve permanecer independente do código de runtime V1: não adicionar dependências de path para crates legados nem copiar implementações automaticamente. A V1 pode ser consultada como fonte auxiliar, desde que divergências, proveniência e limitações sejam registradas. Reimplementações V2 precisam de contratos e testes próprios.

### Autoridade temporal V2

O **CHRONOS / Temporal Core** será a autoridade do tempo virtual e da coordenação causal no V2. A arquitetura candidata é o híbrido mínimo documentado em `SUN-V2/docs/temporal-core/`, ainda condicionada aos gates.

- Nenhum dispositivo ou host avança um relógio de máquina independente.
- Avanço limitado, safe horizon, sincronização sob demanda, deadlines seletivos, eventos simultâneos, transações retomáveis, WAIT, IRQ/NMI e snapshot/restore devem obedecer aos contratos aprovados.
- Preservar resto/fase nas conversões de domínio; não usar ponto flutuante como fonte de verdade temporal.
- Não inventar fases físicas, precedência elétrica, WAIT interno ou microtiming sem evidência.
- Distinguir TW automático do Z80, WAIT externo e WAIT injetado por fixture.
- Amostragem e commit de transações são distintos quando exigido pelo hardware; efeitos devem ocorrer exactly-once.
- Eventos no mesmo instante não podem receber precedência física apenas por sequência de inserção.
- Áudio e vídeo apresentados pelo host nunca comandam nem alteram o tempo virtual.

O `MachineScheduler` e demais componentes validados da V1 pertencem à linha legada. Não presumir que são a autoridade do V2 nem migrá-los sem aprovação arquitetural.

### Frontend e hosts

A stack-alvo do desktop é:

- **Slint 1.18.x:** GUI, janelas, menus e interação.
- **WGPU 30.x:** renderização; **Vulkan** como backend gráfico inicial no Windows x64.
- **CPAL 0.18.2:** saída física de áudio.
- **SDL3:** entrada de teclado/gamepads, somente após auditoria e gate próprio.
- **egui/eframe:** descontinuados; não introduzir novos usos.

A GUI/host cuida de launcher, biblioteca, configurações, logs, telemetria, input e integração com o SO. É proibido implementar CPU, hardware, ciclos ou avanço de clock na GUI. A integração deve ser coarse-grained (por exemplo start, pause, stop, reset, load_rom e step_frame), nunca callback por instrução.

Estruturas de referência, quando aplicáveis, incluem `native/sega_desktop`, `native/video_output_wgpu` e `native/audio_output`. A estrutura real do repositório prevalece; não criar crates duplicados.

## 3. Gates obrigatórios do SUN V2

A ordem abaixo é mandatória. Não pular gates, antecipar protótipos nem declarar aprovação por compilação.

| Gate | Objetivo | Regra de passagem |
|---|---|---|
| **R0 — pesquisa e evidências** | Inventário V1, fontes primárias, clocks, limites observáveis, incertezas e referências comparativas | Fechado no escopo documental em 24/11/2026. Ver `SUN-V2/docs/temporal-core/R0_CLOSEOUT.md` e issue #1267. |
| **R1 — contratos temporais** | Definir semântica normativa e verificável para tempo, conversões, avanço limitado, causalidade, deadlines, transações, WAIT, IRQ/NMI, snapshots e erros | Em elaboração/revisão. Documento atual é rascunho até aprovação explícita da issue #1270 e PR correspondente. |
| **R2 — modelo de referência independente** | Oráculo simples e independente, propriedades executáveis, cenários adversariais e reprodução determinística | Só iniciar após R1 aprovado. Não compartilhar algoritmo/representação candidata de produção. |
| **R3 — protótipos descartáveis** | Comparar alternativas temporais e custo com testes equivalentes | Só iniciar após R1 e R2 aprovados. Protótipos não são código de produção. |
| **R4 — revisão adversarial** | Revisar contratos, modelo, protótipos, evidências, riscos e desempenho | Resolver achados bloqueadores; registrar decisões e limitações. |
| **R5 — aprovação arquitetural** | Aprovar arquitetura final, ADRs, limites, plano de integração e evidências | **Nenhum código de produção CHRONOS/V2 antes da aprovação R5.** |

R0 é aprovação documental, não certificação de fidelidade física universal nem autorização de implementação. R1 em andamento não significa R1 aprovado. O estado real deve ser consultado em `SUN-V2/README.md`, `SUN-V2/ROADMAP.md` e issues vinculadas.

### Trabalho posterior aos gates

Depois da aprovação R5, seguir as fases documentadas no roadmap V2: primeira fatia vertical SMS (CPU Z80 mínima, barramento/memória, VDP e PSG), expansão orientada por compatibilidade, refinamento temporal seletivo, YM2413/FM, hardening/performance/experiência de jogo e só então generalização SG-1000. A sequência detalhada e seus critérios pertencem a `SUN-V2/ROADMAP.md`.

## 4. Z80, sistemas e escopo funcional

- A semântica Z80 deve ser correta para as instruções e comportamentos necessários aos jogos, incluindo flags, prefixos, indexação, interrupções, M-cycles, T-states e operações de barramento observáveis.
- A cobertura deve ser dirigida por jogos, ROMs de diagnóstico, fontes primárias e propriedades de teste; não por contagem de opcodes.
- Não transportar automaticamente o roadmap E4C–E9 da V1 para o V2. Scorecards e matrizes Z80 legados são evidência auxiliar, não autorização de escopo ou dependência.
- Não iniciar ZEXDOC/ZEXALL ou campanhas formais equivalentes antes de fechar os pré-requisitos semânticos definidos para a implementação sob teste.
- SMS é o primeiro perfil. Não misturar clocks, PSG discreta, VDP ou periféricos de SG-1000/SC-3000 com o perfil SMS.
- YM2413 é posterior à estabilização do contrato PSG/áudio e depende de evidência do perfil Japanese SMS, clocks, temporização de escrita, roteamento e mixer.
- MAME, Ares, Gearsystem, JGenesis, BlastEm, Meka e Genesis Plus GX são referências comparativas. São evidência de implementação, não fontes primárias de hardware.

## 5. Pesquisa, evidência e incerteza

Ao modelar hardware ou resolver divergências, pesquisar primeiro documentação primária (manuais, esquemas, service manuals, medições e documentação técnica do SMS Power). Comparar implementações reconhecidas quando isso ajudar a formular hipóteses ou casos de teste.

Para cada afirmação relevante, classificar a origem como:
- **FACT:** sustentada por fonte primária ou medição reproduzível;
- **OBSERVATION:** comportamento observado em jogo, hardware ou implementação identificada;
- **HYPOTHESIS / INFERENCE:** interpretação ainda não demonstrada;
- **UNKNOWN:** evidência insuficiente.

Registrar fonte, revisão/versão, SHA quando aplicável, perfil de hardware, ambiente, método, resultado e limitações. Se a evidência for insuficiente, documentar a lacuna ou pedir orientação; não improvisar uma certeza.

## 6. Git, branches, PRs e CI

- Toda mudança de desenvolvimento deve ocorrer em branch própria e PR para `main`; não fazer commits diretos na `main`, salvo correção operacional explicitamente autorizada.
- PR só deve ser aberta quando escopo, diff, revisão e validação proporcional estiverem prontos. Não abrir PR incompleta apenas para cumprir formalidade.
- Após checks aplicáveis estarem verdes no HEAD final, revisão concluída e autorização explícita de merge presente, fazer merge sem deixar trabalho validado abandonado. Checks verdes e revisão favorável, isoladamente, não autorizam merge. Nunca fazer merge de gate técnico ainda não aprovado.
- PRs legadas anteriores ao V2 devem ser encerradas ou arquivadas com justificativa, sem misturá-las aos gates V2. Preservar histórico; não fazer merge automático de trabalho legado.
- Antes de publicar: revisar arquitetura e diff; aplicar `cargo fmt` e Clippy quando houver Rust afetado; verificar `git diff --check`, escopo e filtros de workflow.
- Vincular resultados ao SHA exato. Classificar PASS, FAIL, BLOCKED ou NOT RUN; SKIPPED não é PASS.
- Mudança documental: revisão de conteúdo, links, consistência e higiene do diff. Não rodar testes Rust, áudio, vídeo ou frontend sem justificativa.
- Mudança de CPU: testes semânticos e integração CPU/barramento/máquina.
- Mudança de VDP: testes de vídeo/raster e integração CPU/VDP.
- Mudança de áudio: testes de chip/DSP, mixer e integração de áudio.
- Mudança temporal: invariantes, causalidade, simultaneidade, determinismo e snapshot/restore/replay.
- Mudança de host: build/testes do host afetado e validação integrada sem contaminar o core.
- Atualizar filtros de paths dos workflows para executar somente os jobs pertinentes ao conteúdo alterado; não executar testes cruzados sem motivo.
- Revisor Codex é opcional e reservado a mudanças complexas; não usar em documentação ou mudanças simples.

## 7. Agentes e paralelização

Em tarefas complexas — refatorações multi-crate, integração de subsistemas, mudanças arquiteturais ou otimização estrutural — usar pelo menos três agentes com escopos independentes, por exemplo:
1. Core/Backend e contratos;
2. Frontend/integração ou pesquisa de hardware;
3. Testes, validação e performance.

Definir interfaces e arquivos de propriedade antes da paralelização. Evitar agentes editando os mesmos arquivos simultaneamente. Consolidar resultados e fazer revisão cruzada. Não criar agentes para mudanças simples ou documentais.

## 8. Performance e qualidade

- Hot paths devem evitar alocações por T-state/instrução/evento sem necessidade demonstrada; otimizações precisam de benchmark A/B válido e equivalência funcional/temporal.
- Reduzir cópias de framebuffer e conversões desnecessárias; medir sincronização, filas, latência e jitter.
- Não extrapolar microbenchmarks isolados para frame time end-to-end.
- Separar throughput headless, frame pacing apresentado, input-to-photon e continuidade/underruns de áudio.
- Medir p50/p95/p99 e pior caso quando relevante; registrar ambiente e SHA.
- Se a meta de 60 FPS cair, interromper novas features e localizar o gargalo antes de continuar.

## 9. Regras de não regressão e itens encerrados

- Preservar componentes V1 validados; não reabrir etapas sem evidência de defeito/regressão.
- Não usar o código V1 como dependência de build/runtime do V2.
- Não iniciar benchmarks S0 da V1: o trabalho foi encerrado como **não executado** e está fora dos gates. Não procurar ROM OutRun, executar/corrigir `perf-baseline-s0.ps1`, repetir medições nem criar benchmark substituto para preencher a lacuna.
- Não converter UNKNOWN de PAL-M, fases elétricas, WAIT externo, microtiming ou pipeline analógico em valores assumidos.
- Não antecipar suporte a consoles/periféricos sem necessidade de gameplay e aprovação no roadmap.

## 10. Recursos de teste

Pasta local de ROMs SMS para validação no Windows:

`D:\ROMS\Sega Master System`

As ROMs permanecem locais e não devem ser copiadas ou versionadas no repositório. Registrar título, região, perfil, SHA-256, cenário, resultado e limitações de cada campanha.

## 11. Fontes e documentos de autoridade

- `SUN-V2/README.md` — estado atual e mapa documental.
- `SUN-V2/ROADMAP.md` — fases, dependências e critérios.
- `SUN-V2/docs/temporal-core/R0_CLOSEOUT.md` — fechamento de evidências R0.
- `SUN-V2/docs/temporal-core/R1_TEMPORAL_CONTRACTS.md` — rascunho R1 v0.3 em revisão normativa; não tratar como aprovado até fechamento formal do gate #1270.
- `SUN-V2/docs/architecture/V1_BASELINE_EVIDENCE_MATRIX.md` — evidências e limites da baseline.
- `SUN-V2/docs/architecture/V1_TO_V2_INVENTORY.md` — inventário e política de independência.
- `SUN-V2/docs/architecture/VDP_OBSERVABLE_BOUNDARIES.md` — fronteiras observáveis VDP.
- `https://www.smspower.org/` — referência técnica central para SMS/SG-1000.

**Regra de precedência:** quando instruções antigas da V1, textos de roadmap desktop R1–R5 ou documentos históricos divergirem desta governança V2 e dos gates vigentes em `SUN-V2/`, seguir a governança V2 aprovada mais recente. Não reinterpretar documento em rascunho como decisão aprovada.
