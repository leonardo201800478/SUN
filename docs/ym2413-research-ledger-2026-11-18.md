# YM2413 — Caderno de evidências de saída, EG e Rhythm Mode

**Data:** 2026-11-18
**Escopo:** consolidar pesquisas recentes e definir fronteiras de evidência para a implementação YM2413.
**Status:** documento de engenharia; não representa conclusão de FM-1/FM-5/FM-6 e não altera o runtime.

## 1. Regras de leitura das evidências

Toda afirmação técnica deve ser classificada em uma destas categorias:

- **Especificação primária:** manual/datasheet Yamaha ou documentação técnica do fabricante.
- **Medição/reengenharia:** observações experimentais publicadas por pesquisadores (por exemplo, Andete no SMS Power).
- **Referência comportamental:** código de outro emulador. É útil para gerar hipóteses e casos comparativos, mas não constitui prova física independente.
- **Hipótese/candidato:** fórmula, limiar, fase, ganho ou circuito ainda sem medição suficiente. Deve permanecer parametrizado e explicitamente identificado.

Não converter código de referência em “verdade do silício” sem rastrear a origem. A documentação SMS Power sobre die-shots alerta que o material citado para determinados derivados OPLL/FHB não equivale a um die-shot confirmado do YM2413. Portanto, evitar a frase “confirmado pelo die-shot do YM2413” ao descrever descobertas do Nuked-OPLL.

## 2. Saída analógica MO/RO e temporização

### 2.1 Pinagem e nomenclatura

Para o encapsulamento DIP-18 documentado no manual/datasheet YM2413:

| Pino | Sinal | Observação |
|---:|---|---|
| 14 | MO | Saída analógica |
| 15 | RO | Saída analógica |
| 16 | VCC | Alimentação |

**Correção importante:** MO não é o pino 16. A proposta anterior que identificava MO como pino 16 estava incorreta.

O manual descreve MO/RO como saídas do tipo *source follower* e indica que o sinal requer circuito externo de processamento/amplificação. Não modelar automaticamente essas saídas como dreno aberto ou como uma fonte ideal de tensão sem resistência/carga.

O datasheet descreve conversão de 9 bits e especifica condições elétricas de saída. Valores de tensão devem sempre ser citados junto da carga e das condições do ensaio; não transformar uma especificação de carga de 2,2 kΩ em 2,2 Ω.

### 2.2 Relógio e cadência

A frequência de referência comumente documentada para o sistema é 3.579.545 Hz. A organização de 18 slots com quatro clocks por slot resulta em:

- 72 clocks por rotação completa;
- 49.715,90 rotações/atualizações por segundo (3.579.545 / 72);
- 894.886,25 slots por segundo (3.579.545 / 4).

A distinção de unidades é essencial: 49.715,90 é aproximadamente **49,716 kHz**, não 49,716 Hz; 894.886,25 é aproximadamente **894,886 kHz**, não 894,886 Hz.

A pesquisa comunitária sobre a saída descreve janelas ativas e intervalos neutros por canal. Isso é uma pista para reconstrução temporal, não autorização para substituir a cadência já implementada no núcleo sem vetor independente.

### 2.3 Proposta de filtro analógico: estado

Os seguintes componentes propostos ainda **não** estão certificados para uma revisão específica de placa:

- pull-down de 1–2,2 kΩ;
- resistores de soma de 10 kΩ;
- capacitor de 2,2 nF e corte LPF de 12–16 kHz;
- acoplamento AC com corte HPF de 10 Hz;
- ganho mestre 1/16384;
- ganho FM relativo ao PSG de +2/+3 dB.

Esses valores não devem virar defaults globais. Revisões de placas de expansão FM e a implementação japonesa integrada podem possuir topologias distintas. Exigir esquema elétrico da revisão-alvo ou medição de hardware antes de selecionar componentes/ganhos.

A fórmula bilinear/Tustin para um RC de primeira ordem pode estar matematicamente correta e ainda assim representar o sinal errado se aplicada apenas a amostras já somadas em 49,716 kHz. Se a hipótese física incluir pulsos ativos e intervalos neutros dentro dos slots, o modelo precisa preservar essas transições antes da filtragem e da decimação. Interpolação linear/cúbica não equivale a um filtro de reconstrução band-limited.

### 2.4 Arquitetura recomendada para a saída

Manter três camadas separadas:

1. **Saída digital temporal do chip:** posição do slot, estado de saída, transições, janelas ativas/neutras e valores digitais correspondentes.
2. **Modelo elétrico da placa:** carga, soma, resistores, filtros e acoplamento, selecionados por revisão de hardware documentada.
3. **Host de áudio:** reamostragem para a taxa CPAL e mixagem com PSG. O filtro do circuito FM deve atuar no ramo FM, sem modificar o estado temporal do chip.

Não introduzir callbacks de áudio por instrução nem permitir que CPAL controle o clock YM2413/Scheduler.

## 3. Rhythm Mode — mapa funcional e pontos não confirmados

### 3.1 Organização de vozes

A documentação Yamaha e as referências técnicas descrevem o modo de ritmo como reutilização dos canais/operadores finais para cinco vozes:

| Voz | Recursos associados | Estado de pesquisa |
|---|---|---|
| Bass Drum (BD) | Par de operadores do canal 6 | A estrutura de dois operadores é referência; validar parâmetros e nível |
| High-Hat (HH) | Modulador do canal 7 + lógica de fase/ruído | Fórmula combinacional ainda candidata |
| Snare Drum (SD) | Portadora do canal 7 + fase/ruído | Fórmula combinacional ainda candidata |
| Tom-Tom (TOM) | Modulador do canal 8 | Fase/roteamento precisam de validação no pipeline |
| Top Cymbal (TCY) | Portadora do canal 8 + lógica compartilhada de fase | Fórmula combinacional ainda candidata |

A documentação do projeto já possui mapa dos seis slots e primitiva LFSR. Isso não significa que as cinco vozes estejam sintetizadas, temporizadas ou misturadas corretamente.

### 3.2 LFSR e fórmulas de fase

O material de pesquisa anexado propõe um LFSR de 23 bits, taps nos bits 0/14/15/22, avanço por clock mestre e leitura do LSB. Também propõe equações para rm_bit e palavras de fase HH/SD/TCY.

Essas fórmulas devem ser tratadas como **candidatas até serem confrontadas com uma referência de implementação rastreável e vetores independentes**. O texto fornecido contém inconsistências internas na construção da fase HH (descrições de bits inferiores e tabela de estados não são totalmente coerentes). Não copiar diretamente a implementação Rust apresentada no texto para o núcleo.

A referência Nuked-OPLL pode ser usada para gerar vetores comparativos. Registrar sempre:
- versão/commit exato;
- fase/ciclo de leitura de cada operador;
- estado anterior/atual dos acumuladores;
- estado e instante do LFSR;
- palavra de fase resultante;
- saída de operador após log-sine/EG;
- saída final por slot.

O código de outro emulador é referência comportamental, não prova física. Comparações entre emuladores devem registrar divergências em vez de escolher automaticamente um lado como verdade.

### 3.3 Pipeline e ordem de atualização

O texto anexado propõe que HH leia estado de fase de CAR8 em um instante diferente de TCY, e que as leituras do LFSR sejam separadas por slots. Isso é uma hipótese de dependência temporal que deve ser testada contra o pipeline real.

Antes de alterar a ordem dos slots:
1. documentar o mapeamento físico slot -> operador -> voz;
2. determinar se cada entrada é lida antes ou depois da atualização do acumulador;
3. determinar exatamente quando o LFSR avança e quando cada voz amostra seu estado;
4. construir vetores ciclo-a-ciclo para uma rotação completa de 72 clocks;
5. testar determinismo entre execução contínua e execução em blocos.

Não assumir que “4 clocks por slot” significa necessariamente que todo estado interno relevante atualiza quatro vezes por slot. O avanço de cada bloco deve seguir a evidência específica do circuito.

## 4. EG — conclusões utilizáveis e ressalvas

### 4.1 Taxa efetiva e KSR

O manual Yamaha descreve RATE = 4 × R + Rks e a condição especial R=0 => RATE=0, independentemente de KSR. Essa condição foi implementada na branch da PR #1024 e deve permanecer coberta por matriz BLOCK/FNUM/KSR.

Para R não nulo, o mapeamento KSR medido/documentado deve ser testado separadamente para KSR=0 e KSR=1. Não inferir o comportamento de R=0 apenas pela fórmula geral.

### 4.2 Escala e SL

As notas experimentais de Andete descrevem o alvo interno de sustain como 8 × SL. Isso não resolve sozinho:
- comparação por igualdade versus cruzamento/limiar;
- comportamento quando SL muda durante DECAY;
- tratamento de incremento que salta sobre o alvo;
- alinhamento exato da avaliação e do incremento no mesmo tick.

O texto anexado afirma resolução de 0,75 dB por nível. Essa afirmação não deve ser reproduzida como fato: a escala logarítmica e o deslocamento para o domínio de atenuação precisam ser tratados conforme a tabela/implementação de referência e a fonte experimental; 0,75 dB por unidade não foi estabelecido pelas fontes verificadas nesta auditoria.

### 4.3 DAMP

As notas experimentais descrevem duração da ordem de 10 ms sob condições específicas. O limiar 124, a fase de início do contador, o instante da transição e a coordenação modulador/portadora continuam candidatos. O teste com threshold 124 verifica a aritmética da configuração selecionada, não prova que o chip físico usa esse limiar.

### 4.4 Referências experimentais

- Andete, [YM2413 Reverse Engineering Notes — 2015-12-24](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-24).
- Andete, [YM2413 Reverse Engineering Notes — 2015-12-31](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-31).
- Andete, [YM2413 Reverse Engineering Notes — 2015-03-20](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20).
- Andete, [YM2413 Reverse Engineering Notes — 2015-03-27](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-27).
- Yamaha, [YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual).
- SMS Power, [YM2413](https://www.smspower.org/), incluindo a distinção entre YM2413 e derivados OPLL usados em pesquisas de die-shot.
- Nuked-OPLL, [repositório de referência](https://github.com/nukeykt/Nuked-OPLL). Usar como referência comportamental versionada; inspecionar TODOs e limitações antes de considerar qualquer saída como autoritativa.

## 5. Estado atual do projeto e próximos critérios

O scorecard YM2413 deve continuar sendo a fonte de estado das etapas FM-0 a FM-6. A biblioteca native/ym2413 contém componentes isolados para registradores/clock, PG, EG, operadores, LFO, instrumentos, canais e candidatos de ritmo; o lib.rs declara explicitamente que ainda não existe pipeline FM completo de 18 slots nem PCM integrado.

### Próximas tarefas recomendadas

1. Fechar o CI e revisar os vetores da PR #1024; não promover DAMP=124 a constante física.
2. Auditar rhythm_phase.rs, rhythm.rs, rhythm_channel.rs e synthesis.rs contra o documento, registrando divergências por símbolo e teste.
3. Adicionar testes exaustivos das quatro combinações HH/SD e do domínio de entrada de rm_bit, mas somente após corrigir as contradições da fórmula candidata com fonte versionada.
4. Construir um traço de uma rotação de 72 clocks para os seis slots de ritmo, sem alterar o MachineScheduler nesta etapa.
5. Auditar a implementação real MO/RO e estabelecer a revisão exata da placa antes de adicionar um filtro analógico padrão.
6. Só então planejar a integração do pipeline de 18 slots e PCM, preservando a autoridade temporal do MachineScheduler.

**Critério de não regressão:** nenhum trabalho deste documento autoriza alteração do Scheduler, temporização SMS já validada, PSG, barramentos ou dependências externas. Componentes candidatos permanecem isolados até existirem vetores independentes e cobertura de estado/snapshot suficientes.


---

## 6. Saída MO/RO — roteamento lógico por slot (2026-11-19)

A PR #1042 adicionou `native/ym2413/src/output.rs`, com um contrato puro de encaminhamento de eventos para os pinos de saída.

### MO — melodia

- Em modo melódico, os nove slots carrier (índices zero-based 3, 4, 5, 9, 10, 11, 15, 16 e 17) são associados aos canais 1–9 e encaminhados a MO.
- Em Rhythm Mode, apenas os carriers dos canais 1–6 (slots 3, 4, 5, 9, 10 e 11) permanecem no caminho melódico. Os canais 7–9 são reservados ao modo de percussão.

### RO — ritmo

- Eventos originais e repetições atrasadas são validados contra `YM2413_RHYTHM_OUTPUT_MAP` e encaminhados a RO.
- O estado `RepeatNotInitialized` é preservado como condição distinta de `NoRhythmOutput`; nenhum dos casos é convertido em nível zero.

### Janela de saída

A API registra três input clocks ativos e um clock neutro dentro de cada slot de quatro clocks. A evidência disponível não identifica a fase exata do pulso dentro do slot; por isso o tipo `Ym2413OutputPulse` expõe somente largura, slot, pino, origem lógica e nível medido.

### Evidência e limites

- Manual Yamaha: MO é saída melódica e RO é saída rítmica; a tabela de slots fornece a alocação serial.
- Medições comunitárias publicadas: multiplexação em uma rotação de 72 clocks e permanência do valor de cada canal por três clocks antes do retorno ao nível neutro.
- O valor fornecido pela API é `Ym2413DacCode`, domínio de amplitude assinada reconstruído a partir das medições. Não é uma palavra bruta de DAC.
- Não foi implementada conversão operador→DAC, posição sub-clock, nível neutro elétrico, circuito source-follower, filtro de placa ou reamostragem.
- Não há ligação a PG/EG, ao banco de síntese, ao MachineScheduler ou ao runtime SMS.

**CI:** workflow #44450484181, gate Rust final SUCCESS; format, Clippy, testes de workspace e testes/Clippy de observability SUCCESS. Gates de subsistemas não afetados foram SKIPPED.



## Atualização do bloqueio operador→DAC — variantes de polaridade (2026-11-19)

A auditoria adicional das notas de Andete identificou duas expressões de saída em modelos publicados:

- Nota 2015-12-03: `lookupExp(carrier) + 256`. O próprio texto informa que a forma de onda medida aparece invertida e suspeita de inversão de sinal no setup.
- Nota 2015-12-12: `255 - c`, no modelo de feedback que reproduz as capturas posteriores.
- Nota 2015-03-11: níveis reconstruídos como -255..+255, incluindo -0 e +0; a aquisição ADC apresenta ruído e espaçamento não uniforme.

Para o intermediário candidato x ∈ [-256,255], as fórmulas `x+256` e `255-x` são complementos exatos no domínio de 9 bits (soma 511). Invertem a polaridade e trocam os códigos dos dois zeros. Portanto, não são diferenças triviais de arredondamento.

A Issue #1031 continua aberta: nenhum desses modelos demonstra diretamente a palavra interna aplicada ao DAC físico. Não implementar conversão padrão, clamp de -256, saturação, wrap ou bias. A nova auditoria detalhada está em `docs/audits/ym2413-operator-dac-boundary-variants-2026-11-19.md`.


---

## 7. EG — fronteira DAMP, KEY-ON e atualização da nova fase (2026-11-19)

### 7.1 Evento de conclusão e reset de PG

As PRs #1045 e #1047 separam a detecção do fim de DAMP do consumo do evento:

- `EnvelopeGenerator::tick_with_events` informa a conclusão candidata de DAMP.
- O EG não avança clock nem reseta PG automaticamente.
- O banco agregado oferece consumo explícito do evento. A política `CarrierDrivesBoth` sincroniza o estágio do modulator e reseta os dois PGs; a política independente reseta somente o PG do operador que gerou o evento.
- A escolha do instante de consumo continua externa. Nenhuma subfase ou ordem física foi inferida.

### 7.2 Latch KEY-ON na sincronização carrier-driven

A PR #1049 identificou uma inconsistência entre o estado de KEY-ON compartilhado pelo canal e a cópia local mantida por cada EG. Se o carrier completasse DAMP antes do primeiro tick do modulator, sincronizar somente o estágio deixava o latch local do modulator desatualizado. O próximo tick poderia interpretar o mesmo KEY-ON como nova borda e reiniciar DAMP.

A correção propaga o latch observado pelo carrier junto com o estágio, somente no candidato explícito `CarrierDrivesBoth`. Isso fecha a consistência interna da API; não decide se essa política é o comportamento físico definitivo.

### 7.3 DAMP concluído na entrada da atualização

A PR #1050 alinhou o modelo candidato com a ordem de `Operator::calcEnv` em Andete `results/20170126/model5.cc`:

- Se o limiar já está satisfeito no início da atualização, o estado passa para ATTACK/DECAY antes do seletor de fase, e o novo estágio pode consumir o mesmo valor do contador global.
- Se o incremento de DAMP alcança o limiar durante a atualização corrente, a transição é selecionada após esse incremento; o novo estágio começa na atualização seguinte.
- Para AR=15 no candidato `SkipAttackToDecay`, o estágio passa a DECAY, mas o nível de atenuação não é zerado artificialmente.

O teste de regressão usa nível 124, AR efetivo 40 e contador 8: a atualização seleciona ATTACK e produz 124 → 116. Isso valida a sequência do modelo de referência no domínio do software, não o instante físico de uma subfase.

### 7.4 Divergência SUS × EG-TYPE em KEY-OFF

A descrição experimental Andete de 2015-12-31 indica, para timbre sustentado, SUS=ON → taxa 5 e SUS=OFF → RR programado. O modelo de software Andete de 2017 em `results/20170126/model5.cc` seleciona RR para EG-TYPE sustentado sem consultar SUS. O manual Yamaha descreve SUS=ON como taxa 5 em KEY-OFF.

A divergência está registrada na Issue #1051. Não alterar a implementação até construir a matriz EG-TYPE × SUS × RR e reconciliar a evidência. O comportamento do modulator deve ser analisado separadamente, pois suas transições de KEY-OFF não são iguais às do carrier.

### 7.5 Validação de software

PR #1050 foi integrada após CI #44512860362 (run #8835) SUCCESS nos gates aplicáveis: Rustfmt, Clippy workspace, testes unitários/integração e Clippy/testes observability. Os gates de subsistemas não afetados foram SKIPPED.

**Limite geral:** nenhum desses marcos define a cadência física do contador EG, o alinhamento absoluto pós-reset ou a posição do tick dentro dos 18 slots. O EG continua isolado do pipeline temporal e do PCM final.


---

## 8. KEY-OFF — regressões por fase e distinção carrier/modulator (2026-11-19)

A PR #1053 adicionou cobertura de regressão para o comportamento KEY-OFF descrito nas medições Andete de 2015-12-31:

- Carrier: KEY-ON→OFF usa o caminho de release de segunda/única fase quando parte de DAMP, ATTACK, DECAY ou SUSTAIN.
- Modulator EG-TYPE=1: o envelope permanece no nível de sustain após KEY-OFF.
- Modulator EG-TYPE=0: o envelope continua na primeira taxa RR, sem usar o segundo release do carrier.

A cobertura confirma a implementação candidata nas combinações testadas; não resolve a divergência de seleção de taxa do carrier para EG-TYPE=1 e SUS=ON/OFF. Essa questão permanece aberta na Issue #1051 e não deve ser ocultada por estes testes.

**CI:** #44513838591 (run #8840) SUCCESS para Rustfmt, Clippy workspace, testes unitários/integração e Clippy/testes observability. Os gates não relacionados foram SKIPPED.


---

## 9. PG integer arithmetic and 18-slot accumulator convergence — current-main re-run (2026-11-20)

PR #1119 reapplied the staged Nuked integer truncation in `PhaseGenerator::phase_step` and was squash-merged to main as `fc166cc8cca2e3a4a4d5c3565b5d125d3dc20408`.

Current-main differential evidence against Nuked-OPLL `1269cf5a783b65583b50fa2464d08be75830aaa0`:

- 147,456 `OPLL_Clock` calls; 146,885 PM-enabled calls; all PM positions, FNUM high rows and BLOCK values covered; zero PG increment mismatches.
- 2,654,208 phase accumulator values compared across all 18 slots; zero low-19-bit mismatches; 5,076 Rust wrap events.
- Separate KEY-ON phase-reset response trace: 2,600 calls, 46,800 phase values, 54 wraps, two oracle-supplied reset events and zero phase mismatches.
- The phase-reset response trace does not yet validate event generation by the Rust serial EG core; that remains an integration requirement.
- MAME/ymfm source-level static audit completed in the same workflow. It supports base/no-PM agreement after unit normalization and documents unresolved PM fractional-rounding differences; no MAME executable/runtime differential was performed.

CI evidence:
- Rust workspace regression and Rust quality/unit regression: SUCCESS.
- Nuked PG differential: SUCCESS.
- External Nuked oracle adapter: SUCCESS.
- Existing serial EG characterization: SUCCESS.
- Unrelated Z80, SMS, Scheduler, Windows audio/input and Slint/WGPU jobs were SKIPPED; they are not promoted to PASS.

**Research conclusion:** the PG integer increment and the tested 18-slot accumulator path now have reproducible software-oracle evidence. This does not close FM-1B: the Rust EG reset event is not yet connected to PG, and the combined PG/EG/operator/feedback pipeline, Rhythm Mode output, DAC and PCM remain incomplete. The explicit slot-service mapping/event contract was integrated by PR #1124 (merge 8a80fc784c3b9b2563cea248f5c3ba1b4073dfc9). The core-owned pre/post EG service hooks are under PR #1126; PG/EG synthesis state is not connected yet.
