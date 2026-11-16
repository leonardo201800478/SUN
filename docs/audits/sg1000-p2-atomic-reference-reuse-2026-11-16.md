# SG-1000 P2 — reutilização do executor atômico e do core funcional legado

**Data:** 2026-11-16  
**Baseline:** `main` em `1b16256358e3f17837a95cc071048c5ee5307f82`  
**Escopo:** auditoria documental, sem alteração de runtime.

## Decisão arquitetural

O SG-1000 legado deve ser tratado como referência funcional de comportamento e fonte de regressões. Seu executor atômico não será duplicado nem transportado como uma segunda CPU. O runtime SG-1000 final deve consumir o Z80 compartilhado `sega-z80-cpu`, preservando a semântica validada para SMS e SG-1000.

A implementação antiga pode ser útil como oráculo de regressão funcional para estados finais, I/O, memória, vídeo e áudio, desde que cada expectativa seja classificada por proveniência (DOC/HW/COMP/INT) e não seja confundida com evidência temporal independente.

## Auditoria do estado atual

Na baseline consultada:

- `native/sg1000_core/src/cpu.rs` é uma fachada que reexporta `sega_z80_cpu::cpu::*`; não deve ser substituída pela antiga CPU local.
- `native/sg1000_core/src/machine.rs` ainda executa `cpu.step()` por instrução. O próprio contrato do módulo declara temporização em fronteiras de instrução, não execução cycle-accurate.
- `native/sg1000_core/src/temporal.rs` já contém `Sg1000TemporalCoordinator`, com `MachineScheduler`, domínio Z80 explícito, eventos e estado serializável.
- `native/sg1000_core/src/temporal_bus.rs` já agenda operações de memória, acknowledge e ciclos internos, com conclusão consumida uma única vez e snapshot da transação pendente. PortRead/PortWrite continuam explicitamente não suportados.
- `native/sg1000_core/src/memory.rs` já cobre ROM mirroring, RAM interna, RAM de cartucho de 1/2/8 KiB e o caso DahJee Type-A em `0x2000..=0x3FFF`.
- `native/sg1000_core/src/io.rs` modela entradas ativas em nível baixo, aliases lógicos e portas físicas `0xDC/`0xDD; a integração temporal de I/O ainda precisa ser feita.
- `native/sg1000_core/src/vdp.rs` implementa o TMS9918A, raster, VRAM, status, sprites e publicação de frame. A versão atual inclui o limite de sprites configurável para testes.
- `native/sg1000_core/src/psg.rs` reutiliza `sega-sn76489` com perfil `Sg1000`; não deve ser substituído por uma implementação paralela.

## Fonte legada examinada

A branch `refactor/sg1000-native-core-consolidation` contém uma implementação anterior de `native/sg1000_core/src/machine.rs` (blob `d0738638108899f811e4e92fcf4ff8baca408476`) e `vdp.rs` (blob `2693bc2f082f31f520f4a1c4e2e5b3226b6255b4`). A máquina antiga reúne CPU, memória, I/O, VDP e PSG, captura writes e avança os dispositivos em timestamps nominais derivados da instrução. Isso é valioso como referência de integração funcional, mas não fornece por si só micro-operação retomável nem sincronização completa no Scheduler.

A comparação dessa branch com a baseline deste documento mostrou divergência histórica substancial (67 commits à frente e 3.540 atrás na consulta). Não fazer merge integral nem cherry-pick amplo. Extrair apenas casos de teste, regras de mapeamento e comportamento de periféricos que ainda não estejam presentes e que passem por revisão independente.

## Sequência de implementação após fechamento E7

1. **Gate E7:** manter E8/ZEXDOC/ZEXALL bloqueada até F-022, F-023, F-028 e revisão independente serem formalmente fechadas.
2. **Inventário de regressões:** mapear os testes e casos do core legado para os módulos Rust atuais; eliminar duplicações e identificar lacunas reais.
3. **Máquina temporal:** conduzir o Z80 compartilhado retomável por `Sg1000TemporalCoordinator` e `Sg1000TemporalBus`; não manter `cpu.step()` como executor da máquina temporal final.
4. **I/O temporal:** suportar PortRead/PortWrite e aplicar side effects de controladores, TMS9918A e SN76489 no deadline agendado, sem callbacks por instrução na GUI.
5. **Domínios de clock:** declarar razões de clock/perfil para CPU, VDP e PSG; não promover a constante nominal existente a verdade física sem fonte de hardware.
6. **Snapshot composto:** incluir CPU/microsequenciador, Scheduler, request/continuação pendente, memória, VDP, PSG, I/O e resíduos de conversão relevantes.
7. **Determinismo:** demonstrar execução contínua versus em blocos, restore em cada safe point, ordem causal e ausência de side effects duplicados.
8. **Validação independente:** executar regressões herdadas e novas, ROMs/vetores independentes e os gates SMS/SG-1000.

## Proteção do SMS

Toda PR funcional SG-1000 deve explicitar impacto nos dois sistemas. Alterações compartilhadas ficam limitadas a correções semânticas Z80 comprovadas. Não alterar SMS Runtime, VDP, PSG, barramentos ou Scheduler para acomodar o SG-1000. Exigir regressão SMS integral, testes direcionados Z80/Scheduler e testes SG-1000 no mesmo SHA candidato.

## Estado nesta baseline

- E7: IN_PROGRESS; E8: BLOCKED.
- SG-1000: CPU compartilhada integrada; máquina temporal P2 ainda não integrada.
- Nenhuma alteração funcional é introduzida por este documento.
- O core legado é fonte de regressões e comportamento de referência, não uma autorização para reintroduzir uma CPU ou runtime duplicado.
