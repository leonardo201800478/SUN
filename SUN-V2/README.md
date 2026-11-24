# SUN V2

SUN V2 é uma reconstrução independente do emulador SUN, iniciada com o Master System (SMS).

## Estado

**Fase atual: fundação arquitetural e pesquisa do Temporal Core.**

Nenhum código de emulação foi portado ou implementado nesta etapa. A implementação só começa depois da especificação, dos modelos concorrentes, dos critérios de correção e da estratégia de benchmark serem revisados e aprovados.

## Princípios

- O código legado permanece intacto e fora da árvore de dependências do V2.
- O legado é uma fonte de consulta e comparação, nunca uma dependência de runtime ou de build.
- Código, testes, benchmarks e documentação do V2 nascem dentro desta pasta.
- O Temporal Core é o primeiro componente e a autoridade sobre o tempo virtual da máquina.
- A execução temporal deve ser determinística; paralelismo é seletivo e orientado por evidência.
- Otimizações devem provar equivalência funcional/temporal e ganho líquido no caminho real.
- O objetivo é executar jogos com compatibilidade, fluidez, baixa latência e áudio/vídeo estáveis.

## Decisões iniciais

- Primeiro sistema: Sega Master System.
- Estratégia de paralelismo: seletiva; não existe obrigação de uma thread por dispositivo ou por núcleo.
- Prioridade de produto: equilíbrio entre compatibilidade de gameplay, frame pacing, latência de entrada e qualidade/continuidade de áudio.
- Stack de host prevista: Slint 1.18.x, WGPU 30.x, Vulkan inicialmente no Windows x64 e CPAL 0.18.2. SDL3 permanece sujeito a auditoria.

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

Concluir pesquisa, especificação e revisão adversarial do Temporal Core antes de escrever sua implementação.

A avaliação arquitetural do **CHRONOS — SUN Temporal Core** está em [docs/temporal-core/CHRONOS_ARCHITECTURE_EVALUATION.md](docs/temporal-core/CHRONOS_ARCHITECTURE_EVALUATION.md). A análise SWOT, stakeholders e roadmap estão em [docs/temporal-core/CHRONOS_STRATEGIC_ASSESSMENT.md](docs/temporal-core/CHRONOS_STRATEGIC_ASSESSMENT.md). A hipótese atual é um híbrido mínimo, ainda não aprovado como arquitetura final.
