# YM2413 FM Completion Scorecard

**Data de referência:** 2026-11-18
**Repositório:** `leonardo201800478/sega-emulator`
**Main após FM-3 parcial:** `daf950d0e0bf3242dfb213d5e9e50a24db19a70e`

Este scorecard distingue implementação de componente, integração funcional, validação temporal e evidência física. Uma etapa só é considerada concluída quando seus critérios próprios estão demonstrados.

## Estado executivo

**Status geral: EM DESENVOLVIMENTO — NÃO PRONTO PARA TESTES CONTROLADOS COM ROMS.**

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
| FM-5 — Rhythm Mode | **Pendente** | Nenhum gerador de ritmo integrado | Cinco vozes, seis slots, noise LFSR, volumes e temporização |
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
| Rhythm / LFSR | Apenas bit RHYTHM de $0E decodificado | Vozes, LFSR e temporização não implementados |
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
7. **Rhythm LFSR:** o polinômio medido é conhecido, mas o estado inicial/reset físico não está estabelecido.
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
