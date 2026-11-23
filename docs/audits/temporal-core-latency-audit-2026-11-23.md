# Auditoria de latência do \`temporal_core\` — barramento, RAM, VRAM, VDP, PSG e YM2413

**Data:** 2026-11-23  
**Escopo:** \`native/temporal_core\`, \`native/machine_scheduler\` e consumidores em \`sms_core\`, \`video_core\`, \`sn76489\` e \`ym2413\`.  
**Baseline:** \`main@207ccc4\` (antes da implementação P0 da PR #1261).  
**Método:** leitura de código, microbenchmarks e execução headless com ROMs sintéticas.  
**Plataforma da auditoria original:** VM Neoverse-N1 (ARM64, 6 vCPU), Rust 1.99.1, Release, thin LTO.

---

## 1. Resumo executivo

A auditoria original identificou um volume elevado de eventos no SMS sintético: **5 despachos do
scheduler por opcode fetch** e aproximadamente **49.780 eventos por frame** no workload \`jr $\`.
Esse contador é uma observação desse workload e do baseline, não uma constante universal para todos
os jogos.

Os dados históricos de latência foram obtidos por métodos diferentes. A janela instrumentada
estimou **137 ns por despacho** após descontar o custo da instrumentação; o microbenchmark de
decomposição reportou **158,5 ns**. Esses valores não são intercambiáveis nem devem ser multiplicados
para afirmar que o Scheduler consome 43% do frame.

A revisão posterior da PR [#1261](https://github.com/leonardo201800478/SUN/pull/1261) corrigiu a
fronteira do harness para medir **agendamento + dispatch**. No host ARM64, o baseline medido foi
202,2 ns/op e o caminho com buffer do chamador foi 184,0 ns/op. O resultado end-to-end foi menor e
ainda existe uma diferença de projeção não reconciliada.

| Componente | Resultado observado | Natureza da evidência |
|---|---:|---|
| Eventos por frame (\`jr $\`) | **49.780** | Contador do workload sintético |
| Scheduler — janela instrumentada | **~137 ns/dispatch** | Estimativa ajustada pelo custo da instrumentação |
| Scheduler — decomposição isolada | **158,5 ns/dispatch** | Microbenchmark histórico, fronteira diferente |
| Scheduler — harness corrigido pré-P0 | **202,2 ns/op** | Agendamento + dispatch, ARM64 |
| Scheduler — buffer do chamador P0 | **184,0 ns/op** | Agendamento + dispatch, ARM64; zero alocações |
| PSG por frame | **1,22 ms** | Benchmark isolado com drenagem, 44,1/48 kHz |
| VDP \`advance\` por frame | **~0,5 ms** | Estimativa instrumentada, não medição independente |
| Frame total (ROM sintética) | **13,6–16,3 ms** | \`step_frame\` Release, instrumentação desligada |

Os números não formam uma decomposição aditiva única: há fronteiras de medição distintas,
componentes estimados e possíveis sobreposições de trabalho. Por isso, esta auditoria não atribui
uma fração percentual do frame ao Scheduler nem publica uma projeção consolidada de frame time após
otimizações.

**Conclusão sustentada:** o custo do caminho temporal merece investigação porque o scheduler é
invocado muitas vezes por frame. As latências de RAM/VRAM medidas em microbenchmarks são pequenas
comparadas ao round-trip de barramento e scheduler; isso não justifica reescrever memória sem
evidência de perfil real.

---

## 2. Como foi medido (e o que isso não prova)

- **Microbenchmarks:** laço fechado, \`black_box\` em entradas e saídas, mediana de 7–9 amostras.
  Parâmetros de runtime foram usados para evitar que o LLVM transforme operações variáveis em
  operações constantes.
- **Máquina:** \`SmsTemporalMachine::new(rom, 3)\` com ROM sintética de 16 KB (\`jr $\`,
  \`ld ($C000),a\`, \`ld a,(hl)\` e \`out ($7F),a\` em laços), 60 frames após aquecimento e
  \`master_ticks_per_tstate = 3\`.
- **Contagem de eventos:** \`set_subsystem_timing_enabled(true)\` + \`take_subsystem_timing()\`.
  Essa instrumentação paga **2 × \`Instant::now\` por despacho** (~37 ns cada nesta VM) e elevou
  o frame observado de 15,9 ms para 26,7 ms. O frame time foi medido separadamente, com
  instrumentação desligada.
- **Ajuste histórico:** a estimativa de 137 ns subtraiu o custo medido de duas chamadas a
  \`Instant::now\` e \`duration_since\` (74,5 ns). Trata-se de uma estimativa ajustada, não de uma
  medição direta isolada do dispatch.

Os valores absolutos foram obtidos em VM ARM64 e não podem ser extrapolados para Windows x64. A
contagem de operações vale para o workload sintético executado; a proporção de tempo por subsistema
depende da plataforma, compilador, perfil do jogo e condições de execução. Nenhuma ROM comercial
foi usada. A validação com jogos reais permanece pendente e está descrita na seção 9.

---

## 3. Modelo temporal e caminho de despacho no baseline

### 3.1 Modelo

- \`MachineTime(u128)\`: timestamp absoluto em ticks de mestre, com unidade definida pelo perfil.
- \`ClockDomain\`: razão racional \`n/d\`, timestamp, total de ciclos e fase.
- \`ClockDomainAdvance\`: token de preview que carrega os dados de origem necessários para detectar
  um preview obsoleto e o snapshot de destino.
- \`EventQueue\`: \`BinaryHeap\` ordenado por
  \`(MachineTime, HardwarePhase, PlatformPriority, sequence)\`.

Layouts medidos no baseline: \`MachineTime\` 16 B, \`ClockDomain\` 96 B,
\`ClockDomainAdvance\` 128 B, \`ClockDomainSnapshot\` 48 B e \`ClockAdvance\` do Scheduler 192 B.

### 3.2 Despacho no baseline \`main@207ccc4\`

O caminho histórico de \`MachineScheduler::dispatch_next\` executava, por evento:

\`\`\`text
next_timestamp()                      // peek no heap
calculate_advances(timestamp)
  Vec::with_capacity(clocks.len())    // alocação no baseline
  preview_advance por domínio
apply_advances(timestamp, &advances)
  preflight/validação dos previews
  commit dos avanços
pop_next()                            // pop do heap
\`\`\`

**Estado posterior:** a PR [#1261](https://github.com/leonardo201800478/SUN/pull/1261) adiciona
\`dispatch_next_into()\` e um buffer reutilizável no \`SmsTemporalBus\`. O caminho legado
\`dispatch_next()\` permanece disponível; o caminho quente SMS evita alocação até quatro domínios.
Acima dessa capacidade, o buffer faz spill lossless para heap.

### 3.3 Multiplicadores de custo observados

1. **Volume:** cada opcode fetch do workload de referência gerou cinco eventos — quatro transições
   do M1 e uma conclusão de barramento. A contagem observada foi \`49.780 = 5 × 9.956\`.
2. **Razões de clock:** os domínios SMS usam a mesma razão nominal
   \`1/master_ticks_per_tstate\`, mas isso, isoladamente, não prova que seus previews sejam
   intercambiáveis. Timestamp, ciclos acumulados e fase também precisam coincidir. A tentativa de
   compartilhar previews foi medida posteriormente e rejeitada por regressão de desempenho (PR #1262).
3. **Alocação:** no baseline, \`Vec::with_capacity(3)\` e a construção dos tokens criavam uma
   alocação por dispatch. A PR #1261 substituiu esse transporte no caminho SMS por buffer
   reutilizável, com zero alocações no perfil de até quatro domínios.

---

## 4. Diagnóstico por subsistema

### 4.1 Scheduler / \`temporal_core\`

A decomposição histórica de **158,5 ns** para três domínios pertence a um microbenchmark isolado.
Ela não tem a mesma fronteira do harness corrigido posteriormente (agendamento + dispatch) e não
deve ser usada para calcular a participação do Scheduler no frame.

| Parte do microbenchmark histórico | Custo reportado | Observação |
|---|---:|---|
| 3 × \`preview_advance\` | 48,2 ns | Conversão racional por domínio |
| \`Vec::with_capacity(3)\` + 3 tokens | 58,3 ns | Alocação/cópia no baseline |
| Buscas \`BTreeMap\` | 8,6 ns | Seis operações reportadas |
| \`EventQueue\` push/pop | 26,9 ns | Operações da fila |
| \`tracing::trace!\` | 4,6 ns | Sem subscriber no microbenchmark |
| \`edge_time_after\` | 12,9 ns → 2,1 ns sem GCD | Fora do hot path observado |

Os valores são a decomposição histórica do microbenchmark, não uma contabilidade precisa do custo
end-to-end. A afirmação de que o compilador eliminaria toda a revalidação foi removida: não há
evidência suficiente para afirmá-la como regra geral.

**Comparação posterior com fronteira corrigida:** o harness de #1261 mede uma operação combinada
de agendamento + dispatch. No baseline ARM64, foram observados 202,2 ns/op; com buffer do chamador,
184,0 ns/op (−9,0%), sem alocação. Esses valores não substituem a decomposição acima e não devem
ser somados às parcelas da tabela.

### 4.2 Barramento — round-trip temporal dominante; memória medida separadamente

Caminho histórico de um acesso em \`native/sms_core/src/temporal_bus.rs\`:

\`\`\`text
begin_request                     → checa ocupação e registra instante
schedule_bus_completion           → agenda conclusão na EventQueue
dispatch_next                     → Scheduler + avanço dos dispositivos no baseline
memory.read/write                 → acesso ao mapa de memória
\`\`\`

- \`SmsMemory::read\`/\`write\` são \`#[inline]\`. Work RAM usa array de 8 KB com espelhamento por
  máscara; VRAM usa buffer de 16 KB indexado com \`& 0x3FFF\`. O caminho não usa dispatch dinâmico
  nem alocação por acesso.
- \`BusTransaction\` é construído por APIs de inspeção, não pelo caminho de execução descrito.
- O que pesa é o **round-trip temporal**, não apenas o load/store. A estimativa histórica de
  ~164 ns por acesso não pode ser multiplicada por uma contagem genérica sem confirmar quantos
  acessos completam esse caminho no mesmo workload. Como referência aritmética, 10.000 operações
  × 164 ns equivalem a **1,64 ms**, não 8 ms. O tempo efetivo também depende dos avanços de PSG/VDP
  associados e da distribuição de eventos.
- \`schedule_external_th_transition\` usa aritmética modular para localizar transições externas TH;
  o impacto precisa ser medido em workload que efetivamente use esses dispositivos.
- O clone do \`Z80MicroSequencer\` (96 B) antes de cada commit foi medido em 7,3 ns por ciclo de
  barramento, aproximadamente 0,07 ms para 9.956 ciclos. O custo é associado à estratégia
  transacional e não deve ser removido sem preservar atomicidade em falhas.

### 4.3 RAM e VRAM — latência de acesso pequena nos microbenchmarks

| Recurso | Modelo observado | Latência reportada | Observação |
|---|---|---:|---|
| Work RAM 8 KB | \`[u8; 8192]\` + máscara | ~1–2 ns | Microbenchmark |
| VRAM 16 KB | \`Vec<u8>\` + \`& 0x3FFF\` | ~2 ns + branch | Microbenchmark |
| CRAM 32 B | \`[u8; 32]\` + máscara | ~1 ns | Microbenchmark |
| Cartucho/SRAM | Mapper com janelas | ~2–5 ns | Microbenchmark |

Essas latências são resultados de microbenchmarks em condições específicas; não representam
latência física de DRAM nem o custo total de uma transação em jogo. Trocar \`Vec<u8>\` por
\`Box<[u8; 16384]>\` ou alterar a seleção de modo são hipóteses que precisam de benchmark de
produção antes de qualquer mudança.

O SUN não modela arbitragem dot-cycle neste caminho. O instante em que uma escrita se torna visível
é definido pelo timestamp e pela ordenação temporal, não pela latência do acesso em memória.

### 4.4 VDP — avanço e render

- No workload de baseline \`jr $\`, \`SmsVdp::advance\` foi chamado aproximadamente **49.780 vezes
  por frame**. A função constrói e percorre fronteiras temporais antes de decidir o avanço. O
  comportamento de não cruzar fronteira depende do perfil e do argumento recebido; não deve ser
  generalizado a todas as chamadas.
- A preparação do buffer de sprites retorna cedo em grande parte das chamadas, conforme o caminho
  observado.
- O render percorre pixels da scanline, resolve os bitplanes e processa sprites. A avaliação de
  sprites usa vetores temporários em alguns caminhos; o número de alocações por frame precisa ser
  medido com contadores antes de justificar uma estrutura fixa.
- A publicação de frame copia o buffer em determinados caminhos. Trocar por buffer swap exige
  provar ownership e sincronização com o frontend.

O valor de **~0,5 ms/frame** citado na auditoria original é uma estimativa instrumentada, não uma
medição independente do VDP.

### 4.5 PSG (SN76489) — geração de amostras

Medições isoladas de \`Sn76489Core::new(3_579_545, rate)\`, com drenagem de amostras por frame:

| Configuração | ms/frame | ns por \`advance_clocks(2)\` |
|---|---:|---:|
| 44.100 Hz | **1,218** | 48,9 |
| 48.000 Hz | **1,217** | 48,9 |
| 44.100 Hz, geração de amostras desativada | **0,118** | 4,7 |

- Alterar o acumulador de fase de \`u128\` para \`u64\` não reduziu materialmente o tempo observado
  nessa medição (1,120 ms).
- Desativar geração de amostras reduziu o custo para 0,118 ms; isso indica que a geração/filtragem
  de amostras domina esse benchmark, mas não identifica isoladamente o custo de cada operação FIR.
- O FIR percorre taps e calcula índices circulares. A hipótese de substituir módulo por máscara só
  é válida se a capacidade for potência de dois e a resposta do filtro permanecer equivalente.
- A hipótese de reutilizar a saída filtrada anterior precisa preservar a interpolação e a sequência
  de amostras.
- A fila de amostras possui caminho de overflow com log. Seu impacto depende do comportamento de
  drenagem do consumidor.

A projeção original de reduzir o PSG de 1,22 ms para 0,15 ms/frame **não foi validada** e não deve
ser apresentada como resultado.

### 4.6 YM2413 — SMS japonês

- O clone transacional de \`Ym2413Core\` foi medido em 10,8 ns por slot; com aproximadamente 14.915
  slots por frame, a projeção aritmética é 0,16 ms/frame no perfil citado.
- A síntese usa operações inteiras e tabelas no caminho analisado.
- A hipótese de avançar o YM em fronteiras próprias em vez de cada evento do Z80 altera a
  sincronização entre domínios. Não deve ser aplicada sem prova temporal e comparação com jogos.

---

## 5. Plano de otimização e situação após as PRs de avaliação

Os números abaixo são hipóteses ou resultados de medições específicas; não são ganhos garantidos
para qualquer ROM/plataforma. A numeração P0/P1/P2/P3 nas PRs é a do plano de otimização do Scheduler
e não deve ser confundida com as hipóteses de redução de eventos M1 descritas nesta auditoria.

### E1 — Reduzir o volume de eventos M1 (hipótese ainda não implementada)

**Hipótese:** consolidar transições T-state do M1 em um evento quando não houver WAIT armado,
mantendo o caminho detalhado quando WAIT exigir observabilidade por T-state.

- A contagem observada no workload \`jr $\` foi 49.780 eventos/frame; o alvo de ~10.000 é projeção,
  não resultado medido.
- **Risco:** alto para fidelidade temporal. Exige prova de amostragem do opcode em T3, WAIT real,
  TH, interrupções e ordem causal.
- **Situação:** não implementar sem evidência e testes direcionados de equivalência.

### E2 — Clocking por fronteira própria (hipótese estrutural)

PSG e VDP poderiam avançar apenas em suas próprias fronteiras ou quando acessados, desde que todos
os efeitos observáveis sejam preservados no timestamp correto.

- O ganho de remover conversões por despacho ainda não foi medido em uma implementação equivalente.
- **Risco:** alto para sincronização causal e visibilidade de escritas.
- **Situação:** pendente; não tratar como otimização de baixo risco.

### D1 — Custo do dispatch e conversões de clock

| Item | Evidência posterior | Situação |
|---|---|---|
| Buffer reutilizável no lugar de \`Vec\` por dispatch | PR #1261: 202,2 → 184,0 ns/op em agendamento + dispatch (−9,0%), zero alocações até 4 domínios; ganho end-to-end de 1,0–3,3% nos workloads sintéticos | Implementado na branch da PR; CI passou; PR aberta |
| Compartilhar previews entre domínios equivalentes | PR #1262: +6,5% mais lento com 3 domínios e +14,3% com 4 | Rejeitado; protótipo removido |
| Guard para evitar revalidação redundante | PR #1263: +74,9% (3 domínios) e +68,0% (4) na comparação alternada | Rejeitado; P2 permanece pendente |
| Fast path checked para timestamps pequenos, mantendo \`MachineTime(u128)\` | PR #1264: −6,3%/−5,8% no harness e −2,1%/−1,3% nos workloads SMS headless em Linux x86_64 | Candidato P3; CI passou; PR aberta; Windows x64/ARM64 pendentes |
| Encolher \`ClockDomainAdvance\`, trocar \`BTreeMap\` ou remover tracing | Sem comparação isolada atual e pareada que justifique alteração | Não aprovado; medir antes de propor |

**Importante:** as medições das PRs ocorreram em hosts e configurações diferentes. Não somar percentuais,
subtrair números entre relatórios ou projetar novo frame time a partir deles. O ganho end-to-end de
#1261 também não reconcilia ainda com a projeção do harness; essa diferença está explicitamente
pendente no relatório P0.

### PSG-A — Otimizações do SN76489 (hipóteses pendentes)

Investigar FIR, interpolação e fila de amostras em workloads com drenagem real. Qualquer alteração
de índice circular, cache de amostra ou capacidade fixa deve preservar resposta espectral,
interpolação, overflow e underrun. A projeção de 0,15 ms/frame não foi validada.

### VDP-A — Otimizações do VDP (hipóteses pendentes)

Medir o custo de fronteiras em \`SmsVdp::advance\` por workload antes de cachear a próxima fronteira.
Avaliar armazenamento fixo de sprites e troca de buffers somente após provar prioridade, overflow,
ownership e sincronização com o frontend. O valor de ~0,5 ms/frame é estimativa, não medição
independente.

### CPU-A — CPU e barramento (hipóteses pendentes)

Qualquer alteração no commit de \`Z80MicroSequencer\` deve preservar atomicidade em falhas.
A aritmética de transições TH só deve mudar após teste de limites e medição no caminho efetivo.
Tornar traces opcionais somente se o custo for medido no perfil de produção e a observabilidade
necessária aos jogos continuar intacta.

### Estrutural-A — Evolução futura do modelo temporal

Uma abstração \`ClockLane\` só é justificável se reduzir trabalho no caminho real sem introduzir custo
de busca por dispatch nem misturar domínios com estado de origem diferente. Domínios com fronteiras
próprias exigem prova de que nenhum estado observável é pulado. A representação pública de
\`MachineTime\` permanece \`u128\`; o fast path local de #1264 não autoriza migrar a timeline global
para \`u64\`.

---

## 6. O que não vale otimizar sem evidência

- \`edge_time_after\` / \`next_edge_time\`: o benchmark original encontrou custo de GCD/divisões,
  mas essas funções não foram observadas no caminho de execução do Scheduler/SMS analisado.
- \`BusTransaction\` e validadores de contrato: não foram identificados no caminho quente descrito.
- Núcleo de síntese do YM2413: é inteiro e baseado em tabelas no caminho analisado; o custo de clone
  transacional deve ser comparado ao risco de perder atomicidade.
- Mapa de memória \`SmsMemory\`: é direto, sem alocação ou dispatch dinâmico no caminho medido.
  Reescrever sem perfil de jogo não tem justificativa demonstrada.

---

## 7. Invariantes que qualquer otimização precisa preservar

1. **Determinismo:** ordem \`(timestamp, hardware_phase, priority, sequence)\` intacta; nenhuma
   otimização pode reordenar eventos no mesmo timestamp.
2. **Atomicidade multi-domínio:** ou todos os domínios avançam, ou nenhum. O preflight deve concluir
   antes de qualquer commit.
3. **Snapshot/restore:** mudanças de representação exigem revisão de
   \`SCHEDULER_STATE_FORMAT_VERSION\`, codec e testes de replay correspondentes.
4. **WAIT real:** qualquer consolidação de M1 precisa manter o caminho por T-state quando WAIT
   estiver armado e provar os instantes observáveis.
5. **Frontend coarse-grained:** nenhuma otimização do backend altera a fronteira
   \`start/pause/stop/reset/load_rom/step_frame\`.

---

## 8. Evidência posterior e decisões das PRs

As avaliações posteriores separaram as alterações e corrigiram a fronteira dos benchmarks:

| PR | Alteração avaliada | Resultado | Decisão atual |
|---|---|---|---|
| [#1261](https://github.com/leonardo201800478/SUN/pull/1261) | Buffer reutilizável para os avanços do Scheduler | 202,2 → 184,0 ns/op (−9,0%) em agendamento + dispatch; zero alocações. End-to-end: −2,0% \`relative-loop\`, −2,4% bus-heavy, −3,3% read-heavy, −1,0% port-heavy | Mantida na branch; CI passou; PR aberta. A projeção do harness (~0,906 ms/frame) não foi reconciliada com o ganho end-to-end de 0,322 ms/frame no \`relative-loop\`. |
| [#1262](https://github.com/leonardo201800478/SUN/pull/1262) | Compartilhamento de previews equivalentes | +6,5% mais lento em 3 domínios e +14,3% em 4 | Rejeitada; nenhum código runtime mantido. |
| [#1263](https://github.com/leonardo201800478/SUN/pull/1263) | Guards para evitar revalidação no commit | +74,9% mais lento em 3 domínios e +68,0% em 4 na comparação alternada | Rejeitada; nenhum código runtime mantido. |
| [#1264](https://github.com/leonardo201800478/SUN/pull/1264) | Fast path de \`checked_duration_since()\`, preservando \`MachineTime(u128)\` | −6,3%/−5,8% no harness; −2,1%/−1,3% nos workloads SMS headless, em Linux x86_64 | Candidato P3; CI passou. PR aberta; Windows x64 e ARM64 não medidos. |

Os valores de #1261 foram obtidos em ARM64/Neoverse-N1; os de #1264 em Linux x86_64 com processo
fixado na CPU. Não comparar diretamente as latências absolutas entre esses hosts. As porcentagens
são resultados de seus respectivos experimentos, não ganhos cumulativos.

Os testes e validações específicos estão registrados nos relatórios vinculados em cada PR. Nenhuma
dessas evidências autoriza remover validações, alterar a ordem temporal ou assumir ganhos em jogos
comerciais sem teste real.

---

## 9. Próximos passos sugeridos

1. **Validar com jogos reais em duas passagens separadas.** Primeiro, executar com contador de
   dispatches habilitado para registrar \`dispatches/frame\`, perfil do sistema e comportamento de
   WAIT. Depois, repetir com a instrumentação temporal desligada para medir frame time; não usar os
   tempos da execução instrumentada como FPS ou latência real.
2. **Validar em Windows x64**, plataforma inicial do frontend, e repetir em ARM64 quando houver
   ambiente disponível. Não extrapolar proporções de um host para outro.
3. **Manter a ordem baseada em evidência:** P0 buffer do chamador (#1261) e P3 fast path local
   (#1264) permanecem candidatos nas respectivas PRs; compartilhamento de previews (#1262) e guards
   de commit (#1263) foram rejeitados.
4. **Não iniciar redução de eventos M1 ou clocking preguiçoso** sem prova explícita de WAIT real, TH,
   interrupções, visibilidade de efeitos e determinismo.
5. **Para PSG/VDP**, medir isoladamente e em workload de máquina antes de alterar buffers, FIR,
   fronteiras ou sincronização.

---

## Apêndice A — Como reproduzir as medições

O harness original foi mantido em crate separado do workspace e incluiu binários para microbenchmarks,
decomposição do Scheduler, execução headless da máquina SMS, PSG, áudio/YM2413 e probes aritméticos.
Os comandos e dependências exatos devem ser obtidos do commit/harness que gerou cada resultado;
não presumir que uma medição antiga possa ser reproduzida com o harness corrigido da PR #1261.

Para contar eventos, \`set_subsystem_timing_enabled(true)\` adiciona duas chamadas
\`Instant::now\` por despacho. Use essa execução somente para contagem/instrumentação; faça outra
execução com a instrumentação desligada para medir frame time.

## Apêndice B — Números de referência do workload sintético

\`\`\`text
SMS NTSC, master_ticks_per_tstate = 3
  T-states/frame .............. 59.736
  eventos/frame (jr $) ........ 49.780  = 5 × 9.956 fetches
  eventos/frame (bus-heavy) ... 40.621
  eventos/frame (read-heavy) .. 50.543
  ticks internos do PSG/frame . 3.733  (59.736 / 16)
  slots YM2413/frame .......... 14.915 (só SMS japonês)
  amostras PSG/frame .......... 735 @ 44,1 kHz / 800 @ 48 kHz
  orçamento a 60 FPS .......... 16,667 ms
\`\`\`
