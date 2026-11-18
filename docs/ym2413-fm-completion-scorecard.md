# YM2413 FM Completion Scorecard

**Data de referência:** 2026-11-18
**Repositório:** `leonardo201800478/sega-emulator`
**Main após FM-3 parcial:** `daf950d0e0bf3242dfb213d5e9e50a24db19a70e`

Este scorecard distingue implementação de componente, integração funcional, validação temporal e evidência física. Uma etapa só é considerada concluída quando seus critérios próprios estão demonstrados.

## Estado executivo

**Status geral: EM DESENVOLVIMENTO — NÃO PRONTO PARA TESTES CONTROLADOS COM ROMs.**

A fundação de registradores, slots, clock, PG-1 e PM LFO está integrada. O EG e o par de operadores FM foram implementados como componentes candidatos, com testes e snapshots locais, mas ainda não estão conectados ao pipeline de 18 slots nem ao estado agregado do YM2413. Ainda não existe geração PCM FM integrada ao runtime SMS.

## Marcos já publicados

| Marco | Resultado | Evidência |
|---|---|---|
| Auditoria inicial | Integrada | PR #994; `docs/audits/ym2413-pre-implementation-audit-2026-11-17.md` |
| Fundação YM2413 | Integrada | PR #996; latch, registradores, 18 slots, 4 clocks/slot, rotação de 72 clocks, snapshot base |
| PG-1 | Integrada | PR #997; commit `a6adda38cc7db9317e93ca3c072f980fb5d239ec` |
| PM LFO global | Integrado | PR #998; commit `fc383defbe035e699f33b07e7a525cfdb5b7fa5a` |
| Auditoria de alinhamento PM | Integrada | PR #999; merge commit `1ce082a950ce2940b3f3de4c56dc7dbf37c536f9` |
| EG candidato | Integrado | PR #1001; merge commit `995dce7025b189ba25ce82a2dbf665d9cce9fb14` |
| Operadores FM e KSL | Integrados como candidatos isolados | PR #1003; merge commit `c9285d2fc2d8e3987197d5186478dae784da1570` |
| AM LFO | Integrado como candidato sample-clocked | PR #1005; merge commit `daf950d0e0bf3242dfb213d5e9e50a24db19a70e` |

## Scorecard por etapa

| Etapa | Estado | Entrega existente | Pendência para conclusão |
|---|---|---|---|
| FM-0 — Auditoria e consolidação | **Concluída** | Main/PRs auditadas; PR #999 integrada; arquitetura documentada | Manter o scorecard atualizado conforme evidências novas |
| FM-1 — PG e EG | **Parcial** | PG-1; PM LFO global; EG candidate DAMP/ATTACK/DECAY/SUSTAIN; KSR; snapshots locais; 17 testes EG, 14 testes de tabelas/operadores, 6 testes KSL, 7 testes AM LFO e 8 testes de instrumentos/canais | Integrar 18 EGs ao pipeline de operadores; resolver contratos de update; integrar snapshot ao estado agregado; validar transições com vetores independentes |
| FM-2 — Operadores e síntese FM | **Parcial** | Tabelas Log-Sine/EXP; lookup de fase/sinal; par modulator/carrier; feedback de dois samples; KSL BLOCK/FNUM; snapshots locais | Integrar KSL ao decoder/canal, validar PCM diferencial, integrar aos slots e confirmar escala/quantização fisicamente |
| FM-3 — LFO e modulação | **Parcial** | Contador PM global, tabela PM 8×8 e AM LFO de 210 posições/64 amostras, 14 níveis e snapshot | Conectar PM/AM aos operadores; comprovar fase absoluta e alinhamento temporal |
| FM-4 — Instrumentos | **Parcial** | User patch decoder; 15 presets melódicos candidatos com licença MIT/procedência; decoder FNUM/BLOCK/KEY/SUS/INST/VOL por canal | Validar patches contra hardware; ligar parâmetros ao PG/EG/operadores; confirmar os nove canais; ROM física exata ainda não disponível |
| FM-5 — Rhythm Mode | **Parcial** | Mapa de seis slots (PR #1010) e LFSR de 23 bits como primitiva explícita (PR #1011) | Síntese das cinco vozes, aplicação do ruído, volumes, seed/cadência física e temporização |
| FM-6 — Scheduler | **Pendente** | Core tem avanço de input clocks e rotação de 72 clocks | Clock domain, ordem dos slots, sample counter compartilhado, escritas temporalmente ordenadas e equivalência contínuo/blocos |
| FM-7 — Hardware SMS | **Pendente** | Nenhuma porta FM conectada | Portas F0–F2, latch, barramento, perfis Mark III/FM Unit/MK-2000/exportação e FM Auto/Off/On |
| FM-8 — Áudio e validação final | **Pendente** | Nenhuma saída PCM FM integrada | Mixer FM+PSG, resampling, clipping, continuidade, Save States agregados, regressões SMS/SG-1000 e testes com ROMs |

## Inventário funcional atual

| Recurso | Implementação | Integração / validação |
|---|---|---|
| Latch e imagem de 64 registradores | Implementados | Integrados ao crate |
| Máquina de 18 slots | Implementada | Rotação de 72 clocks; pipeline funcional FM ainda pendente |
| PG-1 | Implementado | Testes unitários existentes; vetores independentes de andete não equivalem a medição própria |
| PM LFO global | Implementado | Período 8192 amostras; não confundir avanço por amostras com clocks |
| EG por operador | Implementado como candidato | 17 testes unitários; não conectado aos slots |
| Log-Sine / EXP | Implementados como tabelas inteiras e funções de lookup isoladas | Ainda não integrados ao pipeline dos 18 slots; modelo matemático candidato |
| Modulator / carrier | Par de operadores candidato implementado | Recebe PG/EG/KSL/AM externamente; ainda sem instrument ROM e sem slot scheduling |
| Feedback FM | Histórico p0/p1 e cálculo de fase implementados | Pendente validação independente de PCM e integração ao canal real |
| KSL | Lookup BLOCK/FNUM e escala KSL=0..3 implementados | Ainda não conectado ao decoder de instrumentos/canais |
| AM LFO | Implementado como LFO compartilhado, sample-clocked, 210 posições, 64 amostras/posição, 14 níveis e snapshot | Ainda não conectado aos bits AM dos instrumentos/18 operadores |
| Instrument ROM | 15 patches candidatos licenciados (MIT) e decodificação do user patch | Valores não são dump físico certificado; revisão de hardware ainda necessária |
| Channel register decode | FNUM/BLOCK/KEY-ON/SUSTAIN/INST/VOL para nove canais | Ainda não conectado ao pipeline temporal/operadores |
| Rhythm / LFSR | Bit RHYTHM de $0E, mapa de seis slots e LFSR de 23 bits implementados como primitiva explicitamente clockada (PRs #1010/#1011) | Cinco vozes, aplicação do ruído, seed/cadência física, envelopes e pipeline rítmico não implementados |
| Portas F0–F2 | Não integradas | Pendente |
| Mixer FM + PSG | Não integrado | Pendente |
| Save State agregado | Parcial | Snapshots base, PG, PM e EG locais; estado completo de todos os operadores ainda não agregado |
| PCM FM | Não disponível | Pendente |

## Contratos e incertezas que devem permanecer explícitos

1. **Contador global:** o EG recebe o contador externamente; não cria nem reinicia seu relógio. A evidência disponível descreve um contador compartilhado entre os 18 operadores, incrementado uma vez por amostra. O alinhamento absoluto com reset IC e com PG/PM ainda exige integração e validação.
2. **DAMP:** o EG expõe limiar configurável. O valor 124 é um candidato do modelo de software; as notas de engenharia reversa também registram limiar próximo de -45 dB e incerteza.
3. **Coordenação modulator/carrier:** o EG oferece modelos explícitos independente e carrier-driven. Nenhum deve ser tratado como comportamento físico definitivo sem validação.
4. **AR=15:** o comportamento na fronteira DAMP/ATTACK é selecionável como candidato; não foi certificado em hardware pelo projeto.
5. **Slots:** a ordem exata de atualização de PG, EG, modulator e carrier deve ser estabelecida antes da integração temporal.
6. **Instrument ROM:** os parâmetros exatos dos instrumentos ainda exigem procedência e validação. Não copiar tabelas de emuladores sem licença e origem verificadas.
7. **Rhythm LFSR:** PR #1011 implementa a recorrência Galois de 23 bits com seed explícito. O polinômio/recorrência estão documentados por reverse engineering; o estado inicial/reset físico e a cadência de clock-enable continuam desconhecidos.
8. **Fidelidade:** nenhuma implementação atual equivale a certificação de fidelidade física completa.

## Gates CI publicados

### PR #999 — documentação PM

- HEAD validado: `701753f272e139dc89e946e5cabb3656eb06a5bf`
- Workflow #44227356163: sucesso nos gates documentais aplicáveis.
- Gates de runtime não executados para aquela alteração documental.

### PR #1001 — EG candidato

- HEAD validado: `ac08b570431042e9e9daf66211d04902d4f7964c`
- Workflow #44243755335:
  - Change impact classification: SUCCESS
  - Documentation patch hygiene: SUCCESS
  - Rust workspace regression: SUCCESS
  - Scheduler deterministic stress: SUCCESS
  - Rust quality and unit regression: SUCCESS
  - Validações de PSG/console, CPAL, SDL3, Z80, VDP e desktop não relacionadas: SKIPPED

**Interpretação:** os gates Rust aplicáveis ao EG passaram. Os itens SKIPPED não são aprovados por inferência e não validam a futura integração FM/SMS.

## Próxima dependência técnica

O próximo desenvolvimento funcional é ligar o patch/channel decoder aos parâmetros do par de operadores: MULTI/PG, KSR/EG, KSL, AM, waveform, TL/VOL e feedback. Os 15 patches continuam candidatos até comparação física; depois disso deve ser fechado o pipeline dos 18 slots.

Não iniciar a integração SMS ou mixer como substituto da síntese FM ainda inexistente. Não declarar o chip pronto para ROMs até haver PCM FM reproduzível, integração temporal e testes de regressão.



### PR #1003 — operador FM e KSL

- HEAD validado: `25154edce743db8ed03f86e74641500dc2df722e`
- Merge commit: `c9285d2fc2d8e3987197d5186478dae784da1570`
- Workflow #44246095781: SUCCESS nos gates aplicáveis.
- Rust workspace regression, Rustfmt, Clippy, observability feature tests, Scheduler deterministic stress e documentation hygiene: SUCCESS.
- Validações de hardware/console/desktop não relacionadas: SKIPPED.



### PR #1005 — AM LFO

- HEAD validado: `5b929574bef91d3145c6dc818c0455a8883001a9`
- Merge commit: `daf950d0e0bf3242dfb213d5e9e50a24db19a70e`
- Workflow #44247005132: SUCCESS nos gates aplicáveis.
- Rustfmt, Clippy, workspace unit/integration tests, observability feature tests, Scheduler deterministic stress e documentation hygiene: SUCCESS.
- Validações de console/desktop não relacionadas: SKIPPED.

## Capacidade de execução multiagente

A sessão de desenvolvimento utilizada para estes marcos não disponibiliza três executores independentes. As três frentes foram coordenadas sequencialmente; não houve execução paralela de três agentes. A separação de responsabilidades permanece válida para a próxima etapa, mas não se deve registrar trabalho paralelo inexistente.

## Referências técnicas

- [SMS Power — YM2413 reverse engineering index](https://www.smspower.org/Development/YM2413)
- [Yamaha YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [andete — decay/release measurements (2015-03-20)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20)
- [andete — attack measurements (2017-01-26)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2017-01-26)
- [andete — ADSR transitions and operator behavior (2015-12-31)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31)
- [andete — rhythm/noise LFSR (2018-05-13)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13)


---

## Atualização FM-4 — ponte de parâmetros por canal

**Branch:** `feat/ym2413-channel-synthesis-bridge`
**Base de implementação:** main `a5a9eb826b086966b1436e1b584f91deeada78d4`

Foi adicionado `native/ym2413/src/channel.rs`, uma camada pura de resolução entre os registradores/instrumentos e os componentes de síntese existentes.

### Entrega

- Resolve os nove canais melódicos a partir de FNUM, BLOCK, KEY-ON, SUSTAIN, instrumento e volume.
- Resolve instrumento 0 para o user patch e 1–15 para os presets candidatos já licenciados/documentados.
- Converte MULTI e VIB do patch em entradas explícitas do PG por operador.
- Converte AR/DR/RR/SL, KSR, BLOCK/FNUM, SUS e EG-TYP em `EnvelopeParameters` por operador.
- Calcula KSL de modulator e carrier pela tabela existente.
- Constrói `OperatorPairParameters` com TL, VOL, feedback, waveform, KSL, níveis EG e AM habilitado individualmente por operador.
- Exporta os tipos e a função de resolução pelo crate `sega-ym2413`.

### Limites preservados

- A ponte não avança PG, EG, AM/PM, clocks ou slots.
- Não decide alinhamento de fase, ordem modulator/carrier, DAMP, reset, nem cadência global do EG.
- Não interpreta os canais 7–9 em Rhythm Mode; eles não podem ser enviados por este caminho melódico enquanto RHYTHM estiver ativo.
- Os presets continuam candidatos, não ROM física certificada.
- Nenhuma geração PCM, integração com Scheduler, portas SMS ou mixer é declarada.

### Testes adicionados — aguardando CI no HEAD da PR

- Resolução do user patch e mapeamento completo dos campos por operador.
- Resolução dos 15 presets nos nove canais.
- Rejeição de canal inválido e de código de preset inválido na API de resolução.
- AM enable independente por operador.
- Validação dos níveis dinâmicos EG/AM.
- Seleção de MULTI e PM no cálculo de incremento de fase.

**Estado:** FM-4 permanece PARCIAL. Esta entrega fecha somente a ponte estática de parâmetros; o próximo marco funcional é o pipeline agregado dos 18 slots, ainda sem declarar temporização física fechada.


---

## Atualização FM-4 — PR #1008 integrada

- PR #1008: feat(ym2413): connect channel patches to synthesis parameters
- Merge commit: c1b0ce50d1219bd97197901d2617b4f49f75b88e
- CI #44276258963: SUCCESS.
- Formatação, Clippy, testes unitários/integração do workspace e feature observability: SUCCESS.
- Gates de Scheduler, PSG, CPAL, SMS, SDL3, Z80 e desktop: SKIPPED por não serem afetados.
- A resolução melódica rejeita canais 7–9 quando RHYTHM está ativo.

A ponte de parâmetros está integrada, mas FM-4 continua PARCIAL: presets candidatos não foram validados em hardware e os parâmetros ainda precisam ser exercitados pelo pipeline agregado.

## Atualização FM-5 — banco agregado digital (em validação)

**Branch:** feat/ym2413-aggregate-synthesis-bank

Foi iniciado native/ym2413/src/synthesis.rs, que agrega:

- 18 acumuladores PG independentes;
- 18 geradores EG independentes;
- nove históricos de feedback modulator/carrier;
- um contador PM LFO e uma fase AM LFO globais;
- snapshot agregado versionado de todos os estados acima.

### Contrato preservado

- Fases PG, níveis/estágios/key states EG, feedback e fases iniciais dos LFOs são parâmetros explícitos; nenhum reset físico é inventado.
- A atualização PG e EG é feita por operador, permitindo que o futuro adaptador forneça a ordem de slots pesquisada.
- O contador global do EG continua externo e é fornecido em cada tick.
- A coordenação DAMP CarrierDrivesBoth permanece uma operação explícita, não automática.
- A avaliação modulator/carrier consome os estados PG/EG já atualizados e não avança clocks.
- O banco não implementa rotação de slots, PCM contínuo, mixer, portas SMS ou Scheduler.
- O avanço dos LFOs é explícito e recebe amostras concluídas externamente.

### Testes adicionados — aguardando CI

- Retenção dos 18 PGs e 18 EGs a partir de condições iniciais explícitas.
- Atualizações individuais PG/EG e avaliação de canal determinística.
- Avanço compartilhado dos LFOs independente da partição em blocos.
- Snapshot/restore de todos os estados locais e globais.
- Rejeição de versão/counter LFO inválidos e PG fora da faixa.
- Proteção contra resolver canais de ritmo como canais melódicos.

**Estado:** FM-5 PARCIAL. A agregação de estado não equivale à implementação da sequência física de 18 slots nem à geração de PCM FM.


---

## Atualização FM-6 — atribuição serial dos 18 slots

**Branch:** feat/ym2413-slot-sequencer-map

Foi adicionada a tabela constante YM2413_SLOT_MAP, baseada na Tabela II-3 do Yamaha YM2413 Application Manual. O índice do slot do core (0..17) agora pode ser consultado como descritor de canal, operador e atribuição opcional de voz rítmica.

### Ordem documentada

| Slots físicos | Canal | Operadores |
|---|---:|---|
| 1, 4 | 1 | Modulator, Carrier |
| 2, 5 | 2 | Modulator, Carrier |
| 3, 6 | 3 | Modulator, Carrier |
| 7, 10 | 4 | Modulator, Carrier |
| 8, 11 | 5 | Modulator, Carrier |
| 9, 12 | 6 | Modulator, Carrier |
| 13, 16 | 7 | Modulator, Carrier |
| 14, 17 | 8 | Modulator, Carrier |
| 15, 18 | 9 | Modulator, Carrier |

A ordem serial completa é M1, M2, M3, C1, C2, C3, M4, M5, M6, C4, C5, C6, M7, M8, M9, C7, C8, C9.

### Reatribuição em Rhythm Mode

- Slot 13 + 16: Bass Drum.
- Slot 14: Hi-Hat.
- Slot 15: Tom-Tom.
- Slot 17: Snare Drum.
- Slot 18: Top Cymbal.

O descritor de ritmo não implementa geração de percussão; preserva a atribuição física na camada de mapeamento.

### Integração no core

- Ym2413Core::next_slot_descriptor() expõe a atribuição do próximo slot sem avançar o tempo.
- O índice é o mesmo que já é avançado pela fundação de clock, com 4 input clocks por slot e 18 slots por rotação.
- Testes verificam a ordem, a reatribuição de seis slots rítmicos, rejeição de índices inválidos e o wrap após uma rotação.

### Limite técnico mantido

Esta entrega fecha apenas a tabela de atribuição serial. Ainda não implementa nem certifica:
- offsets entre as etapas PG, EG e operador;
- deslocamentos dos registradores internos de fase e envelope;
- pipeline de feedback e latência modulator → carrier;
- contador EG global e sua relação com os slots;
- eventos MO/RO, DAC e soma de saída;
- ritmo, noise LFSR e PCM FM final.

A documentação do chip confirma a reutilização da célula FM em 18 slots e a tabela de atribuição. A descrição de uma implementação Rust recente relata quatro input clocks por slot, pipeline PG/EG/OP com offsets e uma amostra por 18 clocks internos; isso é uma referência de arquitetura, não substitui a validação independente do nosso pipeline. O projeto não copia código GPL de Nuked-OPLL nem adiciona essa dependência.

**Referências externas:**
- Yamaha YM2413 Application Manual, II-5 Channels and Slots, IV-1 Clock Generation: https://map.grauw.nl/resources/sound/yamaha_ym2413.pdf
- SMS Power — YM2413 reverse engineering: https://www.smspower.org/Development/YM2413
- Descrição técnica de pipeline e clock (documentação Rust): https://docs.rs/emu198x-yamaha-ym2413/latest/emu198x_yamaha_ym2413/

**Estado:** FM-6 PARCIAL. O mapeamento serial está implementado e testado por CI pendente; o pipeline de estágios continua aberto.


---

## Atualização documental — PRs #1010 e #1011 / dossiê de evidências

**Referência:** [Dossiê YM2413 — pesquisa, evidências e lacunas](audits/ym2413-research-evidence-dossier-2026-11-18.md).

- PR #1010 integrou o mapa serial de 18 slots, com ordem baseada na Tabela II-3 do manual Yamaha e atribuição de seis slots em Rhythm Mode. Isso não implementa as subfases internas do pipeline.
- PR #1011 integrou o LFSR de ruído de 23 bits como primitiva explícita, com recorrência, snapshot e seed fornecido pelo chamador. O chip não avança esse LFSR automaticamente.
- O seed físico de reset, a cadência de clock-enable e a combinação do ruído nas quatro vozes não-Bass Drum permanecem pendentes.
- O scorecard anterior dizia que LFSR/slots não existiam; esse inventário foi corrigido. A síntese das cinco vozes de ritmo permanece pendente.
- O dossiê diferencia fatos do manual, medições publicadas, reverse engineering, inferências e lacunas. A prioridade seguinte é especificar o pipeline de 18 slots por subfase antes de conectar o banco agregado aos operadores.
- Nenhuma alteração de Scheduler, barramento SMS, mixer, saída PCM ou GUI é autorizada por esta atualização documental.

**Estado FM-5:** parcial — mapa de slots e gerador LFSR isolado; Rhythm Mode ainda não sintetiza as cinco vozes.

**Estado FM-6:** parcial — rotação e atribuição serial existem; offsets PG/EG/OP, feedback, DAC e latência continuam sem validação física.

**Validação:** alteração documental apenas. Não declara testes de hardware, PCM FM ou regressões de runtime executados.


---

## Investigação P1 — pipeline de 18 slots (2026-11-18)

Documento complementar: [YM2413 — Investigação P1 do pipeline de 18 slots](audits/ym2413-slot-pipeline-investigation-2026-11-18.md).

- Confirmada a ordem dos 18 slots e o período de 72 input clocks por rotação.
- Quatro clocks por slot permanecem uma derivação temporal; as quatro subfases funcionais não foram identificadas.
- PG de 19 bits, feedback com dois valores históricos, EG com contador global e saída MO/RO multiplexada foram mapeados às evidências disponíveis.
- Offsets de PG/EG/OP, latência M→C, commit de registradores e latch exato do DAC continuam pendentes.
- **FM-6 permanece pendente.** Esta investigação é documentação; não implementa nem valida o pipeline executável.

---

## FM-7 — decodificação estática dos registradores de Rhythm Mode

**Branch:** feat/ym2413-rhythm-register-decode

Foi adicionada a decodificação pura do estado dos cinco instrumentos de percussão, no módulo `native/ym2413/src/rhythm.rs`.

### Campos cobertos

- `$0E.D5`: habilitação de Rhythm Mode.
- `$0E.D4..D0`: triggers de Bass Drum, Snare Drum, Tom-Tom, Top Cymbal e Hi-Hat.
- `$36.D3..D0`: volume de Bass Drum.
- `$37.D7..D4`: volume de Hi-Hat; `$37.D3..D0`: volume de Snare Drum.
- `$38.D7..D4`: volume de Tom-Tom; `$38.D3..D0`: volume de Top Cymbal.
- FNUM, BLOCK, KEY-ON e SUS brutos dos canais 7–9 (índices 6–8), preservados separadamente dos registradores de volume de ritmo.

A ativação efetiva de cada voz é calculada como Rhythm Mode habilitado **e** trigger correspondente. Os bits KEY-ON melódicos de `$26–$28` são reportados sem serem reescritos: o manual orienta mantê-los em zero durante Rhythm Mode, mas o decoder não altera o estado de registradores do chamador.

### Testes adicionados

- Mapeamento dos cinco triggers e dos cinco nibbles de volume.
- Gate de ativação por Rhythm Mode, sem descartar o trigger bruto.
- Decodificação de FNUM/BLOCK/KEY/SUS dos canais 7–9.
- Garantia de que `$36–$38` são tratados como volumes de percussão, não como seleção melódica.

### Limites

- Nenhuma forma de onda ou envelope de percussão é sintetizado.
- O Bass Drum ainda não percorre o par FM de slots 13/16.
- As fases não-BD derivadas de ruído branco + frequências dos canais 8/9 não estão implementadas.
- A tabela ROM de envelopes de ritmo, a duplicação de saída descrita pelo manual, a cadência do LFSR e o roteamento MO/RO permanecem pendentes.
- Não há alteração no Scheduler, no banco agregado FM, no mixer ou no runtime SMS.

**Estado:** FM-7 PARCIAL — apenas configuração estática dos registradores. Os testes direcionados, incluindo os cinco casos one-hot, e o CI #44297715529 passaram no HEAD da PR #1015 antes do merge squash. Os gates não aplicáveis foram SKIPPED. Não equivale a Rhythm Mode funcional nem a PCM de percussão validado.

**Referência primária:** Yamaha YM2413 Application Manual, III-1-7 (Rhythm), III-1-8 (Instrument/Volume) e V-4 (Percussion Sound Synthesis): https://map.grauw.nl/resources/sound/yamaha_ym2413.pdf. A alocação dos bits do registrador $0E também foi comparada com as notas de reverse engineering de andete: https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-16.

---
## Correção de estado — 2026-11-18 / PR #1014 integrada

**Main após PR #1015:** 1f0cfe37a5531c8e802ee9ba5f558eaa10b23797.

- PR #1014 foi integrada por squash. O commit de merge é o HEAD confirmado da main.
- Workflow CI #44292422118, executado no HEAD da PR b2a0b054b1fe6a5d18724dd83ff9f020e36629bc: Change impact classification, Documentation patch hygiene, Rust workspace regression e Rust quality/unit regression concluíram com SUCCESS.
- Os gates não relacionados foram SKIPPED e não são considerados validação do YM2413 em hardware.
- A decodificação estática de RHY/BD/SD/TOM/TOP-CY/HH, volumes e campos de frequência dos canais 7–9 está integrada em native/ym2413/src/rhythm.rs.
- FM-7 continua PARCIAL: não existe síntese das cinco vozes, ROM de envelopes de percussão validada, integração temporal do LFSR, PCM ou validação física.

### Reorganização da sequência de fechamento

A sequência de trabalho atual, definida para esta retomada, é:

1. **FM-7 — Rhythm Mode:** implementar e validar separadamente as cinco vozes, sem inventar cadência de LFSR ou fases temporais.
2. **FM-8 — Pipeline temporal:** fechar PG/EG/operadores/feedback/M→C/registradores/slots/DAC/MO-RO com contrato sustentado por evidências.
3. **FM-9 — Integração de áudio:** auditar native/audio_runtime e integrar somente após o contrato temporal estar validado.
4. **FM-10 — Validação independente:** vetores diferenciais, determinismo, snapshots, continuidade por blocos e comparação de hardware quando disponível.

Esta sequência substitui a numeração antiga de FM-5 a FM-8 acima, que refletia o roadmap anterior. Não altera o significado dos componentes já implementados nem promove estados de validação.

### Correção de validação da PR #1014

A frase anterior “os testes de unidade e o CI desta PR ainda precisam ser associados ao HEAD final” fica superada pelo registro acima. O CI passou no HEAD da PR antes do merge; não foi executado um workflow adicional no commit de merge, e isso não deve ser alegado.

**Referência de investigação:** audits/ym2413-rhythm-mode-investigation-2026-11-18.md.


## FM-7 — avanço a partir das capturas de Rhythm Mode (2026-11-18)

- PR #1015 integrada por squash; CI #44297715529 passou em Rust workspace regression, Rust quality/unit regression, Documentation patch hygiene e Change impact classification. Os demais gates foram SKIPPED.
- Main após merge: `1f0cfe37a5531c8e802ee9ba5f558eaa10b23797`.
- A investigação de andete em `results/rhythm/rhythm.md` e a atualização de LFSR em `results/lfsr/lfsr.txt` acrescentam capturas físicas e esclarecem a fase das vozes e a cadência interna do ruído.
- A primeira função Top Cymbal derivada do Genesis Plus GX foi descartada: usava os operadores/canais errados em relação às medições de andete. A branch #1016 foi corrigida para o candidato baseado em mod7 + car8 e na forma OR confirmada na atualização de Nuke.YKT. O relatório mantém explícitas as 16 variantes equivalentes por offsets PG iniciais desconhecidos.
- `rhythm_phase.rs` agora contém candidatos combinacionais de Top Cymbal, Hi-Hat e Snare Drum. As entradas de ruído permanecem explícitas; nenhum relógio é avançado dentro das funções.
- `rhythm.rs` preserva o LFSR decimado `x^23+x^9+x^8+x+1` e adiciona o LFSR slot-rate `x^23+x^9+1`, com um passo por slot de quatro clocks. O contrato medido é 18 passos por rotação de 72 clocks; HH no slot 13 e SD no slot 16, separados por três passos. Seed físico continua desconhecido.
- `instrument.rs` contém cinco conjuntos candidatos de parâmetros por operador derivados das capturas: BD, HH, SD, TOM e TCY. BD ainda apresenta diferença residual de alinhamento de uma amostra e pico; os bytes não são declarados como dump da ROM física.
- Testes adicionados cobrem a lógica combinacional, a relação de decimação do LFSR, o espaçamento HH→SD, limites de fase, parâmetros candidatos e snapshot/restore. São testes internos do modelo, não comparação física independente.
- A repetição medida de cada voz foi mapeada aos slots 0–2 e 6–7, além dos slots originais 13–17. O buffer de atraso ainda não foi implementado.

**Estado:** FM-7 segue PARCIAL. Os blocos de fase, ruído e parâmetros de operador estão implementados como candidatos isolados, mas não há integração de cinco vozes ao banco de síntese, buffer de duplicação, roteamento MO/RO, PCM, Scheduler ou validação contínua/blocos. A fase inicial dos operadores e o seed do LFSR permanecem desconhecidos.
