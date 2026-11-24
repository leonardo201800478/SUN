# SUN V2

SUN V2 é uma reconstrução independente do emulador SUN, iniciada com o Master System (SMS).

## Estado

**Fase atual: Fase 0 — inventário crítico da V1 e fechamento de evidências R0.**

A direção recomendada para o CHRONOS está documentada como **híbrido mínimo**, ainda sujeito a evidências, modelo de referência, protótipos e revisão independente. Nenhum código de emulação foi portado ou implementado nesta etapa. A implementação de produção só começa após aprovação dos gates R0–R5.

## Princípios

- O código legado permanece intacto e fora da árvore de dependências do V2.
- O legado é uma fonte de consulta e comparação, nunca uma dependência de runtime ou de build.
- Código, testes, benchmarks e documentação do V2 nascem dentro desta pasta.
- O Temporal Core é a autoridade sobre o tempo virtual, os limites seguros de avanço e a coordenação causal.
- Cada dispositivo mantém a propriedade do próprio estado.
- A execução temporal deve ser determinística; paralelismo é seletivo e orientado por evidência.
- Otimizações devem provar equivalência funcional/temporal e ganho líquido no caminho real.
- O objetivo é executar jogos com compatibilidade, fluidez, baixa latência e áudio/vídeo estáveis.

## Decisões iniciais

- Primeiro sistema: Sega Master System.
- Estratégia de paralelismo: seletiva; não existe obrigação de uma thread por dispositivo ou por núcleo.
- Prioridade de produto: equilíbrio entre compatibilidade de gameplay, frame pacing, latência de entrada e qualidade/continuidade de áudio.
- Stack de host prevista: Slint 1.18.x, WGPU 30.x, Vulkan inicialmente no Windows x64 e CPAL 0.18.2. SDL3 permanece sujeito a auditoria.
- CHRONOS: proposta de híbrido mínimo com avanço limitado, sincronização sob demanda e agenda seletiva de deadlines autônomos. A proposta não é uma aprovação de implementação.

## Roadmap de desenvolvimento

- [Roadmap SUN V2](ROADMAP.md) — fases, prioridades, primeira fatia vertical mínima, critérios de validação e política de uso da V1.
- [ADR-0003 — fatia vertical mínima](docs/decisions/ADR-0003-minimal-vertical-slice.md) — decisão de construir CPU, barramento, VDP e PSG mínimos já integrados.

## Roadmap de desenvolvimento

- [Roadmap SUN V2](ROADMAP.md) — fases, prioridades, primeira fatia vertical mínima, critérios de validação e política de uso da V1.
- [ADR-0003 — fatia vertical mínima](docs/decisions/ADR-0003-minimal-vertical-slice.md) — decisão de integrar CPU, barramento, VDP e PSG desde o primeiro protótipo.
- [Inventário V1 → V2](docs/architecture/V1_TO_V2_INVENTORY.md) — conhecimento aproveitável, limites temporais e lacunas a fechar em R0.

## Documentos do Temporal Core

- [Proposta arquitetural do híbrido mínimo](docs/temporal-core/CHRONOS_MINIMAL_HYBRID_SPEC.md) — documento de referência para a direção recomendada, responsabilidades, mecanismos, invariantes, prototipagem e gates.
- [Avaliação arquitetural](docs/temporal-core/CHRONOS_ARCHITECTURE_EVALUATION.md) — comparação dos modelos e análise técnica.
- [Avaliação estratégica](docs/temporal-core/CHRONOS_STRATEGIC_ASSESSMENT.md) — SWOT, stakeholders, riscos e roadmap.
- [Plano de pesquisa e especificação](docs/temporal-core/RESEARCH_AND_DESIGN_PLAN.md) — perguntas, entregáveis, provas e gates.

## Estrutura

- `docs/architecture/`: princípios e arquitetura.
- `docs/temporal-core/`: pesquisa, especificação e decisões do núcleo temporal.
- `docs/quality/`: política de testes, revisão e desempenho.
- `crates/`: crates independentes do V2, criados após aprovação das especificações.
- `tests/`: testes exclusivos do V2.
- `benchmarks/`: benchmarks exclusivos do V2.

A estrutura poderá evoluir por ADR (Architecture Decision Record). Não criar abstrações ou crates antecipadamente sem necessidade demonstrada.

## Regra de independência

O workspace V2 não deve adicionar dependências de path para crates do legado. Se uma ideia for reutilizada, sua origem deve ser registrada, criticada e reimplementada com testes próprios.

## Primeiro marco

Concluir pesquisa, especificação e revisão adversarial do Temporal Core antes de escrever sua implementação. O próximo trabalho é concluir o inventário V1→V2 e fechar R0; em seguida serão definidos os contratos R1 e o modelo de referência R2 antes do primeiro protótipo integrado.
