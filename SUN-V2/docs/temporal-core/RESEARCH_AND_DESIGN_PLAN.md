# Temporal Core — plano de pesquisa e especificação

**Status:** R0 APROVADO; R1 APROVADO em 2026-11-24 (conteúdo v0.3.18, PR #1281, merge SHA `88e105d6495f7f5105502e53666a7b0d29f77572`); R2 EM ANDAMENTO (issue #1282). R3–R5 bloqueados. Nenhuma implementação de produção autorizada.

## 1. Missão

Projetar o núcleo temporal que coordenará o avanço da máquina SMS. Ele será o fundamento de CPU, barramentos, RAM/VRAM, VDP, PSG e YM2413.

O Temporal Core não deve conhecer detalhes internos de registradores ou instruções de um dispositivo. Deve fornecer contratos temporais suficientemente expressivos para que a máquina modele causalidade, prazos e efeitos observáveis.

## 2. Documento arquitetural de referência

A proposta consolidada está em [CHRONOS_MINIMAL_HYBRID_SPEC.md](CHRONOS_MINIMAL_HYBRID_SPEC.md). Esse é o documento de referência para a direção recomendada: autoridade temporal única, propriedade de estado nos dispositivos e três mecanismos temporais mínimos — avanço limitado, sincronização sob demanda e agenda seletiva de deadlines autônomos.

Os documentos [CHRONOS_ARCHITECTURE_EVALUATION.md](CHRONOS_ARCHITECTURE_EVALUATION.md) e [CHRONOS_STRATEGIC_ASSESSMENT.md](CHRONOS_STRATEGIC_ASSESSMENT.md) preservam a análise dos modelos concorrentes, SWOT, stakeholders e riscos que fundamentaram a proposta. Eles não aprovam uma implementação específica.

## 3. Perguntas que precisam ser respondidas

### 3.1 Representação do tempo
- Qual unidade interna representa o tempo virtual?
- Inteiro de largura fixa, inteiro amplo, racional ou combinação?
- Como representar razões de clock não inteiras sem deriva acumulada?
- Como representar fase residual e conversões entre domínios?
- Como detectar e tratar overflow sem wrap silencioso?
- Quais garantias de precisão e intervalo são necessárias para sessões longas e save states?

### 3.2 Modelo de execução

A prototipagem seguirá um funil, não uma obrigação de implementar todos os modelos:
- P-A: baseline por instrução/orçamento;
- P-D: agenda de eventos discretos;
- P-E: sincronização sob demanda;
- P-F: híbrido mínimo, somente se a combinação resolver uma lacuna demonstrada;
- P-B: quanta/interleave, condicional;
- P-C: corrotinas, condicional.

Não escolher um modelo por familiaridade. A decisão deve refletir os requisitos reais do SMS e os custos medidos.

### 3.3 Ordenação e causalidade
- Como ordenar eventos com timestamp idêntico?
- Quais fases temporais são necessárias?
- Como definir a relação entre CPU, barramento, IRQ e dispositivos?
- Como impedir causalidade retroativa?
- O que acontece quando um evento agenda outro no mesmo timestamp?
- Como evitar starvation e eventos de delta-cycle infinitos?

### 3.4 Operações suspensíveis
- Como representar operações de barramento pendentes?
- Como modelar WAIT states sem bloquear a thread?
- Como suspender e retomar uma transação sem perder fase ou efeitos parciais?
- Quais pontos são seguros para pause, reset e save state?

A suspensão/retomada é contrato da CPU e do barramento, não um quarto mecanismo de scheduler.

### 3.5 Estado, replay e persistência
- Qual estado é necessário para snapshot determinístico?
- Como serializar agenda, sequência, fase, deadlines e operações pendentes?
- Como validar restore e replay contra execução contínua?
- Como versionar snapshots sem acoplar o formato a detalhes internos instáveis?

### 3.6 Concorrência e integração
- Qual estado pertence a cada componente?
- Quais cálculos podem ser paralelos sem introduzir sincronização cara?
- Como workers entregam resultados sem mudar o tempo virtual?
- Qual política de filas, backpressure e descarte é segura para cada tipo de dado?

## 4. Modelos e fontes a estudar

A pesquisa deve incluir documentação e código de projetos reconhecidos, com foco em:
- MAME: scheduler de dispositivos, timers, sincronização e interleave.
- Ares: arquitetura de sistemas, execução e sincronização.
- BlastEm: modelagem temporal do hardware Mega Drive/Genesis e sincronização.
- Gearsystem: Master System/Game Gear, timing e integração de dispositivos.
- Genesis Plus GX: sincronização de chips e execução de sistemas Sega.
- Documentação técnica primária do SMS/SG-1000 e testes de hardware relevantes.

Cada referência deve ser analisada no contexto em que foi construída. Não copiar uma arquitetura inteira de outro emulador sem demonstrar adequação ao SUN.

## 5. Entregáveis da pesquisa

1. Glossário temporal e unidades.
2. Matriz de evidências por requisito: afirmação, fonte primária, revisão de hardware, confiança, divergência, impacto e teste discriminante.
3. Catálogo de invariantes e propriedades obrigatórias.
4. Semântica de avanço, observação e deadlines.
5. Modelo de clock domains e conversão de fases.
6. Contrato de transações suspensíveis e WAIT states.
7. Semântica de eventos simultâneos.
8. Estratégia de snapshot, restore e replay.
9. Plano de concorrência e comunicação com workers.
10. Análise de riscos e ADR com decisão recomendada e alternativas rejeitadas.

## 6. Provas e testes exigidos antes da implementação

- Aritmética temporal e overflow.
- Conversão entre domínios e conservação de fase.
- Invariância por particionamento, quando permitida pelo contrato.
- Ordenação determinística para timestamps iguais.
- Agendamento de eventos no limite atual e no futuro.
- Prevenção de causalidade retroativa.
- Pausa/retomada de operações pendentes.
- WAIT state e retomada sem duplicação de efeitos.
- Snapshot/restore com agenda e transações não vazias.
- Replay determinístico.
- Carga com eventos densos e esparsos.
- Benchmarks de throughput, latência, alocações e memória.

Os testes devem começar como especificações executáveis ou modelos de referência independentes. Não devem simplesmente reproduzir o algoritmo que será implementado.

## 7. Benchmark e custo

Medir, no mínimo:
- custo por evento, avanço e sincronização;
- custo de inserir, cancelar e despachar eventos;
- custo por domínio de clock;
- alocações e bytes alocados no hot path;
- memória por evento e por domínio;
- p50, p95, p99 e pior caso;
- comportamento sob agenda densa e esparsa;
- custo de snapshot/restore;
- custo de tracing ligado e desligado.

Separar microbenchmarks de benchmarks integrados com a carga de trabalho SMS. Nenhum resultado sintético isolado será aceito como prova de ganho em gameplay. A baseline deve preceder a definição de limites numéricos de overhead.

## 8. Portões de aprovação

- **R0 — Pesquisa:** fontes, incertezas e requisitos documentados.
- **R1 — Especificação:** invariantes, semântica e contratos completos.
- **R2 — Modelo de referência:** testes independentes passam sem depender da implementação final.
- **R3 — Protótipos:** alternativas discriminantes implementadas isoladamente e comparadas.
- **R4 — Revisão adversarial:** revisor independente procura bugs, ambiguidades e alternativas mais eficientes.
- **R5 — Aprovação:** decisão arquitetural registrada; somente então começa o crate de produção.

Falhar em qualquer portão exige corrigir a especificação ou o modelo antes de avançar.


## 9. Execução atual — R2 aprovado; R3 em andamento

R1 foi aprovado em 2026-11-24. Conteúdo normativo R1 v0.3.18, SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76`, PR #1281 / merge `88e105d6495f7f5105502e53666a7b0d29f77572`.

R2 foi aprovado pelo responsável após parecer independente e corrigir os dois P2 documentais. PR #1283 foi merged em `main`, merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0`. CI #10798 SUCCESS no HEAD documental `a858f8f21caece3567385d64745823c655dbd135`: 256/256 Linux e Windows, mutation PASS, JSON e worktrees CLEAN. O oráculo R2 permanece como referência abstrata, não backend/runtime.

R3 está em andamento na branch `research/chronos-r3-disposable-prototypes`. A fase é estritamente experimental: protótipos descartáveis P-A, P-D, P-E e, somente se houver lacuna demonstrada, P-F; comparar semântica/cenários/cargas equivalentes e medir custo, p50/p95/p99, pior caso, memória e alocações. Não produzir crate, runtime ou integração CHRONOS/V2. R4 revisa adversarialmente os resultados; produção só após aprovação R5.


