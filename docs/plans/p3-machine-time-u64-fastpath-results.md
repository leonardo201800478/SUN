# P3 — Fast path de `MachineTime` com preservação de `u128`

**Data:** 2026-11-23

**Plano:** [`temporal-core-optimization-plan.md`](temporal-core-optimization-plan.md), §4.5 e §7

**Base:** `origin/perf/p0-dispatch-allocation`, commit `7ac1450f1` (P0)

**Ambiente medido:** sandbox Linux x86_64, Rust 1.99.1, 6 vCPU; amostras fixadas na CPU permitida 0 quando indicado.

## Escopo da mudança

`MachineTime` continua armazenando e expondo `u128`. Em `MachineTime::checked_duration_since()`, quando ambos os timestamps cabem em `u64`, o caminho comum usa subtração checked de 64 bits. Se qualquer timestamp exceder esse intervalo, a implementação usa o `u128::checked_sub()` original. Subtração fora de ordem continua retornando `None`.

Não foi alterado `checked_add()`, `convert_elapsed()`, a conversão racional, o estado dos clocks, os codecs/snapshots nem o scheduler. `edge_time_after()` não possui consumidores no runtime atual, então não se ampliou a mudança para `checked_add()` sem workload que justificasse isso.

## Correção e integridade

Foi adicionado teste com `0`, `u64::MAX`, `u64::MAX + 1`, operandos acima de `u64`, `u128::MAX`, subtração válida e underflow. A comparação com o fallback preserva resultados inteiros e erros; o caminho de overflow amplo continua checked.

O assembly Release de `ClockDomain::preview_advance()` foi inspecionado. No P0, a comparação/subtração de timestamps era expandida como operação de dois limbs (`cmp`/`sbb` e `sub`/`sbb`) mesmo para timestamps pequenos. No candidato, o compilador emite uma checagem das metades altas, seguida por comparação/subtração de 64 bits quando ambas são zero; valores mais amplos seguem pelo caminho de 128 bits. Não há divisão ou alocação introduzida pelo fast path.

## Medições

### Scheduler pós-P0 — `caller_buffer`

O harness mede agendamento + `dispatch_next_into()` com tipos reais, 3 domínios SMS ou 4 domínios com YM2413 e contador de alocações. Para reduzir ruído, a comparação principal usou `--release`, 2 milhões de iterações medidas por execução, processo fixado na CPU 0 e seis pares em ordem ABBA (P3/P0 e P0/P3 alternados).

| Perfil | P0 mediana | P3 mediana | Variação P3 vs. P0 | Alocações |
|---|---:|---:|---:|---:|
| 3 domínios | 78,98 ns/op | 74,00 ns/op | **−6,3%** | 0 em ambos |
| 4 domínios | 103,83 ns/op | 97,81 ns/op | **−5,8%** | 0 em ambos |

Cinco dos seis pares favoreceram P3 no cenário de 4 domínios. A dispersão permaneceu visível, inclusive um outlier na medição P0 de 3 domínios; por isso estes valores são indicativos, não uma garantia universal. A comparação não fixada e com menos iterações foi inconclusiva, o que motivou a repetição maior e pinada.

### SMS headless — carga funcional com estado final idêntico

Foi executado o exemplo determinístico `native/sms_core/examples/dp0_trace_overhead.rs`, em `--release`, com processo fixado na CPU 0, ordem ABBA e seis pares. Cada variante forneceu 30 amostras por workload. A tabela mostra medianas em ns/instrução.

| Workload | Instruções / ticks por amostra | P0 | P3 | Variação |
|---|---:|---:|---:|---:|
| `relative-loop` | 149.340 / 5.376.240 | 1.948,586 | 1.907,551 | **−2,1%** |
| `bus-heavy` | 143.367 / 5.376.264 | 1.732,267 | 1.709,028 | **−1,3%** |

Contagem de instruções e ticks finais foram iguais entre P0 e P3 em todas as amostras. Essa medição é headless e sintética; não equivale a FPS de frontend nem substitui validação em Windows x64/ARM64.

### Probe aritmético isolado (diagnóstico, não critério principal)

Um probe temporário, não mantido no repositório, comparou o método candidato e `u128::checked_sub()` em seis amostras alternadas de 50 milhões de operações. Para entradas que cabem em `u64`, as medianas foram 0,908 vs. 1,003 ns/op (−9,5%); em entradas amplas que forçam o fallback, 1,068 vs. 0,976 ns/op (+9,4% mais lento). Isso confirma o custo do branch de classificação e o benefício local no caso comum, mas não é tomado como medida representativa do emulador.

## Validação executada

Passaram:

- `cargo fmt --all -- --check`;
- `cargo test -p sega-temporal-core` (42 testes);
- `cargo test -p sega-machine-scheduler` (suítes unitária e de integração; os dois testes de stress explicitamente ignorados permaneceram ignorados);
- `cargo test -p sega-sms-core` (todos os alvos/testes do crate, incluindo os workloads temporais);
- `cargo clippy -p sega-temporal-core -p sega-machine-scheduler -p sega-sms-core --all-targets -- -D warnings`.

Não foram medidos Windows x64 nem ARM64 neste ambiente. A alteração não introduz API nova ou comportamento de hardware; a validação SMS headless confirmou equivalência de instruções e tempo.

## Decisão

**O fast path restrito a `checked_duration_since()` é mantido como candidato P3**, pois as medições pinadas mostraram ganho pequeno no scheduler e nos dois workloads SMS, sem alocações e sem divergência temporal. Não generalizar a técnica a outras operações sem medir seus consumidores. A revisão de CI e code review da PR ainda são gates; não declarar P3 concluído até esses gates passarem e não fazer merge sem autorização explícita.
