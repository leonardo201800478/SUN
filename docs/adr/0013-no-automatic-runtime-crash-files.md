# ADR 0013 — diagnóstico RAM-first e ausência de crash files automáticos

- **Estado:** Aceita.
- **Data:** 2026-11-20.
- **Escopo:** `sega-observability`, runtimes de emulação e hosts desktop.
- **Supersede:** ADR 0012 — política de panic e crash diagnostics.
- **Relacionadas:** ADR 0011, Sprint #1108, DP0 #1110, PRs #1107 e #1117.

## Contexto

O produto precisa manter evidências diagnósticas suficientes para investigar falhas de emulação sem introduzir I/O contínuo, alocação ilimitada ou contenção no caminho temporal. O trace essencial do SMS deve existir em RAM durante a sessão, mas não deve ser confundido com logging persistente ou consentimento para exportação.

A política anterior permitia instalar um panic hook privado após consentimento e gravar um arquivo de crash. Isso amplia a coleta para panics comuns e não é necessário para o diagnóstico normal do runtime. A saída padrão do Rust já oferece o comportamento esperado para falhas não recuperáveis.

## Decisão

### 1. Trace essencial RAM-only

- O runtime SMS mantém um ring buffer fixo, pré-alocado e limitado, sempre ativo durante a sessão.
- A captura é independente do consentimento para logging detalhado.
- A gravação no ring não faz I/O, serialização, alocação, bloqueio, chamada de GUI ou espera por outro thread.
- O ring registra somente eventos selecionados com valor diagnóstico: conclusão de barramento/WAIT, prefixos Z80 amostrados, transições efetivas de INT e falhas observadas nas fronteiras do Scheduler/CPU/barramento.
- Transições rotineiras M1/T-state não entram no ring essencial; permanecem no trace detalhado opt-in.
- Overflow usa overwrite-oldest e contador monotônico de descartes.
- O ring não faz parte de Save States e não é limpo por revogação do consentimento.

### 2. Logging detalhado e persistência

- Logging estruturado, métricas e trace detalhado continuam opt-in por sessão.
- Buffers, filas e históricos são limitados e têm contadores de descarte.
- Revogar consentimento interrompe novos eventos e coleta detalhada; não desativa nem limpa o ring essencial em RAM.
- Não há snapshot automático no encerramento nem escrita contínua de logs sem consentimento.
- Exportar snapshots ou traces exige consentimento central ativo e ação explícita do usuário.
- A exportação pode incluir o ring essencial mesmo com o modo de trace detalhado desligado.

### 3. Panic e arquivos de crash

- A infraestrutura de observabilidade não instala nem substitui um panic hook global.
- O hook padrão do Rust permanece intacto e é responsável pela saída padrão e pelo comportamento normal de falha.
- Panics comuns não criam arquivos de crash, independentemente do estado do consentimento.
- Não capturar payload arbitrário, backtrace, caminhos absolutos, ROM/BIOS, memória, SRAM, entrada ou conteúdo de arquivos.
- Não usar `catch_unwind` para mascarar falhas de hardware/runtime.
- Não criar snapshot automático durante shutdown ou panic.

### 4. Exceção de bootstrap crítico

Um relatório mínimo só poderá ser criado quando existir um caminho fatal de inicialização explicitamente classificado, que impeça o produto de iniciar e para o qual um relatório seja necessário para diagnóstico. O relatório deve:
- ser acionado pelo caminho de erro classificado, nunca por um hook global;
- conter somente timestamp, estágio tipado e categoria de erro sanitizada;
- excluir payload arbitrário, backtrace, caminhos pessoais, ROM/BIOS e dados de usuário;
- ser limitado, versionado e documentado;
- não ser emitido para erros recuperáveis, falhas de ROM, erros normais de runtime ou panics comuns.

Nenhum callsite de bootstrap foi classificado como elegível nesta revisão; portanto, não há reporter automático de bootstrap instalado.

## Consequências

- O diagnóstico essencial sobrevive à revogação do consentimento enquanto a sessão está viva, mas permanece apenas em RAM.
- Um crash fatal do processo ainda perde o conteúdo volátil do ring; não se tenta mascarar ou recuperar panics para preservar esse estado.
- A investigação após falhas recuperáveis pode solicitar snapshot e exportação enquanto o host permanece ativo.
- A ausência de crash file é comportamento intencional, não falha de logging.
- Qualquer futura exceção de bootstrap exige revisão de segurança/privacidade e teste do callsite específico.

## Critérios de aceite

1. Testar capacidade fixa, wrap-around, overwrite-oldest e contador de descartes.
2. Testar que o ring permanece ativo sem consentimento e não é limpo por revogação.
3. Testar que trace detalhado continua opt-in e limitado.
4. Testar que a inicialização consentida não substitui o panic hook padrão.
5. Testar em subprocesso que panic comum não cria crash file.
6. Testar que snapshots/exportações exigem consentimento e ação explícita.
7. Demonstrar equivalência de instruções e ticks entre baseline e trace essencial.
8. Executar benchmark Release A/B e reportar overhead sem confundir ruído do runner com ganho real.


## Validação da implementação DP0 — 2026-11-20

A política desta ADR foi implementada e validada na revisão final da PR #1117:

1. Ring essencial fixo de 2.048 eventos, pré-alocado e sempre ativo em RAM; wrap-around, overwrite-oldest e contador saturante cobertos por testes.
2. Captura essencial independente do consentimento detalhado; revogação do trace detalhado não limpa o ring.
3. Trace detalhado separado, opt-in e limitado a 2.048 eventos, com contador de overwrite.
4. Snapshot essencial não destrutivo; teste comprova que o ring não integra Save States e inicia vazio após restore.
5. Exportação Slint exige consentimento central ativo e ação explícita de destino; o relatório pode incluir o ring essencial com trace detalhado desligado.
6. Testes cobrem falhas de execução, prefixos Z80, transições efetivas de INT e equivalência com captura detalhada ligada/desligada.
7. Benchmark Release A/B #31: 20 amostras por revisão; -0,47% bus-heavy e +0,16% relative-loop; instruções e ticks idênticos.
8. CI #9422 e SMS Runtime Release #1707 aprovados; CPAL 0.18.2 validado separadamente na CI #9418 da base #1107.

A comparação é Linux CI e não prova overhead zero; benchmark físico Windows x64 e smoke test interativo no hardware do usuário permanecem NOT RUN. A evidência detalhada e os IDs dos workflows estão na seção 12 da auditoria DP0.

A ADR define a política; a conclusão formal do DP0 depende ainda da confirmação do merge da PR #1117 e da validação pós-merge.
