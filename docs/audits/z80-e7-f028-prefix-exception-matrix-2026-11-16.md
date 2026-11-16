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


## Aditivo supersedente — ampliação e validação final — 2026-11-16

A descrição anterior refletia a primeira versão da matriz e foi supersedida pela validação final abaixo.

- **HEAD de código/harness:** 98c1cf41587f6af66c3d272e4b07a638d5da7919.
- **CI:** [#43859000666](https://github.com/leonardo201800478/sega-emulator/actions/runs/43859000666) SUCCESS; Z80 focused, Rust quality/unit e documentation hygiene SUCCESS.
- **SMS Runtime:** [#43859000676](https://github.com/leonardo201800478/sega-emulator/actions/runs/43859000676) SUCCESS; Release format, Clippy, testes e build Windows x64 concluídos.
- **Cobertura:** quatro testes; nove cadeias DD/FD × 12 formas (EX AF, EXX, EX DE/HL, HALT, ED NEG, NOP, DAA, CPL, SCF, CCF, DI, EI) = 108 combinações prefixadas por executor.
- Os 34.000 vetores SingleStepTests DD/FD também foram explicitamente executados e aprovados neste HEAD.
- A falha intermediária em SCF/CCF era fixture incorreto: o estado inicial correto é expresso pelo latch Q, porque o início de instrução transfere Q para previous-Q; o caminho DD/FD limpa previous-Q conforme a regra NMOS. O fixture agora parametriza Q inicial e o caso BASE inicia com Q=0. Os quatro testes passam; nenhuma produção CPU foi alterada.
- F-028 permanece AUDIT_PARTIAL: cobertura dirigida, não todos os 256 opcodes DD/FD e estados. E7 IN_PROGRESS; E8 BLOCKED.

## 6. Ledger integral de classificação e interrupções — extensão 2026-11-16

### 6.1 Classificação dos 256 bytes após DD/FD

Foi adicionado o ledger docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv com 256 registros, um por byte após DD/FD. A partição efetiva da FSM é 85 UsesIndex, 167 Ignored, 1 IndexedCB, 1 ED que cancela a seleção IX/IY e 2 bytes DD/FD que repetem/substituem o prefixo anterior.

A função auxiliar classify_index_opcode() retorna Ignored para DD/FD/ED quando chamada diretamente porque a FSM trata esses bytes antes de invocá-la. O ledger distingue o resultado auxiliar da transição efetiva. O teste index_prefix_classification_f028.rs enumera todos os bytes e compara a classificação com uma lista explícita de 85 opcodes UsesIndex; as contagens esperadas da função auxiliar são 85 UsesIndex, 170 Ignored e 1 IndexedCB.

Exceções explicitadas: HALT 76 permanece BASE/Ignored; EX AF,AF' 08, EXX D9 e EX DE,HL EB preservam semântica BASE; 66/6E/74/75 usam IX/IY+d mas mantêm H/L convencionais; CB entra em DDCB/FDCB; ED cancela a indexação; DD/FD repetidos obedecem last-prefix-wins.

### 6.2 Interrupções solicitadas durante a cadeia

Foram adicionados dois testes em z80_dd_fd_exception_matrix_f028.rs. Eles solicitam NMI ou INT depois do M1 de DD, antes de FD, durante a cadeia DD FD 00. Cada M1 recebe WAIT e passa por snapshot encode/decode/restore. Os testes verificam que a cadeia continua até a aposentadoria do NOP e que NMI/INT só são aceitos na fronteira seguinte. É um teste do microsequenciador Z80, não um novo teste de integração MachineScheduler/VDP SMS.

### 6.3 Validação final das extensões

- HEAD validado: b3320ea79c6b951d2ad8b70e762589f14da597df.
- CI [#43863265215](https://github.com/leonardo201800478/sega-emulator/actions/runs/43863265215): SUCCESS. Z80 focused validation, Rust quality/unit e documentation patch hygiene SUCCESS.
- Teste index_prefix_classification_f028.rs: 3 passed/0 failed; enumeração de todos os 256 bytes.
- Teste z80_dd_fd_exception_matrix_f028.rs: 6 passed/0 failed; inclui os dois novos testes INT/NMI.
- SingleStepTests DD/FD: 2.000 SCF/CCF/Q + 32.000 indexed ALU = 34.000/34.000 PASS no mesmo SHA.
- SMS Runtime [#43863265197](https://github.com/leonardo201800478/sega-emulator/actions/runs/43863265197): SUCCESS; Release format, Clippy, testes e build Windows x64 PASS.
- A primeira tentativa #43862912238/#43862912321 falhou somente em rustfmt; correção aplicada antes do HEAD b3320ea. Não houve falha semântica nessa primeira tentativa, pois os testes foram SKIPPED após o gate de formato.
- Jobs não relacionados ao diff ficaram SKIPPED por classificação de impacto; não são contabilizados como PASS.

### 6.4 Limites mantidos

O ledger e o teste completam a classificação explícita de cada byte após DD/FD, mas não constituem validação semântica integral dos 256 opcodes em todos os estados. O repositório já possui smoke tests de 256 encodings DD/FD e DDCB/FDCB; eles demonstram dispatch sem erro. A suíte indexed-CB SMS exaustiva cobre rotate/shift para todos os valores/carry, e outras famílias têm testes dirigidos; isso não equivale a uma prova combinatória de BIT/RES/SET e todas as interações de estado/barramento. Atomic↔micro continua sendo consistência interna, não oracle independente.

**Classificação:** a classificação de todos os 256 bytes e os novos casos de INT/NMI estão PASS no HEAD b3320ea. F-028 permanece AUDIT_PARTIAL; E7 IN_PROGRESS; E8/ZEXDOC/ZEXALL BLOCKED. Nenhuma alteração de produção CPU, decoder, microsequenciador, Scheduler ou runtime.