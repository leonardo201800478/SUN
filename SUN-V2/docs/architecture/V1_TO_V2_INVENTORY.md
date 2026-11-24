# Inventário crítico V1 → V2

**Projeto:** SUN V2  
**Escopo:** primeira máquina SMS  
**Baseline V1 congelada:** `207ccc4cebbc4f8c0aed45c68262eae963491689` (2026-11-23 05:19:37 UTC)  
**Status:** inventário revisado contra código e documentação do SHA congelado; não equivale a validação física.  
**Regra:** este documento registra conhecimento transferível, não autoriza cópia de código.

## 1. Conclusão executiva

A V1 contém uma implementação funcional extensa e um caminho temporal SMS significativamente mais avançado do que o inventário inicial descrevia. O estado atual não pode ser resumido como “CPU por instrução + VDP/PSG isolados”:

- O Z80 compartilhado mantém o executor atômico `Z80::step`, mas também possui `Z80MicroSequencer` retomável, usado por `SmsTemporalMachine`.
- O barramento temporal possui requests com identidade estável, WAIT acumulado, deadlines de conclusão e efeitos aplicados nos boundaries despachados.
- A máquina SMS temporal conecta Z80, barramento/memória, VDP SMS e PSG SN76489 ao MachineScheduler.
- O VDP continua com renderização scanline-level e refinamentos subline seletivos; não há dot/fetch arbitration universal.
- O YM2413 possui perfil físico opcional Japanese SMS, core/serviço serial e caminho de PCM de compatibilidade integrado. A qualidade sonora dos jogos de referência permanece não aprovada.
- O áudio possui mixer multi-source e sessão independente do host; as fontes precisam chegar alinhadas em taxa e contagem de frames.
- O runtime define perfis regionais de clock, mas esses valores de configuração não comprovam fase elétrica entre chips.
- Existe harness de performance S0 para OutRun, mas o próprio dossiê declara a baseline medida ainda aberta.

A distinção essencial para o V2 continua válida: implementação existente não prova cobertura funcional completa, e cobertura de software não prova fidelidade física. O V2 deve aprender com os contratos e falhas observadas, mas manter implementação independente.

## 2. Estado por subsistema no SHA congelado

| Subsistema | Implementação observada na V1 | Limite demonstrado/documentado | Direção V2 |
|---|---|---|---|
| Z80 | Executor semântico atômico e microsequenciador retomável compartilhado; a máquina SMS temporal usa o sequenciador | Subconjunto temporal e cenários de teste não equivalem a precisão elétrica universal nem a microsequenciamento necessário em todo caminho | Reimplementar independentemente; separar semântica, requests de barramento e continuidade temporal |
| Barramento | Handshake com ID, request pendente, WAIT, conclusão e integração com Scheduler | WAIT por fixture é coberto; WAIT físico, DMA e arbitragem externa permanecem não verificados | Contrato explícito e independente; incluir exactly-once, cancelamento/reset e snapshot pendente |
| Memória/cartucho | Memória SMS e adaptadores de cartucho usados na máquina temporal | Catálogo de mappers e escopo de compatibilidade devem ser derivados dos jogos-alvo | Começar com ROM/RAM e mappers mínimos comprovados |
| VDP SMS | VDP conectado ao barramento temporal, com portas, raster, IRQ e composição scanline/subline seletiva | Sem dot/fetch arbitration universal; CRAM dots, fetch-ahead e alguns latches permanecem aproximados | Perfil SMS1 mínimo e boundaries observáveis; refinar somente com teste discriminante |
| PSG SN76489 | Core digital puro com perfil SMS, divisor, fase, tone/noise, saída amostrada e eventos digitais opcionais | FIR e PCM são pipeline digital de saída, não modelo analógico físico completo | Core independente, avanço por tempo explícito, fase preservada e saída determinística |
| YM2413 | Perfil opcional Japanese SMS; core, serviço PG serial, domínio temporal e rota PCM de compatibilidade | Qualidade sonora não aprovada; C-Sync, DAC/analógico, calibração e validação em jogos permanecem pendentes | Implementação V2 separada; contrato de áudio multi-fonte desde o início, FM posterior à primeira fatia |
| Mixer/resampler | Mixer multi-source e sessão nativa com resampler/sink host separados | Fontes precisam estar previamente alinhadas; continuidade completa do host/resampler não é save state de máquina | Timestamp/fase por fonte e restore determinístico, sem CPAL no core |
| Scheduler | MachineTime u128, razões racionais, domínios, eventos ordenados, snapshot/restore e limites de avanço | HardwarePhase é ordinal, não fase elétrica; snapshot do Scheduler não é snapshot completo da máquina | CHRONOS independente; comparar modelos temporais antes de fixar representação/estrutura |
| Host | Worker contínuo, comandos coarse-grained, pacing separado da timeline e transporte de áudio/vídeo | Headless throughput não mede scanout, latência física ou underruns reais | Host posterior, sem autoridade sobre o tempo virtual |

## 3. Fatos e confiança

### 3.1 CPU Z80 e integração temporal

O README de `native/z80_cpu` descreve corretamente o contrato do executor `Z80::step`: instrução completa e T-states agregados. Entretanto, esse README não descreve todo o caminho ativo da máquina SMS. No SHA congelado:

- `native/z80_cpu/src/sequencer.rs` declara o microsequenciador retomável;
- `native/sms_core/src/temporal_machine.rs` mantém `Z80MicroSequencer` como CPU da máquina;
- a execução avança por requests de barramento, ciclos internos e dispatch do Scheduler;
- o estado do sequenciador pode ser exportado/restaurado nos safe points cobertos.

**Confiança:** alta para a arquitetura implementada; a cobertura semântica/temporal deve ser vinculada aos testes e SHAs que a executaram.  
**Implicação V2:** não repetir a premissa de que a V1 não possui microsequenciador. Reimplementar a partir de contratos e casos discriminantes, sem copiar o executor.

### 3.2 Barramento, WAIT e interrupções

O handshake Z80 mantém uma transação pendente, identidade, operação, endereço, duração base e WAIT acumulado. A máquina SMS agenda a conclusão no Scheduler e aplica os efeitos no boundary de conclusão. A máquina temporal também registra eventos de INT/NMI e acknowledge.

**Confiança:** alta para a existência dos mecanismos e para os casos de software documentados.  
**Limite:** WAIT físico/externo, DMA e arbitragem elétrica externa não são comprovados por WAIT de fixture.  
**Implicação V2:** WAIT é parte do contrato de transação, mas sua fonte deve ser explicitada (fixture, dispositivo ou sinal físico); nunca tratar os três como a mesma evidência.

### 3.3 VDP SMS

A documentação V1 descreve o VDP SMS1/315-5124 e o engine compartilhado, incluindo portas, VRAM/CRAM, Mode 4, modos legados, sprites, counters, IRQ e boundaries seletivos. A integração temporal avança o VDP a partir dos T-states do Scheduler. A rasterização permanece scanline-level com boundaries subline específicos.

**Confiança:** alta para os limites explicitamente documentados; média para detalhes sustentados por ROMs/modelos comparativos sem medição física.  
**Limite:** não declarar dot accuracy, fetch-ahead completo ou pipeline elétrico universal.  
**Implicação V2:** precisão seletiva por efeito observável, não pipeline por pixel/dot por princípio.

### 3.4 PSG e áudio

O SN76489 mantém clock, divisor /16, fase do divisor, fase de saída, contadores, LFSR, filtro e clocks decorridos. O core avança por quantidade explícita de clocks. A sessão de áudio aceita fontes mono alinhadas com a mesma taxa e contagem de frames; o resampler final e o sink ficam fora do chip.

**Confiança:** alta para os contratos estruturais no código.  
**Limite:** PCM/FIR não é calibração analógica da placa; fontes com taxas nativas distintas precisam de alinhamento anterior ao mixer.  
**Implicação V2:** o contrato de chip deve ser independente da taxa PCM e do host; o alinhamento multi-fonte deve preservar timestamp e fase.

### 3.5 YM2413

A V1 possui perfil físico explícito de Japanese SMS com YM2413 opcional, domínio temporal adicional, core e serviço PG serial. A rota PCM de compatibilidade usa pipeline digital de referência; o scorecard vigente no congelamento registra que a qualidade sonora de Kenseiden e Phantasy Star não foi aprovada. C-Sync, calibração do DAC, transferência analógica e correspondência de presets permanecem lacunas.

**Confiança:** alta para o estado declarado no scorecard e para a presença do caminho de software.  
**Implicação V2:** não considerar FM concluído, não inferir presença de chip pela região e não permitir Auto sem evidência exata de ROM/placa. O FM não bloqueia a primeira fatia PSG, mas o contrato multi-fonte deve nascer desde o início.

### 3.6 Clocks e fases

O worker resolve perfis regionais: USA NTSC 3.579.545 Hz, Brasil PAL-M 3.579.545 Hz, Europa PAL 3.546.895 Hz e Japão/Coreia NTSC 3.579.545 Hz. O caminho atual usa um tick lógico por T-state Z80; o coordenador registra CPU, PSG e VDP com razão comum e YM2413 como domínio opcional na mesma escala lógica.

**Confiança:** alta para a configuração do software.  
**Limite:** isso não prova fonte elétrica comum, fase inicial ou ausência de deriva entre chips/placas.  
**Implicação V2:** frequência nominal, razão racional, fase inicial e convenção de tick mestre precisam ser evidenciadas separadamente por perfil.

### 3.7 Scheduler e snapshots

O Scheduler possui timestamp inteiro u128, razões racionais, fase residual, ordenação determinística, delta-event budget, snapshot e restore validados. A documentação é explícita: `HardwarePhase` é ordinal e não representa fase elétrica. O snapshot do Scheduler preserva apenas o estado temporal que lhe pertence; a máquina precisa preservar CPU, transações, dispositivos, latches e pipelines.

**Confiança:** alta para o contrato do Scheduler isolado.  
**Limite:** a campanha longa histórica de stress foi executada em SHA anterior; não atribuir automaticamente o resultado à implementação atual sem reconciliação dos blobs.  
**Implicação V2:** modelo de referência próprio deve provar chunk invariance, causalidade e restore/replay antes de escolher a estrutura definitiva.

## 4. Evidências e proveniência

A matriz completa, com SHA/blob de fontes, confiança, divergências, riscos e testes discriminantes, está em:

- [V1_BASELINE_EVIDENCE_MATRIX.md](V1_BASELINE_EVIDENCE_MATRIX.md)

Fontes principais fixadas no SHA:

- [Z80 shared README](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/z80_cpu/README.md)
- [Z80 sequencer](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/z80_cpu/src/sequencer.rs)
- [Z80 bus handshake](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/z80_cpu/src/micro_ops.rs)
- [SMS temporal machine](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/sms_core/src/temporal_machine.rs)
- [SMS temporal bus](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/sms_core/src/temporal_bus.rs)
- [SMS VDP architecture](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/docs/architecture/sms-vdp.md)
- [SN76489 core](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/sn76489/src/psg.rs)
- [Audio session](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/native/audio_runtime/src/session.rs)
- [Scheduler clock-domain rules](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/docs/architecture/scheduler/clock-domain-rules.md)
- [YM2413 completion scorecard](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/docs/ym2413-fm-completion-scorecard.md)
- [PERF-01/S0 baseline protocol](https://github.com/leonardo201800478/SUN/blob/207ccc4cebbc4f8c0aed45c68262eae963491689/docs/testing/perf-01-s0-baseline.md)

## 5. O que pode ser transferido como conhecimento

Após verificar origem, licença, SHA, parâmetros e validade:

- mapas de memória e I/O SMS;
- documentação de registradores e comportamento funcional;
- casos de regressão e ROMs de diagnóstico legalmente utilizáveis;
- vetores de referência e traces com proveniência;
- limites conhecidos de CPU, VDP, PSG e YM2413;
- benchmarks e protocolos, desde que reexecutados no ambiente-alvo;
- divergências entre implementações e hipóteses ainda abertas.

## 6. O que não será herdado automaticamente

- código de CPU, VDP, PSG, YM2413, Scheduler, barramento ou worker;
- estruturas de dados, APIs e divisão de crates;
- razões/fases de clock sem evidência suficiente;
- WAIT de fixture como prova de WAIT físico;
- resultado de CI antigo como prova no SHA congelado;
- stress de Scheduler executado em SHA diferente sem comparação dos blobs;
- PCM de compatibilidade como prova de fidelidade acústica;
- throughput headless como prova de 60 FPS apresentados;
- snapshot em memória como prova de codec persistente completo da máquina.

## 7. Lacunas que fecham R0

| ID | Pergunta | Evidência necessária | Prioridade |
|---|---|---|---|
| INV-01 | SHA V1 congelado? | Fechado: `207ccc4cebbc4f8c0aed45c68262eae963491689` | Fechado |
| INV-02 | CPU semântica vs temporal? | Sequenciador, requests, WAIT, interrupções e testes por família | Bloqueante |
| INV-03 | WAIT físico e arbitragem SMS? | Sinal/fonte de hardware e casos discriminantes | Bloqueante |
| INV-04 | Boundaries VDP observáveis por gameplay? | ROMs, traces e evidência por perfil ASIC | Alta |
| INV-05 | Clocks e fases CPU/VDP/PSG por NTSC/PAL/PAL-M? | Fonte primária/medição, razão, fase e convenção de tick | Bloqueante |
| INV-06 | Saída de referência PSG? | Vetores/PCM com clock, fase, perfil e proveniência | Alta |
| INV-07 | Qualidade e perfil de placa YM2413? | Captura separada, A/B, avaliação auditiva e evidência física | Posterior à primeira fatia |
| INV-08 | Baseline de desempenho? | OutRun com ROM SHA-256, host, toolchain e artefatos S0 | Bloqueante |
| INV-09 | Quais evidências V1 são transferíveis? | SHA de código/teste, ambiente e oráculo independente | Alta |
| INV-10 | Codec persistente completo de máquina? | Codec versionado e restore/replay com todos os estados pendentes | Alta |

## 8. Limite deste inventário

A auditoria de primeira passagem foi feita contra o SHA congelado e encontrou drift entre alguns READMEs/scorecards e o caminho temporal ativo. Ela não reexecutou todas as suítes, não validou hardware físico e não declara TEMPORAL_PASS global.

R0 permanece aberto até que as questões críticas de clocks/fases, colisões temporais, WAIT, boundaries VDP e baseline headless tenham evidências suficientes e testes discriminantes registrados.

**Próxima ação:** fechar a matriz específica de clocks/fases e colisões simultâneas; em paralelo documental, definir os cenários ROM/homebrew e o protocolo de baseline. Só então congelar os contratos R1 e construir o modelo de referência R2.
