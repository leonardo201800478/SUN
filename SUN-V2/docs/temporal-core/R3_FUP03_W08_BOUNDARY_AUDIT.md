# R3 FUP-03 — W08 TransactionBook / ResourceLedger boundary audit

**Status:** PARTIAL — composition behavior tested; recovery remains incomparable
**R3 baseline:** `98cd9e258ae463e32397a119d4d4e6a007346c22`
**W08 composition PR:** #1293, corrected HEAD `354d2a5da7b5d12885ade6f045de8387f309acde`
**Correction CI:** #46173011966 SUCCESS — https://github.com/leonardo201800478/SUN/actions/runs/46173011966
**Earlier composition evidence:** PR #1293 / CI #11289 SUCCESS at `7d4eb4dbaa729a49a588fe4f7bbbcf8e4742efa5`

## 1. TransactionBook storage budget decision

**Decision for R3: do not add a TransactionBook capacity field or transaction-count limit. Defer any runtime storage budget to a separately specified implementation concern after memory/performance and snapshot/replay retention requirements are measured.**

Evidence and rationale:

- R2 `TransactionBook` and R3 `TransactionBook` both store an immutable tuple of transaction records. Neither has a `capacity`, `max_transactions`, byte count, eviction policy or hardware limit.
- Submit is keyed by stable transaction identity. An identical descriptor retry returns the recorded transaction; a divergent descriptor under the same identity is InvalidInput without mutation.
- `generation` is part of the descriptor. It is not a replacement mechanism: same identity with a different generation is a divergent retry, not an implicit successor.
- Terminal records are retained in the current model. There is no R2 contract or R3 evidence authorizing eviction, compaction, or generation replacement.
- The bridge's one-transaction/one-reservation restriction is explicitly an isolated fixture/composition scope, not a global TransactionBook capacity.
- The R3 ResourceLedger capacity is a distinct, caller-supplied abstract budget. It does not represent transaction count, memory bytes, bus cycles, or a physical SMS limit.

A future implementation may need a host-side memory budget, but it must be separately named and specified, must not be confused with ResourceLedger capacity, and must not evict state required by active transactions, snapshots or replay. No fixed per-transaction cost or numeric budget is inferred here. This decision is not a hardware claim.

## 2. ResourceLedger contract boundary

R2-T15 and R3 ResourceLedger model an explicit abstract reservation budget:

- `capacity` and `used` are integer budget units supplied by the fixture/caller.
- Reservation admission is atomic; exactly-at-capacity is admitted and over-capacity returns ResourceExhausted without partial mutation.
- Reservation identity retry is idempotent only for the same amount/effect set; divergent reuse is InvalidInput.
- Effects are partitioned between pending and committed sets. A committed effect is exactly-once/idempotent.
- Failure preserves the committed prefix and pending suffix.
- Resource recovery commits the pending suffix and marks the resource ledger completed.

None of these units is mapped to hardware transaction slots, bytes, CPU cycles, memory size or SMS bus occupancy.

## 3. Composition bridge invariants

`transaction_resource_bridge.py` is an independent disposable composition fixture:

1. The fixture supplies the reservation amount and the marker→effect mapping explicitly.
2. The bridge validates both ledgers and the mapping before mutation.
3. Reservation and transaction submit are composed atomically: a rejected reservation or submit returns the original composite state.
4. Each commit marker and its mapped resource effect commit together; retries are idempotent.
5. The bridge checks that transaction committed markers correspond exactly to ResourceLedger committed effects, and pending effects correspond to uncommitted markers.
6. A partial failure preserves the same committed prefix and pending suffix in both ledgers.
7. The bridge accepts only one transaction and one reservation in an isolated state. This is a fixture scope restriction, not a physical capacity claim.

The mapping is not inferred from marker names. Marker IDs and effect IDs are distinct namespaces joined only by fixture input.

## 4. Recovery is intentionally incomparable

The two independent recovery operations have different terminal semantics:

| TransactionBook recovery | ResourceLedger recovery |
|---|---|
| Requires a Failed transaction | Requires a failed resource ledger |
| Resumes transaction as InProgress | Marks ledger completed |
| Preserves sampled/committed transaction markers and pending markers | Moves all pending effects into committed effects |
| Does not imply that the pending suffix was physically committed | Explicitly commits the resource pending suffix |

The bridge's `audit_recovery_boundary()` returns `IncomparableRecovery` and records the differences without applying either recovery path. The regression test confirms the audit does not mutate the composite state.

**Do not** call TransactionBook recovery and ResourceLedger recovery sequentially as if they were one atomic operation. No evidence defines which subsystem owns recovery, whether effects should be replayed, or how a real device reconciles partially committed state.

## 5. Generation and identity

- TransactionBook identity is stable across retries.
- Generation is a descriptor field checked as part of exact retry equality.
- Same identity + different generation/descriptor is invalid retry.
- No TransactionBook generation replacement, old-generation fencing or eviction semantics are inferred from W05 deadline generation replacement.
- W05 deadline generations and W08 transaction request identities remain separate contracts.

## 6. Coverage crosswalk

| Area | R2 evidence | R3 evidence | Status |
|---|---|---|---|
| Transaction identity/retry | R2-T08 `test_transaction.py` (18 cases) | P-A transaction tests (18); W08 replayable fixtures (12); shared transaction tests (3) | **Strong functional coverage** for tested lifecycle; no capacity contract |
| Sample/commit exactly-once | R2-T08 marker/sample/commit cases | P-A transaction tests; W08 replayable fixtures; shared transaction tests | **Covered** for selected marker traces |
| WAIT/TW lifecycle | R2-T08/T09 | Shared transaction tests and W08 fixtures | **Covered** for modeled automatic TW, external TW and WAIT sources; not a hardware timing claim |
| Resource capacity/reservation | R2-T15 `test_resource.py` (10 cases) | R3 ResourceLedger tests (5) and bridge tests (capacity boundary + retry) | **Covered** for explicit abstract budget semantics |
| Bridge reserve + submit atomicity | R2 ResourceLedger and TransactionBook independently | `test_reservation_and_transaction_admission_are_atomic_at_capacity` | **Covered** for fixture-supplied mapping/amount and one-transaction scope |
| Marker/effect commit atomicity | R2 separate ledgers | `test_commit_marker_and_resource_effect_commit_exactly_once` | **Covered** for selected mapping |
| Partial failure | R2-T08/T15 partial prefix/suffix tests | `test_partial_failure_preserves_matching_committed_prefix_and_pending_suffix` | **Covered** for selected fixture |
| Recovery composition | R2 TransactionBook and ResourceLedger recovery have distinct contracts | `test_transaction_and_resource_recovery_are_explicitly_incomparable` | **Incomparable by design; not closed** |
| TransactionBook storage limit | No R2 capacity/eviction contract | No R3 capacity field; tuple retains records | **No semantic limit justified; runtime budget deferred** |
| Transaction generation replacement | No evidence that transaction generation is replaceable | Exact descriptor retry only | **UNKNOWN/not inferred** |

## 7. CI evidence and exact scope

- PR #1293 initial composition HEAD `7d4eb4dbaa729a49a588fe4f7bbbcf8e4742efa5`, CI #11289 SUCCESS: R3 282/282 Linux and Windows; P-A 186/186 per OS; R2 independent 256/256 Linux; R2 Python 256/256 Windows; mutation checks, corpus equivalence and Rust quality/unit regression passed.
- Correction HEAD `354d2a5da7b5d12885ade6f045de8387f309acde`, CI #46173011966 SUCCESS: R2 independent Linux, R2 Python Windows, R3 Linux/Windows, canonical/replayable corpus equivalence and Rust quality/unit regression passed.
- The correction validates malformed retry descriptor ordering: `spec=None` returns InvalidInput before reading `spec.identity`; composite state remains unchanged.
- Production SMS/Z80/VDP/PSG/YM2413/CPAL/SDL3/Slint/WGPU/scheduler jobs were SKIPPED, not PASS.

## 8. Remaining FUP-03 work

1. Keep the storage-budget decision above; do not add arbitrary `max_transactions`, byte costs or eviction.
2. Preserve ResourceLedger capacity as caller-supplied abstract units.
3. Keep TransactionBook and ResourceLedger recovery separate until a normative owner/sequence and effect-reconciliation contract exists.
4. Expand bridge composition invariants only with a defined observable state transition; do not infer marker/effect mapping.
5. Keep transaction generation replacement UNKNOWN/not supported by current evidence.
6. FUP-03 remains PARTIAL because recovery composition and broader integration invariants are not equivalent or closed.
