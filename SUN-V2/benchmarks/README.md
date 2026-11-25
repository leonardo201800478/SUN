# Benchmarks do SUN V2

Benchmarks exclusivos do V2.

Toda medição deve registrar:
- commit e configuração de build;
- CPU, sistema operacional e toolchain;
- carga de trabalho e parâmetros;
- ROM e hash, quando aplicável;
- estado de instrumentação/tracing;
- aquecimento e número de repetições;
- throughput, latência, percentis, alocações e memória;
- limitações e fronteiras da medição.

Microbenchmarks não substituem benchmarks integrados de gameplay.


## R3 — contrato mínimo para medições comparativas

Todos os protótipos R3 devem ser isolados e descartáveis. Antes da execução, registre a semântica temporal e os invariantes que cada alternativa implementa; cenários e sequências de entrada precisam ser equivalentes entre P-A, P-D e P-E. P-F só pode ser incluído quando uma lacuna funcional mensurável exigir a composição híbrida.

Cada resultado deve registrar, além dos metadados gerais acima:
- SHA do protótipo e SHA do harness de benchmark;
- variante (P-A/P-D/P-E/P-F), parâmetros e flags de compilação;
- carga sintética identificada, densidade e distribuição de deadlines, sincronizações e cancelamentos;
- número de domínios, eventos, transações, snapshots e operações por amostra;
- warm-up, duração, repetições e estratégia de amostragem;
- throughput de operações e custo por classe;
- latência p50/p95/p99 e máximo observado;
- alocações por operação, bytes alocados, pico e memória retida;
- trace/hash de saída observável para comprovar equivalência funcional;
- limitações e fontes de ruído.

### Matriz de cargas obrigatória

1. Avanços esparsos sem deadlines próximos.
2. Agenda esparsa com deadlines autônomos distantes.
3. Agenda densa, deadlines agrupados e cancelamentos.
4. Sincronizações causais cross-domain com conjunto participante pequeno/grande.
5. Empates simultâneos e ordem parcial UNKNOWN — preservar a semântica R1/R2, sem desempate físico inventado.
6. Microsteps e ciclos no mesmo timestamp até o limite declarado.
7. Transações retomáveis, amostragem/commit e efeitos parciais.
8. Snapshot/restore/replay com estado pendente.
9. Escala de domínios e crescimento do conjunto de participantes.

Não misturar resultado de microbenchmark com gameplay, latência de host ou frame pacing. R3 mede custo de mecanismos temporais abstratos e registra o que ainda precisará de integração F6. Nenhuma variante se torna implementação de produção por vencer microbenchmark.
