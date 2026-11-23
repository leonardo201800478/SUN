# P2 — Evitar revalidação redundante: avaliação e decisão

**Data:** 2026-11-23
**Plano:** [`temporal-core-optimization-plan.md`](temporal-core-optimization-plan.md), §4.3 e §7
**Base de comparação:** `origin/perf/p0-dispatch-allocation`, commit `7ac1450f1` (P0)
**Escopo:** avaliar um lote de guards validados que mantém empréstimo mutável exclusivo sobre cada `ClockDomain` até todos os previews serem aprovados; não altera o runtime entregue.

## Hipótese e critério

`MachineScheduler::apply_advances()` valida todos os previews para assegurar atomicidade e depois `ClockDomain::apply_preview()` repete a validação individual. Um guard que conserva o empréstimo mutável entre as duas etapas poderia eliminar a segunda verificação sem oferecer uma operação unchecked nem permitir mutação entre preflight e commit.

O aceite de P2 exige preservar validação pública, falha sem mutação parcial e determinismo, além de ganho mensurável no caminho real. Equivalência funcional, isoladamente, não justifica regressão de throughput.

## Método

O benchmark foi `tools/temporal_bench` / binário `caller_buffer`, com a mesma fronteira pós-P0: 200.000 iterações medidas de agendamento mais `dispatch_next_into()`, aquecimento de 2.000 iterações, razão de relógio 1/3, 3 ou 4 domínios e compilação `--release`. O alocador contador ficou ativo. O candidato e um worktree limpo do P0 foram executados alternadamente na mesma máquina para reduzir o viés de carga; também foram feitas cinco execuções independentes por variante. Valores são ns/op.

A proposta candidata armazenou até quatro `ValidatedClockDomainAdvance` em slots inline; perfis com mais domínios conservaram o caminho anterior. Também se experimentaram dicas `#[inline]` nas operações pequenas do guard. A ordem canônica dos IDs foi verificada durante a validação. Falha em preview stale ou em ID não canônico descartava os guards antes de qualquer commit.

## Resultados

### Comparação alternada (três pares de execuções)

| Domínios | P0 mediana (mín.–máx.) | P2 guard mediana (mín.–máx.) | Diferença da mediana |
|---:|---:|---:|---:|
| 3 | 72,85 (72,52–74,23) | 127,40 (124,34–141,54) | **+74,9% mais lento** |
| 4 | 94,42 (94,41–101,69) | 158,65 (158,26–158,67) | **+68,0% mais lento** |

### Cinco execuções independentes e verificação de inlining

| Domínios | P0 mediana (mín.–máx.) | P2 mediana (mín.–máx.) | Diferença da mediana |
|---:|---:|---:|---:|
| 3 | 72,76 (70,64–81,35) | 184,10 (138,60–195,28) | **+153,0% mais lento** |
| 4 | 100,36 (92,14–113,90) | 210,93 (168,46–226,33) | **+110,2% mais lento** |

A maior dispersão nas execuções não pareadas recomenda cautela com números absolutos. Porém, a comparação alternada também mostra a regressão de forma consistente. A tentativa de `#[inline]` ainda mediu medianas de 125,65 ns/op (3 domínios) e 155,09 ns/op (4), portanto não explicou nem resolveu o custo adicional.

P0 e candidato registraram **0,000 alocações e 0 bytes por operação**. O candidato não trouxe redução de alocação, e o tempo adicionado é inaceitável para a otimização proposta.

## Validação funcional

Durante a avaliação, passaram:

- `cargo fmt --all -- --check`;
- `cargo test -p sega-temporal-core`;
- `cargo test -p sega-machine-scheduler`, incluindo equivalência de dispatch e replay multi-domínio;
- `cargo test -p sega-sms-core` (131 testes);
- `cargo clippy -p sega-temporal-core -p sega-machine-scheduler --all-targets -- -D warnings`.

Os testes do candidato cobriram commit exato do preview, descarte de guards sem mutação, preview stale em domínio posterior sem avanço parcial e IDs fora da ordem canônica. Esses resultados demonstram a segurança funcional do protótipo, mas não compensam a regressão de desempenho.

## Decisão

**O desenho P2 baseado em guards não é adotado e foi removido da implementação.** Apesar de manter preflight público e atomicidade, o custo dos guards/armazenamento/commit no caminho SMS superou a revalidação que pretendia eliminar. Nenhum runtime ou componente validado deve ser alterado com base nesta tentativa.

P2 permanece pendente. Só reabrir com uma hipótese de implementação substancialmente diferente, preservando validação e atomicidade e com medição pareada estável desde o início. Não relaxar a validação pública nem introduzir operação unchecked apenas para recuperar o benchmark. A branch não deve ser mergeada com o protótipo regressivo.
