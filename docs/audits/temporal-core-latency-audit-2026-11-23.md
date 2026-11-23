# Auditoria de latência do `temporal_core` — barramento, RAM, VRAM, VDP, PSG e YM2413

**Data:** 2026-11-23
**Escopo:** `native/temporal_core`, `native/machine_scheduler` e os consumidores quentes
(`native/sms_core`, `native/video_core`, `native/sn76489`, `native/ym2413`)
**Baseline:** `main` em `207ccc4` (perf: reuse validated clock advance previews #1259)
**Método:** leitura integral do código + microbenchmarks isolados + medição em máquina real com ROM sintética
**Plataforma de medição:** VM Neoverse-N1 (ARM64, 6 vCPU), Rust 1.99.1, `--release`, thin LTO

---

## 1. Resumo executivo

A auditoria original identificou um volume elevado de eventos no SMS sintético: **5 despachos do
scheduler por opcode fetch** e aproximadamente **49.780 eventos por frame** no workload `jr $`.
Esse contador é uma observação do workload e do baseline, não uma constante universal para todos os
jogos.

Os dados históricos de latência foram obtidos por métodos diferentes. A janela instrumentada
estimou **137 ns por despacho** após descontar o custo de instrumentação; o microbenchmark de
decomposição reportou **158,5 ns**. Esses valores não são intercambiáveis nem devem ser multiplicados
para afirmar que o Scheduler consome 43% do frame. A revisão posterior da PR
[#1261](https://github.com/leonardo201800478/SUN/pull/1261) corrigiu a fronteira do harness para
medir **agendamento + dispatch**: no host ARM64, o baseline foi 202,2 ns/op e o caminho com buffer
do chamador 184,0 ns/op. O resultado end-to-end foi menor e ainda existe uma diferença de projeção
não reconciliada.

| Componente | Resultado observado | Natureza da evidência |
|---|---|---|
| Eventos por frame (`jr $`) | **49.780** | contador do workload sintético |
| Scheduler — janela instrumentada | **~137 ns/dispatch** | estimativa ajustada pelo custo de instrumentação |
| Scheduler — decomposição isolada | **158,5 ns/dispatch** | microbenchmark histórico, fronteira diferente |
| Scheduler — harness corrigido pré-P0 | **202,2 ns/op** | agendamento + dispatch, ARM64 |
| Scheduler — buffer do chamador P0 | **184,0 ns/op** | agendamento + dispatch, ARM64; 0 alocações |
| PSG por frame | **1,22 ms** | benchmark isolado com drenagem, 44,1/48 kHz |
| VDP `advance` por frame | **~0,5 ms** | estimativa instrumentada, não medição independente |
| Frame total (ROM sintética) | **13,6–16,3 ms** | `step_frame` Release, instrumentação desligada |

Os números não formam uma decomposição aditiva única: há fronteiras de medição distintas, componentes
estimados e possíveis sobreposições de trabalho. Por isso, esta auditoria não atribui uma fração
percentual do frame ao Scheduler nem publica uma projeção consolidada de frame time após otimizações.

**Conclusão sustentada:** o custo do caminho temporal merece investigação porque o scheduler é
invocado muitas vezes por frame. As latências de RAM/VRAM medidas em microbenchmarks são pequenas
comparadas ao round-trip de barramento e scheduler; isso não justifica reescrever memória sem
evidência de perfil real.

## 2. Como foi medido (e o que isso não prova)

- **Microbenchmarks**: laço fechado, `black_box` em entradas e saídas, mediana de 7–9 amostras,
  parâmetros de runtime passados por `black_box` para impedir que o LLVM transforme divisão por
  constante em multiplicação — o scheduler real recebe a razão de clock em runtime.
- **Máquina real**: `SmsTemporalMachine::new(rom, 3)` com ROM sintética de 16 KB
  (`jr $`, `ld ($C000),a` em laço, `ld a,(hl)` em laço, `out ($7F),a` em laço), 60 frames medidos
  após aquecimento, `master_ticks_per_tstate = 3`.
- **Contagem de eventos**: `set_subsystem_timing_enabled(true)` + `take_subsystem_timing()`.
  Atenção: essa instrumentação paga **2 × `Instant::now` por despacho** (~37 ns cada nesta VM), o que
  infla o frame de 15,9 ms para 26,7 ms. Por isso os números de tempo do scheduler foram obtidos
  subtraindo o custo do `Instant::now` medido separadamente (74,5 ns para 2 chamadas +
  `duration_since`), e o frame time honesto foi medido com instrumentação **desligada**.

Limites: os valores absolutos foram obtidos em VM ARM64 e não podem ser extrapolados para Windows
x64. A contagem de operações é válida para o workload sintético executado; a proporção de tempo por
subsistema depende da plataforma, compilador, perfil do jogo e condições de execução. Nenhuma ROM
comercial foi usada (o diretório `D:\ROMS\Sega Master System` não estava acessível ao ambiente de
medição). A validação com jogos reais permanece pendente e está descrita no §9.

---

## 3. O que o `temporal_core` faz hoje

### 3.1 Modelo

- `MachineTime(u128)` (`clock.rs:11`) — timestamp absoluto em ticks de mestre.
- `ClockDomain` (`clock.rs:244`) — razão racional `n/d`, timestamp, total de ciclos e fase.
  Conversão em `convert_elapsed` (`clock.rs:104-128`) com **duas divisões `u128`** por avanço.
- `ClockDomainAdvance` (`clock.rs:218`) — token de 128 bytes que carrega razão, tempo, total, fase de
  origem **e** o snapshot de destino; `apply_preview` revalida o token (`clock.rs:464-470`).
- `EventQueue` (`event.rs:137`) — `BinaryHeap` com chave `(MachineTime, HardwarePhase, PlatformPriority, u64)`
  construída em `QueueEntry::key()` (`event.rs:70-77`).

Layouts medidos: `MachineTime` 16 B, `ClockDomain` 96 B, `ClockDomainAdvance` **128 B**,
`ClockDomainSnapshot` 48 B, `ClockAdvance` do scheduler **192 B**.

### 3.2 O laço de despacho

`MachineScheduler::dispatch_next` no baseline `main@207ccc4` executava, por evento:

```text
next_timestamp()                      // peek no heap
calculate_advances(timestamp)
  Vec::with_capacity(clocks.len())    // alocação no baseline
  preview_advance por domínio
apply_advances(timestamp, &advances)
  preflight/validação dos previews
  commit dos avanços
pop_next()                            // pop do heap
```

**Estado posterior:** a PR [#1261](https://github.com/leonardo201800478/SUN/pull/1261) adiciona
`dispatch_next_into()` e um buffer reutilizável no `SmsTemporalBus`. O caminho legado
`dispatch_next()` permanece disponível; o caminho quente SMS evita a alocação até quatro
domínios. Acima dessa capacidade, o buffer faz spill lossless para heap.

### 3.3 Os três multiplicadores do custo

1. **Volume**: cada opcode fetch gera **5 eventos** — 4 transições de T-state do modelo de pinos M1
   (`Z80M1Phase::{T1,T2,Wait,T3,T4}` em `z80_cpu/src/m1.rs:12-19`, agendadas por
   `schedule_m1_tstate`, `sms_core/src/temporal.rs:226-241`) + 1 conclusão de barramento. Medido:
   `49.780 = 5 × 9.956` exatamente.
2. **Razões de clock**: os domínios SMS usam a mesma razão nominal
   `1/master_ticks_per_tstate` (`sms_core/src/temporal.rs:129-140`), mas isso, isoladamente, não
   prova que seus previews sejam intercambiáveis: timestamp, ciclos acumulados e fase também
   precisam coincidir. A tentativa de compartilhar previews foi medida posteriormente e rejeitada
   por regressão de desempenho (PR #1262).
3. **Granularidade de alocação no baseline**: `Vec::with_capacity(3)` e a construção dos tokens
   criavam uma alocação por dispatch. A PR #1261 substituiu esse transporte no caminho SMS por
   buffer reutilizável, com zero alocações no perfil de até quatro domínios.

---

## 4. Diagnóstico por subsistema

### 4.1 Scheduler / `temporal_core` — custo por evento, sem percentual de frame

A decomposição histórica de **158,5 ns** para três domínios pertence a um microbenchmark isolado.
Ela não tem a mesma fronteira do harness corrigido posteriormente (agendamento + dispatch) e não
deve ser usada para calcular a participação do Scheduler no frame.

| Parte | Custo | Frequência | Evidência |
|---|---|---|---|
| 3 × `preview_advance` (u128) | 48,2 ns | por despacho | `clock.rs:104-128`, `:411-433` |
| `Vec::with_capacity(3)` + 3 pushes de 128 B | 58,3 ns | por despacho | `machine_scheduler/src/lib.rs:585-595` |
| `BTreeMap` get + get_mut (6 buscas) | 8,6 ns | por despacho | `lib.rs:609-620` |
| `EventQueue` push + 2 pops | 26,9 ns | por evento | `event.rs:266-302` |
| 2 × `tracing::trace!` sem subscriber | 4,6 ns | por despacho | `lib.rs:447-455`, `:521-528` |
| Validação de preview | custo não isolado de forma confiável | por avanço | a validação faz parte do contrato de atomicidade |
| `edge_time_after` (gcd + u128) | 12,9 ns → 2,1 ns sem gcd | **fora do hot path observado** | `clock.rs:349-399` |

Os valores desta tabela são a decomposição histórica do microbenchmark, não uma contabilidade
precisa do custo do caminho end-to-end. A afirmação de que o compilador eliminaria toda a
revalidação foi removida: não há evidência suficiente para afirmá-la como regra geral.

Observações relevantes:

- `edge_time_after`/`next_edge_time` **não são chamados** pelo scheduler nem pelo SMS. A API existe,
  é cara (redução por `gcd` por chamada), mas hoje não pesa. Deve permanecer assim.
- `convert_elapsed` (`clock.rs:104-128`) faz `elapsed / denominator` **e** `elapsed % denominator`
  sobre `u128`. Com razão `1/3`, isso é uma divisão `u128` por valor de runtime, que no ARM64 vira
  chamada a `__udivti3`.
- `ClockDomainAdvance` carrega 4 campos de origem (`source_ratio`, `source_time`,
  `source_total_cycles`, `source_phase`) que existem **só** para detectar preview obsoleto. Numa
  arquitetura em que o scheduler calcula e aplica dentro da mesma seção crítica, esses campos são
  redundantes: um contador de geração de 32 bits faz o mesmo trabalho em 4 bytes em vez de 64.

**Comparação posterior com fronteira corrigida:** o harness de #1261 mede uma operação combinada
de agendamento + dispatch. No baseline ARM64, foram observados 202,2 ns/op; com buffer do chamador,
184,0 ns/op (−9,0%), sem alocação. Esses valores não substituem a decomposição acima: são uma
medição diferente e não devem ser somados às parcelas da tabela.

---

### 4.2 Barramento — round-trip temporal dominante; memória medida separadamente

Caminho de um acesso (`native/sms_core/src/temporal_bus.rs`):

```text
begin_request (:761-815)          → checa ocupação, grava instante
schedule_bus_completion (:868)    → u128::from(tstates) * u128::from(scale) + now
                                  → schedule_at_with_phase → BinaryHeap::push
dispatch_next (:868-952)          → scheduler.dispatch_next (baseline histórico)
                                  → advance PSG + advance VDP (por evento no baseline)
memory.read/write (:981-1002)     → 1 branch de mapa + 1 acesso indexado (~2 ns)
```

- `SmsMemory::read`/`write` são `#[inline]` (`memory.rs:284`, `:325`) e o caminho é direto: RAM é
  `[u8; 8192]` com espelhamento por máscara (`memory.rs:308-333`), VRAM é `Vec<u8>` de 16 KB indexado
  com `& 0x3FFF` (`video_core/src/sms/vdp.rs:886-900`). **Não há `BTreeMap`, `HashMap`, `Box<dyn>`,
  `format!` nem alocação por acesso.** O acesso à memória está bem feito.
- `BusTransaction` (`contract.rs:79-89`) usa `Option<u16>`/`Option<u8>` e valida no construtor
  (`contract.rs:91-139`), mas **só é construído por APIs de inspeção**
  (`temporal_machine.rs:777-828`); não está no caminho de execução. Correto.
- O que pesa é o **round-trip temporal**, não apenas o load/store. A estimativa histórica de
  ~164 ns por acesso não pode ser multiplicada por uma contagem genérica sem confirmar quantos
  acessos completam esse caminho no mesmo workload. Como referência aritmética, 10.000 operações
  × 164 ns equivalem a **1,64 ms**, não 8 ms. O tempo efetivo também depende dos avanços de PSG/VDP
  associados e da distribuição de eventos.
- `schedule_external_th_transition` (`temporal.rs:243-260`) faz `elapsed % u128::from(scale)` por
  chamada — só relevante quando há transições externas de TH (light gun/paddle).
- O clone do `Z80MicroSequencer` (96 B) antes de cada commit
  (`temporal_machine.rs:1081-1091`) custa 7,3 ns por ciclo de barramento. É o preço da atomicidade
  "valida no clone, depois instala"; aceitável, mas soma 0,07 ms/frame.

### 4.3 RAM e VRAM — latência já mínima; o gargalo é a arbitragem temporal

| Recurso | Modelo atual | Latência real de acesso | Risco |
|---|---|---|---|
| Work RAM 8 KB | `[u8; 8192]` + máscara de espelhamento | ~1–2 ns no microbenchmark | nenhum observado |
| VRAM 16 KB | `Vec<u8>` + `& 0x3FFF` | ~2 ns + branch no microbenchmark | indireção de `Vec` (um load extra) |
| CRAM 32 B | `[u8; 32]` + máscara de 6 bits | ~1 ns no microbenchmark | nenhum observado |
| Cartucho/SRAM | mapper com janelas | ~2–5 ns no microbenchmark | nenhum observado |

Essas latências são resultados de microbenchmarks em condições específicas; não representam latência
física de DRAM nem o custo total de uma transação em jogo.

Duas melhorias pequenas e seguras: trocar `Vec<u8>` da VRAM por `Box<[u8; 16384]>` (elimina o load do
ponteiro e permite ao LLVM provar o bound) e mover a seleção de modo (TMS9918 vs Mode 4) para um
`enum` comparado uma vez por scanline em vez de por byte.

O ponto conceitual importante: **o SUN não modela arbitragem dot-cycle** (`temporal_bus.rs:7-9`
declara isso explicitamente, e o VDP se descreve como modelo de scanline em `vdp.rs:1-3`). Portanto
não existe "latência de contenção" a otimizar em VRAM — existe latência de **quando** o efeito da
escrita se torna visível, e isso é decidido pelo timestamp do evento, não pela memória.

### 4.4 VDP — avanço e render

- No workload de baseline `jr # Auditoria de latência do `temporal_core` — barramento, RAM, VRAM, VDP, PSG e YM2413

**Data:** 2026-11-23
**Escopo:** `native/temporal_core`, `native/machine_scheduler` e os consumidores quentes
(`native/sms_core`, `native/video_core`, `native/sn76489`, `native/ym2413`)
**Baseline:** `main` em `207ccc4` (perf: reuse validated clock advance previews #1259)
**Método:** leitura integral do código + microbenchmarks isolados + medição em máquina real com ROM sintética
**Plataforma de medição:** VM Neoverse-N1 (ARM64, 6 vCPU), Rust 1.99.1, `--release`, thin LTO

---

## 1. Resumo executivo

A auditoria original identificou um volume elevado de eventos no SMS sintético: **5 despachos do
scheduler por opcode fetch** e aproximadamente **49.780 eventos por frame** no workload `jr $`.
Esse contador é uma observação do workload e do baseline, não uma constante universal para todos os
jogos.

Os dados históricos de latência foram obtidos por métodos diferentes. A janela instrumentada
estimou **137 ns por despacho** após descontar o custo de instrumentação; o microbenchmark de
decomposição reportou **158,5 ns**. Esses valores não são intercambiáveis nem devem ser multiplicados
para afirmar que o Scheduler consome 43% do frame. A revisão posterior da PR
[#1261](https://github.com/leonardo201800478/SUN/pull/1261) corrigiu a fronteira do harness para
medir **agendamento + dispatch**: no host ARM64, o baseline foi 202,2 ns/op e o caminho com buffer
do chamador 184,0 ns/op. O resultado end-to-end foi menor e ainda existe uma diferença de projeção
não reconciliada.

| Componente | Resultado observado | Natureza da evidência |
|---|---|---|
| Eventos por frame (`jr $`) | **49.780** | contador do workload sintético |
| Scheduler — janela instrumentada | **~137 ns/dispatch** | estimativa ajustada pelo custo de instrumentação |
| Scheduler — decomposição isolada | **158,5 ns/dispatch** | microbenchmark histórico, fronteira diferente |
| Scheduler — harness corrigido pré-P0 | **202,2 ns/op** | agendamento + dispatch, ARM64 |
| Scheduler — buffer do chamador P0 | **184,0 ns/op** | agendamento + dispatch, ARM64; 0 alocações |
| PSG por frame | **1,22 ms** | benchmark isolado com drenagem, 44,1/48 kHz |
| VDP `advance` por frame | **~0,5 ms** | estimativa instrumentada, não medição independente |
| Frame total (ROM sintética) | **13,6–16,3 ms** | `step_frame` Release, instrumentação desligada |

Os números não formam uma decomposição aditiva única: há fronteiras de medição distintas, componentes
estimados e possíveis sobreposições de trabalho. Por isso, esta auditoria não atribui uma fração
percentual do frame ao Scheduler nem publica uma projeção consolidada de frame time após otimizações.

**Conclusão sustentada:** o custo do caminho temporal merece investigação porque o scheduler é
invocado muitas vezes por frame. As latências de RAM/VRAM medidas em microbenchmarks são pequenas
comparadas ao round-trip de barramento e scheduler; isso não justifica reescrever memória sem
evidência de perfil real.

## 2. Como foi medido (e o que isso não prova)

- **Microbenchmarks**: laço fechado, `black_box` em entradas e saídas, mediana de 7–9 amostras,
  parâmetros de runtime passados por `black_box` para impedir que o LLVM transforme divisão por
  constante em multiplicação — o scheduler real recebe a razão de clock em runtime.
- **Máquina real**: `SmsTemporalMachine::new(rom, 3)` com ROM sintética de 16 KB
  (`jr $`, `ld ($C000),a` em laço, `ld a,(hl)` em laço, `out ($7F),a` em laço), 60 frames medidos
  após aquecimento, `master_ticks_per_tstate = 3`.
- **Contagem de eventos**: `set_subsystem_timing_enabled(true)` + `take_subsystem_timing()`.
  Atenção: essa instrumentação paga **2 × `Instant::now` por despacho** (~37 ns cada nesta VM), o que
  infla o frame de 15,9 ms para 26,7 ms. Por isso os números de tempo do scheduler foram obtidos
  subtraindo o custo do `Instant::now` medido separadamente (74,5 ns para 2 chamadas +
  `duration_since`), e o frame time honesto foi medido com instrumentação **desligada**.

Limites: os valores absolutos foram obtidos em VM ARM64 e não podem ser extrapolados para Windows
x64. A contagem de operações é válida para o workload sintético executado; a proporção de tempo por
subsistema depende da plataforma, compilador, perfil do jogo e condições de execução. Nenhuma ROM
comercial foi usada (o diretório `D:\ROMS\Sega Master System` não estava acessível ao ambiente de
medição). A validação com jogos reais permanece pendente e está descrita no §9.

---

## 3. O que o `temporal_core` faz hoje

### 3.1 Modelo

- `MachineTime(u128)` (`clock.rs:11`) — timestamp absoluto em ticks de mestre.
- `ClockDomain` (`clock.rs:244`) — razão racional `n/d`, timestamp, total de ciclos e fase.
  Conversão em `convert_elapsed` (`clock.rs:104-128`) com **duas divisões `u128`** por avanço.
- `ClockDomainAdvance` (`clock.rs:218`) — token de 128 bytes que carrega razão, tempo, total, fase de
  origem **e** o snapshot de destino; `apply_preview` revalida o token (`clock.rs:464-470`).
- `EventQueue` (`event.rs:137`) — `BinaryHeap` com chave `(MachineTime, HardwarePhase, PlatformPriority, u64)`
  construída em `QueueEntry::key()` (`event.rs:70-77`).

Layouts medidos: `MachineTime` 16 B, `ClockDomain` 96 B, `ClockDomainAdvance` **128 B**,
`ClockDomainSnapshot` 48 B, `ClockAdvance` do scheduler **192 B**.

### 3.2 O laço de despacho

`MachineScheduler::dispatch_next` no baseline `main@207ccc4` executava, por evento:

```text
next_timestamp()                      // peek no heap
calculate_advances(timestamp)
  Vec::with_capacity(clocks.len())    // alocação no baseline
  preview_advance por domínio
apply_advances(timestamp, &advances)
  preflight/validação dos previews
  commit dos avanços
pop_next()                            // pop do heap
```

**Estado posterior:** a PR [#1261](https://github.com/leonardo201800478/SUN/pull/1261) adiciona
`dispatch_next_into()` e um buffer reutilizável no `SmsTemporalBus`. O caminho legado
`dispatch_next()` permanece disponível; o caminho quente SMS evita a alocação até quatro
domínios. Acima dessa capacidade, o buffer faz spill lossless para heap.

### 3.3 Os três multiplicadores do custo

1. **Volume**: cada opcode fetch gera **5 eventos** — 4 transições de T-state do modelo de pinos M1
   (`Z80M1Phase::{T1,T2,Wait,T3,T4}` em `z80_cpu/src/m1.rs:12-19`, agendadas por
   `schedule_m1_tstate`, `sms_core/src/temporal.rs:226-241`) + 1 conclusão de barramento. Medido:
   `49.780 = 5 × 9.956` exatamente.
2. **Razões de clock**: os domínios SMS usam a mesma razão nominal
   `1/master_ticks_per_tstate` (`sms_core/src/temporal.rs:129-140`), mas isso, isoladamente, não
   prova que seus previews sejam intercambiáveis: timestamp, ciclos acumulados e fase também
   precisam coincidir. A tentativa de compartilhar previews foi medida posteriormente e rejeitada
   por regressão de desempenho (PR #1262).
3. **Granularidade de alocação no baseline**: `Vec::with_capacity(3)` e a construção dos tokens
   criavam uma alocação por dispatch. A PR #1261 substituiu esse transporte no caminho SMS por
   buffer reutilizável, com zero alocações no perfil de até quatro domínios.

---

## 4. Diagnóstico por subsistema

### 4.1 Scheduler / `temporal_core` — custo por evento, sem percentual de frame

A decomposição histórica de **158,5 ns** para três domínios pertence a um microbenchmark isolado.
Ela não tem a mesma fronteira do harness corrigido posteriormente (agendamento + dispatch) e não
deve ser usada para calcular a participação do Scheduler no frame.

| Parte | Custo | Frequência | Evidência |
|---|---|---|---|
| 3 × `preview_advance` (u128) | 48,2 ns | por despacho | `clock.rs:104-128`, `:411-433` |
| `Vec::with_capacity(3)` + 3 pushes de 128 B | 58,3 ns | por despacho | `machine_scheduler/src/lib.rs:585-595` |
| `BTreeMap` get + get_mut (6 buscas) | 8,6 ns | por despacho | `lib.rs:609-620` |
| `EventQueue` push + 2 pops | 26,9 ns | por evento | `event.rs:266-302` |
| 2 × `tracing::trace!` sem subscriber | 4,6 ns | por despacho | `lib.rs:447-455`, `:521-528` |
| Validação de preview | custo não isolado de forma confiável | por avanço | a validação faz parte do contrato de atomicidade |
| `edge_time_after` (gcd + u128) | 12,9 ns → 2,1 ns sem gcd | **fora do hot path observado** | `clock.rs:349-399` |

Os valores desta tabela são a decomposição histórica do microbenchmark, não uma contabilidade
precisa do custo do caminho end-to-end. A afirmação de que o compilador eliminaria toda a
revalidação foi removida: não há evidência suficiente para afirmá-la como regra geral.

Observações relevantes:

- `edge_time_after`/`next_edge_time` **não são chamados** pelo scheduler nem pelo SMS. A API existe,
  é cara (redução por `gcd` por chamada), mas hoje não pesa. Deve permanecer assim.
- `convert_elapsed` (`clock.rs:104-128`) faz `elapsed / denominator` **e** `elapsed % denominator`
  sobre `u128`. Com razão `1/3`, isso é uma divisão `u128` por valor de runtime, que no ARM64 vira
  chamada a `__udivti3`.
- `ClockDomainAdvance` carrega 4 campos de origem (`source_ratio`, `source_time`,
  `source_total_cycles`, `source_phase`) que existem **só** para detectar preview obsoleto. Numa
  arquitetura em que o scheduler calcula e aplica dentro da mesma seção crítica, esses campos são
  redundantes: um contador de geração de 32 bits faz o mesmo trabalho em 4 bytes em vez de 64.

**Comparação posterior com fronteira corrigida:** o harness de #1261 mede uma operação combinada
de agendamento + dispatch. No baseline ARM64, foram observados 202,2 ns/op; com buffer do chamador,
184,0 ns/op (−9,0%), sem alocação. Esses valores não substituem a decomposição acima: são uma
medição diferente e não devem ser somados às parcelas da tabela.

---

### 4.2 Barramento — round-trip temporal dominante; memória medida separadamente

Caminho de um acesso (`native/sms_core/src/temporal_bus.rs`):

```text
begin_request (:761-815)          → checa ocupação, grava instante
schedule_bus_completion (:868)    → u128::from(tstates) * u128::from(scale) + now
                                  → schedule_at_with_phase → BinaryHeap::push
dispatch_next (:868-952)          → scheduler.dispatch_next (baseline histórico)
                                  → advance PSG + advance VDP (por evento no baseline)
memory.read/write (:981-1002)     → 1 branch de mapa + 1 acesso indexado (~2 ns)
```

- `SmsMemory::read`/`write` são `#[inline]` (`memory.rs:284`, `:325`) e o caminho é direto: RAM é
  `[u8; 8192]` com espelhamento por máscara (`memory.rs:308-333`), VRAM é `Vec<u8>` de 16 KB indexado
  com `& 0x3FFF` (`video_core/src/sms/vdp.rs:886-900`). **Não há `BTreeMap`, `HashMap`, `Box<dyn>`,
  `format!` nem alocação por acesso.** O acesso à memória está bem feito.
- `BusTransaction` (`contract.rs:79-89`) usa `Option<u16>`/`Option<u8>` e valida no construtor
  (`contract.rs:91-139`), mas **só é construído por APIs de inspeção**
  (`temporal_machine.rs:777-828`); não está no caminho de execução. Correto.
- O que pesa é o **round-trip temporal**, não apenas o load/store. A estimativa histórica de
  ~164 ns por acesso não pode ser multiplicada por uma contagem genérica sem confirmar quantos
  acessos completam esse caminho no mesmo workload. Como referência aritmética, 10.000 operações
  × 164 ns equivalem a **1,64 ms**, não 8 ms. O tempo efetivo também depende dos avanços de PSG/VDP
  associados e da distribuição de eventos.
- `schedule_external_th_transition` (`temporal.rs:243-260`) faz `elapsed % u128::from(scale)` por
  chamada — só relevante quando há transições externas de TH (light gun/paddle).
- O clone do `Z80MicroSequencer` (96 B) antes de cada commit
  (`temporal_machine.rs:1081-1091`) custa 7,3 ns por ciclo de barramento. É o preço da atomicidade
  "valida no clone, depois instala"; aceitável, mas soma 0,07 ms/frame.

### 4.3 RAM e VRAM — latência já mínima; o gargalo é a arbitragem temporal

| Recurso | Modelo atual | Latência real de acesso | Risco |
|---|---|---|---|
| Work RAM 8 KB | `[u8; 8192]` + máscara de espelhamento | ~1–2 ns no microbenchmark | nenhum observado |
| VRAM 16 KB | `Vec<u8>` + `& 0x3FFF` | ~2 ns + branch no microbenchmark | indireção de `Vec` (um load extra) |
| CRAM 32 B | `[u8; 32]` + máscara de 6 bits | ~1 ns no microbenchmark | nenhum observado |
| Cartucho/SRAM | mapper com janelas | ~2–5 ns no microbenchmark | nenhum observado |

Essas latências são resultados de microbenchmarks em condições específicas; não representam latência
física de DRAM nem o custo total de uma transação em jogo.

Duas melhorias pequenas e seguras: trocar `Vec<u8>` da VRAM por `Box<[u8; 16384]>` (elimina o load do
ponteiro e permite ao LLVM provar o bound) e mover a seleção de modo (TMS9918 vs Mode 4) para um
`enum` comparado uma vez por scanline em vez de por byte.

O ponto conceitual importante: **o SUN não modela arbitragem dot-cycle** (`temporal_bus.rs:7-9`
declara isso explicitamente, e o VDP se descreve como modelo de scanline em `vdp.rs:1-3`). Portanto
não existe "latência de contenção" a otimizar em VRAM — existe latência de **quando** o efeito da
escrita se torna visível, e isso é decidido pelo timestamp do evento, não pela memória.

, `SmsVdp::advance` (`vdp.rs:573-605`) foi chamado
  aproximadamente **49.780 vezes por frame** e, em cada chamada,
  **constrói um array de 6–7 fronteiras**, itera, filtra e tira o mínimo (`:580-594`) antes de
  decidir o passo. O comportamento de não cruzar fronteira depende do perfil e do argumento recebido;
  não deve ser generalizado a todas as chamadas.
- `prime_sprite_line_buffer` (`:706-726`) sai cedo na maioria das chamadas — ok.
- Render: por scanline visível, `render_scanline_for_revision_with_evaluation` (`:821-846`) percorre
  256 pixels resolvendo name table e 4 bitplanes (`:1746-1776`), depois os sprites (`:1801-1865`).
  A avaliação de sprites aloca `Vec::with_capacity` **por linha** (`:1221`, e o caminho legado em
  `:1082`) → ~192 alocações por frame (~4 µs, pequeno, mas gratuito de remover com array fixo).
- Publicação de frame copia o buffer inteiro com `clone_from` (`:678-695`): 256 × 192 × 4 B = 196 KB
  por frame, mais `fill(0)` no wrap (`:874-880`). ~30–40 µs/frame. Dá para eliminar trocando os dois
  buffers (swap) em vez de copiar.

### 4.5 PSG (SN76489) — 1,22 ms/frame medidos

Medições diretas (`Sn76489Core::new(3_579_545, rate)`, drenagem de 735 amostras por frame):

| Configuração | ms/frame | ns por `advance_clocks(2)` |
|---|---|---|
| 44.100 Hz | **1,218** | 48,9 |
| 48.000 Hz | **1,217** | 48,9 |
| 44.100 Hz, geração de amostras desativada (experimento) | **0,118** | 4,7 |

- **O custo não está na divisão de clock nem no acumulador de fase.** Trocar `advance_output_phase`
  (`psg.rs:23-32`) de `u128` para `u64` não mudou nada (1,120 ms). Desligar a geração de amostras
  derrubou para 0,118 ms → **~1,0 ms/frame está na geração de amostras**.
- O FIR é o suspeito e a evidência fecha: `filtered_output` (`psg.rs:732-740`) faz
  `(self.filter_position + taps - 1 - index) % taps` **por tap**, com `taps` de runtime — uma divisão
  inteira por coeficiente. E é avaliado **duas vezes por tick** (`previous_filtered` em `:671-675` e
  `current_filtered` em `:688`), quando o valor "anterior" é exatamente o "atual" do tick anterior.
- O número de taps varia com a taxa (`design_lowpass`, `psg.rs:168-183`: 319 taps a 44,1 kHz, 63 a
  223 kHz, ~1.755 a 8 kHz), mas o **produto** `taps × amostras_por_tick` fica ≈ 63 em todas as taxas
  — exatamente o que explica o custo por tick ser constante (346–364 ns) na varredura de 8 kHz a
  223 kHz.
- A fila de amostras é `VecDeque<f64>` com caminho de overflow que loga `tracing::trace!` por amostra
  descartada (`psg.rs:701-710`). Num frontend que drena por frame isso não dispara; num consumidor que
  não drena, ele domina.

### 4.6 YM2413 — 0,16 ms/frame (apenas SMS japonês)

- `Ym2413Core` tem **160 bytes**; o clone transacional por slot (`ym2413/src/lib.rs:460-463`) custa
  10,8 ns, e há ~14.915 slots por frame → 0,16 ms. É barato e defensável (garante atomicidade do
  avanço por slot).
- A síntese em si é inteira e baseada em tabela (`operator.rs:179-220`), sem `f64` por operador. A
  normalização final divide uma vez por rotação de 18 slots (`temporal_bus.rs:1310-1319`), ou seja
  ~830 vezes por frame — irrelevante.
- O ganho aqui não está no núcleo FM; está em **não avançar o YM a cada evento do Z80**. Hoje o
  adaptador quebra o avanço em passos de 4 clocks (`temporal_bus.rs:1215-1238`), chamado por
  despacho. Com o domínio YM avançando por varredura, os 14.915 clones caem para ~262.

---

## 5. Plano de otimização e situação após as PRs de avaliação

Os números abaixo são hipóteses ou resultados de medições específicas; não são ganhos garantidos
para qualquer ROM/plataforma. A numeração P0/P1/P2/P3 nas PRs é a do plano de otimização do Scheduler
e não deve ser confundida com os itens de redução de eventos M1 descritos nesta auditoria.

### E1 — Reduzir o volume de eventos M1 (hipótese ainda não implementada)

**Hipótese:** consolidar transições T-state do M1 em um evento quando não houver WAIT armado,
mantendo o caminho detalhado quando WAIT exigir observabilidade por T-state.

- A contagem observada no workload `jr $` foi 49.780 eventos/frame; o alvo de ~10.000 é projeção,
  não resultado medido.
- **Risco:** alto para fidelidade temporal. Exige prova de amostragem do opcode em T3, WAIT real,
  TH, interrupções e ordem causal.
- **Situação:** não implementar sem evidência de que jogos não observam estados intermediários e sem
  testes direcionados de equivalência.

### E2 — Clocking por fronteira própria (hipótese estrutural)

PSG e VDP poderiam avançar apenas em suas próprias fronteiras ou quando acessados, desde que todos os
efeitos observáveis sejam preservados no timestamp correto.

- O ganho de remover conversões por despacho ainda não foi medido em uma implementação equivalente.
- **Risco:** alto para sincronização causal e visibilidade de escritas.
- **Situação:** pendente; não tratar como otimização de baixo risco.

### D1 — Custo do dispatch e conversões de clock

| Item | Evidência posterior | Situação |
|---|---|---|
| Buffer reutilizável no lugar de `Vec` por dispatch | PR #1261: 202,2 → 184,0 ns/op em agendamento + dispatch (−9,0%), zero alocações no caminho de até 4 domínios; ganho end-to-end de 1,0–3,3% nos workloads sintéticos | Implementado na branch da PR; CI passou; PR aberta |
| Compartilhar previews entre domínios equivalentes | PR #1262: +6,5% mais lento com 3 domínios e +14,3% com 4 | Rejeitado; protótipo removido |
| Guard para evitar revalidação redundante | PR #1263: +74,9% (3 domínios) e +68,0% (4) na comparação alternada | Rejeitado; P2 permanece pendente |
| Fast path checked para timestamps pequenos, mantendo `MachineTime(u128)` | PR #1264: −6,3%/−5,8% no harness e −2,1%/−1,3% nos workloads SMS headless em Linux x86_64 | Candidato P3; CI passou; PR aberta; Windows x64/ARM64 pendentes |
| Encolher `ClockDomainAdvance`, trocar `BTreeMap` ou remover tracing | Sem comparação isolada atual e pareada que justifique alteração | Não aprovado; medir antes de propor |

**Importante:** as medições das PRs ocorreram em hosts e configurações diferentes. Não somar percentuais,
subtrair números entre relatórios ou projetar novo frame time a partir deles. O ganho end-to-end de
#1261 também não reconcilia ainda com a projeção do harness; essa diferença está explicitamente
pendente no relatório P0.

### PSG-A — Otimizações do SN76489 (hipóteses pendentes)

1. Investigar o custo do índice circular do FIR e comparar alternativas sem alterar a resposta
   espectral.
2. Verificar se a amostra filtrada anterior pode ser reutilizada sem alterar a interpolação.
3. Só avaliar acumulador `u64` ou recíproca pré-calculada com prova de faixa e equivalência bit a bit
   ou dentro da tolerância de áudio definida.
4. Medir a interpolação e a fila de amostras em workloads com drenagem real.
5. Não substituir a fila por capacidade fixa sem provar comportamento de overflow/underrun.

A medição isolada original encontrou 1,22 ms/frame para a geração de áudio na configuração testada.
A projeção de 0,15 ms/frame ainda não foi validada e não deve ser apresentada como resultado.

### VDP-A — Otimizações do VDP (hipóteses pendentes)

1. Medir o custo de fronteiras em `SmsVdp::advance` por workload antes de cachear a próxima fronteira.
2. Avaliar armazenamento fixo para avaliação de sprites, preservando overflow e prioridade.
3. Avaliar troca de buffers somente após provar ownership e sincronização com o frontend.
4. Não trocar o tipo de VRAM sem medir impacto real e compatibilidade dos caminhos de save state.

O valor de ~0,5 ms/frame é uma estimativa instrumentada, não uma medição independente do VDP.

### CPU-A — CPU e barramento (hipóteses pendentes)

1. Qualquer alteração no commit de `Z80MicroSequencer` deve preservar atomicidade em falhas.
2. A aritmética de transições TH só deve mudar após teste de limites e medição no caminho efetivo.
3. Tornar traces opcionais somente se o custo tiver sido medido no perfil de produção e a observabilidade
   necessária aos jogos continuar intacta.

### Estrutural-A — Evolução futura do modelo temporal

1. Uma abstração `ClockLane` só é justificável se reduzir trabalho no caminho real sem introduzir
   custo de busca por dispatch nem misturar domínios com estado de origem diferente.
2. Domínios com fronteiras próprias exigem prova de que nenhum estado observável é pulado.
3. A representação pública de `MachineTime` permanece `u128`; o fast path local de #1264 não autoriza
   migrar a timeline global para `u64`.

## 6. O que **não** vale otimizar (e por quê)

- `edge_time_after` / `next_edge_time`: caros (12,9 ns, `gcd` + divisões) mas **não estão no caminho
  de execução**. Deixar como estão; só valem otimização se algum dia entrarem no laço.
- `BusTransaction` e validadores de contrato: fora do caminho de execução. Nenhuma mudança.
- Núcleo de síntese do YM2413: já é inteiro e tabelado; o clone por slot (10,8 ns) é o preço da
  atomicidade e vale mais que o ganho de removê-lo.
- Mapa de memória (`SmsMemory`): `#[inline]`, sem alocação, sem dispatch dinâmico. Não há o que
  ganhar em reescrever; o ganho está em chamá-lo menos vezes por evento.

---

## 7. Invariantes que as mudanças precisam preservar

1. **Determinismo**: ordem `(timestamp, hardware_phase, priority, sequence)` intacta; nenhuma
   otimização pode reordenar dois eventos no mesmo timestamp.
2. **Atomicidade multi-domínio**: ou todos os domínios avançam, ou nenhum. A proposta de encolher o
   token mantém isso computando todos os snapshots antes de qualquer commit (o token vira um índice
   de geração, não um portador de estado).
3. **Snapshot/restore**: `SCHEDULER_STATE_FORMAT_VERSION` + codec (`machine_scheduler/src/codec.rs`)
   precisam de migração; os testes `snapshot_replay.rs`, `multi_domain_replay.rs` e
   `stress_replay.rs` são a rede de segurança.
4. **WAIT real**: P0.a só é válida se o caminho com WAIT armado continuar por T-state.
5. **Frontend coarse-grained**: nada aqui muda a fronteira `start/pause/stop/reset/load_rom/step_frame`.

---

## 8. Evidência posterior e decisões das PRs

A hipótese original de conversão compartilhada não foi confirmada. As avaliações posteriores
separaram as alterações e corrigiram a fronteira dos benchmarks:

| PR | Alteração avaliada | Resultado | Decisão atual |
|---|---|---|---|
| [#1261](https://github.com/leonardo201800478/SUN/pull/1261) | Buffer reutilizável para os avanços do Scheduler | 202,2 → 184,0 ns/op (−9,0%) em agendamento + dispatch; zero alocações. End-to-end: −2,0% `relative-loop`, −2,4% bus-heavy, −3,3% read-heavy, −1,0% port-heavy | Mantida na branch; CI passou; PR aberta. A projeção do harness (~0,906 ms/frame) não foi reconciliada com o ganho end-to-end de 0,322 ms/frame no `relative-loop`. |
| [#1262](https://github.com/leonardo201800478/SUN/pull/1262) | Compartilhamento de previews equivalentes | +6,5% mais lento em 3 domínios e +14,3% em 4 | Rejeitada; nenhum código runtime mantido. |
| [#1263](https://github.com/leonardo201800478/SUN/pull/1263) | Guards para evitar revalidação no commit | +74,9% mais lento em 3 domínios e +68,0% em 4 na comparação alternada | Rejeitada; nenhum código runtime mantido. |
| [#1264](https://github.com/leonardo201800478/SUN/pull/1264) | Fast path de `checked_duration_since()`, preservando `MachineTime(u128)` | −6,3%/−5,8% no harness; −2,1%/−1,3% nos workloads SMS headless, em Linux x86_64 | Candidato P3; CI passou. PR aberta; Windows x64 e ARM64 não medidos. |

Os valores de #1261 foram obtidos em ARM64/Neoverse-N1; os de #1264 em Linux x86_64 com processo
fixado na CPU. Não comparar diretamente as latências absolutas entre esses hosts. As porcentagens
são resultados de seus respectivos experimentos, não ganhos cumulativos.

Os testes e validações específicos estão registrados nos relatórios vinculados em cada PR. Nenhuma
dessas evidências autoriza remover validações, alterar a ordem temporal ou assumir ganhos em jogos
comerciais sem teste real.

## 9. Próximos passos sugeridos

1. **Validar com jogos reais em duas passagens separadas.** Primeiro, executar com contador de
   dispatches habilitado para registrar `dispatches/frame`, perfil do sistema e comportamento de WAIT.
   Depois, repetir com a instrumentação temporal desligada para medir frame time; não usar os tempos
   da execução instrumentada como FPS ou latência real.
2. **Validar em Windows x64**, plataforma inicial do frontend, e repetir em ARM64 quando houver
   ambiente disponível. Não extrapolar proporções de um host para outro.
3. **Manter a ordem de prioridade baseada em evidência:** P0 buffer do chamador (#1261) e P3 fast
   path local (#1264) permanecem candidatos nas respectivas PRs; compartilhamento de previews (#1262)
   e guards de commit (#1263) foram rejeitados.
4. **Não iniciar redução de eventos M1 ou clocking preguiçoso** sem prova explícita de WAIT real,
   TH, interrupções, visibilidade de efeitos e determinismo.
5. **Para PSG/VDP**, medir isoladamente e em workload de máquina antes de alterar buffers, FIR,
   fronteiras ou sincronização.

---

## Apêndice A — Como reproduzir as medições

O harness usado nesta auditoria é um crate separado (não faz parte do workspace) com dependências de
caminho para `sega-temporal-core`, `sega-machine-scheduler`, `sega-sms-core`, `sega-z80-cpu`,
`sega-sn76489` e `sega-ym2413`, e `rust-toolchain.toml` fixando 1.99.1. Binários:

| Binário | O que mede |
|---|---|
| `micro` | `advance_to`, `preview_advance`, `EventQueue` push/pop, `edge_time_after`, `tracing::trace!`, alocação de `Vec` |
| `decompose` | cada componente de um despacho de 3 domínios |
| `sched` | `MachineScheduler` real vs protótipo no mesmo fluxo de eventos |
| `machine` | `SmsTemporalMachine` real: frame time, `dispatches/frame`, `bus-ops/frame`, subsistemas |
| `psg_frame` | custo do PSG por frame com drenagem |
| `audio` | clone do YM2413 por slot, custo por tick do PSG |
| `div`, `clone` | `u128` vs `u64` na divisão, clone do `Z80MicroSequencer` |

Para contar eventos, `set_subsystem_timing_enabled(true)` adiciona 2 × `Instant::now` por
despacho. Use essa execução somente para contagem/instrumentação; faça outra execução com a
instrumentação desligada para medir frame time.

## Apêndice B — Números de referência

```
SMS NTSC, master_ticks_per_tstate = 3
  T-states/frame .............. 59.736
  eventos/frame (jr $) ........ 49.780  = 5 × 9.956 fetches
  eventos/frame (bus-heavy) ... 40.621
  eventos/frame (read-heavy) .. 50.543
  ticks internos do PSG/frame .. 3.733   (59.736 / 16)
  slots YM2413/frame .......... 14.915   (só SMS japonês)
  amostras PSG/frame .......... 735 @ 44,1 kHz / 800 @ 48 kHz
  orçamento a 60 FPS .......... 16,667 ms
```
