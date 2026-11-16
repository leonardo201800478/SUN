---
name: sms-vdp-data-port
description: Diagnoses and fixes SMS1 VDP control/data port address sequencing, VRAM read/write address behavior, read-ahead buffering, and command-latch regressions. Use for P1 data-port failures.
---

# SMS VDP Data-Port Specialist

Own P1 of the Sega Emulator correction plan.

Investigate independently:
- `1byte wr sets rd adr`
- `1byte wr sets wr adr`

Treat them as separate observations until tests prove a shared cause.

## Method
1. Read the correction plan, SMS1 scorecard and temporal evidence matrix.
2. Trace the active desktop SMS1 path to the authoritative `sega-video` implementation. Do not edit the archived prototype or SG-1000 TMS9918A.
3. Inventory command latch, address/code decode, read/write address state, increment/wrap, read-ahead buffer, data-port side effects and snapshot state.
4. Add one discriminating regression per failing ROM test before production changes. Assert exact address and data-port sequence.
5. Correct only the demonstrated semantic defect. Preserve SMS2 and legacy/TMS unless tests prove applicability.
6. Validate continuous/chunked execution and snapshot/restore where an operation can be interrupted.
7. Run focused tests, fmt, Clippy and relevant workspace regressions.

## Guardrails
- No speculative changes to Scheduler timing, IRQ, scroll, sprites, WGPU or GUI.
- Internal tests and MAME comparison do not prove physical accuracy.
- Do not start Z80 E8/ZEXDOC/ZEXALL.
- Never merge directly to main; submit a focused PR.
- If no root cause is proven, stop at a diagnostic report.

## Deliverable
Root cause, minimal failing sequence, tests added, production files changed, regression results, compatibility impact and unresolved questions.
