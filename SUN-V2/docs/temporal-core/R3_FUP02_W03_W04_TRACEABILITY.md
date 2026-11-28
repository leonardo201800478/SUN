# R3 FUP-02 — W03/W04 and R2-T07 traceability

**Status:** PARTIAL — causal synchronization coverage expanded; not a gate approval  
**R3 baseline:** `98cd9e258ae463e32397a119d4d4e6a007346c22`  
**P-E differential:** PR #1291, HEAD `a729acf77205fcef7ff5a2291c3d164b98b1c8fb`, CI #11284 SUCCESS  
**CI:** https://github.com/leonardo201800478/SUN/actions/runs/46169747181  
**Scope:** W03 observation-directed dependency closure and W04 repeated observations; applicable R2-T07 partial-order cases.

## 1. Evidence model

- P-A is the independent full-scan candidate and is directly compared with R2 in its own tests and replayable W03/W04 fixtures.
- P-E is the synchronization-on-demand candidate. Its expanded differential tests compare P-E against P-A, not directly against R2.
- The W03 and W04 replayable fixtures rebuild independent P-A and R2-owned model objects and compare complete state cuts. They do not execute P-E.
- Therefore, P-A↔R2 and P-E↔P-A are two distinct evidence edges. They do not, by themselves, establish one identical end-to-end P-E↔P-A↔R2 replay trace.
- Synthetic transition labels, dependencies, commutativity relations and coordinates do not establish physical SMS bus/device order or sample phase.

## 2. R2-T07 causal synchronization — R2 test_causal.py (13 methods)

| R2 case | P-A evidence | P-E evidence | Assessment |
|---|---|---|---|
| `test_observation_applies_only_target_transitions_at_or_before_sample` | Same-named P-A test | P-E on-demand tests exercise requested-domain closure and selected transitions; P-E/P-A matrix has selected-observation cases | **Partial** — P-E selected-domain semantics are tested, but not the complete R2 observation/alters contract. |
| `test_write_commit_synchronizes_only_the_altered_destination` | Same-named P-A test | No exact altered-destination counterpart identified in P-E | **Absent/partial** — do not equate a requested observation with an altered destination. |
| `test_declared_cross_device_dependency_is_resolved_before_observation` | Same-named P-A test | `test_transitive_dependency_closure_adds_only_required_domains`; `test_explicit_same_instant_causality_orders_transitions`; P-E/P-A `test_explicit_same_instant_dependency_orders_noncommutative_transitions` | **Direct for declared dependency closure/order; partial for full state mapping.** |
| `test_unrelated_device_is_not_advanced` | Same-named P-A test | `test_unrelated_domains_are_not_advanced`; P-E/P-A `test_repeated_observation_is_idempotent_and_unrelated_domain_is_unchanged` | **Direct for non-advancement.** |
| `test_missing_dependency_blocks_without_partial_progress` | Same-named P-A test | `test_missing_dependency_is_invalid_before_any_progress`; `test_same_instant_dependency_cycle_blocks_without_mutation` | **Partial.** P-E distinguishes an unknown/missing transition descriptor as InvalidInput; R2's missing dependency scenario returns Blocked. These must not be collapsed into one case. |
| `test_reciprocal_dependency_blocks_without_speculative_advance` | Same-named P-A test | `test_same_instant_dependency_cycle_blocks_without_mutation` | **Direct for cycle/no-mutation result;** exact reciprocal cross-domain fixture shape is not identical. |
| `test_required_participant_must_be_declared` | Same-named P-A test | `test_unknown_requested_participant_blocks_without_mutation` | **Partial/direct for unknown requested participant Blocked;** distinguish from invalid transition owner/dependency identities. |
| `test_noncommutative_simultaneous_transitions_remain_indeterminate` | Same-named P-A test | `test_unknown_same_instant_pair_is_indeterminate_without_mutation`; P-E/P-A expanded tie test | **Direct for selected pairwise UNKNOWN tie.** |
| `test_commutative_simultaneous_transitions_have_canonical_result` | Same-named P-A test | `test_commutative_same_instant_pair_uses_stable_order`; P-E/P-A pairwise commutative differential | **Direct for a pair; not exhaustive permutation equivalence.** |
| `test_request_behind_participant_progress_is_late_without_mutation` | Same-named P-A test | `test_retrograde_target_is_late_input`; P-E/P-A `test_retrograde_observation_is_late_without_mutation` | **Direct for selected retrograde target case.** |
| `test_invalid_descriptor_precedes_dependency_resolution` | Same-named P-A test | `test_missing_dependency_is_invalid_before_any_progress`; `test_unknown_observed_identity_is_rejected_without_exception`; `test_float_target_is_rejected` | **Partial.** P-E has validation-before-progress tests, but the complete R2 descriptor-precedence matrix is not one-to-one. |
| `test_same_instant_dependency_declares_order_without_physical_phase` | Same-named P-A test | `test_explicit_same_instant_causality_orders_transitions`; P-E/P-A expanded differential | **Direct for explicit dependency order in synthetic state.** No physical phase is inferred. |
| `test_preapplied_causal_ledger_must_match_owner_progress_and_dependencies` | Same-named P-A test | No corresponding pre-applied ledger contract/test identified in P-E | **Absent.** P-E's observed transition index is not a substitute for R2's pre-applied causal ledger. |

P-A has an additional `test_repeated_observation_trace_matches_r2_at_each_cut` that captures complete state after each observation and compares each cut directly with R2.

## 3. R2-T07 partial-order matrix — R2 test_partial_order.py (6 methods)

| R2 partial-order case | P-A/P-E correspondence | Assessment |
|---|---|---|
| `test_phase_order_without_matching_evidence_does_not_create_precedence` | P-E `test_w06_different_phase_bases_remain_unknown`; P-E `test_w06_common_phase_basis_is_a_synthetic_positive_control` | **Partial.** It proves the candidate keeps different bases UNKNOWN and has a synthetic positive control; it does not validate an SMS phase basis. |
| `test_unknown_noncommutative_tie_never_closes_watermark_under_insertion_permutation` | P-A unknown simultaneous groups under insertion permutations; P-E unknown same-instant pair | **Partial.** P-E tests a pair, not the exhaustive R2 permutation family. |
| `test_all_120_permutations_of_five_commutative_events_are_equivalent` | P-A canonical commutative tie test; P-E pairwise commutative order and cache test | **Absent for the 120-permutation property in P-E.** A pairwise stable-order result is not a substitute. |
| `test_all_120_permutations_of_five_noncommutative_unknown_events_stay_indeterminate` | P-A unknown tie test; P-E unknown pair test | **Absent for the 120-permutation property in P-E.** |
| `test_all_120_permutations_preserve_five_event_explicit_dependency_order` | P-A same-instant dependency order; P-E transitive dependency closure and explicit same-instant dependency | **Partial.** Selected chains are covered; no five-event 120-permutation property in P-E. |
| `test_all_120_permutations_preserve_partial_order_with_commutative_siblings` | P-A dependency and commutativity tests; P-E pairwise commutative and dependency tests | **Absent for the combined exhaustive partial-order family in P-E.** |

Additional R2 deadline tests classified under T07 include noncommutative UNKNOWN ties and commutative tie invariance in `test_deadlines.py`. Their W05 deadline lifecycle crosswalk is recorded separately in `R3_FUP01_W05_TRACEABILITY.md`.

## 4. W03/W04 replayable fixtures

| Fixture | R3 test | Comparator | What is established | Limitation |
|---|---|---|---|---|
| W03 cross-device dependency + missing dependency | `test_w03_declared_dependency_synchronizes_and_missing_dependency_blocks` | P-A vs independent R2 at each of 2 cuts | VDP dependency resolves before CPU observation; missing PSG dependency blocks atomically; unrelated device remains unchanged | Synthetic causal contract only; P-E is not part of this replay fixture. |
| W04 repeated observations | `test_w04_repeated_observations_preserve_each_complete_state_cut` | P-A vs independent R2 at each of 4 cuts | Each exact rational cut preserves complete participant state and cumulative applied ledgers | Synthetic cut labels are not SMS observation cadence; P-E is not part of this replay fixture. |

P-E adds a separate P-E/P-A repeated-observation differential in `test_p_e_differential_matrix.py` and P-D/P-E `test_p_e_frequent_observations_match_p_a_w04_causal_sync`. These are useful corroborating comparisons, but not a three-model replay of the same W04 fixture.

## 5. W06/W07 — explicit unresolved boundaries

- **W06 physical phase basis: UNKNOWN.** P-E tests that different synthetic bases remain UNKNOWN and a common synthetic basis can act as a positive control. Neither test authenticates a physical SMS phase basis, captures a hardware edge, or proves a phase-order relation.
- **W07 relation B/C: UNKNOWN/BLOCKED.** `test_w07_unknown_b_c_relation_is_not_inferred_from_microstep` explicitly prevents microstep numbering from inventing the missing B/C relation. It is a negative control, not a resolved relation.
- No metadata, insertion order, pairwise relation, or positive synthetic control changes either status.

## 6. CI evidence

PR #1291 / CI #11284 at exact HEAD `a729acf77205fcef7ff5a2291c3d164b98b1c8fb`:
- R3 278/278 Linux and 278/278 Windows.
- P-A corpus 186/186 per OS.
- R2 independent 256/256 Linux; R2 Python 256/256 Windows.
- Mutation checks, canonical/replayable corpora byte-identical and Rust quality/unit regression: SUCCESS.
- Production SMS/Z80/VDP/PSG/YM2413/CPAL/SDL3/Slint/WGPU/scheduler jobs: SKIPPED, not PASS.

## 7. Remaining FUP-02 work

1. Add a five-event P-E permutation matrix for commutative, noncommutative UNKNOWN, explicit dependency order, and partial order with commutative siblings only if it can preserve P-E's independent implementation and avoid copying R2's oracle logic.
2. Clarify/retain the distinct missing-dependency classifications (Blocked vs InvalidInput) with discriminating fixtures; do not normalize the result codes for superficial parity.
3. Decide whether P-E's observation/alters boundary can be mapped to the R2 causal contract without inventing state fields. If not, document it as non-comparable.
4. Keep the pre-applied causal ledger absent/non-comparable unless P-E acquires an independently justified representation.
5. Re-run Linux and Windows R3, R2 independent/Python, mutation checks and corpus equivalence on the exact resulting SHA.
6. Keep FUP-02 PARTIAL while W06 remains UNKNOWN and W07 remains UNKNOWN/BLOCKED; neither uncertainty is a reason to invent a synthetic resolution.
