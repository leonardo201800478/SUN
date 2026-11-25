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

The current candidate defines 80 tests:
- R2-T01: monotonicity, retrograde-input no-mutation, participant completeness, structural-validation precedence over `Blocked`, exact-time validation, invalid clock conversion/watermark, obligation identity/owner validation.
- R2-T02: chunk invariance, including 100 deterministic random partitions, safe-horizon boundary stops, pending-obligation stops and continuation after resolution.
- R2-T03: exact rational round-trip/composition and 10,000-step conversion sessions across three ratios.
- R2-T04: inclusive safe horizon, endpoint StoppedAt(T), earlier obligations, resume after resolution and multi-domain minimum horizon.
- R2-B deadline slice: 52 cases across publication/admission, owner and generation validation, cancellation versus watermark closure, complete cancellation-frontier derivation from every participant and W, stale-generation suppression, commutative and explicitly ordered ties, UNKNOWN causal order (including C=D without profile relation, with and without an open W), finite logical microsteps including direct-publication bounds, exact/non-regressed horizon validation on publication and cancellation, retroactive scheduled-deadline detection at W closure, and malformed-input rejection for participant/domain/book structures. This is partial coverage for R2-T06, T07 and T14; these properties are not yet PASS.

**Earlier executed candidate:** 24 tests PASS in CI #10619 on Git SHA `a0c0782ff4d8e9886144b20bde36e42bcc6f1eff`, with oracle SHA-256 `160489b753575f0e7fcb12892eb58f7199a1dc5828c5bb4f13c266f557ba001a`, Python 3.14.4, Linux x86_64. This is historical evidence for that exact candidate only.

**Superseded historical execution:** 25 tests PASS in CI #10623 on Git SHA `a8cef1bcc39e284729a7a55fc16e27deb9fb4b22`. This result is retained only as historical evidence for that exact candidate; it is not the current candidate result. Oracle SHA-256 `de802612b198996cd6e888aa9fa8840596fa42ab8319aef0611b3841052a30b3`; runner SHA-256 `bb4798b3cc073b785a8c71e5796a24a595be0cc2ebf32501978bd5d23494bc77`; test module SHA-256 `b5a6ad833240b5f622ac313d98595bd17a079a51289dedf71c7d0f7bdd1f3d1e`. Python 3.14.4, Linux x86_64. Earlier 19-test results are also historical, unbound local evidence.

**Latest validated baseline before the P1/P2 corrections:** 75 tests PASS in CI #10673 on Git SHA `1e73d837afd25782f63b5b69965165e4cef166bc`, with worktree CLEAN. Oracle SHA-256 `41ee0d615996c8003c289eb113623873ecaa8b1a6e852a33a47f1510c354ccd1`; deadline model SHA-256 `27d1d859a77f976187cf8fad1eac8090e879a5a9eea71defe119b60a48a34498`; runner SHA-256 `a22448b28fb8a37e033c17bcc0c47e4aa9c8997a9109dea1afee70f5a9019fa6`; `test_deadlines.py` SHA-256 `dec6b21314b2f3011fa0d701f99dbd6db3dd5fe86d3c922d35b4bbb2ba36681f`; `test_temporal_core.py` SHA-256 `ad4dbc4c7df135db34cea71cecbeae30149a1bca87f51ca7c5704b95212c8eef`. Python 3.14.4, Linux x86_64. Actionlint, documentation hygiene and quality gate passed; runtime Rust, Windows and hardware jobs were SKIPPED. This is the baseline for the next candidate, not evidence for the corrections.

R2-T05, T08–T13 and T15 remain NOT RUN. R2-T06, T07 and T14 are IN PROGRESS and must not be treated as PASS until their full mandatory matrices are implemented and executed. The R2-A and R2-B slices do not complete R2.


The runner sets `sys.dont_write_bytecode` before test discovery. CI invokes it twice from the same checkout and checks `git status --porcelain --untracked-files=all` between invocations; the second run must still begin with a clean worktree. CI also executes the exact documented `python -m unittest -v` command, removes only generated `__pycache__` directories, and verifies that the worktree is clean before the second pinned run. The test suite does not assert runner-specific process state.
