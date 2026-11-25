# SUN V2 — roadmap de execução

**Sistema inicial:** Sega Master System (SMS)

**Arquitetura temporal:** CHRONOS — híbrido mínimo, sujeito aos gates técnicos

**Estratégia:** componentes mínimos com contratos sólidos; integração vertical de produção somente após R5

**Status:** R0 APROVADO; R1 APROVADO em 24/11/2026; R2 APROVADO e MERGED pela PR #1283 (merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0`, CI #10798 SUCCESS, 256/256 Linux e Windows); R3 EM ANDAMENTO na branch `research/chronos-r3-disposable-prototypes`, restrito a protótipos temporais descartáveis e benchmarks comparativos; R4–R5 BLOQUEADOS; nenhuma implementação de produção iniciada.

**Branch de trabalho R3:** `research/chronos-r3-disposable-prototypes` (branch de protótipos isolada; nenhum código de produção).

**Branch de trabalho:** `architecture/sun-v2-foundation`

**R0:** [parecer de fechamento](docs/temporal-core/R0_CLOSEOUT.md) · [Issue #1267](https://github.com/leonardo201800478/SUN/issues/1267)

---

## 1. Decisão de execução

O SUN V2 será construído por **fatias verticais pequenas e mensuráveis**, não por uma longa sequência de componentes isolados e tampouco por uma implementação monolítica de toda a máquina.

A proposta do usuário — CPU, VDP e áudio mínimos com base sólida, refinados progressivamente — é adotada com um ajuste importante:

> Os componentes serão mínimos em escopo funcional, mas nascerão integrados aos contratos temporais, de barramento, snapshot e observabilidade. Não serão protótipos atômicos que precisem ser reescritos quando a sincronização real começar.

Após a aprovação R5, a primeira fatia executável de produção conterá CPU Z80 mínima, barramento/memória/cartucho mínimos, VDP SMS mínimo e PSG SN76489 mínimo, todos avançando sob uma única autoridade temporal. R2 está aprovado; R3 está autorizado e em execução apenas como protótipos descartáveis e benchmarks comparativos, sem integração de produção. Antes de R5, somente trabalhos dos gates R3–R4 são permitidos na sequência aprovada. Não implementar a fatia de produção antecipadamente. A fatia não precisa rodar imediatamente um catálogo amplo de jogos. Precisa executar cenários pequenos e reproduzíveis, revelar custo e permitir atribuir divergências ao componente responsável.

O YM2413 entra depois que o caminho PSG, os contratos de áudio multi-fonte e as evidências específicas do perfil Japanese SMS estiverem definidos. Não será usado para atrasar a primeira fatia, mas sua integração futura será prevista desde o desenho do contrato de áudio.

## 2. O que muda em relação à V1

A V1 já contém muito conhecimento funcional aproveitável: semântica Z80, memória/cartucho, I/O, VDP SMS, PSG, mixer, resampler, runtime e testes. Isso reduz o risco de pesquisa e fornece referências comparativas. Não significa que seus componentes possam ser copiados como blocos prontos para o V2.

A diferença fundamental é a ordem de construção e o contrato de integração:

| Dimensão | V1 — conhecimento existente | V2 — direção |
|---|---|---|
| CPU | `Z80::step` continua atômico; a V1 também possui `Z80MicroSequencer` retomável, ativo em `SmsTemporalMachine`, mas com cobertura delimitada e não equivalente a precisão elétrica universal | Estado arquitetural e semântica correta desde o início; microsequência apenas nas operações temporalmente observáveis |
| Barramento | Handshake com request ID, WAIT acumulado e conclusão agendada; WAIT adicional é fixture, I/O/INT têm TW automáticos e WAIT físico não é amostrado pela máquina SMS V1 | Contrato explícito de transação, timestamp, observação, WAIT quando exigido por evidência e efeito exactly-once |
| VDP | SMS1 integrado ao Scheduler em scanline/span; 228 T-states/342 HPos, boundaries T13–T17 e HBlank seletivo; sem fetch por dot universal; SMS2 HBlank/raster-write não caracterizado | Núcleo funcional mínimo conectado a limites causais desde o primeiro protótipo; granularidade adicional só por evidência |
| PSG | Core digital integrado; divisor /16, fase e LFSR; período de tom <=1 tratado como nível alto; perfil PAL-M V1 usa clock NTSC em conflito com modelo MAME | Core isolável, avanço por tempo explícito, fase preservada e saída temporizada que permita comparar blocos |
| FM | Caminho opcional Japanese SMS, core/serviço serial e PCM de compatibilidade já existem; qualidade sonora não aprovada e C-Sync/DAC/placa ainda pendentes | Core separado e integração posterior, condicionada ao perfil físico, clock/fase, $F0–$F2, MO/RO e mixer |
| Scheduler | MachineTime u128, razões racionais, ordenação e snapshot do Scheduler existem; snapshot isolado não é save state completo da máquina; stress histórico precisa ser vinculado ao SHA executado | CHRONOS especificado/testado antes de virar dependência de todos os componentes |
| Validação | Testes por subsistema e integração acumulados em diferentes etapas | Modelo de referência independente, testes por componente e testes cruzados em cada fatia vertical |
| Performance | Otimizações sobre runtime integrado | Baseline desde a primeira fatia, atribuição de custo por componente e orçamento antes de adicionar mecanismos |

**O que não muda:** o conhecimento da V1 continua valioso. Testes, vetores, ROMs de diagnóstico, resultados reproduzíveis e documentação podem orientar o V2, desde que origem, licença, SHA e limitações sejam registrados. O código legado não será dependência nem será copiado sem análise e reimplementação independente.

## 3. Regras de construção

1. **Mínimo não significa improvisado.** Cada componente começa com um contrato estável e uma lista explícita de funcionalidades incluídas/excluídas.
2. **Integração na primeira fatia pós-R5.** Não deixar CPU, VDP e áudio isolados por meses depois da autorização de produção; conectá-los conforme os contratos e ADRs aprovados.
3. **Uma autoridade temporal.** Nenhum componente avança seu próprio relógio de máquina de forma independente.
4. **Precisão proporcional.** Modelar detalhe temporal quando ele altera gameplay, interação observável ou resultado de teste; manter o restante atômico quando seguro.
5. **Sem falsa completude.** “Compila”, “renderiza uma tela” e “emite áudio” não significam fidelidade temporal.
6. **V1 como oráculo auxiliar, não verdade absoluta.** Divergências devem ser classificadas e confrontadas com documentação, testes independentes e, quando disponível, hardware.
7. **Sem duplicação permanente.** Protótipos são descartáveis; cada componente terá uma única implementação ativa no V2 após aprovação.
8. **Medição desde cedo.** Cada fatia terá custo de CPU, memória, eventos, sincronizações, alocações, áudio e frame time medidos.
9. **Sem host no caminho causal.** Slint, WGPU, CPAL, VSync e o SO não comandam o tempo virtual.
10. **Sem expansão prematura.** SMS fecha primeiro; SG-1000 será o segundo piloto de generalização.

## 4. Roadmap por fases

### Fase 0 — inventário V1 e fechamento de evidências (R0)

**Status: concluída — 24/11/2026**

O gate R0 foi aprovado no escopo documental. O parecer completo está em [R0_CLOSEOUT.md](docs/temporal-core/R0_CLOSEOUT.md); a issue [#1267](https://github.com/leonardo201800478/SUN/issues/1267) foi encerrada.

Entregas:
- baseline V1 congelada em 207ccc4cebbc4f8c0aed45c68262eae963491689;
- inventário V1→V2 e matriz de evidências revisados;
- fontes primárias Z80, clocks, áudio e limites observáveis VDP documentados;
- comparação arquitetural com snapshots de MAME, Ares, Gearsystem, BlastEm e Genesis Plus GX;
- incertezas PAL-M, WAIT externo, fases elétricas, microtiming VDP e pipeline analógico classificadas como UNKNOWN/adiáveis por perfil;
- testes discriminantes definidos para R2 e para protótipos dependentes.

**Limites da aprovação:** R0 não certifica fidelidade física universal, não escolhe representação temporal nem estrutura de agenda e não autoriza código de produção.

**S0 da V1:** encerrado como não executado e fora dos gates. Não procurar ROM OutRun, executar/corrigir script, repetir medições ou criar benchmark substituto.

### Fase 1 — contratos temporais (R1)

**Status: CONCLUÍDA — APROVADA em 24/11/2026.**

Conteúdo normativo v0.3.18 aprovado no SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76` e publicado na `main` pela PR #1281 (merge SHA `88e105d6495f7f5105502e53666a7b0d29f77572`). Issue #1270 encerrada. A aprovação cobre o contrato documental; não autoriza R2 além de seu próprio gate, protótipos R3 ou código de produção.

Entregas:
- contratos verificáveis de monotonicidade, conversões, progresso local, H_d, watermark e avanço limitado;
- sincronização sob demanda e dependências causais;
- deadlines, simultaneidade, microsteps e UNKNOWN sem precedência inventada;
- transações retomáveis, amostragem/commit exactly-once e WAIT;
- IRQ/NMI/acknowledge, snapshot/restore, replay, erros e independência do host;
- classificação FACT/OBSERVATION/HYPOTHESIS/INFERENCE/UNKNOWN e rastreabilidade;
- revisão adversarial e critérios formais de saída.

**Gate R1:** APROVADO. Critérios da seção 14 atendidos no conteúdo v0.3.18; revisão independente do SHA exato concluída sem major issues; aprovação explícita do responsável registrada na issue #1270. Nenhuma alteração posterior deste registro modifica a semântica normativa aprovada.

### Fase 2 — modelo de referência independente (R2)

**Status: IMPLEMENTAÇÃO EXECUTÁVEL T01–T15 COMPLETA PARA VALIDAÇÃO EXTERNA; GATE FORMAL PENDENTE — issue #1282 / PR #1283.** Consulte [R2_CLOSEOUT.md](docs/temporal-core/R2_CLOSEOUT.md), [R2_REFERENCE_MODEL.md](docs/temporal-core/R2_REFERENCE_MODEL.md) e a matriz [R2_TRACEABILITY.md](docs/temporal-core/R2_TRACEABILITY.md). Nenhum R3 ou código de produção autorizado.

Entregas:
- oráculo simples e independente do algoritmo/representação candidata;
- propriedades executáveis de monotonicidade, conversão, chunk invariance, avanço seguro, causalidade, deadlines, transações, WAIT, IRQ/NMI, snapshot/restore e replay;
- cenários adversariais com seeds, perfil, entradas, primeira divergência e estado mínimo reproduzível;
- UNKNOWN físico preservado como indeterminação/restrição, nunca promovido a FACT.

**Gate R2:** não aprovado. A implementação e a cobertura executável T01–T15 existem, evidências executáveis, mutation testing, JSON e runner Windows estão PASS; os 11 findings P2 conhecidos foram respondidos e resolvidos. A saída do gate ainda exige revisão humana externa independente do SHA final sem blockers e aprovação explícita do responsável. A revisão Codex foi solicitada, mas o serviço respondeu limite de uso atingido. Nenhum R3 começa antes de todos esses itens e da aprovação formal R2.

### Fase 3 — protótipos temporais descartáveis (R3; somente após R1 e R2)

Entregas:
- comparar P-A, P-D e P-E sob semântica, cenários e cargas equivalentes;
- implementar P-F apenas se os modelos simples demonstrarem lacuna real;
- comparar custo de avanço, sincronização, deadlines, cancelamento, dispatch, microsteps, snapshot/restore, alocações e memória;
- medir p50/p95/p99 e pior caso em cargas esparsas/densas;
- descartar alternativas sem ganho demonstrável e registrar hipóteses falsificadas/sustentadas.

Protótipos R3 são descartáveis: não são a primeira fatia integrada, não viram código de produção e não autorizam integração permanente de CPU/VDP/PSG.

**Gate R3:** comparação reproduzível e decisão técnica sustentada por evidência, custo e limitações.

### Fase 4 — revisão adversarial e aprovação arquitetural (R4/R5)

- **R4:** revisar contratos, modelo, protótipos, evidências, riscos, desempenho e UNKNOWNs; resolver blockers.
- **R5:** aprovar arquitetura final, ADRs, limites, estratégia de validação e plano de integração.

**Gate R5:** aprovação arquitetural explícita. Somente depois de R5 a implementação de produção CHRONOS/V2 pode começar.

### Fase 5 — primeira fatia vertical mínima de produção (pós-R5)

A primeira fatia será pequena, mas conterá os quatro elementos conectados:

| Elemento | Escopo mínimo inicial | O que fica fora inicialmente |
|---|---|---|
| CPU Z80 | Registradores essenciais, fetch/decode/execute de um subconjunto documentado, flags e duração nominal correta; bus requests explícitos nas operações que precisam ser observadas | Cobertura ampla de opcodes/prefixos não necessários ao cenário inicial; fases internas sem consumidor |
| Barramento/memória | ROM de teste, RAM mínima, mapa de memória e I/O essencial; transações com timestamp e conclusão exactly-once | Catálogo amplo de mappers, periféricos e wait states sem evidência |
| VDP SMS | Perfil SMS1 inicial, VRAM/CRAM, registradores/portas essenciais, raster mínimo, frame determinístico e IRQs requeridas pelo cenário | Todos os modos legados, revisões de ASIC, fetch por dot e quirks sem teste discriminante |
| PSG SN76489 | Tone/noise, latch/data writes, divisores, atenuação, clock/fase e saída digital determinística | Modelagem analógica de placa, filtros complexos e FM |
| Máquina SMS mínima | Integra CPU, barramento, VDP, PSG e CHRONOS; ROM sintética/homebrew de teste; input reproduzível | Launcher completo, biblioteca e recursos de produto |

A seleção de instruções Z80 deve ser dirigida pelos cenários de teste da fatia. O subconjunto é uma etapa de desenvolvimento, não uma promessa de compatibilidade de jogos. A semântica adicionada deve ser correta; não criar instruções falsas ou placeholders silenciosos.

**Gate F2:** a máquina executa uma sequência reproduzível; CPU lê/escreve memória e portas; VDP produz frame esperado; IRQ/entrada seguem o contrato; PSG produz sequência digital determinística; trace e snapshot identificam o estado de cada componente.

### Fase 6 — expansão funcional orientada por compatibilidade (F3, após F2)

**Prioridade: alta, após a primeira fatia passar.**

Ordem:
1. expandir cobertura Z80 e prefixos conforme matriz semântica e ROMs alvo;
2. expandir mapa de memória, cartucho e mappers exigidos pelos jogos selecionados;
3. completar registradores, modos e comportamentos funcionais do VDP;
4. expandir portas, controles e periféricos com impacto em jogos;
5. fechar regressões de cada ampliação antes de iniciar a próxima.

A expansão será guiada por uma matriz de compatibilidade e não por contagem de features. A cobertura CPU e a fidelidade temporal serão acompanhadas como eixos diferentes.

**Gate F3:** jogos de referência executam sem falhas funcionais conhecidas atribuíveis às funcionalidades implementadas; cada lacuna restante está documentada e priorizada.

### Fase 7 — refinamento temporal seletivo (F4)

**Prioridade: máxima para fidelidade, após base funcional estável.**

- transformar operações de barramento que exigirem em microsequências retomáveis;
- manter WAIT no contrato genérico, mas ativar apenas fontes comprovadas; não inventar WAIT interno de VDP/PSG SMS. Investigar WAIT externo do cartridge port e manter PSG discreta SG-1000/SC-3000 em perfil separado;
- refinar H/V counter, raster, IRQ, latches, acesso a VRAM/CRAM e efeitos mid-scanline onde jogos/testes observarem;
- revisar alinhamento de CPU, VDP e PSG em NTSC/PAL;
- introduzir deadlines seletivos somente para eventos autônomos necessários;
- comparar execução em blocos com traces de referência e cenários discriminantes.

**Gate F4:** cada refinamento tem evidência, teste antes/depois e custo atribuído; nenhuma precisão adicional entra apenas por simetria ou estética arquitetural.

### Fase 8 — áudio completo e YM2413 (F5)

**Prioridade: alta, mas posterior ao caminho PSG estável.**

1. validar SN76489 por vetores, clocks, fase, chunking e comparação PCM/eventos;
2. definir o contrato de áudio multi-fonte e o alinhamento comum de timestamps;
3. implementar o core YM2413 puro em Rust como componente isolado;
4. validar registradores, latch, temporização de escrita, slots, operadores, envelopes, modulação, feedback, presets e rhythm;
5. integrar apenas no perfil Japanese SMS com evidência de placa e clock;
6. fechar $F0/$F1/$F2, C-Sync, mute/roteamento e saídas MO/RO;
7. testar mixer, headroom, conversão de taxa, snapshot/restore e replay.

**Gate F5:** PSG e FM não alteram o clock da máquina; mute não pausa síntese sem evidência; mistura e restore são determinísticos; regressões de vídeo/CPU permanecem PASS.

### Fase 9 — hardening, performance e experiência de jogo (F6)

**Prioridade: obrigatória antes de declarar SMS estável.**

- testes longos, overflow e estado pendente;
- save state/restore e replay diferencial;
- catálogo de ROMs de diagnóstico e jogos reais;
- benchmark headless com e sem YM2413;
- profiling de alocações e hot paths;
- 60 FPS estáveis na apresentação, medidos separadamente de throughput headless;
- input-to-photon ponta a ponta;
- áudio sem underruns recorrentes e sem filas excessivas;
- validação Windows x64 com Slint 1.18, WGPU 30/Vulkan e CPAL 0.18.2;
- revisão adversarial e documentação de suporte/limites.

**Gate F6:** fidelidade, desempenho, restore/replay, host e experiência real aprovados no mesmo SHA candidato.

### Fase 10 — generalização e segunda máquina (F7)

Somente após SMS estável:
- testar primitivas temporais com SG-1000;
- separar requisitos genéricos dos específicos do SMS;
- medir o custo de adaptação e alterações no CHRONOS;
- rejeitar abstrações que só existam para antecipar um console ainda não validado.

## 5. Ordem prática de implementação após R5

Antes de R5, o trabalho fica limitado aos gates R1 (contratos), R2 (modelo independente), R3 (protótipos descartáveis), R4 (revisão) e R5 (aprovação arquitetural). Não criar crates, componentes integrados ou fatia de produção durante esses gates.

Após R5, a ordem da primeira fatia vertical será:

1. implementar o CHRONOS mínimo conforme arquitetura/ADRs aprovados;
2. construir barramento/memória mínimos com transações retomáveis;
3. conectar CPU Z80 mínima ao barramento;
4. conectar VDP SMS mínimo e somente os deadlines/boundaries exigidos;
5. conectar PSG SN76489 ao mesmo tempo virtual;
6. integrar a máquina SMS vertical e executar testes cruzados;
7. expandir compatibilidade e refinar temporalmente em ciclos curtos, conforme fases F3/F4.

As etapas 2–5 podem ter desenvolvimento em paralelo somente quando interfaces e contratos estiverem congelados pela arquitetura R5, mas cada incremento deve integrar e validar a fatia completa. Não haverá implementações isoladas permanentes nem antecipação de produção antes de R5.

## 6. Política de uso da V1

### Pode ser reutilizado como conhecimento e evidência auxiliar
- documentação de hardware e auditorias;
- listas de casos de teste e ROMs de diagnóstico;
- vetores de referência com proveniência e licença verificadas;
- comportamento funcional observado em jogos;
- benchmarks e metodologias, se revalidados;
- resultados de comparação, sempre vinculados ao SHA original.

### Não será reutilizado automaticamente
- implementação de CPU, VDP, PSG, Scheduler ou barramento;
- estruturas de dados ou APIs;
- decisões temporais sem nova análise;
- testes que apenas confirmem a própria implementação V1;
- resultados de CI antigos como evidência do V2.

Cada componente V2 terá implementação independente e testes próprios. Quando a V1 divergir de documentação, teste independente ou evidência física, a divergência será investigada, não copiada.

## 7. Métricas obrigatórias desde a primeira fatia

| Métrica | Uso |
|---|---|
| Tempo de CPU por frame/instrução | Custo por componente |
| Custo do CHRONOS por avanço e evento | Overhead temporal |
| Eventos e sincronizações por frame | Complexidade operacional |
| Alocações no hot path | Estabilidade de performance |
| Memória de estado e agenda | Custo de snapshot e execução |
| p50/p95/p99 e pior caso | Jitter e cauda de latência |
| Hash de frame e estado | Determinismo/regressão |
| Hash ou comparação de sequência PCM/eventos | Continuidade de áudio |
| Frames/s headless | Throughput sem host |
| Frame pacing de apresentação | Experiência real a 60 FPS |
| Input-to-photon | Latência ponta a ponta |
| Divergências por ROM/caso | Progresso de compatibilidade |

Não estabelecer limites numéricos de overhead sem baseline. Uma regressão abaixo da meta interrompe novas features até o gargalo ser medido e corrigido.

## 8. Gates e governança

Nenhuma etapa é concluída somente por compilação.

- Mudança de CPU: testes semânticos, barramento e integração CPU/máquina.
- Mudança de VDP: testes de vídeo/raster e integração CPU/VDP.
- Mudança de áudio: testes de chip/DSP e integração de áudio.
- Mudança temporal: invariantes, simultaneidade, determinismo e restore/replay.
- Mudança de host: validação da integração exposta sem contaminar o core.
- Mudança documental: revisão de links, consistência e escopo; não executar testes de código sem motivo.

Toda decisão estrutural deve ter ADR. Protótipos de pesquisa podem ser descartados. Código de produção só é autorizado após R1, R2, R3, R4 e R5 concluídos na ordem obrigatória; a aprovação R5 é a autorização explícita para começar a implementação de produção.

## 9. Estado inicial e próxima ação

Estado atual:
- SUN V2 permanece isolado em `SUN-V2/`.
- CHRONOS híbrido mínimo está documentado como proposta.
- Nenhum código de produção V2 foi iniciado.
- A V1 oferece componentes e auditorias como fonte de conhecimento, mas não será dependência do V2.
- R0 aprovado no escopo documental; nenhuma implementação de produção autorizada.

**Próxima ação:** concluir o fechamento formal de R1 na issue #1270, registrando a auditoria da seção 14 e a revisão Codex sem major issues do HEAD exato. Manter R2 bloqueado até aprovação explícita de R1; depois, iniciar R2 como modelo independente e propriedades executáveis, sem antecipar protótipos R3 ou produção. R3 só começa após R1 e R2 aprovados; R4/R5 precedem produção. Não retomar S0 nem criar benchmark substituto.
