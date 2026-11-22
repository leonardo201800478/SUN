# PERF-01/S1 — Resultado ABBA ThinLTO (2026-11-22)

## Identificação da campanha

- Repositório: `leonardo201800478/SUN`
- Commit medido: `331fc7e97bf67964b0db6ad84ed7e27d171705d3`
- Branch: `perf/perf-01-s1-abba-runner`
- Manifesto: `abba-manifest.json`
- CPU: AMD Ryzen 5 5600 6-Core Processor
- SO: Windows 11 Pro
- Toolchain: Rust/Cargo 1.99.1
- Target: `x86_64-pc-windows-msvc`
- ROM: `OutRun (World).sms`
- SHA-256: `069c130135d8b61393d89f26b233509d6f9de76b0fbac53087e698a7fda0ba88`
- Ordem: A1 Release → B1 ThinLTO → B2 ThinLTO → A2 Release
- Cada captura: 5 s warmup + 30 s throughput + 5 s diagnóstico separado
- Campanha completa: 4/4 execuções, 12 observações, todos os testes individuais com exit code 0 e working tree limpo no manifesto.

## Resultados de throughput

FPS de cada captura:

| Cenário | A1 Release | B1 ThinLTO | B2 ThinLTO | A2 Release |
|---|---:|---:|---:|---:|
| A — sem YM2413 | 82,3285 | 95,2573 | 95,5541 | 83,0233 |
| B — YM2413 clockado, FM mudo | 50,9991 | 57,2936 | 56,5994 | 51,3930 |
| C — YM2413 clockado e FM mixado | 51,3914 | 56,7910 | 56,6966 | 50,5588 |

Resumo das duas amostras por perfil:

| Cenário | Perfil | Média FPS | Desvio-padrão amostral | Mín. | Máx. |
|---|---|---:|---:|---:|---:|
| A | Release | 82,6759 | 0,4913 | 82,3285 | 83,0233 |
| A | ThinLTO | 95,4057 | 0,2099 | 95,2573 | 95,5541 |
| B | Release | 51,1960 | 0,2785 | 50,9991 | 51,3930 |
| B | ThinLTO | 56,9465 | 0,4909 | 56,5994 | 57,2936 |
| C | Release | 50,9751 | 0,5887 | 50,5588 | 51,3914 |
| C | ThinLTO | 56,7438 | 0,0668 | 56,6966 | 56,7910 |

Ganho médio ThinLTO sobre Release:
- A: +15,40%.
- B: +11,23%.
- C: +11,32%.

O log B1 registra explicitamente `-C opt-level=3 -C lto=thin -C codegen-units=16`; B2 reutilizou o mesmo target/profile já compilado. A1/A2 foram compilados no perfil `release`. A configuração do perfil padrão não foi alterada.

## Diagnóstico separado — ThinLTO

Os valores abaixo são médias das duas janelas diagnósticas de cinco segundos. A instrumentação não está incluída nas capturas de throughput.

| Métrica | A — sem YM2413 | B — FM mudo | C — FM mixado |
|---|---:|---:|---:|
| frame interval médio | 10,860 ms | 19,167 ms | 19,167 ms |
| frame interval p50 | 10,646 ms | 18,415 ms | 18,400 ms |
| frame interval p95 | 12,737 ms | 21,399 ms | 21,530 ms |
| frame processing médio | 10,858 ms | 19,163 ms | 19,162 ms |
| YM2413 advance médio | 0 | 5,353 ms | 5,412 ms |
| YM2413 advance p50 | 0 | 5,052 ms | 5,119 ms |
| YM2413 advance p95 | 0 | 6,978 ms | 6,987 ms |
| audio prepare médio | 0,001 ms | 0,001 ms | 0,091 ms |
| audio submit médio | 0,001 ms | 0,001 ms | 0,001 ms |

No cenário C, `audio_prepare` p99 médio foi 1,316 ms, com média de 0,091 ms; esse outlier não muda a conclusão do custo médio. O custo de preparo/submissão do áudio não explica a queda sustentada de throughput.

## Interpretação

1. **ThinLTO é vencedor para este host/carga.** O ganho de 11,23% no cenário B e 11,32% no cenário C é consistente entre as duas repetições. O desvio-padrão do cenário C ThinLTO foi 0,0668 FPS.
2. **O mixer FM não é o gargalo dominante neste benchmark.** C ficou 0,2027 FPS abaixo de B em ThinLTO (aprox. -0,36%). O tempo médio de `audio_prepare` foi 0,091 ms e `audio_submit` aproximadamente 0,001 ms. Não há justificativa para reescrever o mixer com base neste resultado.
3. **O custo de avanço YM2413 é o principal alvo medido.** Em ThinLTO, o avanço do chip consome aproximadamente 5,35–5,41 ms por frame na janela diagnóstica. O processamento total em B/C é aproximadamente 19,16 ms. A ativação do caminho FM eleva o processamento médio em cerca de 8,30 ms em relação a A na janela diagnóstica; os 5,35 ms medidos no avanço do chip são uma parcela importante, mas não explicam sozinhos toda a diferença. O restante não deve ser atribuído a uma função específica sem profiling por função.
4. **O resultado não atende aos gates de desempenho.** O cenário C alcançou 56,7438 FPS de throughput headless, abaixo de 60 FPS em tempo real e muito abaixo da meta PERF-01 de 120 frames/s uncapped/headless com FM ativo.
5. O diagnóstico registra que `Ym2413Core::advance_input_clocks_with_pg_service_in_place` é o caminho integrado. A autoridade temporal deve permanecer no MachineScheduler/barramento; não criar thread de clock FM independente.

## Próxima ação técnica

- Manter `release-llvm` como perfil candidato opt-in; não trocar o perfil padrão `release` ainda.
- Investigar o caminho serial YM2413 por função, começando por `SmsTemporalBus::advance_ym2413_inner`, `Ym2413Core::clock_one_slot_with_service`, `Ym2413PgSerialService::before_eg_update` e `after_eg_update`.
- Examinar o trabalho repetido de dispatch/decodificação por slot e o custo do pipeline PG/operador/EG. Qualquer remoção de cálculo ou cópia deve preservar a ordem serial e os testes diferenciais.
- Não mexer no mixer, não adicionar SIMD/fast-math e não alterar a causalidade temporal sem evidência.
- Nenhum novo teste local Windows será solicitado nesta fase. Validar alterações por testes focados e CI; declarar limitações onde a medição física não puder ser repetida.

## Limitações

Esta campanha representa somente o Ryzen 5 5600, Windows 11 Pro, Rust 1.99.1, o commit e a ROM identificados acima. O harness desativa pacing e apresentação WGPU e usa sink de áudio determinístico. Portanto, não mede FPS da GUI, CPAL físico, underruns do dispositivo, latência acústica, nem valida fidelidade de saída analógica. A amostra ABBA tem duas observações por perfil; é suficiente para uma comparação inicial estável nesta máquina, não para generalização a todo hardware.
