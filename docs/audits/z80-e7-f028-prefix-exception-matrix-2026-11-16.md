# E7 — matriz dirigida de exceções DD/FD — F-028

**Data:** 2026-11-16
**PR:** [#943](https://github.com/leonardo201800478/sega-emulator/pull/943) (Draft)
**Base main:** `caa9a76a6f09d588155ea4e0cbc95f487f079143`
**HEAD testado:** `cab4cc387818888dfac0b4f9e41b60cafeb843c3`
**CI:** [#43853516396](https://github.com/leonardo201800478/sega-emulator/actions/runs/43853516396)
**SMS Runtime:** [#43853516405](https://github.com/leonardo201800478/sega-emulator/actions/runs/43853516405)

## 1. Resultado

Os três testes de `native/z80_cpu/tests/z80_dd_fd_exception_matrix_f028.rs` passaram no HEAD indicado. Z80 focused e Rust quality/unit concluíram SUCCESS; SMS Runtime também concluiu SUCCESS.

A matriz usa nove cadeias de prefixos:
- `DD`, `FD`;
- `DD DD`, `DD FD`, `FD DD`, `FD FD`;
- `DD FD DD`, `FD DD FD`;
- `DD FD DD FD`.

As nove cadeias são aplicadas a cinco formas:
- `EX AF,AF'` (08);
- `EXX` (D9);
- `EX DE,HL` (EB);
- `HALT` (76);
- `ED NEG` (ED 44).

Isso resulta em **45 combinações prefixadas por executor** (45 atômicas + 45 microsequenciadas), além dos cinco casos BASE executados em ambos os caminhos.

## 2. Invariantes verificados

Para cada combinação, o teste compara o estado prefixado com a forma BASE:
- PC avança exatamente pelo número de bytes DD/FD, incluindo wrap a partir de 0xFFFF;
- R incrementa uma vez por M1 de prefixo, preservando o bit 7;
- cada prefixo acrescenta 4 T-states no executor atômico;
- no microsequenciador, cada prefixo acrescenta um M1 e um WAIT de 1T, totalizando +5 T-states;
- Q/P e flags finais permanecem iguais à forma BASE; previous-Q é limpo pela fronteira de prefixo;
- IX/IY, WZ, bancos alternativos, I, IFF1/2, IM, EI delay e interrupções pendentes preservam a semântica esperada;
- a cadeia é consumida integralmente e `prefix_state` retorna a None;
- atomic e microsequenciador produzem estado arquitetural idêntico.

O microsequenciador recebe um WAIT em cada M1 e o estado é serializado/restaurado antes e depois de cada conclusão de barramento. Isso cobre retomada em fronteiras de fetch, não apenas execução contínua.

## 3. Exceções cobertas

### EX AF,AF', EXX e EX DE,HL

DD/FD não substituem HL por IX/IY nessas instruções. A matriz verifica o resultado do swap, preservação de IX/IY/WZ e bancos alternativos, flags, Q/previous-Q, PC e R para cadeias simples, repetidas e alternadas.

A cobertura anterior de PR #935 já verificava um prefixo DD ou FD antes das três instruções, em ambos os executores. Esta PR estende para cadeias de até quatro bytes e compara todos os campos arquiteturais.

### HALT

HALT continua terminal depois de cadeias DD/FD. O teste verifica `halted=true`, PC após todos os bytes, refresh R, flags/Q, IX/IY e limpeza de `prefix_state`. O teste existente `halt_e7.rs` continua sendo a cobertura complementar dos ciclos repetidos de HALT após a entrada no estado halted.

### ED NEG

A cadeia DD/FD é cancelada quando o decoder ED é alcançado. `ED 44` continua executando NEG, produzindo A=0xEF a partir de A=0x11; os prefixos apenas acrescentam seus M1, PC e refresh correspondentes. O teste compara o estado final atômico e microsequenciado com a forma ED BASE.

A cobertura microsequenciada de prefix fallback e ED já existente em `prefix_fallback_e6.rs` permanece complementar; esta PR adiciona a comparação atômica e a verificação arquitetural completa.

## 4. Limites

Esta é uma matriz dirigida de exceções conhecidas, não uma varredura de todos os 256 opcodes DD/FD nem de todas as combinações possíveis de prefixos e estados. Ela não substitui:
- a auditoria completa de todos os opcodes DD/FD e exceções não indexadas;
- a validação temporal de todas as fases e WAIT patterns;
- os testes externos independentes de semântica/flags;
- a integração temporal específica de todos os casos no runtime SMS.

A comparação atomic↔microsequenciador é uma propriedade de consistência interna, não um oracle independente de silício. Não há alteração no decoder, handlers, CPU, Scheduler ou runtime.

## 5. CI e classificação

- CI [#43853516396](https://github.com/leonardo201800478/sega-emulator/actions/runs/43853516396): Z80 focused SUCCESS; três testes novos PASS; Rust quality/unit SUCCESS.
- SMS Runtime [#43853516405](https://github.com/leonardo201800478/sega-emulator/actions/runs/43853516405): Release format, Clippy e testes SUCCESS.
- Jobs não relacionados ao diff ficaram SKIPPED e não são contabilizados como PASS.

**Classificação:** a matriz dirigida de exceções coberta está PASS no HEAD da PR #943. F-028 permanece **AUDIT_PARTIAL** até a reconciliação de todo o ledger DD/FD. E7 IN_PROGRESS; E8 BLOCKED.
