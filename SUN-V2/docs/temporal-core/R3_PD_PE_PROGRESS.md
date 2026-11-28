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

---

## Current status — W08 differential, P-D/P-E paired results (CI #11195)

**Validated HEAD:** `43afa850d713ec12593d0ba83a35205deadc8cc2`.
**CI #11195:** [run 46151720301](https://github.com/leonardo201800478/SUN/actions/runs/46151720301) — **SUCCESS**.

This section supersedes earlier status/count/benchmark statements in this document where they differ.

### 1. Executable validation

- R3 disposable suite: **255/255 PASS Linux and 255/255 PASS Windows**.
- P-A canonical/replayable corpus emitter: **184/184 PASS per OS**.
- Canonical corpus: **119 fixtures / 1,483 cuts** (1,472 differential + 11 property); digest `ab5e146eda37e8292159cc5440beca615c5b511383e3807b24adbacbb490056b`.
- Replayable corpus: **25 fixtures / 134 operations-cuts**; two deliberate divergences retained (W06 and W07); digest `6189653bb3aa18fc5805f7dd1ae9708ea31b349d03da7ea9fcc906350cbcfbb6`.
- Canonical and replayable corpus artifacts: **byte-identical Linux/Windows**.
- R2 independent reference: **256/256 PASS Linux and 256/256 PASS Windows**; deliberate mutation checks PASS on both.
- Change-impact classification, workflow validation, documentation hygiene, corpus equivalence and Rust quality supervisor: PASS.
- Hardware/runtime/desktop/subsystem Rust jobs: **SKIPPED**, not PASS. The Rust quality job is a supervisor result; it does not mean Cargo build, Clippy or Rust subsystem tests ran.

### 2. W08 shared transaction validation

P-D and P-E now carry the shared immutable `TransactionBook` in their candidate states. P-D uses `field(default_factory=TransactionBook)`; P-E also uses an independent factory. The benchmark metadata was corrected to distinguish state integration from timing coverage.

The differential helper compares the complete canonical transaction records and transaction book, including descriptor, generation, lifecycle state, progress, duration ledger, automatic/external TW ledgers, wait source, ready marker, sampled values, committed effects, failure and recovery count. It also compares result code, returned transaction and value.

W08 shared differential cases PASS on Linux and Windows, including:
- Identical submission retry and divergent descriptor reuse.
- Start/idempotency and state transition.
- External, fixture and dependency WAIT as distinct sources; wrong-source release is rejected without changing the waiting transaction.
- Sample exactly-once and first-value retention on retry.
- Commit exactly-once.
- Independent base duration, automatic TW and external TW budgets.
- Cancellation before irreversible effects and rejection after an irreversible sample.
- Failure after partial commit, preserved effect prefix and explicit recovery.
- Completion only after markers and all budgets are satisfied.

The new `test_w08_all_wait_sources_are_differentially_distinct` passed on both OS runners. This validates the tested logical transaction lifecycle, not a physical bus timing model.

**W09 limitation:** snapshot/restore/replay is still not implemented for P-D/P-E state. W09 currently validates P-A against R2; it must not be reported as P-D/P-E snapshot equivalence.

### 3. Paired performance — synthetic Python only

Artifacts from CI #11195:
- Linux: `chronos-r3-pd-pe-linux-46151720301`, 36 points.
- Windows: `chronos-r3-pd-pe-windows-46151720301`, 36 points.

Each OS run has 18 P-D and 18 P-E points, 5 warm-ups and 30 measured repetitions per candidate/point. Fixture construction and JSON serialization are outside the timed region. The table reports the **median across points of candidate/P-A ratios** for each quantile; values below 1.0 indicate a lower candidate time in that quantile summary.

| Candidate | OS | p50 ratio | p95 ratio | p99 ratio | Worst per-point p50 ratio |
|---|---|---:|---:|---:|---:|
| P-D | Linux | 0.755× | 0.726× | 0.738× | **1.367×** |
| P-D | Windows | 0.775× | 0.825× | 0.849× | **1.292×** |
| P-E | Linux | 0.839× | 0.772× | 0.497× | **1.129×** |
| P-E | Windows | 0.901× | 0.860× | 0.480× | **1.174×** |

All **36/36 paired outputs per OS** were equivalent.

Worst p50 cases:
- P-D Linux: 16 domains, 128 deadlines, dense.
- P-D Windows: 4 domains, 128 deadlines, dense.
- P-E on both systems: dependency chain, fan-in 1, depth 16.

These are per-point ratios, not a claim that every workload is faster. The P-D worst cases remain slower than P-A; P-E also retains a smaller worst-case regression. p95/p99 ratios are medians of per-point ratios and should not be read as one aggregate latency distribution.

Linux CPU-time was measured. Windows CPU-time remains `SKIPPED_COARSE_RESOLUTION` (clock resolution 15,625,000 ns), so there is no cross-platform CPU-time comparison. Wall-time comparisons are valid only within each runner and run.

These results are **OBSERVATION** for synthetic Python prototypes only. They are not native Rust performance, SMS hardware timing, frontend frame pacing or a 60 FPS claim.

### 4. Coverage boundaries and remaining work

- **P-D:** local tests cover W05 contextual publication, participants/safe horizons, watermark closure and cancellation arbitration. The current P-D/P-A differential benchmark/test path still covers W01/W02 claim ordering, not the complete W05 contract. The paired timing matrix also measures W01/W02 only.
- **P-E:** P-E/P-A differential tests cover W03/W04. W06/W07 have conformance tests and synthetic positive controls, but no physical phase or B/C relation has been established.
- **W08:** shared transaction lifecycle is represented in P-D/P-E state and differentially tested; the paired timing matrix does not time W08 operations.
- **W09:** P-D/P-E snapshot/restore/replay remains unimplemented; current W09 replayable fixtures compare P-A with R2.
- The benchmark matrix does not cover all W00–W09 operations in P-D/P-E.
- W06 physical phase remains **UNKNOWN**. W07 physical B/C remains **UNKNOWN**; the accepted R2/P-A divergence remains explicit and replayable, not physical evidence.
- No formal GitHub review approval is recorded for the current HEAD. Independent adversarial review of the W08 integration and updated P-D/P-E coverage remains required.
- R3 remains **IN PROGRESS**. R4 and R5 are not approved. No production CHRONOS/V2 implementation is authorized before R5.

**Next technical work:** extend P-D/P-A differential coverage to W05; implement P-D/P-E snapshot/restore/replay for W09 against the immutable reference; review transaction-generation and resource-boundary assumptions; then repeat cross-platform differential validation and request an independent adversarial review.


### 5. Adversarial implementation findings — open assumptions, not physical conclusions

- **Identity/generation (FACT):** `TransactionBook` lookup is keyed by stable `identity`. An identical descriptor retry (including the same generation) is idempotent; reusing the same identity with any divergent descriptor, including another generation, returns `InvalidInput`. The current model does not implement same-identity generation supersession. P-A and the R2 transaction oracle use the same identity-unique rule. Whether a bus transaction needs generation replacement under one stable identity remains **UNKNOWN** and must be resolved from the R1/R2 contract before changing it.
- **Resource bound (FACT):** W08 `TransactionBook` currently has no explicit maximum transaction count/capacity budget. P-D's event capacity and microstep limit do not bound transaction-book growth; P-E also has no transaction-book capacity. Resource exhaustion/eviction policy is therefore **NOT MODELED** in this W08 slice, not implicitly unlimited hardware capacity.
- **Integration boundary (FACT):** P-D/P-E scheduling operations preserve the immutable transaction book when reconstructing `AgendaState`/`SyncState`; `apply_transaction` updates the shared ledger. However, transaction lifecycle operations are not yet causally coupled to agenda claims, P-E observed transitions or a modeled bus completion pipeline. This is a shared logical transaction-state integration, not end-to-end bus integration.
- **Invalid operations (OBSERVATION):** tested invalid retries and wrong-source WAIT releases return the original transaction book and preserve the complete transaction record. The immutable dataclasses/tuple ledgers prevent in-place mutation. This does not replace adversarial tests for arbitrary forged books or resource exhaustion.
- **Snapshot/replay:** P-D/P-E have no canonical snapshot artifact for the shared transaction book. A raw dataclass copy or `replace` is not considered snapshot/restore validation. W09 remains open for these candidates.
- **Physical semantics:** automatic TW, external TW, fixture WAIT and dependency WAIT are logical categories in this prototype. The differential tests establish candidate/oracle consistency for the tested cases, not the exact SMS bus timing or the physical provenance of any WAIT.

These items remain review findings/assumptions for R4. They are not reasons to invent hardware semantics or to mark W08/W09 physical fidelity as PASS.
