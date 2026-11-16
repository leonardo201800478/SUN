# E7 — validação ROM v5.0 de flags das instruções de bloco

**Data da validação:** 2026-11-16  
**PR:** [#936](https://github.com/leonardo201800478/sega-emulator/pull/936) (Draft)  
**HEAD de código validado:** `a7f551336104e1c2c03d7e6790b0aafbb5e346e7`  
**CI:** [#43846321428](https://github.com/leonardo201800478/sega-emulator/actions/runs/43846321428), job `Z80 focused validation`  
**Escopo:** comportamento das flags quando instruções Z80 de bloco repetitivas são interrompidas durante a repetição, no perfil NMOS.

## 1. Resultado

O teste `z80_block_instruction_flags_test_v5_passes_on_spectrum48_timing` executou e passou no HEAD indicado: **1 passed, 0 failed, 6,48 s**.

O harness verificou:
- pelo menos 60 comparações principais de instrução/PC, sem marcador de divergência;
- 390 registros B-loop em seis grupos: OTIR, OTIR, OTDR, OTDR, INIR e INDR;
- B observado igual ao esperado em todos os registros: `B_after = 13 + 3 × iteration`;
- flags F observadas idênticas às flags esperadas nos 390 casos;
- calibração da rotina de delay concluída, sem marcador de falha;
- ausência do marcador de leitura IN inesperada;
- retorno ao chamador no PC/SP esperados.

O B-loop valida 65 casos por grupo. No primeiro caso de cada grupo, o contador inicial é 0x10 e a observação esperada após três iterações é B=0x0D; os casos seguintes variam B em passos de três. O resultado confirma o comportamento de repetição interrompida nos cenários cobertos pela ROM.

## 2. Proveniência da referência

Fonte upstream: [Peter Helcmanovsky — Z80 Block Flags Test](https://github.com/MrKWatkins/ZXSpectrumNextTests/tree/8adcbc8384b2311fb6d36d424478aef583d5c307/Tests/ZX48_ZX128/Z80BlockInstructionFlags).

- O README atribui a pesquisa original de flags não documentadas a David Banks.
- O autor declara que a suíte foi testada em hardware real: ZX Spectrum 128 (v5.0) e Spectrum+ (v4.0), confirmando os resultados da pesquisa de Banks para CPUs Z80 NMOS.
- O README declara expressamente que o teste cobre um conjunto limitado de condições e não é exaustivo.
- A suíte é MIT; o SNA é obtido somente no CI e não é commitado.

Fixture:
- Arquivo: `z80bltst.sna`.
- Tamanho: 49.179 bytes.
- Git blob SHA: `41b7b0da0f5abfd5d2841fbae005f05291986f8b`.
- Commit upstream: `8adcbc8384b2311fb6d36d424478aef583d5c307`.
- Harness: `native/z80_cpu/tests/z80_block_flags_rom_v5.rs`.

## 3. Modelo temporal do harness

O harness carrega o snapshot 48K (cabeçalho de 27 bytes e RAM em 0x4000–0xFFFF) e reproduz:
- frequência de quadro Spectrum 48K 50 Hz;
- 69.888 T-states por quadro;
- pulso INT de 32 T-states;
- leitura ULA no porto 0xFE com valor 0x80;
- joystick Kempston no porto 0x1F com valor 0x00;
- floating bus no porto 0xFF com valor 0xFF;
- os pontos de entrada de ROM usados somente para apresentação/saída do teste.

O harness usa limite máximo de 80.000.000 T-states e 20.000.000 steps. O limite de 50.000.000 T-states da tentativa inicial era insuficiente para completar a sequência finita de calibração e seis grupos: cada caso inclui um delay LDIR calibrado de aproximadamente 3.500 iterações (~73.500 T-states), e a estimativa do percurso completo era ~61 milhões de T-states. O limite de 80M já estava presente no harness antes desta reconciliação; não foi aumentado nesta investigação e não é uma compensação aplicada ao runtime.

A tentativa inicial de 50M terminou durante LDIR por esgotamento do orçamento. As instrumentações subsequentes identificaram que o handler IM2 contém seu próprio LDIR, o que produzia falsos marcadores de conclusão quando o trace não acompanhava a retomada do LDIR original. A máquina de estados de trace foi corrigida para exigir o retorno ao PC original após RETI antes de reconhecer BC=0. A execução final alcançou o fim do SNA e passou todas as asserções acima.

## 4. Cobertura semântica e limites

O README upstream descreve o comportamento observado quando:
1. uma instrução de bloco repetitiva já executou pelo menos uma iteração;
2. o contador permanece diferente de zero;
3. uma interrupção é aceita no ciclo adicional que repete a instrução e retorna o PC ao byte ED;
4. as flags são capturadas no handler antes da retomada da instrução.

Para LDxR/CPxR, a referência descreve X/Y a partir de PC.13/PC.11. Para INxR/OTxR, também há ajustes adicionais de P/V e H. A ROM utiliza IM2 e portas Spectrum comuns para observar o estado das flags.

**Limites que permanecem:**
- A própria suíte declara que testa apenas um conjunto limitado de condições; não é uma matriz exaustiva de operandos/estados.
- Os 390 B-loop records cobrem seis grupos de repetição de I/O, não todas as combinações temporais possíveis.
- As comparações principais verificadas pelo harness são pelo menos 60, não uma prova formal de todos os opcodes e fases.
- A suíte testa o perfil Z80 NMOS descrito pelo autor; não prova equivalência universal de CMOS ou clones.
- O teste foi executado no emulador, não em hardware Master System nesta sessão. A origem hardware-derived do oracle é a execução upstream em ZX Spectrum 128/Spectrum+ descrita pelo autor.

A evidência não substitui:
- fechamento da matriz de iteração simples de LDI/LDD/CPI/CPD em todos os estados;
- fechamento da matriz microsequenciada completa, incluindo WAIT/snapshot e equivalência atômico/microsequenciado;
- integração e preservação do comportamento no runtime SMS/SG-1000 além dos testes já executados.

## 5. CI e regressões associadas

No mesmo HEAD `a7f551336104e1c2c03d7e6790b0aafbb5e346e7`:
- CI [#43846321428](https://github.com/leonardo201800478/sega-emulator/actions/runs/43846321428): Z80 focused validation SUCCESS.
- SingleStepTests DD/FD: 2.000 SCF/CCF/Q + 32.000 indexed ALU PASS.
- Testes integrados Z80/SMS/SG-1000, format e Clippy PASS.
- SMS Debugger Runtime [#43846321313](https://github.com/leonardo201800478/sega-emulator/actions/runs/43846321313): Release format, Clippy e testes PASS.
- Jobs não relacionados ao diff ficaram SKIPPED e não são contabilizados como PASS.

## 6. Decisão

**F-022 permanece AUDIT_PARTIAL.** A ROM v5.0 agora é evidência hardware-derived executada e aprovada para os cenários de repetição interrompida cobertos. Não fecha a matriz completa de iteração simples nem toda a temporalidade microsequenciada.

Não houve alteração de runtime, CPU, Scheduler ou temporização de produção nesta validação. A correção foi restrita ao harness de teste/CI e à sua instrumentação.

**E7:** IN_PROGRESS. **E8/ZEXDOC/ZEXALL:** BLOCKED até fechamento demonstrável de E7.
