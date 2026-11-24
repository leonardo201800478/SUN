# SUN V2 — roadmap de execução

**Sistema inicial:** Sega Master System (SMS)  
**Arquitetura temporal:** CHRONOS — híbrido mínimo, sujeito aos gates técnicos  
**Estratégia:** componentes mínimos com contratos sólidos, integrados desde cedo em uma fatia vertical  
**Status:** roadmap aprovado como direção de trabalho; nenhuma implementação de produção iniciada  
**Branch de trabalho:** `architecture/sun-v2-foundation`  
**R0:** [Issue #1267 — pesquisa e especificação do Temporal Core](https://github.com/leonardo201800478/SUN/issues/1267)

---

## 1. Decisão de execução

O SUN V2 será construído por **fatias verticais pequenas e mensuráveis**, não por uma longa sequência de componentes isolados e tampouco por uma implementação monolítica de toda a máquina.

A proposta do usuário — CPU, VDP e áudio mínimos com base sólida, refinados progressivamente — é adotada com um ajuste importante:

> Os componentes serão mínimos em escopo funcional, mas nascerão integrados aos contratos temporais, de barramento, snapshot e observabilidade. Não serão protótipos atômicos que precisem ser reescritos quando a sincronização real começar.

A primeira fatia executável deve conter CPU Z80 mínima, barramento/memória/cartucho mínimos, VDP SMS mínimo e PSG SN76489 mínimo, todos avançando sob uma única autoridade temporal. A fatia não precisa rodar imediatamente um catálogo amplo de jogos. Precisa executar cenários pequenos e reproduzíveis, revelar custo e permitir atribuir divergências ao componente responsável.

O YM2413 entra depois que o caminho PSG, os contratos de áudio multi-fonte e as evidências específicas do perfil Japanese SMS estiverem definidos. Não será usado para atrasar a primeira fatia, mas sua integração futura será prevista desde o desenho do contrato de áudio.

## 2. O que muda em relação à V1

A V1 já contém muito conhecimento funcional aproveitável: semântica Z80, memória/cartucho, I/O, VDP SMS, PSG, mixer, resampler, runtime e testes. Isso reduz o risco de pesquisa e fornece referências comparativas. Não significa que seus componentes possam ser copiados como blocos prontos para o V2.

A diferença fundamental é a ordem de construção e o contrato de integração:

| Dimensão | V1 — conhecimento existente | V2 — direção |
|---|---|---|
| CPU | Executor funcional amplo; `Z80::step` documentado como instrução completa, com T-states agregados e sem suspensão geral do barramento | Estado arquitetural e semântica correta desde o início; microsequência apenas nas operações temporalmente observáveis |
| Barramento | Caminhos de máquina existentes e evolução temporal incremental | Contrato explícito de transação, timestamp, observação, WAIT quando exigido e efeito exactly-once |
| VDP | Renderer SMS funcional, predominantemente scanline/span com fronteiras seletivas já investigadas | Núcleo funcional mínimo conectado a limites causais desde o primeiro protótipo; granularidade adicional só por evidência |
| PSG | Core SN76489 digital, clock/divisor, geração de amostras e pipeline DSP | Core isolável, avanço por tempo explícito, fase preservada e saída temporizada que permita comparar blocos |
| FM | Auditoria de YM2413 e requisitos de placa/roteamento documentados; síntese ainda não é pressuposto do runtime base | Core separado e integração posterior, condicionada ao perfil físico, clock/fase, $F0–$F2, MO/RO e mixer |
| Scheduler | Infraestrutura e integração evoluíram ao longo do projeto, com limites temporais conhecidos | CHRONOS especificado/testado antes de virar dependência de todos os componentes |
| Validação | Testes por subsistema e integração acumulados em diferentes etapas | Modelo de referência independente, testes por componente e testes cruzados em cada fatia vertical |
| Performance | Otimizações sobre runtime integrado | Baseline desde a primeira fatia, atribuição de custo por componente e orçamento antes de adicionar mecanismos |

**O que não muda:** o conhecimento da V1 continua valioso. Testes, vetores, ROMs de diagnóstico, resultados reproduzíveis e documentação podem orientar o V2, desde que origem, licença, SHA e limitações sejam registrados. O código legado não será dependência nem será copiado sem análise e reimplementação independente.

## 3. Regras de construção

1. **Mínimo não significa improvisado.** Cada componente começa com um contrato estável e uma lista explícita de funcionalidades incluídas/excluídas.
2. **Integração cedo.** Não deixar CPU, VDP e áudio isolados por meses para conectá-los somente no final.
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

**Prioridade: máxima.**

Objetivo: extrair o conhecimento útil da V1 sem herdar suas suposições.

Entregas:
- [inventário crítico V1→V2](docs/architecture/V1_TO_V2_INVENTORY.md) como baseline inicial, a reconfirmar no SHA congelado;
- matriz V1 → V2 por subsistema: comportamento existente, evidência, testes reutilizáveis como referência, lacunas e decisão de reimplementação;
- matriz de evidências de clocks NTSC/PAL, fase, Z80/barramento, WAIT, IRQ/NMI, VDP e áudio;
- catálogo de ROMs de diagnóstico e casos de gameplay, com hash e resultado esperado;
- baseline de desempenho e metodologia headless;
- mapa de riscos e discrepâncias;
- classificação de fatos, hipóteses, inferências e itens desconhecidos.

**Gate F0:** não promover hipótese de hardware a constante de API sem evidência suficiente; fontes e incertezas rastreáveis; escopo mínimo da primeira fatia definido.

### Fase 1 — contratos temporais e modelo de referência (R1–R2)

**Prioridade: máxima; bloqueia os protótipos integrados.**

Entregas:
- representação candidata de tempo e conversão entre domínios, escolhida por testes e não por preferência;
- contrato de avanço limitado, sincronização sob demanda e deadlines;
- ordenação de eventos simultâneos e política para eventos no timestamp atual;
- contrato de transação de barramento retomável;
- semântica de snapshot/restore dos estados pendentes;
- modelo de referência independente e propriedades executáveis;
- harness headless determinístico com clock controlável, entradas registradas e traces.

**Gate F1:** monotonicidade, conservação de fase, chunk invariance aplicável, ordenação determinística, ausência de causalidade retroativa e restore/replay demonstrados no modelo de referência.

### Fase 2 — primeira fatia vertical mínima (R3 — protótipos)

**Prioridade: alta; primeiro marco executável do V2.**

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

### Fase 3 — expansão funcional orientada por compatibilidade

**Prioridade: alta, após a primeira fatia passar.**

Ordem:
1. expandir cobertura Z80 e prefixos conforme matriz semântica e ROMs alvo;
2. expandir mapa de memória, cartucho e mappers exigidos pelos jogos selecionados;
3. completar registradores, modos e comportamentos funcionais do VDP;
4. expandir portas, controles e periféricos com impacto em jogos;
5. fechar regressões de cada ampliação antes de iniciar a próxima.

A expansão será guiada por uma matriz de compatibilidade e não por contagem de features. A cobertura CPU e a fidelidade temporal serão acompanhadas como eixos diferentes.

**Gate F3:** jogos de referência executam sem falhas funcionais conhecidas atribuíveis às funcionalidades implementadas; cada lacuna restante está documentada e priorizada.

### Fase 4 — refinamento temporal seletivo

**Prioridade: máxima para fidelidade, após base funcional estável.**

- transformar operações de barramento que exigirem em microsequências retomáveis;
- implementar WAIT e amostragem nas fases comprovadamente consumidas pelo SMS;
- refinar H/V counter, raster, IRQ, latches, acesso a VRAM/CRAM e efeitos mid-scanline onde jogos/testes observarem;
- revisar alinhamento de CPU, VDP e PSG em NTSC/PAL;
- introduzir deadlines seletivos somente para eventos autônomos necessários;
- comparar execução em blocos com traces de referência e cenários discriminantes.

**Gate F4:** cada refinamento tem evidência, teste antes/depois e custo atribuído; nenhuma precisão adicional entra apenas por simetria ou estética arquitetural.

### Fase 5 — áudio completo e YM2413

**Prioridade: alta, mas posterior ao caminho PSG estável.**

1. validar SN76489 por vetores, clocks, fase, chunking e comparação PCM/eventos;
2. definir o contrato de áudio multi-fonte e o alinhamento comum de timestamps;
3. implementar o core YM2413 puro em Rust como componente isolado;
4. validar registradores, latch, temporização de escrita, slots, operadores, envelopes, modulação, feedback, presets e rhythm;
5. integrar apenas no perfil Japanese SMS com evidência de placa e clock;
6. fechar $F0/$F1/$F2, C-Sync, mute/roteamento e saídas MO/RO;
7. testar mixer, headroom, conversão de taxa, snapshot/restore e replay.

**Gate F5:** PSG e FM não alteram o clock da máquina; mute não pausa síntese sem evidência; mistura e restore são determinísticos; regressões de vídeo/CPU permanecem PASS.

### Fase 6 — hardening, performance e experiência de jogo

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

### Fase 7 — generalização e segunda máquina

Somente após SMS estável:
- testar primitivas temporais com SG-1000;
- separar requisitos genéricos dos específicos do SMS;
- medir o custo de adaptação e alterações no CHRONOS;
- rejeitar abstrações que só existam para antecipar um console ainda não validado.

## 5. Ordem prática de implementação

A ordem não será “terminar toda a CPU, depois todo o VDP, depois todo o som”. Isso esconderia por muito tempo os problemas de integração temporal.

A ordem operacional será:

1. harness headless e modelo de referência;
2. CHRONOS mínimo e contratos de tempo;
3. barramento/memória mínimos;
4. CPU mínima conectada ao barramento;
5. VDP mínimo conectado ao barramento e aos deadlines necessários;
6. PSG mínimo conectado ao mesmo tempo virtual;
7. máquina SMS vertical e testes cruzados;
8. expansão funcional e refinamento temporal em ciclos curtos.

As etapas 4–6 podem ter desenvolvimento de componentes em paralelo quando seus contratos estiverem congelados, mas cada incremento deve integrar e validar a fatia completa. Não haverá três implementações isoladas que só se encontrem no fim.

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

Toda decisão estrutural deve ter ADR. Protótipos de pesquisa podem ser descartados. Código de produção só é autorizado após a especificação, referência, protótipos comparáveis e revisão independente aprovados.

## 9. Estado inicial e próxima ação

Estado atual:
- SUN V2 permanece isolado em `SUN-V2/`.
- CHRONOS híbrido mínimo está documentado como proposta.
- Nenhum código de produção V2 foi iniciado.
- A V1 oferece componentes e auditorias como fonte de conhecimento, mas não será dependência do V2.
- R0 permanece aberto.

**Próxima ação:** executar a Fase 0, produzir a matriz de inventário V1→V2 e fechar as evidências críticas de clocks/barramento/VDP/áudio. Em seguida, escrever e validar os contratos R1 e o modelo de referência R2. A primeira implementação experimental será a fatia vertical mínima da Fase 2 — não três chips isolados.
