# Z80 E7 — revisão adversarial F-028 (DD/FD)

**Data:** 2026-11-16
**Main base:** `22c489e03aaed440bbb150ee9a4b7b70c74b1b84`
**PR de cobertura final:** #977
**Resultado:** auditoria interna concluída; candidata a fechamento, pendente de revisão independente registrada.

## 1. Inventário e classificação

O ledger `docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv` classifica os 256 bytes efetivos após DD/FD:

- **85 UsesIndex** — instruções que consomem IX/IY, IXH/IXL/IYH/IYL, (IX/IY+d) ou DDCB/FDCB;
- **167 Ignored** — ação `ExecuteBaseWithoutRefetch`, preservando a semântica BASE com custo adicional do M1 de DD/FD;
- ações de prefixo CB/ED e demais exceções são classificadas separadamente, não confundidas com Ignored.

A matriz de classificação e os testes E4A/E4B/E4C/E5/E6 foram cruzados com o decoder/microsequenciador ativo. Não foi encontrada reimplementação paralela de UsesIndex nem divergência de contagem.

## 2. Cobertura UsesIndex e exceções

- E4A: loads e memória indexada, displacement assinado, limites e snapshots.
- E4B: ALU com operandos indexados e flags.
- E4C: INC/DEC indexado, incluindo semântica de registradores parciais.
- E5: DDCB/FDCB, BIT/RES/SET, cópias para registradores e flags.
- E6: resolução de prefixos redundantes/múltiplos, prioridade do último DD/FD e ações CB/ED.
- Matriz de exceções: nove cadeias de prefixos × cinco formas (EX AF,AF', EXX, EX DE,HL, HALT, ED NEG), total de 45 combinações por executor atômico/microsequenciado.

Os vetores SingleStepTests DD/FD indexados são obtidos de fixture pinned no CI e executados no job Z80 focused. A proveniência é COMP; não é apresentada como medição própria de silício.

## 3. Matriz dos 167 Ignored

O teste `native/z80_cpu/tests/z80_dd_fd_ignored_semantic_wait_matrix_f028.rs` deriva o inventário diretamente do ledger e executa os 167 bytes com DD e FD, em dois conjuntos de flags/BC para caminhos condicionais.

Para cada execução, compara:
- estado arquitetural BASE versus prefixado, com o incremento correto de R;
- PC final e caminhos tomados/não tomados;
- WZ/MEMPTR, incluindo a regra de +1 no destino de JR/DJNZ/JR cc tomado e preservação quando não tomado;
- Q/previous-Q;
- tipos, ordem, endereços e payloads de requests, ciclos internos e T-states;
- WAIT de fixture em cada request e snapshot/restore em cada request/ciclo interno.

Casos adicionais:
- data read cujo endereço coincide com PC, para provar que não é classificado incorretamente como operand stream;
- CALL em PC FFFC e RST em FFFE, DD/FD, verificando retorno 16-bit FFFF→0000 e os dois bytes empilhados;
- JR, DJNZ e JR NZ/Z/NC/C, ambos os resultados, DD/FD, PCs 2000 e FFFD, verificando PC/WZ e wrap do destino relativo.

A comparação estrutural BASE↔prefixado é um oráculo apropriado para a ação `ExecuteBaseWithoutRefetch`, mas não é apresentada como oracle independente de hardware para cada opcode BASE.

## 4. Revisão adversarial — achados

1. A classificação 85/167 e as ações de prefixo estão alinhadas ao ledger e ao decoder.
2. As exceções e cadeias redundantes não foram confundidas com instruções Ignored.
3. O incremento de R pelo M1 extra é verificado.
4. A normalização de CALL/RST trata a palavra completa do retorno, incluindo borrow no byte alto quando o low byte cruza 00.
5. WZ não fica mais excluído da matriz Ignored: o teste calcula o delta apenas para branches relativos tomados; demais fallbacks exigem WZ igual ao BASE.
6. JR/DJNZ/JR cc verificam ambos os caminhos e wrap do PC; chamadas e retornos verificam PC/stack.
7. WAIT é fixture determinística; não comprova WAIT físico. Os testes não são apresentados como integração MachineScheduler/SMS por si só.

Não foi encontrada divergência de produção que exigisse alterar decoder, prefix resolver ou microsequenciador.

## 5. CI vinculado ao HEAD exato

PR #977 HEAD `35a9028971a7a5f7140a8cbf097c63255214ab9d`:

- CI #43940980741: SUCCESS; Z80 focused validation SUCCESS; Rust quality/unit regression SUCCESS. O job buscou SingleStepTests, executou Z80/SMS/SG-1000 integration packages, ROM v5.0 e Woodmass MEMPTR.
- SMS Runtime #43940971722: SUCCESS; Release format/Clippy/tests SUCCESS.
- Scheduler deterministic stress: SKIPPED.
- Demais jobs não relacionados: SKIPPED.
- Alteração da PR #977: exclusivamente testes; nenhum código de produção foi modificado.

## 6. Decisão

**F-028: INTERNAL_REVIEW_COMPLETE — CANDIDATE FOR CLOSURE.** A cobertura técnica dos 85 UsesIndex, 167 Ignored, exceções, PC/WZ, bus trace, WAIT/snapshot e prefix chains foi internamente revisada e o HEAD exato passou os jobs Z80 focused e SMS Runtime.

O fechamento formal depende de revisão independente registrada no GitHub. Não promover esta auditoria interna a aprovação externa; não integrar PR #977 nem a documentação consolidada sem satisfazer esse gate.

**E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL permanece BLOCKED.**


## Decisão de escopo supersedente — 2026-11-16

E7 está `CLOSED_WITH_LIMITATIONS` para o perfil Zilog NMOS delimitado e para as necessidades praticáveis dos sistemas SMS/SG-1000. Esta decisão do projeto não declara revisão externa, conformidade elétrica universal ou validação de hardware para os casos não executados. As evidências desta auditoria permanecem internas e vinculadas aos SHAs/workflows descritos em cada seção. CMOS/clones não caracterizados, WAIT físico, DMA e sincronização física global permanecem NOT_VERIFIED/NOT_RUN. E8/ZEXDOC/ZEXALL é validação complementar futura e não bloqueia o início do SG-1000. Reabrir E7 somente diante de defeito/regressão reproduzível ou evidência técnica concreta.
