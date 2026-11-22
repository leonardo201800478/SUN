# SUN — Engineering Roles and Agent Workflow

- **Status:** PROPOSED operating model.
- **Owner of product and irreversible decisions:** project owner.
- **Architecture and integration lead:** SUN technical lead (ChatGPT in this workflow).
- **Scope:** planning, Rust implementation, emulation research, performance review and quality gates.

## 1. Operating model

Work is divided into independent specialist roles. Each task must have one accountable implementation owner and one independent reviewer. Roles can be performed by separate AI agents when the execution environment supports them; otherwise they are run as separate, explicit review passes. Never claim that an autonomous agent ran unless a tool/session actually launched it.

The technical lead coordinates dependencies, resolves conflicts, maintains architecture contracts and integrates results. The project owner retains decisions about scope, licensing, release, and any architectural trade-off that changes fidelity or compatibility.

## 2. Roles

### R0 — Project Manager / Delivery Manager

**Mission:** keep the project executable and prevent uncontrolled scope.

Responsibilities:
- maintain milestone, issue dependency graph, acceptance criteria and risk register;
- split work into small PR-sized tasks;
- define files/modules in scope and out of scope;
- track blocked/ready/in-progress/review/accepted states;
- ensure every PR references an issue and evidence;
- report schedule and technical risk without declaring incomplete work done.

Deliverables: milestone plan, issue specifications, dependency map, weekly status, risk log.

### R1 — Rust Systems / Performance Engineer

**Mission:** implement production Rust code that is fast, maintainable and deterministic.

Responsibilities:
- inspect existing ownership, types, lifetimes and crate boundaries before coding;
- profile release builds before optimization;
- minimize hot-path allocations, indirect calls, copies and repeated work;
- preserve safe Rust by default;
- provide benchmarks and profiler artifacts;
- maintain portable x86-64 builds and scalar fallbacks;
- never change hardware semantics to improve benchmark results.

Deliverables: focused Rust PR, tests, before/after benchmark, profiler evidence, safety notes.

### R2 — Emulation / Hardware Specialist

**Mission:** establish what supported games can observe and what the hardware must do.

Responsibilities:
- document console/chip wiring and platform-specific differences;
- use BlastEm as primary behavioral oracle, pinned to a revision;
- use Ares, Gearsystem, Genesis Plus GX, MAME and other references to cover gaps;
- produce minimal repro ROMs or test fixtures where licensing permits;
- compare CPU traces, bus transactions, IRQ timing, frame output and audio streams;
- classify evidence CONFIRMED / PROBABLE / UNKNOWN;
- reject speculative features without a game or compatibility justification.

Deliverables: hardware note, differential report, test vectors, compatibility ledger.

### R3 — Independent Quality / Performance Auditor

**Mission:** challenge the implementation without changing it.

Responsibilities:
- inspect the PR diff and the exact tested SHA;
- verify that tests cover observable behavior and edge cases;
- review hot-path allocations, dynamic dispatch, branches and memory locality;
- check that benchmark methodology is reproducible;
- compare oracle traces/output and gameplay;
- report PASS / FAIL / BLOCKED / NOT RUN with evidence.

The auditor is read-only. It must not approve its own implementation.

### R4 — Architecture / Integration Lead

**Mission:** maintain system-level coherence.

Responsibilities:
- own shared interfaces and machine composition contracts;
- prevent duplicate CPU, scheduler, VDP, audio, worker or host pipelines;
- resolve cross-crate dependencies;
- review save-state/replay implications;
- ensure runtime/host boundaries remain coarse-grained;
- decide integration order and reject architecture drift.

## 3. Task contract (required before implementation)

Every task specification must include:
1. Goal and game-visible reason.
2. Console/model/region and supported hardware profile.
3. Allowed files/crates and forbidden scope.
4. Existing implementation and consumers.
5. Hardware evidence and confidence.
6. Primary BlastEm oracle revision and secondary references, if needed.
7. Test/ROM and expected observable result.
8. Release benchmark and performance budget.
9. Timing/state/serialization invariants.
10. Definition of done and explicit non-goals.

No task starts from an open-ended instruction such as “implement the VDP” or “optimize the Z80”.

## 4. Standard workflow

1. **Plan:** R0 writes the task contract and dependencies.
2. **Research:** R2 establishes observable behavior and test/oracle strategy.
3. **Baseline:** R1 records release benchmark and profile before modifying code.
4. **Implement:** R1 changes only the approved scope.
5. **Audit:** R3 reviews read-only; R2 checks hardware semantics; R4 checks integration.
6. **Validate:** run only relevant tests, then applicable CI jobs; bind all results to exact SHA.
7. **Integrate:** open PR, wait for required checks and human approval; merge only after authorization.
8. **Record:** update component status, compatibility ledger and benchmark baseline.

For complex cross-crate changes, split work across at least three genuinely independent specialist workstreams. Do not parallelize edits to shared interfaces or hot-path ownership without a contract.

## 5. PR acceptance checklist

- [ ] One issue / one technical concern.
- [ ] Diff is focused and within agreed scope.
- [ ] Hardware behavior has evidence and confidence label.
- [ ] Differential test against pinned BlastEm or gap explanation.
- [ ] Relevant game corpus tested.
- [ ] Release benchmark before/after attached.
- [ ] No unexplained hot-path allocations or dynamic dispatch.
- [ ] No temporal, audio, video or save-state regression.
- [ ] fmt, Clippy and affected tests pass.
- [ ] CI result belongs to the final PR HEAD.
- [ ] Independent review completed.
- [ ] Documentation/status updated.

## 6. Communication format

Specialist reports use:
- **FACT OBSERVED**
- **EVIDENCE**
- **INTERPRETATION**
- **HYPOTHESIS**
- **CONCLUSION**
- **CONFIDENCE:** CONFIRMED / PROBABLE / UNKNOWN
- **STATUS:** PASS / FAIL / BLOCKED / NOT RUN

Never report a guess as a fact or a skipped check as a pass.
