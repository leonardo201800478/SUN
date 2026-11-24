# Temporal Core — plano de pesquisa e especificação

**Status:** levantamento comparativo e avaliação arquitetural registrados; R0 permanece em fechamento. Nenhuma implementação de produção autorizada.

## 1. Missão

Projetar o núcleo temporal que coordenará o avanço da máquina SMS. Ele será o fundamento de CPU, barramentos, RAM/VRAM, VDP, PSG e YM2413.

O Temporal Core não deve conhecer detalhes internos de registradores ou instruções de um dispositivo. Deve fornecer contratos temporais suficientemente expressivos para que a máquina modele causalidade, prazos e efeitos observáveis.

## Documento de avaliação arquitetural

A avaliação arquitetural do CHRONOS está em [CHRONOS_ARCHITECTURE_EVALUATION.md](CHRONOS_ARCHITECTURE_EVALUATION.md). A SWOT, análise de stakeholders, recomendações de mudança e roadmap estratégico estão em [CHRONOS_STRATEGIC_ASSESSMENT.md](CHRONOS_STRATEGIC_ASSESSMENT.md). O modelo híbrido mínimo permanece hipótese, não decisão aprovada.

## 2. Perguntas que precisam ser respondidas

### 2.1 Representação do tempo
- Qual unidade interna representa o tempo virtual?
- Inteiro de largura fixa, inteiro amplo, racional ou combinação?
- Como representar razões de clock não inteiras sem deriva acumulada?
- Como representar fase residual e conversões entre domínios?
- Como detectar e tratar overflow sem wrap silencioso?
- Quais garantias de precisão e intervalo são necessárias para sessões longas e save states?

### 2.2 Modelo de execução
Comparar formalmente:
- avanço por ciclos;
- avanço por eventos discretos;
- agenda híbrida de eventos e avanços em lote;
- scheduler central com dispositivos anunciando deadlines;
- modelo de quanta/interleave e seus limites.

Não escolher um modelo por familiaridade. A decisão deve refletir os requisitos reais do SMS e os custos medidos.

### 2.3 Ordenação e causalidade
- Como ordenar eventos com timestamp idêntico?
- Quais fases temporais são necessárias?
- Como definir prioridade entre CPU, barramento, IRQ e dispositivos?
- Como impedir causalidade retroativa?
- O que acontece quando um evento agenda outro no mesmo timestamp?
- Como evitar starvation e eventos de delta-cycle infinitos?

### 2.4 Operações suspensíveis
- Como representar operações de barramento pendentes?
- Como modelar WAIT states sem bloquear a thread?
- Como suspender e retomar uma transação sem perder fase ou efeitos parciais?
- Quais pontos são seguros para pause, reset e save state?

### 2.5 Estado, replay e persistência
- Qual estado é necessário para snapshot determinístico?
- Como serializar agenda, sequência, fase, deadlines e operações pendentes?
- Como validar restore e replay contra execução contínua?
- Como versionar snapshots sem acoplar o formato a detalhes internos instáveis?

### 2.6 Concorrência e integração
- Qual estado precisa de proprietário único?
- Quais cálculos podem ser paralelos sem introduzir sincronização cara?
- Como workers entregam resultados sem mudar o tempo virtual?
- Qual política de filas, backpressure e descarte é segura para cada tipo de dado?

## 3. Modelos a estudar

A pesquisa deve incluir documentação e código de projetos reconhecidos, com foco em:
- MAME: scheduler de dispositivos, timers, sincronização e interleave.
- Ares: arquitetura de sistemas, execução e sincronização.
- BlastEm: modelagem temporal do hardware Mega Drive/Genesis e sincronização.
- Gearsystem: Master System/Game Gear, timing e integração de dispositivos.
- Genesis Plus GX: sincronização de chips e execução de sistemas Sega.
- Documentação técnica primária do SMS/SG-1000 e testes de hardware relevantes.

Cada referência deve ser analisada no contexto em que foi construída. Não copiar uma arquitetura inteira de outro emulador sem demonstrar adequação ao SUN.

## 4. Entregáveis da pesquisa

1. Glossário temporal e unidades.
2. Catálogo de invariantes e propriedades obrigatórias.
3. Comparação dos modelos de scheduler.
4. Tabela de semântica de eventos simultâneos.
5. Modelo de clock domains e conversão de fases.
6. Modelo de transações suspensíveis e WAIT states.
7. Estratégia de snapshot, restore e replay.
8. Plano de concorrência e comunicação com workers.
9. Análise de riscos e falhas possíveis.
10. ADR com a decisão recomendada e alternativas rejeitadas.

## 5. Provas e testes exigidos antes da implementação

- Aritmética temporal e overflow.
- Conversão entre domínios e conservação de fase.
- Invariância por particionamento: avançar N unidades em um bloco deve equivaler a avançar em blocos menores, quando o contrato permitir.
- Ordenação determinística para timestamps iguais.
- Agendamento de eventos no limite atual e no futuro.
- Prevenção de causalidade retroativa.
- Pausa/retomada de operações pendentes.
- WAIT state e retomada sem duplicação de efeitos.
- Snapshot/restore com agenda não vazia.
- Replay determinístico.
- Carga com grande quantidade de eventos e domínios.
- Benchmarks de throughput, latência, alocações e memória.

Os testes devem começar como especificações executáveis ou modelos de referência independentes. Não devem simplesmente reproduzir o algoritmo que será implementado.

## 6. Benchmark e custo

Medir, no mínimo:
- custo por evento;
- custo por avanço;
- custo de inserir, cancelar e despachar eventos;
- custo por domínio de clock;
- alocações e bytes alocados;
- memória por evento e por domínio;
- p50, p95, p99 e pior caso;
- comportamento sob agenda densa e esparsa;
- custo de snapshot/restore;
- custo de tracing ligado e desligado.

Separar microbenchmarks de benchmarks integrados com a carga de trabalho SMS. Nenhum resultado sintético isolado será aceito como prova de ganho em gameplay.

## 7. Portões de aprovação

- **R0 — Pesquisa:** fontes e modelos estudados, com evidências registradas.
- **R1 — Especificação:** invariantes, semântica e contratos completos.
- **R2 — Modelo de referência:** testes independentes passam sem depender da implementação final.
- **R3 — Protótipos:** alternativas implementadas isoladamente e comparadas.
- **R4 — Revisão adversarial:** revisor independente procura bugs, ambiguidades e alternativas mais eficientes.
- **R5 — Aprovação:** decisão arquitetural registrada; somente então começa o crate de produção.

Falhar em qualquer portão exige corrigir a especificação ou o modelo antes de avançar.
