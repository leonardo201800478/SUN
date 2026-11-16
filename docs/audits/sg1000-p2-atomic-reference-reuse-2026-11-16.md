# SG-1000 P2 — reutilização do executor atômico e do core funcional legado

**Data:** 2026-11-16
**Baseline histórica da auditoria:** `main` em `1b16256358e3f17837a95cc071048c5ee5307f82`; estado operacional atual reconciliado com `bb155bdfe1a188ccc8f5b4a01896b23a0cfde808`
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

1. **Gate de entrada:** E7 `CLOSED_WITH_LIMITATIONS` no perfil NMOS delimitado satisfaz a condição para P2. E8/ZEXDOC/ZEXALL e revisão independente são complementares, não bloqueios para SG-1000.
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

- E7: `CLOSED_WITH_LIMITATIONS`; E8: validação complementar futura.
- SG-1000: CPU compartilhada integrada; máquina temporal P2 ainda não integrada.
- Nenhuma alteração funcional é introduzida por este documento.
- O core legado é fonte de regressões e comportamento de referência, não uma autorização para reintroduzir uma CPU ou runtime duplicado.


## Integração obrigatória com a interface desktop atual

A implementação SG-1000 final deve permanecer integralmente dentro da arquitetura já montada e integrada para SMS 1/2. Não criar um frontend, launcher, janela, pipeline de vídeo, backend de áudio ou sistema de entrada paralelo para SG-1000.

- **GUI:** reutilizar a aplicação Slint existente, menus, biblioteca, ciclo de vida e integração coarse-grained do runtime. A seleção do sistema/cartridge deve encaminhar para a máquina correspondente sem duplicar a aplicação desktop.
- **Vídeo:** reutilizar o pipeline WGPU 30 já funcional, o contrato de frame/mailbox, upload de textura e composição Slint. O SG-1000 fornece frames no formato/contrato esperado pelo host; dimensões e modos próprios devem ser tratados no runtime/adaptador de vídeo, sem contaminar o timing da máquina.
- **Áudio:** reutilizar integralmente `sega-audio-output` / CPAL 0.18.2 e o pipeline de áudio existente. O core SG-1000 entrega amostras/blocos no contrato do runtime de áudio; não criar dispositivo, stream CPAL ou callback de áudio alternativo. O perfil PSG SG-1000 configura a geração sonora, não a saída física.
- **Inputs:** reutilizar o mesmo esquema de entrada SMS já existente no host (SDL3/input runtime, teclado, gamepads, bindings, snapshots e combinação de fontes). Adaptar o snapshot lógico aos controles SG-1000 e à codificação ativa-baixa/portas físicas no limite do core; não criar polling ou mapeamento de host independente.
- **Runtime:** a interface só controla operações coarse-grained. A execução SG-1000 permanece no backend Rust e no Scheduler; callbacks de GUI, áudio e renderização não podem executar instruções nem controlar ciclos.
- **Compatibilidade:** os fluxos atuais SMS 1 e SMS 2, áudio, vídeo, inputs, preferências e ciclo de vida devem permanecer intactos. Toda PR que tocar no host compartilhado deve executar regressão SMS nos dois modos e demonstrar que não houve regressão.

A regra de aceitação é uma única aplicação desktop e uma única infraestrutura de host para SMS 1, SMS 2 e SG-1000, com diferenças de hardware confinadas aos runtimes/adaptadores específicos. Python não faz parte do build, runtime, ferramentas obrigatórias nem caminho de implementação do SG-1000.


## Atualização de escopo — P2 pode prosseguir (2026-11-16)

E7 foi encerrada como `CLOSED_WITH_LIMITATIONS` no perfil NMOS delimitado. E8/ZEXDOC/ZEXALL, validação física universal ou revisão independente por outra IA não são gates permanentes para iniciar SG-1000.

A implementação deve permanecer na arquitetura atual. O core legado serve como fonte seletiva de regressões, nunca como segunda CPU/runtime. Conectar a máquina ao coordenador/barramento temporal existente; concluir PortRead/PortWrite, coordenação CPU/VDP/PSG, clocks, side effects, snapshots compostos e equivalência contínua/em blocos. Preservar desktop Slint/WGPU/CPAL/SDL3 e executar regressões SMS1/SMS2 e SG-1000 no mesmo SHA candidato.


## Acompanhamento P2.1 — candidato temporal de I/O (2026-11-16)

**Main reconciliada:** `f8315ed5a95d6ef1d7cc02aa2edd813bb1d6b434`, após integração documental da PR #980.  
**Candidato:** PR [#981](https://github.com/leonardo201800478/sega-emulator/pull/981), branch `feat/sg1000-temporal-port-io-20261116`, HEAD de código validado `17dd9ca18382e10b666155f375a9556944f4e4e4`, base main, 11 commits ahead / 0 behind. Draft; não integrado.

### Implementação candidata e rastreabilidade

| Arquivo | Símbolos/contrato | Evidência de teste | Workflow / SHA |
|---|---|---|---|
| `native/sg1000_core/src/temporal.rs` | `Sg1000TemporalCoordinator::new_with_clock_profile`; domínios Z80, PSG e VDP; razão PSG/CPU racional e exata | `scheduled_boundary_advances_cpu_vdp_and_psg_clock_domains_together`; `psg_clock_domain_uses_exact_configured_frequency_ratio` | [#43969077180](https://github.com/leonardo201800478/sega-emulator/actions/runs/43969077180), HEAD `17dd9ca` |
| `native/sg1000_core/src/temporal_bus.rs` | `PortRead`/`PortWrite`; leitura de controle/VDP no deadline; writes VDP/PSG no deadline; snapshots dos dispositivos | `port_read_samples_controller_at_deadline_and_advances_all_device_domains`; `configured_psg_clock_advances_by_its_exact_ratio`; `psg_port_write_is_committed_once_and_survives_completion_restore`; `vdp_port_writes_apply_at_their_scheduled_completion_boundaries`; `restore_rejects_device_clock_state_that_diverges_from_scheduler` | [#43969077180](https://github.com/leonardo201800478/sega-emulator/actions/runs/43969077180), HEAD `17dd9ca` |
| `native/sg1000_core/src/vdp.rs` | `Tms9918A::elapsed_tstates`, usado para verificar coerência temporal do snapshot | `restore_rejects_device_clock_state_that_diverges_from_scheduler` | [#43969077180](https://github.com/leonardo201800478/sega-emulator/actions/runs/43969077180), HEAD `17dd9ca` |
| `native/sg1000_core/src/lib.rs` | exportação dos IDs dos domínios e do clock padrão provisório | compilação do workspace e validação focada Z80 | [#43969077180](https://github.com/leonardo201800478/sega-emulator/actions/runs/43969077180), HEAD `17dd9ca` |

No HEAD de código `17dd9ca`, os jobs **Change impact classification**, **Rust workspace regression**, **SMS1 VDP core and machine integration validation**, **Z80 focused validation**, **Scheduler deterministic stress** e **Rust quality and unit regression** concluíram SUCCESS. Os jobs fora do escopo SG-1000 foram SKIPPED e não são classificados como PASS. Não houve alteração em SMS Runtime, desktop, CPAL ou SDL3 nesta fatia.

### Limites que permanecem abertos

- A PR #981 amplia o componente temporal, mas ainda não conecta o `Sg1000Machine` ao `Z80MicroSequencer`. A máquina existente continua usando `cpu.step()` e avançando VDP/PSG em fronteiras de instrução.
- O barramento temporal candidato não é um segundo runtime ativo: a integração seguinte deve substituir o caminho de execução dentro do mesmo `Sg1000Machine`, não manter duas máquinas em paralelo.
- O domínio VDP usa T-state units do modelo TMS9918A atual; não representa validação de pixel clock, fetch físico ou arbitragem de placa.
- A razão PSG/CPU é exata e configurável no Scheduler, mas o perfil padrão continua provisório, sem alegação de clock físico de placa.
- Ainda faltam M1/T4 agendado, integração de IRQ do VDP por evento, snapshot composto da máquina/microsequenciador, restore em todos os safe points e equivalência contínua versus execução em blocos.
- WAIT continua sendo fixture de software. Não foi feita alegação de WAIT físico ou validação de hardware.

### Próxima fatia P2.2

Integrar o próprio `Sg1000Machine` ao barramento e coordenador acima, usando o `Z80MicroSequencer` compartilhado e a continuação M1 reutilizável do Z80. Remover a execução atômica por instrução do caminho temporal final sem descartar o core funcional legado como oráculo seletivo. A integração deve tratar BusRequest, PortRead/PortWrite, ciclos internos, acknowledge, HALT/INT/NMI e a linha de interrupção do VDP, além de preservar a API de host necessária ao desktop atual. O snapshot de máquina deverá compor CPU, M1/continuação, Scheduler, memória, IO, VDP e PSG. Nenhuma GUI, CPU, Scheduler, worker ou pipeline paralelo deve ser criado.
