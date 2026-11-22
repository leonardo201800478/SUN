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

No cenário C, `audio_prepare` p99 médio foi 1,316 ms, com média de 0,091 ms; esse outlier não muda a conclusão do custo médio. O custo de preparo/submissão do áudio não explica a queda sustentada de throughput. Importante: a métrica `ym2413_advance_ns` é cronometrada ao redor de `SmsTemporalBus::advance_ym2413` e inclui todo `advance_ym2413_inner` — loop de slots, chamadas ao core/serviço e captura de saída quando habilitada. Ela não isola somente o tempo interno de `Ym2413Core`.

## Interpretação

1. **ThinLTO é vencedor para este host/carga.** O ganho de 11,23% no cenário B e 11,32% no cenário C é consistente entre as duas repetições. O desvio-padrão do cenário C ThinLTO foi 0,0668 FPS.
2. **O mixer FM não é o gargalo dominante neste benchmark.** C ficou 0,2027 FPS abaixo de B em ThinLTO (aprox. -0,36%). O tempo médio de `audio_prepare` foi 0,091 ms e `audio_submit` aproximadamente 0,001 ms. Não há justificativa para reescrever o mixer com base neste resultado.
3. **O caminho temporal de avanço YM2413 é o principal alvo medido.** Em ThinLTO, `SmsTemporalBus::advance_ym2413` consome aproximadamente 5,35–5,41 ms por frame na janela diagnóstica; essa medição inclui o loop por slots, o core/serviço e a captura de saída, não sendo um perfil isolado do core. O processamento total em B/C é aproximadamente 19,16 ms. A ativação do caminho FM eleva o processamento médio em cerca de 8,30 ms em relação a A na janela diagnóstica; os 5,35 ms medidos no caminho de avanço são uma parcela importante, mas não explicam sozinhos toda a diferença. O restante não deve ser atribuído a uma função específica sem profiling por função.
4. **O resultado não atende aos gates de desempenho.** O cenário C alcançou 56,7438 FPS de throughput headless, abaixo de 60 FPS em tempo real e muito abaixo da meta PERF-01 de 120 frames/s uncapped/headless com FM ativo.
5. O diagnóstico registra que `Ym2413Core::advance_input_clocks_with_pg_service_in_place` é o caminho integrado. A autoridade temporal deve permanecer no MachineScheduler/barramento; não criar thread de clock FM independente.

## Próxima ação técnica — PERF-01 transversal

O YM2413 é um hotspot relevante nesta configuração, mas não define sozinho a estratégia de desempenho do SUN. A próxima etapa deve construir um perfil comparável do caminho completo de emulação e do frontend, priorizando pelo custo agregado e pela frequência real de execução, não por preferência por um componente.

### Matriz de profiling

| Domínio | O que medir | Exemplos de candidatos (não defeitos presumidos) |
|---|---|---|
| Z80 | custo por instrução/M-cycle, dispatch, fetch/decode, WAIT/IRQ e tempo agregado por frame | dispatch de opcode/prefixo, acesso ao barramento e caminhos quentes |
| MachineScheduler e barramentos | custo por evento, conversões de clock, dispatch, sincronização e tempo de espera | trabalho repetido, chamadas indiretas e eventos sem efeito observável |
| VDP e vídeo | tempo de renderização, atualização de registradores, framebuffer, cópias, conversão RGB/RGBA e upload WGPU | alocações/cópias por frame e publicação de textura |
| PSG e YM2413 | custo de avanço por frame/slot, geração de amostras e integração temporal | custo de síntese e trabalho repetido; preservar serialidade e causalidade |
| Audio runtime | preparo/mixagem, resampler, buffers, conversões, FIFO e callback CPAL | alocações por bloco, cópias, underruns/overruns e ocupação da fila |
| Memória e dados | alocações, clones, cópias, acessos e pressão de cache nos hot paths | buffers temporários e estruturas recriadas a cada frame |
| Frontend Slint/WGPU | frame time de apresentação, conversão/upload, sincronização e espera da GPU | cópias CPU→GPU, bloqueios e composição |
| Integração completa | tempo de frame end-to-end, pacing, estabilidade e interação áudio/vídeo | contenção e custos que não aparecem no benchmark headless |

### Sequência de trabalho

1. Preservar o ABBA Release × ThinLTO como evidência de ganho de compilação geral. `release-llvm` continua opt-in até os gates globais; não alterar o perfil padrão por causa de uma única ROM.
2. Consolidar a instrumentação existente (`step_frame_elapsed_ns`, `frame_processing_elapsed_ns`, `ym2413_advance_elapsed_ns`, `audio_prepare_elapsed_ns`, `audio_submit_elapsed_ns` e `video_publish_elapsed_ns`) e identificar os trechos ainda sem atribuição.
3. Produzir baseline por workload: OutRun, Kenseiden e Phantasy Star, com ROM/configuração/SHA registrados. Comparar execução headless sem pacing com a execução integrada Slint/WGPU/CPAL; não misturar as métricas.
4. Usar ETW/WPA ou profiling equivalente para atribuição por função nos componentes dominantes. Instrumentação diagnóstica e throughput principal devem permanecer em campanhas separadas.
5. Ordenar hotspots por custo total estimado por frame (tempo médio × frequência), p95/p99 e impacto nos jogos. Otimizar primeiro o maior custo comprovado, seja Z80, Scheduler, VDP, áudio, memória ou host.
6. Para cada otimização, registrar hipótese, custo antes/depois, ganho em jogo real, testes diferenciais e regressões. Preservar estados, M-cycles/T-states, WAIT/IRQ, ordem causal, fidelidade audiovisual e determinismo.
7. Só avaliar PGO/SIMD explícito quando a matriz identificar código adequado. SIMD deve ter fallback escalar e equivalência testada; não vetorizar sequências temporais dependentes.

### Metas e gates que não devem ser confundidos

- **Throughput do runtime:** meta PERF-01 de pelo menos 120 frames/s uncapped/headless em OutRun com FM ativo.
- **Produto integrado:** manter 60 FPS estáveis, com orçamento de 16,67 ms por frame, sem underruns recorrentes nem degradação de áudio/vídeo.
- **Correção:** nenhuma otimização pode alterar semântica de hardware, ordem temporal, resultados de registradores ou determinismo para ganhar FPS.

O resultado atual de 56,7438 FPS em C é uma referência inicial para o runtime headless, não um diagnóstico global nem evidência de que o YM2413 deva ser o único próximo alvo.

O perfil `release-llvm` permanece candidato opt-in. Não alterar o perfil padrão `release` até avaliação da matriz completa. Nenhum novo teste local Windows será solicitado nesta fase; as alterações devem ser validadas com testes focados e CI, e limitações de medição física devem ser declaradas.

## Limitações

Esta campanha representa somente o Ryzen 5 5600, Windows 11 Pro, Rust 1.99.1, o commit e a ROM identificados acima. O harness desativa pacing e apresentação WGPU e usa sink de áudio determinístico. Portanto, não mede FPS da GUI, CPAL físico, underruns do dispositivo, latência acústica, nem valida fidelidade de saída analógica. A amostra ABBA tem duas observações por perfil; é suficiente para uma comparação inicial estável nesta máquina, não para generalização a todo hardware.
