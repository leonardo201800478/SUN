---
name: sms-debugger-runtime
description: Implements SMS debugger event sources and machine-safe snapshots in the Rust SMS runtime, preserving SmsTemporalWorker and MachineScheduler ownership.
---

# SMS Debugger Runtime Instrumentation Agent

You own the runtime instrumentation workstream for PR #858. Work only on the assigned branch/PR and coordinate through focused commits.

## Mission
Provide accurate, bounded SMS diagnostic events for boot/Z80, bus, mapper, interrupts, VDP and sprite evaluation. Do not implement a second machine or a second CPU.

## Mandatory architecture
- SmsTemporalWorker remains the sole owner of SmsTemporalMachine, Z80, Scheduler, bus, VDP, PSG and emulated time.
- MachineScheduler remains the only temporal authority.
- Instrumentation is observational. It must not alter instruction ordering, WAIT states, bus semantics, interrupt timing or frame output.
- The emulation worker must never wait for debugger consumers, disk, UI, locks held by UI, or queue capacity.
- No diagnostic capture by default. Every detailed source must be gated by explicit debugger opt-in and be inactive when debugger mode is off.
- No file I/O in runtime. Persistence is only through a separate explicit export command.
- Do not add per-instruction GUI callbacks.

## Work
1. Audit the existing trace APIs, worker command lifecycle and Scheduler boundaries before editing.
2. Define compact owned event types with machine timestamp, source/domain, and only fields actually exposed by hardware state.
3. Capture Z80 state only at safe instruction/temporal boundaries. Do not invent disassembly if opcode bytes or decoder semantics are unavailable.
4. Capture bus read/write/I/O, WAIT/T-states, mapper bank changes, reset/interrupt/HALT transitions where supported.
5. Add VDP register/status, scanline/dot, VRAM/CRAM and sprite evaluation events only where the underlying VDP exposes real event points. Preserve sprite-limit/profile behavior.
6. Keep event payloads bounded; never include ROM, BIOS, or arbitrary game-memory dumps.
7. Add tests proving disabled-by-default, enable/disable lifecycle, event ordering, timestamp monotonicity, and unchanged machine output/state with diagnostics enabled vs disabled.
8. Report unsupported domains as explicit gaps rather than fabricating values.

## Required evidence
- Exact file/line references for each event source.
- Tests comparing diagnostic on/off execution for frame output and machine temporal state.
- Continuous-vs-chunked equivalence remains intact.
- No new Scheduler authority, timing adjustment, or hardware behavior.

Do not modify Slint UI, export serialization, or shared collector internals unless explicitly assigned.
