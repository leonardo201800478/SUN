# CHRONOS R2 reference oracle — Python verification artifact

This directory contains an independent, test-only reference model for R2.

## Reproducible run

From this directory, run:

    python run_reference_tests.py

The runner records:
- Git repository HEAD SHA and clean-worktree status;
- SHA-256 of every executable reference-model module, the runner and every discovered test module;
- Python version and platform;
- test-suite result.

For a plain unittest invocation (without the pinned evidence header; it runs the same test cases and does not depend on runner-specific process state):

    python -m unittest -v

No third-party dependencies are required. The model uses exact rational arithmetic and a full scan of obligations. It is intentionally not optimized and must not be imported by the Rust backend or desktop host.

## Current candidate coverage

The current candidate defines 93 tests:
- R2-T01: 13 cases covering monotonicity, retrograde-input no-mutation, participant completeness, structural-validation precedence over `Blocked`, exact-time validation, invalid clock conversion/watermark, obligation identity/owner validation, and detection of a known W/H contradiction before an incomplete-participant block.
- R2-T02: 4 cases for chunk invariance, including 100 deterministic random partitions, safe-horizon boundary stops, pending-obligation stops and continuation after resolution.
- R2-T03: 3 cases for exact rational round-trip/composition and 10,000-step conversion sessions across three ratios.
- R2-T04: 9 cases for inclusive safe horizon, endpoint StoppedAt(T), earlier obligations, resume after resolution, multi-domain minimum horizon and invariant rejection when the confirmed frontier (W or participant progress) exceeds the global safe horizon.
- R2-B deadline slice: 64 cases across publication/admission, owner and generation validation, global minimum-horizon versus known W/progress checks on publication and cancellation, complete cancellation-frontier derivation including mandatory direct-owner participation, empty/incomplete participant blocking, frontier validation before lifecycle classification, cross-validation of pending cancellation owner/identity/generation, strict C<D temporal ordering and orphan rejection, cancellation versus watermark closure including explicit D=W coverage, stale-generation suppression, commutative and explicitly ordered ties, UNKNOWN causal order, finite logical microsteps (including validation of preconstructed books before watermark claims), replacement descriptor prevalidation before cancellation admission, and atomic replacement ordering (new D<C -> LateInput; new D=C -> Indeterminate), horizon validation, and malformed-input rejection. This is partial coverage for R2-T06, T07 and T14; these properties are not yet PASS.

**Earlier executed candidate:** 24 tests PASS in CI #10619 on Git SHA `a0c0782ff4d8e9886144b20bde36e42bcc6f1eff`, with oracle SHA-256 `160489b753575f0e7fcb12892eb58f7199a1dc5828c5bb4f13c266f557ba001a`, Python 3.14.4, Linux x86_64. This is historical evidence for that exact candidate only.

**Superseded historical execution:** 25 tests PASS in CI #10623 on Git SHA `a8cef1bcc39e284729a7a55fc16e27deb9fb4b22`. This result is retained only as historical evidence for that exact candidate; it is not the current candidate result. Oracle SHA-256 `de802612b198996cd6e888aa9fa8840596fa42ab8319aef0611b3841052a30b3`; runner SHA-256 `bb4798b3cc073b785a8c71e5796a24a595be0cc2ebf32501978bd5d23494bc77`; test module SHA-256 `b5a6ad833240b5f622ac313d98595bd17a079a51289dedf71c7d0f7bdd1f3d1e`. Python 3.14.4, Linux x86_64. Earlier 19-test results are also historical, unbound local evidence.

**Latest validated baseline before the P1/P2 corrections:** 75 tests PASS in CI #10673 on Git SHA `1e73d837afd25782f63b5b69965165e4cef166bc`, with worktree CLEAN. Oracle SHA-256 `41ee0d615996c8003c289eb113623873ecaa8b1a6e852a33a47f1510c354ccd1`; deadline model SHA-256 `27d1d859a77f976187cf8fad1eac8090e879a5a9eea71defe119b60a48a34498`; runner SHA-256 `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` SHA-256 `dec6b21314b2f3011fa0d701f99dbd6db3dd5fe86d3c922d35b4bbb2ba36681f`; `test_temporal_core.py` SHA-256 `ad4dbc4c7df135db34cea71cecbeae30149a1bca87f51ca7c5704b95212c8eef`. Python 3.14.4, Linux x86_64. Actionlint, documentation hygiene and quality gate passed; runtime Rust, Windows and hardware jobs were SKIPPED. This is the baseline for the next candidate, not evidence for the corrections.

**Previous executed candidate before the follow-up adversarial findings:** 80/80 tests PASS in CI #10678 on Git SHA `ff57ca2d2500906cbfefe3ca43d8b35745a5a97b`; [CI run](https://github.com/leonardo201800478/SUN/actions/runs/45529329833) — **SUCCESS**. The pinned runner passed twice, and the exact documented `python -m unittest -v` command passed; worktree CLEAN. Environment: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `701ca24e4f61bcb083504c87a079b72f0f034bd4b7abc5e3ec5fd9d176986265`; `deadline_reference.py` = `b92716d4b29e0e57846679b310ba1519f72685967308c6d16206b54c0ac0ef07`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `587f43e2dde2816f70b1e59b6f73ee8fdf828f4c9ecce79d5b50a1a0fd8d2c4b`; `test_temporal_core.py` = `fdbc70d9bd8dd79092b604bcb317d7f5af29205f73aa11a9b9ef9a6648730b55`. Actionlint, documentation patch hygiene and quality supervisor: PASS. Rust workspace, Windows and hardware-specific jobs: SKIPPED, not PASS. This is historical evidence for the exact SHA above, not the current candidate.

**Current executed candidate after the follow-up adversarial findings:** 89/89 tests PASS in CI #10681 on Git SHA `8c466dcaa1b5d5b4d6e9f076a03a1df9f0a2f2ab`; [CI run](https://github.com/leonardo201800478/SUN/actions/runs/45530977080) — **SUCCESS**. The dedicated runner passed twice, the exact documented `python -m unittest -v` command passed, and the worktree was CLEAN. Environment: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `d972005df827412bed74f0d1cc655e64fdad2aae3325f1f8842e4d4b5b531edb`; `deadline_reference.py` = `4a3999f3c0fa800a0feca4e35ffa5297e34a0eea0e6562014005f6b22496e942`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `fb33bda1923ca1d026fdb70a1c4a9ef11adb6a71b0fe8357a4ff2cc46a6ca9c7`; `test_temporal_core.py` = `c9fce1e6e3d247e9811d3a0c252066a91af8e81cc2ae2e08a32cf4a13ee03770`. Actionlint, documentation patch hygiene and Rust quality supervisor: PASS. Rust workspace regression, Windows and hardware-specific jobs: SKIPPED, not PASS. This execution validates the 89 implemented cases only; it does not approve R2.


R2-T05, T08–T13 and T15 remain NOT RUN. R2-T06, T07 and T14 are IN PROGRESS and must not be treated as PASS until their full mandatory matrices are implemented and executed. The R2-A and R2-B slices do not complete R2.


The runner sets `sys.dont_write_bytecode` before test discovery. CI invokes it twice from the same checkout and checks `git status --porcelain --untracked-files=all` between invocations; the second run must still begin with a clean worktree. CI also executes the exact documented `python -m unittest -v` command, removes only generated `__pycache__` directories, and verifies that the worktree is clean before the second pinned run. The test suite does not assert runner-specific process state.


**Current executed candidate — three additional adversarial findings:** CI #10686 on code/test HEAD `03c07437bf3a062735602c2a62bdac31b41d4503` ([run](https://github.com/leonardo201800478/SUN/actions/runs/45539159258)) — **SUCCESS**. CHRONOS R2: **93/93 PASS**; the pinned runner passed twice and the exact `python -m unittest -v` invocation passed; worktree CLEAN on both pinned runs. Environment: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `d972005df827412bed74f0d1cc655e64fdad2aae3325f1f8842e4d4b5b531edb`; `deadline_reference.py` = `a1fc6b078e0165ff4fcd59adcbe6317288edab6ee5952fe534bb56fb51c218dd`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `2c55c6789d4d426b069a31f045af5437ce09a66a3368783f3ea62f515b6c4943`; `test_temporal_core.py` = `c9fce1e6e3d247e9811d3a0c252066a91af8e81cc2ae2e08a32cf4a13ee03770`. Actionlint, documentation patch hygiene and Rust quality/unit regression: PASS. Rust workspace, Windows and hardware-specific jobs: SKIPPED, not PASS. This execution validates the 93 implemented cases only; it does not approve R2.

**Current executed candidate — preconstructed microstep bound:** CI #10684 on code/test HEAD `62480ce2502b3783d92c39d2afe91ea064ce8f65` ([run](https://github.com/leonardo201800478/SUN/actions/runs/45537064335)) — **SUCCESS**. CHRONOS R2: **90/90 PASS**; the pinned runner passed twice and the exact `python -m unittest -v` invocation passed; worktree CLEAN on both pinned runs. Environment: Python 3.14.4 / Linux x86_64. SHA-256: `chronos_reference.py` = `d972005df827412bed74f0d1cc655e64fdad2aae3325f1f8842e4d4b5b531edb`; `deadline_reference.py` = `30cbe9870750958752f2a2e7338bcfb523fa903aa8066f30aaffc93081fbd0c6`; runner = `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` = `0d594b09a18af442b7cb142a85862eb8a8f459f45d4f6b8bd17592f65111b130`; `test_temporal_core.py` = `c9fce1e6e3d247e9811d3a0c252066a91af8e81cc2ae2e08a32cf4a13ee03770`. Actionlint, documentation patch hygiene and Rust quality/unit regression: PASS. Rust workspace, Windows, hardware and unrelated subsystem jobs: SKIPPED, not PASS. This evidence validates only the 90 implemented cases and does not approve R2.


**New candidate checks (not yet PASS):** cancellation admission now requires the deadline owner to be included in the declared participant set and present in the complete set; a missing owner yields `Blocked`, never a partial A(E). A pending cancellation at C=D yields `IndeterminateCausalOrder`; C>D yields `LateInput`; both are rejected before arbitration or watermark mutation. `replace_deadline()` validates owner/identity/generation, next generation, timestamps, effect ID and boolean flags before attempting cancellation admission, so a malformed replacement descriptor returns `InvalidInput` even when `cancel_at` is temporally late. Regression coverage checks each case and original-book preservation.
