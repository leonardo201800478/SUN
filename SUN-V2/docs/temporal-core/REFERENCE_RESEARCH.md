# Temporal Core — pesquisa comparativa inicial

**Status:** comparação de referências concluída para R0. Não constitui decisão arquitetural nem autorização para implementação.

## 1. SMS: requisitos temporais que afetam gameplay

A documentação técnica de Charles MacDonald descreve:
- H/V counters como elementos básicos do VDP;
- H counter incrementado por dot clock;
- V counter incrementado ao fim da scanline;
- V counter legível continuamente;
- H counter associado à transição do pino TH, que captura seu valor;
- 262 linhas por frame NTSC e 313 por frame PAL;
- interrupções de frame e linha com flags persistentes até leitura da porta de controle.

**Limitação importante da fonte:** o próprio documento registra que não possuía informação suficiente sobre o ponto exato, dentro de uma scanline, em que ocorrem alguns eventos (line IRQ, frame IRQ e incremento do V counter). Portanto, a documentação é referência de base, não especificação completa de microtiming. Esses pontos precisam de confirmação por documentação complementar, testes de hardware e ROMs de diagnóstico.

Referência: [Sega Master System VDP documentation — Charles MacDonald](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt).

## 2. Modelos encontrados nos projetos de referência

### 2.1 MAME — timeslices, quanta e timers

Inspeção de `src/emu/schedule.cpp` e `schedule.h`:
- O scheduler define um alvo temporal a partir do quantum atual.
- O próximo timer pode reduzir esse alvo.
- Os dispositivos executáveis avançam até o alvo usando seu tempo local e ciclos por segundo.
- Timers vencidos são executados no limite temporal.
- O quantum pode ser ajustado dinamicamente para melhorar o interleave.
- Timers guardam índice para preservar ordenação entre timers com mesmo instante em operações de restauração.

**Leitura para o SUN:** MAME demonstra um modelo de execução em fatias temporais com timers e ajuste de interleave. Não é simplesmente uma thread de sistema operacional por chip. É uma referência para estudar limites de quantum, timers, ordenação e persistência.

Fontes:
- [MAME schedule.cpp](https://github.com/mamedev/mame/blob/master/src/emu/schedule.cpp)
- [MAME schedule.h](https://github.com/mamedev/mame/blob/master/src/emu/schedule.h)

### 2.2 ares — corrotinas cooperativas por componente

Inspeção de `ares/ares/scheduler/scheduler.cpp`, `thread.cpp`, e do runtime SMS:
- Cada Thread mantém um contador de clock e um escalar de conversão.
- `Thread::step(clocks)` avança seu contador.
- `Thread::synchronize()` faz o componente ceder até os demais alcançarem seu tempo.
- O Scheduler usa IDs estáveis para desempatar componentes no mesmo clock.
- CPU, VDP e PSG do SMS executam em contextos cooperativos e sincronizam durante o avanço.
- No VDP SMS inspecionado, o contador horizontal avança em clocks individuais e a sincronização com a CPU ocorre durante esse avanço.

**Leitura para o SUN:** modelo útil para investigar execução por corrotinas e pontos de sincronização explícitos. Essas corrotinas não significam automaticamente threads nativas em paralelo. O custo de troca de contexto, granularidade de sincronização e complexidade de serialização devem ser medidos.

Fontes:
- [ares scheduler.cpp](https://github.com/ares-emulator/ares/blob/master/ares/ares/scheduler/scheduler.cpp)
- [ares thread.cpp](https://github.com/ares-emulator/ares/blob/master/ares/ares/scheduler/thread.cpp)
- [ares SMS CPU](https://github.com/ares-emulator/ares/blob/master/ares/ms/cpu/cpu.cpp)
- [ares SMS VDP](https://github.com/ares-emulator/ares/blob/master/ares/ms/vdp/vdp.cpp)
- [ares SMS PSG](https://github.com/ares-emulator/ares/blob/master/ares/ms/psg/psg.cpp)

### 2.3 Gearsystem — CPU em passos curtos e avanço dos dispositivos

Inspeção de `GearsystemCore.h`, `GearsystemCore.cpp` e `Video.cpp`:
- O loop principal chama `Processor::RunFor(1)`, recebe a quantidade efetiva de T-states e atualiza o contador mestre.
- **Correção importante após inspeção de `Processor::RunFor`:** o argumento `1` é um orçamento mínimo, não um passo exato de um T-state. A função executa operações completas (`ExecuteOPCode`) e soma seus T-states até satisfazer o orçamento; portanto, uma chamada pode consumir vários T-states.
- Em seguida, o loop externo entrega o avanço acumulado ao VDP e ao áudio.
- Isso caracteriza uma execução CPU-led com fronteiras de retorno por instrução, não lockstep CPU/VDP a cada T-state. A sincronização exata de cada acesso de barramento precisa ser verificada nos callbacks de memória/I/O separadamente; não deve ser inferida apenas pelo loop externo.
- O VDP mantém contador de ciclos da linha e testa fronteiras internas para VINT, HINT, scroll latch, VCounter, flags, colisão, overflow e renderização.
- A ordem e os limites dessas fronteiras são explícitos no código.

**Leitura para o SUN:** demonstra uma abordagem CPU-led com atualização dos dispositivos por blocos delimitados por instrução. É um baseline importante para comparar simplicidade e custo, mas não prova que deadlines sub-instrução sejam atendidos. Precisamos medir o custo e auditar os callbacks de barramento antes de caracterizar sua precisão temporal.

Fontes:
- [Processor.cpp — implementação de RunFor](https://github.com/drhelius/Gearsystem/blob/master/src/Processor.cpp)
- [GearsystemCore.h](https://github.com/drhelius/Gearsystem/blob/master/src/GearsystemCore.h)
- [GearsystemCore.cpp](https://github.com/drhelius/Gearsystem/blob/master/src/GearsystemCore.cpp)
- [Video.cpp](https://github.com/drhelius/Gearsystem/blob/master/src/Video.cpp)
- [Processor.h](https://github.com/drhelius/Gearsystem/blob/master/src/Processor.h)

### 2.4 BlastEm — sincronização sob demanda

Inspeção de `sms.c`, `system.c`, `vdp.c` e documentação:
- O loop SMS executa o Z80 até um alvo futuro.
- Depois, avança VDP e PSG até o timestamp/ciclo atual do Z80.
- Acessos a portas sincronizam o dispositivo relevante antes de efetivar a operação.
- A documentação do BlastEm descreve sync-on-demand e um limite configurável de sincronização para forçar sincronização global periódica.

**Leitura para o SUN:** abordagem potencialmente eficiente quando os dispositivos podem avançar em blocos e sincronizar apenas em pontos observáveis. O risco é deixar a janela de dessincronização grande demais. A documentação citada sobre `max_cycles` discute principalmente o sistema Genesis e menciona uma limitação de sincronização M68K/Z80; não deve ser transposta diretamente para o SMS.

Fontes:
- [BlastEm SMS implementation](https://github.com/retrorepair/blastem/blob/master/sms.c)
- [BlastEm VDP implementation](https://github.com/retrorepair/blastem/blob/master/vdp.c)
- [BlastEm clock/synchronization notes](https://github.com/retrorepair/blastem/blob/master/README)

### 2.5 Genesis Plus GX — execução por linhas e sincronização dos chips

Inspeção de `core/system.c`:
- O modo SMS mantém uma contagem comum de ciclos do VDP.
- A CPU Z80 é executada até limites de linha.
- O tratamento de HINT considera a posição do ciclo da CPU para definir o momento em que a linha de interrupção é percebida.
- A documentação do projeto declara sincronização dos chips em uma base de clock comum e saída de áudio reamostrada.

**Leitura para o SUN:** referência adicional para sincronização por linha e tratamento de interrupções. Não é suficiente para concluir que um modelo por scanline atende todos os acessos sublinha do SMS.

Fontes:
- [Genesis Plus GX system.c](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/system.c)
- [Genesis Plus GX features](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/wiki/Features.md)

## 2.6 Snapshots de referência externa consultados em 2026-11-24

Os caminhos de código abaixo foram inspecionados nos seguintes HEADs, para tornar a comparação reproduzível. São snapshots de implementação, não fontes primárias de hardware:

| Projeto | HEAD consultado | Escopo usado na comparação |
|---|---|---|
| MAME | e37f88658ed84e0a07f84bdccbeed3bfd58c8eb1 | scheduler/timers e perfil SMS |
| ares | d97dba2d661946bbb203a5de55ee19f60b3a23d6 | scheduler cooperativo e runtime SMS |
| Gearsystem | 2fdd8f7c8ff3ae9523dafb7f1d8125776e96aae6 | CPU-led RunFor e avanço de dispositivos |
| BlastEm | 8d9e43ee50f7fb887236777d7349b8f9757f054d | sync-on-demand e caminho SMS |
| Genesis Plus GX | 2499cd3e3d1d8b7d0130d8b2825adb4cc608b2c0 | execução SMS por linha e IRQ |

## 3. Comparação preliminar

| Modelo | Vantagem potencial | Risco/custo a medir |
|---|---|---|
| Passo curto CPU-led | Simples; fronteira frequente entre CPU e dispositivos | Overhead de chamadas e custo por T-state |
| Eventos discretos globais | Executa apenas nas próximas fronteiras relevantes | Fila, ordenação, inserção/cancelamento e deadlines dinâmicos |
| Quanta/timeslices | Amortiza custo e controla interleave | Atraso entre dispositivos e tamanho do quantum |
| Corrotinas cooperativas | Modela cada componente com fluxo próprio e pontos explícitos de yield | Custo de troca, stack/contexto, estado de corrotina e snapshot |
| Sync-on-demand | Evita avançar dispositivos sem necessidade imediata | Sincronização tardia, deadlines não observados e dívida temporal |
| Híbrido | Pode combinar blocos baratos e sincronização exata em fronteiras observáveis | Complexidade de contratos e risco de múltiplos caminhos inconsistentes |

Nenhum modelo está escolhido. O Temporal Core deverá ser desenhado após protótipos comparáveis.

## 4. Hipóteses de desenho que precisam de prova

1. Um dispositivo pode avançar em lote somente até o próximo ponto em que seu estado possa afetar outro dispositivo ou ser observado pelo software.
2. Um acesso de barramento precisa sincronizar o dispositivo de destino até o timestamp correto antes de aplicar a leitura/escrita.
3. CPU não pode ultrapassar uma fronteira temporal que altere o resultado de uma leitura, IRQ, WAIT ou operação pendente.
4. Eventos com timestamp igual exigem ordenação determinística definida pelo contrato da máquina, não por ordem acidental de alocação de memória.
5. O Temporal Core precisa representar fase residual de clocks, inclusive quando a conversão entre domínios não for uma divisão inteira.
6. Snapshot precisa incluir todos os dados temporais que alteram a continuação: relógio, fases, eventos pendentes, desempate e transações suspensas.
7. Trabalho de áudio/vídeo do host não deve avançar nem retroceder o tempo virtual.

## 5. Questões ainda abertas

- A base temporal será um clock mestre inteiro, uma representação racional por domínio ou tempo de ponto fixo?
- O scheduler terá fila global de eventos, agenda de deadlines por dispositivo ou combinação?
- Como expressar fases e prioridades de eventos sem codificar detalhes de SMS no núcleo genérico?
- Como a CPU informa que alcançou um limite sem concluir uma instrução inteira?
- Como um ciclo de barramento pode ser suspenso por WAIT e retomado sem duplicar efeitos?
- Quais deadlines precisam ser globais e quais podem ser processados internamente por um dispositivo?
- Qual política impede starvation e eventos de mesmo timestamp que geram novos eventos infinitamente?
- Qual estrutura de dados é mais barata para a quantidade e o padrão real de eventos do SMS?

## 6. Próxima rodada de pesquisa

1. Estudar a documentação primária do Z80 para bus cycles, WAIT, M1, INT/NMI e amostragem.
2. Estudar os clocks reais SMS NTSC/PAL e as relações entre CPU, VDP, PSG e YM2413.
3. Levantar ROMs de teste que observem H/V counters, IRQs, portas e acessos de memória.
4. Inspecionar o modelo de eventos do MAME e os caminhos SMS do Ares/Gearsystem/BlastEm em nível de chamada e timestamp.
5. Criar modelos de referência matemáticos para clock domains e ordenação.
6. Definir microbenchmarks comparáveis antes de prototipar estruturas de produção.

## 7. Conclusão da pesquisa comparativa

A pesquisa inicial mostra que emuladores consolidados usam estratégias diferentes: quanta e timers, corrotinas cooperativas, passos curtos CPU-led e sincronização sob demanda. Isso reforça que o SUN não deve escolher um modelo por preferência estética.

O critério de escolha será: correção temporal demonstrável, capacidade de suspender/retomar operações, determinismo, custo mensurado e simplicidade suficiente para ser auditável.

**Nenhuma API, estrutura de fila, tipo de timestamp ou granularidade de execução foi aprovada em R0.** A seleção permanece para R1–R3.
