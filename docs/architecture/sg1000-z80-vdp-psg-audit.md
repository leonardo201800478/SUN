> **Estado vigente (2026-10-27):** o hardware SG-1000 ativo está em Rust. Implementações Python anteriores e ferramentas que dependiam delas foram arquivadas em `legacy/` e não fazem parte do runtime ou da validação ativa. Trechos que descrevem o estado anterior são históricos.

# SG-1000 Z80, VDP and PSG: audit and completion plan

## Purpose

Close the functional gaps in the native Rust SG-1000 machine by auditing the existing implementation against the Zilog Z80 programmer's model and documented SG-1000 peripherals. This document is an acceptance ledger, not a claim that the CPU or machine is already complete.

## Baseline observed on `main`

- `native/sg1000_core/src/cpu.rs` explicitly describes itself as an incremental subset and returns `CpuError::UnsupportedOpcode` for unimplemented instructions.
- The CPU currently models only the primary AF/BC/DE/HL registers, SP, PC, HALT and aggregate T-states. Alternate register banks, I/R, interrupt flip-flops/modes, and prefix state are not represented in the public CPU state.
- `native/sg1000_core/src/vdp.rs` and `psg.rs` exist as native modules. Their existence and unit tests alone do not establish complete hardware behavior or cycle-level synchronization.
- The current machine advances devices at instruction boundaries. Do not describe this as bus-cycle-accurate emulation; wait states, contention and sub-instruction event ordering require a separate explicit timing model.

## Progress in the current PR branch

The following implementation slices have been added, but this ledger remains open until independent coverage and integration gates pass on the final candidate SHA:

- Z80 state now includes alternate AF/BC/DE/HL, IX/IY, I/R, IFF1/IFF2, interrupt mode, WZ/MEMPTR, EI delay, NMI/INT requests, Q latch and the P/V interrupt edge latch.
- The primary opcode decoder is extended with the CB matrix, ED arithmetic/load/I/O/block families, and DD/FD index substitution, displacement, repeated-prefix and indexed-CB handling.
- The refresh register increments on M1 opcode fetches and HALT refresh cycles; SCF/CCF use the Q latch, and the LD A,I/R interrupt edge behavior is modeled.
- NMI and maskable interrupt service cover IM 0/1/2. IM 0 injects the external opcode into the common decoder and fetches any operands from the current PC.
- The machine synchronizes the VDP's enabled VBlank interrupt line to the Z80 INT request at instruction boundaries.
- VDP regressions cover control latching, VRAM read-ahead/address wrap, all four display modes, raster/framebuffer agreement, VBlank/status effects, sprite overflow/collision, transparency, offscreen collision and Y-coordinate wrap.
- PSG regressions cover noise-rate selection including tone-3 linkage, attenuation/mute behavior, fixed LFSR vectors, period/frequency calculations and chunking determinism.

### Known limitations that prevent a completion claim

- The scheduler still advances devices at instruction boundaries; this is not bus-cycle-accurate emulation.
- The primary matrix and CB/ED/DD/FD families have encoding-classification coverage and focused regressions, but still require exhaustive independent vectors for flags, prefix interactions and timing.
- IM 0 opcode injection still needs additional independent edge-case vectors for prefixes, HALT and acknowledge timing.
- The VDP and PSG now have focused fixed-vector regressions, but need remaining independent vectors for all mode/address combinations, sprite priority/size/magnification, status edge cases, PSG clock/LFSR variants and audio pipeline behavior; unit tests alone do not prove full silicon fidelity.
- A green Python SingleStepTests workflow is evidence for the Python adapter only unless the workflow explicitly executes the native Rust CPU. Do not present it as native Rust differential coverage.

## Workstreams

### CPU: documented Z80 instruction coverage

Implement in independently reviewable slices, retaining explicit errors until each instruction family has tests:

1. Base opcode matrix: all 256 primary opcodes, including undocumented/undefined encodings classified explicitly rather than silently treated as NOP.
2. Register model: AF'/BC'/DE'/HL', IX/IY, I/R, IFF1/IFF2, interrupt mode, and HALT/interrupt state.
3. CB prefix: rotate/shift, BIT, RES, SET for registers and (HL), including flags and documented T-states.
4. ED prefix: 16-bit arithmetic/load, block transfer/search/I/O, interrupt-mode, I/R transfers, RETN/RETI and aliases, with each encoding classified.
5. DD/FD prefixes: IX/IY substitution and displacement forms, including prefix repetition and DD/FD interactions with CB/ED.
6. Flag correctness: S/Z/H/PV/N/C and undocumented X/Y bits; overflow, half-carry, carry preservation, DAA, block-operation flags, and edge cases.
7. Control state: DI/EI delay semantics, IM 0/1/2, interrupt acknowledge behavior, NMI, RETN/RETI, HALT exit, and reset behavior.
8. Timing and bus contract: distinguish instruction totals from individual memory/I/O machine cycles; do not claim cycle accuracy until bus events have timestamped tests.

### VDP: TMS9918A behavioral audit

- Verify register write latch and two-byte command sequencing, read-ahead buffer semantics, VRAM address wraparound, status-read side effects, interrupt/VBlank behavior, and reset defaults.
- Validate each supported display mode and name/pattern/color/sprite table addressing against independent golden vectors or a trusted reference.
- Test sprite limits, overflow/collision status, transparency, priority, palette/color mapping, scanline/frame boundaries and framebuffer determinism.
- Separate documented chip behavior from SG-1000 board wiring, inferred behavior, and unresolved hardware-specific questions.

### PSG: SN76489 behavioral audit

- Validate latch/data protocol, tone periods including zero-period behavior, attenuation table, periodic/white noise feedback taps and shift direction, noise clock selection, and tone-3-linked noise.
- Test exact clock-to-sample accounting, chunking invariance, reset, bounded buffers and deterministic output.
- Treat FIR/filtering and output scaling as analog/output-pipeline choices, not as proof of digital chip fidelity.
- Confirm the clock source and divider for the target SG-1000 board from reliable hardware documentation before fixing production defaults.

### Machine integration

- Route CPU memory and port operations through the machine bus with deterministic ordering.
- Test event timestamps and CPU/VDP/PSG advancement over split and unsplit execution windows.
- Add integration vectors covering ROM/RAM mapping, mirrored or unmapped regions, controller I/O, VDP ports, PSG writes, frame output and audio samples.
- Keep the frontend/Qt/PyO3 boundary out of per-instruction execution; expose batch/frame-level APIs only.

## Primary hardware references

- **Texas Instruments, TMS9918A/TMS9928A/TMS9929A Video Display Processors Data Manual (1982)** — [PDF mirror of the original TI manual](https://computers.baffa.tec.br/pages/datasheet/TMS9918A_TMS9928A_TMS9929A_Video_Display_Processors_Data_Manual_Nov82.pdf). Relevant sections: 2.1.2–2.1.6 (register protocol, VRAM, status and interrupt), 2.3 (status flags), 2.4 (display modes) and 2.4.5 (sprites).
- **Texas Instruments, SN76489AN Data Sheet** — [PDF mirror](https://map.grauw.nl/resources/sound/texas_instruments_sn76489an.pdf) and [text-extracted copy](https://computers.baffa.tec.br/pages/datasheet/76489.pdf). Relevant sections: tone-divider equation, attenuation table, noise feedback/rate selection, register protocol and data transfer.
- The PDFs are original manufacturer documents hosted on third-party mirrors; the host is not the manufacturer. Use these manuals as the primary behavioral reference and record any variant-specific differences separately.

## Independent validation

Use fixed, independently sourced vectors where licensing and provenance permit. Candidate sources include Zilog manuals, the project's uploaded Z80 technical manuals, public SingleStepTests vectors (verify their license and schema before importing), and independently generated synthetic programs. Never treat expected values calculated by the implementation under test as an oracle.

For each opcode family, record:
- opcode/prefix and operand form;
- before/after register and memory state;
- flags, PC/SP and wraparound;
- T-states and bus access sequence (when modeled);
- unsupported/undefined classification;
- source/provenance of expected behavior.

## Required gates

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- Z80 differential tests and prefix/opcode coverage report.
- VDP and PSG fixed-vector regressions plus deterministic repeat/chunking tests.
- Integration tests for CPU + bus + VDP + PSG + machine scheduler.
- Python pytest/Ruff and Release/Maturin/Qt parity checks when the extension or frontend is touched.

Report each gate as PASS, FAIL, BLOCKED or NOT RUN against the exact candidate SHA. A skipped workflow is not a pass.

## Completion criteria

The CPU is complete for the selected Z80 silicon profile only when every encoding is implemented or explicitly classified, all supported instruction families have independent state/flag/timing tests, and interrupt/prefix behavior is covered. The SG-1000 standalone milestone remains open until CPU, bus/memory, I/O, VDP, PSG, scheduler and the required frontend integration are tested together. No claim of cycle accuracy is allowed without timestamped bus-level evidence.


## Saída host e pipeline de áudio

Separar os testes do SN76489 (estado digital, clocks, writes e vetores de amostras) da sessão host (resampling, fila, underruns e API Windows). O caminho Qt ativo encaminha amostras diretamente à sessão Rust; não usar o frontend como oráculo de síntese. WASAPI Shared é padrão e XAudio2 alternativa. Ver [Runtime de áudio nativo](native-audio-runtime.md).


## Reconciliação de arquitetura pós-main 22c489e03 — 16/11/2026

Esta seção atualiza a leitura do baseline e supersede somente as afirmações antigas incompatíveis com o código atual. O histórico do plano permanece preservado.

### Estado efetivo observado na main

- `native/sg1000_core/src/cpu.rs` agora é uma fachada de compatibilidade: `pub use sega_z80_cpu::cpu::*;`. O SG-1000 já aponta para o executor Z80 compartilhado; a afirmação histórica de que o arquivo ainda contém um subconjunto local do decoder não descreve mais o código atual.
- `native/sg1000_core/src/machine.rs` ainda expõe `Sg1000Machine::step()` e `run_instructions()` com execução por instrução. O próprio módulo declara que o timing é instruction-boundary based, não cycle-exact.
- `native/sg1000_core/src/temporal.rs` fornece `Sg1000TemporalCoordinator` e um domínio Z80; `native/sg1000_core/src/temporal_bus.rs` fornece transações de memória/acknowledge e ciclos internos agendados.
- Esses dois componentes temporais ainda não estão conectados ao `Sg1000Machine`. A máquina de produção/referência continua avançando VDP e PSG em torno de instruções completas.
- `Sg1000TemporalBus::is_supported_bus_operation` aceita OpcodeFetch, MemoryRead, MemoryWrite e InterruptAcknowledge. PortRead/PortWrite são explicitamente rejeitados; logo, VDP/PSG I/O ainda não percorrem o caminho temporal.
- `native/sg1000_core/src/lib.rs` descreve o crate como temporary Rust reference core durante a migração SMS-first, não como runtime autônomo ativo do produto. A existência do crate no workspace e de testes unitários não fecha o milestone standalone.

### Sequência P2 após o fechamento E7

1. Criar a máquina temporal SG-1000 que conduza o `sega-z80-cpu` retomável pelo `Sg1000TemporalCoordinator`, sem um segundo decoder e sem alterar a semântica compartilhada.
2. Completar o adaptador de barramento para portas de I/O: leituras do TMS9918A/controladores e writes do SN76489, com side effects no deadline agendado. Não encaminhar I/O por callbacks da GUI.
3. Integrar VDP, PSG e I/O ao scheduler com razões de clock explícitas e documentadas. Não assumir que a constante atual de 3.579.545 Hz prova a frequência/divisor de cada periférico ou variante de placa.
4. Definir snapshot versionado do estado composto: CPU/microsequenciador, scheduler, transação pendente, memória, VDP, PSG, I/O/controladores e restos de conversão de clock/amostras.
5. Provar execução contínua versus execução em blocos, restore em cada safe point, ordem causal e que writes/reads não são repetidos após restore.
6. Validar ROMs e vetores independentes de boot, memória/mirrors, I/O, VDP e PSG; classificar DOC/HW/COMP/INT e manter limites de oráculo explícitos.
7. Só depois expor API coarse-grained para o host/Slint: load, start, pause, stop, reset, step_frame e extração de frame/áudio. Nenhum ciclo/instrução deve atravessar a GUI.

**Gate de entrada:** não iniciar alteração funcional do SG-1000 antes de E7 estar formalmente CLOSED. A preparação e auditoria acima não alteram o runtime. E8 continua bloqueada até o fechamento E7; o SG-1000 P2 reutilizará o Z80 compartilhado já validado e não exige reabrir E8.


**Atualização de baseline — 16/11/2026:** main `1b16256358e3f17837a95cc071048c5ee5307f82` integra a PR #972, que altera somente testes de integração SMS/Scheduler. Nenhum código SG-1000, Z80 de produção, VDP, PSG ou I/O foi modificado por essa integração. O estado P2 descrito acima permanece válido; sua implementação funcional continua bloqueada até o fechamento formal de E7.


**Atualização de baseline — pós-PR #975:** main `22c489e03aaed440bbb150ee9a4b7b70c74b1b84` recebeu somente testes DD/FD Ignored no crate Z80; nenhuma implementação SG-1000/VDP/PSG/I/O foi alterada. O estado P2 acima permanece inalterado e segue bloqueado até E7 CLOSED.
