# F-023 — RRD/RLD: auditoria de nibbles, flags e ciclos

**Data:** 2026-11-09
**Baseline main:** \`397b01e32af41746d00b016858c4cc316147ba6b\`
**Branch candidata:** \`test/z80-f023-exhaustive-nibble-oracle-20261109\`

## 1. Frente documental — fabricante

### ZiLOG UM0080 (UM008011-0816)

As descrições oficiais RLD (p. 238) e RRD (p. 240) especificam a permutação entre os nibbles do acumulador e da memória apontada por HL. Ambas são 5 M-cycles e 18 T-states, com sequência nominal 4,4,3,4,3.

As flags explicitamente documentadas são:
- S e Z conforme o novo A;
- H=0;
- P/V conforme paridade par do novo A;
- N=0;
- C não afetado.

O UM0080 não especifica explicitamente os bits X/Y no texto de flags de RRD/RLD, nem WZ/MEMPTR. Portanto, esses itens não são promovidos a especificação pelo projeto.

Fonte: [ZiLOG UM0080 — RLD/RRD](https://www.zilog.com/docs/z80/um0080.pdf).

O ZiLOG Z80 CPU Technical Manual de 1977 também lista RLD/RRD com 5 M-cycles e 18 T-states e os efeitos de flags documentados. A matriz não é usada para inferir X/Y ou WZ.

Fonte histórica: [ZiLOG Z80 CPU Technical Manual (1977)](https://www.cpcwiki.eu/imgs/1/1b/Z80_CPU_Technical_Manual_1977.pdf).

## 2. Frente de engenharia reversa

Foram comparadas implementações sem reutilização de código:

| Referência | Arquivo/funções | Observação | Limite |
|---|---|---|---|
| MAME | \`src/devices/cpu/z80/z80.lst\`, macros \`rrd\` e \`rld\`; commit \`df58101a63890084c249119566c97bd849ca28ac\` | Define WZ=HL+1; X/Y são derivados de A; mantém C e atualiza S/Z/PV; H/N zerados | Emulador de referência, não prova de silício |
| ares | \`ares/component/processor/z80/instructions.cpp\`, \`instructionRLD\` e \`instructionRRD\`; commit \`19d051721bb14c46be92a713216421447dec91b7\` | Mesma transformação, WZ=HL+1 e flags X/Y de A | Emulador de referência, não prova de silício |

A concordância MAME/ares sustenta uma hipótese de compatibilidade observada. Não há evidência experimental independente nesta etapa para X/Y ou WZ.

## 3. Frente de arquitetura — oracle independente

A candidata adiciona em \`native/z80_cpu/src/cpu.rs\`:

\`tests::ed_rrd_rld_exhaustive_nibble_transform_and_flag_contract\`

Cobertura:
- RRD e RLD;
- 256 valores de A;
- 256 valores de (HL);
- Carry inicial 0 e 1;
- Total: 2 × 256 × 256 × 2 = 262.144 vetores;
- HL=FFFF incluído em vetor de borda para verificar o wrap de WZ.

O oracle calcula diretamente:
- RRD: \`A'=(A&F0)|(M&0F)\`; \`M'=((A&0F)<<4)|(M>>4)\`.
- RLD: \`A'=(A&F0)|(M>>4)\`; \`M'=(M<<4)|(A&0F)\`.
- Flags documentadas: C preservado; S/Z conforme A'; P/V conforme paridade par de A'; H/N zerados.

O teste também confere A, memória, HL, PC, R, Q, duração de 18T e WZ=HL+1. A verificação X/Y deriva bits 5/3 de A segundo a concordância MAME/ares; essa parte não é apresentada como oracle de fabricante ou evidência física.

## 4. Microsequenciador, WAIT e snapshot

A candidata adiciona em \`native/z80_cpu/src/sequencer.rs\`:

\`ed_return_nibble_rotate_differential_tests::rrd_rld_boundary_vectors_preserve_wait_snapshot_and_bus_order\`

São 16 casos dirigidos (dois opcodes × oito vetores), incluindo zeros, nibbles alternados, extremos, ambos os estados de Carry e HL=FFFF. Cada pedido de barramento recebe um WAIT T-state e o helper aplica encode/decode + restore após cada ciclo de barramento e ciclo interno.

Verifica:
- 18T nominais e 22T com quatro WAITs (um por cada pedido);
- sequência de barramento: fetch ED, fetch opcode terminal, read (HL), write (HL);
- estado arquitetural e memória finais iguais ao executor atômico;
- PC=1002, R=80 a partir de FE, WZ=HL+1 e ausência de request pendente.

A comparação atômico↔microsequenciador é diferencial interna, não oracle independente. O oracle independente é a fórmula direta das permutações e das flags documentadas.

## 5. Critérios de aceitação

1. 262.144 vetores atômicos passam para RRD/RLD.
2. S/Z/PV/H/N/C correspondem à fórmula derivada do UM0080.
3. X/Y é rotulado como hipótese de referência, não especificação.
4. 16 vetores microsequenciados passam com WAIT e snapshot/restore.
5. Ordem de barramento e tempos 18T/22T conferidos.
6. fmt, Clippy e testes Z80/SMS/SG-1000 passam no HEAD exato da PR.
7. F-023 permanece AUDIT_PARTIAL até resolver X/Y/WZ com evidência independente e ampliar a validação temporal.

## 6. Limites e estado

- WZ=HL+1: concordância MAME/ares, não verificação física.
- X/Y derivados de A: concordância MAME/ares, não especificação UM0080.
- ROM externo dedicado: NOT RUN.
- Hardware SMS: NOT RUN.
- CMOS/clones: NOT_VERIFIED.
- A alteração é de testes e documentação; nenhum runtime é alterado.

**Classificação:** a semântica documentada de transformação e flags ganha cobertura exaustiva no executor atômico; a trilha temporal ganha vetores dirigidos com WAIT/snapshot. F-023 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## 7. Integração e validação

PR #714 MERGED por squash em `3a81437362dad79577058852b5f469381d3bde2c`. HEAD validado da PR: `bb15e672f30d8aae3777761f1545347fc3d520ea`. CI #6056 / run `42651927378` PASS no HEAD da PR; CI pós-merge #6057 / run `42652163312` PASS no HEAD exato da main.

A fixture exaustiva passou com 262.144 vetores. O primeiro ensaio da fixture tinha um erro de isolamento: a memória fonte não era restaurada entre os dois estados de Carry; isso foi corrigido antes da aprovação final. Não houve alteração de runtime nem bug do Z80 identificado nesta tarefa.

F-023 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8 BLOCKED.


## 8. Oracle independente hardware-derived — Mark Woodmass MEMPTR test 09

**PR candidata:** #942 (Draft), HEAD de código testado: `7402ff138ced95da2d8b9e0008919c8cd22484f1`.

A suíte **Z80 Test Suite (2008), Mark Woodmass**, possui uma variante MEMPTR. A documentação upstream descreve a suíte como verificação das regras MEMPTR e informa comparação dos resultados com um NEC D780C-1 e testes adicionais em outros clones. A tabela MEMPTR upstream inclui explicitamente RLD/RRD com WZ/MEMPTR = HL+1.

Proveniência e integridade das fixtures:
- TAP GPL-2.0 obtido somente durante CI, sem commit/redistribuição no repositório: 5.573 bytes; SHA3-512 `BB1DCE3187FBF5EA871FE41EB76AE7E0C94D08136FF4D8E756E2624DA589AAE69CAD074EB43CBFB9E2B9E0D2CC505343D9804EDCDE3CEA1FB1881DCBF5C865B0`.
- Metadados do caso 09 pinados em `redcode/Z80/sources/test-Z80.c`, commit `266f460fea6b07096404c8e8170b65581dfe6eca`: offset do código 120, tamanho 5.452, load 0x8000, PC inicial 0x8049, HALT 0x80E6, 50.904.931 T-states, hash FNV-1 `F5AE5140`, 61 linhas e 32 colunas.
- A ROM Spectrum 48K original (16.384 bytes) é reconstruída no runner a partir de `antirez/ZOT/rom.h`, commit `1cb85abbbd1b5e00d9e2b7c6c2ab87b3f8c11cf3`, com Git blob SHA `626b8eb763b0964de3b1496f47647a154c1322cb`. O arquivo-fonte preserva a nota de copyright e a permissão Amstrad para uso em emuladores. Nenhum binário ROM é commitado.
- O harness replica o caminho específico Woodmass: RST 10h → JP 0x70F2 → hook PRINT que retorna RET; também reproduz CHAN_OPEN em 0x1601, CLS em 0x0D6B, portas pares 0xBF/ímpares 0xFF e writes de I/O sem efeito.

**Resultado no HEAD exato:** CI [#43850972785](https://github.com/leonardo201800478/sega-emulator/actions/runs/43850972785), job Z80 focused, PASS. O teste explícito `woodmass_memptr_tap_matches_hardware_reference` terminou com 1 passed/0 failed em 0,10 s:
- HALT em 0x80E6;
- 50.904.931 T-states;
- FNV-1 `F5AE5140`;
- 61 linhas e 32 colunas.

SMS Runtime [#43850972708](https://github.com/leonardo201800478/sega-emulator/actions/runs/43850972708) também PASS em Release format, Clippy e testes. No CI Z80, SingleStepTests DD/FD (2.000 SCF/CCF/Q + 32.000 indexed ALU), testes integrados Z80/SMS/SG-1000, format e Clippy passaram. Jobs não relacionados ao diff foram SKIPPED e não são contabilizados como PASS.

**Interpretação e limites:** esta é evidência independente hardware-derived para o comportamento MEMPTR/WZ exercitado pela suíte, incluindo a regra RLD/RRD → HL+1. Não é uma medição física executada neste projeto nem prova universal de todos os clones. A suíte MEMPTR não fecha os bits X/Y não especificados pelo UM0080, não substitui a auditoria temporal microsequenciada completa e não valida hardware SMS diretamente. CMOS/clones permanecem NOT_VERIFIED.

**Classificação atualizada:** a lacuna de oracle independente para WZ/MEMPTR foi reduzida; F-023 permanece AUDIT_PARTIAL por X/Y, abrangência temporal e limites de plataforma. A PR #942 continua Draft e não está integrada. E7 IN_PROGRESS; E8 BLOCKED. Nenhuma alteração de runtime/CPU.
