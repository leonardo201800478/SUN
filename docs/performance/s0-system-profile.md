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

Os diagnósticos atuais do worker medem percentis de intervalo de quadro,
step_frame, avanço YM2413, publicação de vídeo, preparação/submissão de áudio
e processamento agregado do quadro. São medições de etapas inclusivas, não
um perfil exclusivo e completo de cada dispositivo.

## Execução

Executar em PowerShell elevado, Windows x64, com working tree limpo e ROM
OutRun SMS obtida legalmente:

    .\scripts\benchmark\perf-profile-s0.ps1 -RomPath "D:\ROMS\Sega Master System\OutRun (Japan).sms" -CargoProfile release-llvm-debug -WarmupSeconds 5 -CaptureSeconds 30 -DiagnosticSeconds 10

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
com os PDB correspondentes ao mesmo build release-llvm-debug. Para heap,
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

Adicionar contadores somente nos limites existentes do backend, sem alterar
ordem de execução ou transições de estado:

1. Scheduler: quantidade de dispatches, avanços dos domínios de clock e tempo
   exclusivo de dispatch.
2. Z80: contagem de instruções/M-cycles/T-states e tempo exclusivo do sequencer;
   não cronometrar cada instrução na campanha oficial de throughput.
3. Barramento SMS: leituras/escritas/WAIT e duração agregada das transações.
4. PSG/YM2413: chamadas de avanço, clocks/slots e tempo exclusivo de serviço.
5. VDP: scanlines, trabalho de sprites e duração exclusiva da renderização.
6. Áudio: quantidade de amostras de entrada/saída e duração exclusiva de
   mixer/resampler.
7. Vídeo: bytes do framebuffer, bytes copiados/convertidos e contagens de
   publicação/consumo do mailbox.
8. Host: upload/submissão CPU WGPU e GPU timestamp queries; duração do callback
   CPAL, profundidade FIFO, underruns e overruns.

Contadores devem ser de tamanho fixo ou pré-alocados, desabilitados fora do
modo diagnóstico e exportados após a captura. Evitar eventos por instrução,
alocação no hot path, locks compartilhados com a thread de emulação ou qualquer
feedback para o tempo do MachineScheduler.

## Estado

Este runner estabelece a primeira etapa reproduzível de CPU/heap profiling.
Não afirma que todos os subsistemas já possuem atribuição exclusiva. Os
contadores por dispositivo e o profiling integrado Slint/WGPU/CPAL permanecem
incrementos posteriores de S0, não critérios de PASS desta primeira entrega.
