# SUN V2 — progresso dos candidatos P-D/P-E (R3)

**Data:** 2026-11-28
**Baseline P-A:** congelada em `466f789bc9bfc73c7d6388f31b3f9142b48acf78`
**HEAD validado:** `9feec831eb3c99294d8077aedfda46a896306dc4`
**CI:** [#11145 / run 46107673712](https://github.com/leonardo201800478/SUN/actions/runs/46107673712) — SUCCESS.

## 1. Validação executável

- Suíte R3: **241/241 Linux e 241/241 Windows**.
- Emissor do corpus P-A: **184 testes por OS**.
- Oráculo R2 independente: **256/256 por OS**, mutation checks PASS.
- Equivalência dos corpora canônico e replayável: PASS, byte a byte entre Linux e Windows.
- Change-impact, workflow validation, documentation hygiene e Rust quality supervisor: PASS.
- Jobs de hardware/runtime/desktop/áudio/vídeo/Z80/workspace Rust não aplicáveis: SKIPPED, não PASS.

Os testes diferenciais comparam P-D/P-A em ordenação de claims W01/W02 e P-E/P-A em dependências/observações W03/W04. W06/W07 têm controles positivos/negativos em ambos os candidatos; UNKNOWN não cria ordem nem mutação.

## 2. Benchmark pareado

Artefatos do CI #11145:

- Linux: `chronos-r3-pd-pe-linux-46107673712`, 36 pontos, JSON de 28.388 bytes.
- Windows: `chronos-r3-pd-pe-windows-46107673712`, 36 pontos, JSON de 14.523 bytes.

Matriz:
- P-D: 3 cardinalidades de domínio × 3 cardinalidades de deadline × 2 densidades = **18 pontos**.
- P-E: 3 fan-ins × 3 profundidades de cadeia + 3 fan-ins × 3 cadências de observação = **18 pontos**.
- 5 warm-ups e 30 amostras por candidato/ponto.
- P-A e candidato executam a mesma fixture e sequência de operação; cada amostra verifica igualdade de saída antes de aceitar o tempo.
- p50/p95/p99/máximo, wall-time, CPU-time quando a resolução é adequada e probe separado de alocação.
- Fixtures, geração e serialização ficam fora da região cronometrada.

### 2.1 Resultado no Linux

| Candidato | Razão mediana dos p50 por ponto | Melhor ponto | Pior ponto |
|---|---:|---:|---:|
| P-D vs P-A | **6,44×** | 2,14× | **37,78×** |
| P-E vs P-A | **1,31×** | 0,89× | **12,83×** |

P-D:
- 8 deadlines: aproximadamente 2,17×.
- 32 deadlines: aproximadamente 4,6× sparse e 8,2× dense.
- 128 deadlines: aproximadamente 8,6–8,8× sparse e 37,6–37,8× dense.
- O crescimento denuncia o custo de validar/copiar o estado inteiro e localizar eventos repetidamente durante a drenagem, neutralizando a vantagem do heap.

P-E:
- Cadeias de dependência pequenas ficam próximas de P-A e alguns pontos ficam ligeiramente abaixo.
- Fan-in 16 com observações frequentes chega a 9,33×, 9,50× e 12,83× nos pontos de 1/8/32 observações.
- A implementação ainda reconstrói/valida estruturas extensas e varre transições a cada sincronização; o sync-on-demand não está explorando índices persistentes por domínio.

### 2.2 Resultado no Windows

| Candidato | Razão mediana dos p50 por ponto | Pior ponto |
|---|---:|---:|
| P-D vs P-A | **7,08×** | **39,17×** |
| P-E vs P-A | **1,38×** | **11,23×** |

CPU-time Windows foi `SKIPPED_COARSE_RESOLUTION`. Os números acima usam somente wall-time no mesmo runner Windows; não são comparação de CPU entre sistemas.

### 2.3 Interpretação e disposição

Todas as 36 saídas pareadas foram equivalentes (`equivalent_output=true`) nos dois runners. Isso valida a equivalência funcional da fatia medida, não a arquitetura completa.

**P-D e P-E não são selecionados nesta etapa.** A diferença de custo é suficientemente grande para bloquear qualquer alegação de ganho. Os resultados são OBSERVATION de protótipos Python sintéticos, não desempenho Rust, hardware SMS ou frame pacing.

## 3. Otimizações exigidas antes de repetir a matriz

### P-D — agenda central

1. Substituir o estado baseado em tuplas copiadas a cada claim por heap e índice de eventos próprios do candidato.
2. Separar validação estrutural completa (construção/restore) de verificações locais O(1)/O(log n) no hot path; não remover invariantes.
3. Eliminar buscas lineares por identidade e remoção linear do heap; usar tombstones/generation fencing e limpeza lazy.
4. Resolver o grupo simultâneo em preflight e só então confirmar a mutação; UNKNOWN/ciclo deve deixar o estado lógico inalterado.
5. Repetir diferencial P-D/P-A antes de medir.

### P-E — sync-on-demand

1. Indexar transições por owner e identidade, sem varrer todos os dispositivos a cada solicitação.
2. Manter índice de relações par-a-par reutilizável entre observações; invalidar apenas quando a topologia/declarações mudarem.
3. Separar validação estrutural de validação local da solicitação.
4. Calcular apenas o fecho causal solicitado e seus predecessores, sem incluir eventos futuros ou domínios não relacionados.
5. Preservar UNKNOWN, atomicidade e os controles W06/W07 durante a otimização.
6. Repetir diferencial P-E/P-A antes de medir.

O benchmark deve ser executado novamente no mesmo conjunto de parâmetros e nos dois runners. Melhorias de throughput não podem relaxar validação, causalidade ou replay.

## 4. Cobertura ainda ausente

- **P-D:** a fatia atual compara ordenação de claims W01/W02; ainda faltam contrato A(E) completo, contexto de participantes/safe horizons, watermark e arbitragem C<D/C=D/C>D (W05).
- **P-E:** a fatia atual cobre dependências/observações W03/W04; ainda faltam integração do ciclo de transação W08 e snapshot/restore/replay W09.
- Nenhum dos candidatos foi comparado em todos os fixtures W00–W09.
- W06 físico permanece UNKNOWN; W07 B/C permanece UNKNOWN. Não há medição física.
- R4/R5 continuam pendentes. Nenhum código de produção antes de R5.

**Próxima ação:** otimizar os caminhos de dados P-D/P-E sem alterar o contrato, adicionar W05 ao P-D, integrar W08/W09 por uma camada de estado independente dos mecanismos de scheduling e repetir os testes/benchmarks pareados.