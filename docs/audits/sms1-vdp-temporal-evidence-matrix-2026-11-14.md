# SMS1 VDP — matriz de evidências temporais

**Data:** 15/11/2026
**Baseline:** `main` em `a41c7706c2527858fb4454837afca21b4f0ae108` (pós-PR #910)
**ASIC-alvo:** Sega 315-5124 / SMS1
**Estado:** auditoria em andamento; sem liberação in-game.

## 1. Como interpretar as evidências

As fontes não têm o mesmo peso nem representam o mesmo tipo de evidência:

| Classe | Fonte | O que sustenta | O que não sustenta |
|---|---|---|---|
| Documentação pública | [SMS Power — Master System VDP](https://www.smspower.org/Development/VDP) | Registradores, portas, comportamento funcional documentado | Não fornece, para todos os eventos, timestamps elétricos dot-a-dot |
| Documentação de periférico | [SMS Power — Light Phaser](https://www.smspower.org/Development/LightPhaser) | Relação observável entre TH, Light Phaser e H-counter | Não fecha sozinho a borda física e o atraso interno de captura |
| ROM de diagnóstico | [FluBBaOfWard/SmsVdpTest](https://github.com/FluBBaOfWard/SmsVdpTest) | Expectativas e procedimentos do teste para H-counter, HINT, VINT, VCounter e overflow | Este projeto ainda não executou a ROM no runtime nem em hardware físico |
| Modelo comparativo | [MAME — SMS VDP implementation](https://github.com/mamedev/mame/blob/master/src/mame/sega/315_5124.cpp) e [SMS machine/peripheral model](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms_m.cpp) | Uma implementação independente e seus limites/hipóteses | Não é medição física independente |
| CPU | Manuais Zilog Z80 anexados ao projeto | Semântica Z80, ciclos de barramento e amostragem de INT | Não determina os latches internos do VDP |
| Testes do projeto | Workflows Rust/Release | Determinismo e consistência do modelo implementado | Não comprovam, isoladamente, equivalência elétrica com um SMS real |

**Medições físicas executadas neste projeto: nenhuma.** Os testes automatizados validam o modelo temporal e sua integração, não substituem uma captura de hardware ou a execução da ROM de diagnóstico em um SMS real.

## 2. Divergência central de timing

O modelo SMS1 atual usa boundaries CPU-visíveis derivados da descrição do SmsVdpTest:

| Evento | Modelo atual | Base de timing |
|---|---:|---|
| VINT IRQ pendente | HPos≈20 | T13 |
| HINT/R10 IRQ pendente | HPos≈21 | T14 |
| VCounter latch | HPos≈23 | T15 |
| Sprite overflow / publicação do frame | HPos≈24 | T16 |
| VBlank status flag | HPos≈25 | T17 |
| Fechamento da área ativa / HBlank modelado | HPos 319 | T213 |

O modelo MAME 315-5124 usa, em sua tabela de timing, VINT_HPOS 24, VINT_FLAG_HPOS 24, HINT_HPOS 26, VCOUNTER_HPOS 23 e OVERFLOW_HPOS 24; também lista XSCROLL_HPOS 21 e COLLISION_HPOS 59. Isso diverge principalmente dos eventos VINT/HINT adotados pelo código atual.

Os valores MAME são uma referência comparativa de implementação, não uma medição. Os valores do SmsVdpTest são a base diagnóstica adotada no código, mas a ROM ainda não foi executada neste projeto. Portanto, HPos 20/T13 e HPos 21/T14 permanecem **hipóteses de trabalho testadas internamente**, não timing físico fechado.

Não converter HClock em HPos por simples igualdade: as referências usam origens e convenções de coordenadas distintas.

## 3. Matriz técnica

| Recurso | Implementação atual | Evidência externa | Teste existente | Pendência |
|---|---|---|---|---|
| HCounter / porta $7F | Conversão HPos/T-state e latch por transição TH via software; leitura espelhada mantém valor capturado | SMS Power descreve latch relacionado ao TH do Light Phaser; SmsVdpTest documenta o teste TH 0→1; MAME implementa sua própria conversão e latch | Leitura estável por $7F, espelhamento, captura TH software e invariância de chunking | Borda e atraso do latch por TH externo; HCounter dot-phase; validação física |
| HBlank | T213/HPos319 fecha o span de 256 pixels; T212 não fecha o último pixel | A documentação pública consultada não fecha o dot físico exato; MAME é apenas comparador | T212/T213, pixel 255, colisão e leitura de status | Boundary elétrico real, border e fetch pipeline |
| HINT / R10 | R10 decrementa/recarrega e gera IRQ pendente em T14 (HPos≈21) | README SmsVdpTest: HINT em HClock −17/HCount `$F3`; MAME: HINT_HPOS26, origem ainda não normalizada | Antes/depois T13/T14, snapshot/restore e propagação até INT do Z80 | Captura CPU-observável após HALT/ISR/TH ainda não reproduzida; não mover T14 sem fixture discriminante |
| VBlank / VINT | VINT IRQ pendente em T13/HPos≈20; publicação do frame T16; status VBlank em T17/HPos≈25 | README SmsVdpTest: VINT em HClock −18/HCount `$F3`, VBlank flag em HClock −15/HCount `$F5`; MAME: VINT_HPOS24/VINT_FLAG_HPOS24, origem não normalizada | T12/T13/T17, snapshot/restore, contínuo versus blocos e serviço de VBlank pelo Z80 | `Frame IRQ HCount` ainda sem reprodução CPU-observável; separar instante do pino INT, amostragem HALT T4, acknowledge e flag de status |
| Sprite overflow | SAT é avaliada por scanline; índice/status latched em T16/HPos24 | SmsVdpTest e MAME fornecem referências de overflow, mas não constituem medição feita neste projeto | Boundary T16, índice, buffers Mode 4 e legacy/TMS | Índice físico exato, interação com fetch SAT e seleção em casos-limite |
| Sprite collision | Composição identifica pixels opacos sobrepostos; status é publicado quando o commit alcança o pixel; status read reconhece | SmsVdpTest fornece testes funcionais; MAME usa um boundary de colisão próprio no modelo | Colisão por pixel, último pixel ativo, status read e limpeza | Pipeline por dot, instante físico exato e interação com fetch-ahead |
| CRAM timing | Spans preservam prefixo/sufixo de paleta em escritas relevantes | SMS Power descreve CRAM funcionalmente; fontes consultadas não fecham o latch dot-a-dot | Escritas intralinha, mudança de backdrop e spans | CRAM dot, instante físico de latch e comportamento durante fetch |
| VRAM / pattern fetch | Escritas commitam o prefixo; tiles/bitplanes ainda são consultados na composição | Documentação descreve VRAM e padrões, mas não fornece timestamps individuais de fetch para o modelo atual | Escritas intralinha, transições de modo e regressões de renderização | Fetch-ahead de tiles, bitplanes, padrões de sprites e WAIT states reais |
| SAT evaluation | Buffers separados Mode 4 e legacy/TMS; metadados de sprites capturados para a próxima scanline | Documentação descreve SAT e limites funcionais; MAME oferece modelo comparativo | Seleção, overflow, preservação de metadados e transição de modo | Timestamps por entrada SAT/X/tile/pattern e fetch real de padrões |
| Scheduler / CPU INT | VDP avança pelo Scheduler; `sync_vdp_interrupt_line()` roda após cada dispatch e antes de `begin_instruction()`; HALT amostra INT em T4 | Zilog UM008011-0816: INT aceito ao fim da instrução; HALT repete M1/NOP e amostra em T4. SmsVdpTest mede após a ISR, não no instante bruto do pino | HINT T14 até linha INT; testes Z80 de HALT T4; VINT; chunking e restore | Falta fixture end-to-end que execute a sequência de HALT/ISR/TH e leia `$7F` após Frame/Line IRQ |

## 4. Evidência automatizada integrada

Baseline atualizada após PR #909 e #910. A PR #910 foi validada no HEAD `3fb41e8184a08ab07e5a258ae9d1239fbe40749a`; CI e SMS Debugger Runtime passaram, incluindo Scheduler deterministic stress, workspace regression, SMS Runtime Release e Windows x64 Release. Merge commit: `a41c7706c2527858fb4454837afca21b4f0ae108`.

No HEAD da PR #843, antes do merge:

- 98 testes `sega-video`.
- 131 testes `sega-sms-core`.
- 14 testes de integração SMS.
- Testes direcionados de VINT contínuo versus blocos, snapshot em T12, IRQ em T13 e flag em T16.
- Testes de HINT/R10 antes/depois de T14, restore e propagação até a linha INT do Z80.
- Z80 focused, input focused, Scheduler deterministic stress e workspace regression aprovados.
- Rustfmt, Clippy, runtime Release e build Windows x64 Release aprovados.

Esses resultados provam consistência do modelo nos cenários cobertos. Não provam que os timestamps escolhidos são os mesmos do ASIC físico.

## 5. Próximos passos técnicos

1. Construir fixture reproduzível para `RunTest32` (Frame IRQ HCount) e `RunTest34` (Line IRQ HCount), incluindo HALT, amostragem T4, acknowledge, ISR, TH rising edge e leitura de `$7F`.
2. Registrar a primeira divergência em timestamp de VDP, atualização da linha INT, saída de HALT, acknowledge, retorno da ISR ou latch TH; não mover T13/T14 sem teste discriminante.
3. Executar a ROM SmsVdpTest no runtime SMS1 quando o runner/artefato estiver disponível e comparar com a fixture, sem tratar README como execução.
3. Validar H-counter externo/Light Phaser: TH low/high, instante de latch e atraso; não importar o default MAME de 19 HPos como constante universal.
4. Obter medição física ou referência de hardware reproduzível para HBlank, CRAM dot e fetch-ahead.
5. Expandir testes de clone/restore e chunking para overflow, colisão e escritas CRAM/VRAM em todos os boundaries relevantes.

**Conclusão:** VDP SMS1 continua NÃO LIBERADO PARA TESTE IN-GAME. A temporalidade CPU-visível está mais bem coberta e determinística, mas há divergências entre fontes e pendências de fetch/latch físico que impedem declarar fidelidade temporal suficiente.


## 6. Auditoria Frame IRQ HCount / Line IRQ HCount — 15/11/2026

### Evidência primária da ROM SmsVdpTest v1.31

O README do teste documenta, em sua própria convenção HClock:

- VINT: HClock −18, HCount `$F3`.
- HINT: HClock −17, HCount `$F3`.
- VINT status flag: HClock −15, HCount `$F5`.

O código-fonte confirma que os testes não observam apenas um booleano interno:

- `RunTest32` alinha a execução por polling do HCounter (`$F2` e `$F1`), habilita interrupções, entra em `HALT`, retorna da ISR, provoca TH 0→1 em `$3F` e lê `$7F`. As duas amostras esperadas são `$82` e `$84`.
- `RunTest34` configura HINT/R10 para a linha `$80`, aguarda a interrupção por `HALT`, retorna da ISR, captura TH e lê `$7F`. As duas amostras esperadas são `$88` e `$8A`.
- Portanto, esses casos exercitam o caminho completo VDP → linha INT → amostragem Z80 em HALT → acknowledge/ISR → retorno → TH latch → leitura HCounter. Um teste que apenas verifica `interrupt_pending()` ou `int_pending` após T13/T14 não substitui essa observação.

### Estado de implementação no HEAD atual

- `SmsVdp` gera VINT pendente em T13 e HINT/R10 em T14.
- `SmsTemporalMachine::dispatch_next()` sincroniza a saída VDP com a linha INT após cada evento do Scheduler; `begin_instruction()` também sincroniza antes da fronteira de instrução.
- O Z80 possui amostragem de INT em T4 durante HALT e testes próprios para atraso entre M1s, EI/HALT e latch da saída de HALT.
- A captura HCounter é disparada pela transição TH software 0→1 em `$3F`; `$7F` devolve o valor latched.
- Os testes SMS existentes verificam os boundaries T13/T14 e a propagação da linha INT, mas ainda não executam a sequência completa de `RunTest32`/`RunTest34` e não comparam as amostras finais `$82/$84` e `$88/$8A`.

### Interpretação e decisão

A documentação SMS Power/Charles MacDonald descreve o comportamento vertical do frame interrupt e a contagem/reload do line interrupt, mas declara que a posição horizontal intra-scanline desses eventos não é conhecida nessa referência. A ROM SmsVdpTest é evidência de software diagnóstica mais específica, mas não equivale a uma medição física independente.

A tabela MAME 315-5124 usa VINT_HPOS 24 e HINT_HPOS 26. Não comparar esses números diretamente com HClock −18/−17 sem normalizar a origem horizontal de cada implementação. MAME permanece comparador, não oráculo.

**Conclusão desta etapa:** a primeira divergência ainda não foi isolada. Não há justificativa para alterar as constantes T13/T14, a amostragem T4 do Z80 ou o Scheduler. A próxima alteração deve ser a fixture end-to-end descrita acima; só então escolher entre correção convencional por T-state ou granularidade híbrida localizada. Nenhuma medição física foi realizada.
