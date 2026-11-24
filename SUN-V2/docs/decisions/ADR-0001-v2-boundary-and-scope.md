# ADR-0001 — Isolamento e escopo inicial do SUN V2

- **Status:** Aceito
- **Data:** 2026-11-23
- **Escopo:** estrutura de desenvolvimento e sequência de reconstrução

## Contexto

O projeto SUN anterior acumulou implementação, testes e decisões que não devem limitar a arquitetura de uma reconstrução. O objetivo do V2 é recomeçar com especificações, testes, documentação e código próprios, usando o legado apenas como fonte de consulta crítica.

## Decisões

1. O V2 será desenvolvido exclusivamente dentro de `SUN-V2/`.
2. O código legado permanecerá preservado e não será dependência de build ou runtime do V2.
3. Testes, documentação, benchmarks e ferramentas do V2 serão criados do zero.
4. O primeiro sistema-alvo será Sega Master System.
5. A estratégia de paralelismo será seletiva, justificada por dependência e benchmark.
6. A prioridade de produto será equilibrada entre compatibilidade de gameplay, fluidez, latência de entrada e qualidade/continuidade audiovisual.
7. O primeiro componente reconstruído será o Temporal Core.
8. A integração de novos componentes ocorrerá um por vez, com especificação, testes, revisão e medição de custo.

## Consequências

- Nenhum crate legado será importado por path.
- Reutilização de conhecimento exige análise crítica e registro de origem.
- O estado anterior serve como referência comparativa, não como contrato.
- A expansão para outros sistemas fica bloqueada até o SMS estabelecer a arquitetura e os critérios de qualidade.

## Critério de revisão

Revisar esta decisão somente se houver evidência concreta de que o isolamento impede um objetivo essencial do V2 ou cria custo sem benefício arquitetural.
