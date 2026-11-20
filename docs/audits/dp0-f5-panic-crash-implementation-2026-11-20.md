# DP0-F5 — implementação da política de panic/crash

- **Data:** 2026-11-20.
- **Base:** main após PR #1098, merge commit 5f669f4017dae8824ec9e984c00cb697b91489b0.
- **Branch:** feat/dp0-f5-sanitized-panic-crash-2026-11-20.
- **ADR normativa:** [ADR 0012 — política de panic e crash diagnostics](../adr/0012-panic-crash-diagnostics-policy.md).
- **Estado:** implementação submetida para CI; não declarar DP0-F5 concluída até os gates Debug/Release Windows x64 passarem.

## Alterações nesta fatia

1. O hook privado grava apenas timestamp Unix, ID de sessão sanitizado, basename do arquivo-fonte, linha, coluna e indicador booleano de payload textual.
2. O texto do payload e o backtrace não são formatados nem persistidos.
3. O basename e o identificador de sessão são limitados e filtrados para caracteres ASCII seguros; caminhos e diretórios não são incluídos.
4. Cada registro tem limite de 1 KiB. O arquivo respeita o limite configurado; se o registro não couber, é descartado.
5. Rotação/retenção é executada durante a inicialização consentida, antes da instalação do hook. Nenhuma operação de rename/remove/rotação ocorre no caminho de panic.
6. O estado atômico de consentimento é consultado no instante do panic. Revogação impede escrita privada, mas não altera o hook padrão.
7. Falha de I/O é ignorada no caminho privado; a chamada do hook anterior continua exatamente uma vez.
8. Não são usados logger, subscriber, worker, GUI, Scheduler, mutexes de aplicação, rede, minidumps ou catch_unwind no hook.

## Testes adicionados

- Formatter puro com sentinelas de caminho/ROM/identificador e verificação de ausência de conteúdo sensível.
- Limite de tamanho do arquivo e descarte de registro que excederia o máximo.
- Rotação e retenção de backups antes da instalação do hook.
- Subprocessos isolados para ausência de opt-in, opt-in ativo, revogação anterior ao panic e falha de I/O.
- O hook anterior grava marcador; o teste exige exatamente uma chamada em cada cenário.
- O cenário com opt-in confirma que o registro privado não contém sentinelas do payload.

## Limites e validação pendente

- A implementação ainda precisa passar por cargo fmt, Clippy, testes com feature runtime e os workflows Windows x64 Debug/Release.
- O teste de subprocesso verifica delegação exatamente uma vez e término não-success do processo de teste; a saída stderr pode variar por harness/ambiente e não é usada como evidência textual idêntica.
- O hook privado continua instalado após revogação, mas o flag atômico bloqueia a captura. O hook anterior permanece ativo por decisão da ADR.
- A retenção é aplicada antes do hook e o hook nunca faz rotação; uma falha de preparação não impede o comportamento normal do panic.
- Esta fatia não inventaria sinks (DP0-F6), não valida fila/overhead/revogação com eventos já enfileirados (DP0-F7) e não fecha DP-0.

## Preservação arquitetural

Nenhuma alteração foi feita em CPU, Z80, MachineScheduler, VDP, PSG, barramentos, mappers, runtime SMS/SG-1000 ou temporização emulada.
