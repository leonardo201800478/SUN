# SMS1 VDP — auditoria de Frame IRQ HCount e Line IRQ HCount

**Data:** 15/11/2026
**Baseline:** `main` pós-PR #910, merge commit `a41c7706c2527858fb4454837afca21b4f0ae108`
**ASIC-alvo:** Sega 315-5124 (SMS1)
**Estado:** investigação reproduzível pendente; nenhuma constante temporal alterada.

## 1. Pergunta da auditoria

Determinar se os boundaries atuais VINT T13 e HINT/R10 T14 reproduzem os resultados CPU-observáveis dos testes `Frame IRQ HCount` e `Line IRQ HCount` da SmsVdpTest v1.31. Distinguir o timestamp interno do VDP do instante em que o Z80 amostra INT, inicia acknowledge, executa a ISR e o software captura HCounter via TH.

## 2. Evidência da SmsVdpTest v1.31

O README da ROM documenta:

| Evento | HClock | HCount |
|---|---:|---:|
| VINT IRQ | −18 | `$F3` |
| HINT IRQ | −17 | `$F3` |
| VINT status flag | −15 | `$F5` |

O código-fonte fornece procedimentos observáveis:

- `RunTest32` (`Frame IRQ HCount`) alinha o loop por HCounter `$F2` e `$F1`, habilita INT, entra em HALT, retorna da ISR, provoca TH 0→1 no port `$3F` e lê HCounter em `$7F`. As amostras de aceitação são `$82` e `$84`.
- `RunTest34` (`Line IRQ HCount`) configura HINT e R10 para `$80`, aguarda a IRQ com HALT, retorna da ISR, captura TH e lê `$7F`. As amostras esperadas são `$88` e `$8A`.
- As leituras finais incluem a latência da CPU e a ISR. Não são leituras diretas do timestamp do pino INT do VDP.

Fonte: [SmsVdpTest v1.31](https://github.com/FluBBaOfWard/SmsVdpTest), arquivos `testmain.asm` (RunTest32/34) e `VDPTest-ReadMe.txt`.

## 3. Implementação atual

- `native/video_core/src/sms/vdp.rs`: VINT pending em T13; HINT/R10 em T14; VCounter latch em T15; overflow e frame publication em T16; VBlank status em T17 após PR #910.
- `SmsVdp::interrupt_pending()`: combina VINT pending habilitado por R1 bit 5 e line IRQ pending habilitado por R0 bit 4.
- `SmsTemporalMachine::dispatch_next()`: sincroniza a saída IRQ do VDP com a linha INT do Z80 depois de cada evento despachado pelo Scheduler.
- `SmsTemporalMachine::begin_instruction()`: sincroniza novamente antes da fronteira de instrução.
- `Z80MicroSequencer`: HALT repete M1/NOP e possui amostragem de INT em T4; testes `halt_e7.rs` e `nmi_e7.rs` cobrem T4, atraso entre ciclos, EI/HALT e latch da saída de HALT.
- `SmsTemporalBus`: a transição TH software 0→1 no port `$3F` chama `latch_h_counter()`; leitura do port `$7F` retorna o valor capturado.

Os testes atuais confirmam boundaries T13/T14 e a propagação para a linha INT. Não foi localizado teste full-machine que reproduza as rotinas RunTest32/34 e compare as amostras finais de HCounter.

## 4. Referências e limites

### SMS Power / Charles MacDonald

A documentação descreve:
- Frame IRQ nas linhas verticais C1/E1/F1 para modos 192/224/240;
- comportamento do contador de linha: decremento durante a área ativa e a linha seguinte, reload fora desse período;
- bit 7 de status mantido até leitura de control port;
- posição horizontal intra-scanline de line/frame IRQ não conhecida naquela documentação.

Fonte: [Sega Master System VDP documentation](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt), seções 11 e 12.

### MAME 315-5124

A tabela de timing usa VINT_HPOS 24 e HINT_HPOS 26. Esses valores são úteis como implementação comparativa, mas sua origem horizontal ainda não foi normalizada com a convenção HClock do SmsVdpTest. Não importar esses valores como constantes sem fixture.

Fonte: [MAME 315_5124.cpp](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp).

### Zilog Z80

O manual UM008011-0816 especifica que INT é honrada ao final da instrução corrente e que durante HALT a CPU repete ciclos M1/NOP e amostra as linhas em T4. Os testes do microsequenciador já cobrem essa semântica. Isso define a resposta da CPU, não o instante interno do VDP.

## 5. Primeira divergência ainda não isolada

A baseline comparativa histórica registrou falhas em `Frame IRQ HCount` e `Line IRQ HCount`. A PR #910 alterou somente a flag de status VBlank de T16 para T17; não alterou os eventos VINT T13 nem HINT T14. Os dois testes de HCount ainda não foram reexecutados após esse merge.

O atual boundary T13/T14 é quantizado em T-states. O README descreve VINT/HINT em HClock −18/−17, próximos no tempo. Isso, por si só, não prova que a granularidade T-state seja insuficiente: o teste observa a consequência após amostragem T4, acknowledge, ISR e TH latch. É necessário reproduzir esse fluxo completo primeiro.

## 6. Próximo teste discriminante

Criar fixture Rust em `sega-sms-core` que execute, pelo Z80 real e MachineScheduler:

1. Setup de R0/R1/R10 equivalente à ROM.
2. Loop de alinhamento HCounter/TH equivalente a RunTest32 ou RunTest34.
3. EI e HALT.
4. Acknowledge e ISR com a mesma sequência de ciclos da ROM.
5. Retorno, TH 0→1 via port `$3F` e leitura do port `$7F`.
6. Assert dos valores `$82/$84` para VINT e `$88/$8A` para HINT.
7. Execução contínua versus blocos, snapshot/restore antes e depois do boundary.
8. Regressões SMS1, SMS2 e SG-1000.

A fixture deve expor timestamps de: evento VDP, atualização INT, amostragem HALT T4, início do acknowledge, primeira leitura de status na ISR, TH rising edge e leitura de HCounter. A primeira divergência nessa sequência determina o subsistema a corrigir.

## 7. Decisão

**Não alterar T13, T14, HCounter conversion, Z80 ou Scheduler nesta etapa.** A evidência atual não discrimina ainda se a diferença está no boundary VDP, na fase CPU/INT, no ISR/acknowledge ou na captura TH. Preferir correção mínima T-state se a fixture reproduzir a ROM; adotar timestamp híbrido localizado somente se o teste demonstrar que um evento intra-T-state é necessário.

Nenhuma medição física de SMS1/SMS2 foi realizada.
