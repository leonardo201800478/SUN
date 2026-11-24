# SUN V2 — Política de validação e revisão

## 1. Regra de qualidade

Compilar não significa concluir. Um componente só é aceito após testes proporcionais, revisão técnica, benchmark e evidência de que atende aos contratos.

## 2. Camadas de teste

- **Unitários:** aritmética, estruturas e invariantes locais.
- **Propriedade/property-based:** explorar combinações e limites além dos casos escritos manualmente.
- **Stress:** volume elevado, sessões longas e condições extremas.
- **Integração:** interação entre contratos e componentes.
- **Diferenciais:** comparar com modelo independente, referência documentada ou comportamento de hardware conhecido.
- **Replay/determinismo:** repetir a mesma sequência e comparar resultados.
- **Performance:** custo, alocação, memória e latência sob cargas representativas.

Testes de uma área não devem ser executados sem necessidade em alterações não relacionadas.

## 3. Protocolo de revisão adversarial

Cada componente terá:
1. **Autor/implementador:** apresenta hipótese, desenho, código e evidências.
2. **Revisor independente:** procura bugs, condições de corrida, overflow, estados inválidos, ambiguidades e casos não cobertos.
3. **Analista de desempenho:** verifica se o custo foi medido corretamente e se existe alternativa mais eficaz.

O revisor não deve apenas confirmar a intenção do autor. Deve tentar refutar a implementação, propor casos que falhem e comparar soluções equivalentes.

A disponibilidade de agentes externos será confirmada por tarefa. Quando não houver agente independente executável, a revisão deve ser explicitamente identificada como revisão assistida, sem alegar que um agente foi acionado.

## 4. Registro obrigatório por mudança

- Problema observado.
- Hipótese e comportamento esperado.
- Alternativas consideradas.
- Contrato/invariantes afetados.
- Testes adicionados e executados.
- Benchmark antes/depois e ambiente.
- Custo de CPU, memória, alocações e latência.
- Riscos e limitações restantes.
- Parecer do revisor.
- Decisão: aceitar, revisar ou descartar.

## 5. Política de performance

O custo só pode ser considerado aceitável em relação a uma carga de trabalho e a um orçamento explícitos. Não otimizar com base apenas em microbenchmark.

Comparar:
- caminho frio e quente quando relevante;
- carga leve, média e pesada;
- distribuição p50/p95/p99;
- throughput e latência;
- memória e alocações;
- impacto em áudio, vídeo e input.

Uma otimização será rejeitada se o ganho local provocar regressão global de frame pacing, áudio, latência ou determinismo.

## 6. Política de mudanças

- Uma hipótese principal por alteração.
- Não misturar refatoração e otimização sem necessidade.
- Preservar o baseline e os artefatos de teste.
- Revisar o diff integralmente.
- Executar fmt, clippy e testes apenas nos componentes afetados, além dos testes de integração necessários.
- Não fazer merge de alteração não validada.
