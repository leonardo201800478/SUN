# DP-0 — auditoria inicial de diagnóstico e observabilidade

**Estado:** auditoria parcial; não autoriza declarar DP-0 concluído.  
**Baseline auditada:** `main` em `a58edc27dc6c07df019987d904a49c18fb972172`.  
**Data:** 2026-11-20.  
**Escopo desta revisão:** host desktop Slint, worker SMS, infraestrutura compartilhada de observabilidade e caminho legado egui identificado no workspace.

## 1. Resultado executivo

A regra de consentimento opt-in ainda **não está aplicada a todos os caminhos do desktop Slint**.

Foram encontrados emissores diretos para stderr no host ativo, inclusive relatórios periódicos de telemetria. Esses caminhos contornam qualquer filtro de `tracing` e não dependem de consentimento. O worker SMS também emite mensagens diretas durante resolução automática de região.

O crate `sega-observability` tem uma arquitetura útil de fila limitada, rotação e escrita em thread dedicada, mas a API pública `init()` cria diretório/arquivo, instala subscriber e instala panic hook no momento da inicialização. O contrato seguro depende hoje do chamador só invocá-la após opt-in; isso não é garantido pela API em si.

**Conclusão:** DP-0 permanece ABERTO. Esta revisão identifica defeitos concretos e delimita correções; não afirma que todos os sinks do repositório foram encontrados nem que a política já foi corrigida.

## 2. Achados confirmados

| ID | Severidade | Arquivo/linhas na baseline | Achado | Consequência |
|---|---|---|---|---|
| DP0-001 | Bloqueante | `native/sega_desktop/src/main.rs:2249-2354` | `HostFramePacingWindow::report_if_due` imprime um relatório extenso `R2_FRAME_PACING_HOST` via `eprintln!` após janelas de um segundo. | Telemetria de host é emitida sem ação ou consentimento explícito. |
| DP0-002 | Bloqueante | `native/sega_desktop/src/main.rs:2386-2418` | Saúde do áudio CPAL (`R3_AUDIO_HEALTH`) é impressa diretamente via `eprintln!`. | Métricas de áudio são registradas automaticamente em stderr. |
| DP0-003 | Bloqueante | `native/sega_desktop/src/main.rs:2670-2690` | Timer de diagnóstico é iniciado incondicionalmente e chama `report_if_due`; consulta métricas de worker/áudio e QPC. | A coleta/sampling do host continua ativa mesmo sem trace habilitado. |
| DP0-004 | Bloqueante | `native/sega_desktop/src/main.rs:223-368` | Falhas e eventos SDL3 (poll thread, inicialização, polling, NMI e atualizações de controle) usam `eprintln!`. | Mensagens automáticas contornam qualquer consentimento global. |
| DP0-005 | Bloqueante | `native/sega_desktop/src/main.rs:1746-1840`, `2653` | Eventos de lifecycle WGPU, importação de textura, teardown e notificação de frame usam `eprintln!`. | Diagnóstico de renderer é gravado diretamente no console sem opt-in. |
| DP0-006 | Bloqueante | `native/sega_desktop/src/main.rs:3167-3195`, `3327-3352` | Configuração e falhas de CPAL em SMS/SG-1000 são reportadas diretamente via stderr. | Eventos de áudio e configuração do host são registrados automaticamente. |
| DP0-007 | Alta | `native/sega_desktop/src/main.rs:1603-1605`, `3236-3276` | Falha ao salvar ROM recente e decisões/perfis SRAM são impressos diretamente. | Erros operacionais e metadados de diagnóstico não usam um canal de status consentido. A saída SRAM inclui hashes da ROM. |
| DP0-008 | Alta | `rust/runtime/sms-runtime/src/worker.rs:1210-1220`, `1249-1259` | Resolução automática de região e fallback do catálogo usam `eprintln!` em dois caminhos de construção de máquina. | Uma operação normal de carregamento produz saída de diagnóstico independente da UI. |
| DP0-009 | Alta | `native/observability/src/runtime.rs:39-49, 255-300` | `LogConfig::default()` seleciona filtro `info`; `init()` cria diretório/arquivo, inicia writer/subscriber e instala panic hook. | A garantia opt-in depende do chamador e não é invariante da própria API. |
| DP0-010 | Média | `native/observability/src/runtime.rs:343-365` | Panic hook personalizado só grava arquivo quando o flag está ativo, mas sempre chama o hook anterior. | Depois da instalação, panic ainda pode produzir saída padrão no stderr mesmo após desativação da coleta. Exige decisão explícita de política; não suprimir o mecanismo padrão de falha sem ADR. |
| DP0-011 | Média — classificação pendente | `native/sms_core/src/temporal_bus.rs:206,307,811-818,852,879-887` | `SmsTemporalBus` mantém uma fila de até 64 escritas de porta, acrescentada em toda escrita e incluída em export/restore do estado do barramento. A busca atual encontrou consumidores somente em testes. | Não há saída externa nem consumidor de produto identificado, mas o campo é explicitamente chamado de trace diagnóstico e persiste como estado. Decidir se é histórico semântico de barramento ou observabilidade; não remover nem alterar snapshots sem análise de compatibilidade. |
| DP0-012 | Média — limite de API | `native/sms_core/src/temporal_machine.rs:285-306,711-768,807-860` | `SmsTemporalMachine::trace_enabled` inicia falso e a captura só ocorre após habilitação explícita, mas o vetor interno `trace_events` não possui limite próprio. O worker de produto drena para um coletor limitado; consumidores diretos do core podem reter eventos indefinidamente enquanto habilitados. | A captura é opt-in, porém o contrato público do core não garante limite de memória. Avaliar limite/drop policy no core sem perder eventos necessários aos testes temporais.
| DP0-013 | Alta | `rust/runtime/sms-runtime/src/worker.rs:619-630,674-688,1941-1944` | `record_input_detected` e `record_frame_completed` atualizam FPS, frame time e latência input→frame sem verificar `diagnostics_enabled`; `performance_metrics()` podia expor esses dados mesmo com coleta desativada. | Métricas de desempenho do worker eram amostradas continuamente e o caminho de conclusão de frame adquiria mutex para atualizar dados diagnósticos sem consentimento. A PR #1088 condiciona coleta/leitura ao opt-in e limpa o histórico em transições de consentimento. |

As linhas acima são relativas ao SHA de baseline indicado; devem ser atualizadas quando os arquivos mudarem.

## 3. Caminhos que já têm uma fronteira explícita

### 3.1 Trace SMS

O host Slint expõe `trace-toggle-requested` e encaminha a ação a `DesktopSession::set_sms_trace_enabled`; o worker SMS também recebe `set_sms_diagnostics_enabled`. A exportação verifica `snapshot.trace_enabled` e abre diálogo de destino antes de gravar.

A captura de trace é, portanto, iniciada por ação explícita no menu DEBUG e a exportação é uma segunda ação explícita. O histórico é drenado no worker e limitado conforme o contrato do debugger compartilhado. Isso não autoriza que métricas host/audio paralelas sejam coletadas automaticamente.

### 3.2 Snapshot visível na UI

`debug-snapshot-requested` chama `debug_summary()` após ação explícita e apresenta o resultado no status da interface. Não foi identificado, nesse callback, arquivo ou persistência automática.

### 3.3 Estado da sessão SMS

O worker SMS oferece controle separado para trace e telemetria opcional. O host não deve inferir consentimento para logs de arquivo/console a partir do simples fato de o usuário habilitar um trace de barramento. São finalidades e volumes distintos; a UI deve explicar e autorizar cada finalidade.

### 2.1 Controles de captura já existentes

A auditoria também confirmou fronteiras positivas que devem ser preservadas:

- `SmsTemporalMachine` inicializa `trace_enabled = false`; os eventos temporais são acrescentados somente quando explicitamente habilitado.
- O worker SMS cria o `DiagnosticCollector` como `None`; o caminho de trace do worker o habilita por comando e drena eventos para um coletor limitado.
- `WorkerTelemetry` começa desabilitado e os métodos de amostragem verificam o flag antes de atualizar as métricas.
- O snapshot resumido de CPU é solicitado por callback explícito da UI; a exportação do trace exige captura ativa e uma segunda ação de escolha de arquivo.

Essas garantias são específicas dos caminhos verificados. Não compensam as emissões diretas do host Slint listadas acima.

## 4. Infraestrutura compartilhada: avaliação

A implementação em `native/observability` possui pontos positivos:

- fila assíncrona limitada e política lossy;
- writer em thread separada;
- rotação e retenção configuráveis;
- ring buffer limitado;
- contador de eventos descartados;
- filtro recarregável e flag atômica para panic log.

Lacunas que precisam de correção:

1. A API de inicialização não é fail-closed: `init()` tem efeitos de filesystem e instala subscriber antes de um estado consentido explícito.
2. O filtro inicial padrão é `info`, não `off`.
3. O estado do consentimento não está vinculado a um objeto de autorização/finalidade passado ao initializer.
4. A desativação do filtro não cancela eventos já enfileirados; o comportamento deve ser documentado e testado contra a exigência de revogação imediata.
5. O panic hook mantém o hook anterior ativo após instalação; sua saída padrão precisa ser tratada separadamente de arquivo de crash.
6. `write_diagnostics_json()` é uma operação de escrita explícita, mas a API não exige token/consentimento nem distingue exportação iniciada pelo usuário de snapshot automático no encerramento.

Não se recomenda criar um segundo logger. A correção deve preservar a infraestrutura existente, tornar o início fail-closed e encaminhar todos os emissores pelo mesmo contrato de consentimento.

## 5. Legado e escopo de produto

`native/sega_app` contém o host egui legado e um fluxo de logging que é iniciado a partir de `set_diagnostics_enabled(true)`. O host não é o destino final de produto segundo a arquitetura atual. Deve permanecer inventariado até que consumidores/builds/workflows confirmem que não é distribuído nem executado; não se deve gastar a migração R5 reescrevendo esse frontend.

Mesmo assim, o crate compartilhado `sega-observability` deve ter semântica segura independentemente do host legado.

## 5.1 Atualização de correção em andamento — PR #1088

A PR [#1088](https://github.com/leonardo201800478/sega-emulator/pull/1088) implementa a primeira fatia de correção sobre a baseline:

- O host Slint passa a usar um gate atômico, inicialmente falso, para suas emissões diagnósticas diretas.
- O painel DEBUG expõe autorização explícita, limitada à sessão e não persistida.
- A amostragem host de frame pacing, conversão e fila de áudio só registra métricas quando a autorização está ativa.
- As métricas SMS de frame time/FPS/latência input→frame também ficam inativas sem consentimento; o histórico é reiniciado em cada transição de autorização e há teste de revogação.
- A telemetria detalhada do worker SMS é vinculada ao consentimento de diagnóstico; foi separada da ação independente de captura de trace. Novas sessões herdam o estado de consentimento vigente.
- Os quatro caminhos de resolução automática de região do worker SMS deixam de escrever diretamente em stderr e passam a emitir eventos estruturados `tracing`.
- Três sinks de erro de persistência no host egui legado (autosave de preferências, salvamento no encerramento e ROMs recentes) passam a emitir eventos `tracing`; sem subscriber opt-in, não são coletados.

Limites observados na PR:
- O timer Slint ainda executa a verificação operacional de fallback do renderer; a parte de amostragem diagnóstica é condicionada ao consentimento. A separação do polling operacional e da telemetria permanece candidata a refinamento, caso a medição demonstre trabalho desnecessário.
- A falha ao gravar o snapshot JSON no encerramento do host egui ainda usa stderr, mas só é alcançada quando o usuário já habilitou diagnóstico e a exportação final falha.
- A saída de panic padrão continua deliberadamente preservada; não foi confundida com o arquivo de crash opt-in.
- O worker SMS emite eventos `tracing`, mas o host Slint não instala subscriber de arquivo; no host Slint esses eventos não são atualmente encaminhados para um destino visível. Isso deve ser resolvido ou explicitamente classificado antes de declarar a experiência de diagnóstico completa.
- PR ainda em validação CI e não mesclada.

## 5.2 Classificação preliminar das demais saídas encontradas

| Local | Ocorrência | Classificação inicial | Ação |
|---|---|---|---|
| `native/sega_app/src/main.rs`, `native/sms_debugger/src/main.rs`, `native/sega_desktop/src/main.rs` | Mensagem em `main()` de build stub não-Windows | Mensagem operacional de binário não suportado, não telemetria de sessão | Manter clara e sem dados privados; confirmar se esses stubs são distribuídos |
| `native/sms_core/src/bin/z80test_runner.rs` | Uso, progresso e erros do runner | Saída explícita de CLI | Manter, documentar que a execução foi solicitada pelo usuário |
| `native/ym2413/examples/*trace*.rs`, `native/sn76489/examples/psg_trace.rs`, `native/sms_core/examples/*trace*.rs` | Trace/relatórios de ferramentas de desenvolvimento | Artefatos explícitos de CLI | Confirmar caminhos de saída e ausência de invocação automática em Release |
| `native/machine_scheduler/tests/stress_replay.rs` e testes de core | `eprintln!`/WAV de teste | Saída de testes e artefatos de validação | Manter confinados a testes; não reutilizar como logging de runtime |
| `native/sn76489/src/psg.rs` e `native/sn76489/src/psg_h2_model.rs` | WAV/JSON condicionais a `PSG_AUDIO_ARTIFACT_DIR` dentro do módulo `#[cfg(test)]` | Artefatos explícitos de validação de áudio | Manter restritos aos testes; confirmar que CI só define o diretório de artefatos nos jobs de validação |
| `native/observability/src/runtime.rs` | Log rotativo, ring buffer, crash log e snapshot JSON | Infraestrutura de diagnóstico | Só iniciar após consentimento; exportação deve continuar ação explícita |
| `native/save_runtime/src/lib.rs` | Escrita atômica de SRAM/save data | Persistência funcional de dados do usuário | Não classificar como telemetria; preservar integridade e semântica |
| `native/sega_app/src/app.rs`, `native/sega_desktop/src/main.rs` | Preferências e histórico recente | Persistência funcional de preferências | Não classificar como log; revisar apenas mensagens de falha |
| `native/sms_debugger/src/app.rs` | Exportação de relatório via diálogo/caminho selecionado | Exportação explícita iniciada pelo usuário | Preservar confirmação e destino explícito |

A busca textual é uma triagem e não prova cobertura completa de macros indiretas, crates de terceiros ou sinks de plataforma. A classificação acima deve ser validada por leitura de cada callsite e pelos workflows que invocam os exemplos.

## 7. Plano de correção ordenado

1. **DP0-F1 — parar coleta host incondicional:** remover o timer de telemetria automática ou torná-lo inerte até consentimento específico; separar health checks necessários à operação (mensagens de status na UI) de amostragem diagnóstica.
2. **DP0-F2 — eliminar stdout/stderr diagnóstico do desktop:** substituir `eprintln!` por resultados/status na UI para falhas operacionais e pelo canal de diagnóstico opt-in para eventos diagnósticos. Nenhum log direto pode permanecer no caminho normal.
3. **DP0-F3 — corrigir worker SMS:** retirar `eprintln!` dos caminhos de resolução de região; retornar metadados ao chamador ou usar canal opt-in sem afetar a resolução da máquina.
4. **DP0-F4 — tornar observabilidade fail-closed:** configuração inicial `off`; inicialização sem criar arquivos nem instalar writer/hook até autorização explícita; consentimento separado para logs e captura detalhada; revogação interrompe novos eventos e trata a fila pendente conforme contrato definido.
5. **DP0-F5 — revisar panic/crash:** especificar o comportamento do hook sem coletar payload/backtrace automaticamente. Não suprimir o hook padrão sem decisão explícita e teste.
6. **DP0-F6 — auditoria completa de sinks:** buscar macros `println!`/`eprintln!`, `tracing`, `log`, writers, arquivos, snapshots, CSV/trace, métricas e instrumentação de terceiros em todos os crates, binários, examples, tests, scripts e workflows. Classificar cada ocorrência como teste/tooling, ação explícita do usuário, status operacional na UI ou coleta diagnóstica.
7. **DP0-F7 — validação:** testes em Debug e Release demonstrando ausência de arquivos/saídas diagnósticas sem consentimento, consentimentos independentes, revogação, fila cheia sem bloqueio, nenhuma amostragem periódica desnecessária e ausência de regressão no Scheduler/áudio/vídeo.

## 8. Limites desta auditoria

Esta é uma revisão estática inicial da baseline GitHub. Não houve checkout/execução local, instrumentação de stderr, inspeção de binários empacotados, nem execução de testes. A busca de código retornou ocorrências e trechos relevantes, mas não substitui a varredura exaustiva de todo o repositório.

Não se declara:
- DP-0 concluído;
- inexistência de outros sinks;
- overhead zero;
- comportamento de todas as bibliotecas de terceiros;
- aprovação de CI ou validação física.

Cada correção deverá ser feita em commits pequenos, com testes focados e SHA próprio. Nenhuma alteração no MachineScheduler, CPU, VDP, PSG, barramentos ou temporização é necessária para cumprir esta política.
