# SUN — Plano de otimização do Temporal Core e subsistemas

- **Projeto:** SUN — Strife Uranus Neptune
- **Repositório:** https://github.com/leonardo201800478/SUN
- **Base:** auditoria de latência recebida em 23/11/2026 e revisão técnica do Item 1 — MachineScheduler / Temporal Core.
- **Status:** planejamento técnico. Nenhuma alteração de código é declarada como implementada.

## 1. Objetivo

Reduzir o custo dos caminhos críticos do SUN — MachineScheduler, barramentos, RAM/VRAM, VDP, PSG e YM2413 — preservando determinismo, ordem causal, M-cycles, T-states, WAIT states, interrupções, timestamps de barramento, fidelidade audiovisual e integridade de save states.

O MachineScheduler permanece como autoridade temporal única. Nenhum subsistema poderá avançar seu clock independentemente.

## 2. Regras obrigatórias

1. Preservar componentes validados; reabrir somente com evidência.
2. Não alterar semântica de hardware para melhorar benchmarks.
3. Não remover eventos, estados de espera ou validações sem prova de equivalência.
4. Separar microbenchmarks, throughput headless e desempenho do frontend.
5. Toda otimização deve ter baseline, hipótese mensurável, alteração isolada e comparação posterior.
6. Medir alocações, cópias e tempo nos caminhos realmente executados pelos jogos.
7. Executar apenas testes pertinentes ao subsistema alterado.
8. Não considerar compilação como critério suficiente de conclusão.
9. Não tratar ganhos projetados como ganhos comprovados.
10. Não fazer merge antes de revisão, formatação, testes e validação proporcionais.

## 3. Ordem da auditoria

| Ordem | Subsistema | Foco | Situação |
|---:|---|---|---|
| 1 | MachineScheduler / Temporal Core | Dispatch, conversões, tokens, alocações e validações | Revisado |
| 2 | Barramento e conclusão dos ciclos | BusCompletion, pending_request, clones Z80 e instante dos efeitos | Próxima revisão |
| 3 | RAM e VRAM | Leitura/escrita, mapeamento, cópias e sincronização | Pendente |
| 4 | VDP | Fronteiras temporais, sprites, framebuffer e publicação | Pendente |
| 5 | PSG SN76489 | Amostras, FIR, buffers e processamento por tick | Pendente |
| 6 | YM2413 | Clock, slots, alocações e integração com PSG | Pendente |

A ordem só deve mudar se medições demonstrarem prioridade superior para outro gargalo.

# 4. Item 1 — MachineScheduler / Temporal Core

## 4.1 Reduzir alocação por dispatch

**Diagnóstico confirmado:** `calculate_advances()` cria um `Vec::with_capacity(self.clocks.len())` por dispatch e `MachineDispatch` mantém `Vec<ClockAdvance>`.

Plano:
- [ ] Avaliar armazenamento inline para `ClockAdvance`, considerando a quantidade máxima de domínios suportada.
- [ ] Definir fallback ou erro explícito para capacidade excedida; nunca truncar silenciosamente.
- [ ] Preservar ordem determinística dos domínios.
- [ ] Medir alocações por frame e custo por dispatch.
- [ ] Usar o tamanho real de `ClockAdvance`, não estruturas menores do microbenchmark.

**Aceite:** redução comprovada de alocações no caminho de produção, sem mudança de sequência, estado ou resultado temporal.

## 4.2 Compartilhar conversões equivalentes

Plano:
- [ ] Identificar domínios com razão, tempo, ciclos acumulados e fase de origem idênticos.
- [ ] Reutilizar conversão somente entre domínios comprovadamente equivalentes.
- [ ] Manter caminho genérico quando qualquer estado diferir.
- [ ] Validar overflow e arredondamento.
- [ ] Comparar avanços ciclo a ciclo com o caminho original.

**Aceite:** equivalência exata e ganho mensurável no scheduler real.

## 4.3 Eliminar revalidação redundante sem perder atomicidade

**Diagnóstico confirmado:** `apply_advances()` valida os previews e `ClockDomain::apply_preview()` valida novamente.

Plano:
- [ ] Manter a validação da API pública.
- [ ] Investigar operação interna de aplicação após preflight completo.
- [ ] Garantir que todos os domínios sejam validados antes de qualquer mutação.
- [ ] Preservar atomicidade: falha em um domínio não pode deixar os anteriores parcialmente aplicados.
- [ ] Testar previews obsoletos, divergência de fase, overflow e múltiplos domínios.

**Aceite:** eliminar somente a validação redundante interna, preservando as garantias de falha e atomicidade.

## 4.4 Reavaliar BTreeMap

**Diagnóstico:** existem consultas repetidas aos domínios. A decomposição fornecida não contabiliza corretamente todas as consultas de produção.

Plano:
- [ ] Corrigir benchmark incluindo todos os `get` e `get_mut` realmente executados.
- [ ] Medir quantidade e densidade dos IDs.
- [ ] Verificar registro dinâmico e dependências de snapshots.
- [ ] Comparar alternativas após caracterizar essas restrições.
- [ ] Preservar domínios opcionais.

**Decisão:** não substituir `BTreeMap` por array antes da medição e da auditoria de compatibilidade.

## 4.5 Preservar MachineTime(u128)

**Decisão:** não migrar globalmente para `u64`.

Plano:
- [ ] Manter a representação pública `u128`.
- [ ] Investigar fast paths quando operandos couberem comprovadamente em `u64`.
- [ ] Manter fallback checked para valores maiores.
- [ ] Validar overflow, conversões e snapshots.
- [ ] Medir em Windows x64 e ARM64, quando disponível.

**Aceite:** ganho mensurável sem reduzir o domínio representável nem alterar resultados.

## 4.6 FastScheduler experimental

O protótipo de aproximadamente 9,3 ns/evento não equivale ao scheduler de produção: utiliza uma lane, razão fixa, `u64` e modelo simplificado.

- [ ] Preservar como referência experimental.
- [ ] Não substituir o scheduler real com base nessa comparação.
- [ ] Não usar a projeção de aproximadamente 17x como expectativa de ganho do SUN.

## 4.7 Corrigir a metodologia de benchmark

A decomposição de aproximadamente 158,5 ns/evento mistura fronteiras de medição e não representa fielmente o custo real.

- [ ] Reproduzir as operações de produção na mesma ordem.
- [ ] Usar o tipo real `ClockAdvance`.
- [ ] Contabilizar todas as consultas e validações.
- [ ] Separar agendamento, dispatch, tracing e instrumentação.
- [ ] Documentar o que cada benchmark inclui e exclui.

Os valores de 137 ns/evento e 158,5 ns/evento não devem ser comparados diretamente sem alinhar suas fronteiras de medição.

## 4.8 Eventos M1 — alteração bloqueada

O volume de 49.780 dispatches/frame para a ROM sintética `jr $` é coerente com a decomposição do relatório, mas não é universal para jogos.

**Decisão:** não colapsar as quatro transições M1 em um único evento neste estágio.

Antes de qualquer mudança, demonstrar preservação de:
- Amostragem do opcode.
- Conclusão do ciclo de barramento.
- WAIT states.
- Interrupções.
- Ordenação de eventos no mesmo timestamp.
- Estados suspensos e retomada.
- Resultados diferenciais em jogos reais.

# 5. Item 2 — Barramento e conclusão dos ciclos

**Status:** revisão técnica ainda não concluída. Os pontos abaixo são investigação, não diagnósticos confirmados.

### Pontos de auditoria
- [ ] `dispatch_cpu_event_inner()` e evento `BusCompletion`.
- [ ] Ciclo de vida de `pending_request`.
- [ ] `complete_bus_cycle()` e `finish_m1_fetch()`.
- [ ] Clonagens do estado Z80 e custo de `export_state()`.
- [ ] `trace_data` e caminhos condicionados a profiling.
- [ ] Instante exato em que leituras/escritas produzem efeitos em RAM, VRAM, portas e dispositivos.
- [ ] Tratamento de WAIT states e operações suspensíveis/retomáveis.

### Diretrizes
- Distinguir clones sempre executados daqueles condicionados a profiling/tracing.
- Não remover clonagem antes de entender sua função transacional.
- Não mover efeitos de leitura/escrita para outro timestamp sem prova temporal.
- Preservar causalidade entre CPU, barramento e dispositivo.
- Comparar custo real de `BusCompletion` em jogos e ROMs pertinentes.

**Aceite:** reduzir custo sem alterar timestamps, valores lidos/escritos, WAIT states ou sequência observável pelo jogo.

# 6. Itens 3 a 6 — investigação planejada

As otimizações específicas serão definidas após a revisão individual. Não antecipar mudanças sem evidência.

## 6.1 RAM e VRAM
- [ ] Mapear caminhos de leitura e escrita.
- [ ] Identificar indireções, verificações e cópias redundantes.
- [ ] Verificar mapeadores e janelas de memória.
- [ ] Preservar efeitos no timestamp correto.
- [ ] Validar acessos da CPU e interações com VDP.

## 6.2 VDP
- [ ] Medir avanço e processamento por fronteira.
- [ ] Investigar construção de fronteiras dentro de loops.
- [ ] Medir avaliação de sprites e alocações.
- [ ] Medir cópia/publicação de framebuffer e stride RGB.
- [ ] Avaliar troca de buffers e upload sem alterar o resultado visual.
- [ ] Preservar IRQs, H/V timing, scroll, sprites e comportamento de frame.

## 6.3 PSG SN76489
- [ ] Medir geração e drenagem de amostras.
- [ ] Investigar FIR e índice circular (`% taps`).
- [ ] Avaliar ring buffer ou máscara somente se o tamanho permitir.
- [ ] Evitar trabalho redundante sem pular transições.
- [ ] Preservar histórico, interpolação, fase e saída PCM.

## 6.4 YM2413
- [ ] Auditar o caminho efetivamente usado pelo runtime SMS.
- [ ] Separar o caminho `in_place` de benchmarks de clone transacional.
- [ ] Medir processamento por slot e operações repetidas.
- [ ] Investigar integração com drenagem PSG e `pending_psg_samples`.
- [ ] Preservar ordem dos operadores, clocks, envelopes, LFO, feedback e saída.
- [ ] Validar FM ativo, FM mutado mas clockado e ausência do chip.

# 7. Priorização preliminar

| Prioridade | Ação | Condição para iniciar |
|---|---|---|
| P0 | Remover alocação por dispatch | Armazenamento inline e capacidade definidos |
| P1 | Compartilhar conversões equivalentes | **Avaliado em 2026-11-23; não adotado por regressão medida.** Ver [resultados P1](p1-equivalent-conversion-results.md); reabrir somente com nova hipótese que elimine o custo de identificação por dispatch. |
| P2 | Evitar revalidação redundante | Preflight e atomicidade demonstrados |
| P3 | Fast path aritmético mantendo `u128` | Limites e overflow cobertos |
| P4 | Reavaliar BTreeMap e tracing | Benchmark corrigido e perfil real |
| P5 | Otimizações do barramento | Item 2 concluído e timestamp causal documentado |
| P6 | RAM/VRAM, VDP, PSG e YM2413 | Auditoria individual concluída |

Não implementar várias otimizações simultaneamente: cada alteração deve permitir atribuir ganho ou regressão à mudança correspondente.

# 8. Estratégia de validação

## Antes de cada alteração
- [ ] Registrar commit-base e configuração de build.
- [ ] Registrar ROM, região, revisão e SHA-256.
- [ ] Capturar baseline do benchmark pertinente.
- [ ] Definir hipótese, métrica e risco semântico.
- [ ] Identificar testes diretamente relacionados.

## Depois de cada alteração
- [ ] `cargo fmt --check`.
- [ ] `cargo clippy` nos crates afetados, conforme aplicável.
- [ ] Testes unitários e de integração pertinentes.
- [ ] Testes temporais/diferenciais do subsistema.
- [ ] Benchmark A/B com a mesma configuração.
- [ ] Comparação de estado, frame e áudio quando pertinente.
- [ ] Verificação de regressão em jogos reais.
- [ ] Revisão do diff antes de abrir/atualizar PR.

## Métricas
- Tempo por dispatch e por frame.
- Throughput de emulação.
- p50, p95 e p99.
- Alocações por frame.
- Bytes copiados por frame.
- Custo de CPU por subsistema.
- Underruns/overruns de áudio.
- Integridade temporal e determinismo.

Não confundir throughput headless com FPS apresentado pelo frontend. Medir separadamente Scheduler, WGPU, Slint e CPAL.

# 9. Critérios globais de conclusão

- [ ] Todos os itens da auditoria revisados individualmente.
- [ ] Cada otimização aprovada possui baseline reproduzível.
- [ ] Nenhuma alteração quebra causalidade ou determinismo.
- [ ] M-cycles, T-states, WAIT states e interrupções permanecem corretos.
- [ ] RAM/VRAM e efeitos de barramento ocorrem nos timestamps corretos.
- [ ] VDP mantém comportamento visual e temporal.
- [ ] PSG/YM2413 mantêm continuidade e equivalência de áudio.
- [ ] Nenhuma regressão funcional em jogos reais.
- [ ] Ganhos medidos no caminho de produção, não apenas em microbenchmarks.
- [ ] Frontend integrado validado quando a alteração afetar interfaces ou cadência de frames.
- [ ] Relatório final registra ganhos, custos, limitações e itens descartados.

# 10. Fora de escopo

- Colapsar eventos M1 sem prova diferencial.
- Migrar globalmente `MachineTime` de `u128` para `u64`.
- Substituir MachineScheduler pelo protótipo FastScheduler.
- Alterar timestamps de barramento para reduzir dispatches.
- Remover WAIT states ou validações necessárias.
- Reescrever subsistemas validados sem evidência.
- Aplicar SIMD em operações dependentes de sequência temporal.
- Aceitar otimizações que alterem áudio ou vídeo de forma não equivalente.

# 11. Estado desta versão

Este documento consolida as decisões já apresentadas para o Item 1 e organiza os próximos itens. O Item 2 e os subsistemas seguintes ainda precisam de revisão técnica antes de receber recomendações definitivas.

**Próximo passo:** concluir a revisão do barramento e dos ciclos de memória; depois, avançar para RAM/VRAM.
