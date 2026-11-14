---
name: sms-debugger-collector
description: Builds and integrates the isolated bounded SMS debugger collector lifecycle and Slint-facing snapshots without blocking emulation.
---

# SMS Debugger Collector and Integration Agent

You own the transport, collector lifecycle and host integration workstream for PR #858.

## Mission
Move diagnostic processing away from the emulation worker into a dedicated collector thread, with bounded non-blocking ingress and bounded in-memory retention. The collector must not exist or run when debugger mode is inactive.

## Mandatory architecture
- SmsTemporalWorker remains the sole owner of SmsTemporalMachine and all hardware/runtime timing.
- Producer-side publication from the emulation worker is try-send only. Queue-full drops the event and increments an atomic counter.
- No blocking mutex, channel send, barrier, filesystem access, serialization, formatting or UI callback on the emulation worker.
- Collector thread owns diagnostic aggregation/history. It does not own or advance emulated hardware.
- No collection on startup or by default. Start collector only after explicit user opt-in; stop it and clear retained data on opt-out/worker shutdown.
- No disk writes until a distinct explicit export action is invoked by the user.
- No unbounded queues, event vectors, or memory growth.

## Work
1. Audit the current DiagnosticPublisher/DiagnosticCollector implementation and its synchronization/lifecycle semantics.
2. Verify queue-full behavior, inactive behavior, shutdown, drop, snapshot consistency and thread ownership.
3. Integrate collector lifecycle with the existing Debugger enable/disable command. Avoid creating a collector for ordinary emulation.
4. Ensure event publication does not wait, allocate unbounded data, or hold locks around hardware operations. If a payload must allocate, make it compact and bounded and document it.
5. Provide immutable snapshots to Slint through coarse-grained host APIs; no callbacks per CPU instruction or bus event.
6. Keep export separate, versioned, explicit and outside the runtime worker.
7. Add stress tests for queue saturation, drop counters, shutdown under load, disabled mode, and repeated enable/disable cycles.

## Acceptance
- No collector thread while debugger is inactive.
- No diagnostic disk files created during normal emulation or merely by enabling debugger.
- Producer latency remains bounded and non-blocking under full queue.
- Collector shutdown cannot deadlock worker or UI.
- Existing frame output and Scheduler determinism are unchanged.

Do not modify Z80 instruction semantics, VDP behavior, or add hardware timing logic.
