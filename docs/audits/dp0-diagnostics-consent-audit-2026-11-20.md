# DP-0 — auditoria inicial de diagnóstico e observabilidade

**Estado:** auditoria parcial; não autoriza declarar DP-0 concluído.
**Baseline auditada originalmente:** `main` em `a58edc27dc6c07df019987d904a49c18fb972172`.
**Reavaliação integrada:** `main` em `cbcbdc531ba32ef933161349c2e860b96df0e95c` (20/11/2026), registrada na seção 9.
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
| DP0-013 | Alta | `rust/runtime/sms-runtime/src/worker.rs:619-630,674-688,1941-1944` | `record_input_detected` e `record_frame_completed` atualizam FPS, frame time e latência input→frame sem verificar `diagnostics_enabled`; `performance_metrics()` podia expor esses dados mesmo com coleta desativada. | Métricas de desempenho do worker eram amostradas continuamente e o caminho de conclusão de frame adquiria mutex para atualizar dados diagnósticos sem consentimento. A PR #1091 condiciona coleta/leitura ao opt-in e limpa o histórico em transições de consentimento. |

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

## 5.1 Atualização histórica de correção — PR #1091

A PR [#1091](https://github.com/leonardo201800478/sega-emulator/pull/1091), sucessora da tentativa #1088, integrou a primeira fatia de correção sobre a baseline:

- O host Slint inicializa a infraestrutura compartilhada `sega-observability` somente após autorização explícita no painel DEBUG; sem consentimento não instala subscriber, writer, arquivo de log nem panic hook.
- A autorização é limitada à sessão e não é persistida.
- As emissões diagnósticas do host Slint passam por um macro lazy que emite eventos estruturados `tracing`; não há mais saída direta de stderr do host em operação normal.
- Ao autorizar, o subscriber compartilhado captura eventos estruturados do host e dos crates de hardware no diretório de logs configurado; o snapshot JSON final só é gravado se o usuário mantiver a autorização ativa.
- A amostragem host de frame pacing, conversão e fila de áudio só registra métricas quando a autorização está ativa.
- As métricas SMS de frame time/FPS/latência input→frame e frame pacing também ficam inativas sem consentimento; o histórico é reiniciado em cada transição de autorização e há teste de revogação.
- A telemetria detalhada do worker SMS é vinculada ao consentimento de diagnóstico; foi separada da ação independente de captura de trace. Novas sessões herdam o estado de consentimento vigente.
- Os quatro caminhos de resolução automática de região do worker SMS deixam de escrever diretamente em stderr e passam a emitir eventos estruturados `tracing`, capturados pelo subscriber Slint após consentimento.
- Três sinks de erro de persistência no host egui legado (autosave de preferências, salvamento no encerramento e ROMs recentes) passam a emitir eventos `tracing`; sem subscriber opt-in, não são coletados.

Limites observados na PR:
- O timer Slint ainda executa a verificação operacional de fallback do renderer; a parte de amostragem diagnóstica é condicionada ao consentimento. A separação do polling operacional e da telemetria permanece candidata a refinamento, caso a medição demonstre trabalho desnecessário.
- A falha ao gravar o snapshot JSON no encerramento dos hosts ainda usa stderr, mas só é alcançada quando o usuário já habilitou diagnóstico e a exportação final falha.
- A saída de panic padrão continua deliberadamente preservada; o arquivo de crash personalizado só é habilitado após consentimento.
- A API pública `sega-observability::init()` ainda não impõe token de consentimento; a segurança depende de os chamadores continuarem invocando-a somente após ação explícita.
- A PR #1091 foi mesclada em `88f6ef8059ea72538d086ab451ba0beaa16374dc`; a reavaliação do estado integrado está na seção 9.

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


## 9. Reavaliação do estado integrado — 20/11/2026

**SHA avaliado:** `cbcbdc531ba32ef933161349c2e860b96df0e95c`.
**Mudanças relevantes:** PR #1091 (`88f6ef8059ea72538d086ab451ba0beaa16374dc`) e PR #1093 (`413058589ae08547bab6c24ae88888c84f45cece`). A PR #1094 atualizou o roadmap documental e não altera o comportamento de observabilidade.

A auditoria inicial permanece válida como registro da baseline `a58edc2`, mas não deve ser lida como fotografia do código atual. A situação após os merges é:

| Achado original | Estado na reavaliação | Evidência/limite |
|---|---|---|
| DP0-001 / DP0-002 — relatórios R2/R3 via stderr | **Corrigido no caminho Slint** | O host encaminha esses eventos pelo macro `diagnostic_log!`, que só emite `tracing` quando `HOST_DIAGNOSTICS_ENABLED` está ativo. |
| DP0-003 — timer de diagnóstico incondicional | **Corrigido para amostragem diagnóstica** | PR #1093 separa o polling operacional de fallback do renderer do timer de frame pacing. O timer de amostragem é parado até a autorização ser bem-sucedida e é parado em revogação ou falha. O polling de fallback continua ativo por ser comportamento operacional, não coleta diagnóstica. |
| DP0-004 / DP0-005 / DP0-006 / DP0-007 — saídas diretas do host Slint | **Corrigido nos caminhos identificados** | Os callsites de diagnóstico do host passaram a usar o canal `tracing` condicionado ao consentimento; falhas operacionais também atualizam o status da UI. Resta um `eprintln!` de falha ao gravar o snapshot final, alcançado somente se o usuário manteve o consentimento ativo e a gravação falhou. |
| DP0-008 — resolução automática de região no worker SMS | **Corrigido no código atual** | A leitura atual de `rust/runtime/sms-runtime/src/worker.rs` não contém `eprintln!`; os caminhos de fallback emitem `tracing::warn!`. |
| DP0-009 — API pública `init()` não fail-closed | **Aberto — bloqueante** | `native/observability/src/runtime.rs` ainda cria diretório/arquivo, instala subscriber e panic hook durante `init()`. O filtro padrão permanece `info`. O comportamento seguro continua dependendo de os chamadores invocarem `init()` apenas após opt-in. |
| DP0-010 — saída do panic hook padrão | **Aberto — decisão de política** | O hook personalizado grava arquivo somente com o flag ativo, mas chama o hook anterior, preservando a saída padrão de panic. Não suprimir sem decisão explícita e teste. |
| DP0-011 — histórico de escritas de porta no estado do barramento | **Aberto — classificação/compatibilidade** | Requer decidir se é estado semântico ou trace diagnóstico; não remover nem alterar snapshots sem análise. |
| DP0-012 — vetor de trace do core sem limite próprio | **Aberto — limite de API** | A captura começa desabilitada, mas consumidores diretos do core podem reter eventos sem limite enquanto habilitada. |
| DP0-013 — telemetria do worker SMS sem consentimento | **Corrigido** | PR #1091 condicionou as amostragens de frame/input/pacing ao estado diagnóstico e limpou histórico nas transições de consentimento. |

### 9.1 Estado das fatias DP0-F

- **DP0-F1 — separar operação de coleta:** integrado pela PR #1093. O timer operacional de fallback permanece independente; o timer de frame pacing depende do opt-in.
- **DP0-F2 — remover saída diagnóstica direta do host:** os callsites de operação normal identificados foram migrados para `tracing` condicionado ao consentimento. O único stderr encontrado no host para esse fluxo é a falha da gravação final após consentimento ativo; pode ser convertido em mensagem de UI numa revisão de UX, mas não representa coleta sem autorização.
- **DP0-F3 — worker SMS:** os callsites de região não emitem mais diretamente em stderr no HEAD avaliado; eventos estruturados são filtráveis pelo subscriber.
- **DP0-F4 — API fail-closed:** permanece a próxima fatia bloqueante. A API pública de inicialização e a escrita do snapshot precisam exigir uma fronteira explícita de ativação/autorização, sem efeitos de filesystem, subscriber ou hook antes do opt-in.
- **DP0-F5 — panic/crash:** pendente de decisão e validação específica; preservar o hook padrão até ADR/decisão documentada.
- **DP0-F6 — inventário completo de sinks:** não concluído. A busca textual atual encontrou saídas em stubs não-Windows, ferramentas CLI, exemplos e testes, que devem ser classificadas por caminho de execução; busca textual isolada não prova cobertura completa.
- **DP0-F7 — validação:** não concluída. Os CI dos merges #1091/#1093 validam os testes afetados, mas não substituem testes explícitos de ausência de arquivos sem consentimento, revogação com fila pendente, fila cheia, Debug/Release e overhead.

### 9.2 Próximo passo técnico

Implementar DP0-F4 no crate compartilhado `native/observability`: separar preparação side-effect-free da ativação após consentimento, iniciar com filtro `off`, impedir escrita de snapshot sem autorização apropriada e preservar singleton do subscriber, fila limitada, rotação e comportamento de shutdown. Atualizar os dois hosts consumidores de forma coordenada e cobrir o contrato com testes isolados de filesystem e transições de estado.

Esta reavaliação é estática e vinculada ao SHA informado. Não declara DP-0 concluído, não comprova execução física nem afirma overhead zero.


## 10. DP0-F6 — inventário inicial de sinks e integrações (20/11/2026)

**Baseline:** `main` após o merge da PR #1099, commit
`68861c17cbf1d4a595e9bbefbb8d95cbad8b9688`.

**Estado:** inventário inicial produzido; DP0-F6 permanece **em andamento**. A pesquisa estática abaixo delimita sinks e integrações conhecidos, mas não prova cobertura exaustiva nem substitui inspeção de todos os callsites e testes em runtime.

### 10.1 Critérios de classificação

Cada ocorrência encontrada deve ser classificada por finalidade e não apenas pelo nome da API:

1. **Diagnóstico opt-in:** evento destinado ao subscriber/arquivo/ring buffer; deve estar atrás da autorização e ser inerte quando desligado.
2. **Ação explícita do usuário:** exportação de trace/snapshot, seleção de arquivo, salvar SRAM ou firmware; exige gesto explícito e não deve ser confundida com coleta contínua.
3. **Estado operacional:** falha necessária para informar se uma operação solicitada foi concluída; preferir resultado/estado na UI, sem transformar isso em telemetria persistente.
4. **Persistência funcional:** SRAM, firmware/configuração e outros dados necessários ao funcionamento; não são logs diagnósticos, mas precisam de inventário de localização, identidade e lifecycle.
5. **Ferramentas, exemplos e testes:** saída de CLI, smoke tests e harnesses; classificar por executabilidade/distribuição, sem migrar automaticamente para o logger do produto.
6. **Build/CI:** mensagens `cargo:`, logs de workflows e captura de artefatos; fora do consentimento de runtime do usuário, mas precisam permanecer sem dados pessoais ou conteúdo de ROM.

### 10.2 Inventário de sinks e pontos de integração identificados

| Área / arquivo | Sink ou mecanismo | Classificação inicial | Fronteira/risco a validar |
|---|---|---|---|
| `native/observability/src/runtime.rs` | subscriber `tracing`, filtro reload, fila non-blocking, writer, ring buffer, snapshot JSON, panic hook | Diagnóstico opt-in | Validar cada entrada pública, revogação com fila pendente e snapshot; F5 foi integrado pela #1099. |
| `native/observability/src/writer.rs` | arquivo rotativo e fanout para eventos recentes | Diagnóstico opt-in | Writer só pode existir após autorização; validar shutdown/flush e limites. |
| `native/sega_desktop/src/main.rs` | macro `diagnostic_log!`, integração do host com subscriber; amostragem de frame pacing/áudio; snapshot de encerramento | Diagnóstico opt-in + estado operacional | Confirmar que todos os callsites ativos estão condicionados; falha do snapshot ainda pode ir a stderr depois de consentimento ativo. |
| `native/sega_desktop/src/desktop_session.rs` | comandos de trace SMS e sessão SMS/SG-1000 | Ação explícita do usuário | Trace de barramento é finalidade separada de logging geral; verificar fluxo de autorização e exportação. |
| `rust/runtime/sms-runtime/src/worker.rs` | `tracing::warn!`/eventos de fallback, métricas e janelas limitadas de frame pacing | Diagnóstico opt-in + estado operacional | A coleta de métricas depende de consentimento; diferenciar métricas necessárias à execução de amostragem diagnóstica. |
| `native/video_runtime/src/diagnostics.rs` | QPC clock/frequency helper | Instrumentação de suporte, não sink | A leitura do relógio por si só não persiste nem publica dados; auditar os consumidores e timers. |
| `native/audio_runtime/src/session.rs`, `native/audio_runtime/src/lib.rs`, `native/audio_runtime/src/psg.rs` | eventos estruturados `tracing` sobre sessão/PSG/saída de áudio | Diagnóstico opt-in | Conferir campos e frequência; eventos de alta frequência não podem bloquear nem alterar o runtime. |
| `native/video_runtime/src/d3d12.rs` | eventos estruturados de inicialização/falha do renderer | Diagnóstico opt-in | Conferir se dados de device/adapter/window são minimizados e não incluem caminhos pessoais. |
| `native/machine_scheduler/src/lib.rs`, `native/z80_cpu/src/cpu.rs`, `native/sms_core/src/temporal_bus.rs`, `native/video_core/src/sms/vdp.rs`, `native/sn76489/src/psg.rs`, `native/sg1000_core/src/machine.rs` | eventos estruturados de erros, violações e configuração | Diagnóstico opt-in | A fachada `tracing` não instala subscriber; validar custo lazy/níveis e ausência de I/O nos caminhos de hardware. |
| `native/sms_core/src/cartridge/image.rs`, `native/sms_core/src/memory.rs` | eventos de imagem ROM/mapper | Diagnóstico opt-in | Apenas metadados permitidos; nunca bytes de ROM/BIOS ou caminhos absolutos. |
| `native/save_runtime/src/lib.rs` | leitura/escrita persistente de SRAM | Persistência funcional | Não classificar como log; revisar identidade/hash, caminhos, atomicidade e somente operações de save/load solicitadas. |
| `native/media_runtime/src/sms_firmware.rs` | leitura/validação de firmware | Persistência/entrada funcional | Não classificar como sink diagnóstico; evitar registrar caminhos completos ou conteúdo do firmware. |
| `native/media_runtime/src/catalog.rs` | catálogo e perfis de mídia | Dados funcionais em memória | Não é sink de log; confirmar ausência de persistência automática de telemetria. |
| `native/sega_app/src/app.rs` | host egui legado, integração de logging e snapshot | Legado + diagnóstico opt-in | Manter inventariado até confirmar todos os consumidores/builds; não é destino da GUI final. |
| `native/sms_debugger/src/main.rs`, `native/sms_core/src/bin/z80test_runner.rs` | stdout/stderr de CLI | Ferramenta explícita | Separar execução interativa de tooling/test harness; confirmar se algum binário é distribuído ao usuário final. |
| `native/*/examples/**`, `native/*/tests/**`, `rust/runtime/**/tests/**` | stdout/stderr, CSV/WAV/trace e arquivos temporários | Teste/ferramenta | Classificar por alvo; artefatos de teste não devem ser tratados como logs do desktop. |
| `native/sega_app/build.rs` e `.github/workflows/**` | `cargo:rustc-env`, logs de build e CI | Build/CI | Não é consentimento de runtime; não introduzir dados pessoais, ROMs ou caminhos locais nos logs. |

### 10.3 Busca estática reproduzível e seus limites

A busca no repositório por `eprintln!`, `println!`, `tracing::`, `log::`, `std::fs`, `std::io::Write`, `create_dir_all`, `write_all`, `OpenOptions`, `Command::new`, `diagnostics_enabled`, `trace_enabled` e `telemetry` encontrou, entre outros, os caminhos listados acima.

Resultados relevantes:
- O caminho Slint ativo e o worker SMS têm pontos de integração explícitos com diagnóstico e consentimento.
- A infraestrutura compartilhada concentra writer, snapshot e panic hook em `native/observability`.
- Há persistência funcional separada em `save_runtime` e leitura/validação de firmware em `media_runtime`.
- Há saídas de CLI/examples/tests e stubs não-Windows que não devem ser automaticamente classificadas como sinks do produto.
- A busca por texto não encontra necessariamente macros importadas por `use`, wrappers locais, APIs de terceiros, FFI, handles de stdout/stderr, subprocessos, arquivos criados por dependências ou caminhos condicionais de plataforma. A indexação de code search também pode atrasar em relação ao HEAD.

### 10.4 Lacunas que impedem fechar F6

1. Percorrer integralmente os callsites do host Slint, host legado, workers SMS/SG-1000, áudio e vídeo, verificando os campos emitidos e o gate real em cada caminho.
2. Auditar todos os pontos de escrita/leitura em disco e exportações (save state, SRAM, trace, snapshot, logs, crash e artefatos), separando persistência funcional de diagnóstico.
3. Inspecionar wrappers, macros importadas, crates externos, FFI e subprocessos; busca por tokens isolados não comprova ausência de outros sinks.
4. Confirmar quais bins/examples são empacotados/distribuídos e quais existem somente em CI/testes.
5. Testar os caminhos sem consentimento e após revogação, incluindo fila pendente, snapshot final, stderr de falha operacional e hook padrão de panic.
6. Produzir matriz final por sink com: origem, consumidor, dados/campos, gatilho, consentimento/finalidade, destino, retenção, redaction, custo/frequência e teste que prova a fronteira.

### 10.5 Relação com DP0-F5

A PR #1099 foi integrada em `main` pelo commit
`68861c17cbf1d4a595e9bbefbb8d95cbad8b9688`. Os checks do HEAD da PR foram concluídos: 11 checks `success` e 9 `skipped` por não aplicabilidade. Os jobs dedicados de observabilidade Windows x64 Debug/Release e o teste/build do shell legado CPAL Windows x64 Release passaram. Isso fecha a implementação DP0-F5, mas não fecha o inventário F6 nem a validação F7.

**Conclusão F6:** inventário inicial documentado; auditoria completa permanece aberta. Nenhuma alteração de hardware, Scheduler, CPU, VDP, PSG, barramentos, mappers ou temporização foi feita nesta fatia.


### 10.6 Resultado da inspeção dos callsites e classificação refinada

A leitura dos arquivos-fonte da baseline confirmou os seguintes grupos:

**A. Eventos `tracing` efetivamente presentes no código de produção**

| Crate / arquivo | Targets e eventos | Dados emitidos / observação |
|---|---|---|
| `native/sms_core/src/cartridge/image.rs` | `sega::rom` | Formato da imagem, tamanho e rejeição de imagem vazia; não registra bytes nem caminho da ROM. |
| `native/sms_core/src/memory.rs` | `sega::mapper` | Tipo de mapper, tamanho/bancos, presença/tamanho de BIOS, endereço/valor de escritas e mudanças de latches. O trace de escritas pode ser frequente. |
| `native/machine_scheduler/src/lib.rs` | `sega::scheduler` | Registro de clock domains, agendamento/dispatch de eventos, timestamps, prioridades, contagens e falhas. Há eventos em operações temporais frequentes. |
| `native/video_runtime/src/d3d12.rs` | `sega::video::d3d12` | Criação/atualização de janela, HWNDs, dimensões, LUID do adaptador, swapchain, Present e erros DXGI/D3D12. HWND/LUID merecem revisão de minimização. |
| `native/sn76489/src/psg.rs` | `sega::psg` | Inicialização, reset, escrita de registrador, drenagem/overflow da fila de amostras. Escrita de registrador e drenagem podem ocorrer frequentemente. |
| `native/sg1000_core/src/machine.rs` | `sega::machine`, `sega::vdp`, `sega::io` | Configuração da máquina, mudança de sprite limit e escrita de porta com endereço, valor e T-state. O último evento é um caminho de I/O de hardware e pode ser frequente. |
| `native/z80_cpu/src/cpu.rs` | `sega::z80` | Reset, NMI/INT latched/accepted e falha de execução, com PC, T-states e vetor/dados. Não há evento por instrução normal nesta busca. |
| `native/audio_runtime/src/lib.rs` | `sega::audio::dsp` | Inicialização, volume, batches de amostras processados e flush. Os eventos trace acompanham processamento de áudio. |
| `native/audio_runtime/src/session.rs` | `sega::audio::session` | Ciclo de vida e operações da sessão/sink, incluindo métricas e eventos de fluxo de áudio. |
| `native/audio_runtime/src/psg.rs` | `sega::audio::psg` | Domínio de clock, perfil, clock e sample rate na criação do PSG agendado. |
| `native/sms_core/src/temporal_bus.rs` | `sega::machine`, `sega::bus` | Configuração de máquina e rejeições/erros de requests, endereços, IDs, operações e T-states. |
| `native/video_core/src/sms/vdp.rs` | `sega::vdp` | Inicialização/reset, IRQ de linha, VBlank, publicação de frame e mudanças de registradores. |
| `native/sega_app/src/app.rs` | `sega::ui::menu`, `sega::app`, `sega::settings` | Eventos de menu, consentimento e lifecycle; os callsites consultados verificam o handle/estado de logging ou estão dentro do callback de opt-in/shutdown. |
| `native/sega_desktop/src/main.rs` | `sega::host`, `sega::app` | Macro `diagnostic_log!` condicionado por `HOST_DIAGNOSTICS_ENABLED`, eventos do host e snapshot final. |
| `rust/runtime/sms-runtime/src/worker.rs` | `sega::rom` e eventos de worker | Resolução de região/fallback e eventos de runtime. A telemetria de performance do SMS tem gates explícitos em `diagnostics_enabled`. |

O crate `native/observability` reexporta `tracing`; o subscriber global e o writer são instalados pelo feature `runtime`. O filtro inicial do subscriber é `off`; `LogConfig::default()` seleciona `info` após opt-in, salvo configuração por `SEGA_LOG_FILTER`. Portanto, os eventos acima não são automaticamente gravados na inicialização normal. Após consentimento, eventos INFO de crates dependentes também podem ser capturados pelo subscriber global; não existe allowlist de targets documentada nesta baseline.

**B. Saídas diretas de console**

A busca estática encontrou 12 arquivos com `eprintln!` e 22 resultados/arquivos com `println!` (incluindo ocorrências de documentação). A classificação das ocorrências de código é:

- **Hosts de produto:** `native/sega_desktop/src/main.rs` e `native/sega_app/src/app.rs` mantêm `eprintln!` apenas no tratamento de falha do snapshot final, depois de uma sessão de diagnóstico consentida e do shutdown do writer. Não é emissão automática sem opt-in, mas ainda é um canal de saída fora do subscriber e deve ser substituído por status da UI/retorno estruturado ou explicitamente justificado.
- **Stubs não-Windows:** `native/sega_app/src/main.rs`, `native/sms_debugger/src/main.rs` e os exemplos D3D12 imprimem que o alvo não é suportado. Não são caminhos do host Windows x64 suportado.
- **CLI/tooling:** `native/sms_core/src/bin/z80test_runner.rs`, `native/z80_alu/src/main.rs` e `native/ym2413/examples/eg_timer_trace.rs` usam stdout/stderr como interface explícita de linha de comando.
- **Examples e smoke tests:** `native/sn76489/examples/psg_trace.rs`, `native/ym2413/examples/*trace.rs`, `native/sms_core/examples/irq_wiring_repro.rs`, `native/sms_core/examples/ace_of_aces_boot_trace.rs`, `native/video_runtime/examples/d3d12_*_smoke.rs`, `native/machine_scheduler/tests/stress_replay.rs` e `rust/runtime/sms-runtime/tests/in_game_rom_smoke.rs` produzem saída/artefatos de desenvolvimento ou teste.
- **Build:** `native/sega_app/build.rs` usa `println!("cargo:...")` para instruções do Cargo e metadados de build; não é logging de runtime.
- **Debugger:** `native/sms_debugger/src/app.rs` não imprime o relatório em console; exporta um TXT somente após o usuário abrir o diálogo e confirmar o caminho.

Não foi encontrado uso de `log::info!`, `log::warn!`, `log::error!` ou `tracing::event!` nos resultados da busca exata. A busca por `log::` isolada produziu falsos positivos por substring e não foi considerada evidência de uso do facade `log`.

**C. Persistência funcional, preferências e exportações — fora do canal de diagnóstico**

| Arquivo/área | Operação | Dados e gatilho |
|---|---|---|
| `native/save_runtime/src/lib.rs` | Leitura e escrita SRM atômica | SRAM persistente vinculada à identidade SHA-1/SHA-256/CRC32 da ROM e perfil do cartucho. É persistência funcional do jogo, não telemetria; a escrita ocorre no fluxo de save do host. |
| `native/sega_desktop/src/main.rs` | `slint_ui_settings.txt`, `recent_roms.txt` | Preferências, teclas e caminhos absolutos das ROMs recentes; persistência de preferências da aplicação. |
| `native/sega_app/src/app.rs` | `ui_settings.txt`, lista recente e arquivos de bindings | Preferências, posições/tamanhos de janelas, estado do debugger, caminhos de ROM e bindings. Persistência funcional; caminhos locais são dados pessoais do ambiente e devem ser tratados como tal. |
| `native/sms_debugger/src/app.rs` | Exportação `sms-debug-report.txt` | Ação explícita do usuário pelo diálogo Save; o relatório inclui caminho da ROM e estado de CPU/VDP/mapper. Não é upload automático; o usuário controla destino e compartilhamento. |
| `native/sega_desktop/src/main.rs` | Exportação de trace/snapshot pelo host | Ação explícita do usuário via diálogo de arquivo; deve permanecer separada da coleta contínua e do consentimento de logs. |
| `native/media_runtime/src/sms_firmware.rs` | Leitura/validação de firmware | As ocorrências de escrita encontradas pertencem a fixtures de teste; o caminho de produção valida/carrega arquivo selecionado, sem evidência de telemetria persistente. |
| `native/input_runtime/src/sdl3_backend.rs` | Criação/escrita de DLLs falsas | Fixtures de teste para resolver prioridade/localização de SDL3, não instalação ou logging do produto. |
| `native/sn76489/src/psg.rs`, `native/sn76489/src/psg_h2_model.rs` | WAV/CSV e artefatos | Testes e estudo de reconstrução de áudio; não são sinks de runtime normal. `PSG_AUDIO_ARTIFACT_DIR` ativa artefatos de teste/benchmark. |
| `native/sms_core/examples/ace_of_aces_boot_trace.rs` | Arquivo de trace | Ferramenta manual que lê ROM/BIOS locais e grava checkpoints/transações em caminho especificado; documentação do exemplo declara que bytes de ROM/BIOS não são copiados ao repositório. |

A busca por `fs::write`, `write_all`, `File::create`, `OpenOptions`, `fs::rename` e `create_dir_all` também retornou fixtures e testes. Esses resultados foram classificados por callsite, e não considerados automaticamente telemetria.

### 10.7 Achados e critérios de saída da F6/F7

**DP0-F6-A — ring buffer de eventos sobrevive à revogação (risco de lifecycle, encaminhado à F7).**

Em `native/observability/src/runtime.rs`, `LoggingHandle::disable()` desliga o flag atômico e recarrega o filtro `off`, mas não limpa `recent_events`. `enable()` reativa o filtro sem limpar esse buffer, e `write_diagnostics_json()` serializa `recent_events()` quando o consentimento está ativo. Assim, eventos coletados durante um período de consentimento anterior podem permanecer em memória e reaparecer em snapshot após uma nova autorização. O comportamento deve ser definido e testado; recomendação: limpar o ring buffer na revogação e confirmar que não há eventos de uma autorização anterior após reconsentimento. A mesma decisão deve especificar o destino de registros já enfileirados no writer non-blocking quando ocorre revogação.

**DP0-F6-B — saída de falha do snapshot ainda usa stderr nos dois hosts.**

A saída só é alcançada depois de consentimento ativo e falha de escrita do snapshot final, mas permanece fora do canal estruturado. A correção de menor risco é apresentar a falha como estado operacional da UI ou encaminhar um erro mínimo para um mecanismo de status sem reativar logging após shutdown. Não escrever um segundo arquivo de diagnóstico em resposta à falha do primeiro.

**DP0-F6-C — instrumentação em caminhos sensíveis a desempenho.**

Existem callsites `tracing::trace!` em escrita de portas SG-1000, escritas do PSG, drenagem de áudio, agendamento/dispatch do Scheduler, barramento SMS e transições do VDP. Os eventos são condicionados pelo filtro global e o writer é non-blocking/lossy, mas a criação/dispatch do evento ainda pode ter custo quando a categoria estiver habilitada. F7 deve comparar Debug/Release com diagnóstico off, INFO e TRACE, incluindo execução contínua e em blocos, e provar que nenhuma métrica altera o estado temporal ou a ordem do Scheduler.

**DP0-F6-D — telemetria operacional SG-1000 não é o mesmo que coleta diagnóstica SMS.**

`rust/runtime/sg1000-runtime/src/worker.rs` mantém contadores de frame/T-states/instruções, erros e snapshot de métricas de áudio em memória para comunicação com o host. Diferentemente do SMS, essa estrutura não possui um flag de consentimento próprio. A inspeção não encontrou writer, subscriber ou persistência nesse worker; os campos são usados como estado operacional/debug snapshot. Manter essa distinção explícita e verificar que nenhum host os exporta automaticamente sem opt-in.

**DP0-F6-E — preferências e histórico de ROMs persistem caminhos locais.**

Os hosts persistem caminhos de ROM em arquivos de preferências/lista recente. É comportamento funcional solicitado pela lista de recentes, não diagnóstico. Deve haver uma ação de usuário para limpar histórico e a política de retenção deve ser documentada; não condicionar automaticamente a persistência de preferências ao consentimento de diagnóstico.

**Critérios para considerar F6 concluída:**
- Todos os callsites dos targets listados revisados quanto a campos e possíveis caminhos locais/identificadores.
- Busca complementar de wrappers/macros próprios, APIs de terceiros, FFI e subprocessos concluída; nenhum sink não classificado nos bins distribuídos.
- Cada destino de arquivo classificado por finalidade, gatilho, dados, retenção e exclusão.
- DP0-F6-A e DP0-F6-B encaminhados com decisão/alteração explícita e testes.
- Matriz final com referências de arquivo/linha e testes que provam cada fronteira.

**Plano de validação F7 atualizado:** testes em subprocesso e filesystem temporário para (1) startup sem consentimento sem criação de diretório/arquivo/subscriber/hook privado; (2) opt-in cria somente os destinos previstos; (3) revogação interrompe novos eventos, limpa memória pendente conforme política e não reexporta eventos antigos após reconsentimento; (4) panic após revogação não grava crash privado; (5) fila cheia permanece lossy/non-blocking; (6) snapshot exige consentimento ativo; (7) Debug/Release e filtros INFO/TRACE medem overhead em worker SMS, SG-1000, Scheduler, Z80, VDP e PSG; (8) execução contínua vs em blocos preserva estado/hash e resultados temporais com diagnóstico off/on.

### 10.8 Reconciliamento do histórico desta auditoria

As seções 9.1 e 9.2 registram a fotografia anterior e os próximos passos daquele SHA; não devem ser interpretadas como estado atual após os merges #1098/#1099. Na baseline desta seção, DP0-F4 usa a API `init_after_explicit_consent` e o filtro inicial `off`; DP0-F5 está integrada pela PR #1099. A reavaliação F6 acima prevalece para a classificação atual. A API de consentimento documenta que o host é responsável por criar a capability somente em resposta à ação afirmativa; a linguagem não consegue provar que uma ação humana ocorreu.

**Estado após esta inspeção:** inventário estático ampliado e achados encaminhados; F6 ainda não está encerrada porque a verificação de wrappers/FFI/dependências e a resolução/testagem dos achados de lifecycle permanecem pendentes. Nenhuma alteração de código de hardware/runtime foi feita.


### 10.9 Referências de código para os achados de lifecycle

- `native/observability/src/runtime.rs:131-159`: `enable()` e `disable()`; a revogação altera o flag e o filtro, mas não limpa `recent_events`.
- `native/observability/src/runtime.rs:184-185, 210-249`: leitura do ring buffer e serialização do snapshot.
- `native/observability/src/runtime.rs:303-370`: inicialização consentida, criação do diretório/writer, fila non-blocking e filtro inicial.
- `native/observability/src/writer.rs:47-58`: publicação de linhas no ring buffer; `47-58` não verifica um estado de consentimento.
- `native/observability/src/writer.rs:62-69`: o fanout escreve no destino e acumula as linhas completas para o snapshot.
- `native/sega_desktop/src/main.rs:73-117`: callback de opt-in/revogação, shutdown e stderr de falha do snapshot.
- `native/sega_app/src/app.rs:1216-1268`: transição de consentimento do host egui legado; `1779-1811`: shutdown e stderr de falha do snapshot.
- `native/sega_desktop/src/main.rs:1562-1685`: persistência de preferências e lista de ROMs recentes.
- `native/sega_app/src/app.rs:620-702, 1996-2021`: persistência de preferências e lista de ROMs recentes.
- `native/sms_debugger/src/app.rs:236-288`: exportação explícita do relatório.
- `rust/runtime/sg1000-runtime/src/worker.rs:50-110`: telemetria operacional em memória do SG-1000.

**Confirmação externa da semântica da fila:** a documentação oficial de `tracing-appender 0.2.5` informa que `NonBlocking` enfileira eventos para um worker dedicado e que `WorkerGuard` drena/flush os registros pendentes ao ser descartado. O método `flush` de `NonBlocking` não fornece uma operação pública de purga seletiva da fila. Portanto, a política de revogação precisa distinguir registros já aceitos/enfileirados antes da revogação de eventos novos, e definir se os registros antigos podem continuar chegando ao arquivo depois do clique de revogação. Referência: https://docs.rs/tracing-appender/0.2.5/tracing_appender/non_blocking/.


**DP0-F6-F — captura de trace do debugger é restaurada automaticamente pelo host egui legado.**

Em `native/sega_app/src/app.rs:522-526`, a preferência `debugger_trace_enabled` é lida do arquivo de configurações; em `1160-1195`, ao criar um novo worker SMS, `trace_capture_enabled` restaurado causa `worker.set_trace_enabled(true)` sem novo clique na sessão atual. A mesma preferência é serializada em `679-702`. O controle visual permite ativar/desativar o trace, mas a persistência faz com que uma autorização anterior seja reutilizada após reinicialização. Diferentemente de breakpoints e geometria de janela, trace captura eventos de barramento e pode crescer em custo/volume. **Recomendação:** não persistir nem restaurar `debugger_trace_enabled`; iniciar sempre desligado e exigir ativação explícita na sessão atual. Preservar os demais ajustes do debugger.

**DP0-F6-G — trace core e coletor do worker têm limites diferentes.**

`native/sms_core/src/temporal_machine.rs` mantém `trace_events: Vec<SmsTemporalTraceEvent>`, inicia com captura desligada e faz `push` quando `trace_enabled` está ativo. O worker SMS cria um `DiagnosticCollector` com capacidade limitada de 512 eventos e política de descarte; consumidores diretos de `SmsTemporalMachine` não recebem automaticamente esse limite. A política de coleta e limite do worker não deve ser confundida com a API pública do core. Manter o limite do core como contrato pendente, sem alterar o formato de snapshots sem análise de compatibilidade.
