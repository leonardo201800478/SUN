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

## 7. Reconciliação dos 85 UsesIndex com as famílias existentes

A extração do ledger CSV foi agrupada por família, sem sobreposição. O resultado soma exatamente 85 bytes:

| Família | Qtde | Bytes após DD/FD |
|---|---:|---|
| E3A1 | 9 | 09, 19, 21, 23, 29, 2B, 39, E9, F9 |
| E3A2 | 5 | 22, 2A, E1, E3, E5 |
| E4A | 15 | 36, 46, 4E, 56, 5E, 66, 6E, 70, 71, 72, 73, 74, 75, 77, 7E |
| E4B | 8 | 86, 8E, 96, 9E, A6, AE, B6, BE |
| E4C | 2 | 34, 35 |
| F-036 | 24 | 44, 45, 4C, 4D, 54, 55, 5C, 5D, 60, 61, 62, 63, 64, 65, 67, 68, 69, 6A, 6B, 6C, 6D, 6F, 7C, 7D |
| PR #680 — INC/DEC/LD imediato half-register | 6 | 24, 25, 26, 2C, 2D, 2E |
| F-037 — ALU half-register | 16 | 84, 85, 8C, 8D, 94, 95, 9C, 9D, A4, A5, AC, AD, B4, B5, BC, BD |
| **Total** | **85** | |

**Implicação:** os 85 UsesIndex têm família e testes preexistentes. F-036 e F-037 permanecem CLOSED; E3A1/E3A2/E4A/E4B/E4C permanecem preservadas; PR #680 já está integrada. Não duplicar essas implementações para fechar F-028.

## 8. Gap residual: os 167 Ignored

A classificação efetiva dos 167 bytes Ignored indica fallback para BASE sem refetch, mas a validação prefixada existente é representativa:
- `prefix_fallback_e6.rs`: nove testes para opcode já buscado, M1/WAIT/snapshot, immediate, ALU immediate, relative branch, memória read/write e ED.
- PR #943: 108 combinações por executor para nove cadeias e 12 formas, incluindo EX AF/EXX/EX DE-HL, HALT, NEG, NOP, DAA, CPL, SCF, CCF, DI e EI.
- `prefix_fsm.rs`: last-prefix-wins, ED cancel e cadeia de 257 prefixos.
- INT/NMI entre DD e FD são aceitos após o opcode terminal em cenário NOP.

Isso não prova fallback de todos os 167 bytes. O próximo trabalho deve agrupar os bytes por famílias BASE e identificar quais já têm cobertura com DD/FD prefixado, quais têm apenas testes BASE sem prefixo e quais permanecem não implementados/reservados. Um teste de igualdade atomic↔micro é consistência interna, não oracle independente.

**Decisão:** a partição 85/167/1/1/2 está reconciliada; F-028 continua AUDIT_PARTIAL até a semântica prefixada dos 167 Ignored e as demais interações relevantes estarem demonstradas. Nenhuma alteração de produção nesta extensão documental.


## Addendum supersedente — cobertura dos 167 Ignored após PRs #950/#953 — 16/11/2026

A seção **8. Gap residual: os 167 Ignored** registra corretamente o estado anterior às PRs #950 e #953, mas não representa mais a pendência atual. Seu diagnóstico de que faltava a matriz de requests dos 167 bytes foi supersedido pelas integrações abaixo. Esta atualização preserva a seção histórica e corrige somente sua interpretação vigente.

### Evidência integrada

- **PR #950 — merge:** `45b40755e46aef752d58f571de856ae25ff9e81c`.
- **Validação #950:** CI #43888693825 e SMS Runtime #43888693845 PASS no HEAD validado; a árvore do merge foi confirmada idêntica à árvore testada.
- **Arquivo:** `native/z80_cpu/tests/z80_dd_fd_ignored_semantic_wait_matrix_f028.rs`.
- **Cobertura #950:** os 167 opcodes efetivamente classificados como Ignored são executados sem prefixo e com DD/FD. A matriz compara estado arquitetural BASE↔prefixado, fetches M1, refresh R, requests e payloads de leitura/escrita, tipo/ordem/endereço/duração das transações, ciclos internos, WAIT de fixture e snapshot encode/decode/restore nas fronteiras de barramento e ciclos internos.
- **PR #953 — merge:** `a6cec0d744b8120c8c3e2d1d56abd1aa75fc1031`.
- **Validação #953:** CI #43891989147 e SMS Runtime #43891989127 PASS no HEAD validado; a árvore do merge foi confirmada idêntica à árvore testada.
- **Teste complementar:** `all_167_ignored_fallbacks_preserve_both_conditional_outcomes_and_exact_pc`. A segunda fixture usa F=$FF e B=1 para complementar a primeira fixture (F=$00 e B=$23), verificar resultados condicionais opostos e exigir o delta exato de PC por opcode, mantendo a comparação de requests e o round-trip de snapshot/WAIT.

### Limites que permanecem

1. BASE↔DD/FD é uma comparação estrutural dentro do mesmo microsequenciador; não é oracle independente de silício.
2. As fixtures cobrem dois conjuntos dirigidos de flags/registradores, não o espaço combinatório de todos os dados e estados de cada instrução BASE.
3. WAIT é inserido como fixture determinística. Isso valida a suspensão/retomada do microsequenciador sob o contrato testado, não WAIT elétrico/físico.
4. Essas matrizes F-028 são testes do microsequenciador Z80; não devem ser descritas como integração MachineScheduler/SMS.
5. A classificação dos 256 bytes e a cobertura dos 167 Ignored não provam, isoladamente, semântica independente de todos os opcodes BASE nem fecham F-028.

**Estado vigente:** a lacuna de ausência da matriz de requests dos 167 Ignored está **RESOLVIDA pelas PRs #950/#953**. F-028 permanece **IN_PROGRESS** exclusivamente para revisão adversarial independente, proveniência/oráculos e limites semânticos ainda não demonstrados. Não reimplementar UsesIndex nem reproduzir a matriz já integrada.


## Adendo pós-PR #975 — wrap de retorno CALL/RST — 16/11/2026

- Merge SHA na main: `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`.
- Arquivo: `native/z80_cpu/tests/z80_dd_fd_ignored_semantic_wait_matrix_f028.rs`.
- Novos casos: CALL em PC `FFFC` e RST em `FFFE`; para DD e FD, verifica M1 de prefixo/opcode, wrap do endereço de retorno `FFFF→0000`, high/low byte gravados na pilha e normalização diferencial BASE↔prefixado.
- Push CI #43937688464: Z80 focused PASS e Rust quality/unit PASS; jobs não relacionados SKIPPED. SMS Runtime #43937688349: Release format/Clippy/tests PASS; Windows debugger SKIPPED.
- Limite: evidência estrutural interna do mesmo microsequenciador, não oracle independente de hardware. GitHub não registra review; Codex review falhou por limite de uso. F-028 permanece IN_PROGRESS.


## Adendo PR #977 — WZ em saltos relativos com DD/FD Ignored

A PR #977 amplia a auditoria F-028/F-023 com JR, DJNZ e JR NZ/Z/NC/C, caminhos tomado/não tomado, prefixos DD/FD e PCs iniciais `2000` e `FFFD`. Verifica o deslocamento de PC/WZ, preservação de WZ quando não tomado, fetch M1 e trace normalizado. HEAD após correção rustfmt: `a783a3c231b5de1678ffc71eb8b1e8dcda42c724`; CI #43939973117 em andamento na última consulta. Não declarar PASS antes do resultado final. F-028 permanece IN_PROGRESS e a revisão independente continua pendente.



## Resultado CI PR #977 — 16/11/2026

HEAD `a783a3c231b5de1678ffc71eb8b1e8dcda42c724`: CI #43939973117 SUCCESS, incluindo Z80 focused e Rust quality/unit; SMS Runtime #43939957730 SUCCESS. O teste cobre WZ para JR/DJNZ/JR cc sob DD/FD Ignored, ambos os resultados condicionais e PC=FFFD. Scheduler stress SKIPPED. A cobertura técnica adicional passou, mas F-028 permanece IN_PROGRESS até review independente integral registrada.



## Resultado final PR #977 — 16/11/2026

HEAD `35a9028971a7a5f7140a8cbf097c63255214ab9d`: CI #43940980741 SUCCESS (Z80 focused e Rust quality/unit); SMS Runtime #43940971722 SUCCESS; Scheduler stress SKIPPED. A matriz dos 167 Ignored agora verifica WZ em todos os opcodes, com +1 para JR/DJNZ/JR cc tomados e preservação quando não tomados. Casos dirigidos verificam PC=2000/FFFD, CALL/RST stack wrap, DD e FD. F-028: auditoria interna completa/candidata; review independente pendente.
