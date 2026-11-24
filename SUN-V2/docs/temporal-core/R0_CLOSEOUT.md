# R0 — parecer de fechamento da pesquisa do Temporal Core

**Projeto:** SUN V2

**Componente:** CHRONOS

**Máquina inicial:** Sega Master System

**Gate:** R0 — pesquisa e evidências

**Parecer:** **APROVADO NO ESCOPO DOCUMENTAL**

**Data:** 2026-11-24

**Issue:** [#1267 — R0: pesquisa e especificação do Temporal Core](https://github.com/leonardo201800478/SUN/issues/1267)

> Este parecer fecha a pesquisa e a classificação de evidências. Não aprova uma implementação de produção, não certifica fidelidade física universal e não escolhe representação temporal, estrutura de agenda, fase de eventos ou granularidade final.

## 1. Decisão

R0 está concluído porque os requisitos temporais relevantes para iniciar a especificação foram identificados, as fontes e seus limites estão registrados, as divergências materiais foram classificadas e cada incerteza remanescente recebeu tratamento explícito.

A ausência de medições físicas PAL-M, WAIT externo e microtiming de todos os ASICs não bloqueia o modelo temporal genérico nem a especificação R1. Esses pontos permanecem UNKNOWN e devem ser representados como parâmetros/perfis ou testes pendentes — nunca convertidos em constantes presumidas.

**R1 está autorizado.** R2, R3, R4 e R5 permanecem gates próprios. Nenhum crate de produção CHRONOS está autorizado por este parecer.

## 2. Evidência Z80 e contrato de barramento

Fonte primária: [Zilog Z80 CPU User Manual UM008011-0816](https://www.zilog.com/docs/z80/um0080.pdf), revisão documental de agosto de 2016, páginas impressas 7–14.

Fatos documentados:
- Instruções são sequências de operações básicas organizadas em M-cycles e T-states.
- M1 de fetch tem quatro, cinco ou seis T-states conforme a operação e estados de espera; ciclos comuns de memória têm normalmente três T-states, podendo ser alongados.
- WAIT é ativo baixo e amostrado na borda de descida durante T2 e em cada TW subsequente; WAIT ativo solicita outro TW.
- Ciclos de I/O inserem um TW automaticamente.
- INT é amostrada na borda de subida do último clock ao final da instrução; o acknowledge gera M1/IORQ e dois TW automáticos.
- NMI é acionada por borda negativa, tem prioridade sobre INT e é reconhecida ao final da instrução, independentemente de IFF, direcionando a execução para 0066h.
- EI habilita IFF1/IFF2, mas a aceitação de INT fica adiada até depois da instrução seguinte.

Consequência arquitetural:
- O contrato temporal precisa representar limites de amostragem, M/T-cycles, espera automática e espera externa sem confundi-los.
- CPU e barramento precisam suspender/retomar operações mantendo request, fase, amostragem, efeitos parciais e conclusão exatamente uma vez.
- O Z80 não é implementado pelo CHRONOS; o núcleo temporal coordena limites e causalidade.

A auditoria V1 no SHA congelado identificou Z80MicroSequencer retomável, request ID, WAIT acumulado e conclusão agendada. Isso é evidência da implementação V1, não uma API a copiar nem prova de WAIT físico.

## 3. Clocks e domínios

| Perfil/domínio | Evidência disponível | Limite explícito |
|---|---|---|
| NTSC VDP | 10.738635 MHz ±100 Hz no pino 30 do IC5, manual de serviço Sega | Evidência da revisão/documentação citada; não universalizar para todas as placas |
| NTSC Z80 | 3.579545 MHz ±30 Hz no pino 34, manual de serviço Sega | Fase inicial em relação ao VDP permanece UNKNOWN |
| NTSC PSG | Entrada nominal 3.579545 MHz e divisor interno /16, SMS Power | Frequência nominal não determina fase de reset nem forma analógica |
| PAL europeu | Candidatos 3.546893 MHz (SMS Power/referência de serviço) e 3.546895 MHz (derivação MAME) | Diferença de 2 Hz; tolerância de placa não localizada nas fontes consultadas; convenção deve permanecer explícita |
| PAL-M TecToy | MAME modela VDP 10.726833 MHz e Z80/PSG 3.575611 MHz; V1 usa 3.579545 MHz | Clock físico CPU/PSG da revisão TecToy-alvo UNKNOWN até esquema ou medição |
| YM2413 Japanese SMS | Manual de aplicação indica clock 3.579545 MHz e taxa derivada fM/72; MAME configura 10.738635/3 | Fase de reset e relação elétrica com CPU/VDP UNKNOWN |

Relações candidatas do modelo NTSC: VDP:Z80 = 3:1; pixel clock = VDP/2; PSG input = clock nominal do Z80; YM2413 do perfil japonês = clock nominal do Z80. São relações documentais/modeladas, não prova de fase elétrica comum.

**Tolerâncias não publicadas:** para PAL europeu, PAL-M TecToy e YM2413, as fontes consultadas não fornecem tolerância de oscilador/placa suficiente. R1 deve permitir perfil parametrizável e registrar tolerância UNKNOWN, sem inventar precisão.

Não aprovaram unidade temporal, epoch, u64/u128, razão pública, fase absoluta, granularidade sub-T-state ou estrutura de eventos. O candidato inteiro de 15 ticks por T-state NTSC é apenas uma hipótese matemática para protótipos posteriores.

## 4. VDP — limites observáveis e qualidade da evidência

A documentação SMS VDP, os modelos comparativos e o relatório histórico sustentam uma lista de pontos observáveis: leitura de H/V counters, latch por TH, portas de dados/controle, flags de VBlank, IRQs H/V, overflow/colisão de sprites, commits de registradores e VRAM/CRAM, scroll latch e limites de linha.

O modelo V1 documenta 228 T-states/linha, 342 HPos, VINT/HINT e boundaries seletivos de raster. A confiança é alta para o que o modelo/testes de software implementam e menor para correspondência elétrica entre revisões ASIC. A fonte de Charles MacDonald declara limites da evidência horizontal, incluindo dados derivados de TMS9918/Genesis e possibilidade de divergência em SMS real.

O relatório anexado comparativo_de_testes_vdp_sega_emulator_vs_ares.md registra diferenças de data ports, IRQs H/V, HCounter, colisões/overflow e X-Scroll latchtime. Como não fixa SHA do executável, hash da ROM, BIOS/configuração ou ambiente, é evidência histórica e fonte de cenários discriminantes — não aprovação do SHA V1 congelado.

A reexecução do SMS VDP Test v1.31 com ROM hash, SHA do executável, perfil, estado inicial e artefatos continua necessária antes de atribuir os resultados ao SHA congelado. A fixture full-machine de IRQ/HCounter/TH pertence ao modelo de referência R2 ou ao protótipo que dependa desses efeitos; não bloqueia o fechamento documental de R0.

**Decisão de escopo:** precisão seletiva nos efeitos observáveis por gameplay. Não exigir pipeline universal de fetch por dot nem dot-accuracy sem consumidor demonstrado.

## 5. Áudio e WAIT

### PSG / YM2413

- SMS Power documenta PSG integrado ao VDP, divisor /16, LFSR Sega de 16 bits e saída constante +1 quando o período de tom é 0 ou 1. O relatório de áudio do projeto que sugere portadora de 111.86 kHz não está sustentado por essa referência; não adotar a portadora sem teste discriminante.
- O reset do divisor/fase, forma analógica da placa e resposta de filtros continuam parcialmente desconhecidos.
- O manual de serviço indica saída do VDP, nível e banda de áudio, mas não fornece a função de transferência que justificaria filtros IIR de 35 Hz/12 kHz ou soft clipping tanh. Esses valores permanecem hipóteses de DSP, não características comprovadas do hardware.
- YM2413 é posterior à primeira fatia SMS com PSG. A arquitetura deve permitir múltiplas fontes temporizadas, mas a qualidade FM, fase e caminho analógico não estão aprovados.

### WAIT

- TW automático de I/O e dois TW automáticos de INT acknowledge são comportamento interno documentado do Z80.
- WAIT adicional V1 por fixture é mecanismo de teste; não é amostragem de sinal físico.
- Não há evidência suficiente nesta pesquisa para inserir WAIT artificial no VDP ou PSG integrado SMS.
- WAIT externo de cartridge permanece UNKNOWN. PSG discreta SG-1000/SC-3000 exige perfil próprio e não deve contaminar o perfil SMS.

## 6. Comparação arquitetural de referências

A pesquisa comparativa identificou modelos distintos, sem eleger um vencedor por familiaridade:
- **MAME:** timeslices/quanta, timers e ajuste de interleave.
- **ares:** corrotinas cooperativas por componente e pontos explícitos de sincronização.
- **Gearsystem:** CPU-led com orçamento mínimo por chamada e avanço acumulado dos dispositivos; não equivale a lockstep de um T-state.
- **BlastEm:** sincronização sob demanda nos pontos de observação, com limites de catch-up.
- **Genesis Plus GX:** execução SMS por linhas e tratamento de interrupções dependente da posição do ciclo.

Esses projetos são evidência de estratégias implementadas, não prova de hardware. As fontes e revisões inspecionadas estão registradas em REFERENCE_RESEARCH.md; os snapshots externos consultados em 2026-11-24 são:
- MAME: e37f88658ed84e0a07f84bdccbeed3bfd58c8eb1
- ares: d97dba2d661946bbb203a5de55ee19f60b3a23d6
- Gearsystem: 2fdd8f7c8ff3ae9523dafb7f1d8125776e96aae6
- BlastEm: 8d9e43ee50f7fb887236777d7349b8f9757f054d
- Genesis Plus GX: 2499cd3e3d1d8b7d0130d8b2825adb4cc608b2c0

## 7. Questões abertas e classificação

| Questão | Classificação | Tratamento |
|---|---|---|
| Fase inicial CPU/VDP/PSG e fase de reset | UNKNOWN; não bloqueia contratos genéricos | Representar fase/offset como dado de perfil; testar relações discriminantes em R2/R3 |
| PAL 3.546893 vs 3.546895 MHz e tolerância | Perfil regional pendente | Preservar candidatos, convenção e tolerância UNKNOWN; não arredondar silenciosamente |
| Clock CPU/PSG TecToy PAL-M | UNKNOWN físico | Não herdar automaticamente NTSC nem MAME; exigir esquema/medição para declarar fidelidade física PAL-M |
| WAIT externo do cartridge port | UNKNOWN físico | Contrato genérico de WAIT; ativação somente com evidência de cartucho/placa |
| Microtiming de H/V IRQ, HCounter/TH e boundaries por ASIC | Parcial/modelo de software | Cenários discriminantes em R2; reexecutar ROM antes de promover resultados históricos |
| Fetch-ahead, CRAM dots e pipeline por dot | Não demonstrado como requisito geral de gameplay | Adiar até jogo/teste demonstrar impacto |
| PSG N=0/N=1 e reset phase | Regra documental para constante +1; reset físico ainda aberto | Testes digitais/diferenciais antes de alegar fidelidade analógica |
| Filtros e clipping analógico | Hipóteses | Não codificar como comportamento de placa sem esquema/medição |
| YM2413 fase/qualidade física | Posterior à fatia PSG | Fase parametrizável; qualidade FM e placa em etapa posterior |
| Snapshot persistente completo da máquina | Não especificado | R1 define contrato; R2 prova restore/replay |

Nenhuma dessas incertezas exige inventar uma fase, clock ou comportamento para avançar à especificação. Se um perfil futuro depender de uma informação física desconhecida, esse perfil não poderá alegar precisão física até a evidência existir.

## 8. O que R0 autoriza

**Autorizado:**
1. R1 — especificar contratos verificáveis de avanço limitado, sincronização sob demanda, deadlines seletivos, causalidade, WAIT/transações suspensíveis, interrupções, snapshot e determinismo.
2. R2 — criar modelo de referência independente e propriedades executáveis.
3. Definir cenários discriminantes para as incertezas classificadas.

**Não autorizado:**
- crate CHRONOS de produção;
- escolher definitivamente u64/u128, racional, epoch ou unidade mestre;
- escolher heap, vetor, árvore, timing wheel, corrotinas ou quantum;
- definir fases elétricas ou desempate de hardware sem evidência;
- iniciar CPU/VDP/PSG/YM2413 de produção;
- declarar 60 FPS ou qualidade de áudio como validados.

R3 permanece condicionado à aprovação de R1 e R2. R4 exige revisão adversarial. R5 é a autorização formal necessária antes de código de produção.

## 9. Fontes principais

- [Zilog Z80 CPU User Manual UM008011-0816](https://www.zilog.com/docs/z80/um0080.pdf)
- [Sega Master System Service Manual](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf)
- [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate)
- [SMS Power — SN76489](https://www.smspower.org/Development/SN76489)
- [SMS Power — YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [Sega Master System VDP documentation — Charles MacDonald](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt)
- [MAME SMS implementation at pinned snapshot](https://github.com/mamedev/mame/blob/e37f88658ed84e0a07f84bdccbeed3bfd58c8eb1/src/mame/sega/sms.cpp)

Documentos internos complementares:
- V1_BASELINE_EVIDENCE_MATRIX.md
- V1_TO_V2_INVENTORY.md
- VDP_OBSERVABLE_BOUNDARIES.md
- CLOCK_DOMAIN_RESEARCH.md
- AUDIO_CLOCK_RESEARCH.md
- SOURCE_DISCREPANCIES.md
- REFERENCE_RESEARCH.md
- CHRONOS_MINIMAL_HYBRID_SPEC.md

## 10. Validação do gate

R0 é um gate documental/de pesquisa. Não requer Cargo, Clippy ou testes de código porque nenhum código Rust foi alterado. A validação desta entrega consiste em:
- consistência cruzada dos documentos e das classificações;
- proveniência e limitações das fontes;
- ausência de decisão física inventada;
- diff restrito a documentação;
- issue #1267 atualizada para refletir a decisão.

**Conclusão:** R0 aprovado no escopo documental. A arquitetura CHRONOS continua uma proposta de híbrido mínimo sob validação; R1 pode começar sem aguardar medições físicas que não são necessárias para o contrato genérico.
