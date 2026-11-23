# P1 — Compartilhar conversões equivalentes: avaliação e decisão

**Data:** 2026-11-23
**Plano:** [`temporal-core-optimization-plan.md`](temporal-core-optimization-plan.md), §4.2 e §7
**Base de comparação:** branch da PR #1261, commit `522d5e73f9f0ce3cd87240c848a9a00c90c7908f` (P0)
**Escopo:** medir o compartilhamento de previews no caminho de produção `dispatch_next_into`; não altera o runtime entregue.

## Hipótese e critério

Domínios com o mesmo alvo e estado de origem completo — razão, timestamp atual, ciclos acumulados e fase — podem reutilizar o `ClockDomainAdvance` calculado anteriormente. Cada ID, entretanto, precisa continuar recebendo seu próprio `ClockAdvance`, e o scheduler deve validar todos os domínios antes de qualquer commit.

O critério de aceite do plano exige equivalência exata **e ganho mensurável**. A equivalência não basta para aceitar uma otimização de desempenho.

## Método

Foi usado o binário `caller_buffer` já introduzido no trabalho P0. Cada iteração medida agenda um evento com `schedule_at_with_phase()` e executa `dispatch_next_into()`; portanto, a fronteira é **agendamento + dispatch**, não dispatch isolado. Baseline e candidato executaram a mesma carga, os mesmos 200.000 operações medidas, aquecimento de 2.000 operações, 3 ou 4 domínios SMS com razão 1/3, compilação `--release` e alocador contador. Foram feitas cinco execuções para cada variante; comparamos a mediana de ns/op.

- **Baseline:** checkout P0 em `522d5e7`, sem compartilhamento de previews.
- **Candidato:** protótipo que tenta reutilizar o preview do domínio imediatamente anterior somente quando o alvo e `validate_preview` confirmam identidade completa do estado de origem. O avanço continuou sendo emitido por domínio e a validação dupla-passada e o commit não foram alterados.
- **Ambiente:** sandbox Linux x86_64, 6 vCPU, Rust 1.99.1. Este ambiente não equivale ao Neoverse-N1 da medição original P0 nem ao Windows x64 alvo do frontend.
- Ambos os caminhos mediram **0,000 alocações e 0 bytes por operação**; logo, o buffer P0 foi mantido constante na comparação.

## Resultados

| Domínios equivalentes | P0 sem P1 — mediana (mín.–máx.), ns/op | P1 candidato — mediana (mín.–máx.), ns/op | Variação da mediana |
|---:|---:|---:|---:|
| 3 | 107,37 ns (79,36–109,65) | 114,33 ns (103,58–170,88) | **+6,5% mais lento** |
| 4 | 129,47 ns (101,42–136,34) | 147,98 ns (133,07–201,91) | **+14,3% mais lento** |

Os tempos apresentam variabilidade alta neste host compartilhado; isso reduz a força da estimativa absoluta. Ainda assim, as medianas e os menores tempos observados não demonstraram benefício do candidato. Uma variante exploratória que pesquisava todos os previews anteriores também ficou mais lenta e foi descartada antes do resultado final apresentado acima.

## Decisão

**P1 não é adotado no runtime neste momento.** No benchmark da operação combinada agendamento + dispatch, o reuso economiza chamadas de conversão, mas o trabalho de identificar um preview compatível não compensou a conversão. O protótipo foi removido da branch; a revisão final contém somente este registro documental e a atualização de status do plano.

Não relaxar `validate_preview`, não fundir estados dos relógios e não substituir o teste de desempenho por uma projeção teórica. Reabrir P1 somente com um harness pareado mais estável ou uma forma de identificar grupos equivalentes sem acrescentar custo por dispatch; a nova hipótese deve ser medida antes de entrar no runtime.

## Validação do protótipo

Enquanto o protótipo esteve presente, passaram `cargo fmt --all -- --check`, `cargo test -p sega-machine-scheduler`, `cargo test -p sega-temporal-core`, `cargo test -p sega-sms-core` e `cargo clippy -p sega-machine-scheduler -p sega-sms-core --all-targets -- -D warnings`. Esses resultados verificaram a correção do candidato, mas **não mudam o veredito de desempenho**. Como a mudança final não altera código executável, não se executaram testes de código para a revisão documental.
