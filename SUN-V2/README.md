# SUN V2

SUN V2 é uma reconstrução independente do emulador SUN, iniciada com o Sega Master System (SMS).

## Estado atual

**Fase atual: R1 — contratos temporais verificáveis, em elaboração e revisão.**

- **R0:** aprovado no escopo documental em 24/11/2026. Consulte o [parecer R0](docs/temporal-core/R0_CLOSEOUT.md) e a [issue #1267 encerrada](https://github.com/leonardo201800478/SUN/issues/1267).
- **R1:** rascunho v0.3.3 com unidades locais de progresso/horizonte explicitadas, ainda não aprovado. Consulte [R1_TEMPORAL_CONTRACTS.md](docs/temporal-core/R1_TEMPORAL_CONTRACTS.md) e acompanhe a [issue #1270](https://github.com/leonardo201800478/SUN/issues/1270).
- **R2–R5:** pendentes. R2 só começa após aprovação R1; R3 só começa após aprovação R1 e R2; R4 é a revisão adversarial; R5 é a aprovação arquitetural.
- **Produção:** nenhum código de produção CHRONOS/V2 está autorizado antes da aprovação R5.
- **PRs legadas:** PRs abertas anteriores ao V2 foram encerradas sem merge, preservando histórico e branches. O trabalho legado não será incorporado automaticamente à V2.

A baseline V1 foi congelada no commit `207ccc4cebbc4f8c0aed45c68262eae963491689` (23/11/2026). O inventário foi reconfirmado contra código e documentação desse SHA; drift documental, limites e incertezas estão registrados na matriz de evidências.

A direção recomendada para o CHRONOS está documentada como **híbrido mínimo**, ainda sujeita a contratos, modelo de referência, protótipos descartáveis, revisão independente e aprovação arquitetural. A proposta não é autorização de implementação.

O benchmark S0 da V1 foi encerrado como **não executado** e está fora dos gates do V2. Não procurar ROM OutRun, executar ou corrigir `perf-baseline-s0.ps1`, repetir medições ou criar benchmarks substitutos para preencher essa lacuna.

## Instruções permanentes

As regras oficiais de arquitetura, escopo, pesquisa, gates, Git/PR, CI, testes, agentes, desempenho e validação estão em [PROJECT_INSTRUCTIONS.md](PROJECT_INSTRUCTIONS.md). Em caso de conflito com instruções legadas da V1, prevalece a governança V2 aprovada mais recente.

## Princípios

- O código legado permanece intacto e fora da árvore de dependências do V2.
- A V1 é fonte auxiliar de consulta e comparação, nunca dependência de runtime ou build.
- Código, testes, benchmarks e documentação do V2 ficam dentro de `SUN-V2/`.
- O Temporal Core/CHRONOS é a autoridade do tempo virtual, dos limites seguros de avanço e da coordenação causal.
- Cada dispositivo mantém a propriedade do próprio estado.
- A execução temporal deve ser determinística; paralelismo é seletivo e orientado por evidência.
- Otimizações devem provar equivalência funcional/temporal e ganho líquido no caminho real.
- O objetivo é executar jogos com compatibilidade, fluidez, baixa latência e áudio/vídeo estáveis.

## Decisões iniciais

- Primeiro sistema: Sega Master System.
- SG-1000: segundo piloto de generalização, somente após SMS estável.
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
- [Fechamento R0](docs/temporal-core/R0_CLOSEOUT.md) — evidências, incertezas e limites da aprovação documental.
- [Contratos temporais R1](docs/temporal-core/R1_TEMPORAL_CONTRACTS.md) — rascunho v0.3.3, incluindo definições normativas de watermark, safe horizon, bloqueios, deadlines, microsteps, transações parciais, snapshot/restore, replay e proveniência; ainda não aprovado.

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
