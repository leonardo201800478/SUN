# SUN S0 — relatório agregado de desempenho

## Objetivo

Medir o consumo de execução do SMS em uma campanha completa, sem produzir
rastreamento contínuo. Os contadores ficam em memória durante cada cenário e
são exportados uma única vez ao final. Não há log por instrução, T-state,
evento, frame ou intervalo de milissegundos.

O throughput oficial continua sendo medido sem profiler externo. A coleta
agregada opt-in acrescenta contadores inteiros nos limites já existentes do
backend; não cria scheduler, clock, callback ou thread de profiling.

## Cenários

O teste `perf_baseline.rs` executa OutRun SMS em três configurações:

- A: SMS sem YM2413.
- B: YM2413 japonês instalado e clockado, com saída FM mutada.
- C: YM2413 clockado e FM mixado pelo mixer/resampler nativo, consumido por
  sink determinístico de descarte.

Cada cenário possui uma passagem de throughput sem timers de subsistema e uma
passagem diagnóstica separada. O FPS da passagem de throughput é a referência;
as medições diagnósticas servem para atribuição e não substituem a baseline.

## Execução

No Windows x64, com working tree limpo e ROM OutRun SMS obtida legalmente:

```powershell
.\scripts\benchmark\perf-baseline-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms" -CargoProfile release-llvm -WarmupSeconds 5 -CaptureSeconds 30 -DiagnosticSeconds 5
```

O alias legado `perf-profile-s0.ps1` agora delega para o mesmo runner agregado.
Ele não inicia WPR Heap nem cria ETL. A opção `-CaptureCpuProfile` do runner
principal continua disponível somente quando uma campanha separada de CPU
sampling for realmente necessária; não habilitar para obter a baseline oficial.

## Dados coletados

### Z80

Os contadores são acumulados em memória e incluem:

- instruções aposentadas;
- T-states reportados na aposentadoria da instrução, incluindo WAIT inserido;
- M-cycles concluídos por instrução, contados pelo índice M-cycle do sequencer na fronteira de aposentadoria;
- M-cycles e T-states do serviço de interrupção, contabilizados separadamente;
- serviços de interrupção e T-states consumidos no serviço;
- fetches de opcode, incluindo bytes de prefixo;
- T-states de ciclos internos.

Não há timer por instrução. Isso evita chamar `Instant::now()` a cada operação
do sequencer e evita que o profiling se transforme em custo relevante.

M-cycles são contados pelo índice interno do sequencer na fronteira em que a
instrução ou o serviço de interrupção termina. Não são estimados dividindo
T-states por três ou quatro.

### BUS

O relatório conta:

- leituras e escritas de memória;
- leituras e escritas de portas I/O;
- acknowledge de interrupção;
- fetches de opcode;
- T-states de WAIT observados nos requests.

Os contadores observam transações concluídas no caminho temporal. Eles não
alteram o barramento, os WAITs, a ordem de dispatch ou o estado salvo.

### Subsistemas já instrumentados

- Scheduler: dispatches e tempo agregado de dispatch.
- PSG: chamadas, clocks entregues e tempo agregado.
- YM2413: tempo agregado de avanço já existente.
- VDP: chamadas, T-states entregues e tempo agregado de avanço/renderização.
- Worker: tempo inclusivo de step_frame, processamento do frame, publicação de
  vídeo e preparação/submissão de áudio.

Os tempos pertencem a fronteiras diferentes. Não somar tempos inclusivos e
exclusivos como se fossem partes disjuntas do frame.

O tempo exclusivo de CPU host do Z80 ainda não é medido por timer interno.
Para atribuição por função, usar CPU sampling em uma campanha separada e
opcional, com símbolos correspondentes. Não usar Heap tracing nesta campanha.

## Artefatos

Cada execução gera:

- `manifest.json`: commit, branch, ROM SHA-256, toolchain, CPU, SO e parâmetros.
- `outrun-a-b-c.csv`: throughput sem profiler externo.
- `subsystem-diagnostics.csv`: uma linha agregada por cenário.
- `system-summary.json`: resumo dos três cenários e diagnósticos.
- `benchmark.log` e `build.log`: saída de execução e compilação.

O relatório é pequeno e limitado à quantidade de cenários. Nenhum ETL ou log de
heap é criado por padrão.

## Interpretação

- `z80_tstates` inclui os WAITs associados à instrução aposentada.
- `wait_tstates` é o contador de espera observado nos limites de BUS/M1; não
  deve ser somado novamente a `z80_tstates`.
- `internal_tstates` conta intervalos internos concluídos pelo Scheduler.
- Fetches de opcode incluem bytes de prefixo, não apenas instruções aposentadas.
- Contadores de BUS e CPU são quantidades; não representam tempo de CPU host.
- `step_frame` é inclusivo. Não subtrair ou somar timers sem respeitar as
  fronteiras medidas.
- A campanha headless não mede CPAL físico, composição Slint, upload WGPU nem
  execução GPU.

## Garantias de baixo impacto

- Contadores de tamanho fixo, com saturação em overflow.
- Nenhuma alocação por instrução ou transação para registrar dados.
- Nenhum arquivo aberto durante o hot path.
- Snapshot dos contadores somente na fronteira de frame e exportação ao final.
- Profiling desabilitado fora da campanha diagnóstica.
- Nenhum dado de profiling é serializado em save states.
- Nenhuma métrica realimenta o MachineScheduler ou altera o clock virtual.

## Estado

Esta etapa acrescenta contagem agregada do Z80 e do BUS ao S0. A análise de
tempo host exclusivo por função continua opcional e separada. Não iniciar F2
com base apenas nesses contadores: primeiro executar a campanha local, conferir
o tamanho dos artefatos e comparar os três cenários com a baseline anterior.
