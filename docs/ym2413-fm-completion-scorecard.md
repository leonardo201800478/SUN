# YM2413 FM Completion Scorecard

**Data de referência:** 2026-11-18
**Repositório:** `leonardo201800478/sega-emulator`
**Main após as integrações iniciais:** `995dce7025b189ba25ce82a2dbf665d9cce9fb14`

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

## Scorecard por etapa

| Etapa | Estado | Entrega existente | Pendência para conclusão |
|---|---|---|---|
| FM-0 — Auditoria e consolidação | **Concluída** | Main/PRs auditadas; PR #999 integrada; arquitetura documentada | Manter o scorecard atualizado conforme evidências novas |
| FM-1 — PG e EG | **Parcial** | PG-1; PM LFO global; EG candidate DAMP/ATTACK/DECAY/SUSTAIN; KSR; snapshots locais; 17 testes EG, 14 testes de tabelas/operadores e 6 testes KSL | Integrar 18 EGs ao pipeline de operadores; resolver contratos de update; integrar snapshot ao estado agregado; validar transições com vetores independentes |
| FM-2 — Operadores e síntese FM | **Parcial** | Tabelas Log-Sine/EXP; lookup de fase/sinal; par modulator/carrier; feedback de dois samples; KSL BLOCK/FNUM; snapshots locais | Integrar KSL ao decoder/canal, validar PCM diferencial, integrar aos slots e confirmar escala/quantização fisicamente |
| FM-3 — LFO e modulação | **Parcial** | Contador PM global e tabela PM 8×8 | AM LFO; conectar PM/AM aos operadores; comprovar fase e cadência |
| FM-4 — Instrumentos | **Pendente** | Decodificação de registradores já existente na fundação | Instrumento usuário; ROM de 15 instrumentos; parâmetros e validação dos nove canais melódicos |
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
| AM LFO | Não implementado | Pendente |
| Instrument ROM | Não implementada | Pendente |
| Rhythm / LFSR | Não implementados | Pendente |
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

O próximo desenvolvimento funcional é fechar FM-2 com vetores diferenciais de saída dos operadores e integração do KSL ao decoder de canal. Em paralelo, FM-3 precisa de AM LFO. Só depois da validação isolada da cadeia de síntese deve ser conectada aos 18 slots e ao Scheduler.

Não iniciar a integração SMS ou mixer como substituto da síntese FM ainda inexistente. Não declarar o chip pronto para ROMs até haver PCM FM reproduzível, integração temporal e testes de regressão.

## Capacidade de execução multiagente

A sessão de desenvolvimento utilizada para estes marcos não disponibiliza três executores independentes. As três frentes foram coordenadas sequencialmente; não houve execução paralela de três agentes. A separação de responsabilidades permanece válida para a próxima etapa, mas não se deve registrar trabalho paralelo inexistente.

## Referências técnicas

- [SMS Power — YM2413 reverse engineering index](https://www.smspower.org/Development/YM2413)
- [Yamaha YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [andete — decay/release measurements (2015-03-20)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20)
- [andete — attack measurements (2017-01-26)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2017-01-26)
- [andete — ADSR transitions and operator behavior (2015-12-31)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31)
- [andete — rhythm/noise LFSR (2018-05-13)](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2018-05-13)
