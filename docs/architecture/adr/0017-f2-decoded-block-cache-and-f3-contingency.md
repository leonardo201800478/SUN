# ADR-0017 — F2 predecoded block cache and conditional F3 dynamic recompilation

- **Status:** ACCEPTED — implementation direction explicitly approved by the project owner on 2026-11-22; normative after merge.
- **Date:** 2026-11-22.
- **Scope:** CPU execution strategy, starting with the SMS Z80 path.
- **Related:** ADR-0016 (observable fidelity and proportional timing), SUN V2 performance targets, PERF-01.

## Context

The SUN must sustain real-time gameplay with meaningful throughput headroom. The prior SMS implementation failed to maintain 60 FPS with FM active. The project therefore needs an optimization path that improves execution cost without weakening hardware semantics or replacing validated components speculatively.

A dynamic recompiler (JIT) is not automatically the best first optimization. A predecoded block cache can remove repeated instruction decoding while retaining the existing interpreter as the executor and semantic reference. This provides a lower-complexity F2 stage before considering native code generation.

## Decision

1. Start SMS CPU optimization at **F2 — predecoded instruction/block cache**.
2. F2 caches decode metadata and safe block boundaries; it does **not** emit or execute generated host machine code.
3. The existing Rust interpreter remains the instruction executor, fallback path and differential reference.
4. Preserve MachineScheduler as the only temporal authority. F2 may batch dispatch/decode work but may not cross an event horizon or observable operation.
5. **F3 — dynamic native-code compilation** is the contingency if the validated SMS F2 full-FM benchmark remains below 400 guest frames/s and profiling identifies CPU decode/dispatch/execution as a material bottleneck.
6. If F2 misses the throughput target because a non-CPU subsystem dominates, optimize that subsystem first. Do not add JIT as a cosmetic response to a bottleneck it cannot accelerate.
7. The future integrated Megazord target is >=100 guest frames/s with Mega Drive, Sega CD and 32X running together. Its own profiling campaign determines whether F3 or other subsystem optimizations are needed.

## Existing SMS execution path and F2 integration constraint

The current SMS path is not a free-running atomic instruction loop. The active machine uses `SmsTemporalMachine` and `Z80MicroSequencer`:
- `SmsTemporalMachine::step_frame_until` advances by completed instructions;
- `step_instruction` starts the instruction and repeatedly dispatches scheduled bus/internal-cycle events;
- opcode bytes are fetched through the real M1 bus transaction;
- `Z80MicroSequencer::complete_bus_cycle` reaches `decode_opcode_fetch` only after the bus completion edge.

F2 must integrate with this path. A cached decode descriptor may accelerate classification/dispatch, but it must not suppress real opcode fetches, operand reads, refresh-register effects, WAIT insertion, bus contention, interrupt sampling or Scheduler dispatch. A block descriptor is metadata for a sequence of instructions; it is not permission to execute guest instructions outside the existing temporal machine or to collapse observable bus transactions.

The first implementation task must map the current decoder and its consumers before choosing whether the cache is keyed by opcode/prefix templates, PC plus mapping identity, or another bounded representation. The cache design must fit the existing microsequencer rather than replacing it with the atomic `Z80::step` path.

## F2 correctness requirements

The implementation task specification must identify and test:
- cache identity for address space, machine profile and mapper/bank mapping;
- invalidation after writes to executable RAM or any memory region that can change fetched instructions;
- invalidation or version changes after mapper/bank remaps, reset and state restoration;
- handling of instruction prefixes, variable-length decode, page/bank boundaries and block termination;
- preservation of every required M1 opcode fetch, operand bus read/write, refresh-register update and bus-visible fetch side effect;
- exits before MMIO or bus operations with observable side effects;
- exits at Scheduler horizons and before events that can affect CPU execution;
- correct interrupt sampling, WAIT, DMA/arbitration and resumable execution where the active machine requires them;
- exact architectural state, flags, memory effects and required cycle accounting;
- deterministic save-state/replay behavior;
- bounded/preallocated cache behavior and no unbounded allocation in hot paths.

The invalidation mechanism (for example page generations or explicit invalidation lists) is an implementation choice to be selected after auditing the current memory/bus architecture. This ADR does not prescribe a new cache crate or a parallel CPU.

## F3 entry and constraints

F3 is entered when the measured F2 SMS result is below 400 guest frames/s, the benchmark is valid, and profiling demonstrates that CPU execution is a material cause. The F3 task must:
- remain isolated behind an explicit feature/architecture boundary;
- retain the interpreter as fallback and oracle;
- support writable/executable RAM and mapper/bank invalidation;
- exit on observable bus/MMIO operations, interrupts, WAIT, DMA/arbitration and Scheduler horizons;
- preserve cycle accounting, suspension/resumption and deterministic state;
- provide portable interpreter fallback for unsupported host architectures;
- review executable-memory safety, security, debugging and maintenance cost;
- pass differential tests and whole-game benchmarks, not only compiler microbenchmarks.

The latest available historical ABBA result (commit `331fc7e97bf67964b0db6ad84ed7e27d171705d3`) measured 56.7438 FPS in the ThinLTO FM-mixed scenario. Its separate diagnostic window measured approximately 5.412 ms/frame around `SmsTemporalBus::advance_ym2413`, while total frame processing was approximately 19.162 ms. These data are not a current-main baseline and do not isolate the entire unexplained cost. Re-run the campaign on the current HEAD and profile by function before attributing the remaining cost to CPU execution.

A specific JIT backend (Cranelift, dynasm or another implementation) is **not selected by this ADR**. Backend selection requires a separate measured technical decision.

## Performance gates

- SMS F2 target: >=400 guest frames/s with YM2413/FM actively clocked, rendered/mixed and combined with SN76489, under the workload defined in the SUN V2 performance-target document.
- Megazord target: >=100 guest frames/s for the complete Mega Drive + Sega CD + 32X machine after that platform is implemented.
- Product requirement: stable 60 FPS real-time execution with integrated video/audio/input on the declared modest reference device.
- Ares comparisons must normalize workload and metric; VPS must not be assumed equivalent to FPS.

The performance targets are throughput goals, not permission to change guest-visible behavior or weaken temporal fidelity.

## Consequences

- F2 can be implemented and measured without introducing runtime code generation.
- The interpreter remains available for fallback and differential validation.
- Cache invalidation and temporal exits become first-class correctness contracts.
- F3 remains possible, but its cost and expected benefit must be justified by profiling.
- The project avoids assuming that CPU JIT will solve FM, VDP, Scheduler, mixer or host bottlenecks.
