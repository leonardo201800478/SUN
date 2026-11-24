# SUN V2

SUN V2 é uma reconstrução independente do emulador SUN, iniciada com o Master System (SMS).

## Estado

**Fase atual: Fase 0 — inventário crítico da V1 e fechamento de evidências R0.**

A baseline V1 foi congelada no commit `207ccc4cebbc4f8c0aed45c68262eae963491689` (23/11/2026). O inventário foi reconfirmado contra código e documentação desse SHA; a auditoria encontrou drift documental e limites importantes que estão registrados na matriz de evidências.

A direção recomendada para o CHRONOS está documentada como **híbrido mínimo**, ainda sujeita a evidências, modelo de referência, protótipos e revisão independente. Nenhum código de emulação V2 foi implementado. A implementação de produção só começa após aprovação dos gates R0–R5.

O benchmark S0 da V1 foi encerrado como **não executado** e não é bloqueador do R0/V2. Não procurar ROM OutRun, executar ou corrigir `perf-baseline-s0.ps1`, repetir medições ou criar benchmarks substitutos para preencher essa lacuna.

## Princípios

- O código legado permanece intacto e fora da árvore de dependências do V2.
- O legado é fonte de consulta e comparação, nunca dependência de runtime ou build.
- Código, testes, benchmarks e documentação do V2 nascem dentro desta pasta.
- O Temporal Core é a autoridade sobre o tempo virtual, limites seguros de avanço e coordenação causal.
- Cada dispositivo mantém a propriedade do próprio estado.
- A execução temporal deve ser determinística; paralelismo é seletivo e orientado por evidência.
- Otimizações devem provar equivalência funcional/temporal e ganho líquido no caminho real.
- O objetivo é executar jogos com compatibilidade, fluidez, baixa latência e áudio/vídeo estáveis.

## Decisões iniciais

- Primeiro sistema: Sega Master System.
- Estratégia de paralelismo: seletiva; não existe obrigação de uma thread por dispositivo ou núcleo.
- Prioridade de produto: equilíbrio entre compatibilidade de gameplay, frame pacing, latência de entrada e qualidade/continuidade de áudio.
- Stack de host prevista: Slint 1.18.x, WGPU 30.x, Vulkan inicialmente no Windows x64 e CPAL 0.18.2. SDL3 permanece sujeito a auditoria.
- CHRONOS: proposta de híbrido mínimo com avanço limitado, sincronização sob demanda e agenda seletiva de deadlines autônomos. A proposta não é aprovação de implementação.

## Roadmap e decisões

- [Roadmap SUN V2](ROADMAP.md) — fases, prioridades, primeira fatia vertical mínima, critérios de validação e política de uso da V1.
- [ADR-0003 — fatia vertical mínima](docs/decisions/ADR-0003-minimal-vertical-slice.md) — decisão de integrar CPU, barramento, VDP e PSG desde o primeiro protótipo.
- [Inventário V1 → V2](docs/architecture/V1_TO_V2_INVENTORY.md) — inventário reconfirmado no SHA congelado.
- [Matriz de evidências da baseline V1](docs/architecture/V1_BASELINE_EVIDENCE_MATRIX.md) — fatos, fontes primárias, confiança, divergências, riscos e testes discriminantes.
- [Boundaries observáveis do VDP SMS](docs/architecture/VDP_OBSERVABLE_BOUNDARIES.md) — granularidade temporal, limites conhecidos, testes e prioridades por impacto em jogos.

## Documentos do Temporal Core

- [Proposta arquitetural do híbrido mínimo](docs/temporal-core/CHRONOS_MINIMAL_HYBRID_SPEC.md) — responsabilidades, mecanismos, invariantes, prototipagem e gates.
- [Avaliação arquitetural](docs/temporal-core/CHRONOS_ARCHITECTURE_EVALUATION.md) — comparação dos modelos e análise técnica.
- [Avaliação estratégica](docs/temporal-core/CHRONOS_STRATEGIC_ASSESSMENT.md) — SWOT, stakeholders, riscos e roadmap.
- [Plano de pesquisa e especificação](docs/temporal-core/RESEARCH_AND_DESIGN_PLAN.md) — perguntas, entregáveis, provas e gates.

## Estrutura

- `docs/architecture/`: princípios, inventário e arquitetura.
- `docs/temporal-core/`: pesquisa, especificação e decisões do núcleo temporal.
- `docs/quality/`: política de testes, revisão e desempenho.
- `crates/`: crates independentes do V2, criados após aprovação das especificações.
- `tests/`: testes exclusivos do V2.
- `benchmarks/`: benchmarks exclusivos do V2.

A estrutura poderá evoluir por ADR. Não criar abstrações ou crates antecipadamente sem necessidade demonstrada.

## Regra de independência

O workspace V2 não deve adicionar dependências de path para crates do legado. Se uma ideia for reutilizada, sua origem deve ser registrada, criticada e reimplementada com testes próprios.

## Próximo marco

Fechar R0 documentando fatos, inferências e incertezas remanescentes sobre clocks/fases, colisões temporais, WAIT e boundaries observáveis do VDP. Incertezas físicas sem evidência disponível devem ser formalmente limitadas, sem bloquear a arquitetura genérica. O benchmark S0 permanece não executado e fora dos gates. Em seguida, definir os contratos R1 e o modelo de referência R2 antes dos protótipos descartáveis R3.
