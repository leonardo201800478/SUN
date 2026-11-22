# SUN S0 — System-wide performance profiling

## Objetivo

Ampliar o harness PERF-01/S0 existente sem criar outro emulador, worker,
scheduler ou implementação de benchmark. A medição oficial de throughput
permanece sem instrumentação. O profiling de CPU/heap ocorre em uma segunda
execução independente.

## Cobertura atual do harness

O teste perf_baseline.rs executa três cenários OutRun SMS:

- A: YM2413 ausente.
- B: YM2413 japonês instalado e clockado, com saída FM mutada.
- C: YM2413 clockado e FM mixado pelo mixer/resampler nativo, consumido por
  um sink determinístico de descarte.

Os diagnósticos do worker medem percentis de intervalo de quadro,
step_frame, avanço YM2413, publicação de vídeo, preparação/submissão de áudio
e processamento agregado do quadro. A instrumentação S0.2 acrescenta, em modo
opt-in, contagem/duração do dispatch do MachineScheduler, chamadas/clocks/duração
do avanço PSG e chamadas/T-states/duração do avanço VDP. O timer do VDP inclui
o trabalho de renderização de scanline executado por `SmsVdp::advance()`.

Esses timers são coletados nos limites reais do caminho temporal e não criam
um scheduler, relógio ou callback paralelo. O tempo do Scheduler mede somente
`SmsTemporalCoordinator::dispatch_next()`; os tempos PSG, YM2413 e VDP medem
o avanço de cada dispositivo após o dispatch. Portanto, são custos separados
do Scheduler, não tempos inclusivos para somar com `step_frame`. A duração
YM2413 continua usando o timer opt-in já existente. Ainda não há tempo exclusivo
do Z80, transações BUS, mixer ou resampler.

## Execução

Executar em PowerShell elevado, Windows x64, com working tree limpo e ROM
OutRun SMS obtida legalmente:

    .\scripts\benchmark\perf-profile-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms" -ThroughputCargoProfile release-llvm -ProfilingCargoProfile release-llvm-debug -WarmupSeconds 5 -CaptureSeconds 30 -DiagnosticSeconds 10

O runner realiza duas execuções:

1. Invoca perf-baseline-s0.ps1 sem profiler ativo. Esse é o resultado oficial
   de throughput.
2. Executa novamente o binário de teste já compilado sob perfis WPR CPU e Heap.
   Essa passagem serve somente para atribuição; seu FPS não é o resultado
   oficial de desempenho.

O heap profiling pode ser omitido com -SkipHeapProfile; a amostragem de CPU
permanece ativa. O rastreamento de heap é configurado para a imagem exata
perf_baseline-*.exe e desabilitado no bloco finally. A execução elevada é
necessária para configurar WPR heap tracing.

## Instrumentação S0.2

A linha `SUN_PERF_DIAGNOSTICS` exporta por cenário os últimos contadores por
quadro: `scheduler_dispatch_count_last`, `scheduler_dispatch_elapsed_ns_last`,
`psg_advance_calls_last`, `psg_advance_clock_cycles_last`,
`psg_advance_elapsed_ns_last`, `vdp_advance_calls_last`,
`vdp_advance_tstates_last` e `vdp_advance_elapsed_ns_last`.

Os contadores são acumuladores inteiros de tamanho fixo, reiniciados na
fronteira de cada frame de diagnóstico e exportados depois do passo. Fora do
modo diagnóstico, os timers não chamam `Instant::now()`. O teste
`sms_subsystem_timing.rs` compara o estado completo e o resultado da instrução
com profiling ligado/desligado, além de verificar os contadores dos três
limites instrumentados.

A medição é por chamada agregada do dispositivo, não por instrução, T-state,
scanline ou amostra individual. O contador de VDP contabiliza T-states entregues
ao VDP; não representa pixels ou dots. O contador de PSG registra clocks
entregues pelo Scheduler, não clocks de CPU inferidos.

## Artefatos

Cada campanha produz:

- manifest.json: SHA do código, branch, SHA-256 da ROM, toolchain, CPU, SO e
  configuração da captura.
- baseline/: resultados do runner S0 existente sem instrumentação.
- cpu-and-heap-profile.etl: amostras ETW de CPU e eventos de alocação heap
  quando o perfil heap está habilitado.
- profiled-run.log: saída da execução diagnóstica separada.
- profiled-throughput.csv: throughput da execução instrumentada, somente
  contexto diagnóstico.
- subsystem-diagnostics.csv: exportação CSV dos registros
  SUN_PERF_DIAGNOSTICS já existentes.

Abrir o ETL no Windows Performance Analyzer (WPA). Para CPU, usar CPU Usage
(Sampled), agrupando por Process / Thread / Stack e resolvendo símbolos Rust
com os PDB correspondentes ao build release-llvm-debug separado. O throughput
oficial usa release-llvm, preservando a comparação com o candidato ThinLTO.
Para heap,
inspecionar contagem/tamanho de alocações e pilha de alocação por processo.

## Regras de interpretação

- CPU sampled time é atribuição estatística, não temporização exata de parede.
- Eventos de heap representam atividade do heap Windows. Allocators customizados
  ou mapeamentos diretos de memória virtual podem exigir instrumentação própria.
- Timers aninhados se sobrepõem. Não somar step_frame, YM2413 e duração do
  worker como custos disjuntos.
- Call stacks atribuem amostras de CPU a funções, mas não fornecem, sozinhas,
  contagem de eventos ou tempo exclusivo por subsistema.
- Os diagnósticos atuais ainda não isolam Z80, Scheduler, barramento, PSG,
  VDP, mixer e resampler.
- A execução é headless e não mede CPAL físico, composição Slint, upload WGPU
  nem execução da GPU. Isso exige uma campanha integrada separada.

## Próximos incrementos de instrumentação

Continuar a instrumentação somente nos limites existentes do backend, sem
alterar ordem de execução ou transições de estado:

1. Z80: contagem de instruções/M-cycles/T-states e tempo de serviço do sequencer;
   não cronometrar cada instrução na campanha oficial de throughput.
2. Barramento SMS: leituras/escritas/WAIT e duração agregada das transações.
3. YM2413: contagem de clocks/slots (o tempo de avanço já é medido).
4. Áudio: quantidade de amostras de entrada/saída e duração de mixer/resampler.
5. Vídeo: bytes do framebuffer, bytes copiados/convertidos e contagens de
   publicação/consumo do mailbox.
6. Host: upload/submissão CPU WGPU e GPU timestamp queries; duração do callback
   CPAL, profundidade FIFO, underruns e overruns.

Contadores devem ser de tamanho fixo ou pré-alocados, desabilitados fora do
modo diagnóstico e exportados após a captura. Evitar eventos por instrução,
alocação no hot path, locks compartilhados com a thread de emulação ou qualquer
feedback para o tempo do MachineScheduler.

## Estado

O runner estabelece a primeira etapa reproduzível de CPU/heap profiling.
A instrumentação S0.2 cobre parcialmente Scheduler, PSG, YM2413 e VDP, mas não
atribui ainda o custo do Z80, BUS, mixer/resampler ou host. O profiling integrado
Slint/WGPU/CPAL permanece um incremento posterior de S0; nenhuma dessas
limitações é promovida a PASS sem execução real e análise dos artefatos.
